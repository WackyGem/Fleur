//! Furnace 的 ClickHouse I/O 边界。
//!
//! 本 crate 负责面向数据库的表名、DDL、SQL 生成、`clickhouse-client` 执行以及运行摘要。
//! 纯指标公式保留在 `furnace-core` 中。

mod clickhouse;
mod error;
mod request;
mod rowbinary;
mod rows;
mod runners;
mod schema;
mod sql;
mod summary;
mod validation;

pub use clickhouse::{ClickHouseCliExecutor, ClickHouseExecutor};
pub use error::FurnaceIoError;
pub use request::{
    BollRunRequest, BollWriteMode, KdjRunRequest, KdjWriteMode, MaRunRequest, MaWriteMode,
    PricePatternRunRequest, PricePatternWriteMode, RsiRunRequest, RsiWriteMode,
};
pub use runners::{run_boll, run_kdj, run_ma, run_price_pattern, run_rsi};
pub use schema::{
    DEFAULT_BOLL_OUTPUT_TABLE, DEFAULT_BOLL_PRICE_COLUMN, DEFAULT_INPUT_TABLE,
    DEFAULT_INSERT_BATCH_SIZE, DEFAULT_KDJ_OUTPUT_TABLE, DEFAULT_MA_OUTPUT_TABLE,
    DEFAULT_MA_PRICE_COLUMN, DEFAULT_MA_VOLUME_COLUMN, DEFAULT_MA_VOLUME_INPUT_TABLE,
    DEFAULT_PRICE_PATTERN_CLOSE_COLUMN, DEFAULT_PRICE_PATTERN_HIGH_COLUMN,
    DEFAULT_PRICE_PATTERN_LOW_COLUMN, DEFAULT_PRICE_PATTERN_OUTPUT_TABLE,
    DEFAULT_PRICE_PATTERN_PREV_CLOSE_COLUMN, DEFAULT_PRICE_PATTERN_STREAK_INPUT_TABLE,
    DEFAULT_PRICE_PATTERN_STRUCTURE_INPUT_TABLE, DEFAULT_RSI_OUTPUT_TABLE,
    DEFAULT_RSI_PRICE_COLUMN, DEFAULT_WARMUP_MULTIPLE, MIN_INSERT_BATCH_SIZE,
    boll_staging_table_name, create_boll_output_table_sql, create_boll_staging_table_sql,
    create_calculation_database_sql, create_kdj_output_table_sql, create_kdj_staging_table_sql,
    create_ma_output_table_sql, create_ma_staging_table_sql, create_price_pattern_output_table_sql,
    create_price_pattern_staging_table_sql, create_rsi_output_table_sql,
    create_rsi_staging_table_sql, drop_boll_staging_table_sql, drop_kdj_staging_table_sql,
    drop_ma_staging_table_sql, drop_price_pattern_staging_table_sql, drop_rsi_staging_table_sql,
    kdj_staging_table_name, ma_staging_table_name, price_pattern_staging_table_name,
    replace_boll_partition_sql, replace_kdj_partition_sql, replace_ma_partition_sql,
    replace_price_pattern_partition_sql, replace_rsi_partition_sql, rsi_staging_table_name,
};
pub use summary::{
    BollRunSummary, KdjRunSummary, MaRunSummary, PartitionReplaceSummary, PerformanceMetrics,
    PricePatternRunSummary, RsiRunSummary, ValidationSummary,
};
