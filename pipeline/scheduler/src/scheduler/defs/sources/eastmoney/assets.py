import time
from datetime import date
from typing import Any

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    METADATA_EXECUTION_ORDERING_DEPENDENCY,
    source_tags,
    year_partition_metadata,
)
from scheduler.defs.baostock.assets import baostock__query_stock_basic, year_partitions
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
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
]

EASTMONEY_RUN_POOL = "eastmoney_run_pool"
EASTMONEY_ASSET_METADATA = year_partition_metadata(allow_empty=True)


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
        metadata[METADATA_EXECUTION_ORDERING_DEPENDENCY] = ordering_dependency.key.to_user_string()

    def materialize(
        context: dg.AssetExecutionContext,
        config: EastmoneyYearConfig,
    ) -> dg.MaterializeResult[dict[str, pa.Table]]:
        return _materialize_eastmoney_asset(context, config, endpoint)

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
        pool=EASTMONEY_RUN_POOL,
        tags=source_tags("eastmoney"),
    )(materialize)


EASTMONEY_ASSETS_BY_NAME: dict[str, dg.AssetsDefinition] = {}
_previous_eastmoney_asset: dg.AssetsDefinition | None = None
for _endpoint in ENDPOINT_CONFIGS:
    _eastmoney_asset = build_eastmoney_asset(_endpoint, _previous_eastmoney_asset)
    EASTMONEY_ASSETS_BY_NAME[_endpoint.asset_name] = _eastmoney_asset
    globals()[_endpoint.asset_name] = _eastmoney_asset
    _previous_eastmoney_asset = _eastmoney_asset

EASTMONEY_ASSETS = list(EASTMONEY_ASSETS_BY_NAME.values())


def _materialize_eastmoney_asset(
    context: dg.AssetExecutionContext,
    config: EastmoneyYearConfig,
    endpoint: EastmoneyEndpointConfig,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    asset_started_at = time.perf_counter()
    s3_config = S3Config.from_env()
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
