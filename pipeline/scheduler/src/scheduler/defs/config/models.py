from __future__ import annotations

from dataclasses import dataclass, field

from scheduler.defs.config.env import (
    RUSTFS_REGION_NAME,
    optional_env_int,
    required_env_int,
    required_env_str,
)

DEFAULT_BAOSTOCK_CONNECT_TIMEOUT_SECONDS = 15
DEFAULT_BAOSTOCK_REQUEST_TIMEOUT_SECONDS = 20
DEFAULT_BAOSTOCK_LOGIN_TIMEOUT_SECONDS = 15
DEFAULT_BAOSTOCK_MAX_REQUEST_ATTEMPTS = 4


@dataclass(frozen=True)
class S3Config:
    endpoint: str
    bucket: str
    access_key: str
    secret_key: str
    region_name: str = RUSTFS_REGION_NAME

    @classmethod
    def from_env(cls) -> S3Config:
        return cls(
            endpoint=required_env_str("RUSTFS_ENDPOINT"),
            bucket=required_env_str("RUSTFS_BUCKET"),
            access_key=required_env_str("RUSTFS_ACCESS_KEY"),
            secret_key=required_env_str("RUSTFS_SECRET_KEY"),
        )


@dataclass(frozen=True)
class BaostockClientConfig:
    host: str
    port: int
    username: str
    password: str
    max_connections: int = 1
    connect_timeout_seconds: int = DEFAULT_BAOSTOCK_CONNECT_TIMEOUT_SECONDS
    request_timeout_seconds: int = DEFAULT_BAOSTOCK_REQUEST_TIMEOUT_SECONDS
    login_timeout_seconds: int = DEFAULT_BAOSTOCK_LOGIN_TIMEOUT_SECONDS
    max_request_attempts: int = DEFAULT_BAOSTOCK_MAX_REQUEST_ATTEMPTS

    @classmethod
    def from_env(cls) -> BaostockClientConfig:
        return cls(
            host=required_env_str("BAOSTOCK_HOST"),
            port=required_env_int("BAOSTOCK_PORT"),
            username=required_env_str("BAOSTOCK_USERNAME"),
            password=required_env_str("BAOSTOCK_PASSWORD"),
            connect_timeout_seconds=optional_env_int(
                "BAOSTOCK_CONNECT_TIMEOUT_SECONDS",
                DEFAULT_BAOSTOCK_CONNECT_TIMEOUT_SECONDS,
            ),
            request_timeout_seconds=optional_env_int(
                "BAOSTOCK_REQUEST_TIMEOUT_SECONDS",
                DEFAULT_BAOSTOCK_REQUEST_TIMEOUT_SECONDS,
            ),
            login_timeout_seconds=optional_env_int(
                "BAOSTOCK_LOGIN_TIMEOUT_SECONDS",
                DEFAULT_BAOSTOCK_LOGIN_TIMEOUT_SECONDS,
            ),
            max_request_attempts=optional_env_int(
                "BAOSTOCK_MAX_REQUEST_ATTEMPTS",
                DEFAULT_BAOSTOCK_MAX_REQUEST_ATTEMPTS,
            ),
        )


def parse_bool_env(value: str, *, field_name: str) -> bool:
    normalized = value.strip().lower()
    if normalized in {"1", "true", "t", "yes", "y", "on"}:
        return True
    if normalized in {"0", "false", "f", "no", "n", "off"}:
        return False

    msg = f"Environment variable {field_name} must be a boolean string"
    raise RuntimeError(msg)


@dataclass(frozen=True)
class ClickHouseConfig:
    host: str
    port: int
    database: str
    username: str
    password: str = field(repr=False)
    secure: bool
    connect_timeout_seconds: int
    query_timeout_seconds: int

    @classmethod
    def from_env(cls) -> ClickHouseConfig:
        return cls(
            host=required_env_str("CLICKHOUSE_HOST"),
            port=required_env_int("CLICKHOUSE_PORT"),
            database=required_env_str("CLICKHOUSE_DATABASE"),
            username=required_env_str("CLICKHOUSE_USER"),
            password=required_env_str("CLICKHOUSE_PASSWORD"),
            secure=parse_bool_env(
                required_env_str("CLICKHOUSE_SECURE"),
                field_name="CLICKHOUSE_SECURE",
            ),
            connect_timeout_seconds=required_env_int("CLICKHOUSE_CONNECT_TIMEOUT_SECONDS"),
            query_timeout_seconds=required_env_int("CLICKHOUSE_QUERY_TIMEOUT_SECONDS"),
        )


@dataclass(frozen=True)
class PipelineDatabaseConfig:
    url: str

    @classmethod
    def from_env(cls) -> PipelineDatabaseConfig:
        return cls(url=required_env_str("PIPELINE_DATABASE_URL"))


@dataclass(frozen=True)
class JiuyanOcrConfig:
    base_url: str
    model_name: str
    timeout_seconds: int
    max_retries: int
    max_concurrent_requests: int
    stale_running_seconds: int

    @classmethod
    def from_env(cls) -> JiuyanOcrConfig:
        return cls(
            base_url=required_env_str("JIUYAN_OCR_BASE_URL"),
            model_name=required_env_str("JIUYAN_OCR_MODEL_NAME"),
            timeout_seconds=required_env_int("JIUYAN_OCR_TIMEOUT_SECONDS"),
            max_retries=required_env_int("JIUYAN_OCR_MAX_RETRIES"),
            max_concurrent_requests=required_env_int("JIUYAN_OCR_MAX_CONCURRENT_REQUESTS"),
            stale_running_seconds=required_env_int("JIUYAN_OCR_STALE_RUNNING_SECONDS"),
        )
