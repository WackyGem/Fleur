import asyncio
import time
from collections.abc import Mapping

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    latest_snapshot_metadata,
    s3_parquet_kinds,
    source_owners,
    source_tags,
)
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY
from scheduler.defs.common.strings import required_string
from scheduler.defs.http.client import HttpRequest
from scheduler.defs.http.client_factory import HttpClientFactory
from scheduler.defs.http.protocols import HttpJsonStatsClientProtocol
from scheduler.defs.http.schemas import (
    FLATTEN_COLUMN_NAMING,
    jiuyan_industry_list_to_table,
)
from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.sources.jiuyan.action_field import jiuyan_header_factory

JIUYAN_INDUSTRY_LIST_URL = "https://app.jiuyangongshe.com/jystock-app/api/v1/industry/list"
JIUYAN_INDUSTRY_LIST_LIMIT = "500"


@dg.asset(
    name="jiuyan__industry_list",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    io_manager_key="s3_io_manager",
    metadata=latest_snapshot_metadata(flatten_column_naming=FLATTEN_COLUMN_NAMING),
    owners=source_owners(),
    kinds=s3_parquet_kinds("http"),
    tags=source_tags("jiuyan"),
)
def jiuyan__industry_list(
    context: dg.AssetExecutionContext,
) -> dg.MaterializeResult[pa.Table]:
    """Latest snapshot of JiuYan industry research list pages."""

    table, metadata = asyncio.run(_fetch_industry_list_table())
    context.log.info("Fetched %s JiuYan industry-list page rows", table.num_rows)
    return dg.MaterializeResult(value=table, metadata=metadata)


async def _fetch_industry_list_table() -> tuple[pa.Table, dict[str, RawMetadataValue]]:
    started_at = time.perf_counter()
    async with HttpClientFactory(retry_policy=DEFAULT_RETRY_POLICY).json_client(
        headers=jiuyan_header_factory(),
    ) as client:
        return await fetch_industry_list_table_with_client(client, started_at=started_at)


async def fetch_industry_list_table_with_client(
    client: HttpJsonStatsClientProtocol,
    *,
    started_at: float,
) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
    pages: list[Mapping[str, object]] = []
    start = "0"
    terminal_has_next: bool | None = None
    while True:
        payload = await client.request_json_object(
            HttpRequest(
                method="POST",
                url=JIUYAN_INDUSTRY_LIST_URL,
                json_body={
                    "keyword": "",
                    "start": start,
                    "limit": JIUYAN_INDUSTRY_LIST_LIMIT,
                },
            )
        )
        err_code = required_string(payload.get("errCode"), field_name="errCode")
        if err_code != "0":
            msg = payload.get("msg")
            raise RuntimeError(f"JiuYan industry_list returned errCode={err_code}: {msg}")
        data = payload.get("data")
        if not isinstance(data, Mapping):
            msg = "JiuYan industry_list response data is not an object"
            raise RuntimeError(msg)
        result = data.get("result")
        if not isinstance(result, list):
            msg = "JiuYan industry_list data.result is not an array"
            raise RuntimeError(msg)

        pages.append(data)
        has_next = data.get("hasNext")
        if not isinstance(has_next, bool):
            msg = "JiuYan industry_list data.hasNext is not a boolean"
            raise RuntimeError(msg)
        terminal_has_next = has_next
        if not has_next:
            break
        next_page = data.get("nextPage")
        if not isinstance(next_page, int | str):
            msg = "JiuYan industry_list data.nextPage is not a page value"
            raise RuntimeError(msg)
        start = str(next_page)

    fetched_at = time.perf_counter()
    industry_total_rows = 0
    result_page_count = 0
    for page in pages:
        result_rows = page.get("result")
        if isinstance(result_rows, list):
            industry_total_rows += len(result_rows)
            if result_rows:
                result_page_count += 1
    if industry_total_rows == 0:
        msg = "JiuYan industry_list returned no result rows"
        raise RuntimeError(msg)

    table_result = jiuyan_industry_list_to_table(pages)
    table_built_at = time.perf_counter()
    stats = client.stats
    metadata: dict[str, RawMetadataValue] = {
        "source_endpoint": JIUYAN_INDUSTRY_LIST_URL,
        "source_err_code": "0",
        "request_count": stats.request_count,
        "retry_count": stats.retry_count,
        "transient_error_count": stats.transient_error_count,
        "http_4xx_count": stats.http_4xx_count,
        "http_5xx_count": stats.http_5xx_count,
        "decode_error_count": stats.decode_error_count,
        "empty_response_count": 0,
        "page_count": len(pages),
        "result_page_count": result_page_count,
        "industry_total_rows": industry_total_rows,
        "has_next_terminal_value": terminal_has_next,
        "row_count": table_result.table.num_rows,
        "column_count": table_result.table.num_columns,
        "unknown_field_count": table_result.unknown_field_count,
        "http_fetch_seconds": round(fetched_at - started_at, 6),
        "table_convert_seconds": round(table_built_at - fetched_at, 6),
        "asset_function_seconds": round(table_built_at - started_at, 6),
    }
    return table_result.table, metadata
