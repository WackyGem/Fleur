from __future__ import annotations

import time
from collections.abc import Mapping
from typing import Any, cast

import dagster as dg
import pyarrow as pa

from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.config.env import (
    RUSTFS_ACCESS_KEY,
    RUSTFS_BUCKET,
    RUSTFS_ENDPOINT,
    RUSTFS_REGION_NAME,
    RUSTFS_SECRET_KEY,
)
from scheduler.defs.config.models import S3Config
from scheduler.defs.storage.parquet import write_parquet_dataset
from scheduler.defs.storage.s3 import asset_key_to_parquet_object_key, build_s3_filesystem


class S3IOManager(dg.ConfigurableIOManager):
    endpoint: str = RUSTFS_ENDPOINT
    bucket: str = RUSTFS_BUCKET
    access_key: str = RUSTFS_ACCESS_KEY
    secret_key: str = RUSTFS_SECRET_KEY
    region_name: str = RUSTFS_REGION_NAME
    object_prefix: str = "source"

    def handle_output(self, context: dg.OutputContext, obj: Any) -> None:
        started_at = time.perf_counter()
        asset_key = self._asset_key(context)
        storage_mode = str(context.definition_metadata.get("storage_mode", "latest_snapshot"))
        partition_key_name = context.definition_metadata.get("partition_key_name")
        allow_empty = bool(context.definition_metadata.get("allow_empty", False))
        if partition_key_name is not None:
            partition_key_name = str(partition_key_name)

        filesystem = build_s3_filesystem(self._config())
        filesystem_built_at = time.perf_counter()
        base_dir = self._base_dir(asset_key)
        partition_row_counts: dict[str, int] = {}
        if storage_mode == "partitioned":
            if partition_key_name is None:
                msg = "Partitioned S3 output requires partition_key_name metadata"
                raise RuntimeError(msg)
            if not context.has_asset_partitions:
                msg = "Partitioned S3 output requires Dagster asset partitions"
                raise RuntimeError(msg)
            partition_tables = self.validate_partition_tables(obj, allow_empty=allow_empty)
            validated_at = time.perf_counter()
            partition_keys = set(context.asset_partition_keys)
            table_keys = set(partition_tables)
            if table_keys != partition_keys:
                msg = (
                    "Partitioned S3 output keys must match Dagster asset partition keys: "
                    f"table_keys={sorted(table_keys)}, partition_keys={sorted(partition_keys)}"
                )
                raise RuntimeError(msg)
            written_paths = []
            for partition_key in sorted(partition_tables):
                partition_table = partition_tables[partition_key]
                written_paths.extend(
                    write_parquet_dataset(
                        partition_table,
                        base_dir,
                        filesystem,
                        partition_key=partition_key,
                        partition_key_name=partition_key_name,
                        allow_empty=allow_empty,
                    )
                )
                partition_row_counts[partition_key] = partition_table.num_rows
            row_count = sum(partition_row_counts.values())
            column_count = self.partition_column_count(partition_tables)
        elif storage_mode == "latest_snapshot":
            table = self.validate_table(obj, allow_empty=allow_empty)
            validated_at = time.perf_counter()
            written_paths = write_parquet_dataset(
                table, base_dir, filesystem, allow_empty=allow_empty
            )
            row_count = table.num_rows
            column_count = table.num_columns
        else:
            msg = f"Unsupported storage mode: {storage_mode}"
            raise ValueError(msg)
        write_finished_at = time.perf_counter()

        object_keys = [self._path_to_object_key(path) for path in written_paths]
        metadata: dict[str, RawMetadataValue] = {
            "s3_bucket": self.bucket,
            "s3_keys": dg.MetadataValue.json(object_keys),
            "s3_endpoint": self.endpoint,
            "file_format": "parquet",
            "compression": "zstd",
            "row_count": row_count,
            "column_count": column_count,
            "storage_mode": storage_mode,
            "allow_empty": allow_empty,
            "io_manager_validate_seconds": elapsed_seconds(started_at, validated_at),
            "s3_filesystem_build_seconds": elapsed_seconds(
                validated_at,
                filesystem_built_at,
            ),
            "pyarrow_write_dataset_seconds": elapsed_seconds(
                filesystem_built_at,
                write_finished_at,
            ),
            "io_manager_handle_output_seconds": elapsed_seconds(
                started_at,
                write_finished_at,
            ),
        }
        if partition_key_name is not None:
            metadata["partition_key_name"] = partition_key_name
            metadata["partition_row_counts"] = dg.MetadataValue.json(partition_row_counts)
            metadata["empty_partition_keys"] = dg.MetadataValue.json(
                sorted(
                    partition_key
                    for partition_key, partition_row_count in partition_row_counts.items()
                    if partition_row_count == 0
                )
            )
        elif len(object_keys) == 1:
            metadata["s3_key"] = object_keys[0]

        context.add_output_metadata(metadata)

    def load_input(self, context: dg.InputContext) -> Any:
        msg = "S3IOManager does not implement input loading yet"
        raise NotImplementedError(msg)

    def _asset_key(self, context: dg.OutputContext) -> dg.AssetKey:
        if context.asset_key is None:
            msg = "S3IOManager requires an asset output"
            raise RuntimeError(msg)
        return context.asset_key

    def _base_dir(self, asset_key: dg.AssetKey) -> str:
        object_key = asset_key_to_parquet_object_key(
            asset_key,
            object_prefix=self.object_prefix,
            storage_mode="latest_snapshot",
        )
        object_dir = object_key.removesuffix("/000000_0.parquet")
        return f"{self.bucket}/{object_dir}"

    def _path_to_object_key(self, path: str) -> str:
        bucket_prefix = f"{self.bucket}/"
        if path.startswith(bucket_prefix):
            return path.removeprefix(bucket_prefix)
        return path

    def _config(self) -> S3Config:
        return S3Config(
            endpoint=self.endpoint,
            bucket=self.bucket,
            access_key=self.access_key,
            secret_key=self.secret_key,
            region_name=self.region_name,
        )

    def validate_table(self, obj: object, *, allow_empty: bool = False) -> pa.Table:
        if not isinstance(obj, pa.Table):
            msg = "S3IOManager expected a pyarrow.Table"
            raise TypeError(msg)
        table = cast(pa.Table, obj)

        if table.num_rows == 0 and not allow_empty:
            msg = "S3IOManager refuses to write an empty pyarrow.Table"
            raise ValueError(msg)

        return table

    def validate_partition_tables(
        self,
        obj: object,
        *,
        allow_empty: bool = False,
    ) -> dict[str, pa.Table]:
        if not isinstance(obj, Mapping):
            msg = "S3IOManager expected a mapping of partition key to pyarrow.Table"
            raise TypeError(msg)

        tables: dict[str, pa.Table] = {}
        for partition_key, table in obj.items():
            if not isinstance(partition_key, str):
                msg = "Partitioned S3 output keys must be strings"
                raise TypeError(msg)
            tables[partition_key] = self.validate_table(table, allow_empty=allow_empty)

        if not tables:
            msg = "S3IOManager refuses to write an empty partition table mapping"
            raise ValueError(msg)
        return tables

    def partition_column_count(self, partition_tables: Mapping[str, pa.Table]) -> int:
        first_table = next(iter(partition_tables.values()))
        first_columns = first_table.column_names
        for partition_key, table in partition_tables.items():
            if table.column_names != first_columns:
                msg = (
                    f"Partition {partition_key!r} columns differ from first partition: "
                    f"{table.column_names} != {first_columns}"
                )
                raise ValueError(msg)
        return first_table.num_columns
