from __future__ import annotations

from typing import Any, cast

import pytest
from scheduler.defs.rearview.assets import (
    _combine_daily_run_range_responses,
    _daily_run_metadata,
    _date_chunks,
    _query_fact_counts_for_succeeded_runs,
    _wait_for_daily_runs,
)
from scheduler.defs.rearview.resources import RearviewApiResource


class FakeRearviewApi:
    def __init__(
        self,
        *,
        statuses: dict[str, dict[str, Any]] | None = None,
        fact_counts: dict[str, dict[str, Any]] | None = None,
    ) -> None:
        self.statuses = statuses or {}
        self.fact_counts = fact_counts or {}

    def get_strategy_portfolio_daily_run_status(self, daily_run_id: str) -> dict[str, Any]:
        return self.statuses[daily_run_id]

    def get_strategy_portfolio_daily_run_fact_counts(self, daily_run_id: str) -> dict[str, Any]:
        return self.fact_counts[daily_run_id]


def test_daily_run_metadata_includes_worker_and_fact_evidence() -> None:
    metadata = _daily_run_metadata(
        partition_key="2026-06-26",
        requested_start_date="2026-06-25",
        requested_end_date="2026-06-26",
        strategy_portfolio_id="portfolio-1",
        settlement_target={"settlement_target_date": "2026-06-26"},
        response={
            "active_portfolio_count": 2,
            "created_run_count": 1,
            "skipped_run_count": 1,
            "daily_run_ids": ["daily-1", "daily-2"],
            "created_daily_run_ids": ["daily-2"],
            "skipped_daily_run_ids": ["daily-1"],
            "resolved_trade_dates": ["2026-06-25", "2026-06-26"],
        },
        statuses={
            "daily-1": {
                "strategy_portfolio_daily_run_id": "daily-1",
                "trade_date": "2026-06-25",
                "status": "succeeded",
                "current_result_attempt_id": "attempt-1",
            },
            "daily-2": {
                "strategy_portfolio_daily_run_id": "daily-2",
                "trade_date": "2026-06-26",
                "status": "succeeded",
                "current_result_attempt_id": "attempt-2",
            },
        },
        fact_counts={
            "daily-2": {
                "nav_row_count": 360,
                "trade_row_count": 42,
                "closed_trade_row_count": 18,
            }
        },
        wait_for_completion=True,
    )

    assert metadata["scheduler_version"] == "0.1.0"
    assert metadata["partition_key"] == "2026-06-26"
    assert metadata["requested_start_date"] == "2026-06-25"
    assert metadata["strategy_portfolio_id"] == "portfolio-1"
    assert metadata["succeeded_run_count"] == 2
    assert metadata["failed_run_count"] == 0
    assert metadata["latest_daily_run_id"] == "daily-2"
    assert metadata["latest_result_attempt_id"] == "attempt-2"
    assert metadata["nav_row_count"] == 360
    assert metadata["trade_row_count"] == 42
    assert metadata["closed_trade_row_count"] == 18


def test_combine_daily_run_range_responses_aggregates_chunks() -> None:
    response = _combine_daily_run_range_responses(
        start_date="2026-06-01",
        end_date="2026-06-30",
        responses=[
            {
                "resolved_trade_dates": ["2026-06-01"],
                "active_portfolio_count": 1,
                "created_run_count": 1,
                "skipped_run_count": 0,
                "daily_run_ids": ["daily-1"],
                "created_daily_run_ids": ["daily-1"],
                "skipped_daily_run_ids": [],
                "trade_date_results": [{"trade_date": "2026-06-01"}],
            },
            {
                "resolved_trade_dates": ["2026-06-02"],
                "active_portfolio_count": 2,
                "created_run_count": 0,
                "skipped_run_count": 2,
                "daily_run_ids": ["daily-2", "daily-3"],
                "created_daily_run_ids": [],
                "skipped_daily_run_ids": ["daily-2", "daily-3"],
                "trade_date_results": [{"trade_date": "2026-06-02"}],
            },
        ],
    )

    assert response["active_portfolio_count"] == 2
    assert response["created_run_count"] == 1
    assert response["skipped_run_count"] == 2
    assert response["daily_run_ids"] == ["daily-1", "daily-2", "daily-3"]
    assert response["resolved_trade_dates"] == ["2026-06-01", "2026-06-02"]


def test_date_chunks_split_natural_date_range() -> None:
    assert _date_chunks("2026-06-01", "2026-06-05", 2) == [
        ("2026-06-01", "2026-06-02"),
        ("2026-06-03", "2026-06-04"),
        ("2026-06-05", "2026-06-05"),
    ]


def test_wait_for_daily_runs_returns_succeeded_statuses() -> None:
    fake = FakeRearviewApi(
        statuses={
            "daily-1": {
                "strategy_portfolio_daily_run_id": "daily-1",
                "trade_date": "2026-06-26",
                "status": "succeeded",
                "current_result_attempt_id": "attempt-1",
            }
        }
    )

    statuses = _wait_for_daily_runs(
        rearview_api=cast(RearviewApiResource, fake),
        daily_run_ids=["daily-1"],
        poll_interval_seconds=1,
        timeout_seconds=1,
    )

    assert statuses["daily-1"]["status"] == "succeeded"


def test_wait_for_daily_runs_raises_on_failed_status() -> None:
    fake = FakeRearviewApi(
        statuses={
            "daily-1": {
                "strategy_portfolio_daily_run_id": "daily-1",
                "trade_date": "2026-06-26",
                "status": "failed_write",
                "error_type": "clickhouse",
                "error_message": "insert failed",
            }
        }
    )

    with pytest.raises(RuntimeError, match="strategy portfolio daily run failed"):
        _wait_for_daily_runs(
            rearview_api=cast(RearviewApiResource, fake),
            daily_run_ids=["daily-1"],
            poll_interval_seconds=1,
            timeout_seconds=1,
        )


def test_query_fact_counts_rejects_missing_nav_rows() -> None:
    fake = FakeRearviewApi(
        fact_counts={
            "daily-1": {
                "nav_row_count": 0,
                "trade_row_count": 0,
                "closed_trade_row_count": 0,
            }
        }
    )

    with pytest.raises(RuntimeError, match="wrote no live nav rows"):
        _query_fact_counts_for_succeeded_runs(
            rearview_api=cast(RearviewApiResource, fake),
            statuses={"daily-1": {"status": "succeeded"}},
        )
