from __future__ import annotations

import asyncio
import time
from collections import Counter
from dataclasses import dataclass
from datetime import date
from typing import Any

import dagster as dg
import pyarrow as pa

from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.securities import filter_active_security_ranges
from scheduler.defs.sources.eastmoney.client import (
    EASTMONEY_CODE_CONCURRENCY,
    EastmoneyAioHttpClient,
)
from scheduler.defs.sources.eastmoney.schema import (
    EastmoneyEndpointConfig,
    EastmoneyFetchedRow,
    eastmoney_rows_to_table,
    empty_eastmoney_table,
)
from scheduler.defs.storage.parquet_readers import read_baostock_stock_basic_from_s3


@dataclass(frozen=True)
class EastmoneyRefreshRequest:
    endpoint: EastmoneyEndpointConfig
    partition_keys: list[str]
    refresh_until_date: str | None


@dataclass(frozen=True)
class EastmoneyRefreshResult:
    tables: dict[str, pa.Table]
    metadata: dict[str, Any]


class EastmoneyYearRefreshService:
    def __init__(self, s3_config: S3Config) -> None:
        self._s3_config = s3_config

    def refresh(self, request: EastmoneyRefreshRequest) -> EastmoneyRefreshResult:
        started_at = time.perf_counter()
        stock_basic = read_baostock_stock_basic_from_s3(self._s3_config)
        stock_basic_read_at = time.perf_counter()
        year_ranges = build_year_ranges(
            request.partition_keys,
            refresh_until_date=request.refresh_until_date,
        )
        year_ranges_built_at = time.perf_counter()
        tables, metadata = asyncio.run(
            fetch_eastmoney_tables(request.endpoint, stock_basic, year_ranges)
        )
        remote_fetch_finished_at = time.perf_counter()

        row_count = sum(table.num_rows for table in tables.values())
        column_count = next(iter(tables.values())).num_columns
        metadata.update(
            {
                "row_count": row_count,
                "column_count": column_count,
                "partition_keys": dg.MetadataValue.json(sorted(year_ranges)),
                "selected_date_field": request.endpoint.date_field,
                "sort_columns": ",".join(request.endpoint.sort_fields),
                "sort_types": ",".join(request.endpoint.sort_directions),
                "source_endpoint": request.endpoint.source_endpoint,
                "code_concurrency_limit": EASTMONEY_CODE_CONCURRENCY,
                "stock_basic_read_seconds": elapsed_seconds(started_at, stock_basic_read_at),
                "year_ranges_build_seconds": elapsed_seconds(
                    stock_basic_read_at,
                    year_ranges_built_at,
                ),
                "eastmoney_remote_fetch_seconds": elapsed_seconds(
                    year_ranges_built_at,
                    remote_fetch_finished_at,
                ),
                "asset_function_seconds": elapsed_seconds(
                    started_at,
                    remote_fetch_finished_at,
                ),
            }
        )
        return EastmoneyRefreshResult(tables=tables, metadata=metadata)


async def fetch_eastmoney_tables(
    endpoint: EastmoneyEndpointConfig,
    stock_basic: pa.Table,
    year_ranges: dict[str, tuple[date, date]],
) -> tuple[dict[str, pa.Table], dict[str, Any]]:
    started_at = time.perf_counter()
    candidate_security_count = stock_basic.num_rows
    annual_rows: dict[str, list[EastmoneyFetchedRow]] = {year: [] for year in year_ranges}
    selected_security_counts: dict[str, int] = {}
    skipped_security_counts: dict[str, int] = {}
    unsupported_market_code_counts: dict[str, int] = {}
    selected_security_types: Counter[str] = Counter()

    async with EastmoneyAioHttpClient() as client:
        client_started_at = time.perf_counter()
        async with asyncio.TaskGroup() as task_group:
            tasks: list[tuple[str, str, date, date, asyncio.Task[list[dict[str, object]]]]] = []
            for year, (start_date, end_date) in year_ranges.items():
                security_ranges = filter_active_security_ranges(
                    stock_basic,
                    requested_start_date=start_date,
                    requested_end_date=end_date,
                    allowed_security_types=frozenset({"1"}),
                )
                selected_security_counts[year] = len(security_ranges)
                skipped_security_counts[year] = candidate_security_count - len(security_ranges)
                selected_security_types.update(
                    security_range.security_type for security_range in security_ranges
                )
                unsupported_market_code_count = 0
                for security_range in security_ranges:
                    eastmoney_code = baostock_code_to_eastmoney_code(security_range.code)
                    if eastmoney_code is None:
                        unsupported_market_code_count += 1
                        continue
                    task = task_group.create_task(
                        client.fetch_code_range(
                            endpoint,
                            eastmoney_code,
                            security_range.start_date,
                            security_range.end_date,
                        )
                    )
                    tasks.append(
                        (
                            year,
                            eastmoney_code,
                            security_range.start_date,
                            security_range.end_date,
                            task,
                        )
                    )
                unsupported_market_code_counts[year] = unsupported_market_code_count
            tasks_scheduled_at = time.perf_counter()

        for _year, _eastmoney_code, _start_date, _end_date, task in tasks:
            for row in task.result():
                annual_rows[_year].append(EastmoneyFetchedRow(data=row))
        tasks_finished_at = time.perf_counter()
        fetch_stats = client.stats

    table_convert_started_at = time.perf_counter()
    tables: dict[str, pa.Table] = {}
    unknown_field_counts: dict[str, int] = {}
    for year, rows in annual_rows.items():
        if rows:
            result = eastmoney_rows_to_table(endpoint, rows)
            tables[year] = result.table
            unknown_field_counts[year] = result.unknown_field_count
        else:
            tables[year] = empty_eastmoney_table(endpoint)
            unknown_field_counts[year] = 0
    table_built_at = time.perf_counter()

    return tables, {
        "candidate_security_count": candidate_security_count,
        "selected_security_count": dg.MetadataValue.json(selected_security_counts),
        "skipped_security_count": dg.MetadataValue.json(skipped_security_counts),
        "unsupported_market_code_count": dg.MetadataValue.json(unsupported_market_code_counts),
        "selected_security_types": dg.MetadataValue.json(sorted(selected_security_types)),
        "requested_ranges": dg.MetadataValue.json(
            {
                year: {
                    "start_date": start_date.isoformat(),
                    "end_date": end_date.isoformat(),
                }
                for year, (start_date, end_date) in year_ranges.items()
            }
        ),
        "source_endpoints": dg.MetadataValue.json([endpoint.source_endpoint]),
        "request_count": fetch_stats.request_count,
        "empty_response_count": fetch_stats.empty_response_count,
        "page_count": fetch_stats.page_count,
        "retry_count": fetch_stats.retry_count,
        "transient_error_count": fetch_stats.transient_error_count,
        "http_4xx_count": fetch_stats.http_4xx_count,
        "http_5xx_count": fetch_stats.http_5xx_count,
        "decode_error_count": fetch_stats.decode_error_count,
        "status_code_counts": dg.MetadataValue.json(fetch_stats.status_code_counts),
        "endpoint_host_counts": dg.MetadataValue.json(fetch_stats.endpoint_host_counts),
        "duplicate_page_row_count": fetch_stats.duplicate_page_row_count,
        "unknown_field_count": dg.MetadataValue.json(unknown_field_counts),
        "eastmoney_client_start_seconds": elapsed_seconds(started_at, client_started_at),
        "security_filter_and_task_schedule_seconds": elapsed_seconds(
            client_started_at,
            tasks_scheduled_at,
        ),
        "eastmoney_task_wall_seconds": elapsed_seconds(
            tasks_scheduled_at,
            tasks_finished_at,
        ),
        "table_convert_seconds": elapsed_seconds(table_convert_started_at, table_built_at),
        "eastmoney_fetch_total_seconds": elapsed_seconds(started_at, table_built_at),
    }


def baostock_code_to_eastmoney_code(code: str) -> str | None:
    if code.startswith("sh.") and len(code) > 3:
        return f"{code[3:]}.SH"
    if code.startswith("sz.") and len(code) > 3:
        return f"{code[3:]}.SZ"
    return None


def build_year_ranges(
    partition_keys: list[str],
    *,
    refresh_until_date: str | None = None,
) -> dict[str, tuple[date, date]]:
    if not partition_keys:
        msg = "EastMoney F10 asset requires at least one year partition"
        raise RuntimeError(msg)

    if refresh_until_date is not None:
        if len(partition_keys) != 1:
            msg = "refresh_until_date can only be used with a single year partition"
            raise ValueError(msg)
        partition_key = partition_keys[0]
        refresh_until = date.fromisoformat(refresh_until_date)
        if int(partition_key) != refresh_until.year:
            msg = (
                f"refresh_until_date {refresh_until.isoformat()} "
                f"is not in partition {partition_key}"
            )
            raise ValueError(msg)
        return {partition_key: (date(int(partition_key), 1, 1), refresh_until)}

    return {
        partition_key: (date(int(partition_key), 1, 1), date(int(partition_key), 12, 31))
        for partition_key in partition_keys
    }
