from __future__ import annotations

import asyncio
from collections.abc import Sequence
from enum import StrEnum

from scheduler.defs.repositories.industry_images import (
    DownloadFailureUpdate,
    DownloadSuccessUpdate,
    OcrFailureUpdate,
    OcrSuccessUpdate,
    PostgresIndustryImageRepository,
)
from scheduler.defs.sources.jiuyan.ocr_schema import ClaimedIndustryImage


class ImageWorkflowStatus(StrEnum):
    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"


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
        return await asyncio.to_thread(self._repository.mark_download_success_many, updates)

    async def mark_download_failed(self, update: DownloadFailureUpdate) -> None:
        await self.mark_download_failed_many([update])

    async def mark_download_failed_many(self, updates: Sequence[DownloadFailureUpdate]) -> int:
        return await asyncio.to_thread(self._repository.mark_download_failed_many, updates)

    async def mark_ocr_success(self, update: OcrSuccessUpdate) -> None:
        await self.mark_ocr_success_many([update])

    async def mark_ocr_success_many(self, updates: Sequence[OcrSuccessUpdate]) -> int:
        return await asyncio.to_thread(self._repository.mark_ocr_success_many, updates)

    async def mark_ocr_failed(self, update: OcrFailureUpdate) -> None:
        await self.mark_ocr_failed_many([update])

    async def mark_ocr_failed_many(self, updates: Sequence[OcrFailureUpdate]) -> int:
        return await asyncio.to_thread(self._repository.mark_ocr_failed_many, updates)
