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
from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY
from scheduler.defs.common.strings import required_string
from scheduler.defs.config.env import JIUYAN_COOKIE, JIUYAN_TOKEN
from scheduler.defs.http.client import (
    HeaderFactory,
    HttpRequest,
    browser_json_headers,
)
from scheduler.defs.http.client_factory import HttpClientFactory
from scheduler.defs.http.partitioning import (
    JIUYAN_BACKFILL_MAX_TRADE_DATES,
    TRADE_DATE_PARTITION_KEY_NAME,
    TradeDateRangeMaterializationResult,
    jiuyan_action_field_daily_partitions,
    materialize_trade_date_range,
)
from scheduler.defs.http.protocols import HttpJsonStatsClientProtocol
from scheduler.defs.http.schemas import (
    FLATTEN_COLUMN_NAMING,
    empty_jiuyan_action_field_table,
    jiuyan_action_field_to_table,
)
from scheduler.defs.market.asset_keys import SINA_TRADE_CALENDAR_ASSET_KEY, SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.resources.s3 import S3SettingsResource

JIUYAN_ACTION_FIELD_URL = "https://app.jiuyangongshe.com/jystock-app/api/v1/action/field"
JIUYAN_RATE_LIMIT_ERR_CODE = "9"
JIUYAN_RATE_LIMIT_MAX_RETRIES = 5
JIUYAN_RATE_LIMIT_BASE_DELAY = 1.0


class MarketEventBackfillConfig(dg.Config):
    max_concurrent_trade_dates: int = 10
    request_delay: float = 0.0


def jiuyan_header_factory() -> HeaderFactory:
    token = JIUYAN_TOKEN.get_value()
    cookie = JIUYAN_COOKIE.get_value()
    if not token:
        msg = "JIUYAN_TOKEN is required"
        raise RuntimeError(msg)
    if not cookie:
        msg = "JIUYAN_COOKIE is required"
        raise RuntimeError(msg)

    def headers() -> Mapping[str, str]:
        return {
            **browser_json_headers(),
            "token": token,
            "cookie": cookie,
            "platform": "3",
            "timestamp": str(int(time.time() * 1000)),
        }

    return headers


@dg.asset(
    name="jiuyan__action_field",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    partitions_def=jiuyan_action_field_daily_partitions,
    deps=[SINA_TRADE_CALENDAR_ASSET_KEY],
    backfill_policy=dg.BackfillPolicy.single_run(),
    metadata=daily_sparse_partition_metadata(
        partition_key_name=TRADE_DATE_PARTITION_KEY_NAME,
        trade_date_filter=SINA_TRADE_CALENDAR_ASSET_KEY.to_user_string(),
        flatten_column_naming=FLATTEN_COLUMN_NAMING,
    ),
    owners=source_owners(),
    kinds=s3_parquet_kinds("http"),
    tags=source_tags("jiuyan"),
)
def jiuyan__action_field(
    context: dg.AssetExecutionContext,
    config: MarketEventBackfillConfig,
    s3_settings: S3SettingsResource,
) -> dg.MaterializeResult:
    """JiuYan action-field market-event content by trade-date partition."""

    result = asyncio.run(_materialize_action_field_range(context, config, s3_settings))
    return dg.MaterializeResult(metadata=result.metadata)


async def _materialize_action_field_range(
    context: dg.AssetExecutionContext,
    config: MarketEventBackfillConfig,
    s3_settings: S3SettingsResource,
) -> TradeDateRangeMaterializationResult:
    async with HttpClientFactory(retry_policy=DEFAULT_RETRY_POLICY).json_client(
        headers=jiuyan_header_factory(),
        request_delay=config.request_delay,
    ) as client:
        result = await materialize_trade_date_range(
            context,
            max_concurrent_trade_dates=config.max_concurrent_trade_dates,
            fetch_table_for_trade_date=lambda trade_date: fetch_action_field_table_with_client(
                client,
                trade_date=trade_date,
            ),
            backfill_window_limit=JIUYAN_BACKFILL_MAX_TRADE_DATES,
            s3_config=s3_settings.config(),
        )
        result.metadata.update(http_stats_metadata(client.stats))
        return result


async def fetch_action_field_table_with_client(
    client: HttpJsonStatsClientProtocol,
    *,
    trade_date: date,
) -> tuple[pa.Table, Mapping[str, RawMetadataValue]]:
    started_at = time.perf_counter()
    retry_rounds = 0

    for attempt in range(JIUYAN_RATE_LIMIT_MAX_RETRIES + 1):
        payload = await client.request_json_object(
            HttpRequest(
                method="POST",
                url=JIUYAN_ACTION_FIELD_URL,
                json_body={"pc": "1", "date": trade_date.isoformat()},
            )
        )
        err_code = required_string(payload.get("errCode"), field_name="errCode")

        # 非限流响应（成功或其它错误），结束重试循环，留待循环外统一校验
        if err_code != JIUYAN_RATE_LIMIT_ERR_CODE:
            break

        # 服务繁忙，指数退避重试
        retry_rounds = attempt + 1
        if attempt < JIUYAN_RATE_LIMIT_MAX_RETRIES:
            delay = JIUYAN_RATE_LIMIT_BASE_DELAY * (2**attempt)
            await asyncio.sleep(delay)

    if err_code == JIUYAN_RATE_LIMIT_ERR_CODE:
        raise RuntimeError(
            f"JiuYan action_field still rate-limited (errCode={err_code}) after "
            f"{JIUYAN_RATE_LIMIT_MAX_RETRIES} retries for trade_date={trade_date.isoformat()}: "
            f"{payload.get('msg')}"
        )

    if err_code != "0":
        msg = payload.get("msg")
        raise RuntimeError(
            f"JiuYan action_field returned errCode={err_code} for trade_date={trade_date.isoformat()}: {msg}"
        )

    fetched_at = time.perf_counter()

    data = payload.get("data")
    if not isinstance(data, list):
        msg = "JiuYan action_field response data is not an array"
        raise RuntimeError(msg)
    content_rows = _mapping_rows(data, context="JiuYan action_field data")
    if content_rows:
        table_result = jiuyan_action_field_to_table(content_rows)
        table = table_result.table
        unknown_field_count = table_result.unknown_field_count
    else:
        table = empty_jiuyan_action_field_table()
        unknown_field_count = 0
    table_built_at = time.perf_counter()
    stats = client.stats
    return table, {
        "source_endpoint": JIUYAN_ACTION_FIELD_URL,
        "source_err_code": err_code,
        "retry_rounds": retry_rounds,
        "request_count": stats.request_count,
        "retry_count": stats.retry_count,
        "transient_error_count": stats.transient_error_count,
        "http_4xx_count": stats.http_4xx_count,
        "http_5xx_count": stats.http_5xx_count,
        "decode_error_count": stats.decode_error_count,
        "empty_response_count": 1 if not content_rows else 0,
        "row_count": table.num_rows,
        "column_count": table.num_columns,
        "unknown_field_count": unknown_field_count,
        "http_fetch_seconds": round(fetched_at - started_at, 6),
        "table_convert_seconds": round(table_built_at - fetched_at, 6),
    }


def _mapping_rows(values: list[object], *, context: str) -> list[Mapping[str, object]]:
    rows: list[Mapping[str, object]] = []
    for value in values:
        if not isinstance(value, Mapping):
            msg = f"{context} item is not an object"
            raise RuntimeError(msg)
        rows.append(value)
    return rows
