//! DDL for the `fleur_portfolio` ClickHouse database.
//!
//! All portfolio result fact tables are owned by the Rust worker (not dbt).
//! `ensure_portfolio_schema` is called at worker startup to idempotently
//! create the database and tables via `CREATE DATABASE/TABLE IF NOT EXISTS`.

/// SQL to create the `fleur_portfolio` database.
pub fn create_database_sql(database: &str) -> String {
    format!("CREATE DATABASE IF NOT EXISTS {database}")
}

fn create_family_run_snapshot_table_sql(
    database: &str,
    table: &str,
    run_id_column: &str,
) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.{table}
(
    {run_id_column}       String,
    result_attempt_id      String,
    source_run_id          String,
    rule_version_id        String,
    rule_hash              String,
    account_snapshot       String,
    execution_snapshot     String,
    start_date             Date,
    end_date               Date,
    summary                String,
    created_at             DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY ({run_id_column}, result_attempt_id)
"#
    )
}

fn create_family_nav_daily_table_sql(database: &str, table: &str, run_id_column: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.{table}
(
    {run_id_column}       String,
    result_attempt_id      String,
    trade_date             Date,
    cash_balance           Float64,
    position_market_value  Float64,
    total_equity           Float64,
    nav                    Float64,
    daily_return           Nullable(Float64),
    drawdown               Float64,
    gross_exposure         Float64,
    position_count         UInt32,
    turnover               Float64,
    fee_amount             Float64,
    warning_count          UInt32
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(trade_date)
ORDER BY ({run_id_column}, result_attempt_id, trade_date)
"#
    )
}

fn create_family_position_day_table_sql(
    database: &str,
    table: &str,
    run_id_column: &str,
) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.{table}
(
    {run_id_column}       String,
    result_attempt_id      String,
    trade_date             Date,
    security_code          String,
    quantity               Float64,
    cost_basis             Float64,
    average_entry_price    Float64,
    close_price            Float64,
    market_value           Float64,
    unrealized_pnl         Float64,
    unrealized_return      Float64,
    holding_days           UInt32,
    is_stale_price         Bool
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(trade_date)
ORDER BY ({run_id_column}, result_attempt_id, trade_date, security_code)
"#
    )
}

fn create_family_trade_table_sql(database: &str, table: &str, run_id_column: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.{table}
(
    {run_id_column}       String,
    result_attempt_id      String,
    portfolio_trade_id     String,
    portfolio_order_id     Nullable(String),
    trade_seq              UInt32,
    order_seq              UInt32,
    trade_date             Date,
    signal_date            Nullable(Date),
    security_code          String,
    side                   String,
    quantity               Float64,
    reference_price        Float64,
    execution_price        Float64,
    gross_amount           Float64,
    commission             Float64,
    stamp_duty             Float64,
    transfer_fee           Float64,
    total_fee              Float64,
    slippage_cost          Float64,
    reason                 String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(trade_date)
ORDER BY ({run_id_column}, result_attempt_id, trade_date, security_code)
"#
    )
}

fn create_family_order_table_sql(database: &str, table: &str, run_id_column: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.{table}
(
    {run_id_column}       String,
    result_attempt_id      String,
    portfolio_order_id     String,
    order_seq              UInt32,
    signal_date            Nullable(Date),
    execution_date         Date,
    security_code          String,
    side                   String,
    order_quantity         Float64,
    order_amount           Float64,
    reference_price        Nullable(Float64),
    reason                 String,
    status                 String,
    event_ref              Nullable(String)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(execution_date)
ORDER BY ({run_id_column}, result_attempt_id, execution_date, security_code)
"#
    )
}

fn create_family_target_table_sql(database: &str, table: &str, run_id_column: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.{table}
(
    {run_id_column}       String,
    result_attempt_id      String,
    signal_date            Date,
    execution_date         Date,
    security_code          String,
    source_rank            UInt32,
    source_score           Float64,
    target_weight          Float64,
    target_amount          Float64,
    target_quantity        Float64,
    target_reason          String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(signal_date)
ORDER BY ({run_id_column}, result_attempt_id, signal_date, security_code)
"#
    )
}

fn create_family_event_table_sql(database: &str, table: &str, run_id_column: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.{table}
(
    {run_id_column}       String,
    result_attempt_id      String,
    portfolio_event_id     String,
    event_seq              UInt32,
    trade_date             Date,
    security_code          Nullable(String),
    event_type             String,
    severity               String,
    message                String,
    payload                String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(trade_date)
ORDER BY ({run_id_column}, result_attempt_id, trade_date)
"#
    )
}

fn family_table_sqls(database: &str, prefix: &str, run_id_column: &str) -> Vec<String> {
    vec![
        create_family_run_snapshot_table_sql(
            database,
            &format!("{prefix}_run_snapshot"),
            run_id_column,
        ),
        create_family_target_table_sql(database, &format!("{prefix}_target"), run_id_column),
        create_family_order_table_sql(database, &format!("{prefix}_order"), run_id_column),
        create_family_trade_table_sql(database, &format!("{prefix}_trade"), run_id_column),
        create_family_position_day_table_sql(
            database,
            &format!("{prefix}_position_day"),
            run_id_column,
        ),
        create_family_nav_daily_table_sql(database, &format!("{prefix}_nav_daily"), run_id_column),
        create_family_event_table_sql(database, &format!("{prefix}_event"), run_id_column),
    ]
}

/// Table SQLs for `fleur_backtest.backtest_*` result facts.
pub fn all_backtest_table_sqls(database: &str) -> Vec<String> {
    family_table_sqls(database, "backtest", "strategy_backtest_run_id")
}

/// Table SQLs for `fleur_portfolio.live_*` result facts.
pub fn all_live_table_sqls(database: &str) -> Vec<String> {
    family_table_sqls(database, "live", "strategy_portfolio_daily_run_id")
}

/// SQL to create the `portfolio_run_snapshot` table.
pub fn create_run_snapshot_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.portfolio_run_snapshot
(
    portfolio_run_id       String,
    result_attempt_id      String,
    source_run_id          String,
    rule_version_id        String,
    rule_hash              String,
    account_snapshot       String,
    execution_snapshot     String,
    start_date             Date,
    end_date               Date,
    summary                String,
    created_at             DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (portfolio_run_id, result_attempt_id)
"#
    )
}

/// SQL to create the `portfolio_nav_daily` table.
pub fn create_nav_daily_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.portfolio_nav_daily
(
    portfolio_run_id       String,
    result_attempt_id      String,
    trade_date             Date,
    cash_balance           Float64,
    position_market_value  Float64,
    total_equity           Float64,
    nav                    Float64,
    daily_return           Nullable(Float64),
    drawdown               Float64,
    gross_exposure         Float64,
    position_count         UInt32,
    turnover               Float64,
    fee_amount             Float64,
    warning_count          UInt32
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(trade_date)
ORDER BY (portfolio_run_id, result_attempt_id, trade_date)
"#
    )
}

/// SQL to create the `portfolio_position_day` table.
pub fn create_position_day_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.portfolio_position_day
(
    portfolio_run_id       String,
    result_attempt_id      String,
    trade_date             Date,
    security_code          String,
    quantity               Float64,
    cost_basis             Float64,
    average_entry_price    Float64,
    close_price            Float64,
    market_value           Float64,
    unrealized_pnl         Float64,
    unrealized_return      Float64,
    holding_days           UInt32,
    is_stale_price         Bool
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(trade_date)
ORDER BY (portfolio_run_id, result_attempt_id, trade_date, security_code)
"#
    )
}

/// SQL to create the `portfolio_trade` table.
pub fn create_trade_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.portfolio_trade
(
    portfolio_run_id       String,
    result_attempt_id      String,
    portfolio_trade_id     String,
    portfolio_order_id     Nullable(String),
    trade_seq              UInt32,
    order_seq              UInt32,
    trade_date             Date,
    signal_date            Nullable(Date),
    security_code          String,
    side                   String,
    quantity               Float64,
    reference_price        Float64,
    execution_price        Float64,
    gross_amount           Float64,
    commission             Float64,
    stamp_duty             Float64,
    transfer_fee           Float64,
    total_fee              Float64,
    slippage_cost          Float64,
    reason                 String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(trade_date)
ORDER BY (portfolio_run_id, result_attempt_id, trade_date, security_code)
"#
    )
}

/// SQL to create the `portfolio_order` table.
pub fn create_order_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.portfolio_order
(
    portfolio_run_id       String,
    result_attempt_id      String,
    portfolio_order_id     String,
    order_seq              UInt32,
    signal_date            Nullable(Date),
    execution_date         Date,
    security_code          String,
    side                   String,
    order_quantity         Float64,
    order_amount           Float64,
    reference_price        Nullable(Float64),
    reason                 String,
    status                 String,
    event_ref              Nullable(String)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(execution_date)
ORDER BY (portfolio_run_id, result_attempt_id, execution_date, security_code)
"#
    )
}

/// SQL to create the `portfolio_target` table.
pub fn create_target_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.portfolio_target
(
    portfolio_run_id       String,
    result_attempt_id      String,
    signal_date            Date,
    execution_date         Date,
    security_code          String,
    source_rank            UInt32,
    source_score           Float64,
    target_weight          Float64,
    target_amount          Float64,
    target_quantity        Float64,
    target_reason          String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(signal_date)
ORDER BY (portfolio_run_id, result_attempt_id, signal_date, security_code)
"#
    )
}

/// SQL to create the `portfolio_event` table.
pub fn create_event_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.portfolio_event
(
    portfolio_run_id       String,
    result_attempt_id      String,
    portfolio_event_id     String,
    event_seq              UInt32,
    trade_date             Date,
    security_code          Nullable(String),
    event_type             String,
    severity               String,
    message                String,
    payload                String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(trade_date)
ORDER BY (portfolio_run_id, result_attempt_id, trade_date)
"#
    )
}

/// All table-creation SQL statements, in dependency order.
pub fn all_table_sqls(database: &str) -> Vec<String> {
    vec![
        create_run_snapshot_table_sql(database),
        create_target_table_sql(database),
        create_order_table_sql(database),
        create_trade_table_sql(database),
        create_position_day_table_sql(database),
        create_nav_daily_table_sql(database),
        create_event_table_sql(database),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_sql_uses_if_not_exists() {
        let sql = create_database_sql("fleur_portfolio");
        assert!(sql.contains("CREATE DATABASE IF NOT EXISTS fleur_portfolio"));
    }

    #[test]
    fn all_table_sqls_covers_seven_tables() {
        let sqls = all_table_sqls("fleur_portfolio");
        assert_eq!(sqls.len(), 7);
        for sql in &sqls {
            assert!(sql.contains("CREATE TABLE IF NOT EXISTS"));
            assert!(sql.contains("MergeTree()"));
            assert!(!sql.contains("ReplacingMergeTree"));
            assert!(sql.contains("ORDER BY (portfolio_run_id, result_attempt_id"));
        }
    }

    #[test]
    fn time_series_tables_are_month_partitioned() {
        assert!(create_nav_daily_table_sql("db").contains("PARTITION BY toYYYYMM(trade_date)"));
        assert!(create_position_day_table_sql("db").contains("PARTITION BY toYYYYMM(trade_date)"));
        assert!(create_trade_table_sql("db").contains("PARTITION BY toYYYYMM(trade_date)"));
        assert!(create_event_table_sql("db").contains("PARTITION BY toYYYYMM(trade_date)"));
        assert!(create_order_table_sql("db").contains("PARTITION BY toYYYYMM(execution_date)"));
        assert!(create_target_table_sql("db").contains("PARTITION BY toYYYYMM(signal_date)"));
    }

    #[test]
    fn split_backtest_and_live_tables_use_concrete_run_id_columns() {
        let backtest_sqls = all_backtest_table_sqls("fleur_backtest");
        assert_eq!(backtest_sqls.len(), 7);
        assert!(backtest_sqls.iter().all(|sql| {
            sql.contains("strategy_backtest_run_id")
                && !sql.contains("portfolio_run_id")
                && sql.contains("backtest_")
        }));

        let live_sqls = all_live_table_sqls("fleur_portfolio");
        assert_eq!(live_sqls.len(), 7);
        assert!(live_sqls.iter().all(|sql| {
            sql.contains("strategy_portfolio_daily_run_id")
                && !sql.contains("portfolio_run_id")
                && sql.contains("live_")
        }));
    }

    #[test]
    fn split_time_series_tables_are_month_partitioned() {
        let live_sql = all_live_table_sqls("db").join("\n");
        assert!(live_sql.contains("live_nav_daily"));
        assert!(live_sql.contains("PARTITION BY toYYYYMM(trade_date)"));
        assert!(live_sql.contains("PARTITION BY toYYYYMM(execution_date)"));
        assert!(live_sql.contains("PARTITION BY toYYYYMM(signal_date)"));
    }
}
