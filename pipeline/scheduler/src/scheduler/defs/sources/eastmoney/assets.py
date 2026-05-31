import time
from contextlib import AbstractAsyncContextManager
from dataclasses import dataclass
from datetime import date
from typing import Any

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    s3_parquet_kinds,
    source_owners,
    source_tags,
    year_partition_metadata,
)
from scheduler.defs.baostock.assets import baostock__query_stock_basic, year_partitions
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.http.client_factory import HttpClientFactory
from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.market.readers import S3SecurityUniverseReader
from scheduler.defs.resources.http import HttpClientFactoryResource
from scheduler.defs.resources.s3 import S3SettingsResource
from scheduler.defs.sources.eastmoney.client import EastmoneyAioHttpClient
from scheduler.defs.sources.eastmoney.schema import (
    ENDPOINT_CONFIGS,
    EastmoneyEndpointConfig,
)
from scheduler.defs.sources.eastmoney.services import (
    EastmoneyClientProtocol,
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


class EastmoneyYearConfig(dg.Config):
    refresh_until_date: str | None = None


@dataclass(frozen=True)
class DefaultEastmoneyClientFactory:
    http_client_factory: HttpClientFactory

    def client(self) -> AbstractAsyncContextManager[EastmoneyClientProtocol]:
        return EastmoneyAioHttpClient(http_client_factory=self.http_client_factory)


def build_eastmoney_asset(endpoint: EastmoneyEndpointConfig) -> dg.AssetsDefinition:
    deps = [baostock__query_stock_basic]
    metadata: dict[str, RawMetadataValue] = dict(EASTMONEY_ASSET_METADATA)

    def materialize(
        context: dg.AssetExecutionContext,
        config: EastmoneyYearConfig,
        s3_settings: S3SettingsResource,
        http_client_factory: HttpClientFactoryResource,
    ) -> dg.MaterializeResult[dict[str, pa.Table]]:
        return _materialize_eastmoney_asset(
            context,
            config,
            s3_settings,
            http_client_factory,
            endpoint,
        )

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
    for endpoint in ENDPOINT_CONFIGS:
        asset = build_eastmoney_asset(endpoint)
        assets.append(asset)
    return assets


EASTMONEY_ASSETS = build_eastmoney_assets()
EASTMONEY_ASSETS_BY_NAME = {asset.node_def.name: asset for asset in EASTMONEY_ASSETS}


def _materialize_eastmoney_asset(
    context: dg.AssetExecutionContext,
    config: EastmoneyYearConfig,
    s3_settings: S3SettingsResource,
    http_client_factory: HttpClientFactoryResource,
    endpoint: EastmoneyEndpointConfig,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    asset_started_at = time.perf_counter()
    s3_config = s3_settings.config()
    config_loaded_at = time.perf_counter()
    service = EastmoneyYearRefreshService(
        security_universe_reader=S3SecurityUniverseReader.from_s3_config(s3_config),
        client_factory=DefaultEastmoneyClientFactory(http_client_factory.factory()),
    )
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
    client_factory: DefaultEastmoneyClientFactory,
) -> tuple[dict[str, pa.Table], dict[str, Any]]:
    return await fetch_eastmoney_tables(endpoint, stock_basic, year_ranges, client_factory)


def _build_year_ranges(
    context: dg.AssetExecutionContext,
    config: EastmoneyYearConfig,
) -> dict[str, tuple[date, date]]:
    return service_build_year_ranges(
        list(context.partition_keys),
        refresh_until_date=config.refresh_until_date,
    )
