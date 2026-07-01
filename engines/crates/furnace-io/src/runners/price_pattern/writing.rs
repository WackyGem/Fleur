use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::PricePatternRunRequest;
use crate::rows::{PricePatternInsertRow, PricePatternResultRow};
use crate::runners::shared::{
    ensure_append_latest_is_safe as ensure_append_latest_is_safe_for_table, insert_typed_rows,
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
    let rows = rows
        .iter()
        .map(PricePatternInsertRow::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    insert_typed_rows(executor, table, &rows, batch_size)
}
