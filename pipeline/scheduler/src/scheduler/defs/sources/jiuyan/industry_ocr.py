from __future__ import annotations

import asyncio
import time
from collections import Counter
from collections.abc import Mapping
from dataclasses import dataclass

import dagster as dg
import pyarrow as pa

from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.config.models import JiuyanOcrConfig, PipelineDatabaseConfig, S3Config
from scheduler.defs.repositories.industry_images import PostgresIndustryImageRepository
from scheduler.defs.sources.jiuyan.image_urls import (
    image_filename_from_url,
    image_s3_key,
    parse_image_urls,
)
from scheduler.defs.sources.jiuyan.industry_list import jiuyan__industry_list
from scheduler.defs.sources.jiuyan.ocr_schema import DiscoveredIndustryImage
from scheduler.defs.sources.jiuyan.ocr_services import (
    download_images_to_s3,
    process_ocr_images,
)
from scheduler.defs.storage.object_store import ImageObjectStore
from scheduler.defs.storage.parquet_readers import read_parquet_table_from_s3

IMAGE_DOWNLOAD_CONCURRENCY = 10


@dataclass(frozen=True)
class IndustryImagesAssetConfig:
    limit: int | None
    force_download: bool
    image_filenames: list[str]


@dataclass(frozen=True)
class IndustryOcrAssetConfig:
    limit: int | None
    force_ocr: bool
    image_filenames: list[str]
    max_concurrent_requests: int | None


@dg.asset(
    name="jiuyan__industry_images",
    group_name="http_sources",
    deps=[jiuyan__industry_list],
    config_schema={
        "limit": dg.Field(int, is_required=False, default_value=0),
        "force_download": dg.Field(bool, is_required=False, default_value=False),
        "image_filenames": dg.Field([str], is_required=False, default_value=[]),
    },
    tags={
        "source": "jiuyan",
        "layer": "raw",
        "storage": "s3",
        "state": "postgres",
        "modality": "ocr",
    },
)
def jiuyan__industry_images(
    context,
) -> dg.MaterializeResult:
    result = asyncio.run(_materialize_industry_images(context))
    return dg.MaterializeResult(metadata=result)


@dg.asset(
    name="jiuyan__industry_ocr",
    group_name="http_sources",
    deps=[jiuyan__industry_images],
    config_schema={
        "limit": dg.Field(int, is_required=False, default_value=0),
        "force_ocr": dg.Field(bool, is_required=False, default_value=False),
        "image_filenames": dg.Field([str], is_required=False, default_value=[]),
        "max_concurrent_requests": dg.Field(int, is_required=False, default_value=0),
    },
    tags={
        "source": "jiuyan",
        "layer": "raw",
        "storage": "s3",
        "state": "postgres",
        "modality": "ocr",
    },
)
def jiuyan__industry_ocr(
    context,
) -> dg.MaterializeResult:
    result = asyncio.run(_materialize_industry_ocr(context))
    return dg.MaterializeResult(metadata=result)


async def _materialize_industry_images(
    context: dg.AssetExecutionContext,
) -> dict[str, RawMetadataValue]:
    started_at = time.perf_counter()
    s3_config = S3Config.from_env()
    database_config = PipelineDatabaseConfig.from_env()
    object_store = ImageObjectStore.from_s3_config(s3_config)
    repository = PostgresIndustryImageRepository(database_config.url)
    config = _images_asset_config(context.op_config)
    upstream_table = read_parquet_table_from_s3(
        s3_config,
        dg.AssetKey("jiuyan__industry_list"),
        storage_mode="latest_snapshot",
    )

    discovered_images, discovery_stats = discover_images_from_table(upstream_table)
    existing_urls = repository.fetch_existing_image_urls(
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

    upsert_count = repository.upsert_discovered_images(discovered_images)
    requested_filenames = set(config.image_filenames)
    selected_images = [
        image
        for image in discovered_images
        if not requested_filenames or image.image_filename in requested_filenames
    ]
    if config.limit is not None:
        selected_images = selected_images[: config.limit]

    download_skip_existing_count = 0
    if not config.force_download:
        current_rows = repository.fetch_images([image.image_filename for image in selected_images])
        current_status_by_filename = {
            str(row["image_filename"]): str(row["download_status"]) for row in current_rows
        }
        filtered_images: list[DiscoveredIndustryImage] = []
        for image in selected_images:
            if current_status_by_filename.get(image.image_filename) == "success":
                download_skip_existing_count += 1
                continue
            filtered_images.append(image)
        selected_images = filtered_images

    download_result = await download_images_to_s3(
        repository=repository,
        object_store=object_store,
        images=selected_images,
        log=context.log,
    )

    metadata: dict[str, RawMetadataValue] = {
        **discovery_stats,
        "postgres_upsert_count": upsert_count,
        "download_request_count": len(selected_images),
        "download_success_count": download_result.success_count,
        "download_skip_existing_count": download_skip_existing_count,
        "download_failure_count": download_result.failure_count,
        "image_s3_write_count": download_result.success_count,
        "asset_function_seconds": round(time.perf_counter() - started_at, 6),
        "s3_bucket": s3_config.bucket,
        "s3_keys_sample": dg.MetadataValue.json(
            [image_s3_key(image.image_filename) for image in selected_images[:3]]
        ),
    }
    return metadata


async def _materialize_industry_ocr(
    context: dg.AssetExecutionContext,
) -> dict[str, RawMetadataValue]:
    started_at = time.perf_counter()
    s3_config = S3Config.from_env()
    database_config = PipelineDatabaseConfig.from_env()
    ocr_config = JiuyanOcrConfig.from_env()
    object_store = ImageObjectStore.from_s3_config(s3_config)
    repository = PostgresIndustryImageRepository(database_config.url)
    config = _ocr_asset_config(context.op_config)

    requested_filenames = config.image_filenames
    max_concurrent_requests = config.max_concurrent_requests or ocr_config.max_concurrent_requests
    if max_concurrent_requests < 1:
        msg = "max_concurrent_requests must be positive"
        raise ValueError(msg)

    ocr_request_started_at = time.perf_counter()
    claimed = repository.claim_ocr_images(
        limit=config.limit,
        image_filenames=requested_filenames,
        stale_after_seconds=ocr_config.stale_running_seconds,
        force_ocr=config.force_ocr,
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
            "ocr_model": ocr_config.model_name,
            "ocr_base_url_host": _base_url_host(ocr_config.base_url),
            "max_concurrent_requests": max_concurrent_requests,
            "asset_function_seconds": round(time.perf_counter() - started_at, 6),
            "ocr_request_seconds": 0.0,
            "table_convert_seconds": 0.0,
            "result_s3_keys_sample": dg.MetadataValue.json([]),
        }

    ocr_result = await process_ocr_images(
        repository=repository,
        object_store=object_store,
        claimed=claimed,
        ocr_config=ocr_config,
        max_concurrent_requests=max_concurrent_requests,
        log=context.log,
    )

    metadata: dict[str, RawMetadataValue] = {
        "claimed_image_count": len(claimed),
        "ocr_request_count": len(claimed),
        "ocr_success_count": ocr_result.ocr_success_count,
        "ocr_empty_count": ocr_result.ocr_empty_count,
        "ocr_failure_count": ocr_result.ocr_failure_count,
        "ocr_result_row_count": ocr_result.ocr_result_rows,
        "ocr_skip_success_count": 0,
        "ocr_model": ocr_config.model_name,
        "ocr_base_url_host": _base_url_host(ocr_config.base_url),
        "max_concurrent_requests": max_concurrent_requests,
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


def _images_asset_config(op_config: Mapping[str, object] | None) -> IndustryImagesAssetConfig:
    config = dict(op_config or {})
    limit = config.get("limit")
    image_filenames = config.get("image_filenames", [])
    return IndustryImagesAssetConfig(
        limit=limit if isinstance(limit, int) and limit > 0 else None,
        force_download=bool(config.get("force_download", False)),
        image_filenames=_string_list_from_config(image_filenames),
    )


def _ocr_asset_config(op_config: Mapping[str, object] | None) -> IndustryOcrAssetConfig:
    config = dict(op_config or {})
    limit = config.get("limit")
    image_filenames = config.get("image_filenames", [])
    max_concurrent_requests = config.get("max_concurrent_requests")
    return IndustryOcrAssetConfig(
        limit=limit if isinstance(limit, int) and limit > 0 else None,
        force_ocr=bool(config.get("force_ocr", False)),
        image_filenames=_string_list_from_config(image_filenames),
        max_concurrent_requests=(
            max_concurrent_requests
            if isinstance(max_concurrent_requests, int) and max_concurrent_requests > 0
            else None
        ),
    )


def _string_list_from_config(value: object) -> list[str]:
    if not isinstance(value, list):
        return []
    return [str(item).strip() for item in value if str(item).strip()]


def _image_mime_type(image_s3_key_value: str) -> str:
    suffix = image_s3_key_value.rsplit(".", 1)[-1].lower()
    if suffix in {"jpg", "jpeg"}:
        return "image/jpeg"
    if suffix == "png":
        return "image/png"
    return "image/png"


def _base_url_host(base_url: str) -> str:
    return base_url.split("://", 1)[-1].split("/", 1)[0]


def _string_or_empty(value: object | None) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return value.strip()
    return str(value).strip()
