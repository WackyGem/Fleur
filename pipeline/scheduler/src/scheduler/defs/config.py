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
