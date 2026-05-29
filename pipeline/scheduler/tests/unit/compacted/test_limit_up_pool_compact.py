from __future__ import annotations

from scheduler.defs.sources.ths.limit_up_pool_compact import ths__limit_up_pool_compacted


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
