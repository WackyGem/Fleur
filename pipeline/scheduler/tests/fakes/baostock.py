from __future__ import annotations

import zlib
from contextlib import AbstractAsyncContextManager
from datetime import date

from scheduler.defs.baostock.client import BaostockAioTcpClient
from scheduler.defs.baostock.protocol import (
    DEFAULT_PAGE_SIZE,
    MESSAGE_END,
    MESSAGE_SPLIT,
    SERVER_VERSION,
    BaostockNetworkError,
    BaostockResponse,
)
from scheduler.defs.baostock.schemas import K_HISTORY_DAILY_FIELDS, STOCK_BASIC_FIELDS
from scheduler.defs.baostock.services import BaostockClientProtocol
from scheduler.defs.common.retry import ExponentialBackoffPolicy
from scheduler.defs.config.models import BaostockClientConfig


def response_message(
    body_parts: list[str],
    *,
    response_code: str = "95",
    compressed: bool = False,
) -> bytes:
    body = MESSAGE_SPLIT.join(body_parts)
    if compressed:
        body_bytes = zlib.compress(body.encode("utf-8"))
        header = (
            f"{SERVER_VERSION}{MESSAGE_SPLIT}{response_code}{MESSAGE_SPLIT}{len(body_bytes):010d}"
        )
        return header.encode("utf-8") + body_bytes + MESSAGE_END

    header = f"{SERVER_VERSION}{MESSAGE_SPLIT}{response_code}{MESSAGE_SPLIT}{len(body):010d}"
    head_body = f"{header}{body}"
    crc32_value = zlib.crc32(head_body.encode("utf-8"))
    return f"{head_body}{MESSAGE_SPLIT}{crc32_value}".encode() + MESSAGE_END


def baostock_response(
    *,
    api_name: str = "query_stock_basic",
    records: list[list[str]] | None = None,
    field_names: list[str] | None = None,
    page: int = 1,
    page_size: int = DEFAULT_PAGE_SIZE,
    error_code: str = "0",
    error_message: str = "",
) -> BaostockResponse:
    return BaostockResponse(
        response_code="95",
        error_code=error_code,
        error_message=error_message,
        api_name=api_name,
        user_id="user",
        page=page,
        page_size=page_size,
        records=[] if records is None else records,
        field_names=STOCK_BASIC_FIELDS if field_names is None else field_names,
        params=[],
    )


def client_config(max_connections: int = 2) -> BaostockClientConfig:
    return BaostockClientConfig(
        host="baostock.test",
        port=10030,
        username="user",
        password="password",
        max_connections=max_connections,
    )


class FakeBaostockAssetClient:
    def __init__(self, *args: object, **kwargs: object) -> None:
        self.history_calls: list[tuple[str, date, date]] = []

    async def __aenter__(self) -> BaostockClientProtocol:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: object,
    ) -> None:
        return None

    async def query_stock_basic(
        self,
        code: str = "",
        code_name: str = "",
    ) -> BaostockResponse:
        return baostock_response(
            api_name="query_stock_basic",
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )

    async def query_history_k_data_plus_daily(
        self,
        code: str,
        start_date: date,
        end_date: date,
    ) -> BaostockResponse:
        self.history_calls.append((code, start_date, end_date))
        return baostock_response(
            api_name="query_history_k_data_plus",
            records=[
                [
                    start_date.isoformat(),
                    code,
                    "1",
                    "2",
                    "1",
                    "2",
                    "1",
                    "100",
                    "200",
                    "3",
                    "1.0",
                    "1",
                    "10.0",
                    "0",
                ]
            ],
            field_names=K_HISTORY_DAILY_FIELDS,
        )


class FakeBaostockClientFactory:
    def __init__(self, client: FakeBaostockAssetClient | None = None) -> None:
        self.created_max_connections: list[int | None] = []
        self._client = client or FakeBaostockAssetClient()

    def client(
        self,
        *,
        max_connections: int | None = None,
    ) -> AbstractAsyncContextManager[BaostockClientProtocol]:
        self.created_max_connections.append(max_connections)
        return self._client


class QueuedBaostockSender:
    def __init__(self, responses: list[BaostockResponse]) -> None:
        self.responses = responses
        self.payloads: list[bytes] = []
        self.timeout_seconds: list[float] = []

    async def __call__(
        self,
        payload: bytes,
        *,
        timeout_seconds: float,
    ) -> BaostockResponse:
        self.payloads.append(payload)
        self.timeout_seconds.append(timeout_seconds)
        return self.responses.pop(0)


class RetryingBaostockSender:
    def __init__(self, failures_before_success: int) -> None:
        self.failures_before_success = failures_before_success
        self.send_count = 0

    async def __call__(self, payload: bytes, *, timeout_seconds: float) -> BaostockResponse:
        self.send_count += 1
        if self.send_count <= self.failures_before_success:
            msg = "temporary TCP failure"
            raise BaostockNetworkError(msg)
        return baostock_response()


def queued_baostock_client(
    responses: list[BaostockResponse],
) -> tuple[BaostockAioTcpClient, QueuedBaostockSender]:
    sender = QueuedBaostockSender(responses)
    return (
        BaostockAioTcpClient(
            config=client_config(),
            retry_policy=ExponentialBackoffPolicy(jitter=False, base_delay=0),
            max_attempts=1,
            send_once=sender,
        ),
        sender,
    )


def retrying_baostock_client(
    failures_before_success: int,
) -> tuple[BaostockAioTcpClient, RetryingBaostockSender]:
    sender = RetryingBaostockSender(failures_before_success)
    return (
        BaostockAioTcpClient(
            config=client_config(),
            retry_policy=ExponentialBackoffPolicy(jitter=False, base_delay=0),
            max_attempts=3,
            send_once=sender,
        ),
        sender,
    )
