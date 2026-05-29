from __future__ import annotations


def positive_int_or_default(value: object, *, default: int) -> int:
    if isinstance(value, bool):
        return default
    if isinstance(value, int | float | str):
        try:
            parsed = int(value)
        except ValueError:
            return default
        if parsed > 0:
            return parsed
    return default
