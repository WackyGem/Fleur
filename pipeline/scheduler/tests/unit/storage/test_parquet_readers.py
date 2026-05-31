from __future__ import annotations

from datetime import date
from pathlib import Path

import dagster as dg
import pyarrow as pa
import pyarrow.parquet as pq
import pytest
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.trade_calendar import trade_date_partition_keys_for_year
from scheduler.defs.storage import parquet_readers
from scheduler.defs.storage.s3 import asset_key_to_parquet_object_key
from tests.fakes.storage import local_filesystem


def test_trade_date_partition_keys_for_year_filters_and_sorts_dates() -> None:
    partition_keys = trade_date_partition_keys_for_year(
        2026,
        trade_dates={
            date(2026, 1, 5),
            date(2025, 12, 31),
            date(2026, 1, 2),
            date(2026, 1, 6),
        },
        refresh_until_trade_date=date(2026, 1, 5),
    )

    assert partition_keys == ["2026-01-02", "2026-01-05"]


def test_read_partitioned_parquet_tables_from_s3_reads_existing_non_empty_partitions(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    config = _local_s3_config(tmp_path)
    asset_key = dg.AssetKey(["source", "source__daily"])
    _write_partition(tmp_path, asset_key, "trade_date", "2026-01-02", pa.table({"value": [1]}))
    _write_partition(tmp_path, asset_key, "trade_date", "2026-01-05", pa.table({"value": [2, 3]}))
    monkeypatch.setattr(parquet_readers, "build_s3_filesystem", lambda config: local_filesystem())

    result = parquet_readers.read_partitioned_parquet_tables_from_s3(
        config,
        asset_key,
        partition_keys=["2026-01-02", "2026-01-03", "2026-01-05"],
        partition_key_name="trade_date",
    )

    assert [table.num_rows for table in result.tables] == [1, 2]
    assert result.read_partition_keys == ["2026-01-02", "2026-01-05"]
    assert result.missing_partition_keys == ["2026-01-03"]
    assert result.empty_partition_keys == []


def test_read_partitioned_parquet_tables_from_s3_skips_empty_tables(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    config = _local_s3_config(tmp_path)
    asset_key = dg.AssetKey("source__daily")
    schema = pa.schema([("value", pa.int64())])
    _write_partition(
        tmp_path,
        asset_key,
        "trade_date",
        "2026-01-02",
        pa.table({"value": []}, schema=schema),
    )
    monkeypatch.setattr(parquet_readers, "build_s3_filesystem", lambda config: local_filesystem())

    result = parquet_readers.read_partitioned_parquet_tables_from_s3(
        config,
        asset_key,
        partition_keys=["2026-01-02"],
        partition_key_name="trade_date",
    )

    assert result.tables == []
    assert result.read_partition_keys == []
    assert result.missing_partition_keys == []
    assert result.empty_partition_keys == ["2026-01-02"]


def test_read_partitioned_parquet_tables_from_s3_wraps_unexpected_errors(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    class BrokenFilesystem:
        def open_input_file(self, path: str) -> object:
            msg = "permission denied"
            raise PermissionError(msg)

    config = _local_s3_config(tmp_path)
    monkeypatch.setattr(parquet_readers, "build_s3_filesystem", lambda config: BrokenFilesystem())

    with pytest.raises(RuntimeError, match="Failed to read parquet table"):
        parquet_readers.read_partitioned_parquet_tables_from_s3(
            config,
            dg.AssetKey("source__daily"),
            partition_keys=["2026-01-02"],
            partition_key_name="trade_date",
        )


def _local_s3_config(tmp_path: Path) -> S3Config:
    bucket = tmp_path / "bucket"
    bucket.mkdir()
    return S3Config(
        endpoint="local",
        bucket=str(bucket),
        access_key="access",
        secret_key="secret",
        region_name="region",
    )


def _write_partition(
    tmp_path: Path,
    asset_key: dg.AssetKey,
    partition_key_name: str,
    partition_key: str,
    table: pa.Table,
) -> None:
    object_key = asset_key_to_parquet_object_key(
        asset_key,
        partition_key=partition_key,
        partition_key_name=partition_key_name,
        storage_mode="partitioned",
    )
    path = tmp_path / "bucket" / object_key
    path.parent.mkdir(parents=True)
    pq.write_table(table, path)
