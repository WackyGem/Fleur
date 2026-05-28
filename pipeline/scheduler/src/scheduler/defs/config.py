from __future__ import annotations

from dataclasses import dataclass

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
            endpoint=RUSTFS_ENDPOINT.get_value(),
            bucket=RUSTFS_BUCKET.get_value(),
            access_key=RUSTFS_ACCESS_KEY.get_value(),
            secret_key=RUSTFS_SECRET_KEY.get_value(),
        )


@dataclass(frozen=True)
class BaostockClientConfig:
    host: str
    port: int
    username: str
    password: str
    max_connections: int = 30

    @classmethod
    def from_env(cls) -> BaostockClientConfig:
        return cls(
            host=BAOSTOCK_HOST.get_value(),
            port=BAOSTOCK_PORT.get_value(),
            username=BAOSTOCK_USERNAME.get_value(),
            password=BAOSTOCK_PASSWORD.get_value(),
        )


@dataclass(frozen=True)
class PipelineDatabaseConfig:
    url: str

    @classmethod
    def from_env(cls) -> PipelineDatabaseConfig:
        return cls(url=PIPELINE_DATABASE_URL.get_value())


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
            base_url=JIUYAN_OCR_BASE_URL.get_value(),
            model_name=JIUYAN_OCR_MODEL_NAME.get_value(),
            timeout_seconds=JIUYAN_OCR_TIMEOUT_SECONDS.get_value(),
            max_retries=JIUYAN_OCR_MAX_RETRIES.get_value(),
            max_concurrent_requests=JIUYAN_OCR_MAX_CONCURRENT_REQUESTS.get_value(),
            stale_running_seconds=JIUYAN_OCR_STALE_RUNNING_SECONDS.get_value(),
        )
