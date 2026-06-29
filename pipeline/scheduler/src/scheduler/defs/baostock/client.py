from __future__ import annotations

import asyncio
import time
from contextlib import suppress
from dataclasses import dataclass, field
from datetime import date
from types import TracebackType
from typing import Protocol, Self

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
from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY, ExponentialBackoffPolicy
from scheduler.defs.config.models import BaostockClientConfig

LOGIN_TTL_SECONDS = 55 * 60
NO_LOGIN_ERROR_CODE = "10001001"
RESPONSE_STREAM_LIMIT_BYTES = 64 * 1024 * 1024


class BaostockSendOnceProtocol(Protocol):
    async def __call__(
        self,
        payload: bytes,
        *,
        timeout_seconds: float,
    ) -> BaostockResponse: ...


@dataclass(eq=False)
class BaostockTcpConnection:
    reader: asyncio.StreamReader
    writer: asyncio.StreamWriter
    lock: asyncio.Lock = field(default_factory=asyncio.Lock)
    reusable: bool = True
    logged_in: bool = False
    login_expires_at: float | None = None

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
        max_attempts: int | None = None,
        send_once: BaostockSendOnceProtocol | None = None,
    ) -> None:
        base_config = config or BaostockClientConfig.from_env()
        if max_connections is not None:
            base_config = BaostockClientConfig(
                host=base_config.host,
                port=base_config.port,
                username=base_config.username,
                password=base_config.password,
                max_connections=max_connections,
                connect_timeout_seconds=base_config.connect_timeout_seconds,
                request_timeout_seconds=base_config.request_timeout_seconds,
                login_timeout_seconds=base_config.login_timeout_seconds,
                max_request_attempts=base_config.max_request_attempts,
            )
        if base_config.max_connections < 1:
            msg = "max_connections must be positive"
            raise ValueError(msg)
        if base_config.connect_timeout_seconds <= 0:
            msg = "connect_timeout_seconds must be positive"
            raise ValueError(msg)
        if base_config.request_timeout_seconds <= 0:
            msg = "request_timeout_seconds must be positive"
            raise ValueError(msg)
        if base_config.login_timeout_seconds <= 0:
            msg = "login_timeout_seconds must be positive"
            raise ValueError(msg)
        if base_config.max_request_attempts < 1:
            msg = "max_request_attempts must be positive"
            raise ValueError(msg)
        if max_attempts is not None and max_attempts < 1:
            msg = "max_attempts must be positive"
            raise ValueError(msg)

        self._config = base_config
        self._retry_policy = retry_policy
        self._max_attempts = max_attempts or base_config.max_request_attempts
        self._send_once_override = send_once
        self._semaphore = asyncio.Semaphore(base_config.max_connections)
        self._idle_connections: asyncio.LifoQueue[BaostockTcpConnection] = asyncio.LifoQueue()
        self._connections: set[BaostockTcpConnection] = set()
        self._pool_lock = asyncio.Lock()
        self._start_lock = asyncio.Lock()
        self._override_login_lock = asyncio.Lock()
        self._override_logged_in = False
        self._override_login_expires_at: float | None = None
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
            if self._send_once_override is not None:
                await self._ensure_override_logged_in(force=True)
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
        self._override_logged_in = False
        self._override_login_expires_at = None

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
                timeout_seconds=self._config.request_timeout_seconds,
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

    async def _request_api(
        self,
        request_code: str,
        api_name: str,
        params: list[str],
        page: int,
        page_size: int,
        timeout_seconds: float,
    ) -> BaostockResponse:
        await self._ensure_started()
        payload = encode_request(
            request_code=request_code,
            api_name=api_name,
            user_id=self._config.username,
            params=params,
            page=page,
            page_size=page_size,
        )
        if self._send_once_override is not None:
            response = await self._request_api_with_override(payload, timeout_seconds)
        else:
            response = await self._send_api_with_retries(payload, timeout_seconds=timeout_seconds)

        if response.error_code != "0":
            raise BaostockResponseError(
                response.error_code,
                response.error_message,
                response.api_name or api_name,
                params,
            )
        return response

    async def _request_api_with_override(
        self,
        payload: bytes,
        timeout_seconds: float,
    ) -> BaostockResponse:
        await self._ensure_override_logged_in()
        response = await self._send_override_with_retries(
            payload,
            timeout_seconds=timeout_seconds,
        )
        if response.error_code != NO_LOGIN_ERROR_CODE:
            return response

        observed_login_expires_at = self._override_login_expires_at
        await self._ensure_override_logged_in(
            force=True,
            observed_login_expires_at=observed_login_expires_at,
        )
        response = await self._send_override_with_retries(
            payload,
            timeout_seconds=timeout_seconds,
        )
        if response.error_code == NO_LOGIN_ERROR_CODE:
            msg = "BaoStock request still reported not logged in after login refresh"
            raise BaostockAuthenticationError(msg)
        return response

    async def _ensure_override_logged_in(
        self,
        *,
        force: bool = False,
        observed_login_expires_at: float | None = None,
    ) -> None:
        now = time.monotonic()
        if (
            not force
            and self._override_logged_in
            and self._override_login_expires_at is not None
            and now < self._override_login_expires_at
        ):
            return

        async with self._override_login_lock:
            now = time.monotonic()
            if (
                not force
                and self._override_logged_in
                and self._override_login_expires_at is not None
                and now < self._override_login_expires_at
            ):
                return
            if (
                force
                and self._override_logged_in
                and self._override_login_expires_at is not None
                and now < self._override_login_expires_at
                and self._override_login_expires_at != observed_login_expires_at
            ):
                return

            response = await self._send_override_with_retries(
                self._login_payload(),
                timeout_seconds=self._config.login_timeout_seconds,
            )
            if response.error_code != "0":
                self._override_logged_in = False
                self._override_login_expires_at = None
                msg = f"BaoStock login failed with {response.error_code}: {response.error_message}"
                raise BaostockAuthenticationError(msg)

            self._override_logged_in = True
            self._override_login_expires_at = time.monotonic() + LOGIN_TTL_SECONDS

    async def _send_override_with_retries(
        self,
        payload: bytes,
        *,
        timeout_seconds: float,
    ) -> BaostockResponse:
        if self._send_once_override is None:
            msg = "BaoStock send override is not configured"
            raise RuntimeError(msg)

        last_error: BaostockNetworkError | None = None
        retry_delays = self._retry_policy.delays(self._max_attempts)
        for attempt_index in range(self._max_attempts):
            try:
                return await self._send_once_override(
                    payload,
                    timeout_seconds=timeout_seconds,
                )
            except BaostockNetworkError as error:
                last_error = error
                if attempt_index >= len(retry_delays):
                    break
                await asyncio.sleep(retry_delays[attempt_index])

        msg = f"BaoStock TCP request failed after {self._max_attempts} attempts"
        raise BaostockNetworkError(msg) from last_error

    async def _send_api_with_retries(
        self,
        payload: bytes,
        *,
        timeout_seconds: float,
    ) -> BaostockResponse:
        last_error: BaostockNetworkError | None = None
        retry_delays = self._retry_policy.delays(self._max_attempts)
        for attempt_index in range(self._max_attempts):
            connection: BaostockTcpConnection | None = None
            try:
                connection = await self._borrow_connection()
                return await self._request_api_on_connection(
                    connection,
                    payload,
                    timeout_seconds,
                )
            except BaostockNetworkError as error:
                last_error = error
                if attempt_index >= len(retry_delays):
                    break
                await asyncio.sleep(retry_delays[attempt_index])
            finally:
                if connection is not None:
                    await self._return_connection(connection)

        msg = f"BaoStock TCP request failed after {self._max_attempts} attempts"
        raise BaostockNetworkError(msg) from last_error

    async def _request_api_on_connection(
        self,
        connection: BaostockTcpConnection,
        payload: bytes,
        timeout_seconds: float,
    ) -> BaostockResponse:
        await self._ensure_connection_logged_in(connection)
        response = await connection.request(payload, timeout_seconds)
        if response.error_code != NO_LOGIN_ERROR_CODE:
            return response

        connection.logged_in = False
        connection.login_expires_at = None
        await self._ensure_connection_logged_in(connection, force=True)
        response = await connection.request(payload, timeout_seconds)
        if response.error_code == NO_LOGIN_ERROR_CODE:
            await connection.close()
            msg = "BaoStock request still reported not logged in after login refresh"
            raise BaostockAuthenticationError(msg)
        return response

    async def _ensure_connection_logged_in(
        self,
        connection: BaostockTcpConnection,
        *,
        force: bool = False,
    ) -> None:
        now = time.monotonic()
        if (
            not force
            and connection.logged_in
            and connection.login_expires_at is not None
            and now < connection.login_expires_at
        ):
            return

        response = await connection.request(
            self._login_payload(),
            timeout_seconds=self._config.login_timeout_seconds,
        )
        if response.error_code != "0":
            connection.logged_in = False
            connection.login_expires_at = None
            await connection.close()
            msg = f"BaoStock login failed with {response.error_code}: {response.error_message}"
            raise BaostockAuthenticationError(msg)

        connection.logged_in = True
        connection.login_expires_at = time.monotonic() + LOGIN_TTL_SECONDS

    def _login_payload(self) -> bytes:
        return encode_request(
            request_code="00",
            api_name="login",
            user_id=self._config.username,
            params=[self._config.password, "0"],
        )

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
                timeout=self._config.connect_timeout_seconds,
            )
        except (TimeoutError, OSError) as error:
            msg = (
                f"Failed to connect to BaoStock TCP server {self._config.host}:{self._config.port}"
            )
            raise BaostockNetworkError(msg) from error
        return BaostockTcpConnection(reader=reader, writer=writer)
