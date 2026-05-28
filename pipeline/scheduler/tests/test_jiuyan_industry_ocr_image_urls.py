from __future__ import annotations

import unittest

from scheduler.defs.http_resources.jiuyan_image_urls import (
    image_filename_from_url,
    image_s3_key,
    parse_image_urls,
)


class JiuyanIndustryOcrImageUrlsTest(unittest.TestCase):
    def test_parse_image_urls_from_json_text(self) -> None:
        imgs = (
            '["https://example.test/a/one.png?x=1", '
            '"prefix https://example.test/b/two.jpg suffix", '
            '"https://example.test/a/one.png?x=1"]'
        )

        urls = parse_image_urls(imgs)

        self.assertEqual(
            urls,
            [
                "https://example.test/a/one.png?x=1",
                "https://example.test/b/two.jpg",
            ],
        )

    def test_parse_image_urls_falls_back_to_regex_for_invalid_json(self) -> None:
        urls = parse_image_urls("bad [https://example.test/a/one.jpeg]")

        self.assertEqual(urls, ["https://example.test/a/one.jpeg"])

    def test_image_filename_uses_path_basename_without_query_string(self) -> None:
        filename = image_filename_from_url("https://example.test/path/one.png?x=1")

        self.assertEqual(filename, "one.png")
        self.assertEqual(image_s3_key(filename), "img/jiuyan__industry_images/one.png")


if __name__ == "__main__":
    unittest.main()
