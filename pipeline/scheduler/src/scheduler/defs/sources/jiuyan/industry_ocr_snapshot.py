from __future__ import annotations

import time
from dataclasses import dataclass
from typing import Protocol

import dagster as dg
import pyarrow as pa

from scheduler.defs.asset_contracts import (
    METADATA_ALLOW_EMPTY,
    latest_snapshot_metadata,
    s3_parquet_kinds,
    source_owners,
    source_tags,
)
from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.repositories.industry_images import (
    OcrStatusSummary,
    SuccessfulOcrResultRecord,
)
from scheduler.defs.resources.database import IndustryImageRepositoryResource
from scheduler.defs.resources.s3 import ImageObjectStoreResource
from scheduler.defs.sources.jiuyan.industry_ocr import jiuyan__industry_ocr
from scheduler.defs.sources.jiuyan.ocr_schema import JIUYAN_INDUSTRY_OCR_SCHEMA

__all__ = [
    "JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA",
    "SNAPSHOT_SCHEMA_VERSION",
    "IndustryOcrSnapshotBuildResult",
    "build_industry_ocr_snapshot",
    "jiuyan__industry_ocr_snapshot",
]

SNAPSHOT_SCHEMA_VERSION = 1

JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA = pa.schema(
    [
        pa.field("industry_id", pa.string(), nullable=False),
        pa.field("image_filename", pa.string(), nullable=False),
        pa.field("image_index", pa.int32(), nullable=False),
        pa.field("ocr_row_index", pa.int32(), nullable=False),
        pa.field("stock_name", pa.string(), nullable=False),
        pa.field("theme_path", pa.string(), nullable=False),
        pa.field("relation", pa.string(), nullable=False),
        pa.field("source", pa.string(), nullable=False),
    ]
)


class OcrSnapshotRepository(Protocol):
    def list_successful_ocr_results(self) -> list[SuccessfulOcrResultRecord]: ...

    def summarize_ocr_status(self) -> OcrStatusSummary: ...


class OcrResultObjectStore(Protocol):
    def read_ocr_result_table(self, key: str) -> pa.Table: ...


@dataclass(frozen=True)
class IndustryOcrSnapshotBuildResult:
    table: pa.Table
    metadata: dict[str, RawMetadataValue]


@dg.asset(
    name="jiuyan__industry_ocr_snapshot",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    deps=[jiuyan__industry_ocr],
    io_manager_key="s3_io_manager",
    description="Latest snapshot of successfully extracted JiuYan industry OCR result rows.",
    metadata=latest_snapshot_metadata(extra={METADATA_ALLOW_EMPTY: True}),
    owners=source_owners(),
    kinds=s3_parquet_kinds("postgres", "ocr", "snapshot"),
    tags=source_tags("jiuyan"),
)
def jiuyan__industry_ocr_snapshot(
    image_object_store: ImageObjectStoreResource,
    industry_image_repository: IndustryImageRepositoryResource,
) -> dg.MaterializeResult[pa.Table]:
    result = build_industry_ocr_snapshot(
        repository=industry_image_repository.repository(),
        object_store=image_object_store.image_object_store(),
    )
    return dg.MaterializeResult(value=result.table, metadata=result.metadata)


def build_industry_ocr_snapshot(
    *,
    repository: OcrSnapshotRepository,
    object_store: OcrResultObjectStore,
) -> IndustryOcrSnapshotBuildResult:
    started_at = time.perf_counter()
    records = repository.list_successful_ocr_results()
    if not records:
        msg = "No successful JiuYan OCR result records are available for snapshot publishing"
        raise RuntimeError(msg)

    summary = repository.summarize_ocr_status()
    snapshot_tables: list[pa.Table] = []
    zero_row_image_count = 0

    for record in records:
        table = _validated_ocr_result_table(
            object_store.read_ocr_result_table(record.ocr_result_s3_key),
            record=record,
        )
        if table.num_rows == 0:
            zero_row_image_count += 1
            continue
        snapshot_tables.append(_add_snapshot_columns(table, record=record))

    if snapshot_tables:
        snapshot_table = pa.concat_tables(snapshot_tables, promote_options="none")
    else:
        snapshot_table = pa.Table.from_arrays(
            [pa.array([], type=field.type) for field in JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA],
            schema=JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA,
        )

    if snapshot_table.num_rows != summary.ocr_success_result_row_count:
        msg = (
            "JiuYan OCR snapshot row count does not match Postgres success row count: "
            f"snapshot_row_count={snapshot_table.num_rows}, "
            f"ocr_success_result_row_count={summary.ocr_success_result_row_count}"
        )
        raise RuntimeError(msg)

    metadata: dict[str, RawMetadataValue] = {
        "snapshot_row_count": snapshot_table.num_rows,
        "successful_image_count": len(records),
        "zero_row_image_count": zero_row_image_count,
        "ocr_result_file_count": len(records),
        "ocr_pending_count": summary.ocr_pending_count,
        "ocr_failed_count": summary.ocr_failed_count,
        "ocr_running_count": summary.ocr_running_count,
        "ocr_success_result_row_count": summary.ocr_success_result_row_count,
        "snapshot_schema_version": SNAPSHOT_SCHEMA_VERSION,
        "s3_keys_sample": dg.MetadataValue.json(
            [record.ocr_result_s3_key for record in records[:20]]
        ),
        "asset_function_seconds": elapsed_seconds(started_at, time.perf_counter()),
    }
    return IndustryOcrSnapshotBuildResult(table=snapshot_table, metadata=metadata)


def _validated_ocr_result_table(
    table: pa.Table,
    *,
    record: SuccessfulOcrResultRecord,
) -> pa.Table:
    missing_fields = [
        field_name
        for field_name in JIUYAN_INDUSTRY_OCR_SCHEMA.names
        if field_name not in table.schema.names
    ]
    if missing_fields:
        msg = f"OCR result table {record.ocr_result_s3_key} is missing fields: {missing_fields}"
        raise RuntimeError(msg)

    selected = table.select(JIUYAN_INDUSTRY_OCR_SCHEMA.names)
    for field in JIUYAN_INDUSTRY_OCR_SCHEMA:
        actual = selected.schema.field(field.name)
        if not pa.types.is_string(actual.type):
            msg = (
                f"OCR result table {record.ocr_result_s3_key} has incompatible field "
                f"{field.name}: expected string, got {actual.type}"
            )
            raise RuntimeError(msg)

    try:
        return selected.cast(JIUYAN_INDUSTRY_OCR_SCHEMA)
    except Exception as error:
        msg = f"OCR result table {record.ocr_result_s3_key} has incompatible schema"
        raise RuntimeError(msg) from error


def _add_snapshot_columns(
    table: pa.Table,
    *,
    record: SuccessfulOcrResultRecord,
) -> pa.Table:
    row_count = table.num_rows
    return pa.Table.from_arrays(
        [
            table.column("industry_id"),
            pa.array([record.image_filename] * row_count, type=pa.string()),
            pa.array([record.image_index] * row_count, type=pa.int32()),
            pa.array(list(range(row_count)), type=pa.int32()),
            table.column("stock_name"),
            table.column("theme_path"),
            table.column("relation"),
            table.column("source"),
        ],
        schema=JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA,
    )
