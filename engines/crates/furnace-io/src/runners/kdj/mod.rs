use std::collections::HashMap;

pub(super) mod materialize;
pub(super) mod planning;
pub(super) mod writing;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{KdjRunRequest, KdjWriteMode};
use crate::runners::shared::{group_input_rows, target_table_exists, validate_staging};
use crate::schema::{
    DEFAULT_KDJ_OUTPUT_TABLE, create_calculation_database_sql, create_kdj_output_table_sql,
    create_kdj_staging_table_sql, drop_kdj_staging_table_sql, kdj_staging_table_name,
    replace_kdj_partition_sql,
};
use crate::summary::{
    KdjRunSummary, PartitionReplaceSummary, PerformanceTimings, ValidationSummary, time_result,
};
use crate::validation::affected_years;

use self::materialize::calculate_outputs;
use self::planning::{read_input_row_binary, read_previous_states, resolve_effective_output_to};
use self::planning::{resolve_input_from, resolve_symbols};
use self::writing::{
    ensure_append_latest_is_safe, insert_result_rows, retain_old_rows_for_staging,
};

/// 当请求校验、ClickHouse I/O 或指标计算失败时，返回 [`FurnaceIoError`]。
pub fn run_kdj<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
) -> Result<KdjRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        executor.execute(create_calculation_database_sql())?;
        executor.execute(&create_kdj_output_table_sql())?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_symbols(executor, request)?;
    if request.mode.writes_applied() && symbols.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production KDJ writes require at least one input security".to_string(),
        ));
    }
    let effective_output_to =
        resolve_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let input_from = resolve_input_from(executor, request, &symbols, all_symbols_requested)?;
    let target_exists = target_table_exists(executor)?;
    let states = if target_exists {
        let timed = time_result(|| {
            read_previous_states(executor, request, &symbols, all_symbols_requested)
        })?;
        timings.read_state = timed.elapsed;
        timed.value
    } else {
        HashMap::new()
    };
    let timed_input = time_result(|| {
        read_input_row_binary(
            executor,
            &symbols,
            all_symbols_requested,
            &input_from,
            &effective_output_to,
        )
    })?;
    timings.read_input = timed_input.elapsed;
    let input_bytes = timed_input.value;
    let timed_groups = time_result(|| group_input_rows(&input_bytes))?;
    timings.group = timed_groups.elapsed;
    drop(input_bytes);
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;

    let calculated = calculate_outputs(
        request,
        &effective_output_to,
        input_groups.groups,
        input_rows_count as usize,
        &states,
        request.mode.writes_applied(),
    )?;
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    if request.mode.writes_applied() && output_rows.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production KDJ writes produced no output rows".to_string(),
        ));
    }
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        KdjWriteMode::DryRun => {}
        KdjWriteMode::AppendLatest => {
            ensure_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_result_rows(
                    executor,
                    DEFAULT_KDJ_OUTPUT_TABLE,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        KdjWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = kdj_staging_table_name(run_id);
            let staging_setup_sql = vec![
                drop_kdj_staging_table_sql(&staging),
                create_kdj_staging_table_sql(&staging),
            ];
            let timed = time_result(|| executor.execute_many(&staging_setup_sql))?;
            timings.staging += timed.elapsed;
            let timed = time_result(|| {
                retain_old_rows_for_staging(
                    executor,
                    request,
                    &staging,
                    &symbols,
                    all_symbols_requested,
                    &affected_years,
                    &effective_output_to,
                )
            })?;
            timings.staging += timed.elapsed;
            retained_rows = timed.value;
            let timed = time_result(|| {
                insert_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
            })?;
            timings.write += timed.elapsed;
            let timed = time_result(|| validate_staging(executor, &staging, &affected_years))?;
            timings.staging += timed.elapsed;
            staging_validation = timed.value;
            if staging_validation.status != "passed" {
                return Err(FurnaceIoError::InvalidRequest(format!(
                    "staging validation failed with {} duplicate keys",
                    staging_validation.duplicate_keys
                )));
            }
            let replace_sql = affected_years
                .iter()
                .map(|year| replace_kdj_partition_sql(&staging, *year))
                .collect::<Vec<_>>();
            let timed = time_result(|| executor.execute_many(&replace_sql))?;
            timings.partition_replace += timed.elapsed;
            let timed = time_result(|| executor.execute(&drop_kdj_staging_table_sql(&staging)))?;
            timings.staging += timed.elapsed;
            partition_replace = PartitionReplaceSummary::replaced(affected_years.clone());
            staging_table = Some(staging);
        }
    }

    let state_source = if states.is_empty() {
        "initial_50".to_string()
    } else {
        format!("previous_kd_rows:{}", states.len())
    };
    let symbols_count = symbols.len() as u64;
    let performance_metrics = timings.finish(input_rows_count, output_rows_count, symbols_count);

    Ok(KdjRunSummary {
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
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        params: request.params,
        state_source,
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}
