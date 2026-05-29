from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from scheduler.defs.http.client import (
    HttpBytesResponse,
    HttpFetchStats,
    HttpRequest,
    HttpTextResponse,
)


@dataclass(frozen=True)
class FakeAioHttpRequest:
    method: str
    url: str
    params: Mapping[str, str] | None = None
    headers: Mapping[str, str] | None = None
    json: object | None = None


class FakeAioHttpResponse:
    def __init__(
        self,
        *,
        status: int = 200,
        body: str | bytes = "",
        headers: Mapping[str, str] | None = None,
    ) -> None:
        self.status = status
        self._body = body
        self.headers = dict(headers or {})

    async def __aenter__(self) -> FakeAioHttpResponse:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: object,
    ) -> None:
        return None

    async def text(self) -> str:
        if isinstance(self._body, bytes):
            return self._body.decode("utf-8", errors="replace")
        return self._body

    async def read(self) -> bytes:
        if isinstance(self._body, bytes):
            return self._body
        return self._body.encode("utf-8")


class FakeAioHttpSession:
    def __init__(self, outcomes: list[object]) -> None:
        self.outcomes = outcomes
        self.requests: list[FakeAioHttpRequest] = []
        self.closed = False

    def request(
        self,
        method: str,
        url: str,
        *,
        params: Mapping[str, str] | None = None,
        headers: Mapping[str, str] | None = None,
        json: object | None = None,
    ) -> FakeAioHttpResponse:
        self.requests.append(
            FakeAioHttpRequest(
                method=method,
                url=url,
                params=params,
                headers=headers,
                json=json,
            )
        )
        outcome = self.outcomes.pop(0)
        if isinstance(outcome, BaseException):
            raise outcome
        if not isinstance(outcome, FakeAioHttpResponse):
            msg = "Fake outcome must be a response or exception"
            raise TypeError(msg)
        return outcome

    async def close(self) -> None:
        self.closed = True


class FakeJsonClient:
    def __init__(self, payloads: list[dict[str, object]]) -> None:
        self.payloads = payloads
        self.requests: list[HttpRequest] = []
        self.stats = HttpFetchStats()

    async def request_json_object(self, request: HttpRequest) -> dict[str, object]:
        self.requests.append(request)
        self.stats.request_count += 1
        return self.payloads.pop(0)


class FakeSinaHttpClient:
    def __init__(self, body: str) -> None:
        self.body = body
        self.requests: list[HttpRequest] = []

    async def request_text(self, request: HttpRequest) -> HttpTextResponse:
        self.requests.append(request)
        return HttpTextResponse(status=200, headers={}, body=self.body)


class FakeBytesClient:
    def __init__(
        self,
        body: bytes,
        headers: Mapping[str, str] | None = None,
    ) -> None:
        self.body = body
        self.headers = dict(headers or {})
        self.request_url: str | None = None
        self.request_headers: Mapping[str, str] | None = None

    async def request_bytes(self, request: HttpRequest) -> HttpBytesResponse:
        self.request_url = request.url
        self.request_headers = request.headers if isinstance(request.headers, Mapping) else None
        return HttpBytesResponse(status=200, headers=self.headers, body=self.body)


class FakeEastmoneyHttpClient:
    def __init__(self, pages: dict[int, dict[str, object]]) -> None:
        self.pages = pages
        self.requested_pages: list[int] = []
        self.stats = HttpFetchStats()

    async def __aenter__(self) -> FakeEastmoneyHttpClient:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: object,
    ) -> None:
        return None

    async def request_json_object(self, request: HttpRequest) -> dict[str, object]:
        params: Mapping[str, str] = request.params or {}
        page_number = int(params.get("p") or params.get("pageNumber") or "0")
        self.requested_pages.append(page_number)
        self.stats.request_count += 1
        return self.pages[page_number]
