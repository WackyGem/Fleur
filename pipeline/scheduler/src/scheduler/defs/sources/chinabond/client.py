from __future__ import annotations

from collections.abc import Mapping

from scheduler.defs.http.client import (
    AioHttpClient,
    HttpFetchStats,
    HttpRequest,
    browser_json_headers,
)

CHINABOND_HISTORY_QUERY_URL = "https://yield.chinabond.com.cn/cbweb-czb-web/czb/historyQuery"


class ChinabondRequestError(RuntimeError):
    """Raised when the ChinaBond response is structurally invalid."""


class ChinabondAioHttpClient:
    def __init__(self, http_client: AioHttpClient) -> None:
        self._http_client = http_client

    async def __aenter__(self) -> ChinabondAioHttpClient:
        await self._http_client.__aenter__()
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: object,
    ) -> None:
        await self._http_client.__aexit__(exc_type, exc_value, traceback)

    @property
    def stats(self) -> HttpFetchStats:
        return self._http_client.stats

    async def fetch_government_bond_curve(
        self,
        *,
        start_date: str,
        end_date: str,
    ) -> Mapping[str, object]:
        return await self._http_client.request_json_object(
            HttpRequest(
                method="GET",
                url=CHINABOND_HISTORY_QUERY_URL,
                params={
                    "startDate": start_date,
                    "endDate": end_date,
                    "gjqx": "0",
                    "locale": "cn_ZH",
                    "qxmc": "1",
                },
                headers=browser_json_headers,
            )
        )
