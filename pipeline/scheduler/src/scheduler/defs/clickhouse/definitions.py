from __future__ import annotations

import dagster as dg

from scheduler.defs.clickhouse.assets import CLICKHOUSE_RAW_ASSETS
from scheduler.defs.clickhouse.specs import (
    ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
    ClickHouseRawTableSpec,
)


def _asset_for_spec(raw_asset_key: dg.AssetKey) -> dg.AssetsDefinition:
    for asset in CLICKHOUSE_RAW_ASSETS:
        if asset.key == raw_asset_key:
            return asset

    msg = f"ClickHouse raw asset is not registered: {raw_asset_key.to_user_string()}"
    raise RuntimeError(msg)


def _assets_matching_specs(
    specs: tuple[ClickHouseRawTableSpec, ...],
    *,
    source_prefixes: tuple[str, ...],
) -> list[dg.AssetsDefinition]:
    return [
        _asset_for_spec(spec.raw_asset_key)
        for spec in specs
        if spec.source_asset_key.path[-1].startswith(source_prefixes)
    ]


def _assets_matching_strategy(
    specs: tuple[ClickHouseRawTableSpec, ...],
    *,
    partition_strategy: str,
) -> list[dg.AssetsDefinition]:
    return [
        _asset_for_spec(spec.raw_asset_key)
        for spec in specs
        if spec.partition_strategy == partition_strategy
    ]


CLICKHOUSE_RAW_JOBS: tuple[dg.UnresolvedAssetJobDefinition, ...] = (
    dg.define_asset_job(
        name="clickhouse__raw_sync_snapshot_job",
        selection=_assets_matching_strategy(
            ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
            partition_strategy="snapshot",
        ),
    ),
    dg.define_asset_job(
        name="clickhouse__raw_sync_baostock_job",
        selection=_assets_matching_specs(
            ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
            source_prefixes=("baostock__query_history_k_data_plus_daily",),
        ),
    ),
    dg.define_asset_job(
        name="clickhouse__raw_sync_eastmoney_job",
        selection=_assets_matching_specs(
            ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
            source_prefixes=("eastmoney__",),
        ),
    ),
    dg.define_asset_job(
        name="clickhouse__raw_sync_jiuyan_market_event_job",
        selection=_assets_matching_specs(
            ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
            source_prefixes=("jiuyan__action_field_compacted",),
        ),
    ),
    dg.define_asset_job(
        name="clickhouse__raw_sync_ths_market_event_job",
        selection=_assets_matching_specs(
            ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
            source_prefixes=("ths__limit_up_pool_compacted",),
        ),
    ),
)

assert ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
