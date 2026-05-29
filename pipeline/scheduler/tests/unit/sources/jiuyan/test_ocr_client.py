from __future__ import annotations

import unittest
from collections.abc import Mapping

from scheduler.defs.sources.jiuyan.ocr_client import (
    StockThemeSchema,
    build_ocr_request_payload,
)


class JiuyanIndustryOcrClientTest(unittest.TestCase):
    def test_build_ocr_request_payload_uses_stock_theme_json_schema(self) -> None:
        payload = build_ocr_request_payload("model", "data:image/png;base64,abc")

        response_format = payload["response_format"]
        self.assertIsInstance(response_format, Mapping)
        assert isinstance(response_format, Mapping)
        json_schema = response_format["json_schema"]
        self.assertIsInstance(json_schema, Mapping)
        assert isinstance(json_schema, Mapping)
        schema = json_schema["schema"]
        self.assertIsInstance(schema, Mapping)
        assert isinstance(schema, Mapping)

        self.assertEqual(response_format["type"], "json_schema")
        self.assertEqual(json_schema["name"], "stock_theme")
        self.assertEqual(json_schema["strict"], True)
        self.assertEqual(
            schema,
            StockThemeSchema.get_json_schema(),
        )
        self.assertEqual(schema["type"], "array")


if __name__ == "__main__":
    unittest.main()
