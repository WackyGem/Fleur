from __future__ import annotations

from typing import Any

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

# 带类型的 schema 定义
STOCK_BASIC_SCHEMA = pa.schema(
    [
        pa.field("code", pa.string()),
        pa.field("code_name", pa.string()),
        pa.field("ipoDate", pa.date32()),
        pa.field("outDate", pa.date32()),  # 空字符串表示未退市，需特殊处理
        pa.field("type", pa.int8()),
        pa.field("status", pa.int8()),
    ]
)

K_HISTORY_DAILY_SCHEMA = pa.schema(
    [
        pa.field("date", pa.date32()),
        pa.field("code", pa.string()),
        pa.field("open", pa.float64()),
        pa.field("high", pa.float64()),
        pa.field("low", pa.float64()),
        pa.field("close", pa.float64()),
        pa.field("preclose", pa.float64()),
        pa.field("volume", pa.int64()),
        pa.field("amount", pa.float64()),
        pa.field("adjustflag", pa.int8()),
        pa.field("turn", pa.float64()),
        pa.field("tradestatus", pa.int8()),
        pa.field("pctChg", pa.float64()),
        pa.field("isST", pa.bool_()),
    ]
)


def response_to_table(response: BaostockResponse, schema: pa.Schema) -> pa.Table:
    """将 BaoStock 响应转换为 pa.Table，使用指定 schema 的类型。"""
    field_names = [field.name for field in schema]
    if not field_names:
        return pa.table({})

    columns: dict[str, list[Any]] = {field_name: [] for field_name in field_names}
    for record in response.records:
        if len(record) != len(field_names):
            msg = (
                f"BaoStock {response.api_name} returned {len(record)} values "
                f"for {len(field_names)} fields"
            )
            raise BaostockProtocolError(msg)
        for index, field_name in enumerate(field_names):
            columns[field_name].append(record[index])

    # 使用 typed_table 做类型转换
    from scheduler.defs.common.schema import typed_table

    rows = [
        {field_name: columns[field_name][i] for field_name in field_names}
        for i in range(len(columns[field_names[0]]))
        if field_names
    ]

    # 处理 BaoStock 特殊情况：空字符串表示未退市
    # outDate 空字符串应转为 None，而非抛出异常
    for row in rows:
        if "outDate" in row and row["outDate"] == "":
            row["outDate"] = None

    return typed_table(rows, schema)


def stock_basic_response_to_table(response: BaostockResponse) -> pa.Table:
    table = response_to_table(response, STOCK_BASIC_SCHEMA)
    _validate_expected_columns(table, STOCK_BASIC_FIELDS, response.api_name)
    return table


def k_history_daily_response_to_table(response: BaostockResponse) -> pa.Table:
    table = response_to_table(response, K_HISTORY_DAILY_SCHEMA)
    if table.num_columns == 0:
        return pa.table(
            {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
            schema=K_HISTORY_DAILY_SCHEMA,
        )

    _validate_expected_columns(table, K_HISTORY_DAILY_FIELDS, response.api_name)
    return table


def _validate_expected_columns(
    table: pa.Table,
    expected_columns: list[str],
    api_name: str,
) -> None:
    missing_columns = set(expected_columns) - set(table.column_names)
    if missing_columns:
        msg = f"BaoStock {api_name} response is missing columns: {sorted(missing_columns)}"
        raise BaostockProtocolError(msg)
