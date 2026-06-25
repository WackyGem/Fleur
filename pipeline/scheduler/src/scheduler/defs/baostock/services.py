from __future__ import annotations

import time
from collections import Counter
from contextlib import AbstractAsyncContextManager
from dataclasses import dataclass
from datetime import date
from typing import Protocol

import dagster as dg
import pyarrow as pa

from scheduler.defs.baostock.protocol import BaostockResponse
from scheduler.defs.baostock.schemas import (
    K_HISTORY_DAILY_SCHEMA,
    k_history_daily_response_to_table,
    stock_basic_response_to_table,
)
from scheduler.defs.common.async_boundary import run_async_boundary
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.concurrency import BoundedTaskOptions, BoundedTaskRunner
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.market.readers import SecurityUniverseReader, TradeCalendarReader
from scheduler.defs.market.securities import filter_active_security_ranges

BAOSTOCK_DAILY_KLINE_CONNECTIONS = 1


class BaostockClientProtocol(Protocol):
    async def query_stock_basic(
        self,
        code: str = "",
        code_name: str = "",
    ) -> BaostockResponse: ...

    async def query_history_k_data_plus_daily(
        self,
        code: str,
        start_date: date,
        end_date: date,
    ) -> BaostockResponse: ...


class BaostockClientFactory(Protocol):
    def client(
        self,
        *,
        max_connections: int | None = None,
    ) -> AbstractAsyncContextManager[BaostockClientProtocol]: ...


@dataclass(frozen=True)
class BaostockDailyKlineRefreshRequest:
    partition_keys: list[str]
    refresh_until_trade_date: str | None


@dataclass(frozen=True)
class BaostockDailyKlineRefreshResult:
    tables: dict[str, pa.Table]
    metadata: dict[str, RawMetadataValue]


class BaostockStockBasicRefreshService:
    def __init__(self, client_factory: BaostockClientFactory) -> None:
        self._client_factory = client_factory

    def refresh(self) -> tuple[pa.Table, dict[str, float]]:
        return run_async_boundary(
            fetch_stock_basic_table(self._client_factory),
            context="BaoStock stock-basic refresh",
        )


class BaostockDailyKlineRefreshService:
    def __init__(
        self,
        *,
        trade_calendar_reader: TradeCalendarReader,
        security_universe_reader: SecurityUniverseReader,
        client_factory: BaostockClientFactory,
    ) -> None:
        self._trade_calendar_reader = trade_calendar_reader
        self._security_universe_reader = security_universe_reader
        self._client_factory = client_factory

    def refresh(
        self,
        request: BaostockDailyKlineRefreshRequest,
    ) -> BaostockDailyKlineRefreshResult:
        started_at = time.perf_counter()
        trade_dates = self._trade_calendar_reader.read_trade_dates()
        trade_calendar_read_at = time.perf_counter()
        stock_basic = self._security_universe_reader.read_stock_basic()
        stock_basic_read_at = time.perf_counter()
        year_ranges = build_year_ranges(
            request.partition_keys,
            refresh_until_trade_date=request.refresh_until_trade_date,
            trade_dates=trade_dates,
        )
        year_ranges_built_at = time.perf_counter()
        tables, metadata = run_async_boundary(
            fetch_k_history_tables(stock_basic, year_ranges, self._client_factory),
            context="BaoStock daily K-line refresh",
        )
        remote_fetch_finished_at = time.perf_counter()
        if not tables:
            msg = "BaoStock daily K-line query returned no rows for the selected partition range"
            raise RuntimeError(msg)
        row_count = sum(table.num_rows for table in tables.values())
        first_table = next(iter(tables.values()))

        metadata.update(
            {
                "row_count": row_count,
                "column_count": first_table.num_columns,
                "partition_keys": dg.MetadataValue.json(sorted(year_ranges)),
                "trade_calendar_read_seconds": elapsed_seconds(
                    started_at,
                    trade_calendar_read_at,
                ),
                "stock_basic_read_seconds": elapsed_seconds(
                    trade_calendar_read_at,
                    stock_basic_read_at,
                ),
                "year_ranges_build_seconds": elapsed_seconds(
                    stock_basic_read_at,
                    year_ranges_built_at,
                ),
                "baostock_remote_fetch_seconds": elapsed_seconds(
                    year_ranges_built_at,
                    remote_fetch_finished_at,
                ),
                "asset_function_seconds": elapsed_seconds(
                    started_at,
                    remote_fetch_finished_at,
                ),
            }
        )
        return BaostockDailyKlineRefreshResult(tables=tables, metadata=metadata)


async def fetch_stock_basic_table(
    client_factory: BaostockClientFactory,
) -> tuple[pa.Table, dict[str, float]]:
    started_at = time.perf_counter()
    async with client_factory.client() as client:
        client_started_at = time.perf_counter()
        response = await client.query_stock_basic()
        query_finished_at = time.perf_counter()
        table = stock_basic_response_to_table(response)
        table_converted_at = time.perf_counter()
    closed_at = time.perf_counter()

    return table, {
        "baostock_client_start_seconds": elapsed_seconds(started_at, client_started_at),
        "baostock_query_seconds": elapsed_seconds(client_started_at, query_finished_at),
        "table_convert_seconds": elapsed_seconds(query_finished_at, table_converted_at),
        "baostock_client_close_seconds": elapsed_seconds(table_converted_at, closed_at),
        "asset_function_seconds": elapsed_seconds(started_at, closed_at),
    }


async def fetch_k_history_tables(
    stock_basic: pa.Table,
    year_ranges: dict[str, tuple[date, date]],
    client_factory: BaostockClientFactory,
) -> tuple[dict[str, pa.Table], dict[str, RawMetadataValue]]:
    started_at = time.perf_counter()
    annual_tables: dict[str, list[pa.Table]] = {year: [] for year in year_ranges}
    candidate_security_count = stock_basic.num_rows
    selected_security_counts: dict[str, int] = {}
    skipped_security_counts: dict[str, int] = {}
    selected_security_types: Counter[str] = Counter()

    async with client_factory.client(max_connections=BAOSTOCK_DAILY_KLINE_CONNECTIONS) as client:
        client_started_at = time.perf_counter()
        tasks: list[tuple[str, str, date, date]] = []
        for year, (start_date, end_date) in year_ranges.items():
            security_ranges = filter_active_security_ranges(
                stock_basic,
                requested_start_date=start_date,
                requested_end_date=end_date,
            )
            selected_security_counts[year] = len(security_ranges)
            skipped_security_counts[year] = candidate_security_count - len(security_ranges)
            selected_security_types.update(
                security_range.security_type for security_range in security_ranges
            )
            for security_range in security_ranges:
                tasks.append(
                    (
                        year,
                        security_range.code,
                        security_range.start_date,
                        security_range.end_date,
                    )
                )
        tasks_scheduled_at = time.perf_counter()

        async def fetch_one(item: tuple[str, str, date, date]) -> tuple[str, pa.Table]:
            year, code, start_date, end_date = item
            table = await _fetch_one_daily_k_table(client, code, start_date, end_date)
            return year, table

        runner_result = await BoundedTaskRunner(
            BoundedTaskOptions(
                max_concurrent_tasks=BAOSTOCK_DAILY_KLINE_CONNECTIONS,
                max_failure_ratio=0,
            )
        ).run(
            tasks,
            item_key=lambda item: f"{item[0]}:{item[1]}",
            worker=fetch_one,
        )

        for year, table in runner_result.successes:
            if table.num_rows > 0:
                annual_tables[year].append(table)
        tasks_finished_at = time.perf_counter()

    tables: dict[str, pa.Table] = {}
    for year, year_tables in annual_tables.items():
        if year_tables:
            tables[year] = pa.concat_tables(year_tables, promote_options="default")
    table_built_at = time.perf_counter()
    if not tables:
        return {}, _k_history_metadata(
            candidate_security_count=candidate_security_count,
            selected_security_counts=selected_security_counts,
            skipped_security_counts=skipped_security_counts,
            selected_security_types=selected_security_types,
            year_ranges=year_ranges,
            started_at=started_at,
            client_started_at=client_started_at,
            tasks_scheduled_at=tasks_scheduled_at,
            tasks_finished_at=tasks_finished_at,
            table_built_at=table_built_at,
            runner_metadata=runner_result.metadata(item_name="security"),
        )

    missing_years = sorted(set(year_ranges) - set(tables))
    if missing_years:
        msg = f"BaoStock daily K-line query returned no rows for partitions: {missing_years}"
        raise RuntimeError(msg)

    return tables, _k_history_metadata(
        candidate_security_count=candidate_security_count,
        selected_security_counts=selected_security_counts,
        skipped_security_counts=skipped_security_counts,
        selected_security_types=selected_security_types,
        year_ranges=year_ranges,
        started_at=started_at,
        client_started_at=client_started_at,
        tasks_scheduled_at=tasks_scheduled_at,
        tasks_finished_at=tasks_finished_at,
        table_built_at=table_built_at,
        runner_metadata=runner_result.metadata(item_name="security"),
    )


async def _fetch_one_daily_k_table(
    client: BaostockClientProtocol,
    code: str,
    start_date: date,
    end_date: date,
) -> pa.Table:
    response = await client.query_history_k_data_plus_daily(code, start_date, end_date)
    return k_history_daily_response_to_table(response)


def build_year_ranges(
    partition_keys: list[str],
    *,
    refresh_until_trade_date: str | None = None,
    trade_dates: set[date],
) -> dict[str, tuple[date, date]]:
    if not partition_keys:
        msg = "BaoStock daily K-line asset requires at least one year partition"
        raise RuntimeError(msg)

    if refresh_until_trade_date is not None:
        if len(partition_keys) != 1:
            msg = "refresh_until_trade_date can only be used with a single year partition"
            raise ValueError(msg)
        partition_key = partition_keys[0]
        refresh_until = date.fromisoformat(refresh_until_trade_date)
        if refresh_until not in trade_dates:
            msg = f"refresh_until_trade_date {refresh_until.isoformat()} is not a trade date"
            raise ValueError(msg)
        if int(partition_key) != refresh_until.year:
            msg = (
                f"refresh_until_trade_date {refresh_until.isoformat()} "
                f"is not in partition {partition_key}"
            )
            raise ValueError(msg)
        return {partition_key: (date(int(partition_key), 1, 1), refresh_until)}

    ranges: dict[str, tuple[date, date]] = {}
    for partition_key in partition_keys:
        year = int(partition_key)
        year_trade_dates = [item for item in trade_dates if item.year == year]
        if not year_trade_dates:
            msg = f"Partition {partition_key} has no trade dates in Sina calendar"
            raise ValueError(msg)
        ranges[partition_key] = (date(year, 1, 1), date(year, 12, 31))
    return ranges


def empty_k_history_table() -> pa.Table:
    return pa.table(
        {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
        schema=K_HISTORY_DAILY_SCHEMA,
    )


def _k_history_metadata(
    *,
    candidate_security_count: int,
    selected_security_counts: dict[str, int],
    skipped_security_counts: dict[str, int],
    selected_security_types: Counter[str],
    year_ranges: dict[str, tuple[date, date]],
    started_at: float,
    client_started_at: float,
    tasks_scheduled_at: float,
    tasks_finished_at: float,
    table_built_at: float,
    runner_metadata: dict[str, RawMetadataValue],
) -> dict[str, RawMetadataValue]:
    return {
        "candidate_security_count": candidate_security_count,
        "selected_security_count": dg.MetadataValue.json(selected_security_counts),
        "skipped_security_count": dg.MetadataValue.json(skipped_security_counts),
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
        "baostock_client_start_seconds": elapsed_seconds(started_at, client_started_at),
        "security_filter_and_task_schedule_seconds": elapsed_seconds(
            client_started_at,
            tasks_scheduled_at,
        ),
        "baostock_kline_task_wall_seconds": elapsed_seconds(
            tasks_scheduled_at,
            tasks_finished_at,
        ),
        "kline_table_concat_seconds": elapsed_seconds(tasks_finished_at, table_built_at),
        "kline_fetch_total_seconds": elapsed_seconds(started_at, table_built_at),
        **runner_metadata,
    }
