from __future__ import annotations

import hashlib
from dataclasses import dataclass

import pyarrow as pa
import pyarrow.fs as pafs

from scheduler.defs.config import S3Config
from scheduler.defs.http_resources.client import CHROME_USER_AGENT, AioHttpClient, HttpRequest
from scheduler.defs.http_resources.jiuyan_image_urls import image_s3_key
from scheduler.defs.http_resources.jiuyan_ocr_schema import ocr_result_base_dir
from scheduler.defs.util import (
    build_s3_filesystem,
    read_bytes_from_filesystem,
    write_bytes_to_filesystem,
    write_parquet_dataset,
)

IMAGE_DOWNLOAD_ACCEPT = "image/avif,image/webp,image/apng,image/*,*/*;q=0.8"


@dataclass(frozen=True)
class DownloadedImage:
    image_bytes: bytes
    mime_type: str
    sha256: str
    byte_count: int


def image_download_headers() -> dict[str, str]:
    return {
        "User-Agent": CHROME_USER_AGENT,
        "Accept": IMAGE_DOWNLOAD_ACCEPT,
    }


def normalize_image_content_type(content_type: str | None) -> str:
    if content_type is None:
        msg = "image response is missing Content-Type"
        raise ValueError(msg)
    mime_type = content_type.split(";", 1)[0].strip().lower()
    if not mime_type.startswith("image/"):
        msg = f"response is not an image: {content_type}"
        raise ValueError(msg)
    return mime_type


async def download_image_bytes(client: AioHttpClient, url: str) -> DownloadedImage:
    response = await client.request_bytes(
        HttpRequest(method="GET", url=url, headers=image_download_headers())
    )
    mime_type = normalize_image_content_type(response.headers.get("Content-Type"))
    image_bytes = response.body
    sha256 = hashlib.sha256(image_bytes).hexdigest()
    return DownloadedImage(
        image_bytes=image_bytes,
        mime_type=mime_type,
        sha256=sha256,
        byte_count=len(image_bytes),
    )


def build_s3_filesystem_for_config(config: S3Config) -> pafs.S3FileSystem:
    return build_s3_filesystem(config)


@dataclass(frozen=True)
class ImageObjectStore:
    filesystem: pafs.S3FileSystem
    bucket: str

    @classmethod
    def from_s3_config(cls, config: S3Config) -> ImageObjectStore:
        return cls(
            filesystem=build_s3_filesystem_for_config(config),
            bucket=config.bucket,
        )

    def write_downloaded_image(self, image_filename: str, image_bytes: bytes) -> str:
        return write_downloaded_image(
            self.filesystem,
            self.bucket,
            image_filename,
            image_bytes,
        )

    def read_image_bytes(self, image_key: str) -> bytes:
        return read_image_bytes(self.filesystem, self.bucket, image_key)

    def write_ocr_result_table(self, image_filename: str, table: pa.Table) -> str:
        return write_ocr_result_table(
            self.filesystem,
            self.bucket,
            image_filename,
            table,
        )


def write_downloaded_image(
    filesystem: pafs.FileSystem,
    bucket: str,
    image_filename: str,
    image_bytes: bytes,
) -> str:
    key = image_s3_key(image_filename)
    write_bytes_to_filesystem(filesystem, f"{bucket}/{key}", image_bytes)
    return key


def read_image_bytes(
    filesystem: pafs.FileSystem,
    bucket: str,
    image_key: str,
) -> bytes:
    return read_bytes_from_filesystem(filesystem, f"{bucket}/{image_key}")


def write_ocr_result_table(
    filesystem: pafs.FileSystem,
    bucket: str,
    image_filename: str,
    table: pa.Table,
) -> str:
    base_dir = ocr_result_base_dir(bucket, image_filename)
    written_paths = write_parquet_dataset(table, base_dir, filesystem, allow_empty=True)
    if len(written_paths) != 1:
        msg = f"Expected a single OCR parquet file, wrote {written_paths}"
        raise RuntimeError(msg)
    return written_paths[0].removeprefix(f"{bucket}/")
