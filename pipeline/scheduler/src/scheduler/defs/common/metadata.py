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


def http_stats_metadata(stats: HttpStatsLike) -> dict[str, int]:
    return {
        "request_count": stats.request_count,
        "retry_count": stats.retry_count,
        "transient_error_count": stats.transient_error_count,
        "http_4xx_count": stats.http_4xx_count,
        "http_5xx_count": stats.http_5xx_count,
        "decode_error_count": stats.decode_error_count,
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
