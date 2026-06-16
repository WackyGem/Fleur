use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::clickhouse::ScreeningRow;
use crate::domain::{MetricCatalog, RuleDependencySnapshot, RuleHash, RuleVersionSpec};
use crate::error::{RearviewError, RearviewResult};
use crate::portfolio::{
    BuySignalInput, OrderReason, OrderSide, OrderStatus, PortfolioEventType,
    PortfolioSimulationOutput, TargetReason,
};

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
        if version.as_deref() != Some("0003_rearview_portfolio") {
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
            "market_fee_template",
            "virtual_account_template",
            "portfolio_run",
            "portfolio_task_outbox",
            "portfolio_target",
            "portfolio_order",
            "portfolio_trade",
            "portfolio_position_day",
            "portfolio_nav",
            "portfolio_event",
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

    pub async fn get_default_market_fee_template(
        &self,
        market: &str,
    ) -> RearviewResult<MarketFeeTemplateRecord> {
        let row = sqlx::query(
            r#"
            select market_fee_template_id, market, name, currency, fee_profile,
                   slippage_profile, is_default, status
            from market_fee_template
            where market = $1 and is_default = true and status = 'active'
            "#,
        )
        .bind(market)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "active default market template not found: {market}"
            ))
        })?;
        Ok(market_fee_template_from_row(&row))
    }

    pub async fn create_account_template(
        &self,
        input: NewAccountTemplate,
    ) -> RearviewResult<AccountTemplateRecord> {
        if input.initial_cash <= 0.0 {
            return Err(RearviewError::Validation(
                "initial_cash must be greater than 0".to_string(),
            ));
        }
        let account_template_id = Uuid::new_v4().to_string();
        let mut transaction = self.pool.begin().await?;
        if input.is_default {
            sqlx::query(
                r#"
                update virtual_account_template
                set is_default = false,
                    updated_at = now()
                where rule_set_id = $1
                  and status = 'active'
                "#,
            )
            .bind(&input.rule_set_id)
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query(
            r#"
            insert into virtual_account_template (
                account_template_id,
                rule_set_id,
                market_fee_template_id,
                name,
                initial_cash,
                currency,
                fee_profile,
                slippage_profile,
                rebalance_policy,
                risk_exit_policy,
                is_default,
                status
            )
            values ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9::jsonb, $10::jsonb,
                    $11, 'active')
            "#,
        )
        .bind(&account_template_id)
        .bind(&input.rule_set_id)
        .bind(&input.market_fee_template_id)
        .bind(&input.name)
        .bind(input.initial_cash)
        .bind(&input.currency)
        .bind(&input.fee_profile)
        .bind(&input.slippage_profile)
        .bind(&input.rebalance_policy)
        .bind(&input.risk_exit_policy)
        .bind(input.is_default)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.get_account_template(&account_template_id).await
    }

    pub async fn update_account_template(
        &self,
        input: PatchAccountTemplate,
    ) -> RearviewResult<AccountTemplateRecord> {
        if input
            .initial_cash
            .is_some_and(|initial_cash| initial_cash <= 0.0)
        {
            return Err(RearviewError::Validation(
                "initial_cash must be greater than 0".to_string(),
            ));
        }
        if let Some(status) = input.status.as_deref()
            && !matches!(status, "active" | "archived")
        {
            return Err(RearviewError::Validation(format!(
                "unsupported account template status: {status}"
            )));
        }
        let existing = self
            .get_account_template(&input.account_template_id)
            .await?;
        let mut transaction = self.pool.begin().await?;
        if input.is_default == Some(true) {
            sqlx::query(
                r#"
                update virtual_account_template
                set is_default = false,
                    updated_at = now()
                where rule_set_id = $1
                  and account_template_id <> $2
                  and status = 'active'
                "#,
            )
            .bind(&existing.rule_set_id)
            .bind(&input.account_template_id)
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query(
            r#"
            update virtual_account_template
            set name = coalesce($2, name),
                initial_cash = coalesce($3::numeric, initial_cash),
                currency = coalesce($4, currency),
                fee_profile = coalesce($5::jsonb, fee_profile),
                slippage_profile = coalesce($6::jsonb, slippage_profile),
                rebalance_policy = coalesce($7::jsonb, rebalance_policy),
                risk_exit_policy = coalesce($8::jsonb, risk_exit_policy),
                is_default = coalesce($9, is_default),
                status = coalesce($10, status),
                updated_at = now()
            where account_template_id = $1
            "#,
        )
        .bind(&input.account_template_id)
        .bind(&input.name)
        .bind(input.initial_cash)
        .bind(&input.currency)
        .bind(&input.fee_profile)
        .bind(&input.slippage_profile)
        .bind(&input.rebalance_policy)
        .bind(&input.risk_exit_policy)
        .bind(input.is_default)
        .bind(&input.status)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.get_account_template(&input.account_template_id).await
    }

    async fn create_default_account_template_for_rule_set(
        &self,
        rule_set_id: &str,
    ) -> RearviewResult<()> {
        let template = self.get_default_market_fee_template("CN_A_SHARE").await?;
        let input = NewAccountTemplate {
            rule_set_id: rule_set_id.to_string(),
            market_fee_template_id: Some(template.market_fee_template_id),
            name: "Default research account".to_string(),
            initial_cash: 1_000_000.0,
            currency: template.currency,
            fee_profile: template.fee_profile,
            slippage_profile: template.slippage_profile,
            rebalance_policy: serde_json::json!({
                "frequency": "signal_day",
                "target_weighting": "equal_weight",
                "max_positions": 10,
                "lot_size": 100,
                "min_trade_lots": 1,
                "cash_reserve_pct": 0,
                "empty_signal_action": "hold"
            }),
            risk_exit_policy: serde_json::json!({
                "trigger_timing": "close_confirm_next_open",
                "exit_rules": []
            }),
            is_default: true,
        };
        self.create_account_template(input).await?;
        Ok(())
    }

    pub async fn get_account_template(
        &self,
        account_template_id: &str,
    ) -> RearviewResult<AccountTemplateRecord> {
        let row = sqlx::query(
            r#"
            select account_template_id, rule_set_id, market_fee_template_id, name,
                   initial_cash::float8 as initial_cash, currency, fee_profile,
                   slippage_profile, rebalance_policy, risk_exit_policy, is_default, status
            from virtual_account_template
            where account_template_id = $1
            "#,
        )
        .bind(account_template_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            RearviewError::Validation(format!("account_template not found: {account_template_id}"))
        })?;
        account_template_from_row(&row)
    }

    pub async fn list_account_templates(
        &self,
        rule_set_id: &str,
    ) -> RearviewResult<Vec<AccountTemplateRecord>> {
        let rows = sqlx::query(
            r#"
            select account_template_id, rule_set_id, market_fee_template_id, name,
                   initial_cash::float8 as initial_cash, currency, fee_profile,
                   slippage_profile, rebalance_policy, risk_exit_policy, is_default, status
            from virtual_account_template
            where rule_set_id = $1 and status = 'active'
            order by is_default desc, created_at asc
            "#,
        )
        .bind(rule_set_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(account_template_from_row).collect()
    }

    pub async fn get_default_account_template(
        &self,
        rule_set_id: &str,
    ) -> RearviewResult<AccountTemplateRecord> {
        let row = sqlx::query(
            r#"
            select account_template_id, rule_set_id, market_fee_template_id, name,
                   initial_cash::float8 as initial_cash, currency, fee_profile,
                   slippage_profile, rebalance_policy, risk_exit_policy, is_default, status
            from virtual_account_template
            where rule_set_id = $1 and is_default = true and status = 'active'
            "#,
        )
        .bind(rule_set_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "active default account template not found for rule_set: {rule_set_id}"
            ))
        })?;
        account_template_from_row(&row)
    }

    pub async fn create_portfolio_run(
        &self,
        input: NewPortfolioRun,
    ) -> RearviewResult<PortfolioRunRecord> {
        let source_run = self.get_run(&input.source_run_id).await?;
        if source_run.status != "succeeded" {
            return Err(RearviewError::Validation(format!(
                "source run must be succeeded before portfolio simulation: {}",
                source_run.status
            )));
        }
        let rule_set_id = source_run.rule_set_id.as_deref().ok_or_else(|| {
            RearviewError::Validation(format!(
                "source run has no rule_set_id: {}",
                input.source_run_id
            ))
        })?;
        let account_template = match input.account_template_id {
            Some(account_template_id) => self.get_account_template(&account_template_id).await?,
            None => self.get_default_account_template(rule_set_id).await?,
        };
        if account_template.rule_set_id != rule_set_id {
            return Err(RearviewError::Validation(
                "account_template_id does not belong to source run rule_set".to_string(),
            ));
        }

        let portfolio_run_id = Uuid::new_v4().to_string();
        let outbox_id = Uuid::new_v4().to_string();
        let account_snapshot = serde_json::json!({
            "account_template_id": account_template.account_template_id,
            "market_fee_template_id": account_template.market_fee_template_id,
            "initial_cash": account_template.initial_cash,
            "currency": account_template.currency
        });
        let execution_snapshot = serde_json::json!({
            "price_basis": "backward_adjusted",
            "fee_profile": account_template.fee_profile,
            "slippage_profile": account_template.slippage_profile,
            "rebalance_policy": account_template.rebalance_policy,
            "risk_exit_policy": account_template.risk_exit_policy
        });
        let payload = serde_json::json!({
            "portfolio_run_id": portfolio_run_id,
            "source_run_id": source_run.run_id,
            "requested_at": "database_created_at"
        });

        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            r#"
            insert into portfolio_run (
                portfolio_run_id,
                source_run_id,
                rule_version_id,
                rule_hash,
                account_template_id,
                account_snapshot,
                execution_snapshot,
                price_basis,
                start_date,
                end_date,
                status,
                dispatch_status,
                summary
            )
            values ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb, 'backward_adjusted',
                    $8, $9, 'queued', 'pending', '{}'::jsonb)
            "#,
        )
        .bind(&portfolio_run_id)
        .bind(&source_run.run_id)
        .bind(&source_run.rule_version_id)
        .bind(&source_run.rule_hash)
        .bind(&account_template.account_template_id)
        .bind(&account_snapshot)
        .bind(&execution_snapshot)
        .bind(source_run.start_date)
        .bind(source_run.end_date)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            r#"
            insert into portfolio_task_outbox (
                outbox_id,
                portfolio_run_id,
                subject,
                payload,
                status
            )
            values ($1, $2, $3, $4::jsonb, 'pending')
            "#,
        )
        .bind(&outbox_id)
        .bind(&portfolio_run_id)
        .bind(&input.subject)
        .bind(&payload)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.get_portfolio_run(&portfolio_run_id).await
    }

    pub async fn get_portfolio_run(
        &self,
        portfolio_run_id: &str,
    ) -> RearviewResult<PortfolioRunRecord> {
        let row = sqlx::query(
            r#"
            select portfolio_run_id, source_run_id, rule_version_id, rule_hash,
                   account_template_id, account_snapshot, execution_snapshot, price_basis,
                   start_date, end_date, status, dispatch_status, nats_stream_sequence,
                   summary, error_type, error_message
            from portfolio_run
            where portfolio_run_id = $1
            "#,
        )
        .bind(portfolio_run_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            RearviewError::Validation(format!("portfolio_run not found: {portfolio_run_id}"))
        })?;
        Ok(portfolio_run_from_row(&row))
    }

    pub async fn list_portfolio_runs(
        &self,
        filter: PortfolioRunListFilter,
    ) -> RearviewResult<ListResult<PortfolioRunRecord>> {
        let rows = sqlx::query(
            r#"
            select portfolio_run_id, source_run_id, rule_version_id, rule_hash,
                   account_template_id, account_snapshot, execution_snapshot, price_basis,
                   start_date, end_date, status, dispatch_status, nats_stream_sequence,
                   summary, error_type, error_message
            from portfolio_run
            where ($1::text is null or source_run_id = $1)
              and (
                $2::text is null
                or ($2 = 'failed' and status like 'failed_%')
                or status = $2
              )
              and ($3::text is null or dispatch_status = $3)
            order by created_at desc, portfolio_run_id
            limit $4
            offset $5
            "#,
        )
        .bind(filter.source_run_id)
        .bind(filter.status)
        .bind(filter.dispatch_status)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(ListResult::from_rows(
            rows.into_iter()
                .map(|row| portfolio_run_from_row(&row))
                .collect(),
            filter.page,
        ))
    }

    pub async fn list_portfolio_nav(
        &self,
        portfolio_run_id: &str,
    ) -> RearviewResult<Vec<PortfolioNavRecord>> {
        let rows = sqlx::query(
            r#"
            select portfolio_run_id, trade_date, cash_balance::float8 as cash_balance,
                   position_market_value::float8 as position_market_value,
                   total_equity::float8 as total_equity, nav::float8 as nav,
                   daily_return::float8 as daily_return, drawdown::float8 as drawdown,
                   gross_exposure::float8 as gross_exposure, position_count,
                   turnover::float8 as turnover, fee_amount::float8 as fee_amount,
                   warning_count
            from portfolio_nav
            where portfolio_run_id = $1
            order by trade_date
            "#,
        )
        .bind(portfolio_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(portfolio_nav_from_row).collect())
    }

    pub async fn list_portfolio_targets(
        &self,
        filter: PortfolioTargetFilter,
    ) -> RearviewResult<ListResult<PortfolioTargetRecord>> {
        let rows = sqlx::query(
            r#"
            select portfolio_run_id, signal_date, execution_date, security_code,
                   source_rank, source_score::float8 as source_score,
                   target_weight::float8 as target_weight,
                   target_amount::float8 as target_amount,
                   target_quantity::float8 as target_quantity, target_reason
            from portfolio_target
            where portfolio_run_id = $1
              and ($2::date is null or signal_date = $2)
            order by signal_date, source_rank nulls last, security_code
            limit $3
            offset $4
            "#,
        )
        .bind(filter.portfolio_run_id)
        .bind(filter.signal_date)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(ListResult::from_rows(
            rows.into_iter().map(portfolio_target_from_row).collect(),
            filter.page,
        ))
    }

    pub async fn list_portfolio_orders(
        &self,
        filter: PortfolioOrderFilter,
    ) -> RearviewResult<ListResult<PortfolioOrderRecord>> {
        let rows = sqlx::query(
            r#"
            select portfolio_order_id, portfolio_run_id, order_seq, signal_date,
                   execution_date, security_code, side,
                   order_quantity::float8 as order_quantity,
                   order_amount::float8 as order_amount,
                   reference_price::float8 as reference_price, reason, status, event_ref
            from portfolio_order
            where portfolio_run_id = $1
              and ($2::date is null or execution_date = $2)
              and ($3::text is null or security_code = $3)
            order by execution_date, order_seq
            limit $4
            offset $5
            "#,
        )
        .bind(filter.portfolio_run_id)
        .bind(filter.execution_date)
        .bind(filter.security_code)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(ListResult::from_rows(
            rows.into_iter().map(portfolio_order_from_row).collect(),
            filter.page,
        ))
    }

    pub async fn list_portfolio_trades(
        &self,
        filter: PortfolioTradeFilter,
    ) -> RearviewResult<ListResult<PortfolioTradeRecord>> {
        let rows = sqlx::query(
            r#"
            select portfolio_trade_id, portfolio_run_id, trade_seq, portfolio_order_id,
                   trade_date, signal_date, security_code, side,
                   quantity::float8 as quantity, reference_price::float8 as reference_price,
                   execution_price::float8 as execution_price,
                   gross_amount::float8 as gross_amount, commission::float8 as commission,
                   stamp_duty::float8 as stamp_duty, transfer_fee::float8 as transfer_fee,
                   total_fee::float8 as total_fee, slippage_cost::float8 as slippage_cost,
                   reason
            from portfolio_trade
            where portfolio_run_id = $1
              and ($2::date is null or trade_date = $2)
              and ($3::text is null or security_code = $3)
            order by trade_date, trade_seq
            limit $4
            offset $5
            "#,
        )
        .bind(filter.portfolio_run_id)
        .bind(filter.trade_date)
        .bind(filter.security_code)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(ListResult::from_rows(
            rows.into_iter().map(portfolio_trade_from_row).collect(),
            filter.page,
        ))
    }

    pub async fn list_portfolio_positions(
        &self,
        filter: PortfolioPositionFilter,
    ) -> RearviewResult<ListResult<PortfolioPositionRecord>> {
        let trade_date = match filter.trade_date {
            Some(trade_date) => Some(trade_date),
            None => sqlx::query_scalar(
                "select max(trade_date) from portfolio_position_day where portfolio_run_id = $1",
            )
            .bind(&filter.portfolio_run_id)
            .fetch_one(&self.pool)
            .await?,
        };
        let rows = sqlx::query(
            r#"
            select portfolio_run_id, trade_date, security_code, quantity::float8 as quantity,
                   cost_basis::float8 as cost_basis,
                   average_entry_price::float8 as average_entry_price,
                   close_price::float8 as close_price, market_value::float8 as market_value,
                   unrealized_pnl::float8 as unrealized_pnl,
                   unrealized_return::float8 as unrealized_return,
                   holding_days, is_stale_price
            from portfolio_position_day
            where portfolio_run_id = $1
              and ($2::date is null or trade_date = $2)
              and ($3::text is null or security_code = $3)
            order by trade_date, security_code
            limit $4
            offset $5
            "#,
        )
        .bind(filter.portfolio_run_id)
        .bind(trade_date)
        .bind(filter.security_code)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(ListResult::from_rows(
            rows.into_iter().map(portfolio_position_from_row).collect(),
            filter.page,
        ))
    }

    pub async fn list_portfolio_events(
        &self,
        filter: PortfolioEventFilter,
    ) -> RearviewResult<ListResult<PortfolioEventRecord>> {
        let rows = sqlx::query(
            r#"
            select portfolio_event_id, portfolio_run_id, event_seq, trade_date, security_code,
                   event_type, severity, message, payload
            from portfolio_event
            where portfolio_run_id = $1
              and ($2::date is null or trade_date = $2)
              and ($3::text is null or event_type = $3)
            order by event_seq
            limit $4
            offset $5
            "#,
        )
        .bind(filter.portfolio_run_id)
        .bind(filter.trade_date)
        .bind(filter.event_type)
        .bind(filter.page.fetch_limit())
        .bind(filter.page.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(ListResult::from_rows(
            rows.into_iter().map(portfolio_event_from_row).collect(),
            filter.page,
        ))
    }

    pub async fn list_portfolio_source_signals(
        &self,
        run_id: &str,
    ) -> RearviewResult<Vec<PortfolioSourceSignalRecord>> {
        let rows = sqlx::query(
            r#"
            select run_id, trade_date, security_code, rank, score::float8 as score
            from buy_signal
            where run_id = $1
            order by trade_date, rank, security_code
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| PortfolioSourceSignalRecord {
                run_id: row.get("run_id"),
                trade_date: row.get("trade_date"),
                security_code: row.get("security_code"),
                rank: row.get("rank"),
                score: row.get("score"),
            })
            .collect())
    }

    pub async fn write_portfolio_results(
        &self,
        portfolio_run_id: &str,
        output: &PortfolioSimulationOutput,
    ) -> RearviewResult<()> {
        let mut transaction = self.pool.begin().await?;
        for table in [
            "portfolio_event",
            "portfolio_nav",
            "portfolio_position_day",
            "portfolio_trade",
            "portfolio_order",
            "portfolio_target",
        ] {
            let sql = format!("delete from {table} where portfolio_run_id = $1");
            sqlx::query(&sql)
                .bind(portfolio_run_id)
                .execute(&mut *transaction)
                .await?;
        }

        for target in &output.targets {
            sqlx::query(
                r#"
                insert into portfolio_target (
                    portfolio_run_id, signal_date, execution_date, security_code, source_rank,
                    source_score, target_weight, target_amount, target_quantity, target_reason
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(portfolio_run_id)
            .bind(target.signal_date)
            .bind(target.execution_date)
            .bind(&target.security_code)
            .bind(i32::try_from(target.source_rank).map_err(|error| {
                RearviewError::Validation(format!("source_rank is out of range: {error}"))
            })?)
            .bind(target.source_score)
            .bind(target.target_weight)
            .bind(target.target_amount)
            .bind(target.target_quantity)
            .bind(target_reason_str(target.target_reason))
            .execute(&mut *transaction)
            .await?;
        }

        let mut order_ids = std::collections::BTreeMap::new();
        for order in &output.orders {
            let portfolio_order_id = Uuid::new_v4().to_string();
            order_ids.insert(order.order_seq, portfolio_order_id.clone());
            sqlx::query(
                r#"
                insert into portfolio_order (
                    portfolio_order_id, portfolio_run_id, order_seq, signal_date,
                    execution_date, security_code, side, order_quantity, order_amount,
                    reference_price, reason, status
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                "#,
            )
            .bind(&portfolio_order_id)
            .bind(portfolio_run_id)
            .bind(i32::try_from(order.order_seq).map_err(|error| {
                RearviewError::Validation(format!("order_seq is out of range: {error}"))
            })?)
            .bind(order.signal_date)
            .bind(order.execution_date)
            .bind(&order.security_code)
            .bind(order_side_str(order.side))
            .bind(order.order_quantity)
            .bind(order.order_amount)
            .bind(order.reference_price)
            .bind(order_reason_str(order.reason))
            .bind(order_status_str(order.status))
            .execute(&mut *transaction)
            .await?;
        }

        for trade in &output.trades {
            sqlx::query(
                r#"
                insert into portfolio_trade (
                    portfolio_trade_id, portfolio_run_id, trade_seq, portfolio_order_id,
                    trade_date, signal_date, security_code, side, quantity, reference_price,
                    execution_price, gross_amount, commission, stamp_duty, transfer_fee,
                    total_fee, slippage_cost, reason
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                        $16, $17, $18)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(portfolio_run_id)
            .bind(i32::try_from(trade.trade_seq).map_err(|error| {
                RearviewError::Validation(format!("trade_seq is out of range: {error}"))
            })?)
            .bind(order_ids.get(&trade.order_seq))
            .bind(trade.trade_date)
            .bind(trade.signal_date)
            .bind(&trade.security_code)
            .bind(order_side_str(trade.side))
            .bind(trade.quantity)
            .bind(trade.reference_price)
            .bind(trade.execution_price)
            .bind(trade.gross_amount)
            .bind(trade.commission)
            .bind(trade.stamp_duty)
            .bind(trade.transfer_fee)
            .bind(trade.total_fee)
            .bind(trade.slippage_cost)
            .bind(order_reason_str(trade.reason))
            .execute(&mut *transaction)
            .await?;
        }

        for position in &output.positions {
            sqlx::query(
                r#"
                insert into portfolio_position_day (
                    portfolio_run_id, trade_date, security_code, quantity, cost_basis,
                    average_entry_price, close_price, market_value, unrealized_pnl,
                    unrealized_return, holding_days, is_stale_price
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                "#,
            )
            .bind(portfolio_run_id)
            .bind(position.trade_date)
            .bind(&position.security_code)
            .bind(position.quantity)
            .bind(position.cost_basis)
            .bind(position.average_entry_price)
            .bind(position.close_price)
            .bind(position.market_value)
            .bind(position.unrealized_pnl)
            .bind(position.unrealized_return)
            .bind(i32::try_from(position.holding_days).map_err(|error| {
                RearviewError::Validation(format!("holding_days is out of range: {error}"))
            })?)
            .bind(position.is_stale_price)
            .execute(&mut *transaction)
            .await?;
        }

        for nav in &output.nav {
            sqlx::query(
                r#"
                insert into portfolio_nav (
                    portfolio_run_id, trade_date, cash_balance, position_market_value,
                    total_equity, nav, daily_return, drawdown, gross_exposure, position_count,
                    turnover, fee_amount, warning_count
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                "#,
            )
            .bind(portfolio_run_id)
            .bind(nav.trade_date)
            .bind(nav.cash_balance)
            .bind(nav.position_market_value)
            .bind(nav.total_equity)
            .bind(nav.nav)
            .bind(nav.daily_return)
            .bind(nav.drawdown)
            .bind(nav.gross_exposure)
            .bind(i32::try_from(nav.position_count).map_err(|error| {
                RearviewError::Validation(format!("position_count is out of range: {error}"))
            })?)
            .bind(nav.turnover)
            .bind(nav.fee_amount)
            .bind(i32::try_from(nav.warning_count).map_err(|error| {
                RearviewError::Validation(format!("warning_count is out of range: {error}"))
            })?)
            .execute(&mut *transaction)
            .await?;
        }

        for event in &output.events {
            sqlx::query(
                r#"
                insert into portfolio_event (
                    portfolio_event_id, portfolio_run_id, event_seq, trade_date, security_code,
                    event_type, severity, message, payload
                )
                values ($1, $2, $3, $4, $5, $6, 'warning', $7, '{}'::jsonb)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(portfolio_run_id)
            .bind(i32::try_from(event.event_seq).map_err(|error| {
                RearviewError::Validation(format!("event_seq is out of range: {error}"))
            })?)
            .bind(event.trade_date)
            .bind(&event.security_code)
            .bind(portfolio_event_type_str(event.event_type))
            .bind(&event.message)
            .execute(&mut *transaction)
            .await?;
        }

        let summary = serde_json::to_value(&output.summary)?;
        sqlx::query(
            r#"
            update portfolio_run
            set status = 'succeeded',
                summary = $2::jsonb,
                error_type = null,
                error_message = null,
                completed_at = now(),
                updated_at = now()
            where portfolio_run_id = $1
            "#,
        )
        .bind(portfolio_run_id)
        .bind(summary)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn set_portfolio_run_status(
        &self,
        portfolio_run_id: &str,
        status: &str,
        error: Option<&RearviewError>,
    ) -> RearviewResult<()> {
        let (error_type, error_message) = match error {
            Some(error) => (Some(error.error_type()), Some(error.to_string())),
            None => (None, None),
        };
        sqlx::query(
            r#"
            update portfolio_run
            set status = $2,
                error_type = $3,
                error_message = $4,
                completed_at = case
                    when $2 in ('succeeded', 'failed_validation', 'failed_market_data',
                                'failed_simulation', 'failed_write', 'cancelled')
                    then now()
                    else completed_at
                end,
                updated_at = now()
            where portfolio_run_id = $1
            "#,
        )
        .bind(portfolio_run_id)
        .bind(status)
        .bind(error_type)
        .bind(error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn claim_portfolio_run_for_calculation(
        &self,
        portfolio_run_id: &str,
    ) -> RearviewResult<Option<PortfolioRunRecord>> {
        let row = sqlx::query(
            r#"
            update portfolio_run
            set status = 'calculating_nav',
                error_type = null,
                error_message = null,
                updated_at = now()
            where portfolio_run_id = $1
              and status in ('created', 'dispatching', 'queued', 'validating',
                             'loading_signals', 'building_targets', 'calculating_nav',
                             'writing_results')
            returning portfolio_run_id, source_run_id, rule_version_id, rule_hash,
                      account_template_id, account_snapshot, execution_snapshot, price_basis,
                      start_date, end_date, status, dispatch_status, nats_stream_sequence,
                      summary, error_type, error_message
            "#,
        )
        .bind(portfolio_run_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| portfolio_run_from_row(&row)))
    }

    pub async fn list_pending_portfolio_outbox(
        &self,
        limit: i64,
    ) -> RearviewResult<Vec<PortfolioOutboxRecord>> {
        let rows = sqlx::query(
            r#"
            select o.outbox_id, o.portfolio_run_id, r.source_run_id, o.subject, o.payload,
                   o.status, o.attempt_count
            from portfolio_task_outbox o
            join portfolio_run r on r.portfolio_run_id = o.portfolio_run_id
            where o.status in ('pending', 'failed')
            order by o.created_at, o.outbox_id
            limit $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| PortfolioOutboxRecord {
                outbox_id: row.get("outbox_id"),
                portfolio_run_id: row.get("portfolio_run_id"),
                source_run_id: row.get("source_run_id"),
                subject: row.get("subject"),
                payload: row.get("payload"),
                status: row.get("status"),
                attempt_count: row.get("attempt_count"),
            })
            .collect())
    }

    pub async fn mark_portfolio_outbox_published(
        &self,
        outbox_id: &str,
        portfolio_run_id: &str,
        stream_sequence: i64,
    ) -> RearviewResult<()> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            r#"
            update portfolio_task_outbox
            set status = 'published',
                nats_stream_sequence = $2,
                published_at = now(),
                updated_at = now()
            where outbox_id = $1
            "#,
        )
        .bind(outbox_id)
        .bind(stream_sequence)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            update portfolio_run
            set dispatch_status = 'published',
                nats_stream_sequence = $2,
                updated_at = now()
            where portfolio_run_id = $1
            "#,
        )
        .bind(portfolio_run_id)
        .bind(stream_sequence)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn mark_portfolio_outbox_failed(
        &self,
        outbox_id: &str,
        portfolio_run_id: &str,
        error_message: &str,
    ) -> RearviewResult<()> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            r#"
            update portfolio_task_outbox
            set status = 'failed',
                attempt_count = attempt_count + 1,
                last_error = $2,
                updated_at = now()
            where outbox_id = $1
            "#,
        )
        .bind(outbox_id)
        .bind(error_message)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            update portfolio_run
            set dispatch_status = 'publish_failed',
                updated_at = now()
            where portfolio_run_id = $1
            "#,
        )
        .bind(portfolio_run_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
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
        self.create_default_account_template_for_rule_set(&rule_set_id)
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
pub struct PortfolioRunListFilter {
    pub source_run_id: Option<String>,
    pub status: Option<String>,
    pub dispatch_status: Option<String>,
    pub page: Page,
}

#[derive(Debug, Clone)]
pub struct PortfolioTargetFilter {
    pub portfolio_run_id: String,
    pub signal_date: Option<NaiveDate>,
    pub page: Page,
}

#[derive(Debug, Clone)]
pub struct PortfolioOrderFilter {
    pub portfolio_run_id: String,
    pub execution_date: Option<NaiveDate>,
    pub security_code: Option<String>,
    pub page: Page,
}

#[derive(Debug, Clone)]
pub struct PortfolioTradeFilter {
    pub portfolio_run_id: String,
    pub trade_date: Option<NaiveDate>,
    pub security_code: Option<String>,
    pub page: Page,
}

#[derive(Debug, Clone)]
pub struct PortfolioPositionFilter {
    pub portfolio_run_id: String,
    pub trade_date: Option<NaiveDate>,
    pub security_code: Option<String>,
    pub page: Page,
}

#[derive(Debug, Clone)]
pub struct PortfolioEventFilter {
    pub portfolio_run_id: String,
    pub trade_date: Option<NaiveDate>,
    pub event_type: Option<String>,
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

#[derive(Debug, Clone)]
pub struct NewPortfolioRun {
    pub source_run_id: String,
    pub account_template_id: Option<String>,
    pub subject: String,
}

#[derive(Debug, Clone)]
pub struct NewAccountTemplate {
    pub rule_set_id: String,
    pub market_fee_template_id: Option<String>,
    pub name: String,
    pub initial_cash: f64,
    pub currency: String,
    pub fee_profile: Value,
    pub slippage_profile: Value,
    pub rebalance_policy: Value,
    pub risk_exit_policy: Value,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct PatchAccountTemplate {
    pub account_template_id: String,
    pub name: Option<String>,
    pub initial_cash: Option<f64>,
    pub currency: Option<String>,
    pub fee_profile: Option<Value>,
    pub slippage_profile: Option<Value>,
    pub rebalance_policy: Option<Value>,
    pub risk_exit_policy: Option<Value>,
    pub is_default: Option<bool>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketFeeTemplateRecord {
    pub market_fee_template_id: String,
    pub market: String,
    pub name: String,
    pub currency: String,
    pub fee_profile: Value,
    pub slippage_profile: Value,
    pub is_default: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountTemplateRecord {
    pub account_template_id: String,
    pub rule_set_id: String,
    pub market_fee_template_id: Option<String>,
    pub name: String,
    pub initial_cash: f64,
    pub currency: String,
    pub fee_profile: Value,
    pub slippage_profile: Value,
    pub rebalance_policy: Value,
    pub risk_exit_policy: Value,
    pub is_default: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioRunRecord {
    pub portfolio_run_id: String,
    pub source_run_id: String,
    pub rule_version_id: String,
    pub rule_hash: String,
    pub account_template_id: Option<String>,
    pub account_snapshot: Value,
    pub execution_snapshot: Value,
    pub price_basis: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub dispatch_status: String,
    pub nats_stream_sequence: Option<i64>,
    pub summary: Value,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioTargetRecord {
    pub portfolio_run_id: String,
    pub signal_date: NaiveDate,
    pub execution_date: NaiveDate,
    pub security_code: String,
    pub source_rank: Option<i32>,
    pub source_score: Option<f64>,
    pub target_weight: f64,
    pub target_amount: f64,
    pub target_quantity: Option<f64>,
    pub target_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioOrderRecord {
    pub portfolio_order_id: String,
    pub portfolio_run_id: String,
    pub order_seq: i32,
    pub signal_date: Option<NaiveDate>,
    pub execution_date: NaiveDate,
    pub security_code: String,
    pub side: String,
    pub order_quantity: f64,
    pub order_amount: f64,
    pub reference_price: Option<f64>,
    pub reason: String,
    pub status: String,
    pub event_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioTradeRecord {
    pub portfolio_trade_id: String,
    pub portfolio_run_id: String,
    pub trade_seq: i32,
    pub portfolio_order_id: Option<String>,
    pub trade_date: NaiveDate,
    pub signal_date: Option<NaiveDate>,
    pub security_code: String,
    pub side: String,
    pub quantity: f64,
    pub reference_price: f64,
    pub execution_price: f64,
    pub gross_amount: f64,
    pub commission: f64,
    pub stamp_duty: f64,
    pub transfer_fee: f64,
    pub total_fee: f64,
    pub slippage_cost: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioPositionRecord {
    pub portfolio_run_id: String,
    pub trade_date: NaiveDate,
    pub security_code: String,
    pub quantity: f64,
    pub cost_basis: f64,
    pub average_entry_price: f64,
    pub close_price: f64,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub unrealized_return: f64,
    pub holding_days: i32,
    pub is_stale_price: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioNavRecord {
    pub portfolio_run_id: String,
    pub trade_date: NaiveDate,
    pub cash_balance: f64,
    pub position_market_value: f64,
    pub total_equity: f64,
    pub nav: f64,
    pub daily_return: Option<f64>,
    pub drawdown: f64,
    pub gross_exposure: f64,
    pub position_count: i32,
    pub turnover: f64,
    pub fee_amount: f64,
    pub warning_count: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioEventRecord {
    pub portfolio_event_id: String,
    pub portfolio_run_id: String,
    pub event_seq: i32,
    pub trade_date: Option<NaiveDate>,
    pub security_code: Option<String>,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioSourceSignalRecord {
    pub run_id: String,
    pub trade_date: NaiveDate,
    pub security_code: String,
    pub rank: i32,
    pub score: f64,
}

impl PortfolioSourceSignalRecord {
    pub fn into_input(self, execution_date: NaiveDate) -> RearviewResult<BuySignalInput> {
        Ok(BuySignalInput {
            signal_date: self.trade_date,
            execution_date,
            security_code: self.security_code,
            rank: u32::try_from(self.rank).map_err(|error| {
                RearviewError::Validation(format!("signal rank is out of range: {error}"))
            })?,
            score: self.score,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioOutboxRecord {
    pub outbox_id: String,
    pub portfolio_run_id: String,
    pub source_run_id: String,
    pub subject: String,
    pub payload: Value,
    pub status: String,
    pub attempt_count: i32,
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

fn market_fee_template_from_row(row: &sqlx::postgres::PgRow) -> MarketFeeTemplateRecord {
    MarketFeeTemplateRecord {
        market_fee_template_id: row.get("market_fee_template_id"),
        market: row.get("market"),
        name: row.get("name"),
        currency: row.get("currency"),
        fee_profile: row.get("fee_profile"),
        slippage_profile: row.get("slippage_profile"),
        is_default: row.get("is_default"),
        status: row.get("status"),
    }
}

fn account_template_from_row(row: &sqlx::postgres::PgRow) -> RearviewResult<AccountTemplateRecord> {
    Ok(AccountTemplateRecord {
        account_template_id: row.get("account_template_id"),
        rule_set_id: row.get("rule_set_id"),
        market_fee_template_id: row.get("market_fee_template_id"),
        name: row.get("name"),
        initial_cash: row.get("initial_cash"),
        currency: row.get("currency"),
        fee_profile: row.get("fee_profile"),
        slippage_profile: row.get("slippage_profile"),
        rebalance_policy: row.get("rebalance_policy"),
        risk_exit_policy: row.get("risk_exit_policy"),
        is_default: row.get("is_default"),
        status: row.get("status"),
    })
}

fn portfolio_run_from_row(row: &sqlx::postgres::PgRow) -> PortfolioRunRecord {
    PortfolioRunRecord {
        portfolio_run_id: row.get("portfolio_run_id"),
        source_run_id: row.get("source_run_id"),
        rule_version_id: row.get("rule_version_id"),
        rule_hash: row.get("rule_hash"),
        account_template_id: row.get("account_template_id"),
        account_snapshot: row.get("account_snapshot"),
        execution_snapshot: row.get("execution_snapshot"),
        price_basis: row.get("price_basis"),
        start_date: row.get("start_date"),
        end_date: row.get("end_date"),
        status: row.get("status"),
        dispatch_status: row.get("dispatch_status"),
        nats_stream_sequence: row.get("nats_stream_sequence"),
        summary: row.get("summary"),
        error_type: row.get("error_type"),
        error_message: row.get("error_message"),
    }
}

fn portfolio_target_from_row(row: sqlx::postgres::PgRow) -> PortfolioTargetRecord {
    PortfolioTargetRecord {
        portfolio_run_id: row.get("portfolio_run_id"),
        signal_date: row.get("signal_date"),
        execution_date: row.get("execution_date"),
        security_code: row.get("security_code"),
        source_rank: row.get("source_rank"),
        source_score: row.get("source_score"),
        target_weight: row.get("target_weight"),
        target_amount: row.get("target_amount"),
        target_quantity: row.get("target_quantity"),
        target_reason: row.get("target_reason"),
    }
}

fn portfolio_order_from_row(row: sqlx::postgres::PgRow) -> PortfolioOrderRecord {
    PortfolioOrderRecord {
        portfolio_order_id: row.get("portfolio_order_id"),
        portfolio_run_id: row.get("portfolio_run_id"),
        order_seq: row.get("order_seq"),
        signal_date: row.get("signal_date"),
        execution_date: row.get("execution_date"),
        security_code: row.get("security_code"),
        side: row.get("side"),
        order_quantity: row.get("order_quantity"),
        order_amount: row.get("order_amount"),
        reference_price: row.get("reference_price"),
        reason: row.get("reason"),
        status: row.get("status"),
        event_ref: row.get("event_ref"),
    }
}

fn portfolio_trade_from_row(row: sqlx::postgres::PgRow) -> PortfolioTradeRecord {
    PortfolioTradeRecord {
        portfolio_trade_id: row.get("portfolio_trade_id"),
        portfolio_run_id: row.get("portfolio_run_id"),
        trade_seq: row.get("trade_seq"),
        portfolio_order_id: row.get("portfolio_order_id"),
        trade_date: row.get("trade_date"),
        signal_date: row.get("signal_date"),
        security_code: row.get("security_code"),
        side: row.get("side"),
        quantity: row.get("quantity"),
        reference_price: row.get("reference_price"),
        execution_price: row.get("execution_price"),
        gross_amount: row.get("gross_amount"),
        commission: row.get("commission"),
        stamp_duty: row.get("stamp_duty"),
        transfer_fee: row.get("transfer_fee"),
        total_fee: row.get("total_fee"),
        slippage_cost: row.get("slippage_cost"),
        reason: row.get("reason"),
    }
}

fn portfolio_position_from_row(row: sqlx::postgres::PgRow) -> PortfolioPositionRecord {
    PortfolioPositionRecord {
        portfolio_run_id: row.get("portfolio_run_id"),
        trade_date: row.get("trade_date"),
        security_code: row.get("security_code"),
        quantity: row.get("quantity"),
        cost_basis: row.get("cost_basis"),
        average_entry_price: row.get("average_entry_price"),
        close_price: row.get("close_price"),
        market_value: row.get("market_value"),
        unrealized_pnl: row.get("unrealized_pnl"),
        unrealized_return: row.get("unrealized_return"),
        holding_days: row.get("holding_days"),
        is_stale_price: row.get("is_stale_price"),
    }
}

fn portfolio_nav_from_row(row: sqlx::postgres::PgRow) -> PortfolioNavRecord {
    PortfolioNavRecord {
        portfolio_run_id: row.get("portfolio_run_id"),
        trade_date: row.get("trade_date"),
        cash_balance: row.get("cash_balance"),
        position_market_value: row.get("position_market_value"),
        total_equity: row.get("total_equity"),
        nav: row.get("nav"),
        daily_return: row.get("daily_return"),
        drawdown: row.get("drawdown"),
        gross_exposure: row.get("gross_exposure"),
        position_count: row.get("position_count"),
        turnover: row.get("turnover"),
        fee_amount: row.get("fee_amount"),
        warning_count: row.get("warning_count"),
    }
}

fn portfolio_event_from_row(row: sqlx::postgres::PgRow) -> PortfolioEventRecord {
    PortfolioEventRecord {
        portfolio_event_id: row.get("portfolio_event_id"),
        portfolio_run_id: row.get("portfolio_run_id"),
        event_seq: row.get("event_seq"),
        trade_date: row.get("trade_date"),
        security_code: row.get("security_code"),
        event_type: row.get("event_type"),
        severity: row.get("severity"),
        message: row.get("message"),
        payload: row.get("payload"),
    }
}

fn target_reason_str(reason: TargetReason) -> &'static str {
    match reason {
        TargetReason::BuySignal => "buy_signal",
    }
}

fn order_side_str(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    }
}

fn order_reason_str(reason: OrderReason) -> &'static str {
    match reason {
        OrderReason::Rebalance => "rebalance",
        OrderReason::FixedStopLoss => "fixed_stop_loss",
        OrderReason::TakeProfit => "take_profit",
        OrderReason::TimeStopLoss => "time_stop_loss",
    }
}

fn order_status_str(status: OrderStatus) -> &'static str {
    match status {
        OrderStatus::Filled => "filled",
        OrderStatus::SkippedPriceMissing => "skipped_price_missing",
        OrderStatus::SkippedCashInsufficient => "skipped_cash_insufficient",
        OrderStatus::SkippedBelowMinLot => "skipped_below_min_lot",
    }
}

fn portfolio_event_type_str(event_type: PortfolioEventType) -> &'static str {
    match event_type {
        PortfolioEventType::PriceMissing => "price_missing",
        PortfolioEventType::CashInsufficientForMinLot => "cash_insufficient_for_min_lot",
        PortfolioEventType::TargetAmountBelowMinLot => "target_amount_below_min_lot",
    }
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
