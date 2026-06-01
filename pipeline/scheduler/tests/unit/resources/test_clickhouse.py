from __future__ import annotations

import pytest
from scheduler.defs.config.models import ClickHouseConfig, parse_bool_env


def test_clickhouse_config_repr_does_not_expose_password() -> None:
    config = ClickHouseConfig(
        host="127.0.0.1",
        port=8123,
        database="raw",
        username="user",
        password="secret",
        secure=False,
        connect_timeout_seconds=10,
        query_timeout_seconds=300,
    )

    assert "secret" not in repr(config)


@pytest.mark.parametrize("value", ["true", "1", "YES", "on"])
def test_parse_bool_env_accepts_true_values(value: str) -> None:
    assert parse_bool_env(value, field_name="CLICKHOUSE_SECURE") is True


@pytest.mark.parametrize("value", ["false", "0", "NO", "off"])
def test_parse_bool_env_accepts_false_values(value: str) -> None:
    assert parse_bool_env(value, field_name="CLICKHOUSE_SECURE") is False


def test_parse_bool_env_rejects_invalid_value() -> None:
    with pytest.raises(RuntimeError, match="must be a boolean string"):
        parse_bool_env("maybe", field_name="CLICKHOUSE_SECURE")
