from __future__ import annotations

import asyncio
from collections.abc import Mapping
from dataclasses import dataclass, field
from datetime import date
from types import TracebackType

from scheduler.defs.common.numbers import positive_int_or_default
from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY, ExponentialBackoffPolicy
from scheduler.defs.http.client import (
    HTTP_CONNECTOR_LIMIT,
    AioHttpClient,
    HttpRequest,
    HttpRequestError,
    browser_json_headers,
)
from scheduler.defs.http.pagination import DuplicateRowTracker
from scheduler.defs.http.protocols import HttpJsonStatsContextClientProtocol
from scheduler.defs.sources.eastmoney.schema import EastmoneyEndpointConfig

EASTMONEY_CODE_CONCURRENCY = 20
EASTMONEY_MAX_ATTEMPTS = 4


class EastmoneyRequestError(RuntimeError):
    """Raised when EastMoney returns an unrecoverable response."""


@dataclass
class EastmoneyFetchStats:
    request_count: int = 0
    empty_response_count: int = 0
    page_count: int = 0
    retry_count: int = 0
    duplicate_page_row_count: int = 0
    transient_error_count: int = 0
    http_4xx_count: int = 0
    http_5xx_count: int = 0
    decode_error_count: int = 0
    status_code_counts: dict[str, int] = field(default_factory=dict)
    endpoint_host_counts: dict[str, int] = field(default_factory=dict)


@dataclass(frozen=True)
class EastmoneyPage:
    rows: list[dict[str, object]]
    total_pages: int
    is_empty: bool = False


class EastmoneyAioHttpClient:
    def __init__(
        self,
        *,
        code_concurrency_limit: int = EASTMONEY_CODE_CONCURRENCY,
        retry_policy: ExponentialBackoffPolicy = DEFAULT_RETRY_POLICY,
        max_attempts: int = EASTMONEY_MAX_ATTEMPTS,
        http_client: HttpJsonStatsContextClientProtocol | None = None,
    ) -> None:
        if code_concurrency_limit < 1:
            msg = "code_concurrency_limit must be positive"
            raise ValueError(msg)
        if max_attempts < 1:
            msg = "max_attempts must be positive"
            raise ValueError(msg)

        self.code_concurrency_limit = code_concurrency_limit
        self._retry_policy = retry_policy
        self._max_attempts = max_attempts
        self._semaphore = asyncio.Semaphore(code_concurrency_limit)
        self._provided_http_client = http_client
        self._http_client: HttpJsonStatsContextClientProtocol | None = None
        self.stats = EastmoneyFetchStats()

    async def __aenter__(self) -> EastmoneyAioHttpClient:
        self._http_client = self._provided_http_client or AioHttpClient(
            headers=browser_json_headers(),
            retry_policy=self._retry_policy,
            max_attempts=self._max_attempts,
            connector_limit=min(self.code_concurrency_limit, HTTP_CONNECTOR_LIMIT),
            connector_limit_per_host=min(self.code_concurrency_limit, HTTP_CONNECTOR_LIMIT),
        )
        await self._http_client.__aenter__()
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        if self._http_client is not None:
            await self._http_client.__aexit__(exc_type, exc_value, traceback)
        self._http_client = None

    async def fetch_code_range(
        self,
        endpoint: EastmoneyEndpointConfig,
        code: str,
        start_date: date,
        end_date: date,
    ) -> list[dict[str, object]]:
        async with self._semaphore:
            return await self._fetch_code_range_unlocked(
                endpoint,
                code,
                start_date,
                end_date,
            )

    async def _fetch_code_range_unlocked(
        self,
        endpoint: EastmoneyEndpointConfig,
        code: str,
        start_date: date,
        end_date: date,
    ) -> list[dict[str, object]]:
        first_page_payload = await self._request_json(
            endpoint.source_endpoint,
            build_request_params(endpoint, code, start_date, end_date, page_number=1),
        )
        first_page = parse_eastmoney_page(endpoint, first_page_payload)
        if first_page.is_empty:
            self.stats.empty_response_count += 1
            return []

        rows = list(first_page.rows)
        duplicate_tracker = DuplicateRowTracker()
        for row in rows:
            duplicate_tracker.record(row)
        self.stats.page_count += 1

        for page_number in range(2, first_page.total_pages + 1):
            payload = await self._request_json(
                endpoint.source_endpoint,
                build_request_params(
                    endpoint,
                    code,
                    start_date,
                    end_date,
                    page_number=page_number,
                ),
            )
            page = parse_eastmoney_page(endpoint, payload)
            self.stats.page_count += 1
            for row in page.rows:
                if not duplicate_tracker.record(row):
                    self.stats.duplicate_page_row_count = duplicate_tracker.duplicate_count
                    msg = (
                        "EastMoney pagination returned a duplicate row across pages: "
                        f"asset={endpoint.asset_name}, code={code}, page={page_number}"
                    )
                    raise EastmoneyRequestError(msg)
                rows.append(row)

        return rows

    async def _request_json(
        self,
        url: str,
        params: Mapping[str, str],
    ) -> Mapping[str, object]:
        if self._http_client is None:
            msg = "EastmoneyAioHttpClient must be used as an async context manager"
            raise RuntimeError(msg)

        try:
            payload = await self._http_client.request_json_object(
                HttpRequest(method="GET", url=url, params=params)
            )
        except HttpRequestError as error:
            self._sync_http_stats()
            msg = f"EastMoney request failed: {error}"
            raise EastmoneyRequestError(msg) from error

        self._sync_http_stats()
        return payload

    def _sync_http_stats(self) -> None:
        if self._http_client is None:
            return
        http_stats = self._http_client.stats
        self.stats.request_count = http_stats.request_count
        self.stats.retry_count = http_stats.retry_count
        self.stats.transient_error_count = http_stats.transient_error_count
        self.stats.http_4xx_count = http_stats.http_4xx_count
        self.stats.http_5xx_count = http_stats.http_5xx_count
        self.stats.decode_error_count = http_stats.decode_error_count
        self.stats.status_code_counts = dict(http_stats.status_code_counts)
        self.stats.endpoint_host_counts = dict(http_stats.endpoint_host_counts)


def build_request_params(
    endpoint: EastmoneyEndpointConfig,
    code: str,
    start_date: date,
    end_date: date,
    *,
    page_number: int,
) -> dict[str, str]:
    if page_number < 1:
        msg = "page_number must be positive"
        raise ValueError(msg)
    if start_date > end_date:
        msg = "start_date must be less than or equal to end_date"
        raise ValueError(msg)

    params = {
        **dict(endpoint.fixed_params),
        "filter": _build_filter(endpoint, code, start_date, end_date),
        "source": "HSF10",
        "client": "PC",
    }
    if endpoint.api_family == "data_get":
        params.update(
            {
                "p": str(page_number),
                "ps": str(endpoint.page_size),
                "st": ",".join(endpoint.sort_fields),
                "sr": ",".join(endpoint.sort_directions),
            }
        )
    elif endpoint.api_family == "data_v1_get":
        params.update(
            {
                "pageNumber": str(page_number),
                "pageSize": str(endpoint.page_size),
                "sortColumns": ",".join(endpoint.sort_fields),
                "sortTypes": ",".join(endpoint.sort_directions),
                "quoteColumns": "",
            }
        )
    else:
        msg = f"Unsupported EastMoney API family: {endpoint.api_family}"
        raise ValueError(msg)
    return params


def parse_eastmoney_page(
    endpoint: EastmoneyEndpointConfig,
    payload: Mapping[str, object],
) -> EastmoneyPage:
    if (
        endpoint.api_family == "data_v1_get"
        and payload.get("code") == 9201
        and payload.get("result") is None
    ):
        return EastmoneyPage(rows=[], total_pages=0, is_empty=True)

    result = payload.get("result")
    if result is None:
        return EastmoneyPage(rows=[], total_pages=0, is_empty=True)
    if not isinstance(result, Mapping):
        msg = f"EastMoney result is not an object for {endpoint.asset_name}"
        raise EastmoneyRequestError(msg)

    data = result.get("data")
    if data is None or data == []:
        return EastmoneyPage(rows=[], total_pages=0, is_empty=True)
    if not isinstance(data, list):
        msg = f"EastMoney result.data is not a list for {endpoint.asset_name}"
        raise EastmoneyRequestError(msg)

    rows: list[dict[str, object]] = []
    for item in data:
        if not isinstance(item, Mapping):
            msg = f"EastMoney result.data item is not an object for {endpoint.asset_name}"
            raise EastmoneyRequestError(msg)
        rows.append(dict(item))

    pages = positive_int_or_default(result.get("pages"), default=1)
    return EastmoneyPage(rows=rows, total_pages=pages)


def _build_filter(
    endpoint: EastmoneyEndpointConfig,
    code: str,
    start_date: date,
    end_date: date,
) -> str:
    return (
        f'(SECUCODE="{code}")'
        f"({endpoint.date_field}>='{start_date.isoformat()}')"
        f"({endpoint.date_field}<='{end_date.isoformat()}')"
    )
