from __future__ import annotations

import pyarrow as pa

from scheduler.defs.baostock.protocol import BaostockProtocolError, BaostockResponse

STOCK_BASIC_FIELDS = [
    "code",
    "code_name",
    "ipoDate",
    "outDate",
    "type",
    "status",
]
K_HISTORY_DAILY_FIELDS = [
    "date",
    "code",
    "open",
    "high",
    "low",
    "close",
    "preclose",
    "volume",
    "amount",
    "adjustflag",
    "turn",
    "tradestatus",
    "pctChg",
    "isST",
]
K_HISTORY_DAILY_FIELD_PARAM = ",".join(K_HISTORY_DAILY_FIELDS)


def response_to_table(response: BaostockResponse) -> pa.Table:
    field_names = response.field_names
    if not field_names:
        return pa.table({})

    columns = {field_name: [] for field_name in field_names}
    for record in response.records:
        if len(record) != len(field_names):
            msg = (
                f"BaoStock {response.api_name} returned {len(record)} values "
                f"for {len(field_names)} fields"
            )
            raise BaostockProtocolError(msg)
        for index, field_name in enumerate(field_names):
            columns[field_name].append(record[index])

    schema = pa.schema([(field_name, pa.string()) for field_name in field_names])
    return pa.table(columns, schema=schema)


def stock_basic_response_to_table(response: BaostockResponse) -> pa.Table:
    table = response_to_table(response)
    _validate_expected_columns(table, STOCK_BASIC_FIELDS, response.api_name)
    return table.select(STOCK_BASIC_FIELDS)


def k_history_daily_response_to_table(response: BaostockResponse) -> pa.Table:
    table = response_to_table(response)
    if table.num_columns == 0:
        schema = pa.schema([(field_name, pa.string()) for field_name in K_HISTORY_DAILY_FIELDS])
        return pa.table({field_name: [] for field_name in K_HISTORY_DAILY_FIELDS}, schema=schema)

    _validate_expected_columns(table, K_HISTORY_DAILY_FIELDS, response.api_name)
    return table.select(K_HISTORY_DAILY_FIELDS)


def _validate_expected_columns(
    table: pa.Table,
    expected_columns: list[str],
    api_name: str,
) -> None:
    missing_columns = set(expected_columns) - set(table.column_names)
    if missing_columns:
        msg = f"BaoStock {api_name} response is missing columns: {sorted(missing_columns)}"
        raise BaostockProtocolError(msg)
