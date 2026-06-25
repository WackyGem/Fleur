from __future__ import annotations

import time
from collections import Counter
from contextlib import AbstractAsyncContextManager
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
from scheduler.defs.market.securities import SecurityDateRange, filter_active_security_ranges

BAOSTOCK_DAILY_KLINE_CONNECTIONS = 4
BAOSTOCK_DAILY_KLINE_SECURITY_TYPES = frozenset({"1", "2"})


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


class BaostockStockBasicRefreshService:
    def __init__(self, client_factory: BaostockClientFactory) -> None:
        self._client_factory = client_factory

    def refresh(self) -> tuple[pa.Table, dict[str, float]]:
        return run_async_boundary(
            fetch_stock_basic_table(self._client_factory),
            context="BaoStock stock-basic refresh",
        )


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


async def fetch_k_history_table_for_trade_date(
    stock_basic: pa.Table,
    trade_date: date,
    client_factory: BaostockClientFactory,
) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
    started_at = time.perf_counter()
    candidate_security_count = stock_basic.num_rows
    security_ranges = filter_active_security_ranges(
        stock_basic,
        requested_start_date=trade_date,
        requested_end_date=trade_date,
        allowed_security_types=BAOSTOCK_DAILY_KLINE_SECURITY_TYPES,
    )
    selected_security_types: Counter[str] = Counter(
        security_range.security_type for security_range in security_ranges
    )
    tasks_scheduled_at = time.perf_counter()

    if not security_ranges:
        table_built_at = time.perf_counter()
        return empty_k_history_table(), _k_history_metadata(
            candidate_security_count=candidate_security_count,
            selected_security_count=0,
            skipped_security_count=candidate_security_count,
            selected_security_types=selected_security_types,
            trade_date=trade_date,
            started_at=started_at,
            client_started_at=tasks_scheduled_at,
            tasks_scheduled_at=tasks_scheduled_at,
            tasks_finished_at=tasks_scheduled_at,
            table_built_at=table_built_at,
            runner_metadata={},
        )

    async with client_factory.client(max_connections=BAOSTOCK_DAILY_KLINE_CONNECTIONS) as client:
        client_started_at = time.perf_counter()

        async def fetch_one(security_range: SecurityDateRange) -> pa.Table:
            return await _fetch_one_daily_k_table(
                client,
                security_range.code,
                trade_date,
                trade_date,
            )

        runner_result = await BoundedTaskRunner(
            BoundedTaskOptions(
                max_concurrent_tasks=BAOSTOCK_DAILY_KLINE_CONNECTIONS,
                max_failure_ratio=0,
            )
        ).run(
            security_ranges,
            item_key=lambda security_range: security_range.code,
            worker=fetch_one,
        )
        tasks_finished_at = time.perf_counter()

    fetched_tables = [table for table in runner_result.successes if table.num_rows > 0]
    table = (
        pa.concat_tables(fetched_tables, promote_options="default")
        if fetched_tables
        else empty_k_history_table()
    )
    table_built_at = time.perf_counter()
    return table, _k_history_metadata(
        candidate_security_count=candidate_security_count,
        selected_security_count=len(security_ranges),
        skipped_security_count=candidate_security_count - len(security_ranges),
        selected_security_types=selected_security_types,
        trade_date=trade_date,
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


def empty_k_history_table() -> pa.Table:
    return pa.table(
        {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
        schema=K_HISTORY_DAILY_SCHEMA,
    )


def _k_history_metadata(
    *,
    candidate_security_count: int,
    selected_security_count: int,
    skipped_security_count: int,
    selected_security_types: Counter[str],
    trade_date: date,
    started_at: float,
    client_started_at: float,
    tasks_scheduled_at: float,
    tasks_finished_at: float,
    table_built_at: float,
    runner_metadata: dict[str, RawMetadataValue],
) -> dict[str, RawMetadataValue]:
    return {
        "candidate_security_count": candidate_security_count,
        "selected_security_count": selected_security_count,
        "skipped_security_count": skipped_security_count,
        "selected_security_types": dg.MetadataValue.json(sorted(selected_security_types)),
        "allowed_security_types": dg.MetadataValue.json(
            sorted(BAOSTOCK_DAILY_KLINE_SECURITY_TYPES)
        ),
        "requested_trade_date": trade_date.isoformat(),
        "max_connections": BAOSTOCK_DAILY_KLINE_CONNECTIONS,
        "max_concurrent_security_requests": BAOSTOCK_DAILY_KLINE_CONNECTIONS,
        "baostock_client_start_seconds": elapsed_seconds(tasks_scheduled_at, client_started_at),
        "security_filter_and_task_schedule_seconds": elapsed_seconds(
            started_at,
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
