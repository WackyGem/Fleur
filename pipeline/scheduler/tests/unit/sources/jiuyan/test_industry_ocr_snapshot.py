from __future__ import annotations

import tempfile
from dataclasses import dataclass
from typing import Any, cast

import pyarrow as pa
import pyarrow.fs as pafs
import pytest
from scheduler.defs.config.models import S3Config
from scheduler.defs.repositories.industry_images import (
    OcrStatusSummary,
    SuccessfulOcrResultRecord,
)
from scheduler.defs.sources.jiuyan.image_object_store import ImageObjectStore
from scheduler.defs.sources.jiuyan.industry_ocr_snapshot import (
    JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA,
    build_industry_ocr_snapshot,
)
from scheduler.defs.sources.jiuyan.ocr_schema import ocr_rows_to_table
from scheduler.defs.storage.object_store import ObjectStore


def local_filesystem() -> Any:
    return cast(Any, pafs).LocalFileSystem()


@dataclass
class FakeRepository:
    records: list[SuccessfulOcrResultRecord]
    summary: OcrStatusSummary

    def list_successful_ocr_results(self) -> list[SuccessfulOcrResultRecord]:
        return self.records

    def summarize_ocr_status(self) -> OcrStatusSummary:
        return self.summary


@dataclass
class FakeObjectStore:
    tables: dict[str, pa.Table]

    def read_ocr_result_table(self, key: str) -> pa.Table:
        if key not in self.tables:
            msg = f"missing key: {key}"
            raise RuntimeError(msg)
        return self.tables[key]


def success_record(
    image_filename: str,
    *,
    industry_id: str = "industry-1",
    image_index: int = 0,
    row_count: int = 1,
) -> SuccessfulOcrResultRecord:
    return SuccessfulOcrResultRecord(
        image_filename=image_filename,
        industry_id=industry_id,
        image_index=image_index,
        ocr_result_s3_key=f"source/jiuyan__industry_ocr/image_filename={image_filename}/000000_0.parquet",
        ocr_result_row_count=row_count,
    )


def summary(*, result_row_count: int, success_count: int = 1) -> OcrStatusSummary:
    return OcrStatusSummary(
        download_success_count=success_count,
        ocr_success_count=success_count,
        ocr_failed_count=1,
        ocr_pending_count=2,
        ocr_running_count=3,
        ocr_success_result_row_count=result_row_count,
    )


def test_build_industry_ocr_snapshot_merges_per_image_tables_with_lineage_columns() -> None:
    first = success_record("first.png", industry_id="industry-1", image_index=0, row_count=2)
    second = success_record("second.png", industry_id="industry-2", image_index=1, row_count=1)
    result = build_industry_ocr_snapshot(
        repository=FakeRepository(
            records=[first, second],
            summary=summary(result_row_count=3, success_count=2),
        ),
        object_store=FakeObjectStore(
            tables={
                first.ocr_result_s3_key: ocr_rows_to_table(
                    "industry-1",
                    [
                        {
                            "stock_name": "A",
                            "theme_path": "T1",
                            "relation": "R1",
                            "source": "S1",
                        },
                        {
                            "stock_name": "B",
                            "theme_path": "T2",
                            "relation": "R2",
                            "source": "S2",
                        },
                    ],
                ),
                second.ocr_result_s3_key: ocr_rows_to_table(
                    "industry-2",
                    [
                        {
                            "stock_name": "C",
                            "theme_path": "T3",
                            "relation": "R3",
                            "source": "S3",
                        }
                    ],
                ),
            }
        ),
    )

    assert result.table.schema == JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA
    assert result.table.to_pylist() == [
        {
            "industry_id": "industry-1",
            "image_filename": "first.png",
            "image_index": 0,
            "ocr_row_index": 0,
            "stock_name": "A",
            "theme_path": "T1",
            "relation": "R1",
            "source": "S1",
        },
        {
            "industry_id": "industry-1",
            "image_filename": "first.png",
            "image_index": 0,
            "ocr_row_index": 1,
            "stock_name": "B",
            "theme_path": "T2",
            "relation": "R2",
            "source": "S2",
        },
        {
            "industry_id": "industry-2",
            "image_filename": "second.png",
            "image_index": 1,
            "ocr_row_index": 0,
            "stock_name": "C",
            "theme_path": "T3",
            "relation": "R3",
            "source": "S3",
        },
    ]
    assert result.metadata["snapshot_row_count"] == 3
    assert result.metadata["successful_image_count"] == 2
    assert result.metadata["ocr_pending_count"] == 2
    assert result.metadata["ocr_failed_count"] == 1
    assert result.metadata["ocr_running_count"] == 3
    assert "snapshot_schema_version" not in result.metadata


def test_build_industry_ocr_snapshot_counts_zero_row_success_images() -> None:
    empty_record = success_record("empty.png", industry_id="industry-empty", row_count=0)
    row_record = success_record("row.png", industry_id="industry-row", row_count=1)

    result = build_industry_ocr_snapshot(
        repository=FakeRepository(
            records=[empty_record, row_record],
            summary=summary(result_row_count=1, success_count=2),
        ),
        object_store=FakeObjectStore(
            tables={
                empty_record.ocr_result_s3_key: ocr_rows_to_table("industry-empty", []),
                row_record.ocr_result_s3_key: ocr_rows_to_table(
                    "industry-row",
                    [
                        {
                            "stock_name": "A",
                            "theme_path": "T",
                            "relation": "R",
                            "source": "S",
                        }
                    ],
                ),
            }
        ),
    )

    assert result.table.num_rows == 1
    assert result.metadata["zero_row_image_count"] == 1
    assert result.metadata["ocr_result_file_count"] == 2


def test_build_industry_ocr_snapshot_allows_all_success_images_to_have_zero_rows() -> None:
    empty_record = success_record("empty.png", industry_id="industry-empty", row_count=0)

    result = build_industry_ocr_snapshot(
        repository=FakeRepository(
            records=[empty_record],
            summary=summary(result_row_count=0, success_count=1),
        ),
        object_store=FakeObjectStore(
            tables={
                empty_record.ocr_result_s3_key: ocr_rows_to_table("industry-empty", []),
            }
        ),
    )

    assert result.table.schema == JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA
    assert result.table.num_rows == 0
    assert result.metadata["zero_row_image_count"] == 1


def test_build_industry_ocr_snapshot_fails_when_no_success_records_exist() -> None:
    with pytest.raises(RuntimeError, match="No successful JiuYan OCR result records"):
        build_industry_ocr_snapshot(
            repository=FakeRepository(
                records=[], summary=summary(result_row_count=0, success_count=0)
            ),
            object_store=FakeObjectStore(tables={}),
        )


def test_build_industry_ocr_snapshot_fails_when_s3_key_is_missing() -> None:
    record = success_record("missing.png", row_count=1)

    with pytest.raises(RuntimeError, match="missing key"):
        build_industry_ocr_snapshot(
            repository=FakeRepository(records=[record], summary=summary(result_row_count=1)),
            object_store=FakeObjectStore(tables={}),
        )


def test_build_industry_ocr_snapshot_fails_on_incompatible_schema() -> None:
    record = success_record("bad.png", row_count=1)
    bad_table = pa.table(
        {
            "industry_id": ["industry-1"],
            "stock_name": ["A"],
            "theme_path": ["T"],
            "relation": ["R"],
        }
    )

    with pytest.raises(RuntimeError, match="missing fields"):
        build_industry_ocr_snapshot(
            repository=FakeRepository(records=[record], summary=summary(result_row_count=1)),
            object_store=FakeObjectStore(tables={record.ocr_result_s3_key: bad_table}),
        )


def test_build_industry_ocr_snapshot_fails_when_snapshot_row_count_mismatches_postgres() -> None:
    record = success_record("one.png", row_count=1)

    with pytest.raises(RuntimeError, match="snapshot row count does not match"):
        build_industry_ocr_snapshot(
            repository=FakeRepository(records=[record], summary=summary(result_row_count=2)),
            object_store=FakeObjectStore(
                tables={
                    record.ocr_result_s3_key: ocr_rows_to_table(
                        "industry-1",
                        [
                            {
                                "stock_name": "A",
                                "theme_path": "T",
                                "relation": "R",
                                "source": "S",
                            }
                        ],
                    )
                }
            ),
        )


def test_image_object_store_reads_ocr_result_table_by_object_key() -> None:
    table = ocr_rows_to_table(
        "industry-1",
        [{"stock_name": "A", "theme_path": "T", "relation": "R", "source": "S"}],
    )

    with tempfile.TemporaryDirectory() as bucket:
        image_store = ImageObjectStore(
            object_store=ObjectStore(
                filesystem=local_filesystem(),
                bucket=bucket,
                s3_config=S3Config(
                    endpoint="http://localhost:9000",
                    bucket=bucket,
                    access_key="access",
                    secret_key="secret",
                ),
            )
        )
        key = image_store.write_ocr_result_table("one.png", table)
        read_table = image_store.read_ocr_result_table(key)

    assert read_table.schema == table.schema
    assert read_table.to_pylist() == table.to_pylist()
