pub(super) mod materialize;
pub(super) mod planning;
pub(super) mod writing;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{BollRunRequest, BollWriteMode};
use crate::runners::shared::{
    RetainStagingRows, cleanup_staging, ensure_output_schema, ensure_production_output_rows,
    ensure_production_symbols, group_boll_input_rows, rebuild_output_schema, replace_partitions,
    retain_existing_rows_for_staging, setup_staging, validate_staging_or_error,
};
use crate::schema::{
    boll_staging_table_name, create_boll_output_table_sql, create_boll_staging_table_sql,
    drop_boll_staging_table_sql,
};
use crate::summary::{
    BollRunSummary, PartitionReplaceSummary, PerformanceTimings, ValidationSummary, time_result,
};
use crate::validation::affected_years;

use self::materialize::calculate_boll_outputs;
use self::planning::{
    read_boll_input_rows, resolve_boll_effective_output_to, resolve_boll_lookback_input_from,
    resolve_boll_symbols,
};
use self::writing::{ensure_boll_append_latest_is_safe, insert_boll_result_rows};

/// 基于 ClickHouse 执行完整 Bollinger Bands 计算。
///
/// # 错误
///
/// 当请求校验、ClickHouse I/O 或指标计算失败时，返回 [`FurnaceIoError`]。
pub fn run_boll<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
) -> Result<BollRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() && request.mode != BollWriteMode::RebuildTable {
        ensure_output_schema(
            executor,
            &create_boll_output_table_sql(&request.output_table),
        )?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_boll_symbols(executor, request)?;
    ensure_production_symbols("Bollinger Bands", request.mode.writes_applied(), &symbols)?;
    let effective_output_to =
        resolve_boll_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let input_from =
        resolve_boll_lookback_input_from(executor, request, &symbols, all_symbols_requested)?;
    let timed_input = time_result(|| {
        read_boll_input_rows(
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
    let timed_groups = time_result(|| Ok(group_boll_input_rows(input_rows)))?;
    timings.group = timed_groups.elapsed;
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;
    let input_valid_close_rows = input_groups.input_valid_close_rows;

    let calculated = calculate_boll_outputs(
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
        "Bollinger Bands",
        request.mode.writes_applied(),
        output_rows.is_empty(),
    )?;
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let output_valid_close_rows = calculated.output_valid_close_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        BollWriteMode::DryRun => {}
        BollWriteMode::AppendLatest => {
            ensure_boll_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_boll_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        BollWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = boll_staging_table_name(&request.output_table, run_id);
            let drop_staging_sql = drop_boll_staging_table_sql(&staging);
            let timed = time_result(|| {
                setup_staging(
                    executor,
                    drop_staging_sql.clone(),
                    create_boll_staging_table_sql(&request.output_table, &staging),
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
                insert_boll_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
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
        BollWriteMode::RebuildTable => {
            let timed = time_result(|| {
                rebuild_output_schema(
                    executor,
                    &request.output_table,
                    create_boll_output_table_sql(&request.output_table),
                )
            })?;
            timings.staging += timed.elapsed;
            let timed = time_result(|| {
                insert_boll_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
    }

    let symbols_count = symbols.len() as u64;
    let performance_metrics = timings.finish(input_rows_count, output_rows_count, symbols_count);

    Ok(BollRunSummary {
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
        input_valid_close_rows,
        output_valid_close_rows,
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        state_source: "rolling-lookback".to_string(),
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}
