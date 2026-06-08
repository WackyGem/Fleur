use std::collections::HashMap;

pub(super) mod materialize;
pub(super) mod planning;
pub(super) mod writing;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{RsiRunRequest, RsiWriteMode};
use crate::runners::shared::{
    RetainStagingRows, cleanup_staging, ensure_output_schema, ensure_production_output_rows,
    ensure_production_symbols, group_rsi_input_rows, replace_partitions,
    retain_existing_rows_for_staging, setup_staging, table_exists, validate_staging_or_error,
};
use crate::schema::{
    create_rsi_output_table_sql, create_rsi_staging_table_sql, drop_rsi_staging_table_sql,
    rsi_staging_table_name,
};
use crate::summary::{
    PartitionReplaceSummary, PerformanceTimings, RsiRunSummary, ValidationSummary, time_result,
};
use crate::validation::affected_years;

use self::materialize::calculate_rsi_outputs;
use self::planning::{
    count_rsi_gap_symbols, read_previous_rsi_states, read_rsi_input_row_binary,
    read_rsi_mixed_input_row_binary, resolve_rsi_effective_output_to, resolve_rsi_input_from,
    resolve_rsi_symbols,
};
use self::writing::{ensure_rsi_append_latest_is_safe, insert_rsi_result_rows};

/// 基于 ClickHouse 执行完整 RSI 计算。
///
/// # 错误
///
/// 当请求校验、ClickHouse I/O 或指标计算失败时，返回 [`FurnaceIoError`]。
pub fn run_rsi<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
) -> Result<RsiRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        ensure_output_schema(
            executor,
            &create_rsi_output_table_sql(&request.output_table),
        )?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_rsi_symbols(executor, request)?;
    ensure_production_symbols("RSI", request.mode.writes_applied(), &symbols)?;
    let effective_output_to =
        resolve_rsi_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let full_history_input_from =
        resolve_rsi_input_from(executor, request, &symbols, all_symbols_requested)?;
    let request_covers_full_history =
        request.request_from.as_str() <= full_history_input_from.as_str();
    let rsi_target_exists = table_exists(executor, &request.output_table)?;
    let previous_states = if rsi_target_exists
        && request.mode != RsiWriteMode::ReplaceCascade
        && !request_covers_full_history
    {
        let timed = time_result(|| {
            read_previous_rsi_states(executor, request, &symbols, all_symbols_requested)
        })?;
        timings.read_state = timed.elapsed;
        timed.value
    } else {
        HashMap::new()
    };
    let timed_gap =
        time_result(|| count_rsi_gap_symbols(executor, request, &symbols, all_symbols_requested))?;
    timings.read_state += timed_gap.elapsed;
    let (gap_symbols_count, gap_fill_from) = timed_gap.value;
    if request.mode == RsiWriteMode::AppendLatest && gap_symbols_count > 0 {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "append-latest found RSI result gaps for {gap_symbols_count} symbols; rerun from {} or use replace-cascade",
            gap_fill_from.as_deref().unwrap_or(&request.request_from)
        )));
    }

    let can_use_previous_state = request.mode != RsiWriteMode::ReplaceCascade
        && gap_symbols_count == 0
        && !previous_states.is_empty();
    let states_for_compute = if can_use_previous_state {
        previous_states
    } else {
        HashMap::new()
    };

    let timed_input = if can_use_previous_state {
        time_result(|| {
            read_rsi_mixed_input_row_binary(
                executor,
                request,
                &symbols,
                all_symbols_requested,
                &effective_output_to,
            )
        })?
    } else {
        time_result(|| {
            read_rsi_input_row_binary(
                executor,
                request,
                &symbols,
                all_symbols_requested,
                &full_history_input_from,
                &effective_output_to,
            )
        })?
    };
    timings.read_input = timed_input.elapsed;
    let input_bytes = timed_input.value;
    let timed_groups = time_result(|| group_rsi_input_rows(&input_bytes))?;
    timings.group = timed_groups.elapsed;
    drop(input_bytes);
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;
    let input_valid_close_rows = input_groups.valid_close_rows;
    let input_from = input_groups
        .input_from
        .clone()
        .unwrap_or(full_history_input_from);

    let calculated = calculate_rsi_outputs(
        request,
        &effective_output_to,
        input_groups.groups,
        input_rows_count as usize,
        &states_for_compute,
        request.mode.writes_applied(),
    )?;
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    ensure_production_output_rows("RSI", request.mode.writes_applied(), output_rows.is_empty())?;
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        RsiWriteMode::DryRun => {}
        RsiWriteMode::AppendLatest => {
            ensure_rsi_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_rsi_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        RsiWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = rsi_staging_table_name(&request.output_table, run_id);
            let drop_staging_sql = drop_rsi_staging_table_sql(&staging);
            let timed = time_result(|| {
                setup_staging(
                    executor,
                    drop_staging_sql.clone(),
                    create_rsi_staging_table_sql(&request.output_table, &staging),
                )
            })?;
            timings.staging += timed.elapsed;
            let timed = time_result(|| {
                retain_existing_rows_for_staging(
                    executor,
                    &RetainStagingRows {
                        output_table: &request.output_table,
                        staging_table: &staging,
                        request_from: &request.request_from,
                        symbols: &symbols,
                        all_symbols_requested,
                        years: &affected_years,
                        effective_output_to: &effective_output_to,
                    },
                )
            })?;
            timings.staging += timed.elapsed;
            retained_rows = timed.value;
            let timed = time_result(|| {
                insert_rsi_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
            })?;
            timings.write += timed.elapsed;
            let timed =
                time_result(|| validate_staging_or_error(executor, &staging, &affected_years))?;
            timings.staging += timed.elapsed;
            staging_validation = timed.value;
            let timed = time_result(|| {
                replace_partitions(executor, &request.output_table, &staging, &affected_years)
            })?;
            timings.partition_replace += timed.elapsed;
            let timed = time_result(|| cleanup_staging(executor, &drop_staging_sql))?;
            timings.staging += timed.elapsed;
            partition_replace = PartitionReplaceSummary::replaced(affected_years.clone());
            staging_table = Some(staging);
        }
    }

    let rsi_state_source = if can_use_previous_state {
        if states_for_compute.len() == symbols.len() {
            format!("previous-state:{}", states_for_compute.len())
        } else {
            format!(
                "mixed:previous-state:{},full-history:{}",
                states_for_compute.len(),
                symbols.len().saturating_sub(states_for_compute.len())
            )
        }
    } else {
        "full-history".to_string()
    };
    let symbols_count = symbols.len() as u64;
    let performance_metrics = timings.finish(input_rows_count, output_rows_count, symbols_count);

    Ok(RsiRunSummary {
        request_from: request.request_from.clone(),
        request_to: request.request_to.clone(),
        effective_output_from: request.request_from.clone(),
        effective_output_to: effective_output_to.clone(),
        input_from,
        input_to: effective_output_to,
        mode: request.mode,
        symbols,
        input_rows: input_rows_count,
        output_rows: output_rows_count,
        valid_close_rows: input_valid_close_rows,
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        rsi_state_source,
        gap_symbols_count,
        gap_fill_from,
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}
