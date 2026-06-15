use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::clickhouse::ScreeningRow;
use crate::domain::{MetricCatalog, RuleDependencySnapshot, RuleHash, RuleVersionSpec};
use crate::error::{RearviewError, RearviewResult};

#[derive(Clone)]
pub struct RearviewPg {
    pool: PgPool,
}

impl RearviewPg {
    pub async fn connect(database_url: &str) -> RearviewResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn check_schema_readiness(&self) -> RearviewResult<()> {
        let version: Option<String> =
            sqlx::query_scalar("select version_num from alembic_version limit 1")
                .fetch_optional(&self.pool)
                .await?;
        if version.as_deref() != Some("0002_create_rearview_schema") {
            return Err(RearviewError::Config(format!(
                "rearview schema version is not compatible: {:?}",
                version
            )));
        }
        let expected_tables = [
            "rule_set",
            "rule_version",
            "metric_catalog",
            "run",
            "run_chunk",
            "run_day",
            "pool_member",
            "buy_signal",
        ];
        let rows = sqlx::query(
            r#"
            select table_name
            from information_schema.tables
            where table_schema = 'public'
              and table_name = any($1)
            "#,
        )
        .bind(&expected_tables[..])
        .fetch_all(&self.pool)
        .await?;

        let found = rows
            .iter()
            .map(|row| row.get::<String, _>("table_name"))
            .collect::<std::collections::BTreeSet<_>>();
        let missing = expected_tables
            .iter()
            .filter(|table| !found.contains(**table))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(RearviewError::Config(format!(
                "rearview schema is missing tables: {}",
                missing.join(", ")
            )));
        }
        Ok(())
    }

    pub async fn sync_metric_catalog(&self, catalog: &MetricCatalog) -> RearviewResult<u64> {
        let mut transaction = self.pool.begin().await?;
        let mut written = 0;
        for metric in catalog.iter() {
            let allowed_ops = serde_json::to_value(&metric.allowed_ops)?;
            let result = sqlx::query(
                r#"
                insert into metric_catalog (
                    logical_metric,
                    mart_database,
                    mart_table,
                    column_name,
                    value_kind,
                    allow_filter,
                    allow_scoring,
                    allowed_ops,
                    null_policy,
                    default_output,
                    description,
                    updated_at
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9, $10, $11, now())
                on conflict (logical_metric) do update set
                    mart_database = excluded.mart_database,
                    mart_table = excluded.mart_table,
                    column_name = excluded.column_name,
                    value_kind = excluded.value_kind,
                    allow_filter = excluded.allow_filter,
                    allow_scoring = excluded.allow_scoring,
                    allowed_ops = excluded.allowed_ops,
                    null_policy = excluded.null_policy,
                    default_output = excluded.default_output,
                    description = excluded.description,
                    updated_at = now()
                "#,
            )
            .bind(&metric.logical_metric)
            .bind(&metric.mart_database)
            .bind(&metric.mart_table)
            .bind(&metric.column_name)
            .bind(metric.value_kind.as_str())
            .bind(metric.allow_filter)
            .bind(metric.allow_scoring)
            .bind(allowed_ops)
            .bind(metric.null_policy.as_str())
            .bind(metric.default_output)
            .bind(&metric.description)
            .execute(&mut *transaction)
            .await?;
            written += result.rows_affected();
        }
        transaction.commit().await?;
        Ok(written)
    }

    pub async fn create_rule_set(&self, input: NewRuleSet) -> RearviewResult<RuleSetRecord> {
        let rule_set_id = Uuid::new_v4().to_string();
        let tags = serde_json::to_value(input.tags)?;
        sqlx::query(
            r#"
            insert into rule_set (rule_set_id, name, description, owner, tags)
            values ($1, $2, $3, $4, $5::jsonb)
            "#,
        )
        .bind(&rule_set_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.owner)
        .bind(tags)
        .execute(&self.pool)
        .await?;
        self.get_rule_set(&rule_set_id).await
    }

    pub async fn get_rule_set(&self, rule_set_id: &str) -> RearviewResult<RuleSetRecord> {
        let row = sqlx::query(
            r#"
            select rule_set_id, name, description, owner, status, tags, current_version_id
            from rule_set
            where rule_set_id = $1
            "#,
        )
        .bind(rule_set_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| RearviewError::Validation(format!("rule_set not found: {rule_set_id}")))?;
        rule_set_from_row(&row)
    }

    pub async fn list_rule_sets(
        &self,
        filter: RuleSetListFilter,
    ) -> RearviewResult<ListResult<RuleSetRecord>> {
        let rows = sqlx::query(
            r#"
            select rule_set_id, name, description, owner, status, tags, current_version_id
            from rule_set
            where ($1::text is null or status = $1)
              and (
                $2::text is null
                or rule_set_id ilike '%' || $2 || '%'
                or name ilike '%' || $2 || '%'
                or coalesce(owner, '') ilike '%' || $2 || '%'
              )
            order by updated_at desc, created_at desc, rule_set_id
            limit $3
            offset $4
            "#,
        )
        .bind(filter.status)
        .bind(filter.keyword)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(ListResult::from_rows(
            rows.into_iter()
                .map(|row| rule_set_from_row(&row))
                .collect::<RearviewResult<Vec<_>>>()?,
            filter.page,
        ))
    }

    pub async fn create_rule_version(
        &self,
        input: NewRuleVersion,
    ) -> RearviewResult<RuleVersionRecord> {
        let mut transaction = self.pool.begin().await?;
        let version_no: i32 = sqlx::query_scalar(
            "select coalesce(max(version_no), 0) + 1 from rule_version where rule_set_id = $1",
        )
        .bind(&input.rule_set_id)
        .fetch_one(&mut *transaction)
        .await?;
        let rule_version_id = Uuid::new_v4().to_string();
        let rule_ast = serde_json::to_value(&input.rule)?;
        let universe_snapshot = serde_json::to_value(&input.rule.universe)?;
        let pool_filters = serde_json::to_value(&input.rule.pool_filters)?;
        let scoring = serde_json::to_value(&input.rule.scoring)?;
        let output_metrics = serde_json::to_value(&input.rule.output_metrics)?;
        let dependency_snapshot = serde_json::to_value(&input.dependencies)?;
        sqlx::query(
            r#"
            insert into rule_version (
                rule_version_id,
                rule_set_id,
                version_no,
                status,
                rule_ast,
                universe_snapshot,
                pool_filters,
                scoring,
                top_n_default,
                output_metrics,
                metric_dependency_snapshot,
                rule_hash,
                created_by
            )
            values ($1, $2, $3, 'active', $4::jsonb, $5::jsonb, $6::jsonb, $7::jsonb,
                    $8, $9::jsonb, $10::jsonb, $11, $12)
            "#,
        )
        .bind(&rule_version_id)
        .bind(&input.rule_set_id)
        .bind(version_no)
        .bind(rule_ast)
        .bind(universe_snapshot)
        .bind(pool_filters)
        .bind(scoring)
        .bind(i32::try_from(input.rule.top_n_default).map_err(|error| {
            RearviewError::Validation(format!("top_n_default is out of range: {error}"))
        })?)
        .bind(output_metrics)
        .bind(dependency_snapshot)
        .bind(&input.rule_hash.0)
        .bind(&input.created_by)
        .execute(&mut *transaction)
        .await?;

        if input.activate {
            sqlx::query(
                r#"
                update rule_set
                set current_version_id = $1,
                    status = 'active',
                    updated_at = now()
                where rule_set_id = $2
                "#,
            )
            .bind(&rule_version_id)
            .bind(&input.rule_set_id)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        self.get_rule_version(&rule_version_id).await
    }

    pub async fn get_rule_version(
        &self,
        rule_version_id: &str,
    ) -> RearviewResult<RuleVersionRecord> {
        let row = sqlx::query(
            r#"
            select rule_version_id, rule_set_id, version_no, status, top_n_default, rule_hash
            from rule_version
            where rule_version_id = $1
            "#,
        )
        .bind(rule_version_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            RearviewError::Validation(format!("rule_version not found: {rule_version_id}"))
        })?;
        Ok(RuleVersionRecord {
            rule_version_id: row.get("rule_version_id"),
            rule_set_id: row.get("rule_set_id"),
            version_no: row.get("version_no"),
            status: row.get("status"),
            top_n_default: row.get("top_n_default"),
            rule_hash: row.get("rule_hash"),
        })
    }

    pub async fn list_rule_versions(
        &self,
        filter: RuleVersionListFilter,
    ) -> RearviewResult<ListResult<RuleVersionRecord>> {
        let rows = sqlx::query(
            r#"
            select rule_version_id, rule_set_id, version_no, status, top_n_default, rule_hash
            from rule_version
            where rule_set_id = $1
              and ($2::text is null or status = $2)
            order by version_no desc
            limit $3
            offset $4
            "#,
        )
        .bind(filter.rule_set_id)
        .bind(filter.status)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(ListResult::from_rows(
            rows.into_iter().map(rule_version_from_row).collect(),
            filter.page,
        ))
    }

    pub async fn resolve_current_rule_version(
        &self,
        rule_set_id: &str,
    ) -> RearviewResult<RuleVersionRecord> {
        let row = sqlx::query(
            r#"
            select rv.rule_version_id, rv.rule_set_id, rv.version_no, rv.status,
                   rv.top_n_default, rv.rule_hash
            from rule_set rs
            join rule_version rv on rv.rule_version_id = rs.current_version_id
            where rs.rule_set_id = $1
            "#,
        )
        .bind(rule_set_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            RearviewError::Validation(format!("rule_set has no current version: {rule_set_id}"))
        })?;
        Ok(RuleVersionRecord {
            rule_version_id: row.get("rule_version_id"),
            rule_set_id: row.get("rule_set_id"),
            version_no: row.get("version_no"),
            status: row.get("status"),
            top_n_default: row.get("top_n_default"),
            rule_hash: row.get("rule_hash"),
        })
    }

    pub async fn create_run(
        &self,
        input: NewRun,
        chunk_threshold_days: u32,
    ) -> RearviewResult<RunRecord> {
        if input.start_date > input.end_date {
            return Err(RearviewError::Validation(
                "start_date must be <= end_date".to_string(),
            ));
        }
        let run_id = Uuid::new_v4().to_string();
        let universe_snapshot = input
            .universe_snapshot
            .unwrap_or_else(|| serde_json::json!({"base": "rule_version"}));
        let chunks = plan_date_chunks(input.start_date, input.end_date, chunk_threshold_days)?;
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            r#"
            insert into "run" (
                run_id,
                rule_version_id,
                rule_hash,
                start_date,
                end_date,
                top_n,
                universe_snapshot,
                status
            )
            values ($1, $2, $3, $4, $5, $6, $7::jsonb, 'created')
            "#,
        )
        .bind(&run_id)
        .bind(&input.rule_version.rule_version_id)
        .bind(&input.rule_version.rule_hash)
        .bind(input.start_date)
        .bind(input.end_date)
        .bind(input.top_n.unwrap_or(input.rule_version.top_n_default))
        .bind(universe_snapshot)
        .execute(&mut *transaction)
        .await?;

        for chunk in chunks {
            sqlx::query(
                r#"
                insert into run_chunk (run_id, chunk_no, start_date, end_date, status)
                values ($1, $2, $3, $4, 'created')
                "#,
            )
            .bind(&run_id)
            .bind(chunk.chunk_no)
            .bind(chunk.start_date)
            .bind(chunk.end_date)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        self.get_run(&run_id).await
    }

    pub async fn get_rule_version_spec(
        &self,
        rule_version_id: &str,
    ) -> RearviewResult<RuleVersionSpec> {
        let row = sqlx::query(
            r#"
            select rule_ast
            from rule_version
            where rule_version_id = $1
            "#,
        )
        .bind(rule_version_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            RearviewError::Validation(format!("rule_version not found: {rule_version_id}"))
        })?;
        let value: Value = row.get("rule_ast");
        Ok(serde_json::from_value(value)?)
    }

    pub async fn get_run(&self, run_id: &str) -> RearviewResult<RunRecord> {
        let row = sqlx::query(
            r#"
            select r.run_id, r.rule_version_id, rv.rule_set_id, rs.name as rule_set_name,
                   r.rule_hash, r.start_date, r.end_date, r.top_n, r.status,
                   r.compiled_sql_hash, r.summary, r.error_type, r.error_message
            from "run" r
            left join rule_version rv on rv.rule_version_id = r.rule_version_id
            left join rule_set rs on rs.rule_set_id = rv.rule_set_id
            where r.run_id = $1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| RearviewError::Validation(format!("run not found: {run_id}")))?;
        Ok(run_from_row(&row))
    }

    pub async fn list_runs(&self, filter: RunListFilter) -> RearviewResult<ListResult<RunRecord>> {
        let rows = sqlx::query(
            r#"
            select r.run_id, r.rule_version_id, rv.rule_set_id, rs.name as rule_set_name,
                   r.rule_hash, r.start_date, r.end_date, r.top_n, r.status,
                   r.compiled_sql_hash, r.summary, r.error_type, r.error_message
            from "run" r
            left join rule_version rv on rv.rule_version_id = r.rule_version_id
            left join rule_set rs on rs.rule_set_id = rv.rule_set_id
            where (
                $1::text is null
                or ($1 = 'failed' and r.status like 'failed_%')
                or r.status = $1
              )
              and ($2::text is null or rv.rule_set_id = $2)
              and ($3::date is null or r.start_date >= $3)
              and ($4::date is null or r.end_date <= $4)
              and (
                $5::text is null
                or r.run_id ilike '%' || $5 || '%'
                or r.rule_version_id ilike '%' || $5 || '%'
                or r.rule_hash ilike '%' || $5 || '%'
                or coalesce(rs.name, '') ilike '%' || $5 || '%'
              )
            order by r.created_at desc, r.run_id
            limit $6
            offset $7
            "#,
        )
        .bind(filter.status)
        .bind(filter.rule_set_id)
        .bind(filter.start_date)
        .bind(filter.end_date)
        .bind(filter.keyword)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(ListResult::from_rows(
            rows.into_iter().map(|row| run_from_row(&row)).collect(),
            filter.page,
        ))
    }

    pub async fn list_run_chunks(&self, run_id: &str) -> RearviewResult<Vec<RunChunkRecord>> {
        let rows = sqlx::query(
            r#"
            select run_id, chunk_no, start_date, end_date, status, clickhouse_query_id,
                   elapsed_ms, error_type, error_message
            from run_chunk
            where run_id = $1
            order by chunk_no
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| RunChunkRecord {
                run_id: row.get("run_id"),
                chunk_no: row.get("chunk_no"),
                start_date: row.get("start_date"),
                end_date: row.get("end_date"),
                status: row.get("status"),
                clickhouse_query_id: row.get("clickhouse_query_id"),
                elapsed_ms: row.get("elapsed_ms"),
                error_type: row.get("error_type"),
                error_message: row.get("error_message"),
            })
            .collect())
    }

    pub async fn set_run_status(
        &self,
        run_id: &str,
        status: &str,
        error: Option<&RearviewError>,
    ) -> RearviewResult<()> {
        let (error_type, error_message) = match error {
            Some(error) => (Some(error.error_type()), Some(error.to_string())),
            None => (None, None),
        };
        sqlx::query(
            r#"
            update "run"
            set status = $2,
                error_type = $3,
                error_message = $4,
                started_at = coalesce(started_at, now()),
                completed_at = case
                    when $2 in ('succeeded', 'failed_validation', 'failed_compile',
                                'failed_clickhouse', 'failed_write', 'cancelled')
                    then now()
                    else completed_at
                end
            where run_id = $1
            "#,
        )
        .bind(run_id)
        .bind(status)
        .bind(error_type)
        .bind(error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_run_compiled_sql_hash(
        &self,
        run_id: &str,
        compiled_sql_hash: &str,
    ) -> RearviewResult<()> {
        sqlx::query(
            r#"
            update "run"
            set compiled_sql_hash = $2
            where run_id = $1
            "#,
        )
        .bind(run_id)
        .bind(compiled_sql_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_chunk_running(
        &self,
        run_id: &str,
        chunk_no: i32,
        query_id: &str,
    ) -> RearviewResult<()> {
        sqlx::query(
            r#"
            update run_chunk
            set status = 'running',
                clickhouse_query_id = $3,
                started_at = now()
            where run_id = $1 and chunk_no = $2
            "#,
        )
        .bind(run_id)
        .bind(chunk_no)
        .bind(query_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_chunk_finished(
        &self,
        run_id: &str,
        chunk_no: i32,
        status: &str,
        error: Option<&RearviewError>,
    ) -> RearviewResult<()> {
        let (error_type, error_message) = match error {
            Some(error) => (Some(error.error_type()), Some(error.to_string())),
            None => (None, None),
        };
        sqlx::query(
            r#"
            update run_chunk
            set status = $3,
                error_type = $4,
                error_message = $5,
                completed_at = now(),
                elapsed_ms = greatest(
                    0,
                    floor(extract(epoch from (now() - coalesce(started_at, now()))) * 1000)::bigint
                )
            where run_id = $1 and chunk_no = $2
            "#,
        )
        .bind(run_id)
        .bind(chunk_no)
        .bind(status)
        .bind(error_type)
        .bind(error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn ensure_run_days(
        &self,
        run_id: &str,
        chunk_no: i32,
        trade_dates: &[NaiveDate],
    ) -> RearviewResult<()> {
        let mut transaction = self.pool.begin().await?;
        for trade_date in trade_dates {
            sqlx::query(
                r#"
                insert into run_day (
                    run_id,
                    trade_date,
                    chunk_no,
                    status,
                    pool_count,
                    signal_count
                )
                values ($1, $2, $3, 'created', 0, 0)
                on conflict (run_id, trade_date) do nothing
                "#,
            )
            .bind(run_id)
            .bind(trade_date)
            .bind(chunk_no)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn finish_chunk_days(&self, run_id: &str, chunk_no: i32) -> RearviewResult<()> {
        sqlx::query(
            r#"
            update run_day
            set status = 'succeeded',
                pool_count = coalesce(pool_count, 0),
                signal_count = coalesce(signal_count, 0)
            where run_id = $1 and chunk_no = $2
            "#,
        )
        .bind(run_id)
        .bind(chunk_no)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn write_chunk_rows(
        &self,
        run_id: &str,
        chunk_no: i32,
        rows: &[ScreeningRow],
    ) -> RearviewResult<()> {
        let mut by_day = std::collections::BTreeMap::<NaiveDate, DayCounts>::new();
        let mut transaction = self.pool.begin().await?;
        for row in rows {
            let selected_metrics: Value = serde_json::from_str(&row.selected_metrics)?;
            let raw_values: Value = serde_json::from_str(&row.raw_values)?;
            let score_breakdown_points: Value = serde_json::from_str(&row.score_breakdown)?;
            let score_breakdown = serde_json::json!({
                "points": score_breakdown_points,
                "raw_score": row.raw_score,
                "score": row.score,
                "raw_values": raw_values,
            });
            let counts = by_day.entry(row.trade_date).or_default();
            counts.pool_count += 1;
            if row.is_buy_signal {
                counts.signal_count += 1;
            }

            sqlx::query(
                r#"
                insert into run_day (run_id, trade_date, chunk_no, status)
                values ($1, $2, $3, 'created')
                on conflict (run_id, trade_date) do nothing
                "#,
            )
            .bind(run_id)
            .bind(row.trade_date)
            .bind(chunk_no)
            .execute(&mut *transaction)
            .await?;

            sqlx::query(
                r#"
                insert into pool_member (
                    run_id,
                    trade_date,
                    security_code,
                    score,
                    signal_rank,
                    selected_metrics,
                    filter_snapshot
                )
                values ($1, $2, $3, $4, $5, $6::jsonb, '{}'::jsonb)
                on conflict (run_id, trade_date, security_code) do update set
                    score = excluded.score,
                    signal_rank = excluded.signal_rank,
                    selected_metrics = excluded.selected_metrics
                "#,
            )
            .bind(run_id)
            .bind(row.trade_date)
            .bind(&row.security_code)
            .bind(row.score)
            .bind(i32::try_from(row.signal_rank).map_err(|error| {
                RearviewError::Validation(format!("signal rank out of range: {error}"))
            })?)
            .bind(&selected_metrics)
            .execute(&mut *transaction)
            .await?;

            if row.is_buy_signal {
                sqlx::query(
                    r#"
                    insert into buy_signal (
                        run_id,
                        trade_date,
                        security_code,
                        rank,
                        score,
                        score_breakdown,
                        selected_metrics
                    )
                    values ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb)
                    on conflict (run_id, trade_date, security_code) do update set
                        rank = excluded.rank,
                        score = excluded.score,
                        score_breakdown = excluded.score_breakdown,
                        selected_metrics = excluded.selected_metrics
                    "#,
                )
                .bind(run_id)
                .bind(row.trade_date)
                .bind(&row.security_code)
                .bind(i32::try_from(row.signal_rank).map_err(|error| {
                    RearviewError::Validation(format!("signal rank out of range: {error}"))
                })?)
                .bind(row.score)
                .bind(score_breakdown)
                .bind(selected_metrics)
                .execute(&mut *transaction)
                .await?;
            }
        }

        for (trade_date, counts) in by_day {
            sqlx::query(
                r#"
                update run_day
                set status = 'succeeded',
                    pool_count = $3,
                    signal_count = $4
                where run_id = $1 and trade_date = $2
                "#,
            )
            .bind(run_id)
            .bind(trade_date)
            .bind(counts.pool_count)
            .bind(counts.signal_count)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn update_run_summary(&self, run_id: &str) -> RearviewResult<()> {
        sqlx::query(
            r#"
            update "run"
            set summary = (
                select jsonb_build_object(
                    'day_count', count(*),
                    'pool_count', coalesce(sum(pool_count), 0),
                    'signal_count', coalesce(sum(signal_count), 0)
                )
                from run_day
                where run_id = $1
            )
            where run_id = $1
            "#,
        )
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_run_days(&self, run_id: &str) -> RearviewResult<Vec<RunDayRecord>> {
        let rows = sqlx::query(
            r#"
            select run_id, trade_date, status, universe_count, pool_count, signal_count,
                   error_type, error_message
            from run_day
            where run_id = $1
            order by trade_date
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| RunDayRecord {
                run_id: row.get("run_id"),
                trade_date: row.get("trade_date"),
                status: row.get("status"),
                universe_count: row.get("universe_count"),
                pool_count: row.get("pool_count"),
                signal_count: row.get("signal_count"),
                error_type: row.get("error_type"),
                error_message: row.get("error_message"),
            })
            .collect())
    }

    pub async fn list_pool_members(
        &self,
        run_id: &str,
        filter: ResultRowsFilter,
    ) -> RearviewResult<ListResult<PoolMemberRecord>> {
        let sql = format!(
            r#"
            select run_id, trade_date, security_code, score::float8 as score,
                   signal_rank, selected_metrics, filter_snapshot
            from pool_member
            where run_id = $1
              and trade_date = $2
              and ($3::text is null or security_code ilike '%' || $3 || '%')
            order by {}
            limit $4
            offset $5
            "#,
            filter.sort.pool_order_by()?,
        );
        let rows = sqlx::query(&sql)
            .bind(run_id)
            .bind(filter.trade_date)
            .bind(filter.security_code)
            .bind(filter.page.fetch_limit())
            .bind(filter.page.offset)
            .fetch_all(&self.pool)
            .await?;
        Ok(ListResult::from_rows(
            rows.into_iter().map(pool_member_from_row).collect(),
            filter.page,
        ))
    }

    pub async fn get_pool_member(
        &self,
        run_id: &str,
        trade_date: NaiveDate,
        security_code: &str,
    ) -> RearviewResult<Option<PoolMemberRecord>> {
        let row = sqlx::query(
            r#"
            select run_id, trade_date, security_code, score::float8 as score,
                   signal_rank, selected_metrics, filter_snapshot
            from pool_member
            where run_id = $1
              and trade_date = $2
              and security_code = $3
            "#,
        )
        .bind(run_id)
        .bind(trade_date)
        .bind(security_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(pool_member_from_row))
    }

    pub async fn list_buy_signals(
        &self,
        run_id: &str,
        filter: ResultRowsFilter,
    ) -> RearviewResult<ListResult<BuySignalRecord>> {
        let sql = format!(
            r#"
            select run_id, trade_date, security_code, rank, score::float8 as score,
                   score_breakdown, selected_metrics
            from buy_signal
            where run_id = $1
              and trade_date = $2
              and ($3::text is null or security_code ilike '%' || $3 || '%')
            order by {}
            limit $4
            offset $5
            "#,
            filter.sort.signal_order_by()?,
        );
        let rows = sqlx::query(&sql)
            .bind(run_id)
            .bind(filter.trade_date)
            .bind(filter.security_code)
            .bind(filter.page.fetch_limit())
            .bind(filter.page.offset)
            .fetch_all(&self.pool)
            .await?;
        Ok(ListResult::from_rows(
            rows.into_iter().map(buy_signal_from_row).collect(),
            filter.page,
        ))
    }

    pub async fn get_buy_signal(
        &self,
        run_id: &str,
        trade_date: NaiveDate,
        security_code: &str,
    ) -> RearviewResult<Option<BuySignalRecord>> {
        let row = sqlx::query(
            r#"
            select run_id, trade_date, security_code, rank, score::float8 as score,
                   score_breakdown, selected_metrics
            from buy_signal
            where run_id = $1
              and trade_date = $2
              and security_code = $3
            "#,
        )
        .bind(run_id)
        .bind(trade_date)
        .bind(security_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(buy_signal_from_row))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub limit: i64,
    pub offset: i64,
}

impl Page {
    fn fetch_limit(self) -> i64 {
        self.limit + 1
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ListResult<T> {
    pub items: Vec<T>,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

impl<T> ListResult<T> {
    fn from_rows(mut rows: Vec<T>, page: Page) -> Self {
        let has_more = rows.len() > page.limit as usize;
        if has_more {
            rows.truncate(page.limit as usize);
        }
        Self {
            items: rows,
            limit: page.limit,
            offset: page.offset,
            has_more,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleSetListFilter {
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub page: Page,
}

#[derive(Debug, Clone)]
pub struct RuleVersionListFilter {
    pub rule_set_id: String,
    pub status: Option<String>,
    pub page: Page,
}

#[derive(Debug, Clone)]
pub struct RunListFilter {
    pub status: Option<String>,
    pub rule_set_id: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub keyword: Option<String>,
    pub page: Page,
}

#[derive(Debug, Clone)]
pub struct ResultRowsFilter {
    pub trade_date: NaiveDate,
    pub security_code: Option<String>,
    pub sort: ResultRowsSort,
    pub page: Page,
}

#[derive(Debug, Clone, Copy)]
pub enum ResultRowsSort {
    PoolSignalRankAsc,
    PoolScoreDesc,
    PoolScoreAsc,
    SignalRankAsc,
    SignalScoreDesc,
    SecurityCodeAsc,
}

impl ResultRowsSort {
    fn pool_order_by(self) -> RearviewResult<&'static str> {
        match self {
            Self::PoolSignalRankAsc => Ok("signal_rank nulls last, security_code"),
            Self::PoolScoreDesc => Ok("score desc nulls last, security_code"),
            Self::PoolScoreAsc => Ok("score asc nulls last, security_code"),
            Self::SecurityCodeAsc => Ok("security_code"),
            Self::SignalRankAsc | Self::SignalScoreDesc => Err(RearviewError::Validation(
                "signal sort cannot be used for pool members".to_string(),
            )),
        }
    }

    fn signal_order_by(self) -> RearviewResult<&'static str> {
        match self {
            Self::SignalRankAsc => Ok("rank, security_code"),
            Self::SignalScoreDesc => Ok("score desc, rank"),
            Self::SecurityCodeAsc => Ok("security_code"),
            Self::PoolSignalRankAsc | Self::PoolScoreAsc | Self::PoolScoreDesc => Err(
                RearviewError::Validation("pool sort cannot be used for buy signals".to_string()),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewRuleSet {
    pub name: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NewRuleVersion {
    pub rule_set_id: String,
    pub rule: RuleVersionSpec,
    pub dependencies: RuleDependencySnapshot,
    pub rule_hash: RuleHash,
    pub activate: bool,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewRun {
    pub rule_version: RuleVersionRecord,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub top_n: Option<i32>,
    pub universe_snapshot: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleSetRecord {
    pub rule_set_id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub status: String,
    pub tags: Value,
    pub current_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleVersionRecord {
    pub rule_version_id: String,
    pub rule_set_id: String,
    pub version_no: i32,
    pub status: String,
    pub top_n_default: i32,
    pub rule_hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunRecord {
    pub run_id: String,
    pub rule_version_id: String,
    pub rule_set_id: Option<String>,
    pub rule_set_name: Option<String>,
    pub rule_hash: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub top_n: i32,
    pub status: String,
    pub compiled_sql_hash: Option<String>,
    pub summary: Value,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunChunkRecord {
    pub run_id: String,
    pub chunk_no: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub clickhouse_query_id: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunDayRecord {
    pub run_id: String,
    pub trade_date: NaiveDate,
    pub status: String,
    pub universe_count: Option<i32>,
    pub pool_count: Option<i32>,
    pub signal_count: Option<i32>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PoolMemberRecord {
    pub run_id: String,
    pub trade_date: NaiveDate,
    pub security_code: String,
    pub score: Option<f64>,
    pub signal_rank: Option<i32>,
    pub selected_metrics: Value,
    pub filter_snapshot: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuySignalRecord {
    pub run_id: String,
    pub trade_date: NaiveDate,
    pub security_code: String,
    pub rank: i32,
    pub score: f64,
    pub score_breakdown: Value,
    pub selected_metrics: Value,
}

#[derive(Debug, Default)]
struct DayCounts {
    pool_count: i32,
    signal_count: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PlannedChunk {
    pub chunk_no: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

fn rule_set_from_row(row: &sqlx::postgres::PgRow) -> RearviewResult<RuleSetRecord> {
    Ok(RuleSetRecord {
        rule_set_id: row.get("rule_set_id"),
        name: row.get("name"),
        description: row.get("description"),
        owner: row.get("owner"),
        status: row.get("status"),
        tags: row.get("tags"),
        current_version_id: row.get("current_version_id"),
    })
}

fn rule_version_from_row(row: sqlx::postgres::PgRow) -> RuleVersionRecord {
    RuleVersionRecord {
        rule_version_id: row.get("rule_version_id"),
        rule_set_id: row.get("rule_set_id"),
        version_no: row.get("version_no"),
        status: row.get("status"),
        top_n_default: row.get("top_n_default"),
        rule_hash: row.get("rule_hash"),
    }
}

fn run_from_row(row: &sqlx::postgres::PgRow) -> RunRecord {
    RunRecord {
        run_id: row.get("run_id"),
        rule_version_id: row.get("rule_version_id"),
        rule_set_id: row.get("rule_set_id"),
        rule_set_name: row.get("rule_set_name"),
        rule_hash: row.get("rule_hash"),
        start_date: row.get("start_date"),
        end_date: row.get("end_date"),
        top_n: row.get("top_n"),
        status: row.get("status"),
        compiled_sql_hash: row.get("compiled_sql_hash"),
        summary: row.get("summary"),
        error_type: row.get("error_type"),
        error_message: row.get("error_message"),
    }
}

fn pool_member_from_row(row: sqlx::postgres::PgRow) -> PoolMemberRecord {
    PoolMemberRecord {
        run_id: row.get("run_id"),
        trade_date: row.get("trade_date"),
        security_code: row.get("security_code"),
        score: row.get("score"),
        signal_rank: row.get("signal_rank"),
        selected_metrics: row.get("selected_metrics"),
        filter_snapshot: row.get("filter_snapshot"),
    }
}

fn buy_signal_from_row(row: sqlx::postgres::PgRow) -> BuySignalRecord {
    BuySignalRecord {
        run_id: row.get("run_id"),
        trade_date: row.get("trade_date"),
        security_code: row.get("security_code"),
        rank: row.get("rank"),
        score: row.get("score"),
        score_breakdown: row.get("score_breakdown"),
        selected_metrics: row.get("selected_metrics"),
    }
}

pub fn plan_date_chunks(
    start_date: NaiveDate,
    end_date: NaiveDate,
    threshold_days: u32,
) -> RearviewResult<Vec<PlannedChunk>> {
    let day_count = (end_date - start_date).num_days() + 1;
    if day_count <= i64::from(threshold_days) {
        return Ok(vec![PlannedChunk {
            chunk_no: 0,
            start_date,
            end_date,
        }]);
    }

    let mut chunks = Vec::new();
    let mut chunk_no = 0;
    let mut cursor = start_date;
    while cursor <= end_date {
        let year_end = NaiveDate::from_ymd_opt(cursor.year(), 12, 31).ok_or_else(|| {
            RearviewError::Validation(format!("invalid chunk year: {}", cursor.year()))
        })?;
        let chunk_end = year_end.min(end_date);
        chunks.push(PlannedChunk {
            chunk_no,
            start_date: cursor,
            end_date: chunk_end,
        });
        chunk_no += 1;
        let Some(next) = chunk_end.succ_opt() else {
            break;
        };
        cursor = next;
    }
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_date_range_should_keep_short_range_as_one_chunk() {
        let chunks = plan_date_chunks(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
            90,
        )
        .unwrap();

        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn chunk_date_range_should_split_long_range_by_natural_year() {
        let chunks = plan_date_chunks(
            NaiveDate::from_ymd_opt(2023, 12, 20).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            90,
        )
        .unwrap();

        assert_eq!(chunks.len(), 3);
    }
}
