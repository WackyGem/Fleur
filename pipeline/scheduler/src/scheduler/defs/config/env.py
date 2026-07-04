from __future__ import annotations

import os

import dagster as dg

DEFAULT_LOCAL_RUSTFS_API_PORT = 34050
DEFAULT_LOCAL_CLICKHOUSE_HTTP_PORT = 34052


def required_env_str(name: str) -> str:
    value = dg.EnvVar(name).get_value()
    if value is None:
        msg = f"Environment variable {name} is required"
        raise RuntimeError(msg)
    return value


def required_env_int(name: str) -> int:
    value = dg.EnvVar.int(name).get_value()
    if isinstance(value, bool) or value is None:
        msg = f"Environment variable {name} is required"
        raise RuntimeError(msg)
    return int(value)


def env_str_or_default(name: str, default: str) -> str:
    value = os.environ.get(name)
    if value is None or not value.strip():
        return default
    return value.strip()


def env_int_or_default(name: str, default: int) -> int:
    value = os.environ.get(name)
    if value is None or not value.strip():
        return default
    stripped = value.strip()
    if stripped.isdecimal():
        return int(stripped)
    msg = f"Environment variable {name} must be an integer"
    raise RuntimeError(msg)


def optional_env_int(name: str, default: int) -> int:
    value = dg.EnvVar.int(name).get_value()
    if isinstance(value, bool):
        msg = f"Environment variable {name} must be an integer"
        raise RuntimeError(msg)
    if value is None:
        return default
    return int(value)


RUSTFS_API_PORT = env_int_or_default("RUSTFS_API_PORT", DEFAULT_LOCAL_RUSTFS_API_PORT)
RUSTFS_ENDPOINT = env_str_or_default(
    "RUSTFS_ENDPOINT",
    f"http://127.0.0.1:{RUSTFS_API_PORT}",
)
RUSTFS_BUCKET = dg.EnvVar("RUSTFS_BUCKET")
RUSTFS_ACCESS_KEY = dg.EnvVar("RUSTFS_ACCESS_KEY")
RUSTFS_SECRET_KEY = dg.EnvVar("RUSTFS_SECRET_KEY")
RUSTFS_REGION_NAME = "us-east-1"
CLICKHOUSE_S3_ENDPOINT = env_str_or_default("CLICKHOUSE_S3_ENDPOINT", "http://rustfs:9000")

BAOSTOCK_HOST = dg.EnvVar("BAOSTOCK_HOST")
BAOSTOCK_PORT = dg.EnvVar.int("BAOSTOCK_PORT")
BAOSTOCK_USERNAME = dg.EnvVar("BAOSTOCK_USERNAME")
BAOSTOCK_PASSWORD = dg.EnvVar("BAOSTOCK_PASSWORD")
BAOSTOCK_CONNECT_TIMEOUT_SECONDS = optional_env_int(
    "BAOSTOCK_CONNECT_TIMEOUT_SECONDS",
    15,
)
BAOSTOCK_REQUEST_TIMEOUT_SECONDS = optional_env_int(
    "BAOSTOCK_REQUEST_TIMEOUT_SECONDS",
    20,
)
BAOSTOCK_LOGIN_TIMEOUT_SECONDS = optional_env_int(
    "BAOSTOCK_LOGIN_TIMEOUT_SECONDS",
    15,
)
BAOSTOCK_MAX_REQUEST_ATTEMPTS = optional_env_int(
    "BAOSTOCK_MAX_REQUEST_ATTEMPTS",
    4,
)

CLICKHOUSE_HTTP_PORT = env_int_or_default(
    "CLICKHOUSE_HTTP_PORT",
    DEFAULT_LOCAL_CLICKHOUSE_HTTP_PORT,
)
CLICKHOUSE_HOST = env_str_or_default("CLICKHOUSE_HOST", "127.0.0.1")
CLICKHOUSE_PORT = env_int_or_default("CLICKHOUSE_PORT", CLICKHOUSE_HTTP_PORT)
CLICKHOUSE_DATABASE = env_str_or_default("CLICKHOUSE_DATABASE", "raw")
CLICKHOUSE_USER = dg.EnvVar("CLICKHOUSE_USER")
CLICKHOUSE_PASSWORD = dg.EnvVar("CLICKHOUSE_PASSWORD")
CLICKHOUSE_SECURE = env_str_or_default("CLICKHOUSE_SECURE", "false")
CLICKHOUSE_CONNECT_TIMEOUT_SECONDS = env_int_or_default("CLICKHOUSE_CONNECT_TIMEOUT_SECONDS", 10)
CLICKHOUSE_QUERY_TIMEOUT_SECONDS = env_int_or_default("CLICKHOUSE_QUERY_TIMEOUT_SECONDS", 300)

JIUYAN_COOKIE = dg.EnvVar("JIUYAN_COOKIE")
PIPELINE_DATABASE_URL = dg.EnvVar("PIPELINE_DATABASE_URL")
JIUYAN_OCR_BASE_URL = dg.EnvVar("JIUYAN_OCR_BASE_URL")
JIUYAN_OCR_MODEL_NAME = dg.EnvVar("JIUYAN_OCR_MODEL_NAME")
JIUYAN_OCR_TIMEOUT_SECONDS = dg.EnvVar.int("JIUYAN_OCR_TIMEOUT_SECONDS")
JIUYAN_OCR_MAX_RETRIES = dg.EnvVar.int("JIUYAN_OCR_MAX_RETRIES")
JIUYAN_OCR_MAX_CONCURRENT_REQUESTS = dg.EnvVar.int("JIUYAN_OCR_MAX_CONCURRENT_REQUESTS")
JIUYAN_OCR_STALE_RUNNING_SECONDS = dg.EnvVar.int("JIUYAN_OCR_STALE_RUNNING_SECONDS")

SLACK_BOT_TOKEN = dg.EnvVar("SLACK_BOT_TOKEN")
SLACK_CHANNEL_ID = dg.EnvVar("SLACK_CHANNEL_ID")
SLACK_HTTP_PROXY = dg.EnvVar("SLACK_HTTP_PROXY")
DAGSTER_WEBSERVER_BASE_URL = dg.EnvVar("DAGSTER_WEBSERVER_BASE_URL")
DAGSTER_CODE_LOCATION_NAME = dg.EnvVar("DAGSTER_CODE_LOCATION_NAME")
