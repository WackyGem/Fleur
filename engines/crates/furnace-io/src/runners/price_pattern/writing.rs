use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::PricePatternRunRequest;
use crate::rows::PricePatternResultRow;
use crate::runners::shared::{
    ensure_append_latest_is_safe as ensure_append_latest_is_safe_for_table, insert_rowbinary_rows,
};

pub(super) fn ensure_price_pattern_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &PricePatternRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<(), FurnaceIoError> {
    ensure_append_latest_is_safe_for_table(
        executor,
        &request.output_table,
        &request.request_from,
        symbols,
        all_symbols_requested,
    )
}

pub(super) fn insert_price_pattern_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[PricePatternResultRow],
    batch_size: usize,
) -> Result<(), FurnaceIoError> {
    let insert_sql = format!(
        "\
INSERT INTO {table}
(
    security_code,
    trade_date,
    close_direction,
    close_up_streak_days,
    close_down_streak_days,
    n_structure_20_valid_bars,
    n_structure_20_high_date,
    n_structure_20_high_price,
    n_structure_20_low_date,
    n_structure_20_low_price,
    n_structure_20_second_low_date,
    n_structure_20_second_low_price,
    n_structure_20_second_low_ratio,
    n_structure_20_is_valid
)
FORMAT RowBinary"
    );
    insert_rowbinary_rows(
        executor,
        &insert_sql,
        rows,
        batch_size,
        128,
        |row, bytes| row.write_row_binary(bytes),
    )
}
