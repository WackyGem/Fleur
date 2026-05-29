from __future__ import annotations

import json


def optional_string(value: object) -> str | None:
    if value is None:
        return None
    cleaned = str(value).strip()
    if not cleaned:
        return None
    return cleaned


def required_string(value: object, *, field_name: str) -> str:
    if not isinstance(value, str):
        msg = f"Expected {field_name} to be a string"
        raise RuntimeError(msg)
    return value


def string_or_null(value: object) -> str | None:
    if value is None:
        return None
    if isinstance(value, str):
        return value
    if isinstance(value, bool | int | float):
        return str(value)
    return json.dumps(value, ensure_ascii=False, sort_keys=True, default=str)
