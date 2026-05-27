from __future__ import annotations

import asyncio
import json
from collections.abc import Mapping
from dataclasses import dataclass
from datetime import date
from typing import Any

import aiohttp

from scheduler.defs.eastmoney.schemas import EastmoneyEndpointConfig
from scheduler.defs.util import DEFAULT_RETRY_POLICY, ExponentialBackoffPolicy

EASTMONEY_HTTP_TOTAL_TIMEOUT_SECONDS = 60
EASTMONEY_HTTP_CONNECT_TIMEOUT_SECONDS = 5
EASTMONEY_HTTP_READ_TIMEOUT_SECONDS = 30
EASTMONEY_CODE_CONCURRENCY = 20
EASTMONEY_MAX_ATTEMPTS = 4
CHROME_USER_AGENT = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/125.0 Safari/537.36"
)


class EastmoneyRequestError(RuntimeError):
    """Raised when EastMoney returns an unrecoverable response."""


class EastmoneyTransientRequestError(RuntimeError):
    """Raised internally for retryable EastMoney request failures."""


@dataclass
class EastmoneyFetchStats:
    request_count: int = 0
    empty_response_count: int = 0
    page_count: int = 0
    retry_count: int = 0
    duplicate_page_row_count: int = 0


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
        self._session: aiohttp.ClientSession | None = None
        self.stats = EastmoneyFetchStats()

    async def __aenter__(self) -> EastmoneyAioHttpClient:
        timeout = aiohttp.ClientTimeout(
            total=EASTMONEY_HTTP_TOTAL_TIMEOUT_SECONDS,
            sock_connect=EASTMONEY_HTTP_CONNECT_TIMEOUT_SECONDS,
            sock_read=EASTMONEY_HTTP_READ_TIMEOUT_SECONDS,
        )
        connector = aiohttp.TCPConnector(
            limit=self.code_concurrency_limit,
            limit_per_host=self.code_concurrency_limit,
        )
        self._session = aiohttp.ClientSession(
            connector=connector,
            timeout=timeout,
            headers={"User-Agent": CHROME_USER_AGENT, "Accept": "application/json,*/*"},
        )
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: object,
    ) -> None:
        if self._session is not None:
            await self._session.close()
        self._session = None

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
        seen_fingerprints = {_row_fingerprint(row) for row in rows}
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
                fingerprint = _row_fingerprint(row)
                if fingerprint in seen_fingerprints:
                    self.stats.duplicate_page_row_count += 1
                    msg = (
                        "EastMoney pagination returned a duplicate row across pages: "
                        f"asset={endpoint.asset_name}, code={code}, page={page_number}"
                    )
                    raise EastmoneyRequestError(msg)
                seen_fingerprints.add(fingerprint)
                rows.append(row)

        return rows

    async def _request_json(
        self,
        url: str,
        params: Mapping[str, str],
    ) -> Mapping[str, object]:
        if self._session is None:
            msg = "EastmoneyAioHttpClient must be used as an async context manager"
            raise RuntimeError(msg)

        delays = self._retry_policy.delays(self._max_attempts)
        for attempt_index in range(self._max_attempts):
            try:
                self.stats.request_count += 1
                async with self._session.get(url, params=params) as response:
                    body = await response.text()
                    if response.status == 429 or response.status >= 500:
                        msg = f"EastMoney HTTP {response.status}: {body[:300]}"
                        raise EastmoneyTransientRequestError(msg)
                    if response.status >= 400:
                        msg = f"EastMoney HTTP {response.status}: {body[:300]}"
                        raise EastmoneyRequestError(msg)
                    return _loads_json_object(body)
            except (
                aiohttp.ClientError,
                asyncio.TimeoutError,
                json.JSONDecodeError,
                EastmoneyTransientRequestError,
            ) as error:
                if attempt_index == self._max_attempts - 1:
                    msg = f"EastMoney request failed after {self._max_attempts} attempts"
                    raise EastmoneyRequestError(msg) from error
                self.stats.retry_count += 1
                await asyncio.sleep(delays[attempt_index])

        msg = "EastMoney request retry loop ended unexpectedly"
        raise EastmoneyRequestError(msg)


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
    if endpoint.api_family == "data_v1_get" and payload.get("code") == 9201:
        if payload.get("result") is None:
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

    pages = _positive_int_or_default(result.get("pages"), default=1)
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


def _positive_int_or_default(value: object, *, default: int) -> int:
    if isinstance(value, bool):
        return default
    if isinstance(value, int | float | str):
        try:
            parsed = int(value)
        except ValueError:
            return default
        if parsed > 0:
            return parsed
    return default


def _loads_json_object(body: str) -> Mapping[str, object]:
    payload: Any = json.loads(body)
    if not isinstance(payload, Mapping):
        msg = "EastMoney response JSON is not an object"
        raise EastmoneyRequestError(msg)
    return payload


def _row_fingerprint(row: Mapping[str, object]) -> str:
    return json.dumps(row, sort_keys=True, ensure_ascii=False, default=str, separators=(",", ":"))
