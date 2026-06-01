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
from scheduler.defs.contract_schemas import (
    CONTRACT_SCHEMA_HASHES,
    CONTRACT_VERSIONS,
    PARQUET_SCHEMA_HASHES,
    PARQUET_SCHEMAS,
    SOURCE_SCHEMA_HASHES,
)
from scheduler.defs.storage.dataset_service import (
    DatasetLocation,
    DatasetWriteOptions,
    S3DatasetService,
)
from scheduler.defs.storage.dataset_writer import partition_column_count
from scheduler.defs.storage.s3 import StorageMode, build_s3_filesystem


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

        s3_config = self._config()
        filesystem = build_s3_filesystem(s3_config)
        filesystem_built_at = time.perf_counter()
        service = S3DatasetService(s3_config=s3_config, filesystem=filesystem)
        location = DatasetLocation(
            bucket=self.bucket,
            object_prefix=self.object_prefix,
            asset_key=asset_key,
        )
        options = DatasetWriteOptions(
            storage_mode=cast(StorageMode, storage_mode),
            allow_empty=allow_empty,
            partition_key_name=partition_key_name,
        )
        if storage_mode == "partitioned":
            if partition_key_name is None:
                msg = "Partitioned S3 output requires partition_key_name metadata"
                raise RuntimeError(msg)
            if not context.has_asset_partitions:
                msg = "Partitioned S3 output requires Dagster asset partitions"
                raise RuntimeError(msg)
            partition_tables = self.validate_partition_tables(obj, allow_empty=allow_empty)
            validated_at = time.perf_counter()
            self.validate_contract_partition_schemas(asset_key, partition_tables)
            partition_keys = set(context.asset_partition_keys)
            table_keys = set(partition_tables)
            if table_keys != partition_keys:
                msg = (
                    "Partitioned S3 output keys must match Dagster asset partition keys: "
                    f"table_keys={sorted(table_keys)}, partition_keys={sorted(partition_keys)}"
                )
                raise RuntimeError(msg)
            write_result = service.write_partitioned(
                location,
                partition_tables,
                options,
            )
        elif storage_mode == "latest_snapshot":
            table = self.validate_table(obj, allow_empty=allow_empty)
            validated_at = time.perf_counter()
            self.validate_contract_schema(asset_key, table)
            write_result = service.write_latest_snapshot(
                location,
                table,
                options,
            )
        else:
            msg = f"Unsupported storage mode: {storage_mode}"
            raise ValueError(msg)
        write_finished_at = time.perf_counter()

        metadata: dict[str, RawMetadataValue] = {
            **service.metadata(result=write_result, options=options),
            **self.contract_metadata(asset_key),
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

        context.add_output_metadata(metadata)

    def load_input(self, context: dg.InputContext) -> Any:
        asset_key = context.asset_key
        if asset_key is None:
            msg = "S3IOManager requires an upstream asset key for input loading"
            raise RuntimeError(msg)

        storage_mode = str(context.definition_metadata.get("storage_mode", "latest_snapshot"))
        partition_key_name = context.definition_metadata.get("partition_key_name")
        if partition_key_name is not None:
            partition_key_name = str(partition_key_name)

        service = S3DatasetService(s3_config=self._config())
        location = DatasetLocation(
            bucket=self.bucket,
            object_prefix=self.object_prefix,
            asset_key=asset_key,
        )
        if storage_mode == "latest_snapshot":
            return service.read_latest_snapshot(location)
        if storage_mode == "partitioned":
            if partition_key_name is None:
                msg = "Partitioned S3 input requires partition_key_name metadata"
                raise RuntimeError(msg)
            if not context.has_asset_partitions:
                msg = "Partitioned S3 input requires Dagster asset partitions"
                raise RuntimeError(msg)
            read_result = service.read_partitioned(
                location,
                partition_keys=context.asset_partition_keys,
                partition_key_name=partition_key_name,
            )
            return dict(zip(read_result.read_partition_keys, read_result.tables, strict=True))

        msg = f"Unsupported S3 input storage mode: {storage_mode}"
        raise NotImplementedError(msg)

    def _asset_key(self, context: dg.OutputContext) -> dg.AssetKey:
        if context.asset_key is None:
            msg = "S3IOManager requires an asset output"
            raise RuntimeError(msg)
        return context.asset_key

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

    def validate_contract_schema(self, asset_key: dg.AssetKey, table: pa.Table) -> None:
        dataset = self.contract_dataset(asset_key)
        expected_schema = self.expected_schema(dataset, asset_key=asset_key)
        if table.schema == expected_schema:
            return
        msg = (
            "S3IOManager Parquet schema mismatch for "
            f"asset={asset_key.to_user_string()}, dataset={dataset}: "
            f"expected schema={expected_schema}, actual schema={table.schema}"
        )
        raise RuntimeError(msg)

    def validate_contract_partition_schemas(
        self,
        asset_key: dg.AssetKey,
        partition_tables: Mapping[str, pa.Table],
    ) -> None:
        dataset = self.contract_dataset(asset_key)
        expected_schema = self.expected_schema(dataset, asset_key=asset_key)
        for partition_key, table in partition_tables.items():
            if table.schema == expected_schema:
                continue
            msg = (
                "S3IOManager Parquet schema mismatch for "
                f"asset={asset_key.to_user_string()}, dataset={dataset}, "
                f"partition_key={partition_key}: expected schema={expected_schema}, "
                f"actual schema={table.schema}"
            )
            raise RuntimeError(msg)

    def contract_metadata(self, asset_key: dg.AssetKey) -> dict[str, RawMetadataValue]:
        dataset = self.contract_dataset(asset_key)
        self.expected_schema(dataset, asset_key=asset_key)
        return {
            "contract_dataset": dataset,
            "contract_version": CONTRACT_VERSIONS[dataset],
            "contract_schema_hash": CONTRACT_SCHEMA_HASHES[dataset],
            "source_schema_hash": SOURCE_SCHEMA_HASHES[dataset],
            "parquet_schema_hash": PARQUET_SCHEMA_HASHES[dataset],
        }

    def contract_dataset(self, asset_key: dg.AssetKey) -> str:
        if not asset_key.path:
            msg = "S3IOManager requires a non-empty asset key for contract schema lookup"
            raise RuntimeError(msg)
        return asset_key.path[-1]

    def expected_schema(self, dataset: str, *, asset_key: dg.AssetKey) -> pa.Schema:
        expected_schema = PARQUET_SCHEMAS.get(dataset)
        if expected_schema is None:
            msg = (
                "S3IOManager could not find generated Parquet schema for "
                f"asset={asset_key.to_user_string()}, dataset={dataset}"
            )
            raise RuntimeError(msg)
        return expected_schema

    def partition_column_count(self, partition_tables: Mapping[str, pa.Table]) -> int:
        return partition_column_count(partition_tables)
