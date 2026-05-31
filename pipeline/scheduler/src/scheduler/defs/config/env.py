from __future__ import annotations

import dagster as dg

RUSTFS_ENDPOINT = dg.EnvVar("RUSTFS_ENDPOINT")
RUSTFS_BUCKET = dg.EnvVar("RUSTFS_BUCKET")
RUSTFS_ACCESS_KEY = dg.EnvVar("RUSTFS_ACCESS_KEY")
RUSTFS_SECRET_KEY = dg.EnvVar("RUSTFS_SECRET_KEY")
RUSTFS_REGION_NAME = "us-east-1"

BAOSTOCK_HOST = dg.EnvVar("BAOSTOCK_HOST")
BAOSTOCK_PORT = dg.EnvVar.int("BAOSTOCK_PORT")
BAOSTOCK_USERNAME = dg.EnvVar("BAOSTOCK_USERNAME")
BAOSTOCK_PASSWORD = dg.EnvVar("BAOSTOCK_PASSWORD")

JIUYAN_TOKEN = dg.EnvVar("JIUYAN_TOKEN")
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
