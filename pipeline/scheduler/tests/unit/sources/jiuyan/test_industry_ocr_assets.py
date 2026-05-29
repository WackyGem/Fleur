from __future__ import annotations

import tempfile
import unittest
from typing import Any, cast

import pyarrow as pa
import pyarrow.fs as pafs
from scheduler.defs.sources.jiuyan.industry_ocr import discover_images_from_table
from scheduler.defs.sources.jiuyan.ocr_schema import ocr_rows_to_table
from scheduler.defs.storage.object_store import ImageObjectStore, ObjectStore


def local_filesystem() -> Any:
    return cast(Any, pafs).LocalFileSystem()


class JiuyanIndustryOcrAssetsTest(unittest.TestCase):
    def test_discover_images_from_table_counts_unique_filenames(self) -> None:
        table = pa.Table.from_pylist(
            [
                {
                    "industry_id": "industry-1",
                    "imgs": '["https://example.test/a/one.png"]',
                },
                {
                    "industry_id": "industry-2",
                    "imgs": '["https://example.test/a/one.png", "https://example.test/b/two.jpg"]',
                },
            ]
        )

        discovered, stats = discover_images_from_table(table)

        self.assertEqual(
            [image.image_filename for image in discovered], ["one.png", "one.png", "two.jpg"]
        )
        self.assertEqual(stats["article_count"], 2)
        self.assertEqual(stats["parsed_image_url_count"], 3)
        self.assertEqual(stats["unique_image_filename_count"], 2)

    def test_discover_images_from_table_rejects_conflicting_filenames(self) -> None:
        table = pa.Table.from_pylist(
            [
                {
                    "industry_id": "industry-1",
                    "imgs": '["https://a.example/path/one.png"]',
                },
                {
                    "industry_id": "industry-2",
                    "imgs": '["https://b.example/path/one.png"]',
                },
            ]
        )

        with self.assertRaises(RuntimeError):
            discover_images_from_table(table)

    def test_write_ocr_result_table_writes_single_parquet_file(self) -> None:
        table = ocr_rows_to_table(
            "industry-1",
            [{"stock_name": "A", "theme_path": "T", "relation": "R", "source": "S"}],
        )

        with tempfile.TemporaryDirectory() as bucket:
            image_store = ImageObjectStore(
                object_store=ObjectStore(
                    filesystem=local_filesystem(),
                    bucket=bucket,
                )
            )
            key = image_store.write_ocr_result_table("one.png", table)

        self.assertEqual(
            key,
            "raw/jiuyan__industry_ocr/image_filename=one.png/000000_0.parquet",
        )


if __name__ == "__main__":
    unittest.main()
