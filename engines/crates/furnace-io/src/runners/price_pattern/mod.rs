pub(super) mod materialize;
pub(super) mod planning;
pub(super) mod writing;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{PricePatternRunRequest, PricePatternWriteMode};
use crate::runners::shared::{
    RetainStagingRows, cleanup_staging, ensure_output_schema, ensure_production_output_rows,
    ensure_production_symbols, group_price_pattern_input_rows, replace_partitions,
    retain_existing_rows_for_staging, setup_staging, validate_staging_or_error,
};
use crate::schema::{
    create_price_pattern_output_table_sql, create_price_pattern_staging_table_sql,
    drop_price_pattern_staging_table_sql, price_pattern_staging_table_name,
};
use crate::summary::{
    PartitionReplaceSummary, PerformanceTimings, PricePatternRunSummary, ValidationSummary,
    time_result,
};
use crate::validation::affected_years;

use self::materialize::calculate_price_pattern_outputs;
use self::planning::{
    read_price_pattern_input_rows, resolve_price_pattern_effective_output_to,
    resolve_price_pattern_full_history_input_from, resolve_price_pattern_symbols,
};
use self::writing::{ensure_price_pattern_append_latest_is_safe, insert_price_pattern_result_rows};

/// Execute a full Price Pattern ClickHouse run.
///
/// # Errors
///
/// Returns [`FurnaceIoError`] when request validation, ClickHouse I/O, or computation fails.
pub fn run_price_pattern<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &PricePatternRunRequest,
) -> Result<PricePatternRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        ensure_output_schema(
            executor,
            &create_price_pattern_output_table_sql(&request.output_table),
        )?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_price_pattern_symbols(executor, request)?;
    ensure_production_symbols("Price Pattern", request.mode.writes_applied(), &symbols)?;
    let effective_output_to = resolve_price_pattern_effective_output_to(
        executor,
        request,
        &symbols,
        all_symbols_requested,
    )?;
    let input_from = resolve_price_pattern_full_history_input_from(
        executor,
        request,
        &symbols,
        all_symbols_requested,
    )?;
    let timed_input = time_result(|| {
        read_price_pattern_input_rows(
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
    let timed_groups = time_result(|| Ok(group_price_pattern_input_rows(input_rows)))?;
    timings.group = timed_groups.elapsed;
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;
    let input_valid_streak_rows = input_groups.input_valid_streak_rows;
    let input_valid_structure_bar_rows = input_groups.input_valid_structure_bar_rows;

    let calculated = calculate_price_pattern_outputs(
        request,
        &effective_output_to,
        input_groups.groups,
        input_rows_count as usize,
        request.mode.writes_applied(),
    )?;
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    ensure_production_output_rows(
        "Price Pattern",
        request.mode.writes_applied(),
        output_rows.is_empty(),
    )?;
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        PricePatternWriteMode::DryRun => {}
        PricePatternWriteMode::AppendLatest => {
            ensure_price_pattern_append_latest_is_safe(
                executor,
                request,
                &symbols,
                all_symbols_requested,
            )?;
            let timed = time_result(|| {
                insert_price_pattern_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        PricePatternWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = price_pattern_staging_table_name(&request.output_table, run_id);
            let drop_staging_sql = drop_price_pattern_staging_table_sql(&staging);
            let timed = time_result(|| {
                setup_staging(
                    executor,
                    drop_staging_sql.clone(),
                    create_price_pattern_staging_table_sql(&request.output_table, &staging),
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
                insert_price_pattern_result_rows(
                    executor,
                    &staging,
                    &output_rows,
                    request.insert_batch_size,
                )
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

    Ok(PricePatternRunSummary {
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
        input_valid_streak_rows,
        input_valid_structure_bar_rows,
        valid_streak_rows: calculated.valid_streak_rows,
        valid_structure_bar_rows: calculated.valid_structure_bar_rows,
        null_streak_rows: calculated.null_streak_rows,
        null_second_low_rows: calculated.null_second_low_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        state_source: "full-history".to_string(),
        n_structure_window: request.params.n_structure_window,
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}
