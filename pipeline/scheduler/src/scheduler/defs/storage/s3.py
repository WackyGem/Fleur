from __future__ import annotations

from typing import Any, Literal, cast
from urllib.parse import urlparse

import dagster as dg
import pyarrow.fs as pafs

from scheduler.defs.config.models import S3Config

StorageMode = Literal["partitioned", "latest_snapshot"]
DEFAULT_OBJECT_PREFIX = "source"


def asset_key_to_parquet_object_key(
    asset_key: dg.AssetKey,
    object_prefix: str = DEFAULT_OBJECT_PREFIX,
    partition_key: str | None = None,
    partition_key_name: str | None = None,
    storage_mode: StorageMode = "partitioned",
) -> str:
    stripped_prefix = object_prefix.strip("/")
    asset_path_parts = list(asset_key.path)
    if stripped_prefix and asset_path_parts[:1] == [stripped_prefix]:
        path_parts = asset_path_parts
    else:
        path_parts = [part for part in (stripped_prefix, *asset_path_parts) if part]

    if storage_mode == "partitioned" and partition_key is not None:
        if partition_key_name is None:
            msg = "partition_key_name is required when partition_key is provided"
            raise ValueError(msg)
        path_parts.append(f"{partition_key_name}={partition_key}")
    elif storage_mode not in {"partitioned", "latest_snapshot"}:
        msg = f"Unsupported storage mode: {storage_mode}"
        raise ValueError(msg)

    path_parts.append("000000_0.parquet")
    return "/".join(path_parts)


PyArrowFileSystem = Any


def build_s3_filesystem(config: S3Config) -> PyArrowFileSystem:
    endpoint = config.endpoint
    scheme = None
    if "://" in endpoint:
        parsed_endpoint = urlparse(endpoint)
        scheme = parsed_endpoint.scheme
        endpoint = parsed_endpoint.netloc

    s3_filesystem_factory = cast(Any, pafs).S3FileSystem
    return s3_filesystem_factory(
        access_key=config.access_key,
        secret_key=config.secret_key,
        endpoint_override=endpoint,
        scheme=scheme,
        region=config.region_name,
        allow_bucket_creation=True,
    )


def write_bytes_to_filesystem(
    filesystem: PyArrowFileSystem,
    path: str,
    data: bytes,
) -> None:
    with filesystem.open_output_stream(path) as sink:
        sink.write(data)


def read_bytes_from_filesystem(
    filesystem: PyArrowFileSystem,
    path: str,
) -> bytes:
    with filesystem.open_input_file(path) as source:
        return source.read()
