from __future__ import annotations

from collections.abc import Mapping
from typing import Final

from scheduler.defs.common.metadata import RawMetadataValue

DEFAULT_OWNER: Final = "team:data-platform"

TAG_SOURCE: Final = "source"
TAG_LAYER: Final = "layer"
TAG_STORAGE: Final = "storage"
TAG_STATE: Final = "state"
TAG_MODALITY: Final = "modality"

LAYER_SOURCE: Final = "source"
LAYER_COMPACTED: Final = "compacted"

STORAGE_S3: Final = "s3"
STORAGE_POSTGRES: Final = "postgres"

STATE_POSTGRES: Final = "postgres"
MODALITY_OCR: Final = "ocr"

METADATA_STORAGE_MODE: Final = "storage_mode"
METADATA_PARTITION_KEY_NAME: Final = "partition_key_name"
METADATA_PARTITIONS_DEF: Final = "partitions_def"
METADATA_TRADE_DATE_FILTER: Final = "trade_date_filter"
METADATA_ALLOW_EMPTY: Final = "allow_empty"
METADATA_SPARSE_PARTITION_OUTPUT: Final = "sparse_partition_output"
METADATA_FLATTEN_COLUMN_NAMING: Final = "flatten_column_naming"
METADATA_INPUT_PARTITION_KEY_NAME: Final = "input_partition_key_name"
METADATA_INPUT_ASSET: Final = "input_asset"
METADATA_EXECUTION_ORDERING_DEPENDENCY: Final = "execution_ordering_dependency"
METADATA_EXECUTION_ORDERING_REASON: Final = "execution_ordering_reason"
METADATA_STATE_BACKEND: Final = "state_backend"
METADATA_OBJECT_STORE: Final = "object_store"
METADATA_EXTERNAL_SERVICE: Final = "external_service"

STORAGE_MODE_LATEST_SNAPSHOT: Final = "latest_snapshot"
STORAGE_MODE_PARTITIONED: Final = "partitioned"

PARTITIONS_DEF_DAILY: Final = "daily_partitions"
PARTITIONS_DEF_YEAR: Final = "year_partitions"


def source_tags(source: str, *, storage: str = STORAGE_S3) -> dict[str, str]:
    return {
        TAG_SOURCE: source,
        TAG_LAYER: LAYER_SOURCE,
        TAG_STORAGE: storage,
    }


def compacted_tags(source: str, *, storage: str = STORAGE_S3) -> dict[str, str]:
    return {
        TAG_SOURCE: source,
        TAG_LAYER: LAYER_COMPACTED,
        TAG_STORAGE: storage,
    }


def ocr_source_tags(source: str) -> dict[str, str]:
    return {
        **source_tags(source),
        TAG_STATE: STATE_POSTGRES,
        TAG_MODALITY: MODALITY_OCR,
    }


def latest_snapshot_metadata(
    *,
    flatten_column_naming: str | None = None,
    extra: Mapping[str, RawMetadataValue] | None = None,
) -> dict[str, RawMetadataValue]:
    metadata: dict[str, RawMetadataValue] = {
        METADATA_STORAGE_MODE: STORAGE_MODE_LATEST_SNAPSHOT,
    }
    if flatten_column_naming is not None:
        metadata[METADATA_FLATTEN_COLUMN_NAMING] = flatten_column_naming
    if extra is not None:
        metadata.update(extra)
    return metadata


def year_partition_metadata(
    *,
    partition_key_name: str = "year",
    allow_empty: bool | None = None,
    extra: Mapping[str, RawMetadataValue] | None = None,
) -> dict[str, RawMetadataValue]:
    metadata: dict[str, RawMetadataValue] = {
        METADATA_STORAGE_MODE: STORAGE_MODE_PARTITIONED,
        METADATA_PARTITION_KEY_NAME: partition_key_name,
    }
    if allow_empty is not None:
        metadata[METADATA_ALLOW_EMPTY] = allow_empty
    if extra is not None:
        metadata.update(extra)
    return metadata


def daily_sparse_partition_metadata(
    *,
    partition_key_name: str,
    trade_date_filter: str,
    flatten_column_naming: str,
) -> dict[str, RawMetadataValue]:
    return {
        METADATA_STORAGE_MODE: STORAGE_MODE_PARTITIONED,
        METADATA_PARTITION_KEY_NAME: partition_key_name,
        METADATA_PARTITIONS_DEF: PARTITIONS_DEF_DAILY,
        METADATA_TRADE_DATE_FILTER: trade_date_filter,
        METADATA_ALLOW_EMPTY: True,
        METADATA_SPARSE_PARTITION_OUTPUT: True,
        METADATA_FLATTEN_COLUMN_NAMING: flatten_column_naming,
    }


def compacted_year_metadata(
    *,
    input_partition_key_name: str,
    input_asset: str,
) -> dict[str, RawMetadataValue]:
    return {
        METADATA_STORAGE_MODE: STORAGE_MODE_PARTITIONED,
        METADATA_PARTITION_KEY_NAME: "year",
        METADATA_PARTITIONS_DEF: PARTITIONS_DEF_YEAR,
        METADATA_INPUT_PARTITION_KEY_NAME: input_partition_key_name,
        METADATA_INPUT_ASSET: input_asset,
    }


def stateful_asset_metadata(
    *,
    state_backend: str = STATE_POSTGRES,
    object_store: str = STORAGE_S3,
    external_service: str | None = None,
) -> dict[str, RawMetadataValue]:
    metadata: dict[str, RawMetadataValue] = {
        METADATA_STATE_BACKEND: state_backend,
        METADATA_OBJECT_STORE: object_store,
    }
    if external_service is not None:
        metadata[METADATA_EXTERNAL_SERVICE] = external_service
    return metadata


def generated_endpoint_metadata(
    *,
    ordering_dependency: str | None,
    ordering_reason: str | None = None,
) -> dict[str, RawMetadataValue]:
    if ordering_dependency is None:
        return {}
    return {
        METADATA_EXECUTION_ORDERING_DEPENDENCY: ordering_dependency,
        METADATA_EXECUTION_ORDERING_REASON: ordering_reason or "external_api_rate_limit",
    }


def source_owners() -> list[str]:
    return [DEFAULT_OWNER]


def s3_parquet_kinds(*extra: str) -> set[str]:
    return {"s3", "parquet", *extra}


def stateful_ocr_kinds(*extra: str) -> set[str]:
    return {"s3", "postgres", "ocr", *extra}
