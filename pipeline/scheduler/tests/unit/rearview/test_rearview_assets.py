from __future__ import annotations

from typing import Any, cast

import dagster as dg
import pytest
import scheduler.defs.rearview.assets as rearview_assets_module
from scheduler.defs.rearview.assets import (
    DAILY_PORTFOLIO_NAV_LIQUIDATION_ASSET_KEY,
    DailyPortfolioNavLiquidationConfig,
    ExamplePortfolioLiveRunConfig,
    _daily_nav_liquidation_metadata,
    _example_0051_live_run_metadata,
    _query_fact_counts_for_succeeded_runs,
    _run_daily_portfolio_nav_liquidation,
    _run_example_0051_portfolio_live_run,
    _validate_example_0051_ensure_response,
    _wait_for_daily_runs,
    daily__portfolio_nav_liquidation,
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


class FakeDailyNavRearviewApi:
    def __init__(
        self,
        *,
        settlement_target: dict[str, Any],
        daily_run_response: dict[str, Any] | None = None,
        statuses: dict[str, dict[str, Any]] | None = None,
        fact_counts: dict[str, dict[str, Any]] | None = None,
    ) -> None:
        self.settlement_target = settlement_target
        self.daily_run_response = daily_run_response or {}
        self.statuses = statuses or {}
        self.fact_counts = fact_counts or {}
        self.calls: list[tuple[str, dict[str, Any]]] = []

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
        return self.settlement_target

    def create_strategy_portfolio_daily_runs(
        self,
        *,
        trade_date: str,
        client_request_id: str,
    ) -> dict[str, Any]:
        self.calls.append(
            (
                "daily_runs",
                {
                    "trade_date": trade_date,
                    "client_request_id": client_request_id,
                },
            )
        )
        return self.daily_run_response

    def get_strategy_portfolio_daily_run_status(self, daily_run_id: str) -> dict[str, Any]:
        self.calls.append(("status", {"daily_run_id": daily_run_id}))
        return self.statuses[daily_run_id]

    def get_strategy_portfolio_daily_run_fact_counts(self, daily_run_id: str) -> dict[str, Any]:
        self.calls.append(("fact_counts", {"daily_run_id": daily_run_id}))
        return self.fact_counts[daily_run_id]


class FakeExampleRearviewApi:
    def __init__(self) -> None:
        self.calls: list[tuple[str, dict[str, Any]]] = []

    def ensure_racingline_0051_low_reversal_portfolio(self) -> dict[str, Any]:
        self.calls.append(("ensure", {}))
        return {
            "case_id": "racingline_0051_low_reversal",
            "version": "v2",
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


def test_rearview_api_resource_posts_single_day_daily_runs_path(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    calls: list[tuple[str, dict[str, Any]]] = []

    def fake_post_json(
        self: RearviewApiResource,
        path: str,
        payload: dict[str, Any],
    ) -> dict[str, Any]:
        calls.append((path, payload))
        return {"daily_run_ids": ["daily-1"]}

    monkeypatch.setattr(RearviewApiResource, "_post_json", fake_post_json)

    response = RearviewApiResource(
        base_url="http://rearview.test"
    ).create_strategy_portfolio_daily_runs(
        trade_date="2026-07-01",
        client_request_id="dagster-run-2026-07-01",
    )

    assert response == {"daily_run_ids": ["daily-1"]}
    assert calls == [
        (
            "/rearview/strategy-portfolios/daily-runs",
            {
                "trade_date": "2026-07-01",
                "client_request_id": "dagster-run-2026-07-01",
            },
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


def test_daily_nav_liquidation_asset_is_unpartitioned_and_config_is_execution_only() -> None:
    config_type = DailyPortfolioNavLiquidationConfig.to_config_schema().as_field().config_type
    assert hasattr(config_type, "fields"), f"Expected shape config type, got {type(config_type)}"
    fields = cast(Any, config_type).fields

    assert daily__portfolio_nav_liquidation.key == DAILY_PORTFOLIO_NAV_LIQUIDATION_ASSET_KEY
    assert daily__portfolio_nav_liquidation.partitions_def is None
    assert set(fields) == {
        "wait_for_completion",
        "poll_interval_seconds",
        "timeout_seconds",
    }
    assert not {
        "trade_date",
        "start_date",
        "end_date",
        "strategy_portfolio_id",
        "chunk_size",
    } & set(fields)


def test_daily_nav_liquidation_metadata_includes_worker_and_fact_evidence() -> None:
    metadata = _daily_nav_liquidation_metadata(
        target_trade_date="2026-06-26",
        settlement_target={"settlement_target_date": "2026-06-26"},
        response={
            "active_portfolio_count": 2,
            "created_run_count": 1,
            "skipped_run_count": 1,
            "daily_run_ids": ["daily-1", "daily-2"],
            "created_daily_run_ids": ["daily-2"],
            "skipped_daily_run_ids": ["daily-1"],
            "daily_run_results": [{"strategy_portfolio_daily_run_id": "daily-2"}],
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
    assert metadata["target_trade_date"] == "2026-06-26"
    assert metadata["settlement_target_date"] == "2026-06-26"
    assert metadata["active_portfolio_count"] == 2
    assert metadata["created_run_count"] == 1
    assert metadata["skipped_run_count"] == 1
    assert metadata["succeeded_run_count"] == 2
    assert metadata["failed_run_count"] == 0
    assert metadata["latest_daily_run_id"] == "daily-2"
    assert metadata["latest_result_attempt_id"] == "attempt-2"
    assert metadata["nav_row_count"] == 360
    assert metadata["trade_row_count"] == 42
    assert metadata["closed_trade_row_count"] == 18


def test_daily_nav_liquidation_skips_when_settlement_target_is_empty() -> None:
    fake = FakeDailyNavRearviewApi(
        settlement_target={
            "settlement_target_date": None,
            "active_portfolio_count": 0,
        }
    )

    result = _run_daily_portfolio_nav_liquidation(
        context=dg.build_asset_context(),
        config=DailyPortfolioNavLiquidationConfig(
            wait_for_completion=True,
            poll_interval_seconds=1,
            timeout_seconds=1,
        ),
        rearview_api=cast(RearviewApiResource, fake),
    )

    assert fake.calls == [("settlement_target", {"strategy_portfolio_id": ""})]
    assert result.metadata is not None
    assert result.metadata["skip_reason"] == "settlement_target_unavailable"
    assert result.metadata["target_trade_date"] is None
    assert "daily_run_ids" in result.metadata


def test_daily_nav_liquidation_calls_single_day_daily_runs_api() -> None:
    fake = FakeDailyNavRearviewApi(
        settlement_target={
            "settlement_target_date": "2026-07-01",
            "active_portfolio_count": 1,
        },
        daily_run_response={
            "active_portfolio_count": 1,
            "created_run_count": 1,
            "skipped_run_count": 0,
            "daily_run_ids": ["daily-1"],
            "created_daily_run_ids": ["daily-1"],
            "skipped_daily_run_ids": [],
        },
        statuses={
            "daily-1": {
                "strategy_portfolio_daily_run_id": "daily-1",
                "trade_date": "2026-07-01",
                "status": "succeeded",
                "current_result_attempt_id": "attempt-1",
            }
        },
        fact_counts={
            "daily-1": {
                "nav_row_count": 602,
                "trade_row_count": 1268,
                "closed_trade_row_count": 633,
            }
        },
    )
    context = dg.build_asset_context()

    result = _run_daily_portfolio_nav_liquidation(
        context=context,
        config=DailyPortfolioNavLiquidationConfig(
            wait_for_completion=True,
            poll_interval_seconds=1,
            timeout_seconds=1,
        ),
        rearview_api=cast(RearviewApiResource, fake),
    )

    assert fake.calls == [
        ("settlement_target", {"strategy_portfolio_id": ""}),
        (
            "daily_runs",
            {
                "trade_date": "2026-07-01",
                "client_request_id": (f"dagster-{context.op_execution_context.run_id}-2026-07-01"),
            },
        ),
        ("status", {"daily_run_id": "daily-1"}),
        ("fact_counts", {"daily_run_id": "daily-1"}),
    ]
    assert result.metadata is not None
    assert result.metadata["target_trade_date"] == "2026-07-01"
    assert result.metadata["latest_daily_run_id"] == "daily-1"
    assert result.metadata["latest_result_attempt_id"] == "attempt-1"
    assert result.metadata["nav_row_count"] == 602


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
                "status": "failed",
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


def test_wait_for_daily_runs_raises_on_timeout(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = FakeRearviewApi(
        statuses={
            "daily-1": {
                "strategy_portfolio_daily_run_id": "daily-1",
                "trade_date": "2026-06-26",
                "status": "running",
            }
        }
    )
    monotonic_values = iter([0.0, 2.0])
    monkeypatch.setattr(rearview_assets_module.time, "monotonic", lambda: next(monotonic_values))
    monkeypatch.setattr(rearview_assets_module.time, "sleep", lambda _seconds: None)

    with pytest.raises(TimeoutError, match="timed out before terminal status"):
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
            "version": "v2",
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
                "version": "v2",
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
            "version": "v2",
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
