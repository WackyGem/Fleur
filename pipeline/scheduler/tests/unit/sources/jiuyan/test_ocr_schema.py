from __future__ import annotations

import json
import unittest

from scheduler.defs.ocr.schemas import OcrSchemaError
from scheduler.defs.sources.jiuyan.ocr_schema import (
    JIUYAN_INDUSTRY_OCR_SCHEMA,
    normalize_ocr_content,
    ocr_rows_to_table,
)


class JiuyanIndustryOcrSchemaTest(unittest.TestCase):
    def test_normalize_ocr_content_maps_aliases_and_deduplicates_rows(self) -> None:
        content = json.dumps(
            [
                {
                    "个股": " 示例股份 ",
                    "主题": ["机器人", "减速器"],
                    "说明": " 核心供应商 ",
                    "来源": "公告",
                    "ignored": "value",
                },
                {
                    "stock_name": "示例股份",
                    "theme_path": "机器人 > 减速器",
                    "relation": "核心供应商",
                    "source": "公告",
                },
                {"stock_name": "", "theme_path": "", "relation": "", "source": ""},
            ],
            ensure_ascii=False,
        )

        rows = normalize_ocr_content(content)

        self.assertEqual(
            rows,
            [
                {
                    "stock_name": "示例股份",
                    "theme_path": "机器人,减速器",
                    "relation": "核心供应商",
                    "source": "公告",
                }
            ],
        )

    def test_normalize_theme_path_does_not_split_internal_spaces(self) -> None:
        content = json.dumps(
            [
                {
                    "stock_name": "江海股份",
                    "theme_path": "MLPC 叠层片式固态铝电解电容器",
                    "relation": "产能规划",
                    "source": "券商纪要",
                }
            ],
            ensure_ascii=False,
        )

        rows = normalize_ocr_content(content)

        self.assertEqual(rows[0]["theme_path"], "MLPC 叠层片式固态铝电解电容器")

    def test_normalize_ocr_content_rejects_non_array_json(self) -> None:
        with self.assertRaises(OcrSchemaError):
            normalize_ocr_content('{"stock_name": "x"}')

    def test_normalize_ocr_content_rejects_object_wrapper(self) -> None:
        with self.assertRaises(OcrSchemaError):
            normalize_ocr_content('{"rows": []}')

    def test_ocr_rows_to_table_uses_fixed_schema(self) -> None:
        table = ocr_rows_to_table(
            "industry-1",
            [{"stock_name": "A", "theme_path": "T", "relation": "R", "source": "S"}],
        )

        self.assertEqual(table.schema, JIUYAN_INDUSTRY_OCR_SCHEMA)
        self.assertEqual(table.to_pylist()[0]["industry_id"], "industry-1")


if __name__ == "__main__":
    unittest.main()
