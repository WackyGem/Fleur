from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

import dagster as dg
import pyarrow as pa
import pyarrow.parquet as pq
from pyarrow import ArrowException

from scheduler.defs.config.models import S3Config
from scheduler.defs.storage.s3 import (
    StorageMode,
    asset_key_to_parquet_object_key,
    build_s3_filesystem,
)


@dataclass(frozen=True)
class PartitionedParquetReadResult:
    tables: list[pa.Table]
    read_partition_keys: list[str]
    missing_partition_keys: list[str]
    empty_partition_keys: list[str]


def read_parquet_table_from_s3(
    config: S3Config,
    asset_key: dg.AssetKey,
    *,
    partition_key: str | None = None,
    partition_key_name: str | None = None,
    storage_mode: StorageMode = "partitioned",
) -> pa.Table:
    object_key = asset_key_to_parquet_object_key(
        asset_key,
        partition_key=partition_key,
        partition_key_name=partition_key_name,
        storage_mode=storage_mode,
    )
    path = f"{config.bucket}/{object_key}"
    filesystem = build_s3_filesystem(config)
    try:
        with filesystem.open_input_file(path) as source:
            return pq.read_table(source)
    except Exception as error:
        msg = f"Failed to read parquet table from s3://{path}"
        raise RuntimeError(msg) from error


def read_partitioned_parquet_tables_from_s3(
    config: S3Config,
    asset_key: dg.AssetKey,
    *,
    partition_keys: Sequence[str],
    partition_key_name: str,
) -> PartitionedParquetReadResult:
    filesystem = build_s3_filesystem(config)
    tables: list[pa.Table] = []
    read_partition_keys: list[str] = []
    missing_partition_keys: list[str] = []
    empty_partition_keys: list[str] = []

    for partition_key in partition_keys:
        object_key = asset_key_to_parquet_object_key(
            asset_key,
            partition_key=partition_key,
            partition_key_name=partition_key_name,
            storage_mode="partitioned",
        )
        path = f"{config.bucket}/{object_key}"
        try:
            with filesystem.open_input_file(path) as source:
                table = pq.read_table(source)
        except FileNotFoundError:
            missing_partition_keys.append(partition_key)
            continue
        except ArrowException as error:
            if _is_missing_parquet_error(error):
                missing_partition_keys.append(partition_key)
                continue
            msg = f"Failed to read parquet table from s3://{path}"
            raise RuntimeError(msg) from error
        except Exception as error:
            msg = f"Failed to read parquet table from s3://{path}"
            raise RuntimeError(msg) from error

        if table.num_rows == 0:
            empty_partition_keys.append(partition_key)
            continue
        tables.append(table)
        read_partition_keys.append(partition_key)

    return PartitionedParquetReadResult(
        tables=tables,
        read_partition_keys=read_partition_keys,
        missing_partition_keys=missing_partition_keys,
        empty_partition_keys=empty_partition_keys,
    )


def _is_missing_parquet_error(error: ArrowException) -> bool:
    message = str(error).lower()
    return "not found" in message or "path does not exist" in message
