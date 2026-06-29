import time
from datetime import date

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
)
from scheduler.defs.market.asset_keys import (
    BAOSTOCK_DAILY_K_ASSET_KEY,
    BAOSTOCK_DAILY_K_COMPACTED_ASSET_KEY,
    SINA_TRADE_CALENDAR_ASSET_KEY,
    SOURCE_ASSET_KEY_PREFIX,
)
from scheduler.defs.market.readers import S3SecurityUniverseReader, S3TradeCalendarReader
from scheduler.defs.resources.baostock import BaostockClientFactoryResource
from scheduler.defs.resources.s3 import S3SettingsResource
from scheduler.defs.sources.daily_compact import compact_daily_asset_by_year
from scheduler.defs.sources.sina.trade_calendar import sina__trade_calendar
from scheduler.defs.storage.dataset_service import (
    DatasetLocation,
    DatasetWriteOptions,
    S3DatasetService,
)
from scheduler.defs.storage.dataset_writer import DatasetPartitionWriteError

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


class BaostockDailyKlineRunConfig(dg.Config):
    overwrite_existing_partitions: bool = False
    cutoff_trade_date: str | None = None


class BaostockDailyKlineCompactedRunConfig(dg.Config):
    cutoff_trade_date: str | None = None


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
    config: BaostockDailyKlineRunConfig,
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
        _materialize_daily_kline_partition_selection(
            context,
            config=config,
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
    config: BaostockDailyKlineCompactedRunConfig,
    s3_settings: S3SettingsResource,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    """BaoStock daily K-line trade-date parquet compacted by natural-year partition."""

    refresh_until_trade_date = (
        _parse_partition_date(config.cutoff_trade_date)
        if config.cutoff_trade_date is not None
        else None
    )
    return compact_daily_asset_by_year(
        context,
        raw_asset_key=baostock__query_history_k_data_plus_daily.key,
        output_dataset="baostock__query_history_k_data_plus_daily_compacted",
        s3_settings=s3_settings,
        require_complete_partitions=True,
        unique_key_columns=("date", "code"),
        sort_key_columns=("date", "code"),
        refresh_until_trade_date=refresh_until_trade_date,
        use_latest_existing_partition_as_refresh_until=True,
    )


async def _materialize_daily_kline_partition_selection(
    context: dg.AssetExecutionContext,
    *,
    config: BaostockDailyKlineRunConfig,
    stock_basic: pa.Table,
    s3_settings: S3SettingsResource,
    baostock_client_factory: BaostockClientFactoryResource,
) -> TradeDateRangeMaterializationResult:
    partition_keys = sorted(context.partition_keys)
    if not partition_keys:
        msg = "BaoStock daily K-line materialization requires at least one partition"
        raise RuntimeError(msg)

    natural_dates = [_parse_partition_date(partition_key) for partition_key in partition_keys]
    cutoff_trade_date = (
        _parse_partition_date(config.cutoff_trade_date)
        if config.cutoff_trade_date is not None
        else None
    )
    range_start = natural_dates[0]
    range_end = natural_dates[-1]
    if cutoff_trade_date is not None and cutoff_trade_date > range_end:
        msg = (
            "BaoStock daily K-line cutoff_trade_date cannot be later than "
            f"partition range end: {cutoff_trade_date.isoformat()} > {range_end.isoformat()}"
        )
        raise RuntimeError(msg)
    if cutoff_trade_date is not None and cutoff_trade_date < range_end:
        range_end = cutoff_trade_date
    if range_end < range_start:
        msg = (
            "BaoStock daily K-line cutoff_trade_date cannot be earlier than "
            f"partition range start: {range_end.isoformat()} < {range_start.isoformat()}"
        )
        raise RuntimeError(msg)

    s3_config = s3_settings.config()
    trade_dates = S3TradeCalendarReader.from_s3_config(s3_config).read_trade_dates()
    candidate_dates = [
        natural_date for natural_date in natural_dates if range_start <= natural_date <= range_end
    ]
    target_trade_dates = [
        natural_date for natural_date in candidate_dates if natural_date in trade_dates
    ]
    skipped_non_trade_dates = [
        natural_date for natural_date in candidate_dates if natural_date not in trade_dates
    ]
    skipped_after_cutoff_dates = [
        natural_date for natural_date in natural_dates if natural_date > range_end
    ]

    range_result = await baostock_services.fetch_k_history_tables_for_trade_date_range(
        stock_basic,
        range_start,
        range_end,
        target_trade_dates,
        baostock_client_factory,
    )

    service = S3DatasetService(s3_config=s3_config)
    location = DatasetLocation(
        bucket=s3_config.bucket,
        object_prefix="source",
        asset_key=context.asset_key,
    )
    write_options = DatasetWriteOptions(
        storage_mode="partitioned",
        allow_empty=True,
        partition_key_name=TRADE_DATE_PARTITION_KEY_NAME,
    )

    partition_tables = range_result.tables
    partition_row_counts = {key: table.num_rows for key, table in partition_tables.items()}
    existing_partition_keys: list[str] = []
    written_object_keys: list[str] = []
    if partition_tables:
        existing_partition_keys = service.existing_partition_keys(
            location,
            partition_keys=sorted(partition_tables),
            partition_key_name=TRADE_DATE_PARTITION_KEY_NAME,
        )
        if existing_partition_keys and not config.overwrite_existing_partitions:
            msg = (
                "BaoStock daily K-line materialization refuses to overwrite "
                "existing daily partitions: "
                f"{existing_partition_keys}"
            )
            raise RuntimeError(msg)

        try:
            write_result = service.write_partitioned(
                location,
                partition_tables,
                write_options,
            )
        except DatasetPartitionWriteError as error:
            context.log.error(
                "BaoStock daily K-line partial write repair context: "
                "attempted_partition_keys=%s; written_partition_keys=%s; "
                "failed_partition_keys=%s",
                error.attempted_partition_keys,
                error.written_partition_keys,
                error.failed_partition_keys,
            )
            msg = (
                "BaoStock daily K-line materialization failed while writing "
                "daily partitions. "
                "Validation already passed, so repair the partial write by rerunning "
                "the failed window with overwrite_existing_partitions=true after "
                "recording the old object row counts, ETags, and sizes. "
                f"attempted_partition_keys={error.attempted_partition_keys}; "
                f"written_partition_keys={error.written_partition_keys}; "
                f"failed_partition_keys={error.failed_partition_keys}"
            )
            raise RuntimeError(msg) from error
        written_object_keys = service.object_keys(write_result)
        write_metadata = service.metadata(result=write_result, options=write_options)
    else:
        write_metadata = {
            "s3_bucket": s3_config.bucket,
            "s3_endpoint": s3_config.endpoint,
            "s3_keys": dg.MetadataValue.json([]),
            "file_format": "parquet",
            "compression": "zstd",
            "storage_mode": "partitioned",
            "allow_empty": True,
            "partition_key_name": TRADE_DATE_PARTITION_KEY_NAME,
            "partition_row_counts": dg.MetadataValue.json({}),
            "empty_partition_keys": dg.MetadataValue.json([]),
            "row_count": 0,
            "column_count": 0,
        }

    metadata: dict[str, RawMetadataValue] = {
        **range_result.metadata,
        **write_metadata,
        "request_start_date": range_start.isoformat(),
        "request_end_date": range_end.isoformat(),
        "requested_partition_count": len(partition_keys),
        "processed_partition_count": len(partition_tables),
        "processed_partition_keys": dg.MetadataValue.json(sorted(partition_tables)),
        "skipped_non_trade_partition_count": len(skipped_non_trade_dates),
        "skipped_after_cutoff_partition_count": len(skipped_after_cutoff_dates),
        "skipped_after_cutoff_partition_keys_sample": dg.MetadataValue.json(
            [item.isoformat() for item in skipped_after_cutoff_dates[:20]]
        ),
        "cutoff_trade_date": cutoff_trade_date.isoformat()
        if cutoff_trade_date is not None
        else None,
        "effective_cutoff_trade_date": range_end.isoformat(),
        "overwrite_existing_partitions": config.overwrite_existing_partitions,
        "overwritten_partition_keys": dg.MetadataValue.json(existing_partition_keys),
        "written_s3_keys": dg.MetadataValue.json(sorted(written_object_keys)),
        "missing_partition_count": 0,
        "read_partition_count": len(partition_tables),
        "partition_row_counts": dg.MetadataValue.json(partition_row_counts),
    }
    return TradeDateRangeMaterializationResult(tables=partition_tables, metadata=metadata)


def _parse_partition_date(value: str) -> date:
    try:
        return date.fromisoformat(value)
    except ValueError as error:
        msg = f"Invalid trade_date partition key: {value!r}"
        raise ValueError(msg) from error


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
