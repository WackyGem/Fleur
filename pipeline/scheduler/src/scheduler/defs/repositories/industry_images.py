from __future__ import annotations

from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Any, cast

import psycopg
from psycopg.rows import dict_row

from scheduler.defs.sources.jiuyan.image_urls import image_s3_key
from scheduler.defs.sources.jiuyan.ocr_schema import (
    ClaimedIndustryImage,
    DiscoveredIndustryImage,
    DownloadStatus,
    OcrStatus,
)

UPSERT_DISCOVERED_IMAGE_SQL = """
insert into jiuyan_industry_images (
    image_filename,
    image_url,
    image_s3_key,
    industry_id,
    image_index
)
values (%(image_filename)s, %(image_url)s, %(image_s3_key)s, %(industry_id)s, %(image_index)s)
on conflict (image_filename) do update set
    image_url = excluded.image_url,
    image_s3_key = excluded.image_s3_key,
    industry_id = coalesce(jiuyan_industry_images.industry_id, excluded.industry_id),
    image_index = coalesce(jiuyan_industry_images.image_index, excluded.image_index),
    updated_at = now()
"""

FETCH_EXISTING_URLS_SQL = """
select image_filename, image_url
from jiuyan_industry_images
where image_filename = any(%s)
"""

FETCH_IMAGES_SQL = """
select
    image_filename,
    image_url,
    image_s3_key,
    industry_id,
    image_index,
    download_status,
    ocr_status,
    ocr_result_s3_key
from jiuyan_industry_images
where image_filename = any(%s)
"""

MARK_DOWNLOAD_SUCCESS_SQL = """
update jiuyan_industry_images
set
    image_s3_key = %(image_s3_key)s,
    download_status = 'success',
    download_error_type = null,
    download_error_message = null,
    download_sha256 = %(download_sha256)s,
    download_bytes = %(download_bytes)s,
    downloaded_at = now(),
    updated_at = now()
where image_filename = %(image_filename)s
"""

MARK_DOWNLOAD_FAILED_SQL = """
update jiuyan_industry_images
set
    download_status = 'failed',
    download_error_type = %(download_error_type)s,
    download_error_message = %(download_error_message)s,
    updated_at = now()
where image_filename = %(image_filename)s
"""

MARK_OCR_SUCCESS_SQL = """
update jiuyan_industry_images
set
    ocr_status = 'success',
    ocr_error_type = null,
    ocr_error_message = null,
    ocr_result_s3_key = %(ocr_result_s3_key)s,
    ocr_result_row_count = %(ocr_result_row_count)s,
    ocr_model = %(ocr_model)s,
    ocr_started_at = coalesce(ocr_started_at, now()),
    ocr_completed_at = now(),
    updated_at = now()
where image_filename = %(image_filename)s
"""

MARK_OCR_FAILED_SQL = """
update jiuyan_industry_images
set
    ocr_status = 'failed',
    ocr_error_type = %(ocr_error_type)s,
    ocr_error_message = %(ocr_error_message)s,
    ocr_completed_at = now(),
    updated_at = now()
where image_filename = %(image_filename)s
"""

RESET_OCR_STATUS_SQL = """
update jiuyan_industry_images
set
    ocr_status = 'pending',
    ocr_error_type = null,
    ocr_error_message = null,
    ocr_started_at = null,
    ocr_completed_at = null,
    ocr_result_s3_key = null,
    ocr_result_row_count = null,
    ocr_model = null,
    updated_at = now()
where image_filename = any(%s)
  and download_status = 'success'
"""

CLAIM_OCR_IMAGES_SQL = """
with candidates as (
    select image_filename
    from jiuyan_industry_images
    where download_status = 'success'
      and (
        ocr_status in ('pending', 'failed')
        or (
          %(force_ocr)s
          and ocr_status = 'success'
        )
        or (
          ocr_status = 'running'
          and ocr_started_at < now() - (%(stale_after_seconds)s * interval '1 second')
        )
      )
{selected_clause}
    order by image_filename
    for update skip locked
    limit %(limit_value)s
), updated as (
    update jiuyan_industry_images images
    set
        ocr_status = 'running',
        ocr_error_type = null,
        ocr_error_message = null,
        ocr_started_at = now(),
        updated_at = now()
    from candidates
    where images.image_filename = candidates.image_filename
    returning
        images.image_filename,
        images.image_url,
        images.image_s3_key,
        images.industry_id,
        images.image_index,
        images.download_status,
        'running'::text as ocr_status,
        images.ocr_result_s3_key
)
select *
from updated
order by image_filename
"""

LIST_SUCCESSFUL_OCR_RESULTS_SQL = """
select
    image_filename,
    industry_id,
    image_index,
    ocr_result_s3_key,
    ocr_result_row_count
from jiuyan_industry_images
where download_status = 'success'
  and ocr_status = 'success'
  and ocr_result_s3_key is not null
order by image_filename
"""

SUMMARIZE_OCR_STATUS_SQL = """
select
    count(*) filter (where download_status = 'success') as download_success_count,
    count(*) filter (where download_status = 'success' and ocr_status = 'success')
        as ocr_success_count,
    count(*) filter (where download_status = 'success' and ocr_status = 'failed')
        as ocr_failed_count,
    count(*) filter (where download_status = 'success' and ocr_status = 'pending')
        as ocr_pending_count,
    count(*) filter (where download_status = 'success' and ocr_status = 'running')
        as ocr_running_count,
    coalesce(
        sum(ocr_result_row_count) filter (
            where download_status = 'success'
              and ocr_status = 'success'
              and ocr_result_s3_key is not null
        ),
        0
    ) as ocr_success_result_row_count
from jiuyan_industry_images
"""


DatabaseRow = Mapping[str, object]
PipelineDatabaseConnectionFactory = Callable[[], psycopg.Connection]


@dataclass(frozen=True)
class IndustryImageRecord:
    image_filename: str
    image_url: str
    image_s3_key: str
    industry_id: str
    image_index: int
    download_status: DownloadStatus
    ocr_status: OcrStatus
    ocr_result_s3_key: str | None


@dataclass(frozen=True)
class SuccessfulOcrResultRecord:
    image_filename: str
    industry_id: str
    image_index: int
    ocr_result_s3_key: str
    ocr_result_row_count: int


@dataclass(frozen=True)
class OcrStatusSummary:
    download_success_count: int
    ocr_success_count: int
    ocr_failed_count: int
    ocr_pending_count: int
    ocr_running_count: int
    ocr_success_result_row_count: int


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


def connect_pipeline_database(url: str) -> psycopg.Connection:
    return psycopg.connect(url, row_factory=cast(Any, dict_row))


def connection_factory_from_url(url: str) -> PipelineDatabaseConnectionFactory:
    return lambda: connect_pipeline_database(url)


class PostgresIndustryImageRepository:
    def __init__(self, connection_factory: PipelineDatabaseConnectionFactory) -> None:
        self._connection_factory = connection_factory

    def fetch_existing_image_urls(
        self,
        image_filenames: Sequence[str],
    ) -> dict[str, str]:
        if not image_filenames:
            return {}
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.execute(FETCH_EXISTING_URLS_SQL, (list(image_filenames),))
            rows = cast(list[DatabaseRow], cursor.fetchall())
        return {
            str(row["image_filename"]): str(row["image_url"])
            for row in rows
            if row.get("image_filename") is not None and row.get("image_url") is not None
        }

    def fetch_images(
        self,
        image_filenames: Sequence[str],
    ) -> list[IndustryImageRecord]:
        if not image_filenames:
            return []
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.execute(FETCH_IMAGES_SQL, (list(image_filenames),))
            rows = cast(list[DatabaseRow], cursor.fetchall())
        return [_industry_image_record_from_row(row) for row in rows]

    def upsert_discovered_images(self, images: Sequence[DiscoveredIndustryImage]) -> int:
        if not images:
            return 0
        params = [
            {
                "image_filename": image.image_filename,
                "image_url": image.image_url,
                "image_s3_key": image_s3_key(image.image_filename),
                "industry_id": image.industry_id,
                "image_index": image.image_index,
            }
            for image in images
        ]
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.executemany(UPSERT_DISCOVERED_IMAGE_SQL, params)
            return int(cursor.rowcount)

    def mark_download_success(
        self,
        *,
        image_filename: str,
        image_s3_key_value: str,
        download_sha256: str,
        download_bytes: int,
    ) -> None:
        self.mark_download_success_many(
            [
                DownloadSuccessUpdate(
                    image_filename=image_filename,
                    image_s3_key=image_s3_key_value,
                    download_sha256=download_sha256,
                    download_bytes=download_bytes,
                )
            ]
        )

    def mark_download_success_many(self, updates: Sequence[DownloadSuccessUpdate]) -> int:
        if not updates:
            return 0
        params = [
            {
                "image_filename": update.image_filename,
                "image_s3_key": update.image_s3_key,
                "download_sha256": update.download_sha256,
                "download_bytes": update.download_bytes,
            }
            for update in updates
        ]
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.executemany(MARK_DOWNLOAD_SUCCESS_SQL, params)
            return int(cursor.rowcount)

    def mark_download_failed(
        self,
        *,
        image_filename: str,
        error_type: str,
        error_message: str,
    ) -> None:
        self.mark_download_failed_many(
            [
                DownloadFailureUpdate(
                    image_filename=image_filename,
                    error_type=error_type,
                    error_message=error_message,
                )
            ]
        )

    def mark_download_failed_many(self, updates: Sequence[DownloadFailureUpdate]) -> int:
        if not updates:
            return 0
        params = [
            {
                "image_filename": update.image_filename,
                "download_error_type": update.error_type,
                "download_error_message": update.error_message,
            }
            for update in updates
        ]
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.executemany(MARK_DOWNLOAD_FAILED_SQL, params)
            return int(cursor.rowcount)

    def mark_ocr_success(
        self,
        *,
        image_filename: str,
        ocr_result_s3_key_value: str,
        ocr_result_row_count: int,
        ocr_model: str,
    ) -> None:
        self.mark_ocr_success_many(
            [
                OcrSuccessUpdate(
                    image_filename=image_filename,
                    ocr_result_s3_key=ocr_result_s3_key_value,
                    ocr_result_row_count=ocr_result_row_count,
                    ocr_model=ocr_model,
                )
            ]
        )

    def mark_ocr_success_many(self, updates: Sequence[OcrSuccessUpdate]) -> int:
        if not updates:
            return 0
        params = [
            {
                "image_filename": update.image_filename,
                "ocr_result_s3_key": update.ocr_result_s3_key,
                "ocr_result_row_count": update.ocr_result_row_count,
                "ocr_model": update.ocr_model,
            }
            for update in updates
        ]
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.executemany(MARK_OCR_SUCCESS_SQL, params)
            return int(cursor.rowcount)

    def mark_ocr_failed(
        self,
        *,
        image_filename: str,
        error_type: str,
        error_message: str,
    ) -> None:
        self.mark_ocr_failed_many(
            [
                OcrFailureUpdate(
                    image_filename=image_filename,
                    error_type=error_type,
                    error_message=error_message,
                )
            ]
        )

    def mark_ocr_failed_many(self, updates: Sequence[OcrFailureUpdate]) -> int:
        if not updates:
            return 0
        params = [
            {
                "image_filename": update.image_filename,
                "ocr_error_type": update.error_type,
                "ocr_error_message": update.error_message,
            }
            for update in updates
        ]
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.executemany(MARK_OCR_FAILED_SQL, params)
            return int(cursor.rowcount)

    def reset_ocr_status(self, image_filenames: Sequence[str]) -> int:
        if not image_filenames:
            return 0
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.execute(RESET_OCR_STATUS_SQL, (list(image_filenames),))
            return int(cursor.rowcount)

    def claim_ocr_images(
        self,
        *,
        limit: int | None,
        image_filenames: Sequence[str] | None,
        stale_after_seconds: int,
        force_ocr: bool = False,
    ) -> list[ClaimedIndustryImage]:
        if limit is not None and limit < 1:
            msg = "limit must be positive when provided"
            raise ValueError(msg)
        if stale_after_seconds < 0:
            msg = "stale_after_seconds must be non-negative"
            raise ValueError(msg)

        only_selected = bool(image_filenames)
        limit_value = limit if limit is not None else 10_000_000
        selected_clause = (
            "      and image_filename = any(%(image_filenames)s)\n" if only_selected else ""
        )
        sql = CLAIM_OCR_IMAGES_SQL.format(selected_clause=selected_clause)
        params = {
            "stale_after_seconds": stale_after_seconds,
            "force_ocr": force_ocr,
            "image_filenames": list(image_filenames or []),
            "limit_value": limit_value,
        }
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.execute(sql, params)
            rows = cast(list[DatabaseRow], cursor.fetchall())
        claimed: list[ClaimedIndustryImage] = []
        for row in rows:
            claimed.append(
                ClaimedIndustryImage(
                    image_filename=str(row["image_filename"]),
                    image_url=str(row["image_url"]),
                    image_s3_key=str(row["image_s3_key"]),
                    industry_id=str(row["industry_id"]),
                    image_index=_required_int(row["image_index"], field_name="image_index"),
                    download_status=_download_status_from_row(row, "download_status"),
                    ocr_status=_ocr_status_from_row(row, "ocr_status"),
                    ocr_result_s3_key=(
                        None
                        if row.get("ocr_result_s3_key") is None
                        else str(row["ocr_result_s3_key"])
                    ),
                )
            )
        return claimed

    def list_successful_ocr_results(self) -> list[SuccessfulOcrResultRecord]:
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.execute(LIST_SUCCESSFUL_OCR_RESULTS_SQL)
            rows = cast(list[DatabaseRow], cursor.fetchall())
        return [_successful_ocr_result_record_from_row(row) for row in rows]

    def summarize_ocr_status(self) -> OcrStatusSummary:
        with self._connection_factory() as connection, connection.cursor() as cursor:
            cursor.execute(SUMMARIZE_OCR_STATUS_SQL)
            row = cast(DatabaseRow, cursor.fetchone())
        return _ocr_status_summary_from_row(row)


def _industry_image_record_from_row(row: DatabaseRow) -> IndustryImageRecord:
    return IndustryImageRecord(
        image_filename=str(row["image_filename"]),
        image_url=str(row["image_url"]),
        image_s3_key=str(row["image_s3_key"]),
        industry_id=str(row["industry_id"]),
        image_index=_required_int(row["image_index"], field_name="image_index"),
        download_status=_download_status_from_row(row, "download_status"),
        ocr_status=_ocr_status_from_row(row, "ocr_status"),
        ocr_result_s3_key=(
            None if row.get("ocr_result_s3_key") is None else str(row["ocr_result_s3_key"])
        ),
    )


def _successful_ocr_result_record_from_row(row: DatabaseRow) -> SuccessfulOcrResultRecord:
    result_key = row.get("ocr_result_s3_key")
    if result_key is None:
        msg = "ocr_result_s3_key must be present for successful OCR result records"
        raise RuntimeError(msg)
    return SuccessfulOcrResultRecord(
        image_filename=str(row["image_filename"]),
        industry_id=str(row["industry_id"]),
        image_index=_required_int(row["image_index"], field_name="image_index"),
        ocr_result_s3_key=str(result_key),
        ocr_result_row_count=_required_int(
            row["ocr_result_row_count"],
            field_name="ocr_result_row_count",
        ),
    )


def _ocr_status_summary_from_row(row: DatabaseRow) -> OcrStatusSummary:
    return OcrStatusSummary(
        download_success_count=_required_int(
            row["download_success_count"],
            field_name="download_success_count",
        ),
        ocr_success_count=_required_int(row["ocr_success_count"], field_name="ocr_success_count"),
        ocr_failed_count=_required_int(row["ocr_failed_count"], field_name="ocr_failed_count"),
        ocr_pending_count=_required_int(row["ocr_pending_count"], field_name="ocr_pending_count"),
        ocr_running_count=_required_int(row["ocr_running_count"], field_name="ocr_running_count"),
        ocr_success_result_row_count=_required_int(
            row["ocr_success_result_row_count"],
            field_name="ocr_success_result_row_count",
        ),
    )


def _required_int(value: object, *, field_name: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        msg = f"{field_name} must be an integer"
        raise RuntimeError(msg)
    return value


def _download_status_from_row(row: DatabaseRow, field_name: str) -> DownloadStatus:
    value = row[field_name]
    if isinstance(value, DownloadStatus):
        return value
    if isinstance(value, str):
        try:
            return DownloadStatus(value)
        except ValueError as error:
            msg = f"{field_name} has invalid download status value: {value!r}"
            raise RuntimeError(msg) from error
    msg = f"{field_name} has invalid download status value: {value!r}"
    raise RuntimeError(msg)


def _ocr_status_from_row(row: DatabaseRow, field_name: str) -> OcrStatus:
    value = row[field_name]
    if isinstance(value, OcrStatus):
        return value
    if isinstance(value, str):
        try:
            return OcrStatus(value)
        except ValueError as error:
            msg = f"{field_name} has invalid OCR status value: {value!r}"
            raise RuntimeError(msg) from error
    msg = f"{field_name} has invalid OCR status value: {value!r}"
    raise RuntimeError(msg)
