from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from typing import Protocol

import dagster as dg

RawMetadataValue = int | float | str | bool | None | dg.MetadataValue
JsonMetadataValue = Mapping[str, object] | Sequence[object]


class HttpStatsLike(Protocol):
    request_count: int
    retry_count: int
    transient_error_count: int
    http_4xx_count: int
    http_5xx_count: int
    decode_error_count: int

    @property
    def status_code_counts(self) -> Mapping[str, int]: ...

    @property
    def endpoint_host_counts(self) -> Mapping[str, int]: ...


@dataclass
class AssetMetadataBuilder:
    values: dict[str, RawMetadataValue] = field(default_factory=dict)

    def add(self, key: str, value: RawMetadataValue) -> AssetMetadataBuilder:
        self.values[key] = value
        return self

    def add_json(self, key: str, value: JsonMetadataValue) -> AssetMetadataBuilder:
        self.values[key] = dg.MetadataValue.json(value)
        return self

    def extend(self, values: Mapping[str, RawMetadataValue]) -> AssetMetadataBuilder:
        self.values.update(values)
        return self

    def build(self) -> dict[str, RawMetadataValue]:
        return dict(self.values)


def http_stats_metadata(stats: HttpStatsLike) -> dict[str, RawMetadataValue]:
    return FetchStatsMetadataBuilder().build(stats)


@dataclass(frozen=True)
class FetchStatsMetadataBuilder:
    def build(self, stats: HttpStatsLike) -> dict[str, RawMetadataValue]:
        return {
            "request_count": stats.request_count,
            "retry_count": stats.retry_count,
            "transient_error_count": stats.transient_error_count,
            "http_4xx_count": stats.http_4xx_count,
            "http_5xx_count": stats.http_5xx_count,
            "decode_error_count": stats.decode_error_count,
            "status_code_counts": dg.MetadataValue.json(dict(stats.status_code_counts)),
            "endpoint_host_counts": dg.MetadataValue.json(dict(stats.endpoint_host_counts)),
        }


@dataclass(frozen=True)
class FailureMetadataBuilder:
    def build(
        self,
        *,
        item_name: str,
        success_count: int,
        failures: Mapping[str, Mapping[str, str]],
        elapsed_seconds: float,
    ) -> dict[str, RawMetadataValue]:
        failed_item_keys = list(failures)
        return {
            f"successful_{item_name}_count": success_count,
            f"failed_{item_name}_count": len(failures),
            f"failed_{item_name}_keys": dg.MetadataValue.json(failed_item_keys),
            f"failed_{item_name}_errors": dg.MetadataValue.json(failures),
            "task_runner_seconds": elapsed_seconds,
        }


@dataclass(frozen=True)
class DatasetMetadataBuilder:
    def build_s3_write_metadata(
        self,
        *,
        s3_bucket: str,
        s3_endpoint: str,
        s3_keys: Sequence[str],
        row_count: int,
        column_count: int,
        storage_mode: str,
        allow_empty: bool,
        partition_key_name: str | None,
        partition_row_counts: Mapping[str, int],
        empty_partition_keys: Sequence[str],
    ) -> dict[str, RawMetadataValue]:
        metadata: dict[str, RawMetadataValue] = {
            "s3_bucket": s3_bucket,
            "s3_keys": dg.MetadataValue.json(list(s3_keys)),
            "s3_endpoint": s3_endpoint,
            "file_format": "parquet",
            "compression": "zstd",
            "row_count": row_count,
            "column_count": column_count,
            "storage_mode": storage_mode,
            "allow_empty": allow_empty,
        }
        if partition_key_name is not None:
            metadata["partition_key_name"] = partition_key_name
            metadata["partition_row_counts"] = dg.MetadataValue.json(dict(partition_row_counts))
            metadata["empty_partition_keys"] = dg.MetadataValue.json(list(empty_partition_keys))
        elif len(s3_keys) == 1:
            metadata["s3_key"] = s3_keys[0]
        return metadata


@dataclass(frozen=True)
class PartitionRunMetadataBuilder:
    def build_counts(
        self,
        *,
        requested_count: int,
        processed_count: int,
        skipped_count: int,
        completed_count: int,
    ) -> dict[str, RawMetadataValue]:
        return {
            "requested_partition_count": requested_count,
            "processed_partition_count": processed_count,
            "skipped_partition_count": skipped_count,
            "completed_partition_count": completed_count,
        }


def storage_metadata(
    *,
    s3_bucket: str,
    s3_keys: Sequence[str],
    file_format: str = "parquet",
    compression: str = "zstd",
) -> dict[str, RawMetadataValue]:
    return {
        "s3_bucket": s3_bucket,
        "s3_keys": dg.MetadataValue.json(list(s3_keys)),
        "file_format": file_format,
        "compression": compression,
    }
