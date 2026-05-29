import asyncio
import time
from collections import Counter
from datetime import date
from typing import Any

import dagster as dg
import pyarrow as pa

from scheduler.defs.baostock.assets import baostock__query_stock_basic, year_partitions
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.securities import filter_active_security_ranges
from scheduler.defs.sources.eastmoney.client import (
    EASTMONEY_CODE_CONCURRENCY,
    EastmoneyAioHttpClient,
)
from scheduler.defs.sources.eastmoney.schema import (
    ENDPOINT_CONFIGS,
    EastmoneyEndpointConfig,
    EastmoneyFetchedRow,
    eastmoney_rows_to_table,
    empty_eastmoney_table,
)
from scheduler.defs.storage.parquet_readers import read_baostock_stock_basic_from_s3

EASTMONEY_RUN_POOL = "eastmoney_run_pool"
EASTMONEY_ASSET_METADATA: dict[str, str | bool] = {
    "storage_mode": "partitioned",
    "partition_key_name": "year",
    "allow_empty": True,
}


class EastmoneyYearConfig(dg.Config):
    refresh_until_date: str | None = None


def build_eastmoney_asset(
    endpoint: EastmoneyEndpointConfig,
    ordering_dependency: dg.AssetsDefinition | None = None,
) -> dg.AssetsDefinition:
    deps = [baostock__query_stock_basic]
    metadata: dict[str, str | bool] = dict(EASTMONEY_ASSET_METADATA)
    if ordering_dependency is not None:
        deps.append(ordering_dependency)
        metadata["execution_ordering_dependency"] = ordering_dependency.key.path[-1]

    def materialize(
        context: dg.AssetExecutionContext,
        config: EastmoneyYearConfig,
    ) -> dg.MaterializeResult[dict[str, pa.Table]]:
        return _materialize_eastmoney_asset(context, config, endpoint)

    materialize.__name__ = endpoint.asset_name
    materialize.__doc__ = f"EastMoney F10 rows for {endpoint.asset_name} by natural-year partition."

    return dg.asset(
        name=endpoint.asset_name,
        group_name="eastmoney",
        io_manager_key="s3_io_manager",
        partitions_def=year_partitions,
        deps=deps,
        backfill_policy=dg.BackfillPolicy.multi_run(max_partitions_per_run=1),
        metadata=metadata,
        pool=EASTMONEY_RUN_POOL,
        tags={"source": "eastmoney", "layer": "raw", "storage": "s3"},
    )(materialize)


EASTMONEY_ASSETS_BY_NAME: dict[str, dg.AssetsDefinition] = {}
_previous_eastmoney_asset: dg.AssetsDefinition | None = None
for _endpoint in ENDPOINT_CONFIGS:
    _eastmoney_asset = build_eastmoney_asset(_endpoint, _previous_eastmoney_asset)
    EASTMONEY_ASSETS_BY_NAME[_endpoint.asset_name] = _eastmoney_asset
    globals()[_endpoint.asset_name] = _eastmoney_asset
    _previous_eastmoney_asset = _eastmoney_asset

EASTMONEY_ASSETS = list(EASTMONEY_ASSETS_BY_NAME.values())


def _materialize_eastmoney_asset(
    context: dg.AssetExecutionContext,
    config: EastmoneyYearConfig,
    endpoint: EastmoneyEndpointConfig,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    asset_started_at = time.perf_counter()
    s3_config = S3Config.from_env()
    config_loaded_at = time.perf_counter()
    stock_basic = read_baostock_stock_basic_from_s3(s3_config)
    stock_basic_read_at = time.perf_counter()
    year_ranges = _build_year_ranges(context, config)
    year_ranges_built_at = time.perf_counter()
    tables, metadata = asyncio.run(_fetch_eastmoney_tables(endpoint, stock_basic, year_ranges))
    remote_fetch_finished_at = time.perf_counter()

    row_count = sum(table.num_rows for table in tables.values())
    column_count = next(iter(tables.values())).num_columns
    metadata.update(
        {
            "row_count": row_count,
            "column_count": column_count,
            "partition_keys": dg.MetadataValue.json(sorted(year_ranges)),
            "selected_date_field": endpoint.date_field,
            "sort_columns": ",".join(endpoint.sort_fields),
            "sort_types": ",".join(endpoint.sort_directions),
            "source_endpoint": endpoint.source_endpoint,
            "code_concurrency_limit": EASTMONEY_CODE_CONCURRENCY,
            "s3_config_load_seconds": elapsed_seconds(asset_started_at, config_loaded_at),
            "stock_basic_read_seconds": elapsed_seconds(
                config_loaded_at,
                stock_basic_read_at,
            ),
            "year_ranges_build_seconds": elapsed_seconds(
                stock_basic_read_at,
                year_ranges_built_at,
            ),
            "eastmoney_remote_fetch_seconds": elapsed_seconds(
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


async def _fetch_eastmoney_tables(
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


def _build_year_ranges(
    context: dg.AssetExecutionContext,
    config: EastmoneyYearConfig,
) -> dict[str, tuple[date, date]]:
    partition_keys = list(context.partition_keys)
    if not partition_keys:
        msg = "EastMoney F10 asset requires at least one year partition"
        raise RuntimeError(msg)

    if config.refresh_until_date is not None:
        if len(partition_keys) != 1:
            msg = "refresh_until_date can only be used with a single year partition"
            raise ValueError(msg)
        partition_key = partition_keys[0]
        refresh_until = date.fromisoformat(config.refresh_until_date)
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
