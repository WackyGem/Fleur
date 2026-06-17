from contextlib import AbstractAsyncContextManager
from dataclasses import dataclass
from typing import cast

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    s3_parquet_kinds,
    source_owners,
    source_tags,
    year_partition_metadata,
)
from scheduler.defs.http.client_factory import HttpClientFactory
from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.resources.http import HttpClientFactoryResource
from scheduler.defs.sources.chinabond.client import ChinabondAioHttpClient
from scheduler.defs.sources.chinabond.services import (
    ChinabondClientProtocol,
    ChinabondGovernmentBondRefreshService,
    ChinabondRefreshRequest,
)

CHINABOND_RUN_POOL = "chinabond_run_pool"

chinabond_year_partitions = dg.TimeWindowPartitionsDefinition(
    start="2006",
    fmt="%Y",
    cron_schedule="0 0 1 1 *",
    timezone="Asia/Shanghai",
    end_offset=1,
)


class ChinabondYearConfig(dg.Config):
    refresh_until_date: str | None = None


@dataclass(frozen=True)
class DefaultChinabondClientFactory:
    http_client_factory: HttpClientFactory

    def client(self) -> AbstractAsyncContextManager[ChinabondClientProtocol]:
        return cast(
            AbstractAsyncContextManager[ChinabondClientProtocol],
            ChinabondAioHttpClient(self.http_client_factory.json_client()),
        )


@dg.asset(
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    io_manager_key="s3_io_manager",
    partitions_def=chinabond_year_partitions,
    backfill_policy=dg.BackfillPolicy.multi_run(max_partitions_per_run=1),
    metadata=year_partition_metadata(),
    owners=source_owners(),
    kinds=s3_parquet_kinds("http"),
    pool=CHINABOND_RUN_POOL,
    tags=source_tags("chinabond"),
)
def chinabond__government_bond(
    context: dg.AssetExecutionContext,
    config: ChinabondYearConfig,
    http_client_factory: HttpClientFactoryResource,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    """ChinaBond government bond yield curve by natural-year partition."""

    result = ChinabondGovernmentBondRefreshService(
        client_factory=DefaultChinabondClientFactory(http_client_factory.factory())
    ).refresh(
        ChinabondRefreshRequest(
            partition_keys=list(context.partition_keys),
            refresh_until_date=config.refresh_until_date,
        )
    )
    return dg.MaterializeResult(value=result.tables, metadata=result.metadata)
