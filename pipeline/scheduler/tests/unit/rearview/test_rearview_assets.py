from __future__ import annotations

from typing import Any, cast

import dagster as dg
import pytest
from scheduler.defs.rearview.assets import (
    ExamplePortfolioLiveRunConfig,
    _combine_daily_run_range_responses,
    _daily_run_metadata,
    _date_chunks,
    _example_0051_live_run_metadata,
    _query_fact_counts_for_succeeded_runs,
    _run_example_0051_portfolio_live_run,
    _validate_example_0051_ensure_response,
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


class FakeExampleRearviewApi:
    def __init__(self) -> None:
        self.calls: list[tuple[str, dict[str, Any]]] = []

    def ensure_racingline_0051_low_reversal_portfolio(self) -> dict[str, Any]:
        self.calls.append(("ensure", {}))
        return {
            "case_id": "racingline_0051_low_reversal",
            "version": "v1",
            "fixture_hash": "fixture-hash",
            "rule_hash": "rule-hash",
            "execution_config_hash": "execution-hash",
            "strategy_portfolio_id": "portfolio-1",
            "portfolio_code": "SP-20260702-ABCDE",
            "initial_signal_date": "2023-12-29",
            "live_start_date": "2024-01-02",
            "created": True,
        }

    def get_strategy_portfolio_settlement_target(
        self,
        *,
        strategy_portfolio_id: str = "",
    ) -> dict[str, Any]:
        self.calls.append(
            (
                "settlement_target",
                {"strategy_portfolio_id": strategy_portfolio_id},
            )
        )
        return {
            "settlement_target_date": "2024-01-12",
            "active_portfolio_count": 1,
        }

    def create_strategy_portfolio_daily_runs_range(
        self,
        *,
        start_date: str,
        end_date: str,
        client_request_id: str,
        max_trade_dates: int,
        strategy_portfolio_id: str = "",
    ) -> dict[str, Any]:
        self.calls.append(
            (
                "daily_run_range",
                {
                    "start_date": start_date,
                    "end_date": end_date,
                    "client_request_id": client_request_id,
                    "max_trade_dates": max_trade_dates,
                    "strategy_portfolio_id": strategy_portfolio_id,
                },
            )
        )
        return {
            "daily_run_ids": ["daily-1"],
            "created_daily_run_ids": ["daily-1"],
            "skipped_daily_run_ids": [],
            "resolved_trade_dates": [end_date],
        }

    def get_strategy_portfolio_daily_run_status(self, daily_run_id: str) -> dict[str, Any]:
        self.calls.append(("status", {"daily_run_id": daily_run_id}))
        return {
            "strategy_portfolio_daily_run_id": daily_run_id,
            "trade_date": "2024-01-02",
            "status": "succeeded",
            "current_result_attempt_id": "attempt-1",
            "signal_summary": {"top_n": 5},
        }

    def get_strategy_portfolio_daily_run_fact_counts(self, daily_run_id: str) -> dict[str, Any]:
        self.calls.append(("fact_counts", {"daily_run_id": daily_run_id}))
        return {
            "nav_row_count": 1,
            "trade_row_count": 5,
            "closed_trade_row_count": 0,
        }


def test_rearview_api_resource_posts_0051_ensure_path(monkeypatch: pytest.MonkeyPatch) -> None:
    calls: list[tuple[str, dict[str, Any]]] = []

    def fake_post_json(
        self: RearviewApiResource,
        path: str,
        payload: dict[str, Any],
    ) -> dict[str, Any]:
        calls.append((path, payload))
        return {"ok": True}

    monkeypatch.setattr(RearviewApiResource, "_post_json", fake_post_json)

    response = RearviewApiResource(
        base_url="http://rearview.test"
    ).ensure_racingline_0051_low_reversal_portfolio()

    assert response == {"ok": True}
    assert calls == [
        (
            "/rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure",
            {},
        )
    ]


def test_rearview_api_resource_gets_portfolio_specific_settlement_target(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    calls: list[str] = []

    def fake_get_json(
        self: RearviewApiResource,
        path: str,
    ) -> dict[str, Any]:
        calls.append(path)
        return {"settlement_target_date": "2024-01-12"}

    monkeypatch.setattr(RearviewApiResource, "_get_json", fake_get_json)

    response = RearviewApiResource(
        base_url="http://rearview.test"
    ).get_strategy_portfolio_settlement_target(strategy_portfolio_id="portfolio-1")

    assert response == {"settlement_target_date": "2024-01-12"}
    assert calls == [
        "/rearview/strategy-portfolios/daily-runs/settlement-target?strategy_portfolio_id=portfolio-1"
    ]


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


def test_validate_example_0051_ensure_response_accepts_expected_contract() -> None:
    _validate_example_0051_ensure_response(
        {
            "case_id": "racingline_0051_low_reversal",
            "version": "v1",
            "fixture_hash": "fixture-hash",
            "rule_hash": "rule-hash",
            "execution_config_hash": "execution-hash",
            "strategy_portfolio_id": "portfolio-1",
            "portfolio_code": "SP-20260702-ABCDE",
            "initial_signal_date": "2023-12-29",
            "live_start_date": "2024-01-02",
        }
    )


def test_validate_example_0051_ensure_response_rejects_date_drift() -> None:
    with pytest.raises(RuntimeError, match="live_start_date"):
        _validate_example_0051_ensure_response(
            {
                "case_id": "racingline_0051_low_reversal",
                "version": "v1",
                "fixture_hash": "fixture-hash",
                "rule_hash": "rule-hash",
                "execution_config_hash": "execution-hash",
                "strategy_portfolio_id": "portfolio-1",
                "portfolio_code": "SP-20260702-ABCDE",
                "initial_signal_date": "2023-12-29",
                "live_start_date": "2024-01-03",
            }
        )


def test_example_0051_asset_calls_rearview_apis_in_expected_order() -> None:
    fake = FakeExampleRearviewApi()
    context = dg.build_asset_context()

    result = _run_example_0051_portfolio_live_run(
        context=context,
        config=ExamplePortfolioLiveRunConfig(
            end_date="2024-01-12",
            max_trade_dates=20,
            wait_for_completion=True,
            poll_interval_seconds=1,
            timeout_seconds=1,
        ),
        rearview_api=cast(RearviewApiResource, fake),
    )

    assert fake.calls == [
        ("ensure", {}),
        (
            "daily_run_range",
            {
                "start_date": "2024-01-12",
                "end_date": "2024-01-12",
                "client_request_id": (
                    f"dagster-example-0051-{context.op_execution_context.run_id}-2024-01-12"
                ),
                "max_trade_dates": 20,
                "strategy_portfolio_id": "portfolio-1",
            },
        ),
        ("status", {"daily_run_id": "daily-1"}),
        ("fact_counts", {"daily_run_id": "daily-1"}),
    ]
    assert result.metadata is not None
    assert result.metadata["latest_daily_run_id"] == "daily-1"
    assert result.metadata["nav_row_count"] == 1


def test_example_0051_asset_defaults_to_settlement_target_date() -> None:
    fake = FakeExampleRearviewApi()
    context = dg.build_asset_context()

    result = _run_example_0051_portfolio_live_run(
        context=context,
        config=ExamplePortfolioLiveRunConfig(
            wait_for_completion=True,
            poll_interval_seconds=1,
            timeout_seconds=1,
        ),
        rearview_api=cast(RearviewApiResource, fake),
    )

    assert fake.calls == [
        ("ensure", {}),
        ("settlement_target", {"strategy_portfolio_id": "portfolio-1"}),
        (
            "daily_run_range",
            {
                "start_date": "2024-01-12",
                "end_date": "2024-01-12",
                "client_request_id": (
                    f"dagster-example-0051-{context.op_execution_context.run_id}-2024-01-12"
                ),
                "max_trade_dates": 250,
                "strategy_portfolio_id": "portfolio-1",
            },
        ),
        ("status", {"daily_run_id": "daily-1"}),
        ("fact_counts", {"daily_run_id": "daily-1"}),
    ]
    assert result.metadata is not None
    assert result.metadata["settlement_target_date"] == "2024-01-12"
    assert result.metadata["settlement_mode"] == "single_full_window_run"
    assert result.metadata["settlement_start_date"] == "2024-01-02"
    assert result.metadata["settlement_end_date"] == "2024-01-12"
    assert result.metadata["latest_daily_run_id"] == "daily-1"


def test_example_0051_live_run_metadata_includes_hashes_and_fact_counts() -> None:
    metadata = _example_0051_live_run_metadata(
        ensure_response={
            "case_id": "racingline_0051_low_reversal",
            "version": "v1",
            "fixture_hash": "fixture-hash",
            "rule_hash": "rule-hash",
            "execution_config_hash": "execution-hash",
            "strategy_portfolio_id": "portfolio-1",
            "portfolio_code": "SP-20260702-ABCDE",
            "initial_signal_date": "2023-12-29",
            "live_start_date": "2024-01-02",
            "created": True,
        },
        daily_run_response={
            "daily_run_ids": ["daily-1"],
            "created_daily_run_ids": ["daily-1"],
            "skipped_daily_run_ids": [],
            "resolved_trade_dates": ["2024-01-02"],
        },
        settlement_target={"settlement_target_date": "2024-01-02"},
        statuses={
            "daily-1": {
                "strategy_portfolio_daily_run_id": "daily-1",
                "trade_date": "2024-01-02",
                "status": "succeeded",
                "current_result_attempt_id": "attempt-1",
                "signal_summary": {"top_n": 5},
            }
        },
        fact_counts={
            "daily-1": {
                "nav_row_count": 1,
                "trade_row_count": 5,
                "closed_trade_row_count": 0,
            }
        },
        wait_for_completion=True,
    )

    assert metadata["case_id"] == "racingline_0051_low_reversal"
    assert metadata["live_start_date"] == "2024-01-02"
    assert metadata["fixture_hash"] == "fixture-hash"
    assert metadata["latest_daily_run_id"] == "daily-1"
    assert metadata["nav_row_count"] == 1
    assert metadata["trade_row_count"] == 5
