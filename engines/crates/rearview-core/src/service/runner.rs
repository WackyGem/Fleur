use crate::error::{RearviewError, RearviewResult};
use crate::planner::{QueryPlanner, QuerySettings};
use crate::service::AppState;

pub async fn execute_run(state: AppState, run_id: String) {
    if let Err(error) = execute_run_inner(&state, &run_id).await {
        let status = failed_status(&error);
        let _ = state
            .postgres
            .set_run_status(&run_id, status, Some(&error))
            .await;
    }
}

async fn execute_run_inner(state: &AppState, run_id: &str) -> RearviewResult<()> {
    state
        .postgres
        .set_run_status(run_id, "compiling", None)
        .await?;
    let run = state.postgres.get_run(run_id).await?;
    let rule = state
        .postgres
        .get_rule_version_spec(&run.rule_version_id)
        .await?;
    let chunks = state.postgres.list_run_chunks(run_id).await?;
    let planner = QueryPlanner::new(state.catalog.clone());
    let settings = QuerySettings {
        max_execution_time_seconds: state.config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: state.config.clickhouse.max_rows_to_read,
        max_bytes_to_read: state.config.clickhouse.max_bytes_to_read,
    };
    let mut compiled_hashes = Vec::with_capacity(chunks.len());
    for chunk in &chunks {
        let compiled = planner.compile(
            &rule,
            Some(chunk.start_date),
            Some(chunk.end_date),
            u32::try_from(run.top_n).map_err(|error| {
                RearviewError::Validation(format!("run top_n out of range: {error}"))
            })?,
            settings,
        )?;
        compiled_hashes.push(compiled.sql_hash.clone());
    }
    state
        .postgres
        .set_run_compiled_sql_hash(run_id, &combined_hash(&compiled_hashes))
        .await?;

    for chunk in chunks {
        let query_id = format!("rearview-{run_id}-chunk-{}", chunk.chunk_no);
        state
            .postgres
            .set_run_status(run_id, "running_clickhouse", None)
            .await?;
        state
            .postgres
            .set_chunk_running(run_id, chunk.chunk_no, &query_id)
            .await?;
        let trade_dates = match state
            .clickhouse
            .query_trade_dates(
                chunk.start_date,
                chunk.end_date,
                &format!("{query_id}-trade-dates"),
            )
            .await
        {
            Ok(trade_dates) => trade_dates,
            Err(error) => {
                state
                    .postgres
                    .set_chunk_finished(run_id, chunk.chunk_no, "failed", Some(&error))
                    .await?;
                return Err(error);
            }
        };
        if let Err(error) = state
            .postgres
            .ensure_run_days(run_id, chunk.chunk_no, &trade_dates)
            .await
        {
            let _ = state
                .postgres
                .set_chunk_finished(run_id, chunk.chunk_no, "failed", Some(&error))
                .await;
            return Err(error);
        }
        let compiled = planner.compile(
            &rule,
            Some(chunk.start_date),
            Some(chunk.end_date),
            u32::try_from(run.top_n).map_err(|error| {
                RearviewError::Validation(format!("run top_n out of range: {error}"))
            })?,
            settings,
        )?;
        let rows = match state
            .clickhouse
            .query_screening_rows(&compiled.sql, &query_id)
            .await
        {
            Ok(rows) => rows,
            Err(error) => {
                state
                    .postgres
                    .set_chunk_finished(run_id, chunk.chunk_no, "failed", Some(&error))
                    .await?;
                return Err(error);
            }
        };
        state
            .postgres
            .set_run_status(run_id, "writing_pool", None)
            .await?;
        if let Err(error) = state
            .postgres
            .write_chunk_rows(run_id, chunk.chunk_no, &rows)
            .await
        {
            state
                .postgres
                .set_chunk_finished(run_id, chunk.chunk_no, "failed", Some(&error))
                .await?;
            return Err(error);
        }
        if let Err(error) = state
            .postgres
            .finish_chunk_days(run_id, chunk.chunk_no)
            .await
        {
            state
                .postgres
                .set_chunk_finished(run_id, chunk.chunk_no, "failed", Some(&error))
                .await?;
            return Err(error);
        }
        state
            .postgres
            .set_chunk_finished(run_id, chunk.chunk_no, "succeeded", None)
            .await?;
    }

    state.postgres.update_run_summary(run_id).await?;
    state
        .postgres
        .set_run_status(run_id, "succeeded", None)
        .await?;
    Ok(())
}

fn combined_hash(parts: &[String]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn failed_status(error: &RearviewError) -> &'static str {
    match error {
        RearviewError::NotFound(_) => "failed_validation",
        RearviewError::Validation(_) => "failed_validation",
        RearviewError::Planner(_) => "failed_compile",
        RearviewError::ClickHouse(_) | RearviewError::Http(_) => "failed_clickhouse",
        RearviewError::Nats(_) => "failed_write",
        RearviewError::Postgres(_) => "failed_write",
        RearviewError::Config(_)
        | RearviewError::Io(_)
        | RearviewError::Json(_)
        | RearviewError::Yaml(_) => "failed_write",
        RearviewError::MetricCatalog(_) => "failed_validation",
    }
}
