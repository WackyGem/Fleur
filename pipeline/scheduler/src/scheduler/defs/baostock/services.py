from __future__ import annotations

import time
from collections import Counter
from collections.abc import Sequence
from contextlib import AbstractAsyncContextManager
from dataclasses import dataclass
from datetime import date
from typing import Protocol

import dagster as dg
import pyarrow as pa

from scheduler.defs.baostock.protocol import BaostockNetworkError, BaostockResponse
from scheduler.defs.baostock.schemas import (
    K_HISTORY_DAILY_SCHEMA,
    k_history_daily_response_to_table,
    stock_basic_response_to_table,
)
from scheduler.defs.common.async_boundary import run_async_boundary
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.concurrency import (
    BoundedTaskOptions,
    BoundedTaskResult,
    BoundedTaskRunner,
)
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.market.securities import SecurityDateRange, filter_active_security_ranges

BAOSTOCK_DAILY_KLINE_CONNECTIONS = 4
BAOSTOCK_DAILY_KLINE_SECURITY_TYPES = frozenset({"1", "2"})
BAOSTOCK_DAILY_KLINE_NETWORK_FAILURE_STOP_THRESHOLD = 20


@dataclass(frozen=True)
class KHistoryRangeBackfillResult:
    tables: dict[str, pa.Table]
    metadata: dict[str, RawMetadataValue]


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
            runner_metadata=_empty_kline_runner_metadata(),
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
                fail_when_all_failed=False,
                stop_on_error_types=(BaostockNetworkError,),
                max_failures_before_stop=(BAOSTOCK_DAILY_KLINE_NETWORK_FAILURE_STOP_THRESHOLD),
            )
        ).run(
            security_ranges,
            item_key=lambda security_range: security_range.code,
            worker=fetch_one,
        )
        tasks_finished_at = time.perf_counter()

    _raise_for_kline_request_failures(runner_result)
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
        runner_metadata=_kline_runner_metadata(runner_result),
    )


async def fetch_k_history_tables_for_trade_date_range(
    stock_basic: pa.Table,
    start_date: date,
    end_date: date,
    trade_dates: Sequence[date],
    client_factory: BaostockClientFactory,
) -> KHistoryRangeBackfillResult:
    started_at = time.perf_counter()
    if start_date > end_date:
        msg = "start_date must be less than or equal to end_date"
        raise ValueError(msg)

    target_trade_dates = sorted(set(trade_dates))
    if target_trade_dates and (
        target_trade_dates[0] < start_date or target_trade_dates[-1] > end_date
    ):
        msg = "trade_dates must be inside the requested date range"
        raise ValueError(msg)

    candidate_security_count = stock_basic.num_rows
    if not target_trade_dates:
        finished_at = time.perf_counter()
        return KHistoryRangeBackfillResult(
            tables={},
            metadata={
                "candidate_security_count": candidate_security_count,
                "selected_security_count": 0,
                "skipped_security_count": candidate_security_count,
                "selected_security_types": dg.MetadataValue.json([]),
                "allowed_security_types": dg.MetadataValue.json(
                    sorted(BAOSTOCK_DAILY_KLINE_SECURITY_TYPES)
                ),
                "backfill_start_date": start_date.isoformat(),
                "backfill_end_date": end_date.isoformat(),
                "processed_trade_dates": dg.MetadataValue.json([]),
                "processed_trade_date_count": 0,
                "partition_row_counts": dg.MetadataValue.json({}),
                "empty_partition_keys": dg.MetadataValue.json([]),
                "duplicate_key_count": 0,
                "min_date": None,
                "max_date": None,
                "uniq_code": 0,
                "max_connections": BAOSTOCK_DAILY_KLINE_CONNECTIONS,
                "max_concurrent_security_requests": BAOSTOCK_DAILY_KLINE_CONNECTIONS,
                "network_failure_count": 0,
                "circuit_breaker_triggered": False,
                "skipped_due_to_circuit_breaker_count": 0,
                "network_failure_stop_threshold": (
                    BAOSTOCK_DAILY_KLINE_NETWORK_FAILURE_STOP_THRESHOLD
                ),
                "baostock_client_start_seconds": 0.0,
                "security_filter_and_task_schedule_seconds": 0.0,
                "baostock_kline_task_wall_seconds": 0.0,
                "kline_table_split_seconds": 0.0,
                "kline_fetch_total_seconds": elapsed_seconds(started_at, finished_at),
            },
        )

    security_ranges = filter_active_security_ranges(
        stock_basic,
        requested_start_date=start_date,
        requested_end_date=end_date,
        allowed_security_types=BAOSTOCK_DAILY_KLINE_SECURITY_TYPES,
    )
    selected_security_types: Counter[str] = Counter(
        security_range.security_type for security_range in security_ranges
    )
    tasks_scheduled_at = time.perf_counter()

    fetched_tables: list[pa.Table] = []
    runner_metadata: dict[str, RawMetadataValue] = {}
    client_started_at = tasks_scheduled_at
    tasks_finished_at = tasks_scheduled_at
    if security_ranges:
        async with client_factory.client(
            max_connections=BAOSTOCK_DAILY_KLINE_CONNECTIONS
        ) as client:
            client_started_at = time.perf_counter()

            async def fetch_one(security_range: SecurityDateRange) -> pa.Table:
                return await _fetch_one_daily_k_table(
                    client,
                    security_range.code,
                    security_range.start_date,
                    security_range.end_date,
                )

            runner_result = await BoundedTaskRunner(
                BoundedTaskOptions(
                    max_concurrent_tasks=BAOSTOCK_DAILY_KLINE_CONNECTIONS,
                    fail_when_all_failed=False,
                    stop_on_error_types=(BaostockNetworkError,),
                    max_failures_before_stop=(BAOSTOCK_DAILY_KLINE_NETWORK_FAILURE_STOP_THRESHOLD),
                )
            ).run(
                security_ranges,
                item_key=lambda security_range: security_range.code,
                worker=fetch_one,
            )
            tasks_finished_at = time.perf_counter()
        _raise_for_kline_request_failures(runner_result)
        fetched_tables = [table for table in runner_result.successes if table.num_rows > 0]
        runner_metadata = _kline_runner_metadata(runner_result)

    combined_table = (
        pa.concat_tables(fetched_tables, promote_options="default")
        if fetched_tables
        else empty_k_history_table()
    )
    split_tables = split_k_history_table_by_trade_date(
        combined_table,
        target_trade_dates=target_trade_dates,
    )
    table_built_at = time.perf_counter()

    partition_row_counts = {
        partition_key: table.num_rows for partition_key, table in split_tables.items()
    }
    empty_partition_keys = [
        partition_key for partition_key, row_count in partition_row_counts.items() if row_count == 0
    ]
    duplicate_key_count = sum(
        duplicate_k_history_key_count(table) for table in split_tables.values()
    )
    if duplicate_key_count:
        msg = f"BaoStock K history range backfill returned {duplicate_key_count} duplicate keys"
        raise RuntimeError(msg)

    return KHistoryRangeBackfillResult(
        tables=split_tables,
        metadata={
            "candidate_security_count": candidate_security_count,
            "selected_security_count": len(security_ranges),
            "skipped_security_count": candidate_security_count - len(security_ranges),
            "selected_security_types": dg.MetadataValue.json(sorted(selected_security_types)),
            "allowed_security_types": dg.MetadataValue.json(
                sorted(BAOSTOCK_DAILY_KLINE_SECURITY_TYPES)
            ),
            "backfill_start_date": start_date.isoformat(),
            "backfill_end_date": end_date.isoformat(),
            "processed_trade_dates": dg.MetadataValue.json(
                [trade_date.isoformat() for trade_date in target_trade_dates]
            ),
            "processed_trade_date_count": len(target_trade_dates),
            "partition_row_counts": dg.MetadataValue.json(partition_row_counts),
            "empty_partition_keys": dg.MetadataValue.json(empty_partition_keys),
            "duplicate_key_count": duplicate_key_count,
            "min_date": _table_min_date(combined_table),
            "max_date": _table_max_date(combined_table),
            "uniq_code": _table_unique_code_count(combined_table),
            "max_connections": BAOSTOCK_DAILY_KLINE_CONNECTIONS,
            "max_concurrent_security_requests": BAOSTOCK_DAILY_KLINE_CONNECTIONS,
            "baostock_client_start_seconds": elapsed_seconds(
                tasks_scheduled_at,
                client_started_at,
            ),
            "security_filter_and_task_schedule_seconds": elapsed_seconds(
                started_at,
                tasks_scheduled_at,
            ),
            "baostock_kline_task_wall_seconds": elapsed_seconds(
                tasks_scheduled_at,
                tasks_finished_at,
            ),
            "kline_table_split_seconds": elapsed_seconds(tasks_finished_at, table_built_at),
            "kline_fetch_total_seconds": elapsed_seconds(started_at, table_built_at),
            **runner_metadata,
        },
    )


async def _fetch_one_daily_k_table(
    client: BaostockClientProtocol,
    code: str,
    start_date: date,
    end_date: date,
) -> pa.Table:
    response = await client.query_history_k_data_plus_daily(code, start_date, end_date)
    return k_history_daily_response_to_table(response)


def _raise_for_kline_request_failures(
    runner_result: BoundedTaskResult[pa.Table],
) -> None:
    if not runner_result.failures:
        return

    network_failure_count = sum(
        1
        for failure in runner_result.failures
        if failure.error_type == BaostockNetworkError.__name__
    )
    failure_sample = "; ".join(
        f"{failure.item_key}: {failure.error_type}: {failure.error_message}"
        for failure in runner_result.failures[:5]
    )
    omitted_count = len(runner_result.failures) - 5
    if omitted_count > 0:
        failure_sample = f"{failure_sample}; ... {omitted_count} more"
    if runner_result.stopped_due_to_failure_threshold:
        msg = (
            "BaoStock K history request circuit breaker stopped scheduling securities "
            f"after {network_failure_count} network failures; "
            f"skipped_due_to_circuit_breaker_count="
            f"{runner_result.skipped_due_to_stop_count}; failures: {failure_sample}"
        )
        raise RuntimeError(msg)

    msg = (
        f"BaoStock K history requests failed for {len(runner_result.failures)} securities; "
        f"network_failure_count={network_failure_count}; failures: {failure_sample}"
    )
    raise RuntimeError(msg)


def _kline_runner_metadata(
    runner_result: BoundedTaskResult[pa.Table],
) -> dict[str, RawMetadataValue]:
    network_failure_count = sum(
        1
        for failure in runner_result.failures
        if failure.error_type == BaostockNetworkError.__name__
    )
    return {
        **runner_result.metadata(item_name="security"),
        "network_failure_count": network_failure_count,
        "circuit_breaker_triggered": runner_result.stopped_due_to_failure_threshold,
        "skipped_due_to_circuit_breaker_count": runner_result.skipped_due_to_stop_count,
        "network_failure_stop_threshold": (BAOSTOCK_DAILY_KLINE_NETWORK_FAILURE_STOP_THRESHOLD),
    }


def _empty_kline_runner_metadata() -> dict[str, RawMetadataValue]:
    return {
        "successful_security_count": 0,
        "failed_security_count": 0,
        "failed_security_keys": dg.MetadataValue.json([]),
        "failed_security_errors": dg.MetadataValue.json({}),
        "task_runner_seconds": 0.0,
        "task_runner_skipped_security_count": 0,
        "task_runner_stopped_due_to_failure_threshold": False,
        "network_failure_count": 0,
        "circuit_breaker_triggered": False,
        "skipped_due_to_circuit_breaker_count": 0,
        "network_failure_stop_threshold": (BAOSTOCK_DAILY_KLINE_NETWORK_FAILURE_STOP_THRESHOLD),
    }


def empty_k_history_table() -> pa.Table:
    return pa.table(
        {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
        schema=K_HISTORY_DAILY_SCHEMA,
    )


def split_k_history_table_by_trade_date(
    table: pa.Table,
    *,
    target_trade_dates: Sequence[date],
) -> dict[str, pa.Table]:
    target_set = set(target_trade_dates)
    rows_by_date: dict[date, list[dict[str, object]]] = {
        trade_date: [] for trade_date in sorted(target_set)
    }
    for row in table.to_pylist():
        row_date = row.get("date")
        if not isinstance(row_date, date):
            msg = f"BaoStock K history row has invalid date value: {row_date!r}"
            raise RuntimeError(msg)
        if row_date not in target_set:
            msg = (
                "BaoStock K history row date is outside the requested trade-date set: "
                f"{row_date.isoformat()}"
            )
            raise RuntimeError(msg)
        rows_by_date[row_date].append(row)

    return {
        trade_date.isoformat(): _k_history_table_from_rows(rows)
        for trade_date, rows in rows_by_date.items()
    }


def duplicate_k_history_key_count(table: pa.Table) -> int:
    seen: set[tuple[date, str]] = set()
    duplicate_count = 0
    for row in table.select(["date", "code"]).to_pylist():
        row_date = row["date"]
        code = row["code"]
        if not isinstance(row_date, date) or not isinstance(code, str):
            continue
        key = (row_date, code)
        if key in seen:
            duplicate_count += 1
        else:
            seen.add(key)
    return duplicate_count


def _k_history_table_from_rows(rows: list[dict[str, object]]) -> pa.Table:
    if not rows:
        return empty_k_history_table()
    return pa.Table.from_pylist(rows, schema=K_HISTORY_DAILY_SCHEMA)


def _table_min_date(table: pa.Table) -> str | None:
    dates = [value for value in table["date"].to_pylist() if isinstance(value, date)]
    if not dates:
        return None
    return min(dates).isoformat()


def _table_max_date(table: pa.Table) -> str | None:
    dates = [value for value in table["date"].to_pylist() if isinstance(value, date)]
    if not dates:
        return None
    return max(dates).isoformat()


def _table_unique_code_count(table: pa.Table) -> int:
    if "code" not in table.column_names:
        return 0
    return len({value for value in table["code"].to_pylist() if isinstance(value, str)})


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
