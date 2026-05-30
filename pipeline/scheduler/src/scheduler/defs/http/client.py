from __future__ import annotations

import asyncio
import json
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from typing import Literal, cast, overload

import aiohttp

from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY, ExponentialBackoffPolicy
from scheduler.defs.http.protocols import HttpResponseContextProtocol, HttpSessionProtocol

HTTP_TOTAL_TIMEOUT_SECONDS = 60
HTTP_CONNECT_TIMEOUT_SECONDS = 5
HTTP_READ_TIMEOUT_SECONDS = 30
HTTP_MAX_ATTEMPTS = 4
HTTP_CONNECTOR_LIMIT = 20
HTTP_CONNECTOR_LIMIT_PER_HOST = 20
CHROME_USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/125.0 Safari/537.36"
)

HttpMethod = Literal["GET", "POST"]
HeaderFactory = Callable[[], Mapping[str, str]]
SessionFactory = Callable[[], HttpSessionProtocol]


class HttpRequestError(RuntimeError):
    """Raised when an HTTP request fails permanently."""


class HttpTransientRequestError(HttpRequestError):
    """Raised internally for retryable HTTP failures."""


class HttpResponseDecodeError(HttpTransientRequestError):
    """Raised when a response body cannot be decoded as the expected type."""


@dataclass(frozen=True)
class HttpRequest:
    method: HttpMethod
    url: str
    params: Mapping[str, str] | None = None
    headers: Mapping[str, str] | HeaderFactory | None = None
    json_body: object | None = None


@dataclass(frozen=True)
class HttpTextResponse:
    status: int
    headers: Mapping[str, str]
    body: str


@dataclass(frozen=True)
class HttpBytesResponse:
    status: int
    headers: Mapping[str, str]
    body: bytes


@dataclass
class HttpFetchStats:
    request_count: int = 0
    retry_count: int = 0
    transient_error_count: int = 0
    http_4xx_count: int = 0
    http_5xx_count: int = 0
    decode_error_count: int = 0


class _AioHttpSessionAdapter:
    def __init__(self, session: aiohttp.ClientSession) -> None:
        self._session = session

    def request(
        self,
        method: str,
        url: str,
        *,
        params: Mapping[str, str] | None = None,
        headers: Mapping[str, str] | None = None,
        json: object | None = None,
    ) -> HttpResponseContextProtocol:
        return cast(
            HttpResponseContextProtocol,
            self._session.request(
                method,
                url,
                params=params,
                headers=headers,
                json=json,
            ),
        )

    async def close(self) -> None:
        await self._session.close()


class AioHttpClient:
    def __init__(
        self,
        *,
        headers: Mapping[str, str] | HeaderFactory | None = None,
        retry_policy: ExponentialBackoffPolicy = DEFAULT_RETRY_POLICY,
        max_attempts: int = HTTP_MAX_ATTEMPTS,
        total_timeout_seconds: float = HTTP_TOTAL_TIMEOUT_SECONDS,
        read_timeout_seconds: float = HTTP_READ_TIMEOUT_SECONDS,
        connector_limit: int = HTTP_CONNECTOR_LIMIT,
        connector_limit_per_host: int = HTTP_CONNECTOR_LIMIT_PER_HOST,
        session_factory: SessionFactory | None = None,
        request_delay: float = 0.0,
    ) -> None:
        if max_attempts < 1:
            msg = "max_attempts must be positive"
            raise ValueError(msg)
        if connector_limit < 1:
            msg = "connector_limit must be positive"
            raise ValueError(msg)
        if connector_limit_per_host < 1:
            msg = "connector_limit_per_host must be positive"
            raise ValueError(msg)
        if total_timeout_seconds <= 0:
            msg = "total_timeout_seconds must be positive"
            raise ValueError(msg)
        if read_timeout_seconds <= 0:
            msg = "read_timeout_seconds must be positive"
            raise ValueError(msg)
        if request_delay < 0:
            msg = "request_delay must be non-negative"
            raise ValueError(msg)

        self._headers = headers
        self._retry_policy = retry_policy
        self._max_attempts = max_attempts
        self._total_timeout_seconds = total_timeout_seconds
        self._read_timeout_seconds = read_timeout_seconds
        self._connector_limit = connector_limit
        self._connector_limit_per_host = connector_limit_per_host
        self._session_factory = session_factory
        self._request_delay = request_delay
        self._session: HttpSessionProtocol | None = None
        self.stats = HttpFetchStats()

    async def __aenter__(self) -> AioHttpClient:
        if self._session_factory is not None:
            self._session = self._session_factory()
            return self

        timeout = aiohttp.ClientTimeout(
            total=self._total_timeout_seconds,
            sock_connect=HTTP_CONNECT_TIMEOUT_SECONDS,
            sock_read=self._read_timeout_seconds,
        )
        connector = aiohttp.TCPConnector(
            limit=self._connector_limit,
            limit_per_host=self._connector_limit_per_host,
        )
        self._session = _AioHttpSessionAdapter(
            aiohttp.ClientSession(connector=connector, timeout=timeout)
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

    async def request_text(self, request: HttpRequest) -> HttpTextResponse:
        return await self._request_with_retries(
            request,
            body_kind="text",
            decode_json=False,
        )

    async def request_json_object(self, request: HttpRequest) -> Mapping[str, object]:
        response = await self._request_with_retries(
            request,
            body_kind="text",
            decode_json=True,
        )
        payload: object = json.loads(response.body)
        if not isinstance(payload, Mapping):
            msg = "HTTP response JSON is not an object"
            raise HttpResponseDecodeError(msg)
        return payload

    async def request_bytes(self, request: HttpRequest) -> HttpBytesResponse:
        return await self._request_with_retries(
            request,
            body_kind="bytes",
            decode_json=False,
        )

    @overload
    async def _request_with_retries(
        self,
        request: HttpRequest,
        *,
        body_kind: Literal["text"],
        decode_json: bool,
    ) -> HttpTextResponse: ...

    @overload
    async def _request_with_retries(
        self,
        request: HttpRequest,
        *,
        body_kind: Literal["bytes"],
        decode_json: bool,
    ) -> HttpBytesResponse: ...

    async def _request_with_retries(
        self,
        request: HttpRequest,
        *,
        body_kind: Literal["text", "bytes"],
        decode_json: bool,
    ) -> HttpTextResponse | HttpBytesResponse:
        delays = self._retry_policy.delays(self._max_attempts)
        for attempt_index in range(self._max_attempts):
            try:
                response = await self._send_once(request, body_kind=body_kind)
                if body_kind == "text" and decode_json:
                    try:
                        json.loads(response.body)
                    except json.JSONDecodeError as error:
                        self.stats.decode_error_count += 1
                        msg = f"HTTP response JSON decode failed: {error}"
                        raise HttpResponseDecodeError(msg) from error
                # 请求成功后添加延迟
                if self._request_delay > 0:
                    await asyncio.sleep(self._request_delay)
                return response
            except (TimeoutError, aiohttp.ClientError) as error:
                self.stats.transient_error_count += 1
                if attempt_index == self._max_attempts - 1:
                    msg = f"HTTP request failed after {self._max_attempts} attempts"
                    raise HttpRequestError(msg) from error
                self.stats.retry_count += 1
                await asyncio.sleep(delays[attempt_index])
            except HttpTransientRequestError as error:
                if attempt_index == self._max_attempts - 1:
                    msg = f"HTTP request failed after {self._max_attempts} attempts"
                    raise HttpRequestError(msg) from error
                self.stats.retry_count += 1
                await asyncio.sleep(delays[attempt_index])

        msg = "HTTP request retry loop ended unexpectedly"
        raise HttpRequestError(msg)

    @overload
    async def _send_once(
        self,
        request: HttpRequest,
        *,
        body_kind: Literal["text"],
    ) -> HttpTextResponse: ...

    @overload
    async def _send_once(
        self,
        request: HttpRequest,
        *,
        body_kind: Literal["bytes"],
    ) -> HttpBytesResponse: ...

    async def _send_once(
        self,
        request: HttpRequest,
        *,
        body_kind: Literal["text", "bytes"],
    ) -> HttpTextResponse | HttpBytesResponse:
        if self._session is None:
            msg = "AioHttpClient must be used as an async context manager"
            raise RuntimeError(msg)

        self.stats.request_count += 1
        headers = self._merged_headers(request.headers)

        async with self._session.request(
            request.method,
            request.url,
            params=request.params,
            headers=headers or None,
            json=request.json_body,
        ) as response:
            if body_kind == "bytes":
                body = await response.read()
                if response.status == 429 or response.status >= 500:
                    self.stats.transient_error_count += 1
                    if response.status >= 500:
                        self.stats.http_5xx_count += 1
                    body_preview = body[:300].decode(errors="replace")
                    msg = f"HTTP {response.status}: {body_preview}"
                    raise HttpTransientRequestError(msg)
                if response.status >= 400:
                    self.stats.http_4xx_count += 1
                    body_preview = body[:300].decode(errors="replace")
                    msg = f"HTTP {response.status}: {body_preview}"
                    raise HttpRequestError(msg)
                return HttpBytesResponse(
                    status=response.status,
                    headers=dict(response.headers),
                    body=body,
                )

            body = await response.text()
            if response.status == 429 or response.status >= 500:
                self.stats.transient_error_count += 1
                if response.status >= 500:
                    self.stats.http_5xx_count += 1
                body_preview = body[:300]
                msg = f"HTTP {response.status}: {body_preview}"
                raise HttpTransientRequestError(msg)
            if response.status >= 400:
                self.stats.http_4xx_count += 1
                body_preview = body[:300]
                msg = f"HTTP {response.status}: {body_preview}"
                raise HttpRequestError(msg)
            return HttpTextResponse(
                status=response.status,
                headers=dict(response.headers),
                body=body,
            )

    def _merged_headers(
        self,
        request_headers: Mapping[str, str] | HeaderFactory | None,
    ) -> dict[str, str]:
        headers: dict[str, str] = {}
        if self._headers is not None:
            headers.update(_headers_from_value(self._headers))
        if request_headers is not None:
            headers.update(_headers_from_value(request_headers))
        return headers


def browser_json_headers() -> dict[str, str]:
    return {
        "User-Agent": CHROME_USER_AGENT,
        "Accept": "application/json,text/plain,*/*",
        "Content-Type": "application/json",
    }


def browser_text_headers() -> dict[str, str]:
    return {
        "User-Agent": CHROME_USER_AGENT,
        "Accept": "text/plain,*/*",
    }


def with_referer(headers: Mapping[str, str], referer: str) -> dict[str, str]:
    return {**headers, "Referer": referer}


def _headers_from_value(headers: Mapping[str, str] | HeaderFactory) -> Mapping[str, str]:
    if callable(headers):
        return headers()
    return headers
