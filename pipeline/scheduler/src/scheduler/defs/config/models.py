from __future__ import annotations

from dataclasses import dataclass

from scheduler.defs.config.env import RUSTFS_REGION_NAME, required_env_int, required_env_str


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
    max_connections: int = 30

    @classmethod
    def from_env(cls) -> BaostockClientConfig:
        return cls(
            host=required_env_str("BAOSTOCK_HOST"),
            port=required_env_int("BAOSTOCK_PORT"),
            username=required_env_str("BAOSTOCK_USERNAME"),
            password=required_env_str("BAOSTOCK_PASSWORD"),
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
