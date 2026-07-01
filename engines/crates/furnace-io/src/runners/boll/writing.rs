use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::BollRunRequest;
use crate::rows::{BollInsertRow, BollResultRow};
use crate::runners::shared::{
    ensure_append_latest_is_safe as ensure_append_latest_is_safe_for_table, insert_typed_rows,
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
    let rows = rows
        .iter()
        .map(BollInsertRow::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    insert_typed_rows(executor, table, &rows, batch_size)
}
