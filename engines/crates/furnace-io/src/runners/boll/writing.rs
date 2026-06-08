use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::BollRunRequest;
use crate::rows::BollResultRow;
use crate::runners::shared::{
    ensure_append_latest_is_safe as ensure_append_latest_is_safe_for_table, insert_rowbinary_rows,
};

pub(super) fn ensure_boll_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
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

pub(super) fn insert_boll_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[BollResultRow],
    batch_size: usize,
) -> Result<(), FurnaceIoError> {
    let insert_sql = format!(
        "\
INSERT INTO {table}
(
    security_code,
    trade_date,
    boll_mid_10_1p5,
    boll_up_10_1p5,
    boll_dn_10_1p5,
    boll_mid_20_2,
    boll_up_20_2,
    boll_dn_20_2,
    boll_mid_50_2p5,
    boll_up_50_2p5,
    boll_dn_50_2p5
)
FORMAT RowBinary"
    );
    insert_rowbinary_rows(
        executor,
        &insert_sql,
        rows,
        batch_size,
        105,
        |row, bytes| row.write_row_binary(bytes),
    )
}
