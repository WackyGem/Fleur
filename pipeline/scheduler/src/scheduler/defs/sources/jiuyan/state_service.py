from __future__ import annotations

import asyncio
from collections.abc import Sequence
from dataclasses import dataclass
from enum import StrEnum

from scheduler.defs.repositories.industry_images import PostgresIndustryImageRepository
from scheduler.defs.sources.jiuyan.ocr_schema import ClaimedIndustryImage


class ImageWorkflowStatus(StrEnum):
    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"


@dataclass(frozen=True)
class OcrSuccessUpdate:
    image_filename: str
    ocr_result_s3_key: str
    ocr_result_row_count: int
    ocr_model: str


@dataclass(frozen=True)
class OcrFailureUpdate:
    image_filename: str
    error_type: str
    error_message: str


@dataclass(frozen=True)
class DownloadSuccessUpdate:
    image_filename: str
    image_s3_key: str
    download_sha256: str
    download_bytes: int


@dataclass(frozen=True)
class DownloadFailureUpdate:
    image_filename: str
    error_type: str
    error_message: str


class IndustryImageStateService:
    def __init__(self, repository: PostgresIndustryImageRepository) -> None:
        self._repository = repository

    async def claim_ocr_images(
        self,
        *,
        limit: int | None,
        image_filenames: Sequence[str] | None,
        stale_after_seconds: int,
        force_ocr: bool,
    ) -> list[ClaimedIndustryImage]:
        return await asyncio.to_thread(
            self._repository.claim_ocr_images,
            limit=limit,
            image_filenames=image_filenames,
            stale_after_seconds=stale_after_seconds,
            force_ocr=force_ocr,
        )

    async def mark_download_success(self, update: DownloadSuccessUpdate) -> None:
        await self.mark_download_success_many([update])

    async def mark_download_success_many(self, updates: Sequence[DownloadSuccessUpdate]) -> int:
        await asyncio.to_thread(
            self._repository.mark_download_success_many,
            [
                {
                    "image_filename": update.image_filename,
                    "image_s3_key": update.image_s3_key,
                    "download_sha256": update.download_sha256,
                    "download_bytes": update.download_bytes,
                }
                for update in updates
            ],
        )
        return len(updates)

    async def mark_download_failed(self, update: DownloadFailureUpdate) -> None:
        await self.mark_download_failed_many([update])

    async def mark_download_failed_many(self, updates: Sequence[DownloadFailureUpdate]) -> int:
        await asyncio.to_thread(
            self._repository.mark_download_failed_many,
            [
                {
                    "image_filename": update.image_filename,
                    "error_type": update.error_type,
                    "error_message": update.error_message,
                }
                for update in updates
            ],
        )
        return len(updates)

    async def mark_ocr_success(self, update: OcrSuccessUpdate) -> None:
        await self.mark_ocr_success_many([update])

    async def mark_ocr_success_many(self, updates: Sequence[OcrSuccessUpdate]) -> int:
        await asyncio.to_thread(
            self._repository.mark_ocr_success_many,
            [
                {
                    "image_filename": update.image_filename,
                    "ocr_result_s3_key": update.ocr_result_s3_key,
                    "ocr_result_row_count": update.ocr_result_row_count,
                    "ocr_model": update.ocr_model,
                }
                for update in updates
            ],
        )
        return len(updates)

    async def mark_ocr_failed(self, update: OcrFailureUpdate) -> None:
        await self.mark_ocr_failed_many([update])

    async def mark_ocr_failed_many(self, updates: Sequence[OcrFailureUpdate]) -> int:
        await asyncio.to_thread(
            self._repository.mark_ocr_failed_many,
            [
                {
                    "image_filename": update.image_filename,
                    "error_type": update.error_type,
                    "error_message": update.error_message,
                }
                for update in updates
            ],
        )
        return len(updates)
