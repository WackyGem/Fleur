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
        table=rows_to_string_table(rows, JIUYAN_ACTION_FIELD_COLUMNS),
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
        table=rows_to_string_table(rows, THS_LIMIT_UP_POOL_COLUMNS),
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
        table=rows_to_string_table(rows, JIUYAN_INDUSTRY_LIST_COLUMNS),
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
