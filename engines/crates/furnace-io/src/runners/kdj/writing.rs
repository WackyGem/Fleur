use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::KdjRunRequest;
use crate::rows::KdjResultRow;
use crate::runners::shared::{
    ensure_append_latest_is_safe as ensure_append_latest_is_safe_for_table, insert_rowbinary_rows,
};
use crate::schema::DEFAULT_KDJ_OUTPUT_TABLE;

pub(super) fn ensure_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<(), FurnaceIoError> {
    ensure_append_latest_is_safe_for_table(
        executor,
        DEFAULT_KDJ_OUTPUT_TABLE,
        &request.request_from,
        symbols,
        all_symbols_requested,
    )
}

pub(super) fn insert_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[KdjResultRow],
    batch_size: usize,
) -> Result<(), FurnaceIoError> {
    let insert_sql = format!(
        "\
INSERT INTO {table}
(
    security_code,
    trade_date,
    rsv_window,
    k_smoothing,
    d_smoothing,
    rsv,
    k_value,
    d_value,
    j_value
)
FORMAT RowBinary"
    );
    insert_rowbinary_rows(executor, &insert_sql, rows, batch_size, 80, |row, bytes| {
        row.write_row_binary(bytes)
    })
}
