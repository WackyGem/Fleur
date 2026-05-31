from __future__ import annotations

import time
from collections.abc import Awaitable, Callable, Mapping, Sequence
from dataclasses import dataclass
from datetime import date
from typing import Protocol

import dagster as dg
import pyarrow as pa

from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.concurrency import BoundedTaskOptions, BoundedTaskRunner
from scheduler.defs.common.metadata import PartitionRunMetadataBuilder, RawMetadataValue
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.asset_keys import SINA_TRADE_CALENDAR_ASSET_KEY
from scheduler.defs.market.trade_calendar import read_trade_dates_from_s3
from scheduler.defs.partitioning.policies import (
    BackfillLimitPolicy,
    PartitionFilter,
    PartitionSelectionPolicy,
    TradeDateFilterPolicy,
)
from scheduler.defs.storage.dataset_service import (
    DatasetLocation,
    DatasetWriteOptions,
    S3DatasetService,
)

TRADE_DATE_PARTITION_KEY_NAME = "trade_date"

# 回填窗口限制
THS_BACKFILL_MAX_NATURAL_DAYS = 380
JIUYAN_BACKFILL_MAX_TRADE_DATES = 80
jiuyan_action_field_daily_partitions = dg.DailyPartitionsDefinition(
    start_date="2021-01-01",
    timezone="Asia/Shanghai",
)
ths_limit_up_pool_daily_partitions = dg.DailyPartitionsDefinition(
    start_date="2025-01-01",
    timezone="Asia/Shanghai",
)

FetchTableForTradeDate = Callable[
    [date],
    Awaitable[tuple[pa.Table, Mapping[str, RawMetadataValue]]],
]
FetchTableForPartition = Callable[
    [str],
    Awaitable[tuple[pa.Table, Mapping[str, RawMetadataValue]]],
]


class PartitionedAssetContextProtocol(Protocol):
    @property
    def partition_keys(self) -> Sequence[str]: ...

    @property
    def asset_key(self) -> dg.AssetKey: ...


@dataclass(frozen=True)
class TradeDateRangeMaterializationResult:
    tables: dict[str, pa.Table]
    metadata: dict[str, RawMetadataValue]


@dataclass(frozen=True)
class PartitionRangeMaterializationResult:
    tables: dict[str, pa.Table]
    metadata: dict[str, RawMetadataValue]


@dataclass(frozen=True)
class PartitionWriteResult:
    partition_key: str
    table: pa.Table
    metadata: Mapping[str, RawMetadataValue]
    object_keys: list[str]


async def materialize_partition_range(
    context: PartitionedAssetContextProtocol,
    *,
    partition_key_name: str,
    max_concurrent_partitions: int,
    fetch_table_for_partition: FetchTableForPartition,
    partition_filter: PartitionFilter | None = None,
    partitions_source_asset: str | None = None,
    backfill_hard_limit: int | None = None,
    s3_config: S3Config,
) -> PartitionRangeMaterializationResult:
    if max_concurrent_partitions < 1:
        msg = "max_concurrent_partitions must be positive"
        raise ValueError(msg)

    partition_keys = sorted(context.partition_keys)
    if not partition_keys:
        msg = "Partitioned asset requires at least one partition"
        raise RuntimeError(msg)
    BackfillLimitPolicy(backfill_hard_limit).validate(partition_keys)

    processed_partition_keys, skipped_partition_keys = PartitionSelectionPolicy(
        partition_filter
    ).select(partition_keys)

    service = S3DatasetService(s3_config=s3_config)
    location = DatasetLocation(
        bucket=s3_config.bucket,
        object_prefix="source",
        asset_key=context.asset_key,
    )
    write_options = DatasetWriteOptions(
        storage_mode="partitioned",
        allow_empty=True,
        partition_key_name=partition_key_name,
    )
    started_at = time.perf_counter()

    async def fetch_and_write(partition_key: str) -> PartitionWriteResult:
        table, metadata = await fetch_table_for_partition(partition_key)
        write_result = service.write_partitioned(
            location,
            {partition_key: table},
            write_options,
        )
        return PartitionWriteResult(
            partition_key=partition_key,
            table=table,
            metadata=dict(metadata),
            object_keys=service.object_keys(write_result),
        )

    runner_result = await BoundedTaskRunner(
        BoundedTaskOptions(
            max_concurrent_tasks=max_concurrent_partitions,
            preserve_order=True,
        )
    ).run(
        processed_partition_keys,
        item_key=str,
        worker=fetch_and_write,
    )

    finished_at = time.perf_counter()
    completed = {
        partition_result.partition_key: partition_result.table
        for partition_result in runner_result.successes
    }
    per_partition_metadata = {
        partition_result.partition_key: partition_result.metadata
        for partition_result in runner_result.successes
    }
    written_object_keys = [
        object_key
        for partition_result in runner_result.successes
        for object_key in partition_result.object_keys
    ]
    row_counts = {key: table.num_rows for key, table in completed.items()}
    column_counts = {key: table.num_columns for key, table in completed.items()}
    metadata: dict[str, RawMetadataValue] = {
        "backfill_start_partition_key": partition_keys[0],
        "backfill_end_partition_key": partition_keys[-1],
        "max_concurrent_partitions": max_concurrent_partitions,
        "partition_keys": dg.MetadataValue.json(partition_keys),
        "processed_partition_keys": dg.MetadataValue.json(processed_partition_keys),
        "skipped_partition_keys_sample": dg.MetadataValue.json(skipped_partition_keys[:20]),
        "partition_key_name": partition_key_name,
        "s3_bucket": s3_config.bucket,
        "s3_keys": dg.MetadataValue.json(sorted(written_object_keys)),
        "file_format": "parquet",
        "compression": "zstd",
        "storage_mode": "partitioned",
        "allow_empty": True,
        "row_count": sum(row_counts.values()),
        "column_count": max(column_counts.values(), default=0),
        "partition_row_counts": dg.MetadataValue.json(row_counts),
        "partition_metadata": dg.MetadataValue.json(_json_safe_mapping(per_partition_metadata)),
        "asset_function_seconds": elapsed_seconds(started_at, finished_at),
    }
    metadata.update(
        PartitionRunMetadataBuilder().build_counts(
            requested_count=len(partition_keys),
            processed_count=len(processed_partition_keys),
            skipped_count=len(skipped_partition_keys),
            completed_count=len(completed),
        )
    )
    metadata.update(runner_result.metadata(item_name="partition"))
    if partitions_source_asset is not None:
        metadata["partitions_source_asset"] = partitions_source_asset
    return PartitionRangeMaterializationResult(tables=completed, metadata=metadata)


async def materialize_trade_date_range(
    context: PartitionedAssetContextProtocol,
    *,
    max_concurrent_trade_dates: int,
    fetch_table_for_trade_date: FetchTableForTradeDate,
    backfill_window_limit: int | None = None,
    s3_config: S3Config,
) -> TradeDateRangeMaterializationResult:
    if max_concurrent_trade_dates < 1:
        msg = "max_concurrent_trade_dates must be positive"
        raise ValueError(msg)

    partition_keys = sorted(context.partition_keys)
    if not partition_keys:
        msg = "Market-event asset requires at least one trade_date partition"
        raise RuntimeError(msg)

    natural_dates = [_parse_date_partition_key(key) for key in partition_keys]
    calendar_dates = read_trade_dates_from_s3(s3_config)
    trade_dates, skipped_window_trade_dates, skipped_non_trade_dates = TradeDateFilterPolicy(
        calendar_dates=calendar_dates,
        backfill_limit=BackfillLimitPolicy(backfill_window_limit),
    ).select(natural_dates)
    requested_trade_dates = [item for item in natural_dates if item in calendar_dates]
    trade_date_keys = {item.isoformat() for item in trade_dates}

    async def fetch_trade_date_partition(
        partition_key: str,
    ) -> tuple[pa.Table, Mapping[str, RawMetadataValue]]:
        return await fetch_table_for_trade_date(_parse_date_partition_key(partition_key))

    try:
        result = await materialize_partition_range(
            context,
            partition_key_name=TRADE_DATE_PARTITION_KEY_NAME,
            max_concurrent_partitions=max_concurrent_trade_dates,
            fetch_table_for_partition=fetch_trade_date_partition,
            partition_filter=lambda partition_key: partition_key in trade_date_keys,
            partitions_source_asset=SINA_TRADE_CALENDAR_ASSET_KEY.to_user_string(),
            s3_config=s3_config,
        )
    except RuntimeError as error:
        prefix = "Partition range materialization failed for "
        if not str(error).startswith(prefix):
            raise
        msg = str(error).replace(prefix, "Trade-date range materialization failed for ", 1)
        raise RuntimeError(msg) from error

    processed_trade_dates = [item.isoformat() for item in trade_dates]
    skipped_window_trade_date_keys = [item.isoformat() for item in skipped_window_trade_dates]
    skipped_non_trade_date_keys = [item.isoformat() for item in skipped_non_trade_dates]
    metadata = result.metadata
    metadata.update(
        {
            "backfill_start_date": partition_keys[0],
            "backfill_end_date": partition_keys[-1],
            "requested_natural_date_count": len(natural_dates),
            "requested_trade_date_count": len(requested_trade_dates),
            "processed_trade_date_count": len(trade_dates),
            "skipped_window_trade_date_count": len(skipped_window_trade_dates),
            "skipped_non_trade_date_count": len(skipped_non_trade_dates),
            "completed_trade_date_count": len(result.tables),
            "failed_trade_date_count": metadata["failed_partition_count"],
            "max_concurrent_trade_dates": max_concurrent_trade_dates,
            "processed_trade_dates": dg.MetadataValue.json(processed_trade_dates),
            "skipped_window_trade_dates_sample": dg.MetadataValue.json(
                skipped_window_trade_date_keys[:20]
            ),
            "skipped_non_trade_dates_sample": dg.MetadataValue.json(
                skipped_non_trade_date_keys[:20]
            ),
            "request_trade_date": dg.MetadataValue.json(processed_trade_dates),
            "partitions_source_asset": SINA_TRADE_CALENDAR_ASSET_KEY.to_user_string(),
        }
    )
    return TradeDateRangeMaterializationResult(tables=result.tables, metadata=metadata)


def _parse_date_partition_key(partition_key: str) -> date:
    try:
        return date.fromisoformat(partition_key)
    except ValueError as error:
        msg = f"Invalid natural-date partition key: {partition_key!r}"
        raise ValueError(msg) from error


def _json_safe(value: object) -> object:
    if isinstance(value, dg.MetadataValue):
        return str(value)
    if isinstance(value, Mapping):
        return {str(key): _json_safe(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_json_safe(item) for item in value]
    if isinstance(value, date):
        return value.isoformat()
    return value


def _json_safe_mapping(value: Mapping[str, object]) -> dict[str, object]:
    return {str(key): _json_safe(item) for key, item in value.items()}
