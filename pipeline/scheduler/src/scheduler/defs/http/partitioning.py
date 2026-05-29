from __future__ import annotations

import asyncio
import time
from collections.abc import Awaitable, Callable, Mapping, Sequence
from dataclasses import dataclass
from datetime import date
from typing import Protocol

import dagster as dg
import pyarrow as pa

from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.asset_keys import SINA_TRADE_CALENDAR_ASSET_KEY
from scheduler.defs.market.trade_calendar import read_trade_dates_from_s3
from scheduler.defs.storage.parquet import write_parquet_dataset
from scheduler.defs.storage.s3 import asset_key_to_parquet_object_key, build_s3_filesystem

TRADE_DATE_PARTITION_KEY_NAME = "trade_date"
TRADE_DATE_BACKFILL_HARD_LIMIT = 20
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
PartitionFilter = Callable[[str], bool]


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


async def materialize_partition_range(
    context: PartitionedAssetContextProtocol,
    *,
    partition_key_name: str,
    max_concurrent_partitions: int,
    fetch_table_for_partition: FetchTableForPartition,
    partition_filter: PartitionFilter | None = None,
    partitions_source_asset: str | None = None,
    backfill_hard_limit: int | None = None,
    s3_config: S3Config | None = None,
) -> PartitionRangeMaterializationResult:
    if max_concurrent_partitions < 1:
        msg = "max_concurrent_partitions must be positive"
        raise ValueError(msg)

    partition_keys = sorted(context.partition_keys)
    if not partition_keys:
        msg = "Partitioned asset requires at least one partition"
        raise RuntimeError(msg)
    if backfill_hard_limit is not None and len(partition_keys) > backfill_hard_limit:
        msg = f"Single-run partition backfill is limited to {backfill_hard_limit} partitions"
        raise ValueError(msg)

    effective_s3_config = s3_config or S3Config.from_env()
    processed_partition_keys = [
        partition_key
        for partition_key in partition_keys
        if partition_filter is None or partition_filter(partition_key)
    ]
    skipped_partition_keys = [
        partition_key
        for partition_key in partition_keys
        if partition_key not in processed_partition_keys
    ]

    filesystem = build_s3_filesystem(effective_s3_config)
    base_dir = _asset_base_dir(effective_s3_config, context.asset_key)
    semaphore = asyncio.Semaphore(max_concurrent_partitions)
    completed: dict[str, pa.Table] = {}
    failed_count = 0
    per_partition_metadata: dict[str, Mapping[str, RawMetadataValue]] = {}
    written_object_keys: list[str] = []
    started_at = time.perf_counter()

    async def fetch_and_write(partition_key: str) -> None:
        nonlocal failed_count
        async with semaphore:
            try:
                table, metadata = await fetch_table_for_partition(partition_key)
                written_paths = write_parquet_dataset(
                    table,
                    base_dir,
                    filesystem,
                    partition_key=partition_key,
                    partition_key_name=partition_key_name,
                    allow_empty=True,
                )
            except Exception:
                failed_count += 1
                raise

        completed[partition_key] = table
        per_partition_metadata[partition_key] = dict(metadata)
        written_object_keys.extend(
            _path_to_object_key(effective_s3_config.bucket, path) for path in written_paths
        )

    try:
        async with asyncio.TaskGroup() as task_group:
            for partition_key in processed_partition_keys:
                task_group.create_task(fetch_and_write(partition_key))
    except ExceptionGroup as error:
        details = "; ".join(
            f"{type(exception).__name__}: {exception}" for exception in error.exceptions
        )
        msg = (
            "Partition range materialization failed for "
            f"{context.asset_key.to_user_string()} "
            f"({partition_keys[0]}...{partition_keys[-1]}): {details}"
        )
        raise RuntimeError(msg) from error

    finished_at = time.perf_counter()
    row_counts = {key: table.num_rows for key, table in completed.items()}
    column_counts = {key: table.num_columns for key, table in completed.items()}
    metadata: dict[str, RawMetadataValue] = {
        "backfill_start_partition_key": partition_keys[0],
        "backfill_end_partition_key": partition_keys[-1],
        "requested_partition_count": len(partition_keys),
        "processed_partition_count": len(processed_partition_keys),
        "skipped_partition_count": len(skipped_partition_keys),
        "completed_partition_count": len(completed),
        "failed_partition_count": failed_count,
        "max_concurrent_partitions": max_concurrent_partitions,
        "partition_keys": dg.MetadataValue.json(partition_keys),
        "processed_partition_keys": dg.MetadataValue.json(processed_partition_keys),
        "skipped_partition_keys_sample": dg.MetadataValue.json(skipped_partition_keys[:20]),
        "partition_key_name": partition_key_name,
        "s3_bucket": effective_s3_config.bucket,
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
    if partitions_source_asset is not None:
        metadata["partitions_source_asset"] = partitions_source_asset
    return PartitionRangeMaterializationResult(tables=completed, metadata=metadata)


async def materialize_trade_date_range(
    context: PartitionedAssetContextProtocol,
    *,
    max_concurrent_trade_dates: int,
    fetch_table_for_trade_date: FetchTableForTradeDate,
) -> TradeDateRangeMaterializationResult:
    if max_concurrent_trade_dates < 1:
        msg = "max_concurrent_trade_dates must be positive"
        raise ValueError(msg)
    if max_concurrent_trade_dates > TRADE_DATE_BACKFILL_HARD_LIMIT:
        msg = f"max_concurrent_trade_dates must be <= {TRADE_DATE_BACKFILL_HARD_LIMIT}"
        raise ValueError(msg)

    partition_keys = sorted(context.partition_keys)
    if not partition_keys:
        msg = "Market-event asset requires at least one trade_date partition"
        raise RuntimeError(msg)
    if len(partition_keys) > TRADE_DATE_BACKFILL_HARD_LIMIT:
        msg = (
            "Single-run market-event backfill is limited to "
            f"{TRADE_DATE_BACKFILL_HARD_LIMIT} natural-date partitions"
        )
        raise ValueError(msg)

    s3_config = S3Config.from_env()
    natural_dates = [_parse_date_partition_key(key) for key in partition_keys]
    calendar_dates = read_trade_dates_from_s3(s3_config)
    trade_dates = [item for item in natural_dates if item in calendar_dates]
    skipped_non_trade_dates = [item for item in natural_dates if item not in calendar_dates]
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
            backfill_hard_limit=TRADE_DATE_BACKFILL_HARD_LIMIT,
            s3_config=s3_config,
        )
    except RuntimeError as error:
        prefix = "Partition range materialization failed for "
        if not str(error).startswith(prefix):
            raise
        msg = str(error).replace(prefix, "Trade-date range materialization failed for ", 1)
        raise RuntimeError(msg) from error

    processed_trade_dates = [item.isoformat() for item in trade_dates]
    skipped_non_trade_date_keys = [item.isoformat() for item in skipped_non_trade_dates]
    metadata = result.metadata
    metadata.update(
        {
            "backfill_start_date": partition_keys[0],
            "backfill_end_date": partition_keys[-1],
            "requested_natural_date_count": len(natural_dates),
            "requested_trade_date_count": len(trade_dates),
            "processed_trade_date_count": len(trade_dates),
            "skipped_non_trade_date_count": len(skipped_non_trade_dates),
            "completed_trade_date_count": len(result.tables),
            "failed_trade_date_count": metadata["failed_partition_count"],
            "max_concurrent_trade_dates": max_concurrent_trade_dates,
            "processed_trade_dates": dg.MetadataValue.json(processed_trade_dates),
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


def _asset_base_dir(config: S3Config, asset_key: dg.AssetKey) -> str:
    object_key = asset_key_to_parquet_object_key(
        asset_key,
        object_prefix="source",
        storage_mode="latest_snapshot",
    )
    object_dir = object_key.removesuffix("/000000_0.parquet")
    return f"{config.bucket}/{object_dir}"


def _path_to_object_key(bucket: str, path: str) -> str:
    bucket_prefix = f"{bucket}/"
    if path.startswith(bucket_prefix):
        return path.removeprefix(bucket_prefix)
    return path


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
