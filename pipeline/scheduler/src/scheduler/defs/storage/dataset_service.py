from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

import dagster as dg
import pyarrow as pa

from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.config.models import S3Config
from scheduler.defs.storage.dataset_writer import (
    DatasetWriteResult,
    S3DatasetWriter,
)
from scheduler.defs.storage.parquet_readers import (
    PartitionedParquetReadResult,
    read_parquet_table_from_s3,
    read_partitioned_parquet_tables_from_s3,
)
from scheduler.defs.storage.s3 import (
    PyArrowFileSystem,
    StorageMode,
    asset_key_to_parquet_object_key,
)


@dataclass(frozen=True)
class DatasetLocation:
    bucket: str
    object_prefix: str
    asset_key: dg.AssetKey


@dataclass(frozen=True)
class DatasetWriteOptions:
    storage_mode: StorageMode
    allow_empty: bool = False
    partition_key_name: str | None = None


class S3DatasetService:
    def __init__(
        self,
        *,
        s3_config: S3Config,
        filesystem: PyArrowFileSystem | None = None,
    ) -> None:
        self._s3_config = s3_config
        self._writer = S3DatasetWriter(s3_config=s3_config, filesystem=filesystem)

    def write_latest_snapshot(
        self,
        location: DatasetLocation,
        table: pa.Table,
        options: DatasetWriteOptions,
    ) -> DatasetWriteResult:
        if options.storage_mode != "latest_snapshot":
            msg = f"Latest snapshot write received unsupported mode: {options.storage_mode}"
            raise ValueError(msg)
        return self._writer.write_latest_snapshot(
            table=table,
            base_dir=self._base_dir(location),
            allow_empty=options.allow_empty,
        )

    def write_partitioned(
        self,
        location: DatasetLocation,
        tables: Mapping[str, pa.Table],
        options: DatasetWriteOptions,
    ) -> DatasetWriteResult:
        if options.storage_mode != "partitioned":
            msg = f"Partitioned write received unsupported mode: {options.storage_mode}"
            raise ValueError(msg)
        if options.partition_key_name is None:
            msg = "Partitioned S3 dataset write requires partition_key_name"
            raise RuntimeError(msg)
        return self._writer.write_partitioned(
            partition_tables=tables,
            base_dir=self._base_dir(location),
            partition_key_name=options.partition_key_name,
            allow_empty=options.allow_empty,
        )

    def read_latest_snapshot(self, location: DatasetLocation) -> pa.Table:
        return read_parquet_table_from_s3(
            self._s3_config,
            location.asset_key,
            storage_mode="latest_snapshot",
        )

    def read_partitioned(
        self,
        location: DatasetLocation,
        *,
        partition_keys: Sequence[str],
        partition_key_name: str,
    ) -> PartitionedParquetReadResult:
        return read_partitioned_parquet_tables_from_s3(
            self._s3_config,
            location.asset_key,
            partition_keys=partition_keys,
            partition_key_name=partition_key_name,
        )

    def metadata(
        self,
        *,
        result: DatasetWriteResult,
        options: DatasetWriteOptions,
    ) -> dict[str, RawMetadataValue]:
        object_keys = result.object_keys(self._s3_config.bucket)
        metadata: dict[str, RawMetadataValue] = {
            "s3_bucket": self._s3_config.bucket,
            "s3_keys": dg.MetadataValue.json(object_keys),
            "s3_endpoint": self._s3_config.endpoint,
            "file_format": "parquet",
            "compression": "zstd",
            "row_count": result.row_count,
            "column_count": result.column_count,
            "storage_mode": options.storage_mode,
            "allow_empty": options.allow_empty,
        }
        if options.partition_key_name is not None:
            metadata["partition_key_name"] = options.partition_key_name
            metadata["partition_row_counts"] = dg.MetadataValue.json(result.partition_row_counts)
            metadata["empty_partition_keys"] = dg.MetadataValue.json(result.empty_partition_keys)
        elif len(object_keys) == 1:
            metadata["s3_key"] = object_keys[0]
        return metadata

    def object_keys(self, result: DatasetWriteResult) -> list[str]:
        return result.object_keys(self._s3_config.bucket)

    def _base_dir(self, location: DatasetLocation) -> str:
        object_key = asset_key_to_parquet_object_key(
            location.asset_key,
            object_prefix=location.object_prefix,
            storage_mode="latest_snapshot",
        )
        object_dir = object_key.removesuffix("/000000_0.parquet")
        return f"{location.bucket}/{object_dir}"
