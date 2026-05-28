from __future__ import annotations

import base64
from collections.abc import Mapping

from scheduler.defs.ocr.schemas import OcrSchemaError


def build_image_data_url(mime_type: str, image_bytes: bytes) -> str:
    encoded = base64.b64encode(image_bytes).decode("ascii")
    return f"data:{mime_type};base64,{encoded}"


def extract_ocr_content(payload: Mapping[str, object]) -> str:
    choices = payload.get("choices")
    if not isinstance(choices, list) or not choices:
        msg = "OCR response missing choices"
        raise OcrSchemaError(msg)

    first_choice = choices[0]
    if not isinstance(first_choice, Mapping):
        msg = "OCR response choices[0] is not an object"
        raise OcrSchemaError(msg)

    message = first_choice.get("message")
    if not isinstance(message, Mapping):
        msg = "OCR response message is not an object"
        raise OcrSchemaError(msg)

    content = message.get("content")
    if not isinstance(content, str):
        msg = "OCR response content is not a string"
        raise OcrSchemaError(msg)
    return content
