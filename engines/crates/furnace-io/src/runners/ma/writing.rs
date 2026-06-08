use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::MaRunRequest;
use crate::rows::MaResultRow;
use crate::runners::shared::{
    ensure_append_latest_is_safe as ensure_append_latest_is_safe_for_table, insert_rowbinary_rows,
};

pub(super) fn ensure_ma_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
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

pub(super) fn insert_ma_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[MaResultRow],
    batch_size: usize,
) -> Result<(), FurnaceIoError> {
    let insert_sql = format!(
        "\
INSERT INTO {table}
(
    security_code,
    trade_date,
    price_ma_3,
    price_ma_5,
    price_ma_6,
    price_ma_10,
    price_ma_12,
    price_ma_14,
    price_ma_20,
    price_ma_24,
    price_ma_28,
    price_ma_57,
    price_ma_60,
    price_ma_114,
    price_ma_250,
    price_avg_ma_3_6_12_24,
    price_avg_ma_14_28_57_114,
    price_ema1_10_state,
    price_ema2_10,
    price_ema2_10_state,
    volume_ma_5,
    volume_ma_10,
    volume_ma_20,
    volume_ma_60
)
FORMAT RowBinary"
    );
    insert_rowbinary_rows(
        executor,
        &insert_sql,
        rows,
        batch_size,
        170,
        |row, bytes| row.write_row_binary(bytes),
    )
}
