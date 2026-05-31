import time
from datetime import date

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    latest_snapshot_metadata,
    s3_parquet_kinds,
    source_owners,
    source_tags,
    year_partition_metadata,
)
from scheduler.defs.baostock import services as baostock_services
from scheduler.defs.baostock.services import (
    BaostockDailyKlineRefreshRequest,
    BaostockDailyKlineRefreshService,
    BaostockStockBasicRefreshService,
)
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.market.asset_keys import (
    BAOSTOCK_DAILY_K_ASSET_KEY,
    SINA_TRADE_CALENDAR_ASSET_KEY,
    SOURCE_ASSET_KEY_PREFIX,
)
from scheduler.defs.market.readers import S3SecurityUniverseReader, S3TradeCalendarReader
from scheduler.defs.resources.baostock import BaostockClientFactoryResource
from scheduler.defs.resources.s3 import S3SettingsResource

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
    metadata=latest_snapshot_metadata(),
    owners=source_owners(),
    kinds=s3_parquet_kinds("tcp"),
    pool=BAOSTOCK_RUN_POOL,
    tags=source_tags("baostock"),
)
def baostock__query_stock_basic(
    baostock_client_factory: BaostockClientFactoryResource,
) -> dg.MaterializeResult[pa.Table]:
    """Latest BaoStock security basic-information snapshot."""

    table, timing_metadata = BaostockStockBasicRefreshService(baostock_client_factory).refresh()
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
    metadata=year_partition_metadata(),
    owners=source_owners(),
    kinds=s3_parquet_kinds("tcp"),
    pool=BAOSTOCK_RUN_POOL,
    tags=source_tags("baostock"),
)
def baostock__query_history_k_data_plus_daily(
    context: dg.AssetExecutionContext,
    config: KLineDailyYearConfig,
    s3_settings: S3SettingsResource,
    baostock_client_factory: BaostockClientFactoryResource,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    """Daily BaoStock K-line data by yearly partition."""

    asset_started_at = time.perf_counter()
    s3_config = s3_settings.config()
    config_loaded_at = time.perf_counter()
    result = BaostockDailyKlineRefreshService(
        trade_calendar_reader=S3TradeCalendarReader.from_s3_config(s3_config),
        security_universe_reader=S3SecurityUniverseReader.from_s3_config(s3_config),
        client_factory=baostock_client_factory,
    ).refresh(
        BaostockDailyKlineRefreshRequest(
            partition_keys=list(context.partition_keys),
            refresh_until_trade_date=config.refresh_until_trade_date,
        )
    )
    result.metadata["s3_config_load_seconds"] = elapsed_seconds(asset_started_at, config_loaded_at)
    return dg.MaterializeResult(value=result.tables, metadata=result.metadata)


async def fetch_stock_basic_table(
    client_factory: baostock_services.BaostockClientFactory,
) -> tuple[pa.Table, dict[str, float]]:
    return await baostock_services.fetch_stock_basic_table(client_factory)


async def fetch_k_history_tables(
    stock_basic: pa.Table,
    year_ranges: dict[str, tuple[date, date]],
    client_factory: baostock_services.BaostockClientFactory,
) -> tuple[dict[str, pa.Table], dict[str, RawMetadataValue]]:
    return await baostock_services.fetch_k_history_tables(stock_basic, year_ranges, client_factory)


def build_year_ranges(
    context: dg.AssetExecutionContext,
    config: KLineDailyYearConfig,
    trade_dates: set[date],
) -> dict[str, tuple[date, date]]:
    return baostock_services.build_year_ranges(
        list(context.partition_keys),
        refresh_until_trade_date=config.refresh_until_trade_date,
        trade_dates=trade_dates,
    )


def empty_k_history_table() -> pa.Table:
    from scheduler.defs.baostock.schemas import K_HISTORY_DAILY_SCHEMA

    return pa.table(
        {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
        schema=K_HISTORY_DAILY_SCHEMA,
    )
