use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::RsiRunRequest;
use crate::rows::RsiResultRow;
use crate::runners::shared::{
    ensure_append_latest_is_safe as ensure_append_latest_is_safe_for_table, insert_rowbinary_rows,
};

pub(super) fn ensure_rsi_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
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

pub(super) fn insert_rsi_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[RsiResultRow],
    batch_size: usize,
) -> Result<(), FurnaceIoError> {
    let insert_sql = format!(
        "\
INSERT INTO {table}
(
    security_code,
    trade_date,
    rsi_6,
    rsi_12,
    rsi_14,
    rsi_24,
    rsi_25,
    rsi_50,
    avg_gain_6_state,
    avg_loss_6_state,
    avg_gain_12_state,
    avg_loss_12_state,
    avg_gain_14_state,
    avg_loss_14_state,
    avg_gain_24_state,
    avg_loss_24_state,
    avg_gain_25_state,
    avg_loss_25_state,
    avg_gain_50_state,
    avg_loss_50_state
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
