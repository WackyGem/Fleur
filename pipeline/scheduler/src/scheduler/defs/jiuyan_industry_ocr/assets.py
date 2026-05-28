from __future__ import annotations

import asyncio
import time
from collections import Counter
from collections.abc import Mapping

import dagster as dg
import pyarrow as pa

from scheduler.defs.config import JiuyanOcrConfig, PipelineDatabaseConfig, S3Config
from scheduler.defs.http_resources.client import AioHttpClient
from scheduler.defs.http_resources.jiuyan__industry_list import jiuyan__industry_list
from scheduler.defs.util import DEFAULT_RETRY_POLICY
from scheduler.defs.jiuyan_industry_ocr.image_store import (
    build_s3_filesystem_for_config,
    download_image_bytes,
    read_image_bytes,
    write_downloaded_image,
    write_ocr_result_table,
)
from scheduler.defs.jiuyan_industry_ocr.image_urls import (
    image_filename_from_url,
    image_s3_key,
    parse_image_urls,
)
from scheduler.defs.jiuyan_industry_ocr.ocr_client import request_ocr_content
from scheduler.defs.jiuyan_industry_ocr.ocr_schema import normalize_ocr_content, ocr_rows_to_table
from scheduler.defs.jiuyan_industry_ocr.postgres import (
    claim_ocr_images,
    fetch_existing_image_urls,
    fetch_images,
    mark_download_failed,
    mark_download_success,
    mark_ocr_failed,
    mark_ocr_success,
    upsert_discovered_images,
)
from scheduler.defs.jiuyan_industry_ocr.schemas import ClaimedIndustryImage, DiscoveredIndustryImage
from scheduler.defs.util import read_parquet_table_from_s3

IMAGE_DOWNLOAD_CONCURRENCY = 10


@dg.asset(
    name="jiuyan__industry_images",
    group_name="jiuyan_industry_ocr",
    deps=[jiuyan__industry_list],
    config_schema={
        "limit": dg.Field(int, is_required=False, default_value=0),
        "force_download": dg.Field(bool, is_required=False, default_value=False),
        "image_filenames": dg.Field([str], is_required=False, default_value=[]),
    },
    tags={"source": "jiuyan", "layer": "raw", "storage": "s3", "state": "postgres"},
)
def jiuyan__industry_images(
    context,
) -> dg.MaterializeResult[None]:
    result = asyncio.run(_materialize_industry_images(context))
    return dg.MaterializeResult(metadata=result)


@dg.asset(
    name="jiuyan__industry_ocr",
    group_name="jiuyan_industry_ocr",
    deps=[jiuyan__industry_images],
    config_schema={
        "limit": dg.Field(int, is_required=False, default_value=0),
        "force_ocr": dg.Field(bool, is_required=False, default_value=False),
        "image_filenames": dg.Field([str], is_required=False, default_value=[]),
        "max_concurrent_requests": dg.Field(int, is_required=False, default_value=0),
    },
    tags={"source": "jiuyan", "layer": "raw", "storage": "s3", "state": "postgres"},
)
def jiuyan__industry_ocr(
    context,
) -> dg.MaterializeResult[None]:
    result = asyncio.run(_materialize_industry_ocr(context))
    return dg.MaterializeResult(metadata=result)


jiuyan__industry_images_job = dg.define_asset_job(
    name="jiuyan__industry_images_job",
    selection=[jiuyan__industry_images],
)

jiuyan__industry_ocr_job = dg.define_asset_job(
    name="jiuyan__industry_ocr_job",
    selection=[jiuyan__industry_ocr],
)

jiuyan__industry_ocr_full_job = dg.define_asset_job(
    name="jiuyan__industry_ocr_full_job",
    selection=[jiuyan__industry_images, jiuyan__industry_ocr],
)


async def _materialize_industry_images(
    context: dg.AssetExecutionContext,
) -> dict[str, object]:
    started_at = time.perf_counter()
    s3_config = S3Config.from_env()
    database_config = PipelineDatabaseConfig.from_env()
    filesystem = build_s3_filesystem_for_config(s3_config)
    config = _images_asset_config(context.op_config)
    upstream_table = read_parquet_table_from_s3(
        s3_config,
        dg.AssetKey("jiuyan__industry_list"),
        storage_mode="latest_snapshot",
    )

    discovered_images, discovery_stats = _discover_images_from_table(upstream_table)
    existing_urls = fetch_existing_image_urls(
        database_config.url,
        [image.image_filename for image in discovered_images],
    )
    for image in discovered_images:
        existing_url = existing_urls.get(image.image_filename)
        if existing_url is not None and existing_url != image.image_url:
            msg = (
                f"Conflicting image filename {image.image_filename}: "
                f"{existing_url!r} != {image.image_url!r}"
            )
            raise RuntimeError(msg)

    upsert_count = upsert_discovered_images(database_config.url, discovered_images)
    requested_filenames = set(config["image_filenames"])
    selected_images = [
        image
        for image in discovered_images
        if not requested_filenames or image.image_filename in requested_filenames
    ]
    if config["limit"] is not None:
        selected_images = selected_images[: config["limit"]]

    download_skip_existing_count = 0
    if not config["force_download"]:
        current_rows = fetch_images(
            database_config.url,
            [image.image_filename for image in selected_images],
        )
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

    if not selected_images:
        metadata = {
            **discovery_stats,
            "postgres_upsert_count": upsert_count,
            "download_request_count": 0,
            "download_success_count": 0,
            "download_skip_existing_count": download_skip_existing_count,
            "download_failure_count": 0,
            "image_s3_write_count": 0,
            "asset_function_seconds": round(time.perf_counter() - started_at, 6),
            "s3_bucket": s3_config.bucket,
            "s3_keys_sample": dg.MetadataValue.json([]),
        }
        return metadata

    downloaded_counter = Counter()
    downloaded_counter["success"] = 0
    downloaded_counter["failure"] = 0
    semaphore = asyncio.Semaphore(IMAGE_DOWNLOAD_CONCURRENCY)
    async with AioHttpClient(retry_policy=DEFAULT_RETRY_POLICY) as client:

        async def process_one(image: DiscoveredIndustryImage) -> None:
            async with semaphore:
                try:
                    downloaded = await download_image_bytes(client, image.image_url)
                    object_key = write_downloaded_image(
                        filesystem,
                        s3_config.bucket,
                        image.image_filename,
                        downloaded.image_bytes,
                    )
                    mark_download_success(
                        database_config.url,
                        image_filename=image.image_filename,
                        image_s3_key_value=object_key,
                        download_sha256=downloaded.sha256,
                        download_bytes=downloaded.byte_count,
                    )
                    downloaded_counter["success"] += 1
                except Exception as error:
                    mark_download_failed(
                        database_config.url,
                        image_filename=image.image_filename,
                        error_type=type(error).__name__,
                        error_message=str(error),
                    )
                    downloaded_counter["failure"] += 1
                    context.log.warning(
                        "Failed to download %s: %s",
                        image.image_filename,
                        error,
                    )

        await asyncio.gather(*(process_one(image) for image in selected_images))

    metadata = {
        **discovery_stats,
        "postgres_upsert_count": upsert_count,
        "download_request_count": len(selected_images),
        "download_success_count": downloaded_counter["success"],
        "download_skip_existing_count": download_skip_existing_count,
        "download_failure_count": downloaded_counter["failure"],
        "image_s3_write_count": downloaded_counter["success"],
        "asset_function_seconds": round(time.perf_counter() - started_at, 6),
        "s3_bucket": s3_config.bucket,
        "s3_keys_sample": dg.MetadataValue.json(
            [image_s3_key(image.image_filename) for image in selected_images[:3]]
        ),
    }
    return metadata


async def _materialize_industry_ocr(
    context: dg.AssetExecutionContext,
) -> dict[str, object]:
    started_at = time.perf_counter()
    s3_config = S3Config.from_env()
    database_config = PipelineDatabaseConfig.from_env()
    ocr_config = JiuyanOcrConfig.from_env()
    filesystem = build_s3_filesystem_for_config(s3_config)
    config = _ocr_asset_config(context.op_config)

    requested_filenames = config["image_filenames"]
    max_concurrent_requests = (
        config["max_concurrent_requests"] or ocr_config.max_concurrent_requests
    )
    if max_concurrent_requests < 1:
        msg = "max_concurrent_requests must be positive"
        raise ValueError(msg)

    claimed = claim_ocr_images(
        database_config.url,
        limit=config["limit"],
        image_filenames=requested_filenames,
        stale_after_seconds=ocr_config.stale_running_seconds,
        force_ocr=config["force_ocr"],
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

    semaphore = asyncio.Semaphore(max_concurrent_requests)
    stats = Counter()
    stats["ocr_success"] = 0
    stats["ocr_empty"] = 0
    stats["ocr_failure"] = 0
    stats["table_convert_seconds_numerator"] = 0.0
    ocr_request_started_at = time.perf_counter()

    async with AioHttpClient(
        headers={"User-Agent": "Mozilla/5.0", "Accept": "application/json,text/plain,*/*"},
        retry_policy=DEFAULT_RETRY_POLICY,
        max_attempts=max(ocr_config.max_retries, 0) + 1,
        total_timeout_seconds=ocr_config.timeout_seconds,
        read_timeout_seconds=ocr_config.timeout_seconds,
    ) as client:

        async def process_one(image: ClaimedIndustryImage) -> tuple[str, int, str | None]:
            async with semaphore:
                try:
                    image_bytes = read_image_bytes(filesystem, s3_config.bucket, image.image_s3_key)
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
                    ocr_result_key = write_ocr_result_table(
                        filesystem,
                        s3_config.bucket,
                        image.image_filename,
                        table,
                    )
                    stats["ocr_success"] += 1
                    if table.num_rows == 0:
                        stats["ocr_empty"] += 1
                    else:
                        stats["ocr_result_rows"] += table.num_rows
                    mark_ocr_success(
                        database_config.url,
                        image_filename=image.image_filename,
                        ocr_result_s3_key_value=ocr_result_key,
                        ocr_result_row_count=table.num_rows,
                        ocr_model=ocr_config.model_name,
                    )
                    context.log.info(
                        "OCR success for %s with %s rows",
                        image.image_filename,
                        table.num_rows,
                    )
                    stats["table_convert_seconds_numerator"] += (
                        time.perf_counter() - table_started_at
                    )
                    return image.image_filename, table.num_rows, ocr_result_key
                except Exception as error:
                    stats["ocr_failure"] += 1
                    mark_ocr_failed(
                        database_config.url,
                        image_filename=image.image_filename,
                        error_type=type(error).__name__,
                        error_message=str(error),
                    )
                    context.log.warning("OCR failed for %s: %s", image.image_filename, error)
                    return image.image_filename, 0, None

        results = await asyncio.gather(*(process_one(image) for image in claimed))

    total_rows = sum(row_count for _, row_count, _ in results)
    s3_keys = [key for _, _, key in results if key is not None]
    metadata = {
        "claimed_image_count": len(claimed),
        "ocr_request_count": len(claimed),
        "ocr_success_count": stats["ocr_success"],
        "ocr_empty_count": stats["ocr_empty"],
        "ocr_failure_count": stats["ocr_failure"],
        "ocr_result_row_count": total_rows,
        "ocr_skip_success_count": 0,
        "ocr_model": ocr_config.model_name,
        "ocr_base_url_host": _base_url_host(ocr_config.base_url),
        "max_concurrent_requests": max_concurrent_requests,
        "asset_function_seconds": round(time.perf_counter() - started_at, 6),
        "ocr_request_seconds": round(time.perf_counter() - ocr_request_started_at, 6),
        "table_convert_seconds": round(stats["table_convert_seconds_numerator"], 6),
        "result_s3_keys_sample": dg.MetadataValue.json(s3_keys[:3]),
    }
    if claimed and stats["ocr_failure"] == len(claimed):
        raise RuntimeError("All OCR requests failed")
    if claimed and stats["ocr_failure"] / len(claimed) > 0.2:
        raise RuntimeError("OCR failure rate exceeded 20%")
    return metadata


def _discover_images_from_table(
    table: pa.Table,
) -> tuple[list[DiscoveredIndustryImage], dict[str, object]]:
    rows = table.to_pylist()
    discovered: list[DiscoveredIndustryImage] = []
    seen: dict[str, str] = {}
    stats = Counter()
    stats["article_count"] = len(rows)
    article_with_imgs = 0
    for row_index, row in enumerate(rows):
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


def _images_asset_config(op_config: Mapping[str, object] | None) -> dict[str, object]:
    config = dict(op_config or {})
    limit = config.get("limit")
    image_filenames = config.get("image_filenames", [])
    return {
        "limit": limit if isinstance(limit, int) and limit > 0 else None,
        "force_download": bool(config.get("force_download", False)),
        "image_filenames": [
            str(value).strip()
            for value in image_filenames
            if str(value).strip()
        ],
    }


def _ocr_asset_config(op_config: Mapping[str, object] | None) -> dict[str, object]:
    config = dict(op_config or {})
    limit = config.get("limit")
    image_filenames = config.get("image_filenames", [])
    max_concurrent_requests = config.get("max_concurrent_requests")
    return {
        "limit": limit if isinstance(limit, int) and limit > 0 else None,
        "force_ocr": bool(config.get("force_ocr", False)),
        "image_filenames": [
            str(value).strip()
            for value in image_filenames
            if str(value).strip()
        ],
        "max_concurrent_requests": (
            max_concurrent_requests
            if isinstance(max_concurrent_requests, int) and max_concurrent_requests > 0
            else None
        ),
    }


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
