from __future__ import annotations

from datetime import date, datetime
from zoneinfo import ZoneInfo

import dagster as dg
import pyarrow as pa
import pytest
from scheduler.defs.resources.s3 import S3SettingsResource
from scheduler.defs.sources import daily_compact
from scheduler.defs.sources.jiuyan.action_field_compact import (
    jiuyan__action_field_compacted,
    jiuyan_action_field_compacted_year_partitions,
)
from scheduler.defs.storage.parquet_readers import PartitionedParquetReadResult


def test_action_field_compacted_asset_contract() -> None:
    assert (
        jiuyan__action_field_compacted.key.to_user_string()
        == "source/jiuyan__action_field_compacted"
    )
    assert (
        jiuyan__action_field_compacted.group_names_by_key[jiuyan__action_field_compacted.key]
        == "s3_sources"
    )
    assert jiuyan__action_field_compacted.partitions_def is not None
    metadata = jiuyan__action_field_compacted.metadata_by_key[jiuyan__action_field_compacted.key]
    assert metadata["storage_mode"] == "partitioned"
    assert metadata["partition_key_name"] == "year"
    assert metadata["input_asset"] == "source/jiuyan__action_field"


def test_action_field_compacted_year_partitions_include_current_year() -> None:
    partition_keys = jiuyan_action_field_compacted_year_partitions.get_partition_keys(
        current_time=datetime(2026, 5, 30, tzinfo=ZoneInfo("Asia/Shanghai"))
    )

    assert "2026" in partition_keys


def test_compact_daily_asset_by_year_merges_non_empty_daily_tables(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured_partition_keys: list[str] = []

    def fake_read_tables(
        config: object,
        asset_key: dg.AssetKey,
        *,
        partition_keys: list[str],
        partition_key_name: str,
    ) -> PartitionedParquetReadResult:
        captured_partition_keys.extend(partition_keys)
        assert asset_key == dg.AssetKey(["source", "jiuyan__action_field"])
        assert partition_key_name == "trade_date"
        return PartitionedParquetReadResult(
            tables=[pa.table({"value": [1]}), pa.table({"value": [2, 3]})],
            read_partition_keys=["2026-01-02", "2026-01-05"],
            missing_partition_keys=["2026-01-06"],
            empty_partition_keys=["2026-01-07"],
        )

    monkeypatch.setattr(
        daily_compact,
        "read_trade_dates_from_s3",
        lambda config: {
            date(2026, 1, 2),
            date(2026, 1, 5),
            date(2026, 1, 6),
            date(2026, 1, 7),
        },
    )
    monkeypatch.setattr(
        daily_compact,
        "read_partitioned_parquet_tables_from_s3",
        fake_read_tables,
    )

    context = dg.build_asset_context(
        partition_key="2026",
        run_tags={"market.trade_date": "2026-01-07"},
    )
    result = daily_compact.compact_daily_asset_by_year(
        context,
        raw_asset_key=dg.AssetKey(["source", "jiuyan__action_field"]),
        s3_settings=S3SettingsResource(
            endpoint="http://localhost:9000",
            bucket="bucket",
            access_key="access",
            secret_key="secret",
        ),
    )

    assert captured_partition_keys == ["2026-01-02", "2026-01-05", "2026-01-06", "2026-01-07"]
    assert result.value is not None
    table = result.value["2026"]
    assert result.metadata is not None
    assert table.column("value").to_pylist() == [1, 2, 3]
    assert result.metadata["row_count"] == 3
    assert result.metadata["column_count"] == 1
    assert result.metadata["input_asset"] == "source/jiuyan__action_field"
    assert result.metadata["requested_partition_count"] == 4
    assert result.metadata["read_partition_count"] == 2
    assert result.metadata["missing_partition_count"] == 1
    assert result.metadata["empty_partition_count"] == 1
    assert result.metadata["refresh_until_trade_date"] == "2026-01-07"


def test_compact_daily_asset_by_year_rejects_empty_input(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(
        daily_compact,
        "read_trade_dates_from_s3",
        lambda config: {date(2026, 1, 2)},
    )
    monkeypatch.setattr(
        daily_compact,
        "read_partitioned_parquet_tables_from_s3",
        lambda *args, **kwargs: PartitionedParquetReadResult(
            tables=[],
            read_partition_keys=[],
            missing_partition_keys=["2026-01-02"],
            empty_partition_keys=[],
        ),
    )

    context = dg.build_asset_context(
        partition_key="2026",
        run_tags={"market.trade_date": "2026-01-02"},
    )

    with pytest.raises(RuntimeError, match="No non-empty daily partitions"):
        daily_compact.compact_daily_asset_by_year(
            context,
            raw_asset_key=dg.AssetKey(["source", "jiuyan__action_field"]),
            s3_settings=S3SettingsResource(
                endpoint="http://localhost:9000",
                bucket="bucket",
                access_key="access",
                secret_key="secret",
            ),
        )


def test_refresh_until_for_year_rejects_wrong_year_tag() -> None:
    with pytest.raises(ValueError, match="is not in year partition"):
        daily_compact.refresh_until_for_year(2026, "2025-12-31")
