use super::*;
#[test]
fn create_kdj_output_table_sql_uses_year_partition_and_expected_order() {
    let sql = create_kdj_output_table_sql();

    assert!(sql.contains("PARTITION BY toYear(trade_date)"));
    assert!(sql.contains("ORDER BY (trade_date, security_code)"));
}

#[test]
fn kdj_staging_table_name_normalizes_run_id() {
    let table_name = kdj_staging_table_name("RUN/2026-01-01");

    assert_eq!(
        table_name,
        "fleur_calculation.calc_stock_kdj_daily__staging__run_2026_01_01"
    );
}

#[test]
fn replace_kdj_partition_sql_replaces_year_partition_from_staging() {
    let sql = replace_kdj_partition_sql("fleur_calculation.stage", 2026);

    assert_eq!(
        sql,
        "ALTER TABLE fleur_calculation.calc_stock_kdj_daily REPLACE PARTITION 2026 FROM fleur_calculation.stage"
    );
}

#[test]
fn create_ma_output_table_sql_uses_canonical_fields() {
    let sql = create_ma_output_table_sql(DEFAULT_MA_OUTPUT_TABLE);

    assert!(sql.contains("price_ma_57 Nullable(Float64)"));
    assert!(sql.contains("price_avg_ma_14_28_57_114 Nullable(Float64)"));
    assert!(sql.contains("volume_ma_5 Nullable(Float64)"));
    assert!(!sql.contains("ma_47"));
    assert!(!sql.contains("price_ma57"));
    assert!(sql.contains("price_ema1_10_state Nullable(Float64)"));
    assert!(sql.contains("price_ema2_10_state Nullable(Float64)"));
    assert!(sql.contains("ORDER BY (trade_date, security_code)"));
}

#[test]
fn create_rsi_output_table_sql_uses_canonical_fields() {
    let sql = create_rsi_output_table_sql(DEFAULT_RSI_OUTPUT_TABLE);

    assert!(sql.contains("rsi_6 Nullable(Float64)"));
    assert!(sql.contains("rsi_50 Nullable(Float64)"));
    assert!(sql.contains("avg_gain_50_state Nullable(Float64)"));
    assert!(sql.contains("avg_loss_50_state Nullable(Float64)"));
    assert!(sql.contains("ORDER BY (trade_date, security_code)"));
}

#[test]
fn ma_staging_table_name_normalizes_run_id() {
    let table_name = ma_staging_table_name(DEFAULT_MA_OUTPUT_TABLE, "RUN/2026-01-01");

    assert_eq!(
        table_name,
        "fleur_calculation.calc_stock_ma_daily__staging__run_2026_01_01"
    );
}

#[test]
fn replace_ma_partition_sql_uses_configurable_output_table() {
    let sql = replace_ma_partition_sql("db.calc_ma", "db.stage", 2026);

    assert_eq!(
        sql,
        "ALTER TABLE db.calc_ma REPLACE PARTITION 2026 FROM db.stage"
    );
}

#[test]
fn create_boll_output_table_sql_uses_canonical_fields() {
    let sql = create_boll_output_table_sql(DEFAULT_BOLL_OUTPUT_TABLE);

    assert!(sql.contains("boll_mid_10_1p5 Nullable(Float64)"));
    assert!(sql.contains("boll_up_20_2 Nullable(Float64)"));
    assert!(sql.contains("boll_dn_50_2p5 Nullable(Float64)"));
    assert!(!sql.contains("boll_mid_n20_k2"));
    assert!(sql.contains("ORDER BY (trade_date, security_code)"));
}
