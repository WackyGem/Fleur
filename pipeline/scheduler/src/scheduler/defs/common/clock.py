from __future__ import annotations


def elapsed_seconds(started_at: float, finished_at: float) -> float:
    return round(finished_at - started_at, 6)
