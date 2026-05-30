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
from scheduler.defs.storage.dataset_writer import S3DatasetWriter, partition_column_count
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
        writer = S3DatasetWriter(s3_config=self._config(), filesystem=filesystem)
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
            write_result = writer.write_partitioned(
                partition_tables=partition_tables,
                base_dir=base_dir,
                partition_key_name=partition_key_name,
                allow_empty=allow_empty,
            )
        elif storage_mode == "latest_snapshot":
            table = self.validate_table(obj, allow_empty=allow_empty)
            validated_at = time.perf_counter()
            write_result = writer.write_latest_snapshot(
                table=table,
                base_dir=base_dir,
                allow_empty=allow_empty,
            )
        else:
            msg = f"Unsupported storage mode: {storage_mode}"
            raise ValueError(msg)
        write_finished_at = time.perf_counter()

        object_keys = write_result.object_keys(self.bucket)
        metadata: dict[str, RawMetadataValue] = {
            "s3_bucket": self.bucket,
            "s3_keys": dg.MetadataValue.json(object_keys),
            "s3_endpoint": self.endpoint,
            "file_format": "parquet",
            "compression": "zstd",
            "row_count": write_result.row_count,
            "column_count": write_result.column_count,
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
            metadata["partition_row_counts"] = dg.MetadataValue.json(
                write_result.partition_row_counts
            )
            metadata["empty_partition_keys"] = dg.MetadataValue.json(
                write_result.empty_partition_keys
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
        return partition_column_count(partition_tables)
