from __future__ import annotations

import dagster as dg


class FakeAssetContext:
    def __init__(self, *, partition_keys: list[str], asset_key: dg.AssetKey) -> None:
        self.partition_keys = partition_keys
        self.asset_key = asset_key


class FakePartitionContext:
    def __init__(self, partition_keys: list[str]) -> None:
        self.partition_keys = partition_keys
