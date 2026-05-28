from __future__ import annotations

from dataclasses import dataclass

import dagster as dg
import pyarrow as pa

JIUYAN_INDUSTRY_IMAGES_ASSET_KEY = dg.AssetKey("jiuyan__industry_images")
JIUYAN_INDUSTRY_OCR_ASSET_KEY = dg.AssetKey("jiuyan__industry_ocr")
JIUYAN_INDUSTRY_OCR_S3_PREFIX = "raw/jiuyan__industry_ocr"

JIUYAN_INDUSTRY_OCR_SCHEMA = pa.schema(
    [
        pa.field("industry_id", pa.string(), nullable=False),
        pa.field("stock_name", pa.string(), nullable=False),
        pa.field("theme_path", pa.string(), nullable=False),
        pa.field("relation", pa.string(), nullable=False),
        pa.field("source", pa.string(), nullable=False),
    ]
)


@dataclass(frozen=True)
class DiscoveredIndustryImage:
    image_filename: str
    image_url: str
    industry_id: str
    image_index: int


@dataclass(frozen=True)
class ClaimedIndustryImage:
    image_filename: str
    image_url: str
    image_s3_key: str
    industry_id: str
    image_index: int
    download_status: str
    ocr_status: str
    ocr_result_s3_key: str | None = None


def ocr_result_base_dir(bucket: str, image_filename: str) -> str:
    return f"{bucket}/{JIUYAN_INDUSTRY_OCR_S3_PREFIX}/image_filename={image_filename}"


def ocr_result_s3_key(image_filename: str) -> str:
    return f"{JIUYAN_INDUSTRY_OCR_S3_PREFIX}/image_filename={image_filename}/000000_0.parquet"
