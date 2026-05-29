from __future__ import annotations

import asyncio
import hashlib
from typing import Any, cast

import dagster as dg
import pyarrow as pa
import pytest
from scheduler.defs.io_managers import s3_io_manager
from scheduler.defs.io_managers.s3_io_manager import S3IOManager
from scheduler.defs.ocr.service import run_bounded_ocr_batch
from scheduler.defs.storage import object_store, s3
from scheduler.defs.storage.object_store import (
    DownloadedImage,
    ObjectStore,
    download_image_bytes,
    normalize_image_content_type,
)
from tests.fakes.http import FakeBytesClient
from tests.fakes.storage import InMemoryFilesystem


def test_s3_path_builder_supports_partitioned_and_latest_snapshot_modes() -> None:
    asset_key = dg.AssetKey(["source", "asset"])

    assert (
        s3.asset_key_to_parquet_object_key(
            asset_key,
            object_prefix="/raw/",
            partition_key="2026",
            partition_key_name="year",
        )
        == "raw/source/asset/year=2026/000000_0.parquet"
    )
    assert (
        s3.asset_key_to_parquet_object_key(
            asset_key,
            object_prefix="",
            storage_mode="latest_snapshot",
        )
        == "source/asset/000000_0.parquet"
    )
    with pytest.raises(ValueError, match="partition_key_name is required"):
        s3.asset_key_to_parquet_object_key(asset_key, partition_key="2026")
    with pytest.raises(ValueError, match="Unsupported storage mode"):
        s3.asset_key_to_parquet_object_key(asset_key, storage_mode="invalid")  # type: ignore[arg-type]


def test_object_store_writes_and_reads_bytes_through_bucket_paths() -> None:
    filesystem = InMemoryFilesystem()
    store = ObjectStore(filesystem=filesystem, bucket="bucket")

    assert store.write_bytes("images/a.png", b"image-bytes") == "images/a.png"
    assert filesystem.data["bucket/images/a.png"] == b"image-bytes"
    assert store.read_bytes("images/a.png") == b"image-bytes"


def test_object_store_returns_relative_key_for_single_parquet_file(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_write_parquet_dataset(
        table: pa.Table,
        base_dir: str,
        filesystem: object,
        *,
        allow_empty: bool = False,
    ) -> list[str]:
        assert table.num_rows == 1
        assert base_dir == "bucket/ocr/result"
        assert allow_empty is True
        return ["bucket/ocr/result/000000_0.parquet"]

    monkeypatch.setattr(object_store, "write_parquet_dataset", fake_write_parquet_dataset)

    store = ObjectStore(filesystem=object(), bucket="bucket")

    assert store.write_table("ocr/result/", pa.table({"value": ["ok"]})) == (
        "ocr/result/000000_0.parquet"
    )


def test_object_store_rejects_multi_file_table_writes(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        object_store,
        "write_parquet_dataset",
        lambda *args, **kwargs: ["bucket/a.parquet", "bucket/b.parquet"],
    )
    store = ObjectStore(filesystem=object(), bucket="bucket")

    with pytest.raises(RuntimeError, match="Expected a single parquet file"):
        store.write_table("base", pa.table({"value": ["ok"]}))


def test_download_image_bytes_normalizes_content_type_and_hashes_body() -> None:
    client = FakeBytesClient(
        body=b"image-bytes",
        headers={"Content-Type": "image/png; charset=utf-8"},
    )

    downloaded = asyncio.run(download_image_bytes(client, "https://example.test/image.png"))

    assert isinstance(downloaded, DownloadedImage)
    assert downloaded.mime_type == "image/png"
    assert downloaded.byte_count == len(b"image-bytes")
    assert downloaded.sha256 == hashlib.sha256(b"image-bytes").hexdigest()
    assert client.request_url == "https://example.test/image.png"
    assert client.request_headers is not None
    assert client.request_headers["Accept"].startswith("image/")


def test_normalize_image_content_type_rejects_missing_or_non_image_headers() -> None:
    with pytest.raises(ValueError, match="missing Content-Type"):
        normalize_image_content_type(None)
    with pytest.raises(ValueError, match="not an image"):
        normalize_image_content_type("application/json")


def test_s3_io_manager_writes_latest_snapshot_and_records_metadata(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    written: list[dict[str, Any]] = []

    def fake_build_s3_filesystem(config: object) -> object:
        return {"config": config}

    def fake_write_parquet_dataset(
        table: pa.Table,
        base_dir: str,
        filesystem: object,
        *,
        allow_empty: bool = False,
        **kwargs: object,
    ) -> list[str]:
        written.append(
            {
                "table": table,
                "base_dir": base_dir,
                "filesystem": filesystem,
                "allow_empty": allow_empty,
                **kwargs,
            }
        )
        return [f"{base_dir}/000000_0.parquet"]

    monkeypatch.setattr(s3_io_manager, "build_s3_filesystem", fake_build_s3_filesystem)
    monkeypatch.setattr(s3_io_manager, "write_parquet_dataset", fake_write_parquet_dataset)
    manager = S3IOManager(
        endpoint="http://rustfs.test",
        bucket="bucket",
        access_key="access",
        secret_key="secret",
        region_name="region",
    )
    context = dg.build_output_context(
        asset_key=dg.AssetKey(["source", "asset"]),
        definition_metadata={"storage_mode": "latest_snapshot"},
    )

    manager.handle_output(context, pa.table({"value": ["one", "two"]}))

    assert written[0]["base_dir"] == "bucket/raw/source/asset"
    metadata = context.consume_logged_metadata()
    assert metadata["row_count"].value == 2
    assert metadata["column_count"].value == 1
    assert metadata["storage_mode"].text == "latest_snapshot"
    assert metadata["s3_key"].text == "raw/source/asset/000000_0.parquet"


def test_s3_io_manager_writes_partitioned_tables_and_records_partition_metadata(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    written: list[dict[str, Any]] = []

    monkeypatch.setattr(s3_io_manager, "build_s3_filesystem", lambda config: object())

    def fake_write_parquet_dataset(
        table: pa.Table,
        base_dir: str,
        filesystem: object,
        *,
        partition_key: str | None = None,
        partition_key_name: str | None = None,
        allow_empty: bool = False,
    ) -> list[str]:
        written.append(
            {
                "table": table,
                "partition_key": partition_key,
                "partition_key_name": partition_key_name,
                "allow_empty": allow_empty,
            }
        )
        return [f"{base_dir}/{partition_key_name}={partition_key}/000000_0.parquet"]

    monkeypatch.setattr(s3_io_manager, "write_parquet_dataset", fake_write_parquet_dataset)
    manager = S3IOManager(
        endpoint="http://rustfs.test",
        bucket="bucket",
        access_key="access",
        secret_key="secret",
        region_name="region",
    )
    context = dg.build_output_context(
        asset_key=dg.AssetKey(["source", "partitioned"]),
        asset_partitions_def=dg.StaticPartitionsDefinition(["2025", "2026"]),
        asset_partition_key_range=dg.PartitionKeyRange("2025", "2026"),
        definition_metadata={
            "storage_mode": "partitioned",
            "partition_key_name": "year",
            "allow_empty": True,
        },
    )

    manager.handle_output(
        context,
        {
            "2025": pa.table({"value": []}, schema=pa.schema([("value", pa.string())])),
            "2026": pa.table({"value": ["ok"]}),
        },
    )

    assert [call["partition_key"] for call in written] == ["2025", "2026"]
    metadata = context.consume_logged_metadata()
    assert metadata["row_count"].value == 1
    assert metadata["column_count"].value == 1
    assert metadata["partition_key_name"].text == "year"
    assert cast(Any, metadata["partition_row_counts"]).data == {"2025": 0, "2026": 1}
    assert cast(Any, metadata["empty_partition_keys"]).data == ["2025"]


def test_s3_io_manager_validates_output_shape() -> None:
    manager = S3IOManager(
        endpoint="http://rustfs.test",
        bucket="bucket",
        access_key="access",
        secret_key="secret",
        region_name="region",
    )

    with pytest.raises(TypeError, match="expected a pyarrow.Table"):
        manager.validate_table({"not": "a table"})
    with pytest.raises(ValueError, match="empty pyarrow.Table"):
        manager.validate_table(pa.table({"value": []}))
    with pytest.raises(TypeError, match="expected a mapping"):
        manager.validate_partition_tables(pa.table({"value": ["ok"]}))
    with pytest.raises(TypeError, match="keys must be strings"):
        manager.validate_partition_tables({1: pa.table({"value": ["ok"]})})
    with pytest.raises(ValueError, match="empty partition table mapping"):
        manager.validate_partition_tables({})
    with pytest.raises(ValueError, match="columns differ"):
        manager.partition_column_count(
            {
                "a": pa.table({"value": ["ok"]}),
                "b": pa.table({"other": ["ok"]}),
            }
        )


def test_s3_io_manager_rejects_partition_key_mismatches(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(s3_io_manager, "build_s3_filesystem", lambda config: object())
    manager = S3IOManager(
        endpoint="http://rustfs.test",
        bucket="bucket",
        access_key="access",
        secret_key="secret",
        region_name="region",
    )
    context = dg.build_output_context(
        asset_key=dg.AssetKey(["source", "partitioned"]),
        asset_partitions_def=dg.StaticPartitionsDefinition(["2026"]),
        partition_key="2026",
        definition_metadata={"storage_mode": "partitioned", "partition_key_name": "year"},
    )

    with pytest.raises(RuntimeError, match="keys must match"):
        manager.handle_output(context, {"2025": pa.table({"value": ["ok"]})})


def test_run_bounded_ocr_batch_limits_concurrency_and_collects_successes() -> None:
    active_count = 0
    max_seen = 0

    async def process_item(item: int) -> None:
        nonlocal active_count, max_seen
        active_count += 1
        max_seen = max(max_seen, active_count)
        await asyncio.sleep(0)
        active_count -= 1

    result = asyncio.run(
        run_bounded_ocr_batch(
            [1, 2, 3, 4],
            max_concurrent_requests=2,
            process_item=process_item,
        )
    )

    assert result.successes == [1, 2, 3, 4]
    assert result.failure_count == 0
    assert max_seen == 2


def test_run_bounded_ocr_batch_counts_failures_before_reraising() -> None:
    async def process_item(item: int) -> None:
        if item == 2:
            msg = "OCR failed"
            raise RuntimeError(msg)

    with pytest.raises(RuntimeError, match="OCR failed"):
        asyncio.run(
            run_bounded_ocr_batch([1, 2], max_concurrent_requests=1, process_item=process_item)
        )


def test_run_bounded_ocr_batch_requires_positive_concurrency() -> None:
    async def process_item(item: int) -> None:
        return None

    with pytest.raises(ValueError, match="must be positive"):
        asyncio.run(
            run_bounded_ocr_batch([1], max_concurrent_requests=0, process_item=process_item)
        )
