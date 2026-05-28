from __future__ import annotations

import unittest

from scheduler.defs.jiuyan_industry_ocr.ocr_client import (
    StockThemeSchema,
    build_ocr_request_payload,
)


class JiuyanIndustryOcrClientTest(unittest.TestCase):
    def test_build_ocr_request_payload_uses_stock_theme_json_schema(self) -> None:
        payload = build_ocr_request_payload("model", "data:image/png;base64,abc")

        response_format = payload["response_format"]
        self.assertEqual(response_format["type"], "json_schema")
        self.assertEqual(response_format["json_schema"]["name"], "stock_theme")
        self.assertEqual(response_format["json_schema"]["strict"], True)
        self.assertEqual(
            response_format["json_schema"]["schema"],
            StockThemeSchema.get_json_schema(),
        )
        self.assertEqual(response_format["json_schema"]["schema"]["type"], "array")


if __name__ == "__main__":
    unittest.main()
