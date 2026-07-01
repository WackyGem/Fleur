use std::collections::HashMap;

pub(super) mod materialize;
pub(super) mod planning;
pub(super) mod writing;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{MacdRunRequest, MacdWriteMode};
use crate::runners::shared::{
    RetainStagingRows, cleanup_staging, ensure_output_schema, ensure_production_output_rows,
    ensure_production_symbols, group_macd_input_rows, replace_partitions,
    retain_existing_rows_for_staging, setup_staging, table_exists, validate_staging_or_error,
};
use crate::schema::{
    create_macd_output_table_sql, create_macd_staging_table_sql, drop_macd_staging_table_sql,
    macd_staging_table_name,
};
use crate::summary::{
    MacdRunSummary, PartitionReplaceSummary, PerformanceTimings, ValidationSummary, time_result,
};
use crate::validation::affected_years;

use self::materialize::{calculate_macd_dry_run_from_input_rows, calculate_macd_outputs};
use self::planning::{
    count_macd_gap_symbols, count_macd_incomplete_state_symbols, read_macd_input_rows,
    read_macd_mixed_input_rows, read_previous_macd_states, resolve_macd_effective_output_to,
    resolve_macd_input_from, resolve_macd_symbols,
};
use self::writing::{ensure_macd_append_latest_is_safe, insert_macd_result_rows};

/// Execute a complete ClickHouse-backed MACD calculation.
///
/// # Errors
///
/// Returns [`FurnaceIoError`] when request validation, ClickHouse I/O, or
/// indicator calculation fails.
pub fn run_macd<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
) -> Result<MacdRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        ensure_output_schema(
            executor,
            &create_macd_output_table_sql(&request.output_table),
        )?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_macd_symbols(executor, request)?;
    ensure_production_symbols("MACD", request.mode.writes_applied(), &symbols)?;
    let effective_output_to =
        resolve_macd_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let full_history_input_from =
        resolve_macd_input_from(executor, request, &symbols, all_symbols_requested)?;
    let request_covers_full_history =
        request.request_from.as_str() <= full_history_input_from.as_str();
    let macd_target_exists = table_exists(executor, &request.output_table)?;

    let previous_states = if macd_target_exists
        && request.mode != MacdWriteMode::ReplaceCascade
        && !request_covers_full_history
    {
        let timed = time_result(|| {
            read_previous_macd_states(executor, request, &symbols, all_symbols_requested)
        })?;
        timings.read_state = timed.elapsed;
        timed.value
    } else {
        HashMap::new()
    };
    let timed_incomplete = time_result(|| {
        if macd_target_exists {
            count_macd_incomplete_state_symbols(executor, request, &symbols, all_symbols_requested)
        } else {
            Ok(0)
        }
    })?;
    timings.read_state += timed_incomplete.elapsed;
    let incomplete_state_symbols_count = timed_incomplete.value;
    let timed_gap = time_result(|| {
        if macd_target_exists {
            count_macd_gap_symbols(executor, request, &symbols, all_symbols_requested)
        } else {
            Ok((0, None))
        }
    })?;
    timings.read_state += timed_gap.elapsed;
    let (gap_symbols_count, gap_fill_from) = timed_gap.value;
    if request.mode == MacdWriteMode::AppendLatest && gap_symbols_count > 0 {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "append-latest found MACD result gaps for {gap_symbols_count} symbols; rerun from {} or use replace-cascade",
            gap_fill_from.as_deref().unwrap_or(&request.request_from)
        )));
    }

    let can_use_previous_state = request.mode != MacdWriteMode::ReplaceCascade
        && gap_symbols_count == 0
        && !previous_states.is_empty();
    let states_for_compute = if can_use_previous_state {
        previous_states
    } else {
        HashMap::new()
    };

    let timed_input = if can_use_previous_state {
        time_result(|| {
            read_macd_mixed_input_rows(
                executor,
                request,
                &symbols,
                all_symbols_requested,
                &effective_output_to,
            )
        })?
    } else {
        time_result(|| {
            read_macd_input_rows(
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
    let input_rows = timed_input.value;
    let (calculated, input_rows_count, valid_close_rows, input_from) =
        if request.mode == MacdWriteMode::DryRun {
            let dry_run = calculate_macd_dry_run_from_input_rows(
                request,
                &effective_output_to,
                input_rows,
                &states_for_compute,
            )?;
            let input_from = dry_run
                .input_from
                .clone()
                .unwrap_or(full_history_input_from);
            (
                dry_run.result,
                dry_run.input_rows,
                dry_run.valid_close_rows,
                input_from,
            )
        } else {
            let timed_groups = time_result(|| Ok(group_macd_input_rows(input_rows)))?;
            timings.group = timed_groups.elapsed;
            let input_groups = timed_groups.value;
            let input_rows_count = input_groups.input_rows;
            let valid_close_rows = input_groups.valid_close_rows;
            let input_from = input_groups
                .input_from
                .clone()
                .unwrap_or(full_history_input_from);
            let calculated = calculate_macd_outputs(
                request,
                &effective_output_to,
                input_groups.groups,
                input_rows_count as usize,
                &states_for_compute,
                request.mode.writes_applied(),
            )?;
            (calculated, input_rows_count, valid_close_rows, input_from)
        };
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    ensure_production_output_rows(
        "MACD",
        request.mode.writes_applied(),
        output_rows.is_empty(),
    )?;
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        MacdWriteMode::DryRun => {}
        MacdWriteMode::AppendLatest => {
            ensure_macd_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_macd_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        MacdWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = macd_staging_table_name(&request.output_table, run_id);
            let drop_staging_sql = drop_macd_staging_table_sql(&staging);
            let timed = time_result(|| {
                setup_staging(
                    executor,
                    drop_staging_sql.clone(),
                    create_macd_staging_table_sql(&request.output_table, &staging),
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
                insert_macd_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
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

    let macd_state_source = if can_use_previous_state {
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

    Ok(MacdRunSummary {
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
        valid_close_rows,
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        macd_state_source,
        incomplete_state_symbols_count,
        gap_symbols_count,
        gap_fill_from,
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}
