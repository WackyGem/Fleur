import time
from datetime import date
from typing import Any

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    generated_endpoint_metadata,
    s3_parquet_kinds,
    source_owners,
    source_tags,
    year_partition_metadata,
)
from scheduler.defs.baostock.assets import baostock__query_stock_basic, year_partitions
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.resources.s3 import S3SettingsResource
from scheduler.defs.sources.eastmoney.schema import (
    ENDPOINT_CONFIGS,
    EastmoneyEndpointConfig,
)
from scheduler.defs.sources.eastmoney.services import (
    EastmoneyRefreshRequest,
    EastmoneyYearRefreshService,
    fetch_eastmoney_tables,
)
from scheduler.defs.sources.eastmoney.services import (
    baostock_code_to_eastmoney_code as _baostock_code_to_eastmoney_code,
)
from scheduler.defs.sources.eastmoney.services import (
    build_year_ranges as service_build_year_ranges,
)

baostock_code_to_eastmoney_code = _baostock_code_to_eastmoney_code

__all__ = [
    "EASTMONEY_ASSETS",
    "EASTMONEY_ASSETS_BY_NAME",
    "EastmoneyYearConfig",
    "baostock_code_to_eastmoney_code",
    "build_eastmoney_asset",
    "build_eastmoney_assets",
]

EASTMONEY_RUN_POOL = "eastmoney_run_pool"
EASTMONEY_ASSET_METADATA = year_partition_metadata(allow_empty=True)
EASTMONEY_ORDERING_REASON = "external_api_rate_limit"


class EastmoneyYearConfig(dg.Config):
    refresh_until_date: str | None = None


def build_eastmoney_asset(
    endpoint: EastmoneyEndpointConfig,
    ordering_dependency: dg.AssetsDefinition | None = None,
) -> dg.AssetsDefinition:
    deps = [baostock__query_stock_basic]
    metadata: dict[str, RawMetadataValue] = dict(EASTMONEY_ASSET_METADATA)
    if ordering_dependency is not None:
        deps.append(ordering_dependency)
        metadata.update(
            generated_endpoint_metadata(
                ordering_dependency=ordering_dependency.key.to_user_string(),
                ordering_reason=EASTMONEY_ORDERING_REASON,
            )
        )

    def materialize(
        context: dg.AssetExecutionContext,
        config: EastmoneyYearConfig,
        s3_settings: S3SettingsResource,
    ) -> dg.MaterializeResult[dict[str, pa.Table]]:
        return _materialize_eastmoney_asset(context, config, s3_settings, endpoint)

    materialize.__name__ = endpoint.asset_name
    materialize.__doc__ = f"EastMoney F10 rows for {endpoint.asset_name} by natural-year partition."

    return dg.asset(
        name=endpoint.asset_name,
        key_prefix=[SOURCE_ASSET_KEY_PREFIX],
        group_name="s3_sources",
        io_manager_key="s3_io_manager",
        partitions_def=year_partitions,
        deps=deps,
        backfill_policy=dg.BackfillPolicy.multi_run(max_partitions_per_run=1),
        metadata=metadata,
        owners=source_owners(),
        kinds=s3_parquet_kinds("http"),
        pool=EASTMONEY_RUN_POOL,
        tags=source_tags("eastmoney"),
    )(materialize)


def build_eastmoney_assets() -> list[dg.AssetsDefinition]:
    assets: list[dg.AssetsDefinition] = []
    previous_asset: dg.AssetsDefinition | None = None
    for endpoint in ENDPOINT_CONFIGS:
        asset = build_eastmoney_asset(endpoint, previous_asset)
        assets.append(asset)
        previous_asset = asset
    return assets


EASTMONEY_ASSETS = build_eastmoney_assets()
EASTMONEY_ASSETS_BY_NAME = {asset.node_def.name: asset for asset in EASTMONEY_ASSETS}


def _materialize_eastmoney_asset(
    context: dg.AssetExecutionContext,
    config: EastmoneyYearConfig,
    s3_settings: S3SettingsResource,
    endpoint: EastmoneyEndpointConfig,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    asset_started_at = time.perf_counter()
    s3_config = s3_settings.config()
    config_loaded_at = time.perf_counter()
    service = EastmoneyYearRefreshService(s3_config)
    result = service.refresh(
        EastmoneyRefreshRequest(
            endpoint=endpoint,
            partition_keys=list(context.partition_keys),
            refresh_until_date=config.refresh_until_date,
        )
    )
    result.metadata["s3_config_load_seconds"] = elapsed_seconds(asset_started_at, config_loaded_at)
    return dg.MaterializeResult(value=result.tables, metadata=result.metadata)


async def _fetch_eastmoney_tables(
    endpoint: EastmoneyEndpointConfig,
    stock_basic: pa.Table,
    year_ranges: dict[str, tuple[date, date]],
) -> tuple[dict[str, pa.Table], dict[str, Any]]:
    return await fetch_eastmoney_tables(endpoint, stock_basic, year_ranges)


def _build_year_ranges(
    context: dg.AssetExecutionContext,
    config: EastmoneyYearConfig,
) -> dict[str, tuple[date, date]]:
    return service_build_year_ranges(
        list(context.partition_keys),
        refresh_until_date=config.refresh_until_date,
    )
