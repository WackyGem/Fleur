use std::collections::HashMap;

pub(super) mod materialize;
pub(super) mod planning;
pub(super) mod writing;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{MaRunRequest, MaWriteMode};
use crate::runners::shared::{
    RetainStagingRows, cleanup_staging, ensure_output_schema, ensure_production_output_rows,
    ensure_production_symbols, group_ma_input_rows, replace_partitions,
    retain_existing_rows_for_staging, setup_staging, table_exists, validate_staging_or_error,
};
use crate::schema::{
    create_ma_output_table_sql, create_ma_staging_table_sql, drop_ma_staging_table_sql,
    ma_staging_table_name,
};
use crate::summary::{
    MaRunSummary, PartitionReplaceSummary, PerformanceTimings, ValidationSummary, time_result,
};
use crate::validation::affected_years;

use self::materialize::calculate_ma_outputs;
use self::planning::{
    ma_symbols_started_before, read_ma_input_rows, read_previous_ma_states,
    resolve_ma_effective_output_to, resolve_ma_input_from, resolve_ma_lookback_input_from,
    resolve_ma_symbols,
};
use self::writing::{ensure_ma_append_latest_is_safe, insert_ma_result_rows};

/// 基于 ClickHouse 执行完整 Moving Average 计算。
///
/// # 错误
///
/// 当请求校验、ClickHouse I/O 或指标计算失败时，返回 [`FurnaceIoError`]。
pub fn run_ma<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
) -> Result<MaRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        ensure_ma_output_schema(executor, &request.output_table)?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_ma_symbols(executor, request)?;
    ensure_production_symbols("MA", request.mode.writes_applied(), &symbols)?;
    let effective_output_to =
        resolve_ma_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let ma_target_exists = table_exists(executor, &request.output_table)?;
    let ma_states = if ma_target_exists {
        let timed = time_result(|| {
            read_previous_ma_states(executor, request, &symbols, all_symbols_requested)
        })?;
        timings.read_state = timed.elapsed;
        timed.value
    } else {
        HashMap::new()
    };
    let missing_state_symbols = symbols
        .iter()
        .filter(|symbol| !ma_states.contains_key(symbol.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let can_consider_previous_state =
        request.mode != MaWriteMode::ReplaceCascade && !symbols.is_empty() && !ma_states.is_empty();
    let lookback_input_from = if can_consider_previous_state {
        Some(resolve_ma_lookback_input_from(
            executor,
            request,
            &symbols,
            all_symbols_requested,
        )?)
    } else {
        None
    };
    let missing_started_before_lookback = if let Some(input_from) = lookback_input_from.as_deref() {
        !missing_state_symbols.is_empty()
            && ma_symbols_started_before(executor, request, &missing_state_symbols, input_from)?
    } else {
        false
    };
    let can_use_previous_state = can_consider_previous_state && !missing_started_before_lookback;
    let input_from = if can_use_previous_state {
        lookback_input_from.unwrap_or_else(|| request.request_from.clone())
    } else {
        resolve_ma_input_from(executor, request, &symbols, all_symbols_requested)?
    };
    let timed_input = time_result(|| {
        read_ma_input_rows(
            executor,
            request,
            &symbols,
            all_symbols_requested,
            &input_from,
            &effective_output_to,
        )
    })?;
    timings.read_input = timed_input.elapsed;
    let input_rows = timed_input.value;
    let timed_groups = time_result(|| Ok(group_ma_input_rows(input_rows)))?;
    timings.group = timed_groups.elapsed;
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;
    let input_valid_close_rows = input_groups.valid_close_rows;
    let input_valid_volume_rows = input_groups.valid_volume_rows;

    let calculated = calculate_ma_outputs(
        request,
        &effective_output_to,
        input_groups.groups,
        input_rows_count as usize,
        &ma_states,
        request.mode.writes_applied(),
    )?;
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    ensure_production_output_rows("MA", request.mode.writes_applied(), output_rows.is_empty())?;
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        MaWriteMode::DryRun => {}
        MaWriteMode::AppendLatest => {
            ensure_ma_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_ma_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        MaWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = ma_staging_table_name(&request.output_table, run_id);
            let drop_staging_sql = drop_ma_staging_table_sql(&staging);
            let timed = time_result(|| {
                setup_staging(
                    executor,
                    drop_staging_sql.clone(),
                    create_ma_staging_table_sql(&request.output_table, &staging),
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
                insert_ma_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
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

    let symbols_count = symbols.len() as u64;
    let performance_metrics = timings.finish(input_rows_count, output_rows_count, symbols_count);

    Ok(MaRunSummary {
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
        valid_close_rows: calculated.valid_close_rows.min(input_valid_close_rows),
        valid_volume_rows: calculated.valid_volume_rows.min(input_valid_volume_rows),
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        ema_state_source: if can_use_previous_state {
            if missing_state_symbols.is_empty() {
                "previous-state".to_string()
            } else {
                "mixed".to_string()
            }
        } else {
            "full-history".to_string()
        },
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}

fn ensure_ma_output_schema<E: ClickHouseExecutor>(
    executor: &mut E,
    output_table: &str,
) -> Result<(), FurnaceIoError> {
    ensure_output_schema(executor, &create_ma_output_table_sql(output_table))?;
    executor.execute(&format!(
        "ALTER TABLE {output_table} ADD COLUMN IF NOT EXISTS price_ma_30 Nullable(Float64) AFTER price_ma_28"
    ))
}
