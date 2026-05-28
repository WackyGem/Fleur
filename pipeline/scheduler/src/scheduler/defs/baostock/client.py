from __future__ import annotations

import asyncio
import time
from contextlib import suppress
from dataclasses import dataclass, field
from datetime import date
from types import TracebackType
from typing import Self

from scheduler.defs.baostock.protocol import (
    DEFAULT_PAGE_SIZE,
    MESSAGE_END,
    BaostockAuthenticationError,
    BaostockNetworkError,
    BaostockProtocolError,
    BaostockResponse,
    BaostockResponseError,
    aggregate_responses,
    decode_response,
    encode_request,
)
from scheduler.defs.baostock.schemas import K_HISTORY_DAILY_FIELD_PARAM
from scheduler.defs.config import BaostockClientConfig
from scheduler.defs.util import DEFAULT_RETRY_POLICY, ExponentialBackoffPolicy

CONNECT_TIMEOUT_SECONDS = 5
REQUEST_TIMEOUT_SECONDS = 30
LOGIN_TIMEOUT_SECONDS = 15
MAX_REQUEST_ATTEMPTS = 4
LOGIN_TTL_SECONDS = 55 * 60
NO_LOGIN_ERROR_CODE = "10001001"
RESPONSE_STREAM_LIMIT_BYTES = 64 * 1024 * 1024


@dataclass(eq=False)
class BaostockTcpConnection:
    reader: asyncio.StreamReader
    writer: asyncio.StreamWriter
    lock: asyncio.Lock = field(default_factory=asyncio.Lock)
    reusable: bool = True

    async def request(self, payload: bytes, timeout_seconds: float) -> BaostockResponse:
        async with self.lock:
            try:
                self.writer.write(payload)
                await asyncio.wait_for(self.writer.drain(), timeout=timeout_seconds)
                response_bytes = await asyncio.wait_for(
                    self.reader.readuntil(MESSAGE_END),
                    timeout=timeout_seconds,
                )
                return decode_response(response_bytes)
            except (
                TimeoutError,
                OSError,
                asyncio.IncompleteReadError,
                asyncio.LimitOverrunError,
            ) as error:
                self.reusable = False
                await self.close()
                msg = "BaoStock TCP request failed"
                raise BaostockNetworkError(msg) from error
            except BaostockProtocolError:
                self.reusable = False
                await self.close()
                raise

    async def close(self) -> None:
        self.reusable = False
        self.writer.close()
        with suppress(OSError):
            await self.writer.wait_closed()


class BaostockAioTcpClient:
    def __init__(
        self,
        config: BaostockClientConfig | None = None,
        *,
        max_connections: int | None = None,
        retry_policy: ExponentialBackoffPolicy = DEFAULT_RETRY_POLICY,
        max_attempts: int = MAX_REQUEST_ATTEMPTS,
    ) -> None:
        base_config = config or BaostockClientConfig.from_env()
        if max_connections is not None:
            base_config = BaostockClientConfig(
                host=base_config.host,
                port=base_config.port,
                username=base_config.username,
                password=base_config.password,
                max_connections=max_connections,
            )
        if base_config.max_connections < 1:
            msg = "max_connections must be positive"
            raise ValueError(msg)

        self._config = base_config
        self._retry_policy = retry_policy
        self._max_attempts = max_attempts
        self._semaphore = asyncio.Semaphore(base_config.max_connections)
        self._idle_connections: asyncio.LifoQueue[BaostockTcpConnection] = asyncio.LifoQueue()
        self._connections: set[BaostockTcpConnection] = set()
        self._pool_lock = asyncio.Lock()
        self._start_lock = asyncio.Lock()
        self._login_lock = asyncio.Lock()
        self._logged_in = False
        self._login_expires_at: float | None = None
        self._started = False
        self._closed = False

    async def __aenter__(self) -> Self:
        await self.start()
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        await self.close()

    async def start(self) -> None:
        if self._closed:
            msg = "BaoStock client has already been closed"
            raise RuntimeError(msg)
        if self._started:
            return
        async with self._start_lock:
            if self._started:
                return
            await self._ensure_logged_in(force=True)
            self._started = True

    async def close(self) -> None:
        self._closed = True
        connections = list(self._connections)
        while not self._idle_connections.empty():
            try:
                connection = self._idle_connections.get_nowait()
            except asyncio.QueueEmpty:
                break
            connections.append(connection)
        for connection in set(connections):
            await connection.close()
        self._connections.clear()
        self._logged_in = False
        self._login_expires_at = None

    async def query_stock_basic(
        self,
        code: str = "",
        code_name: str = "",
    ) -> BaostockResponse:
        return await self._query_paginated(
            request_code="45",
            api_name="query_stock_basic",
            params=[code, code_name],
        )

    async def query_history_k_data_plus_daily(
        self,
        code: str,
        start_date: date,
        end_date: date,
    ) -> BaostockResponse:
        if start_date > end_date:
            msg = "start_date must be less than or equal to end_date"
            raise ValueError(msg)

        return await self._query_paginated(
            request_code="95",
            api_name="query_history_k_data_plus",
            params=[
                code,
                K_HISTORY_DAILY_FIELD_PARAM,
                start_date.isoformat(),
                end_date.isoformat(),
                "d",
                "3",
            ],
        )

    async def _query_paginated(
        self,
        request_code: str,
        api_name: str,
        params: list[str],
        *,
        page_size: int = DEFAULT_PAGE_SIZE,
    ) -> BaostockResponse:
        await self._ensure_started()
        responses: list[BaostockResponse] = []
        page = 1
        while True:
            response = await self._request_api(
                request_code=request_code,
                api_name=api_name,
                params=params,
                page=page,
                page_size=page_size,
                timeout_seconds=REQUEST_TIMEOUT_SECONDS,
            )
            responses.append(response)
            if not response.has_next_page():
                break
            page += 1

        return aggregate_responses(responses)

    async def _ensure_started(self) -> None:
        if self._closed:
            msg = "BaoStock client has already been closed"
            raise RuntimeError(msg)
        if not self._started:
            await self.start()

    async def _ensure_logged_in(
        self,
        *,
        force: bool = False,
        observed_login_expires_at: float | None = None,
    ) -> None:
        now = time.monotonic()
        if (
            not force
            and self._logged_in
            and self._login_expires_at is not None
            and now < self._login_expires_at
        ):
            return

        async with self._login_lock:
            now = time.monotonic()
            if (
                not force
                and self._logged_in
                and self._login_expires_at is not None
                and now < self._login_expires_at
            ):
                return
            if (
                force
                and self._logged_in
                and self._login_expires_at is not None
                and now < self._login_expires_at
                and self._login_expires_at != observed_login_expires_at
            ):
                return

            payload = encode_request(
                request_code="00",
                api_name="login",
                user_id=self._config.username,
                params=[self._config.password, "0"],
            )
            response = await self._send_with_retries(
                payload,
                timeout_seconds=LOGIN_TIMEOUT_SECONDS,
            )
            if response.error_code != "0":
                self._logged_in = False
                self._login_expires_at = None
                msg = f"BaoStock login failed with {response.error_code}: {response.error_message}"
                raise BaostockAuthenticationError(msg)

            self._logged_in = True
            self._login_expires_at = time.monotonic() + LOGIN_TTL_SECONDS

    async def _request_api(
        self,
        request_code: str,
        api_name: str,
        params: list[str],
        page: int,
        page_size: int,
        timeout_seconds: float,
    ) -> BaostockResponse:
        await self._ensure_logged_in()
        payload = encode_request(
            request_code=request_code,
            api_name=api_name,
            user_id=self._config.username,
            params=params,
            page=page,
            page_size=page_size,
        )
        response = await self._send_with_retries(payload, timeout_seconds=timeout_seconds)
        if response.error_code == NO_LOGIN_ERROR_CODE:
            observed_login_expires_at = self._login_expires_at
            await self._ensure_logged_in(
                force=True,
                observed_login_expires_at=observed_login_expires_at,
            )
            response = await self._send_with_retries(payload, timeout_seconds=timeout_seconds)
            if response.error_code == NO_LOGIN_ERROR_CODE:
                msg = "BaoStock request still reported not logged in after login refresh"
                raise BaostockAuthenticationError(msg)

        if response.error_code != "0":
            raise BaostockResponseError(
                response.error_code,
                response.error_message,
                response.api_name or api_name,
                params,
            )
        return response

    async def _send_with_retries(
        self,
        payload: bytes,
        *,
        timeout_seconds: float,
    ) -> BaostockResponse:
        last_error: BaostockNetworkError | None = None
        retry_delays = self._retry_policy.delays(self._max_attempts)
        for attempt_index in range(self._max_attempts):
            try:
                return await self._send_once(payload, timeout_seconds=timeout_seconds)
            except BaostockNetworkError as error:
                last_error = error
                if attempt_index >= len(retry_delays):
                    break
                await asyncio.sleep(retry_delays[attempt_index])

        msg = f"BaoStock TCP request failed after {self._max_attempts} attempts"
        raise BaostockNetworkError(msg) from last_error

    async def _send_once(
        self,
        payload: bytes,
        *,
        timeout_seconds: float,
    ) -> BaostockResponse:
        connection = await self._borrow_connection()
        try:
            response = await connection.request(payload, timeout_seconds)
        finally:
            await self._return_connection(connection)
        return response

    async def _borrow_connection(self) -> BaostockTcpConnection:
        await self._semaphore.acquire()
        try:
            connection = await self._get_or_create_connection()
        except BaseException:
            self._semaphore.release()
            raise
        return connection

    async def _get_or_create_connection(self) -> BaostockTcpConnection:
        while True:
            try:
                connection = self._idle_connections.get_nowait()
            except asyncio.QueueEmpty:
                break
            if connection.reusable:
                return connection
            await self._discard_connection(connection)

        async with self._pool_lock:
            if len(self._connections) < self._config.max_connections:
                connection = await self._create_connection()
                self._connections.add(connection)
                return connection

        try:
            return self._idle_connections.get_nowait()
        except asyncio.QueueEmpty as error:
            msg = "BaoStock connection pool had no available connection after semaphore acquisition"
            raise BaostockNetworkError(msg) from error

    async def _return_connection(self, connection: BaostockTcpConnection) -> None:
        try:
            if connection.reusable and not self._closed:
                self._idle_connections.put_nowait(connection)
            else:
                await self._discard_connection(connection)
        finally:
            self._semaphore.release()

    async def _discard_connection(self, connection: BaostockTcpConnection) -> None:
        self._connections.discard(connection)
        await connection.close()

    async def _create_connection(self) -> BaostockTcpConnection:
        try:
            reader, writer = await asyncio.wait_for(
                asyncio.open_connection(
                    self._config.host,
                    self._config.port,
                    limit=RESPONSE_STREAM_LIMIT_BYTES,
                ),
                timeout=CONNECT_TIMEOUT_SECONDS,
            )
        except (TimeoutError, OSError) as error:
            msg = (
                f"Failed to connect to BaoStock TCP server {self._config.host}:{self._config.port}"
            )
            raise BaostockNetworkError(msg) from error
        return BaostockTcpConnection(reader=reader, writer=writer)
