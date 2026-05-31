from __future__ import annotations

from contextlib import AbstractAsyncContextManager

import dagster as dg

from scheduler.defs.baostock.client import BaostockAioTcpClient
from scheduler.defs.baostock.services import BaostockClientProtocol
from scheduler.defs.config.env import (
    BAOSTOCK_HOST,
    BAOSTOCK_PASSWORD,
    BAOSTOCK_PORT,
    BAOSTOCK_USERNAME,
)
from scheduler.defs.config.models import BaostockClientConfig


class BaostockClientFactoryResource(dg.ConfigurableResource):
    host: str = BAOSTOCK_HOST
    port: int = BAOSTOCK_PORT
    username: str = BAOSTOCK_USERNAME
    password: str = BAOSTOCK_PASSWORD
    max_connections: int = 30

    def config(self, *, max_connections: int | None = None) -> BaostockClientConfig:
        return BaostockClientConfig(
            host=self.host,
            port=self.port,
            username=self.username,
            password=self.password,
            max_connections=max_connections or self.max_connections,
        )

    def client(
        self,
        *,
        max_connections: int | None = None,
    ) -> AbstractAsyncContextManager[BaostockClientProtocol]:
        return BaostockAioTcpClient(config=self.config(max_connections=max_connections))
