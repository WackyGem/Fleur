use std::collections::HashMap;

use furnace_core::{
    BollInput, KdjInput, KdjParams, KdjState, MaInput, MacdInput, PricePatternInput, RsiInput,
};

use super::{run_boll, run_kdj, run_ma, run_macd, run_price_pattern, run_rsi};
use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{
    BollRunRequest, BollWriteMode, KdjRunRequest, KdjWriteMode, MaRunRequest, MaWriteMode,
    MacdRunRequest, MacdWriteMode, PricePatternRunRequest, PricePatternWriteMode, RsiRunRequest,
    RsiWriteMode,
};
use crate::rowbinary::{read_rowbinary_nullable_f64, read_rowbinary_string};
use crate::rows::{
    BollGroupedInput, BollResultRow, KdjGroupedInput, KdjResultRow, MaGroupedInput, MaResultRow,
    MacdGroupedInput, MacdResultRow, PricePatternGroupedInput, PricePatternResultRow,
    RsiGroupedInput, RsiResultRow,
};
use crate::runners::boll::materialize::{
    calculate_boll_grouped_outputs_parallel_with_collection,
    calculate_boll_grouped_outputs_serial_with_collection,
};
use crate::runners::kdj::materialize::{
    calculate_grouped_outputs_parallel, calculate_grouped_outputs_serial,
};
use crate::runners::ma::materialize::{
    calculate_ma_grouped_outputs_parallel_with_collection,
    calculate_ma_grouped_outputs_serial_with_collection,
};
use crate::runners::macd::materialize::{
    calculate_macd_grouped_outputs_parallel_with_collection,
    calculate_macd_grouped_outputs_serial_with_collection,
};
use crate::runners::price_pattern::materialize::{
    calculate_price_pattern_grouped_outputs_parallel_with_collection,
    calculate_price_pattern_grouped_outputs_serial_with_collection,
};
use crate::runners::rsi::materialize::{
    calculate_rsi_grouped_outputs_parallel_with_collection,
    calculate_rsi_grouped_outputs_serial_with_collection,
};
use crate::runners::shared::{
    RetainStagingRows, retain_existing_rows_for_staging, validate_staging,
};
use crate::schema::{
    DEFAULT_BOLL_OUTPUT_TABLE, DEFAULT_KDJ_OUTPUT_TABLE, DEFAULT_MA_OUTPUT_TABLE,
    DEFAULT_MACD_OUTPUT_TABLE, DEFAULT_PRICE_PATTERN_OUTPUT_TABLE, DEFAULT_RSI_OUTPUT_TABLE,
    MIN_INSERT_BATCH_SIZE, create_boll_output_table_sql, create_kdj_output_table_sql,
    create_ma_output_table_sql, create_macd_output_table_sql,
    create_price_pattern_output_table_sql, create_rsi_output_table_sql, kdj_staging_table_name,
    ma_staging_table_name, macd_staging_table_name, replace_kdj_partition_sql,
    replace_ma_partition_sql, replace_macd_partition_sql,
};
use crate::summary::ValidationSummary;

mod boll;
mod fixtures;
mod kdj;
mod ma;
mod macd;
mod price_pattern;
mod rsi;
mod schema;

use fixtures::*;
