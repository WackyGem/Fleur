from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

import dagster as dg
import pyarrow as pa

from scheduler.defs.ocr.schemas import parse_json_array, require_mapping_row

THEME_PATH_DELIMITER = ","
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


OCR_FIELD_ALIASES = {
    "stock_name": ("stock_name", "个股", "公司", "标的"),
    "theme_path": ("theme_path", "theme", "topic", "主题"),
    "relation": ("relation", "relevance", "相关性", "说明", "业务说明"),
    "source": ("source", "信源", "资料来源", "来源"),
}


def normalize_ocr_content(content: str) -> list[dict[str, str]]:
    payload = parse_json_array(content)
    rows: list[dict[str, str]] = []
    seen: set[tuple[str, str, str, str]] = set()
    for index, item in enumerate(payload):
        row = _normalize_row(require_mapping_row(item, index=index))
        signature = (
            row["stock_name"],
            row["theme_path"],
            row["relation"],
            row["source"],
        )
        if signature == ("", "", "", ""):
            continue
        if signature in seen:
            continue
        seen.add(signature)
        rows.append(row)

    return rows


def ocr_rows_to_table(industry_id: str, rows: Sequence[Mapping[str, str]]) -> pa.Table:
    if not industry_id.strip():
        msg = "industry_id must be non-empty"
        raise ValueError(msg)

    column_names = ["industry_id", *JIUYAN_INDUSTRY_OCR_SCHEMA.names[1:]]
    columns: dict[str, list[str]] = {column_name: [] for column_name in column_names}
    for row in rows:
        columns["industry_id"].append(industry_id)
        for field_name in JIUYAN_INDUSTRY_OCR_SCHEMA.names[1:]:
            columns[field_name].append(_coerce_string(row.get(field_name)))

    arrays = [pa.array(columns[name], type=pa.string()) for name in column_names]
    return pa.Table.from_arrays(arrays, schema=JIUYAN_INDUSTRY_OCR_SCHEMA)


def _normalize_row(row: Mapping[str, object]) -> dict[str, str]:
    normalized = {
        "stock_name": _coerce_string(_first_non_empty_value(row, OCR_FIELD_ALIASES["stock_name"])),
        "theme_path": _normalize_theme_path(
            _first_non_empty_value(row, OCR_FIELD_ALIASES["theme_path"])
        ),
        "relation": _coerce_string(_first_non_empty_value(row, OCR_FIELD_ALIASES["relation"])),
        "source": _coerce_string(_first_non_empty_value(row, OCR_FIELD_ALIASES["source"])),
    }
    return normalized


def _first_non_empty_value(row: Mapping[str, object], keys: Sequence[str]) -> object | None:
    for key in keys:
        if key not in row:
            continue
        value = row[key]
        if isinstance(value, str) and not value.strip():
            continue
        if value is None:
            continue
        return value
    return None


def _normalize_theme_path(value: object | None) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return _normalize_theme_path_text(value)
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes)):
        parts = [_coerce_string(item) for item in value]
        parts = [part for part in parts if part]
        return THEME_PATH_DELIMITER.join(parts)
    return _normalize_theme_path_text(_coerce_string(value))


def _normalize_theme_path_text(value: str) -> str:
    stripped = value.strip()
    if not stripped:
        return ""
    for separator in (" > ", ">", "，", "、"):
        stripped = stripped.replace(separator, THEME_PATH_DELIMITER)
    parts = [part.strip() for part in stripped.split(THEME_PATH_DELIMITER)]
    parts = [part for part in parts if part]
    return THEME_PATH_DELIMITER.join(parts)


def _coerce_string(value: object | None) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return value.strip()
    return str(value).strip()
