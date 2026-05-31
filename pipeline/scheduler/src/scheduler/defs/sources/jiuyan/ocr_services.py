from __future__ import annotations

import logging
import time
from collections import Counter
from dataclasses import dataclass, field

from scheduler.defs.common.concurrency import BoundedTaskOptions, BoundedTaskRunner
from scheduler.defs.config.models import JiuyanOcrConfig
from scheduler.defs.http.client_factory import HttpClientFactory
from scheduler.defs.repositories.industry_images import (
    DownloadFailureUpdate,
    DownloadSuccessUpdate,
    OcrFailureUpdate,
    OcrSuccessUpdate,
    PostgresIndustryImageRepository,
)
from scheduler.defs.sources.jiuyan.image_object_store import ImageObjectStore
from scheduler.defs.sources.jiuyan.ocr_client import request_ocr_content
from scheduler.defs.sources.jiuyan.ocr_schema import (
    ClaimedIndustryImage,
    DiscoveredIndustryImage,
    normalize_ocr_content,
    ocr_rows_to_table,
)
from scheduler.defs.sources.jiuyan.state_service import (
    IndustryImageStateService,
)
from scheduler.defs.storage.object_store import download_image_bytes

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
    repository: PostgresIndustryImageRepository | None,
    object_store: ImageObjectStore,
    images: list[DiscoveredIndustryImage],
    log: logging.Logger,
    http_client_factory: HttpClientFactory,
    state_service: IndustryImageStateService | None = None,
) -> ImageDownloadResult:
    result = ImageDownloadResult()
    if not images:
        return result
    effective_state_service = state_service or _state_service_from_repository(repository)

    success_updates: list[DownloadSuccessUpdate] = []
    failure_updates: list[DownloadFailureUpdate] = []
    async with http_client_factory.bytes_client() as client:

        async def process_one(image: DiscoveredIndustryImage) -> DownloadSuccessUpdate:
            downloaded = await download_image_bytes(client, image.image_url)
            object_key = object_store.write_downloaded_image(
                image.image_filename,
                downloaded.image_bytes,
            )
            return DownloadSuccessUpdate(
                image_filename=image.image_filename,
                image_s3_key=object_key,
                download_sha256=downloaded.sha256,
                download_bytes=downloaded.byte_count,
            )

        runner_result = await BoundedTaskRunner(
            BoundedTaskOptions(
                max_concurrent_tasks=IMAGE_DOWNLOAD_CONCURRENCY,
                fail_when_all_failed=False,
            )
        ).run(
            images,
            item_key=lambda image: image.image_filename,
            worker=process_one,
        )
        success_updates.extend(runner_result.successes)
        failure_updates.extend(
            DownloadFailureUpdate(
                image_filename=failure.item_key,
                error_type=failure.error_type,
                error_message=failure.error_message,
            )
            for failure in runner_result.failures
        )
        for failure in runner_result.failures:
            log.warning("Failed to download %s: %s", failure.item_key, failure.error_message)
        result.downloaded_counter["success"] = runner_result.success_count
        result.downloaded_counter["failure"] = runner_result.failure_count
    await effective_state_service.mark_download_success_many(success_updates)
    await effective_state_service.mark_download_failed_many(failure_updates)
    return result


def _image_mime_type(image_s3_key_value: str) -> str:
    suffix = image_s3_key_value.rsplit(".", 1)[-1].lower()
    if suffix in {"jpg", "jpeg"}:
        return "image/jpeg"
    if suffix == "png":
        return "image/png"
    return "image/png"


async def process_ocr_images(
    repository: PostgresIndustryImageRepository | None,
    object_store: ImageObjectStore,
    claimed: list[ClaimedIndustryImage],
    ocr_config: JiuyanOcrConfig,
    max_concurrent_requests: int,
    log: logging.Logger,
    http_client_factory: HttpClientFactory,
    state_service: IndustryImageStateService | None = None,
) -> OcrProcessResult:
    result = OcrProcessResult()
    if not claimed:
        return result
    effective_state_service = state_service or _state_service_from_repository(repository)

    table_convert_seconds_numerator = 0.0
    success_updates: list[OcrSuccessUpdate] = []
    failure_updates: list[OcrFailureUpdate] = []

    async with http_client_factory.json_client(
        headers={"User-Agent": "Mozilla/5.0", "Accept": "application/json,text/plain,*/*"},
        max_attempts=max(ocr_config.max_retries, 0) + 1,
        total_timeout_seconds=ocr_config.timeout_seconds,
        read_timeout_seconds=ocr_config.timeout_seconds,
    ) as client:

        async def process_one(image: ClaimedIndustryImage) -> OcrSuccessUpdate:
            nonlocal table_convert_seconds_numerator
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
            table_convert_seconds_numerator += time.perf_counter() - table_started_at
            return OcrSuccessUpdate(
                image_filename=image.image_filename,
                ocr_result_s3_key=ocr_result_key,
                ocr_result_row_count=table.num_rows,
                ocr_model=ocr_config.model_name,
            )

        runner_result = await BoundedTaskRunner(
            BoundedTaskOptions(
                max_concurrent_tasks=max_concurrent_requests,
                fail_when_all_failed=False,
            )
        ).run(
            claimed,
            item_key=lambda image: image.image_filename,
            worker=process_one,
        )
        success_updates.extend(runner_result.successes)
        failure_updates.extend(
            OcrFailureUpdate(
                image_filename=failure.item_key,
                error_type=failure.error_type,
                error_message=failure.error_message,
            )
            for failure in runner_result.failures
        )

    await effective_state_service.mark_ocr_success_many(success_updates)
    await effective_state_service.mark_ocr_failed_many(failure_updates)
    result.table_convert_seconds = table_convert_seconds_numerator
    for update in success_updates:
        result.ocr_success_count += 1
        if update.ocr_result_row_count == 0:
            result.ocr_empty_count += 1
        else:
            result.ocr_result_rows += update.ocr_result_row_count
        result.s3_keys.append(update.ocr_result_s3_key)
        log.info(
            "OCR success for %s with %s rows",
            update.image_filename,
            update.ocr_result_row_count,
        )
    result.ocr_failure_count = len(failure_updates)
    for failure in failure_updates:
        log.warning("OCR failed for %s: %s", failure.image_filename, failure.error_message)
    return result


def _state_service_from_repository(
    repository: PostgresIndustryImageRepository | None,
) -> IndustryImageStateService:
    if repository is None:
        msg = "repository or state_service is required"
        raise ValueError(msg)
    return IndustryImageStateService(repository)
