//! DDL for portfolio calculation outputs in the `fleur_calculation` database.

/// SQL to create the calculation database.
pub fn create_database_sql(database: &str) -> String {
    format!("CREATE DATABASE IF NOT EXISTS {database}")
}

/// SQL to create the worker-authored portfolio performance metric table.
pub fn create_portfolio_performance_metric_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.calc_portfolio_performance_metric
(
    portfolio_run_id          String,
    result_attempt_id         String,
    security_code             LowCardinality(String),
    window_key                LowCardinality(String),
    window_start              Nullable(Date),
    window_end                Nullable(Date),
    config_hash               String,
    metric_status             LowCardinality(String),
    observation_count         UInt32,
    holding_period_return     Nullable(Float64),
    annualized_return         Nullable(Float64),
    annualized_volatility     Nullable(Float64),
    max_drawdown              Nullable(Float64),
    calmar_ratio              Nullable(Float64),
    downside_deviation        Nullable(Float64),
    sortino_ratio             Nullable(Float64),
    sharpe_ratio              Nullable(Float64),
    information_ratio         Nullable(Float64),
    beta                      Nullable(Float64),
    alpha                     Nullable(Float64),
    treynor_ratio             Nullable(Float64),
    computed_at               DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (portfolio_run_id, result_attempt_id, security_code, window_key)
"#
    )
}

/// SQL to create metric-level status rows for portfolio performance metrics.
pub fn create_portfolio_performance_metric_status_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.calc_portfolio_performance_metric_status
(
    portfolio_run_id          String,
    result_attempt_id         String,
    security_code             LowCardinality(String),
    window_key                LowCardinality(String),
    metric_name               LowCardinality(String),
    metric_status             LowCardinality(String),
    reason_code               LowCardinality(String),
    computed_at               DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (portfolio_run_id, result_attempt_id, security_code, window_key, metric_name)
"#
    )
}

/// SQL to create the worker-authored closed trade ledger table.
pub fn create_portfolio_closed_trade_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.calc_portfolio_closed_trade
(
    portfolio_run_id       String,
    result_attempt_id      String,
    closed_trade_id        String,
    closed_trade_seq       UInt32,
    position_lot_id        String,
    entry_trade_seq        UInt32,
    exit_trade_seq         UInt32,
    security_code          LowCardinality(String),
    entry_date             Date,
    exit_date              Date,
    quantity               Float64,
    entry_gross_amount     Float64,
    exit_gross_amount      Float64,
    entry_fee              Float64,
    exit_fee               Float64,
    realized_pnl           Float64,
    holding_days           UInt32,
    exit_reason            LowCardinality(String),
    created_at             DateTime DEFAULT now()
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(exit_date)
ORDER BY (portfolio_run_id, result_attempt_id, exit_date, security_code, closed_trade_seq)
"#
    )
}

/// SQL to create the worker-authored trade quality metric table.
pub fn create_portfolio_trade_metric_table_sql(database: &str) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS {database}.calc_portfolio_trade_metric
(
    portfolio_run_id          String,
    result_attempt_id         String,
    window_key                LowCardinality(String),
    window_start              Nullable(Date),
    window_end                Nullable(Date),
    closed_trade_count        UInt32,
    winning_trade_count       UInt32,
    losing_trade_count        UInt32,
    breakeven_trade_count     UInt32,
    win_rate_closed_trades    Nullable(Float64),
    average_win_return        Nullable(Float64),
    average_loss_return       Nullable(Float64),
    profit_loss_ratio         Nullable(Float64),
    average_holding_days      Nullable(Float64),
    largest_win_return        Nullable(Float64),
    largest_loss_return       Nullable(Float64),
    computed_at               DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (portfolio_run_id, result_attempt_id, window_key)
"#
    )
}

/// All calculation table creation SQL statements.
pub fn all_table_sqls(database: &str) -> Vec<String> {
    vec![
        create_portfolio_performance_metric_table_sql(database),
        create_portfolio_performance_metric_status_table_sql(database),
        create_portfolio_closed_trade_table_sql(database),
        create_portfolio_trade_metric_table_sql(database),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculation_database_sql_uses_if_not_exists() {
        let sql = create_database_sql("fleur_calculation");
        assert!(sql.contains("CREATE DATABASE IF NOT EXISTS fleur_calculation"));
    }

    #[test]
    fn all_table_sqls_covers_portfolio_calculation_outputs() {
        let sqls = all_table_sqls("fleur_calculation");
        assert_eq!(sqls.len(), 4);
        assert!(
            sqls.iter()
                .any(|sql| sql.contains("calc_portfolio_performance_metric"))
        );
        assert!(
            sqls.iter()
                .any(|sql| sql.contains("calc_portfolio_performance_metric_status"))
        );
        assert!(
            sqls.iter()
                .any(|sql| sql.contains("calc_portfolio_closed_trade"))
        );
        assert!(
            sqls.iter()
                .any(|sql| sql.contains("calc_portfolio_trade_metric"))
        );
    }

    #[test]
    fn closed_trade_ledger_is_month_partitioned_by_exit_date() {
        let sql = create_portfolio_closed_trade_table_sql("db");
        assert!(sql.contains("PARTITION BY toYYYYMM(exit_date)"));
        assert!(sql.contains("closed_trade_id"));
        assert!(sql.contains("entry_gross_amount"));
        assert!(sql.contains("exit_gross_amount"));
        assert!(sql.contains("LowCardinality(String)"));
    }

    #[test]
    fn performance_status_table_has_metric_level_key() {
        let sql = create_portfolio_performance_metric_status_table_sql("db");
        assert!(sql.contains(
            "ORDER BY (portfolio_run_id, result_attempt_id, security_code, window_key, metric_name)"
        ));
    }
}
