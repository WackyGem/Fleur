from __future__ import annotations

import dagster as dg

from scheduler.defs.config.env import (
    CLICKHOUSE_S3_ENDPOINT,
    RUSTFS_ACCESS_KEY,
    RUSTFS_BUCKET,
    RUSTFS_ENDPOINT,
    RUSTFS_REGION_NAME,
    RUSTFS_SECRET_KEY,
)
from scheduler.defs.config.models import S3Config
from scheduler.defs.sources.jiuyan.image_object_store import ImageObjectStore


class S3SettingsResource(dg.ConfigurableResource):
    endpoint: str = RUSTFS_ENDPOINT
    bucket: str = RUSTFS_BUCKET
    access_key: str = RUSTFS_ACCESS_KEY
    secret_key: str = RUSTFS_SECRET_KEY
    region_name: str = RUSTFS_REGION_NAME
    clickhouse_endpoint: str = CLICKHOUSE_S3_ENDPOINT

    def config(self) -> S3Config:
        return S3Config(
            endpoint=self.endpoint,
            bucket=self.bucket,
            access_key=self.access_key,
            secret_key=self.secret_key,
            region_name=self.region_name,
        )


class ImageObjectStoreResource(S3SettingsResource):
    def image_object_store(self) -> ImageObjectStore:
        return ImageObjectStore.from_s3_config(self.config())
