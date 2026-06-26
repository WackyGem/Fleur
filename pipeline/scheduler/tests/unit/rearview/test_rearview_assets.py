from __future__ import annotations

from scheduler.defs.rearview.assets import _daily_run_metadata


def test_daily_run_metadata_includes_scheduler_version() -> None:
    metadata = _daily_run_metadata(
        trade_date="2026-06-26",
        response={
            "active_portfolio_count": 2,
            "created_run_count": 1,
            "skipped_run_count": 1,
            "daily_run_ids": ["daily-1"],
        },
    )

    assert metadata["scheduler_version"] == "0.1.0"
    assert metadata["trade_date"] == "2026-06-26"
