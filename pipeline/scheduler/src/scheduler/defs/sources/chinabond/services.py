from __future__ import annotations

import time
from collections.abc import Mapping
from contextlib import AbstractAsyncContextManager
from dataclasses import dataclass
from datetime import UTC, date, datetime
from typing import Any, Protocol

import dagster as dg
import pyarrow as pa

from scheduler.defs.common.async_boundary import run_async_boundary
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.contract_schemas import PARQUET_SCHEMAS
from scheduler.defs.http.client import HttpFetchStats
from scheduler.defs.sources.chinabond.client import ChinabondRequestError

CHINABOND_DATASET = "chinabond__government_bond"
CHINABOND_CURVE_NAME = "中债国债收益率曲线"
CHINABOND_FIRST_DATA_DATE = date(2006, 3, 1)

RAW_YIELD_FIELDS: tuple[tuple[str, str], ...] = (
    ("threeMonth", "three_month_yield_pct"),
    ("sixMonth", "six_month_yield_pct"),
    ("oneYear", "one_year_yield_pct"),
    ("twoYear", "two_year_yield_pct"),
    ("threeYear", "three_year_yield_pct"),
    ("fiveYear", "five_year_yield_pct"),
    ("sevenYear", "seven_year_yield_pct"),
    ("tenYear", "ten_year_yield_pct"),
    ("fifteenYear", "fifteen_year_yield_pct"),
    ("twentyYear", "twenty_year_yield_pct"),
    ("thirtyYear", "thirty_year_yield_pct"),
)


class ChinabondClientProtocol(Protocol):
    stats: HttpFetchStats

    async def fetch_government_bond_curve(
        self,
        *,
        start_date: str,
        end_date: str,
    ) -> Mapping[str, object]: ...


class ChinabondClientFactory(Protocol):
    def client(self) -> AbstractAsyncContextManager[ChinabondClientProtocol]: ...


@dataclass(frozen=True)
class ChinabondRefreshRequest:
    partition_keys: list[str]
    refresh_until_date: str | None = None


@dataclass(frozen=True)
class ChinabondRefreshResult:
    tables: dict[str, pa.Table]
    metadata: dict[str, Any]


class ChinabondGovernmentBondRefreshService:
    def __init__(self, *, client_factory: ChinabondClientFactory) -> None:
        self._client_factory = client_factory

    def refresh(self, request: ChinabondRefreshRequest) -> ChinabondRefreshResult:
        started_at = time.perf_counter()
        year_ranges = build_year_ranges(
            request.partition_keys,
            refresh_until_date=request.refresh_until_date,
        )
        ranges_built_at = time.perf_counter()
        tables, metadata = run_async_boundary(
            fetch_chinabond_tables(year_ranges, self._client_factory),
            context="ChinaBond government bond refresh",
        )
        fetch_finished_at = time.perf_counter()

        row_counts = {partition_key: table.num_rows for partition_key, table in tables.items()}
        metadata.update(
            {
                "row_count": sum(row_counts.values()),
                "column_count": next(iter(tables.values())).num_columns,
                "partition_keys": dg.MetadataValue.json(sorted(year_ranges)),
                "partition_row_counts": dg.MetadataValue.json(row_counts),
                "requested_ranges": dg.MetadataValue.json(
                    {
                        year: {
                            "start_date": start_date.isoformat(),
                            "end_date": end_date.isoformat(),
                        }
                        for year, (start_date, end_date) in year_ranges.items()
                    }
                ),
                "year_ranges_build_seconds": elapsed_seconds(started_at, ranges_built_at),
                "chinabond_remote_fetch_seconds": elapsed_seconds(
                    ranges_built_at,
                    fetch_finished_at,
                ),
                "asset_function_seconds": elapsed_seconds(started_at, fetch_finished_at),
            }
        )
        return ChinabondRefreshResult(tables=tables, metadata=metadata)


async def fetch_chinabond_tables(
    year_ranges: Mapping[str, tuple[date, date]],
    client_factory: ChinabondClientFactory,
) -> tuple[dict[str, pa.Table], dict[str, Any]]:
    started_at = time.perf_counter()
    tables: dict[str, pa.Table] = {}
    business_flags: dict[str, str] = {}
    min_work_dates: dict[str, str] = {}
    max_work_dates: dict[str, str] = {}
    curve_name_counts: dict[str, int] = {}

    async with client_factory.client() as client:
        client_started_at = time.perf_counter()
        for year, (start_date, end_date) in year_ranges.items():
            payload = await client.fetch_government_bond_curve(
                start_date=start_date.isoformat(),
                end_date=end_date.isoformat(),
            )
            business_flags[year] = str(payload.get("flag"))
            rows = parse_chinabond_rows(payload, partition_key=year)
            table = chinabond_rows_to_table(rows)
            tables[year] = table
            work_dates = table["work_date"].to_pylist()
            min_work_dates[year] = min(work_dates).isoformat()
            max_work_dates[year] = max(work_dates).isoformat()
            curve_name_counts[year] = len(set(table["curve_name"].to_pylist()))
        fetch_finished_at = time.perf_counter()
        fetch_stats = client.stats

    return tables, {
        "business_flags": dg.MetadataValue.json(business_flags),
        "min_work_dates": dg.MetadataValue.json(min_work_dates),
        "max_work_dates": dg.MetadataValue.json(max_work_dates),
        "curve_name_counts": dg.MetadataValue.json(curve_name_counts),
        "source_endpoints": dg.MetadataValue.json(
            ["https://yield.chinabond.com.cn/cbweb-czb-web/czb/historyQuery"]
        ),
        "request_count": fetch_stats.request_count,
        "retry_count": fetch_stats.retry_count,
        "transient_error_count": fetch_stats.transient_error_count,
        "http_4xx_count": fetch_stats.http_4xx_count,
        "http_5xx_count": fetch_stats.http_5xx_count,
        "decode_error_count": fetch_stats.decode_error_count,
        "status_code_counts": dg.MetadataValue.json(fetch_stats.status_code_counts),
        "endpoint_host_counts": dg.MetadataValue.json(fetch_stats.endpoint_host_counts),
        "chinabond_client_start_seconds": elapsed_seconds(started_at, client_started_at),
        "chinabond_fetch_total_seconds": elapsed_seconds(started_at, fetch_finished_at),
    }


def build_year_ranges(
    partition_keys: list[str],
    *,
    refresh_until_date: str | None = None,
) -> dict[str, tuple[date, date]]:
    if not partition_keys:
        msg = "ChinaBond government bond asset requires at least one year partition"
        raise RuntimeError(msg)

    if refresh_until_date is not None:
        if len(partition_keys) != 1:
            msg = "refresh_until_date can only be used with a single year partition"
            raise ValueError(msg)
        refresh_until = date.fromisoformat(refresh_until_date)
        partition_key = partition_keys[0]
        if int(partition_key) != refresh_until.year:
            msg = (
                f"refresh_until_date {refresh_until.isoformat()} "
                f"is not in partition {partition_key}"
            )
            raise ValueError(msg)
        return {partition_key: (_partition_start_date(partition_key), refresh_until)}

    return {
        partition_key: (_partition_start_date(partition_key), date(int(partition_key), 12, 31))
        for partition_key in partition_keys
    }


def parse_chinabond_rows(
    payload: Mapping[str, object],
    *,
    partition_key: str,
) -> list[dict[str, object]]:
    flag = payload.get("flag")
    raw_rows = payload.get("heList")
    if flag != "0":
        msg = f"ChinaBond response flag={flag!r} for partition {partition_key}"
        raise ChinabondRequestError(msg)
    if not isinstance(raw_rows, list):
        msg = f"ChinaBond heList is not a list for partition {partition_key}"
        raise ChinabondRequestError(msg)
    if not raw_rows:
        msg = f"ChinaBond returned no rows for partition {partition_key}"
        raise ChinabondRequestError(msg)

    normalized_rows = []
    seen_dates: set[date] = set()
    for raw_row in raw_rows:
        if not isinstance(raw_row, Mapping):
            msg = f"ChinaBond row is not an object for partition {partition_key}"
            raise ChinabondRequestError(msg)
        normalized = normalize_chinabond_row(raw_row)
        work_date = normalized["work_date"]
        if not isinstance(work_date, date):
            msg = "Normalized ChinaBond work_date is not a date"
            raise TypeError(msg)
        if str(work_date.year) != partition_key:
            msg = (
                f"ChinaBond work_date {work_date.isoformat()} does not belong to "
                f"partition {partition_key}"
            )
            raise ChinabondRequestError(msg)
        if work_date in seen_dates:
            msg = f"Duplicate ChinaBond work_date {work_date.isoformat()}"
            raise ChinabondRequestError(msg)
        seen_dates.add(work_date)
        normalized_rows.append(normalized)

    return sorted(normalized_rows, key=lambda row: row["work_date"])


def normalize_chinabond_row(row: Mapping[str, object]) -> dict[str, object]:
    work_time = row.get("workTime")
    if not isinstance(work_time, str):
        msg = "ChinaBond row is missing string workTime"
        raise ChinabondRequestError(msg)
    work_date = date.fromisoformat(work_time)

    curve_name = row.get("qxmc")
    if not isinstance(curve_name, str) or curve_name != CHINABOND_CURVE_NAME:
        msg = f"Unexpected ChinaBond curve name: {curve_name!r}"
        raise ChinabondRequestError(msg)

    normalized: dict[str, object] = {
        "work_date": work_date,
        "curve_name": curve_name,
    }
    for raw_field, canonical_field in RAW_YIELD_FIELDS:
        normalized[canonical_field] = parse_optional_float(row.get(raw_field), field_name=raw_field)
    return normalized


def chinabond_rows_to_table(rows: list[dict[str, object]]) -> pa.Table:
    if not rows:
        msg = "ChinaBond rows must not be empty"
        raise ValueError(msg)
    columns = {
        field.name: [row[field.name] for row in rows]
        for field in PARQUET_SCHEMAS[CHINABOND_DATASET]
    }
    return pa.table(columns, schema=PARQUET_SCHEMAS[CHINABOND_DATASET])


def parse_optional_float(value: object, *, field_name: str) -> float | None:
    if value is None:
        return None
    if isinstance(value, int | float):
        return float(value)
    if not isinstance(value, str):
        msg = f"ChinaBond field {field_name} is not a string, number, or null"
        raise ChinabondRequestError(msg)
    stripped = value.strip()
    if not stripped:
        return None
    try:
        return float(stripped)
    except ValueError as error:
        msg = f"ChinaBond field {field_name} is not a valid number: {value!r}"
        raise ChinabondRequestError(msg) from error


def _partition_start_date(partition_key: str) -> date:
    year = int(partition_key)
    if year == CHINABOND_FIRST_DATA_DATE.year:
        return CHINABOND_FIRST_DATA_DATE
    if year < CHINABOND_FIRST_DATA_DATE.year:
        msg = f"ChinaBond partition {partition_key} is before first data year"
        raise ValueError(msg)
    return date(year, 1, 1)


def current_utc_date_iso() -> str:
    return datetime.now(UTC).date().isoformat()
