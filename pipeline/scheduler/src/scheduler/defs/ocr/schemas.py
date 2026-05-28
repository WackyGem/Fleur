from __future__ import annotations

import json
from collections.abc import Mapping


class OcrSchemaError(ValueError):
    """Raised when an OCR payload does not match the expected schema."""


def parse_json_array(content: str) -> list[object]:
    try:
        payload = json.loads(content)
    except json.JSONDecodeError as error:
        msg = "OCR response content is not valid JSON"
        raise OcrSchemaError(msg) from error

    if not isinstance(payload, list):
        msg = "OCR response content must be a JSON array"
        raise OcrSchemaError(msg)
    return payload


def require_mapping_row(value: object, *, index: int) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        msg = f"OCR response row {index} is not an object"
        raise OcrSchemaError(msg)
    return value
