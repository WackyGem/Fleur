from __future__ import annotations

import asyncio
import logging
import time
from collections import Counter
from dataclasses import dataclass, field

from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY
from scheduler.defs.config.models import JiuyanOcrConfig
from scheduler.defs.http.client import AioHttpClient
from scheduler.defs.repositories.industry_images import PostgresIndustryImageRepository
from scheduler.defs.sources.jiuyan.ocr_client import request_ocr_content
from scheduler.defs.sources.jiuyan.ocr_schema import (
    ClaimedIndustryImage,
    DiscoveredIndustryImage,
    normalize_ocr_content,
    ocr_rows_to_table,
)
from scheduler.defs.storage.object_store import ImageObjectStore, download_image_bytes

IMAGE_DOWNLOAD_CONCURRENCY = 10

logger = logging.getLogger(__name__)


@dataclass
class ImageDownloadResult:
    downloaded_counter: Counter[str] = field(default_factory=Counter)
    download_skip_existing_count: int = 0

    @property
    def success_count(self) -> int:
        return self.downloaded_counter["success"]

    @property
    def failure_count(self) -> int:
        return self.downloaded_counter["failure"]


@dataclass
class OcrProcessResult:
    ocr_success_count: int = 0
    ocr_empty_count: int = 0
    ocr_failure_count: int = 0
    ocr_result_rows: int = 0
    table_convert_seconds: float = 0.0
    s3_keys: list[str] = field(default_factory=list)


async def download_images_to_s3(
    repository: PostgresIndustryImageRepository,
    object_store: ImageObjectStore,
    images: list[DiscoveredIndustryImage],
    log: logging.Logger,
) -> ImageDownloadResult:
    result = ImageDownloadResult()
    if not images:
        return result

    semaphore = asyncio.Semaphore(IMAGE_DOWNLOAD_CONCURRENCY)
    async with AioHttpClient(retry_policy=DEFAULT_RETRY_POLICY) as client:

        async def process_one(image: DiscoveredIndustryImage) -> None:
            async with semaphore:
                try:
                    downloaded = await download_image_bytes(client, image.image_url)
                    object_key = object_store.write_downloaded_image(
                        image.image_filename,
                        downloaded.image_bytes,
                    )
                    repository.mark_download_success(
                        image_filename=image.image_filename,
                        image_s3_key_value=object_key,
                        download_sha256=downloaded.sha256,
                        download_bytes=downloaded.byte_count,
                    )
                    result.downloaded_counter["success"] += 1
                except Exception as error:
                    repository.mark_download_failed(
                        image_filename=image.image_filename,
                        error_type=type(error).__name__,
                        error_message=str(error),
                    )
                    result.downloaded_counter["failure"] += 1
                    log.warning(
                        "Failed to download %s: %s",
                        image.image_filename,
                        error,
                    )

        await asyncio.gather(*(process_one(image) for image in images))
    return result


def _image_mime_type(image_s3_key_value: str) -> str:
    suffix = image_s3_key_value.rsplit(".", 1)[-1].lower()
    if suffix in {"jpg", "jpeg"}:
        return "image/jpeg"
    if suffix == "png":
        return "image/png"
    return "image/png"


async def process_ocr_images(
    repository: PostgresIndustryImageRepository,
    object_store: ImageObjectStore,
    claimed: list[ClaimedIndustryImage],
    ocr_config: JiuyanOcrConfig,
    max_concurrent_requests: int,
    log: logging.Logger,
) -> OcrProcessResult:
    result = OcrProcessResult()
    if not claimed:
        return result

    semaphore = asyncio.Semaphore(max_concurrent_requests)
    table_convert_seconds_numerator = 0.0

    async with AioHttpClient(
        headers={"User-Agent": "Mozilla/5.0", "Accept": "application/json,text/plain,*/*"},
        retry_policy=DEFAULT_RETRY_POLICY,
        max_attempts=max(ocr_config.max_retries, 0) + 1,
        total_timeout_seconds=ocr_config.timeout_seconds,
        read_timeout_seconds=ocr_config.timeout_seconds,
    ) as client:

        async def process_one(image: ClaimedIndustryImage) -> tuple[str, int, str | None]:
            nonlocal table_convert_seconds_numerator
            async with semaphore:
                try:
                    image_bytes = object_store.read_image_bytes(image.image_s3_key)
                    content = await request_ocr_content(
                        client,
                        base_url=ocr_config.base_url,
                        model_name=ocr_config.model_name,
                        image_bytes=image_bytes,
                        mime_type=_image_mime_type(image.image_s3_key),
                    )
                    normalized_rows = normalize_ocr_content(content)
                    table_started_at = time.perf_counter()
                    table = ocr_rows_to_table(image.industry_id, normalized_rows)
                    ocr_result_key = object_store.write_ocr_result_table(
                        image.image_filename,
                        table,
                    )
                    result.ocr_success_count += 1
                    if table.num_rows == 0:
                        result.ocr_empty_count += 1
                    else:
                        result.ocr_result_rows += table.num_rows
                    repository.mark_ocr_success(
                        image_filename=image.image_filename,
                        ocr_result_s3_key_value=ocr_result_key,
                        ocr_result_row_count=table.num_rows,
                        ocr_model=ocr_config.model_name,
                    )
                    log.info(
                        "OCR success for %s with %s rows",
                        image.image_filename,
                        table.num_rows,
                    )
                    table_convert_seconds_numerator += time.perf_counter() - table_started_at
                    return image.image_filename, table.num_rows, ocr_result_key
                except Exception as error:
                    result.ocr_failure_count += 1
                    repository.mark_ocr_failed(
                        image_filename=image.image_filename,
                        error_type=type(error).__name__,
                        error_message=str(error),
                    )
                    log.warning("OCR failed for %s: %s", image.image_filename, error)
                    return image.image_filename, 0, None

        results = await asyncio.gather(*(process_one(image) for image in claimed))

    result.table_convert_seconds = table_convert_seconds_numerator
    result.s3_keys = [key for _, _, key in results if key is not None]
    return result
