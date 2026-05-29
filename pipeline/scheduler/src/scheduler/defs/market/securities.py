from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from datetime import date
from typing import Any, cast

import pyarrow as pa
import pyarrow.compute as pc

from scheduler.defs.common.dates import parse_date_or_none
from scheduler.defs.common.strings import optional_string

BAOSTOCK_SECURITY_TYPE_DATA_START_DATES = {
    "1": date(1990, 12, 19),
    "2": date(2006, 1, 1),
    "5": date(2026, 1, 5),
}


@dataclass(frozen=True)
class SecurityDateRange:
    code: str
    security_type: str
    start_date: date
    end_date: date


def filter_active_security_ranges(
    stock_basic: pa.Table,
    requested_start_date: date,
    requested_end_date: date,
    allowed_security_types: frozenset[str] = frozenset({"1", "2", "5"}),
) -> list[SecurityDateRange]:
    if requested_start_date > requested_end_date:
        msg = "requested_start_date must be less than or equal to requested_end_date"
        raise ValueError(msg)

    required_columns = {"code", "ipoDate", "outDate", "type"}
    missing_columns = required_columns - set(stock_basic.column_names)
    if missing_columns:
        msg = f"stock_basic is missing required columns: {sorted(missing_columns)}"
        raise ValueError(msg)

    selected = stock_basic.select(["code", "ipoDate", "outDate", "type"])
    ranges: list[SecurityDateRange] = []
    for row in selected.to_pylist():
        code = optional_string(row["code"])
        security_type = optional_string(row["type"])
        if code is None or security_type is None:
            continue
        if security_type not in allowed_security_types:
            continue
        if security_type not in BAOSTOCK_SECURITY_TYPE_DATA_START_DATES:
            continue

        ipo_date = parse_date_or_none(row["ipoDate"])
        if ipo_date is None:
            continue
        out_date = parse_date_or_none(row["outDate"])
        if out_date is not None and out_date < ipo_date:
            continue

        security_start = max(
            ipo_date,
            BAOSTOCK_SECURITY_TYPE_DATA_START_DATES[security_type],
        )
        effective_start = max(requested_start_date, security_start)
        effective_end = requested_end_date
        if out_date is not None:
            effective_end = min(effective_end, out_date)
        if effective_start > effective_end:
            continue

        ranges.append(
            SecurityDateRange(
                code=code,
                security_type=security_type,
                start_date=effective_start,
                end_date=effective_end,
            )
        )

    return ranges


def table_row_count_by_string_column(table: pa.Table, column_name: str) -> dict[str, int]:
    if column_name not in table.column_names:
        msg = f"Column {column_name!r} is missing from table"
        raise ValueError(msg)

    value_counts = cast(Callable[[object], Any], cast(Any, pc).value_counts)
    counts = value_counts(table[column_name]).to_pylist()
    return {row["values"]: row["counts"] for row in counts}
