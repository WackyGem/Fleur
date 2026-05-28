from __future__ import annotations

import json
from collections.abc import Mapping, Sequence

import pyarrow as pa

from scheduler.defs.jiuyan_industry_ocr.schemas import JIUYAN_INDUSTRY_OCR_SCHEMA

THEME_PATH_DELIMITER = ","

OCR_FIELD_ALIASES = {
    "stock_name": ("stock_name", "个股", "公司", "标的"),
    "theme_path": ("theme_path", "theme", "topic", "主题"),
    "relation": ("relation", "relevance", "相关性", "说明", "业务说明"),
    "source": ("source", "信源", "资料来源", "来源"),
}


class OcrSchemaError(ValueError):
    """Raised when the OCR payload does not match the expected schema."""


def normalize_ocr_content(content: str) -> list[dict[str, str]]:
    try:
        payload = json.loads(content)
    except json.JSONDecodeError as error:
        msg = "OCR response content is not valid JSON"
        raise OcrSchemaError(msg) from error

    if not isinstance(payload, list):
        msg = "OCR response content must be a JSON array"
        raise OcrSchemaError(msg)

    rows: list[dict[str, str]] = []
    seen: set[tuple[str, str, str, str]] = set()
    for index, item in enumerate(payload):
        if not isinstance(item, Mapping):
            msg = f"OCR response row {index} is not an object"
            raise OcrSchemaError(msg)
        row = _normalize_row(item)
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
        "relation": _coerce_string(
            _first_non_empty_value(row, OCR_FIELD_ALIASES["relation"])
        ),
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
