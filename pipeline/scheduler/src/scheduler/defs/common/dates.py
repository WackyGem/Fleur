from __future__ import annotations

from datetime import date

from scheduler.defs.common.strings import optional_string


def parse_date_or_none(value: object) -> date | None:
    cleaned = optional_string(value)
    if cleaned is None:
        return None
    try:
        return date.fromisoformat(cleaned)
    except ValueError:
        return None


def is_trade_date(candidate: date, trade_dates: set[date]) -> bool:
    return candidate in trade_dates
