import time
from datetime import date
from typing import Any, cast

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    compacted_tags,
    compacted_year_metadata,
    daily_sparse_partition_metadata,
    latest_snapshot_metadata,
    s3_parquet_kinds,
    source_owners,
    source_tags,
)
from scheduler.defs.baostock import services as baostock_services
from scheduler.defs.baostock.services import (
    BaostockStockBasicRefreshService,
)
from scheduler.defs.common.async_boundary import run_async_boundary
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.http.partitioning import (
    TRADE_DATE_PARTITION_KEY_NAME,
    TradeDateRangeMaterializationResult,
    materialize_trade_date_range,
)
from scheduler.defs.market.asset_keys import (
    BAOSTOCK_DAILY_K_ASSET_KEY,
    BAOSTOCK_DAILY_K_COMPACTED_ASSET_KEY,
    SINA_TRADE_CALENDAR_ASSET_KEY,
    SOURCE_ASSET_KEY_PREFIX,
)
from scheduler.defs.market.readers import S3SecurityUniverseReader
from scheduler.defs.resources.baostock import BaostockClientFactoryResource
from scheduler.defs.resources.s3 import S3SettingsResource
from scheduler.defs.sources.daily_compact import compact_daily_asset_by_year
from scheduler.defs.sources.sina.trade_calendar import sina__trade_calendar

baostock_daily_kline_partitions = dg.DailyPartitionsDefinition(
    start_date="1990-12-19",
    timezone="Asia/Shanghai",
)
year_partitions = dg.TimeWindowPartitionsDefinition(
    start="1990",
    fmt="%Y",
    cron_schedule="0 0 1 1 *",
    timezone="Asia/Shanghai",
    end_offset=1,
)
BAOSTOCK_RUN_POOL = "baostock_run_pool"


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
    partitions_def=baostock_daily_kline_partitions,
    deps=[baostock__query_stock_basic, SINA_TRADE_CALENDAR_ASSET_KEY],
    backfill_policy=dg.BackfillPolicy.single_run(),
    metadata=daily_sparse_partition_metadata(
        partition_key_name=TRADE_DATE_PARTITION_KEY_NAME,
        trade_date_filter=SINA_TRADE_CALENDAR_ASSET_KEY.to_user_string(),
    ),
    owners=source_owners(),
    kinds=s3_parquet_kinds("tcp"),
    pool=BAOSTOCK_RUN_POOL,
    tags=source_tags("baostock"),
)
def baostock__query_history_k_data_plus_daily(
    context: dg.AssetExecutionContext,
    s3_settings: S3SettingsResource,
    baostock_client_factory: BaostockClientFactoryResource,
) -> dg.MaterializeResult:
    """Daily BaoStock K-line data by trade-date partition."""

    asset_started_at = time.perf_counter()
    s3_config = s3_settings.config()
    config_loaded_at = time.perf_counter()
    stock_basic = S3SecurityUniverseReader.from_s3_config(s3_config).read_stock_basic()
    stock_basic_read_at = time.perf_counter()
    result = run_async_boundary(
        _materialize_daily_kline_range(
            context,
            stock_basic=stock_basic,
            s3_settings=s3_settings,
            baostock_client_factory=baostock_client_factory,
        ),
        context="BaoStock daily K-line trade-date materialization",
    )
    result.metadata["s3_config_load_seconds"] = elapsed_seconds(asset_started_at, config_loaded_at)
    result.metadata["stock_basic_read_seconds"] = elapsed_seconds(
        config_loaded_at,
        stock_basic_read_at,
    )
    result.metadata["asset_function_seconds"] = elapsed_seconds(
        asset_started_at, time.perf_counter()
    )
    return dg.MaterializeResult(metadata=result.metadata)


@dg.asset(
    key=BAOSTOCK_DAILY_K_COMPACTED_ASSET_KEY,
    group_name="s3_sources",
    partitions_def=year_partitions,
    deps=[
        dg.AssetDep(
            baostock__query_history_k_data_plus_daily,
            partition_mapping=dg.TimeWindowPartitionMapping(
                allow_nonexistent_upstream_partitions=True
            ),
        ),
        sina__trade_calendar,
    ],
    io_manager_key="s3_io_manager",
    backfill_policy=dg.BackfillPolicy.multi_run(max_partitions_per_run=1),
    automation_condition=dg.AutomationCondition.eager(),
    metadata=compacted_year_metadata(
        input_partition_key_name=TRADE_DATE_PARTITION_KEY_NAME,
        input_asset=baostock__query_history_k_data_plus_daily.key.to_user_string(),
    ),
    owners=source_owners(),
    kinds=s3_parquet_kinds("compact"),
    pool=BAOSTOCK_RUN_POOL,
    tags=compacted_tags("baostock"),
)
def baostock__query_history_k_data_plus_daily_compacted(
    context: dg.AssetExecutionContext,
    s3_settings: S3SettingsResource,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    """BaoStock daily K-line trade-date parquet compacted by natural-year partition."""

    return compact_daily_asset_by_year(
        context,
        raw_asset_key=baostock__query_history_k_data_plus_daily.key,
        output_dataset="baostock__query_history_k_data_plus_daily_compacted",
        s3_settings=s3_settings,
    )


async def _materialize_daily_kline_range(
    context: dg.AssetExecutionContext,
    *,
    stock_basic: pa.Table,
    s3_settings: S3SettingsResource,
    baostock_client_factory: BaostockClientFactoryResource,
) -> TradeDateRangeMaterializationResult:
    selected_security_count_by_trade_date: dict[str, int] = {}
    skipped_security_count_by_trade_date: dict[str, int] = {}
    selected_security_types: set[str] = set()

    async def fetch_table_for_trade_date(
        trade_date: date,
    ) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
        table, metadata = await baostock_services.fetch_k_history_table_for_trade_date(
            stock_basic,
            trade_date,
            baostock_client_factory,
        )
        trade_date_key = trade_date.isoformat()
        selected_security_count_by_trade_date[trade_date_key] = _metadata_int(
            metadata["selected_security_count"]
        )
        skipped_security_count_by_trade_date[trade_date_key] = _metadata_int(
            metadata["skipped_security_count"]
        )
        selected_security_types.update(_metadata_json_list(metadata["selected_security_types"]))
        return table, metadata

    result = await materialize_trade_date_range(
        context,
        max_concurrent_trade_dates=1,
        fetch_table_for_trade_date=fetch_table_for_trade_date,
        s3_config=s3_settings.config(),
    )
    result.metadata.update(
        {
            "selected_security_count": sum(selected_security_count_by_trade_date.values()),
            "selected_security_count_by_trade_date": dg.MetadataValue.json(
                selected_security_count_by_trade_date
            ),
            "skipped_security_count_by_trade_date": dg.MetadataValue.json(
                skipped_security_count_by_trade_date
            ),
            "selected_security_types": dg.MetadataValue.json(sorted(selected_security_types)),
            "allowed_security_types": dg.MetadataValue.json(
                sorted(baostock_services.BAOSTOCK_DAILY_KLINE_SECURITY_TYPES)
            ),
            "max_connections": baostock_services.BAOSTOCK_DAILY_KLINE_CONNECTIONS,
            "max_concurrent_security_requests": baostock_services.BAOSTOCK_DAILY_KLINE_CONNECTIONS,
        }
    )
    return result


def _metadata_json_list(value: RawMetadataValue) -> list[str]:
    data = getattr(value, "data", value)
    if not isinstance(data, list):
        return []
    return [str(item) for item in cast(list[Any], data)]


def _metadata_int(value: RawMetadataValue) -> int:
    data = getattr(value, "value", value)
    if isinstance(data, bool):
        return int(data)
    if isinstance(data, int | float | str):
        return int(data)
    msg = f"Expected numeric metadata value, got {type(value).__name__}"
    raise TypeError(msg)


async def fetch_stock_basic_table(
    client_factory: baostock_services.BaostockClientFactory,
) -> tuple[pa.Table, dict[str, float]]:
    return await baostock_services.fetch_stock_basic_table(client_factory)


async def fetch_k_history_table_for_trade_date(
    stock_basic: pa.Table,
    trade_date: date,
    client_factory: baostock_services.BaostockClientFactory,
) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
    return await baostock_services.fetch_k_history_table_for_trade_date(
        stock_basic,
        trade_date,
        client_factory,
    )


def empty_k_history_table() -> pa.Table:
    from scheduler.defs.baostock.schemas import K_HISTORY_DAILY_SCHEMA

    return pa.table(
        {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
        schema=K_HISTORY_DAILY_SCHEMA,
    )
