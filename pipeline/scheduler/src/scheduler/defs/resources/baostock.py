from __future__ import annotations

from contextlib import AbstractAsyncContextManager

import dagster as dg

from scheduler.defs.baostock.client import BaostockAioTcpClient
from scheduler.defs.baostock.services import BaostockClientProtocol
from scheduler.defs.config.env import (
    BAOSTOCK_CONNECT_TIMEOUT_SECONDS,
    BAOSTOCK_HOST,
    BAOSTOCK_LOGIN_TIMEOUT_SECONDS,
    BAOSTOCK_MAX_REQUEST_ATTEMPTS,
    BAOSTOCK_PASSWORD,
    BAOSTOCK_PORT,
    BAOSTOCK_REQUEST_TIMEOUT_SECONDS,
    BAOSTOCK_USERNAME,
)
from scheduler.defs.config.models import BaostockClientConfig


class BaostockClientFactoryResource(dg.ConfigurableResource):
    host: str = BAOSTOCK_HOST
    port: int = BAOSTOCK_PORT
    username: str = BAOSTOCK_USERNAME
    password: str = BAOSTOCK_PASSWORD
    max_connections: int = 1
    connect_timeout_seconds: int = BAOSTOCK_CONNECT_TIMEOUT_SECONDS
    request_timeout_seconds: int = BAOSTOCK_REQUEST_TIMEOUT_SECONDS
    login_timeout_seconds: int = BAOSTOCK_LOGIN_TIMEOUT_SECONDS
    max_request_attempts: int = BAOSTOCK_MAX_REQUEST_ATTEMPTS

    def config(self, *, max_connections: int | None = None) -> BaostockClientConfig:
        return BaostockClientConfig(
            host=self.host,
            port=self.port,
            username=self.username,
            password=self.password,
            max_connections=max_connections or self.max_connections,
            connect_timeout_seconds=self.connect_timeout_seconds,
            request_timeout_seconds=self.request_timeout_seconds,
            login_timeout_seconds=self.login_timeout_seconds,
            max_request_attempts=self.max_request_attempts,
        )

    def client(
        self,
        *,
        max_connections: int | None = None,
    ) -> AbstractAsyncContextManager[BaostockClientProtocol]:
        return BaostockAioTcpClient(config=self.config(max_connections=max_connections))
