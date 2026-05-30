import asyncio
import time
from collections import Counter
from datetime import date

import dagster as dg
import pyarrow as pa

from scheduler.defs.baostock.client import BaostockAioTcpClient
from scheduler.defs.baostock.schemas import (
    k_history_daily_response_to_table,
    stock_basic_response_to_table,
)
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.asset_keys import (
    BAOSTOCK_DAILY_K_ASSET_KEY,
    SINA_TRADE_CALENDAR_ASSET_KEY,
    SOURCE_ASSET_KEY_PREFIX,
)
from scheduler.defs.market.securities import filter_active_security_ranges
from scheduler.defs.market.trade_calendar import read_trade_dates_from_s3
from scheduler.defs.storage.parquet_readers import read_baostock_stock_basic_from_s3

year_partitions = dg.TimeWindowPartitionsDefinition(
    start="1990",
    fmt="%Y",
    cron_schedule="0 0 1 1 *",
    timezone="Asia/Shanghai",
    end_offset=1,
)
BAOSTOCK_RUN_POOL = "baostock_run_pool"


class KLineDailyYearConfig(dg.Config):
    refresh_until_trade_date: str | None = None


@dg.asset(
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    io_manager_key="s3_io_manager",
    metadata={"storage_mode": "latest_snapshot"},
    pool=BAOSTOCK_RUN_POOL,
    tags={
        "source": "baostock",
        "layer": "source",
        "storage": "s3",
    },
)
def baostock__query_stock_basic() -> dg.MaterializeResult[pa.Table]:
    """Latest BaoStock security basic-information snapshot."""

    table, timing_metadata = asyncio.run(fetch_stock_basic_table())
    return dg.MaterializeResult(
        value=table,
        metadata={
            "row_count": table.num_rows,
            "column_count": table.num_columns,
            "file_format": "parquet",
            **timing_metadata,
        },
    )


@dg.asset(
    key=BAOSTOCK_DAILY_K_ASSET_KEY,
    group_name="s3_sources",
    io_manager_key="s3_io_manager",
    partitions_def=year_partitions,
    deps=[baostock__query_stock_basic, SINA_TRADE_CALENDAR_ASSET_KEY],
    backfill_policy=dg.BackfillPolicy.multi_run(max_partitions_per_run=1),
    metadata={"storage_mode": "partitioned", "partition_key_name": "year"},
    pool=BAOSTOCK_RUN_POOL,
    tags={
        "source": "baostock",
        "layer": "source",
        "storage": "s3",
    },
)
def baostock__query_history_k_data_plus_daily(
    context: dg.AssetExecutionContext,
    config: KLineDailyYearConfig,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    """Daily BaoStock K-line data by yearly partition."""

    asset_started_at = time.perf_counter()
    s3_config = S3Config.from_env()
    config_loaded_at = time.perf_counter()
    trade_dates = read_trade_dates_from_s3(s3_config)
    trade_calendar_read_at = time.perf_counter()
    stock_basic = read_baostock_stock_basic_from_s3(s3_config)
    stock_basic_read_at = time.perf_counter()
    year_ranges = build_year_ranges(context, config, trade_dates)
    year_ranges_built_at = time.perf_counter()
    tables, metadata = asyncio.run(fetch_k_history_tables(stock_basic, year_ranges))
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
            "s3_config_load_seconds": elapsed_seconds(asset_started_at, config_loaded_at),
            "trade_calendar_read_seconds": elapsed_seconds(
                config_loaded_at,
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
                asset_started_at,
                remote_fetch_finished_at,
            ),
        }
    )
    return dg.MaterializeResult(value=tables, metadata=metadata)


async def fetch_stock_basic_table() -> tuple[pa.Table, dict[str, float]]:
    started_at = time.perf_counter()
    async with BaostockAioTcpClient() as client:
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
) -> tuple[dict[str, pa.Table], dict[str, RawMetadataValue]]:
    started_at = time.perf_counter()
    annual_tables: dict[str, list[pa.Table]] = {year: [] for year in year_ranges}
    candidate_security_count = stock_basic.num_rows
    selected_security_counts: dict[str, int] = {}
    skipped_security_counts: dict[str, int] = {}
    selected_security_types: Counter[str] = Counter()

    async with BaostockAioTcpClient(max_connections=30) as client:
        client_started_at = time.perf_counter()
        async with asyncio.TaskGroup() as task_group:
            tasks: list[tuple[str, asyncio.Task[pa.Table]]] = []
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
                    task = task_group.create_task(
                        _fetch_one_daily_k_table(
                            client,
                            year,
                            security_range.code,
                            security_range.start_date,
                            security_range.end_date,
                        )
                    )
                    tasks.append((year, task))
            tasks_scheduled_at = time.perf_counter()

        for year, task in tasks:
            table = task.result()
            if table.num_rows > 0:
                annual_tables[year].append(table)
        tasks_finished_at = time.perf_counter()

    tables: dict[str, pa.Table] = {}
    for year, year_tables in annual_tables.items():
        if year_tables:
            tables[year] = pa.concat_tables(year_tables, promote_options="default")
    table_built_at = time.perf_counter()
    if not tables:
        return {}, {
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
        }

    missing_years = sorted(set(year_ranges) - set(tables))
    if missing_years:
        msg = f"BaoStock daily K-line query returned no rows for partitions: {missing_years}"
        raise RuntimeError(msg)

    metadata: dict[str, RawMetadataValue] = {
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
    }
    return tables, metadata


async def _fetch_one_daily_k_table(
    client: BaostockAioTcpClient,
    year: str,
    code: str,
    start_date: date,
    end_date: date,
) -> pa.Table:
    response = await client.query_history_k_data_plus_daily(code, start_date, end_date)
    return k_history_daily_response_to_table(response)


def build_year_ranges(
    context: dg.AssetExecutionContext,
    config: KLineDailyYearConfig,
    trade_dates: set[date],
) -> dict[str, tuple[date, date]]:
    partition_keys = list(context.partition_keys)
    if not partition_keys:
        msg = "BaoStock daily K-line asset requires at least one year partition"
        raise RuntimeError(msg)

    if config.refresh_until_trade_date is not None:
        if len(partition_keys) != 1:
            msg = "refresh_until_trade_date can only be used with a single year partition"
            raise ValueError(msg)
        partition_key = partition_keys[0]
        refresh_until = date.fromisoformat(config.refresh_until_trade_date)
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

    year_ranges: dict[str, tuple[date, date]] = {}
    for partition_key in partition_keys:
        year_int = int(partition_key)
        year_trade_dates = [trade_date for trade_date in trade_dates if trade_date.year == year_int]
        if not year_trade_dates:
            msg = f"Partition {partition_key} has no trade dates in the Sina trade calendar"
            raise ValueError(msg)
        year_ranges[partition_key] = (date(year_int, 1, 1), date(year_int, 12, 31))
    return year_ranges


def empty_k_history_table() -> pa.Table:
    from scheduler.defs.baostock.schemas import K_HISTORY_DAILY_SCHEMA

    return pa.table(
        {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
        schema=K_HISTORY_DAILY_SCHEMA,
    )
