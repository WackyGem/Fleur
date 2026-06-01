from __future__ import annotations

from typing import cast

import dagster as dg

from scheduler.defs.clickhouse.protocols import ClickHouseClientProtocol
from scheduler.defs.config import env
from scheduler.defs.config.models import ClickHouseConfig, parse_bool_env


class ClickHouseResource(dg.ConfigurableResource):
    host: str = env.CLICKHOUSE_HOST
    port: int = env.CLICKHOUSE_PORT
    database: str = env.CLICKHOUSE_DATABASE
    username: str = env.CLICKHOUSE_USER
    password: str = env.CLICKHOUSE_PASSWORD
    secure: str = env.CLICKHOUSE_SECURE
    connect_timeout_seconds: int = env.CLICKHOUSE_CONNECT_TIMEOUT_SECONDS
    query_timeout_seconds: int = env.CLICKHOUSE_QUERY_TIMEOUT_SECONDS

    def config(self) -> ClickHouseConfig:
        return ClickHouseConfig(
            host=self.host,
            port=self.port,
            database=self.database,
            username=self.username,
            password=self.password,
            secure=parse_bool_env(self.secure, field_name="CLICKHOUSE_SECURE"),
            connect_timeout_seconds=self.connect_timeout_seconds,
            query_timeout_seconds=self.query_timeout_seconds,
        )

    def client(self) -> ClickHouseClientProtocol:
        import clickhouse_connect

        config = self.config()
        return cast(
            "ClickHouseClientProtocol",
            clickhouse_connect.get_client(
                host=config.host,
                port=config.port,
                username=config.username,
                password=config.password,
                secure=config.secure,
                connect_timeout=config.connect_timeout_seconds,
                send_receive_timeout=config.query_timeout_seconds,
            ),
        )
