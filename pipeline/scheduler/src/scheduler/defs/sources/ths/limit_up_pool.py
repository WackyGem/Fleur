import asyncio
import time
from collections.abc import Mapping
from datetime import date

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    daily_sparse_partition_metadata,
    s3_parquet_kinds,
    source_owners,
    source_tags,
)
from scheduler.defs.common.metadata import RawMetadataValue, http_stats_metadata
from scheduler.defs.common.numbers import positive_int_or_default
from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY
from scheduler.defs.http.client import (
    HttpRequest,
    browser_json_headers,
    with_referer,
)
from scheduler.defs.http.client_factory import HttpClientFactory
from scheduler.defs.http.pagination import DuplicateRowTracker
from scheduler.defs.http.partitioning import (
    THS_BACKFILL_MAX_NATURAL_DAYS,
    TRADE_DATE_PARTITION_KEY_NAME,
    TradeDateRangeMaterializationResult,
    materialize_trade_date_range,
    ths_limit_up_pool_daily_partitions,
)
from scheduler.defs.http.protocols import HttpJsonStatsClientProtocol
from scheduler.defs.http.schemas import (
    FLATTEN_COLUMN_NAMING,
    ths_limit_up_pool_to_table,
)
from scheduler.defs.market.asset_keys import SINA_TRADE_CALENDAR_ASSET_KEY, SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.resources.s3 import S3SettingsResource

THS_LIMIT_UP_POOL_URL = "https://data.10jqka.com.cn/dataapi/limit_up/limit_up_pool"
THS_LIMIT_UP_POOL_REFERER = "https://data.10jqka.com.cn/"
THS_LIMIT_UP_POOL_FIELD = (
    "199112,10,9001,330323,330324,330325,9002,330329,133971,133970,1968584,3475914,9003,9004"
)
THS_LIMIT_UP_POOL_LIMIT = "200"


class ThsLimitUpPoolConfig(dg.Config):
    max_concurrent_trade_dates: int = 10
    request_delay: float = 0.0


@dg.asset(
    name="ths__limit_up_pool",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    partitions_def=ths_limit_up_pool_daily_partitions,
    deps=[SINA_TRADE_CALENDAR_ASSET_KEY],
    backfill_policy=dg.BackfillPolicy.single_run(),
    metadata=daily_sparse_partition_metadata(
        partition_key_name=TRADE_DATE_PARTITION_KEY_NAME,
        trade_date_filter=SINA_TRADE_CALENDAR_ASSET_KEY.to_user_string(),
        flatten_column_naming=FLATTEN_COLUMN_NAMING,
    ),
    owners=source_owners(),
    kinds=s3_parquet_kinds("http"),
    tags=source_tags("ths"),
)
def ths__limit_up_pool(
    context: dg.AssetExecutionContext,
    config: ThsLimitUpPoolConfig,
    s3_settings: S3SettingsResource,
) -> dg.MaterializeResult:
    """TongHuaShun limit-up pool pages by trade-date partition."""

    result = asyncio.run(_materialize_limit_up_pool_range(context, config, s3_settings))
    return dg.MaterializeResult(metadata=result.metadata)


async def _materialize_limit_up_pool_range(
    context: dg.AssetExecutionContext,
    config: ThsLimitUpPoolConfig,
    s3_settings: S3SettingsResource,
) -> TradeDateRangeMaterializationResult:
    async with HttpClientFactory(retry_policy=DEFAULT_RETRY_POLICY).json_client(
        headers=with_referer(browser_json_headers(), THS_LIMIT_UP_POOL_REFERER),
        request_delay=config.request_delay,
    ) as client:
        result = await materialize_trade_date_range(
            context,
            max_concurrent_trade_dates=config.max_concurrent_trade_dates,
            fetch_table_for_trade_date=lambda trade_date: fetch_limit_up_pool_table_with_client(
                client,
                trade_date=trade_date,
            ),
            backfill_window_limit=THS_BACKFILL_MAX_NATURAL_DAYS,
            s3_config=s3_settings.config(),
        )
        result.metadata.update(http_stats_metadata(client.stats))
        return result


async def fetch_limit_up_pool_table_with_client(
    client: HttpJsonStatsClientProtocol,
    *,
    trade_date: date,
) -> tuple[pa.Table, Mapping[str, RawMetadataValue]]:
    started_at = time.perf_counter()
    pages: list[Mapping[str, object]] = []
    duplicate_tracker = DuplicateRowTracker()
    page_number = 1
    total_pages = 1
    source_response_date: str | None = None

    while page_number <= total_pages:
        payload = await client.request_json_object(
            HttpRequest(
                method="GET",
                url=THS_LIMIT_UP_POOL_URL,
                params=limit_up_pool_params(trade_date=trade_date, page_number=page_number),
            )
        )
        status_code = payload.get("status_code")
        status_msg = payload.get("status_msg")
        if status_code == -1 and status_msg == "date参数不合法":
            msg = (
                "THS limit_up_pool rejected the date parameter, likely outside "
                f"the retention window: trade_date={trade_date.isoformat()}"
            )
            raise RuntimeError(msg)
        if status_code != 0:
            msg = f"THS limit_up_pool returned status_code={status_code}: {status_msg}"
            raise RuntimeError(msg)

        data = payload.get("data")
        if not isinstance(data, Mapping):
            msg = "THS limit_up_pool response data is not an object"
            raise RuntimeError(msg)
        page = data.get("page")
        if not isinstance(page, Mapping):
            msg = "THS limit_up_pool response data.page is not an object"
            raise RuntimeError(msg)
        if page_number == 1:
            total_pages = positive_int_or_default(page.get("count"), default=1)
        info = data.get("info")
        if not isinstance(info, list):
            msg = "THS limit_up_pool response data.info is not an array"
            raise RuntimeError(msg)

        for row in info:
            if not isinstance(row, Mapping):
                msg = "THS limit_up_pool data.info item is not an object"
                raise RuntimeError(msg)
            if not duplicate_tracker.record(row):
                msg = (
                    "THS limit_up_pool pagination returned duplicate info row: "
                    f"trade_date={trade_date.isoformat()}, page={page_number}"
                )
                raise RuntimeError(msg)

        source_response_date = str(data.get("date")) if data.get("date") is not None else None
        pages.append(data)
        page_number += 1

    fetched_at = time.perf_counter()
    table_result = ths_limit_up_pool_to_table(pages)
    table_built_at = time.perf_counter()
    stats = client.stats
    last_page = pages[-1] if pages else {}
    trade_status = _raw_metadata_value(last_page.get("trade_status"))
    return table_result.table, {
        "source_endpoint": THS_LIMIT_UP_POOL_URL,
        "source_status_code": 0,
        "request_count": stats.request_count,
        "retry_count": stats.retry_count,
        "transient_error_count": stats.transient_error_count,
        "http_4xx_count": stats.http_4xx_count,
        "http_5xx_count": stats.http_5xx_count,
        "decode_error_count": stats.decode_error_count,
        "empty_response_count": 1 if not duplicate_tracker.has_rows else 0,
        "page_count": len(pages),
        "page_total": total_pages,
        "duplicate_page_row_count": duplicate_tracker.duplicate_count,
        "source_response_date": source_response_date,
        "trade_status": trade_status,
        "row_count": table_result.table.num_rows,
        "column_count": table_result.table.num_columns,
        "unknown_field_count": table_result.unknown_field_count,
        "http_fetch_seconds": round(fetched_at - started_at, 6),
        "table_convert_seconds": round(table_built_at - fetched_at, 6),
    }


def _raw_metadata_value(value: object) -> RawMetadataValue:
    if isinstance(value, Mapping):
        return dg.MetadataValue.json(dict(value))
    if isinstance(value, list):
        return dg.MetadataValue.json(value)
    if isinstance(value, str | int | float | bool) or value is None:
        return value
    return str(value)


def limit_up_pool_params(*, trade_date: date, page_number: int) -> dict[str, str]:
    return {
        "page": str(page_number),
        "limit": THS_LIMIT_UP_POOL_LIMIT,
        "field": THS_LIMIT_UP_POOL_FIELD,
        "filter": "HS,GEM2STAR",
        "order_field": "330324",
        "order_type": "0",
        "date": trade_date.strftime("%Y%m%d"),
        "_": str(int(time.time() * 1000)),
    }
