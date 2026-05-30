from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

import pyarrow as pa

from scheduler.defs.common.strings import string_or_null

FLATTEN_COLUMN_NAMING = "shortest_leaf"

JIUYAN_ACTION_FIELD_COLUMNS = (
    "action_field_id",
    "name",
    "date",
    "reason",
    "sort_no",
    "is_delete",
    "delete_time",
    "create_time",
    "update_time",
    "count",
    "code",
    "time",
    "num",
    "price",
    "day",
    "edition",
    "shares_range",
    "expound",
)
JIUYAN_ACTION_FIELD_OUTER_COLUMNS = (
    "action_field_id",
    "name",
    "date",
    "reason",
    "sort_no",
    "is_delete",
    "delete_time",
    "create_time",
    "update_time",
    "count",
)
JIUYAN_ACTION_FIELD_STOCK_COLUMNS = ("code", "name")
JIUYAN_ACTION_FIELD_ACTION_INFO_COLUMNS = (
    "time",
    "num",
    "price",
    "day",
    "edition",
    "shares_range",
    "reason",
    "expound",
)

THS_LIMIT_UP_POOL_COLUMNS = (
    "date",
    "open_num",
    "first_limit_up_time",
    "last_limit_up_time",
    "code",
    "limit_up_type",
    "order_volume",
    "is_new",
    "limit_up_suc_rate",
    "currency_value",
    "market_id",
    "is_again_limit",
    "change_rate",
    "turnover_rate",
    "reason_type",
    "order_amount",
    "high_days",
    "name",
    "high_days_value",
    "change_tag",
    "market_type",
    "latest",
)
THS_LIMIT_UP_POOL_INFO_COLUMNS = tuple(
    column for column in THS_LIMIT_UP_POOL_COLUMNS if column != "date"
)

JIUYAN_INDUSTRY_LIST_COLUMNS = (
    "industry_id",
    "title_red",
    "title_bold",
    "title",
    "author",
    "imgs",
    "keyword",
    "content",
    "is_top",
    "status",
    "sort_no",
    "forward_count",
    "browsers_count",
    "is_delete",
    "delete_time",
    "create_time",
    "update_time",
)

# 带类型的 schema 定义
THS_LIMIT_UP_POOL_SCHEMA = pa.schema(
    [
        pa.field("date", pa.date32()),  # '20260508' → date32
        pa.field("open_num", pa.int64()),  # 开板次数
        pa.field("first_limit_up_time", pa.timestamp("ns", tz="UTC")),  # Unix 时间戳 → UTC
        pa.field("last_limit_up_time", pa.timestamp("ns", tz="UTC")),  # Unix 时间戳 → UTC
        pa.field("code", pa.string()),  # 股票代码
        pa.field("limit_up_type", pa.string()),  # 涨停类型
        pa.field("order_volume", pa.float64()),  # 封单量（手），可能有小数
        pa.field("is_new", pa.bool_()),  # 是否新股
        pa.field("limit_up_suc_rate", pa.float64()),  # 涨停成功率
        pa.field("currency_value", pa.float64()),  # 流通市值（元）
        pa.field("market_id", pa.int64()),  # 市场 ID
        pa.field("is_again_limit", pa.bool_()),  # 是否回封
        pa.field("change_rate", pa.float64()),  # 涨跌幅 (%)
        pa.field("turnover_rate", pa.float64()),  # 换手率 (%)
        pa.field("reason_type", pa.string()),  # 涨停原因标签
        pa.field("order_amount", pa.float64()),  # 封单金额（元）
        pa.field("high_days", pa.string()),  # 连板天数描述（"首板"）
        pa.field("name", pa.string()),  # 股票名称
        pa.field("high_days_value", pa.int64()),  # 连板天数数值
        pa.field("change_tag", pa.string()),  # 变动标签
        pa.field("market_type", pa.string()),  # 市场类型
        pa.field("latest", pa.float64()),  # 最新价
    ]
)

JIUYAN_ACTION_FIELD_SCHEMA = pa.schema(
    [
        pa.field("action_field_id", pa.string()),
        pa.field("name", pa.string()),
        pa.field("date", pa.date32()),
        pa.field("reason", pa.string()),
        pa.field("sort_no", pa.int64()),
        pa.field("is_delete", pa.bool_()),
        pa.field("delete_time", pa.timestamp("ns")),  # 'YYYY-MM-DD HH:mm:ss' 或 null
        pa.field("create_time", pa.timestamp("ns")),  # 'YYYY-MM-DD HH:mm:ss'
        pa.field("update_time", pa.timestamp("ns")),  # 'YYYY-MM-DD HH:mm:ss' 或 null
        pa.field("count", pa.int64()),
        pa.field("code", pa.string()),  # 'sz002350'
        pa.field("time", pa.time32("ms")),  # '09:37:51' → time
        pa.field("num", pa.string()),  # '9天5板' 或 null
        pa.field("price", pa.int64()),  # 1718（分）
        pa.field("day", pa.int64()),  # 连板天数 或 null
        pa.field("edition", pa.int64()),  # 连板板数 或 null
        pa.field("shares_range", pa.float64()),  # 999.0（万股）
        pa.field("expound", pa.string()),
    ]
)

JIUYAN_INDUSTRY_LIST_SCHEMA = pa.schema(
    [
        pa.field("industry_id", pa.string()),
        pa.field("title_red", pa.bool_()),  # 0/1 格式标记
        pa.field("title_bold", pa.bool_()),  # 0/1 格式标记
        pa.field("title", pa.string()),
        pa.field("author", pa.string()),
        pa.field("imgs", pa.string()),
        pa.field("keyword", pa.string()),
        pa.field("content", pa.string()),
        pa.field("is_top", pa.bool_()),
        pa.field("status", pa.int64()),
        pa.field("sort_no", pa.int64()),
        pa.field("forward_count", pa.int64()),
        pa.field("browsers_count", pa.int64()),
        pa.field("is_delete", pa.bool_()),
        pa.field("delete_time", pa.timestamp("ns")),  # 'YYYY-MM-DD HH:mm:ss' 或 null
        pa.field("create_time", pa.timestamp("ns")),
        pa.field("update_time", pa.timestamp("ns")),
    ]
)


@dataclass(frozen=True)
class TableConversionResult:
    table: pa.Table
    unknown_field_count: int


def empty_jiuyan_action_field_table() -> pa.Table:
    return empty_string_table(JIUYAN_ACTION_FIELD_COLUMNS)


def jiuyan_action_field_to_table(
    content_rows: Sequence[Mapping[str, object]],
) -> TableConversionResult:
    rows: list[dict[str, object]] = []
    unknown_field_count = 0
    for content_row in content_rows:
        unknown_field_count += unknown_field_count_for_mapping(
            content_row,
            allowed_top_level=JIUYAN_ACTION_FIELD_OUTER_COLUMNS + ("list",),
        )
        list_value = content_row.get("list")
        if not isinstance(list_value, list):
            continue

        for stock in list_value:
            if not isinstance(stock, Mapping):
                unknown_field_count += 1
                continue
            unknown_field_count += unknown_field_count_for_mapping(
                stock,
                allowed_top_level=JIUYAN_ACTION_FIELD_STOCK_COLUMNS + ("article",),
            )

            article = stock.get("article")
            action_info: Mapping[str, object] = {}
            if isinstance(article, Mapping):
                unknown_field_count += unknown_field_count_for_mapping(
                    article,
                    allowed_top_level=("action_info",),
                )
                action_info_value = article.get("action_info")
                if isinstance(action_info_value, Mapping):
                    action_info = action_info_value
                    unknown_field_count += unknown_field_count_for_mapping(
                        action_info,
                        allowed_top_level=JIUYAN_ACTION_FIELD_ACTION_INFO_COLUMNS,
                    )
                elif action_info_value is not None:
                    unknown_field_count += 1
            elif article is not None:
                unknown_field_count += 1

            output_row = _blank_row(JIUYAN_ACTION_FIELD_COLUMNS)
            copy_selected_fields(output_row, content_row, JIUYAN_ACTION_FIELD_OUTER_COLUMNS)
            copy_selected_fields(output_row, stock, JIUYAN_ACTION_FIELD_STOCK_COLUMNS)
            copy_selected_fields(output_row, action_info, JIUYAN_ACTION_FIELD_ACTION_INFO_COLUMNS)
            rows.append(output_row)

    return TableConversionResult(
        table=rows_to_typed_table(rows, JIUYAN_ACTION_FIELD_SCHEMA),
        unknown_field_count=unknown_field_count,
    )


def ths_limit_up_pool_to_table(
    pages: Sequence[Mapping[str, object]],
) -> TableConversionResult:
    rows: list[dict[str, object]] = []
    unknown_field_count = 0
    for page in pages:
        unknown_field_count += unknown_field_count_for_mapping(
            page,
            allowed_top_level=("date", "info"),
        )
        info = page.get("info")
        if not isinstance(info, list):
            continue

        for item in info:
            if not isinstance(item, Mapping):
                unknown_field_count += 1
                continue
            unknown_field_count += unknown_field_count_for_mapping(
                item,
                allowed_top_level=THS_LIMIT_UP_POOL_INFO_COLUMNS,
            )
            output_row = _blank_row(THS_LIMIT_UP_POOL_COLUMNS)
            output_row["date"] = page.get("date")
            copy_selected_fields(output_row, item, THS_LIMIT_UP_POOL_INFO_COLUMNS)
            rows.append(output_row)

    return TableConversionResult(
        table=rows_to_typed_table(rows, THS_LIMIT_UP_POOL_SCHEMA),
        unknown_field_count=unknown_field_count,
    )


def jiuyan_industry_list_to_table(
    pages: Sequence[Mapping[str, object]],
) -> TableConversionResult:
    rows: list[dict[str, object]] = []
    unknown_field_count = 0
    for page in pages:
        unknown_field_count += unknown_field_count_for_mapping(
            page,
            allowed_top_level=("result",),
        )
        result = page.get("result")
        if not isinstance(result, list):
            continue

        for item in result:
            if not isinstance(item, Mapping):
                unknown_field_count += 1
                continue
            unknown_field_count += unknown_field_count_for_mapping(
                item,
                allowed_top_level=JIUYAN_INDUSTRY_LIST_COLUMNS,
            )
            output_row = _blank_row(JIUYAN_INDUSTRY_LIST_COLUMNS)
            copy_selected_fields(output_row, item, JIUYAN_INDUSTRY_LIST_COLUMNS)
            rows.append(output_row)

    return TableConversionResult(
        table=rows_to_typed_table(rows, JIUYAN_INDUSTRY_LIST_SCHEMA),
        unknown_field_count=unknown_field_count,
    )


def empty_string_table(columns: Sequence[str]) -> pa.Table:
    return rows_to_string_table([], columns)


def rows_to_string_table(
    rows: Sequence[Mapping[str, object]],
    columns: Sequence[str],
) -> pa.Table:
    arrays = {
        column: pa.array(
            [string_or_null(row.get(column)) for row in rows],
            type=pa.string(),
        )
        for column in columns
    }
    return pa.table(arrays, schema=string_schema(columns))


def string_schema(columns: Sequence[str]) -> pa.Schema:
    return pa.schema([(column, pa.string()) for column in columns])


def rows_to_typed_table(
    rows: Sequence[Mapping[str, object]],
    schema: pa.Schema,
) -> pa.Table:
    """从行数据和 schema 构建 pa.Table，自动应用类型转换。

    None 值直接追加，不经过转换函数（转换函数会抛出异常）。
    """
    from scheduler.defs.common.schema import typed_table

    return typed_table(rows, schema)


def _blank_row(columns: Sequence[str]) -> dict[str, object]:
    return {column: None for column in columns}


def copy_selected_fields(
    output_row: dict[str, object],
    source: Mapping[str, object],
    columns: Sequence[str],
) -> None:
    for column in columns:
        if column in source:
            value = source[column]
            if value is not None or output_row.get(column) is None:
                output_row[column] = value


def unknown_field_count_for_mapping(
    source: Mapping[str, object],
    *,
    allowed_top_level: Sequence[str],
) -> int:
    allowed = set(allowed_top_level)
    return sum(1 for key in source if key not in allowed)
