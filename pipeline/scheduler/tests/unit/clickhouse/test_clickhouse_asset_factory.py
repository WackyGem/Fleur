from __future__ import annotations

from scheduler.defs.clickhouse.assets import CLICKHOUSE_RAW_ASSETS
from scheduler.defs.clickhouse.specs import BAOSTOCK_DAILY_K_SPEC, CLICKHOUSE_RAW_GROUP


def test_clickhouse_raw_asset_factory_sets_key_group_deps_and_tags() -> None:
    asset = CLICKHOUSE_RAW_ASSETS[0]

    assert asset.key == BAOSTOCK_DAILY_K_SPEC.raw_asset_key
    assert asset.group_names_by_key[asset.key] == CLICKHOUSE_RAW_GROUP
    assert asset.dependency_keys == {BAOSTOCK_DAILY_K_SPEC.source_asset_key}
    assert {key: asset.tags_by_key[asset.key][key] for key in ("source", "layer", "storage")} == {
        "source": "baostock",
        "layer": "raw",
        "storage": "clickhouse",
    }


def test_clickhouse_raw_asset_uses_same_year_partitions_as_source() -> None:
    asset = CLICKHOUSE_RAW_ASSETS[0]

    assert asset.partitions_def is not None
    assert asset.partitions_def.get_partition_keys()[:1] == ["1990"]
