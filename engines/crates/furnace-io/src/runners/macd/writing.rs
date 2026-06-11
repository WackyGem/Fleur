use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::MacdRunRequest;
use crate::rows::MacdResultRow;
use crate::runners::shared::{
    ensure_append_latest_is_safe as ensure_append_latest_is_safe_for_table, insert_rowbinary_rows,
};

pub(super) fn ensure_macd_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
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

pub(super) fn insert_macd_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[MacdResultRow],
    batch_size: usize,
) -> Result<(), FurnaceIoError> {
    let insert_sql = format!(
        "\
INSERT INTO {table}
(
    security_code,
    trade_date,
    ema_fast_state_12,
    ema_slow_state_26,
    macd_dif,
    macd_dea,
    macd_dea_state,
    macd_histogram
)
FORMAT RowBinary"
    );
    insert_rowbinary_rows(executor, &insert_sql, rows, batch_size, 80, |row, bytes| {
        row.write_row_binary(bytes)
    })
}
