from __future__ import annotations

import logging
import time
from collections import Counter

import dagster as dg
import pyarrow as pa

from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.config.models import JiuyanOcrConfig, S3Config
from scheduler.defs.repositories.industry_images import PostgresIndustryImageRepository
from scheduler.defs.sources.jiuyan.image_urls import (
    image_filename_from_url,
    image_s3_key,
    parse_image_urls,
)
from scheduler.defs.sources.jiuyan.ocr_schema import DiscoveredIndustryImage
from scheduler.defs.sources.jiuyan.ocr_services import (
    download_images_to_s3,
    process_ocr_images,
)
from scheduler.defs.storage.object_store import ImageObjectStore
from scheduler.defs.storage.parquet_readers import read_parquet_table_from_s3


class JiuyanIndustryImageWorkflow:
    def __init__(
        self,
        *,
        s3_config: S3Config,
        repository: PostgresIndustryImageRepository,
        object_store: ImageObjectStore,
        upstream_asset_key: dg.AssetKey,
        log: logging.Logger,
    ) -> None:
        self._s3_config = s3_config
        self._repository = repository
        self._object_store = object_store
        self._upstream_asset_key = upstream_asset_key
        self._log = log

    async def refresh_images(
        self,
        *,
        limit: int | None,
        force_download: bool,
        image_filenames: list[str],
    ) -> dict[str, RawMetadataValue]:
        started_at = time.perf_counter()
        upstream_table = read_parquet_table_from_s3(
            self._s3_config,
            self._upstream_asset_key,
            storage_mode="latest_snapshot",
        )

        discovered_images, discovery_stats = discover_images_from_table(upstream_table)
        self._validate_existing_urls(discovered_images)
        upsert_count = self._repository.upsert_discovered_images(discovered_images)
        selected_images, download_skip_existing_count = self._select_images(
            discovered_images,
            limit=limit,
            force_download=force_download,
            image_filenames=image_filenames,
        )

        download_result = await download_images_to_s3(
            repository=self._repository,
            object_store=self._object_store,
            images=selected_images,
            log=self._log,
        )

        return {
            **discovery_stats,
            "postgres_upsert_count": upsert_count,
            "download_request_count": len(selected_images),
            "download_success_count": download_result.success_count,
            "download_skip_existing_count": download_skip_existing_count,
            "download_failure_count": download_result.failure_count,
            "image_s3_write_count": download_result.success_count,
            "asset_function_seconds": round(time.perf_counter() - started_at, 6),
            "s3_bucket": self._s3_config.bucket,
            "s3_keys_sample": dg.MetadataValue.json(
                [image_s3_key(image.image_filename) for image in selected_images[:3]]
            ),
        }

    def _validate_existing_urls(self, discovered_images: list[DiscoveredIndustryImage]) -> None:
        existing_urls = self._repository.fetch_existing_image_urls(
            [image.image_filename for image in discovered_images]
        )
        for image in discovered_images:
            existing_url = existing_urls.get(image.image_filename)
            if existing_url is not None and existing_url != image.image_url:
                msg = (
                    f"Conflicting image filename {image.image_filename}: "
                    f"{existing_url!r} != {image.image_url!r}"
                )
                raise RuntimeError(msg)

    def _select_images(
        self,
        discovered_images: list[DiscoveredIndustryImage],
        *,
        limit: int | None,
        force_download: bool,
        image_filenames: list[str],
    ) -> tuple[list[DiscoveredIndustryImage], int]:
        requested_filenames = set(image_filenames)
        selected_images = [
            image
            for image in discovered_images
            if not requested_filenames or image.image_filename in requested_filenames
        ]
        if limit is not None:
            selected_images = selected_images[:limit]

        if force_download:
            return selected_images, 0

        current_rows = self._repository.fetch_images(
            [image.image_filename for image in selected_images]
        )
        current_status_by_filename = {
            str(row["image_filename"]): str(row["download_status"]) for row in current_rows
        }
        filtered_images: list[DiscoveredIndustryImage] = []
        skip_existing_count = 0
        for image in selected_images:
            if current_status_by_filename.get(image.image_filename) == "success":
                skip_existing_count += 1
                continue
            filtered_images.append(image)
        return filtered_images, skip_existing_count


class JiuyanIndustryOcrWorkflow:
    def __init__(
        self,
        *,
        repository: PostgresIndustryImageRepository,
        object_store: ImageObjectStore,
        ocr_config: JiuyanOcrConfig,
        log: logging.Logger,
    ) -> None:
        self._repository = repository
        self._object_store = object_store
        self._ocr_config = ocr_config
        self._log = log

    async def refresh_ocr(
        self,
        *,
        limit: int | None,
        force_ocr: bool,
        image_filenames: list[str],
        max_concurrent_requests: int | None,
    ) -> dict[str, RawMetadataValue]:
        started_at = time.perf_counter()
        effective_concurrency = max_concurrent_requests or self._ocr_config.max_concurrent_requests
        if effective_concurrency < 1:
            msg = "max_concurrent_requests must be positive"
            raise ValueError(msg)

        ocr_request_started_at = time.perf_counter()
        claimed = self._repository.claim_ocr_images(
            limit=limit,
            image_filenames=image_filenames,
            stale_after_seconds=self._ocr_config.stale_running_seconds,
            force_ocr=force_ocr,
        )

        if not claimed:
            return {
                "claimed_image_count": 0,
                "ocr_request_count": 0,
                "ocr_success_count": 0,
                "ocr_empty_count": 0,
                "ocr_failure_count": 0,
                "ocr_result_row_count": 0,
                "ocr_skip_success_count": 0,
                "ocr_model": self._ocr_config.model_name,
                "ocr_base_url_host": _base_url_host(self._ocr_config.base_url),
                "max_concurrent_requests": effective_concurrency,
                "asset_function_seconds": round(time.perf_counter() - started_at, 6),
                "ocr_request_seconds": 0.0,
                "table_convert_seconds": 0.0,
                "result_s3_keys_sample": dg.MetadataValue.json([]),
            }

        ocr_result = await process_ocr_images(
            repository=self._repository,
            object_store=self._object_store,
            claimed=claimed,
            ocr_config=self._ocr_config,
            max_concurrent_requests=effective_concurrency,
            log=self._log,
        )

        metadata: dict[str, RawMetadataValue] = {
            "claimed_image_count": len(claimed),
            "ocr_request_count": len(claimed),
            "ocr_success_count": ocr_result.ocr_success_count,
            "ocr_empty_count": ocr_result.ocr_empty_count,
            "ocr_failure_count": ocr_result.ocr_failure_count,
            "ocr_result_row_count": ocr_result.ocr_result_rows,
            "ocr_skip_success_count": 0,
            "ocr_model": self._ocr_config.model_name,
            "ocr_base_url_host": _base_url_host(self._ocr_config.base_url),
            "max_concurrent_requests": effective_concurrency,
            "asset_function_seconds": round(time.perf_counter() - started_at, 6),
            "ocr_request_seconds": round(time.perf_counter() - ocr_request_started_at, 6),
            "table_convert_seconds": round(ocr_result.table_convert_seconds, 6),
            "result_s3_keys_sample": dg.MetadataValue.json(ocr_result.s3_keys[:3]),
        }
        if claimed and ocr_result.ocr_failure_count == len(claimed):
            raise RuntimeError("All OCR requests failed")
        if claimed and ocr_result.ocr_failure_count / len(claimed) > 0.2:
            raise RuntimeError("OCR failure rate exceeded 20%")
        return metadata


def discover_images_from_table(
    table: pa.Table,
) -> tuple[list[DiscoveredIndustryImage], dict[str, RawMetadataValue]]:
    rows = table.to_pylist()
    discovered: list[DiscoveredIndustryImage] = []
    seen: dict[str, str] = {}
    stats = Counter()
    stats["article_count"] = len(rows)
    article_with_imgs = 0
    for row in rows:
        industry_id = _string_or_empty(row.get("industry_id"))
        raw_imgs = row.get("imgs")
        urls = parse_image_urls(raw_imgs)
        if raw_imgs not in (None, "", []) and not urls:
            stats["imgs_parse_error_count"] += 1
        if urls:
            article_with_imgs += 1
        for image_index, url in enumerate(urls):
            stats["parsed_image_url_count"] += 1
            filename = image_filename_from_url(url)
            previous_url = seen.get(filename)
            if previous_url is not None and previous_url != url:
                msg = f"Conflicting image filename {filename}: {previous_url!r} != {url!r}"
                raise RuntimeError(msg)
            seen[filename] = url
            discovered.append(
                DiscoveredIndustryImage(
                    image_filename=filename,
                    image_url=url,
                    industry_id=industry_id,
                    image_index=image_index,
                )
            )
    stats["article_with_imgs_count"] = article_with_imgs
    stats["unique_image_filename_count"] = len(seen)
    return discovered, dict(stats)


def _base_url_host(base_url: str) -> str:
    return base_url.split("://", 1)[-1].split("/", 1)[0]


def _string_or_empty(value: object | None) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return value.strip()
    return str(value).strip()
