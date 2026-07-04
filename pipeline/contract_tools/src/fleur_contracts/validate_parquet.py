from __future__ import annotations

import os
from pathlib import Path
from typing import Any, cast

import pyarrow.parquet as pq

from fleur_contracts.env import load_repo_dotenv_if_present, local_rustfs_endpoint
from fleur_contracts.loader import DEFAULT_CONTRACT_ROOT, load_registry
from fleur_contracts.schema import DatasetContract


def validate_available_parquet(contract_root: Path = DEFAULT_CONTRACT_ROOT) -> int:
    load_repo_dotenv_if_present()
    registry = load_registry(contract_root)
    filesystem = _build_s3_filesystem()
    bucket = _required_env("RUSTFS_BUCKET")
    checked = 0
    skipped = 0
    for dataset in registry.datasets:
        object_key = _object_key_for_dataset(dataset)
        object_path = f"{bucket}/{object_key}"
        try:
            parquet_file = pq.ParquetFile(object_path, filesystem=filesystem)
        except FileNotFoundError:
            skipped += 1
            print(f"Skipping missing Parquet object {object_key}")
            continue

        actual = [(field.name, str(field.type)) for field in parquet_file.schema_arrow]
        expected = [(field.name, field.type) for field in dataset.parquet.fields]
        if actual != expected:
            msg = (
                f"Parquet schema mismatch for {dataset.dataset}: "
                f"expected {len(expected)} fields, got {len(actual)} fields"
            )
            raise RuntimeError(msg)
        checked += 1

    print(
        f"Parquet schema validation checked {checked} objects, skipped {skipped} missing objects."
    )
    return checked


def _object_key_for_dataset(dataset: DatasetContract) -> str:
    path_parts = list(dataset.source_asset_key)
    if dataset.parquet.storage_mode == "partitioned":
        path_parts.append("year=2026")
    path_parts.append("000000_0.parquet")
    return "/".join(path_parts)


def _build_s3_filesystem():
    import pyarrow.fs as pafs

    endpoint = local_rustfs_endpoint()
    scheme = None
    if "://" in endpoint:
        from urllib.parse import urlparse

        parsed = urlparse(endpoint)
        scheme = parsed.scheme
        endpoint = parsed.netloc
    s3_filesystem_factory = cast(Any, pafs).S3FileSystem
    return s3_filesystem_factory(
        access_key=_required_env("RUSTFS_ACCESS_KEY"),
        secret_key=_required_env("RUSTFS_SECRET_KEY"),
        endpoint_override=endpoint,
        scheme=scheme,
        region=os.environ.get("RUSTFS_REGION", "us-east-1"),
        allow_bucket_creation=True,
    )


def _required_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        msg = f"{name} is required for Parquet schema validation"
        raise RuntimeError(msg)
    return value
