import asyncio
import time
from collections.abc import Mapping

import dagster as dg
import pyarrow as pa

from scheduler.defs.http_resources.client import AioHttpClient, HttpRequest
from scheduler.defs.http_resources.jiuyan__action_field import jiuyan_header_factory
from scheduler.defs.http_resources.schemas import (
    FLATTEN_COLUMN_NAMING,
    jiuyan_industry_list_to_table,
)
from scheduler.defs.util import DEFAULT_RETRY_POLICY

JIUYAN_INDUSTRY_LIST_URL = "https://app.jiuyangongshe.com/jystock-app/api/v1/industry/list"
JIUYAN_INDUSTRY_LIST_LIMIT = "500"


@dg.asset(
    name="jiuyan__industry_list",
    group_name="http_sources",
    io_manager_key="s3_io_manager",
    metadata={
        "storage_mode": "latest_snapshot",
        "flatten_column_naming": FLATTEN_COLUMN_NAMING,
    },
    tags={"source": "jiuyan", "layer": "raw", "storage": "s3"},
)
def jiuyan__industry_list(
    context: dg.AssetExecutionContext,
) -> dg.MaterializeResult[pa.Table]:
    """Latest snapshot of JiuYan industry research list pages."""

    table, metadata = asyncio.run(_fetch_industry_list_table())
    context.log.info("Fetched %s JiuYan industry-list page rows", table.num_rows)
    return dg.MaterializeResult(value=table, metadata=metadata)


async def _fetch_industry_list_table() -> tuple[pa.Table, dict[str, object]]:
    started_at = time.perf_counter()
    async with AioHttpClient(
        headers=jiuyan_header_factory(),
        retry_policy=DEFAULT_RETRY_POLICY,
    ) as client:
        return await _fetch_industry_list_table_with_client(client, started_at=started_at)


async def _fetch_industry_list_table_with_client(
    client: AioHttpClient,
    *,
    started_at: float,
) -> tuple[pa.Table, dict[str, object]]:
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
        err_code = _required_string(payload.get("errCode"), field_name="errCode")
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
    industry_total_rows = sum(
        len(page.get("result", [])) for page in pages if isinstance(page.get("result"), list)
    )
    if industry_total_rows == 0:
        msg = "JiuYan industry_list returned no result rows"
        raise RuntimeError(msg)

    table_result = jiuyan_industry_list_to_table(pages)
    table_built_at = time.perf_counter()
    stats = client.stats
    metadata = {
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
        "result_page_count": sum(
            1 for page in pages if isinstance(page.get("result"), list) and len(page["result"]) > 0
        ),
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


def _required_string(value: object, *, field_name: str) -> str:
    if not isinstance(value, str):
        msg = f"Expected {field_name} to be a string"
        raise RuntimeError(msg)
    return value
