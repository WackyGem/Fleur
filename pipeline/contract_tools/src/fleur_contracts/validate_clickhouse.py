from __future__ import annotations

import os
from pathlib import Path
from typing import Any

import clickhouse_connect

from fleur_contracts.clickhouse_types import effective_clickhouse_type
from fleur_contracts.env import (
    load_repo_dotenv_if_present,
    local_clickhouse_host,
    local_clickhouse_port,
)
from fleur_contracts.loader import DEFAULT_CONTRACT_ROOT, load_registry


def validate_available_clickhouse(contract_root: Path = DEFAULT_CONTRACT_ROOT) -> int:
    load_repo_dotenv_if_present()
    registry = load_registry(contract_root)
    client = _build_client_from_env()
    try:
        checked = 0
        skipped = 0
        for dataset in registry.datasets:
            if dataset.clickhouse_raw is None:
                continue
            rows = client.query(
                """
                SELECT name, type
                FROM system.columns
                WHERE database = {database:String}
                  AND table = {table:String}
                ORDER BY position
                """,
                parameters={
                    "database": dataset.clickhouse_raw.database,
                    "table": dataset.clickhouse_raw.table,
                },
            ).result_rows
            if not rows:
                skipped += 1
                print(
                    "Skipping missing ClickHouse table "
                    f"{dataset.clickhouse_raw.database}.{dataset.clickhouse_raw.table}"
                )
                continue

            actual = [(str(row[0]), str(row[1])) for row in rows]
            expected = [
                (
                    field.name,
                    effective_clickhouse_type(field.type, nullable=field.nullable),
                )
                for field in dataset.clickhouse_raw.fields
            ]
            if dataset.clickhouse_raw.partition_strategy == "year":
                expected.append(("year", "UInt16"))
            if actual != expected:
                msg = (
                    f"ClickHouse schema mismatch for {dataset.dataset}: "
                    f"expected {len(expected)} columns, got {len(actual)}"
                )
                raise RuntimeError(msg)
            checked += 1
    finally:
        client.close()

    print(
        f"ClickHouse schema validation checked {checked} tables, skipped {skipped} missing tables."
    )
    return checked


def _build_client_from_env() -> Any:
    return build_client_from_env(database="default")


def build_client_from_env(*, database: str | None = None) -> Any:
    host = local_clickhouse_host()
    port = local_clickhouse_port()
    username = _required_env("CLICKHOUSE_USER")
    password = _required_env("CLICKHOUSE_PASSWORD")
    database_name = (
        database
        or os.environ.get("CLICKHOUSE_DATABASE")
        or os.environ.get("CLICKHOUSE_DB")
        or "default"
    )
    secure = os.environ.get("CLICKHOUSE_SECURE", "").lower() in {"1", "true", "yes"}
    connect_timeout = int(os.environ.get("CLICKHOUSE_CONNECT_TIMEOUT_SECONDS", "10"))
    send_receive_timeout = int(os.environ.get("CLICKHOUSE_QUERY_TIMEOUT_SECONDS", "300"))
    return clickhouse_connect.get_client(
        host=host,
        port=port,
        username=username,
        password=password,
        database=database_name,
        secure=secure,
        connect_timeout=connect_timeout,
        send_receive_timeout=send_receive_timeout,
    )


def _required_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        msg = f"{name} is required for ClickHouse schema validation"
        raise RuntimeError(msg)
    return value
