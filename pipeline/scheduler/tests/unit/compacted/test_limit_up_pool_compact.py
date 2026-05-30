from __future__ import annotations

from datetime import datetime
from zoneinfo import ZoneInfo

from scheduler.defs.sources.ths.limit_up_pool_compact import (
    ths__limit_up_pool_compacted,
    ths_limit_up_pool_compacted_year_partitions,
)


def test_limit_up_pool_compacted_asset_contract() -> None:
    assert (
        ths__limit_up_pool_compacted.key.to_user_string() == "source/ths__limit_up_pool_compacted"
    )
    assert ths__limit_up_pool_compacted.partitions_def is not None
    assert (
        ths__limit_up_pool_compacted.group_names_by_key[ths__limit_up_pool_compacted.key]
        == "s3_sources"
    )
    metadata = ths__limit_up_pool_compacted.metadata_by_key[ths__limit_up_pool_compacted.key]
    assert metadata["storage_mode"] == "partitioned"
    assert metadata["partition_key_name"] == "year"
    assert metadata["input_asset"] == "source/ths__limit_up_pool"


def test_limit_up_pool_compacted_year_partitions_include_current_year() -> None:
    partition_keys = ths_limit_up_pool_compacted_year_partitions.get_partition_keys(
        current_time=datetime(2026, 5, 30, tzinfo=ZoneInfo("Asia/Shanghai"))
    )

    assert "2026" in partition_keys
