from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    from types import TracebackType

    from scheduler.defs.http.client import HttpBytesResponse, HttpRequest, HttpTextResponse


class HttpStatsProtocol(Protocol):
    @property
    def request_count(self) -> int: ...

    @property
    def retry_count(self) -> int: ...

    @property
    def transient_error_count(self) -> int: ...

    @property
    def http_4xx_count(self) -> int: ...

    @property
    def http_5xx_count(self) -> int: ...

    @property
    def decode_error_count(self) -> int: ...

    @property
    def status_code_counts(self) -> Mapping[str, int]: ...

    @property
    def endpoint_host_counts(self) -> Mapping[str, int]: ...


class HttpJsonClientProtocol(Protocol):
    async def request_json_object(self, request: HttpRequest) -> Mapping[str, object]: ...


class HttpJsonStatsClientProtocol(HttpJsonClientProtocol, Protocol):
    @property
    def stats(self) -> HttpStatsProtocol: ...


class HttpJsonStatsContextClientProtocol(HttpJsonStatsClientProtocol, Protocol):
    async def __aenter__(self) -> HttpJsonStatsContextClientProtocol: ...

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...


class HttpTextClientProtocol(Protocol):
    async def request_text(self, request: HttpRequest) -> HttpTextResponse: ...


class HttpBytesClientProtocol(Protocol):
    async def request_bytes(self, request: HttpRequest) -> HttpBytesResponse: ...


class HttpResponseProtocol(Protocol):
    @property
    def status(self) -> int: ...

    @property
    def headers(self) -> Mapping[str, str]: ...

    async def text(self) -> str: ...

    async def read(self) -> bytes: ...


class HttpResponseContextProtocol(Protocol):
    async def __aenter__(self) -> HttpResponseProtocol: ...

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...


class HttpSessionProtocol(Protocol):
    def request(
        self,
        method: str,
        url: str,
        *,
        params: Mapping[str, str] | None = None,
        headers: Mapping[str, str] | None = None,
        json: object | None = None,
    ) -> HttpResponseContextProtocol: ...

    async def close(self) -> None: ...
