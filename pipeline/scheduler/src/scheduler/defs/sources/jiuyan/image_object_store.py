from __future__ import annotations

from dataclasses import dataclass

import pyarrow as pa

from scheduler.defs.config.models import S3Config
from scheduler.defs.sources.jiuyan.image_urls import image_s3_key
from scheduler.defs.sources.jiuyan.ocr_schema import ocr_result_base_dir
from scheduler.defs.storage.object_store import ObjectStore


@dataclass(frozen=True)
class ImageObjectStore:
    object_store: ObjectStore

    @classmethod
    def from_s3_config(cls, config: S3Config) -> ImageObjectStore:
        return cls(object_store=ObjectStore.from_s3_config(config))

    def write_downloaded_image(self, image_filename: str, image_bytes: bytes) -> str:
        return self.object_store.write_bytes(image_s3_key(image_filename), image_bytes)

    def read_image_bytes(self, image_key: str) -> bytes:
        return self.object_store.read_bytes(image_key)

    def write_ocr_result_table(self, image_filename: str, table: pa.Table) -> str:
        return self.object_store.write_table(ocr_result_base_dir("", image_filename), table)
