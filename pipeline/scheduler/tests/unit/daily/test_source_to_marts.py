from __future__ import annotations

from collections.abc import Mapping
from datetime import date, datetime
from typing import Any, Protocol, cast, get_args
from zoneinfo import ZoneInfo

import dagster as dg
import pytest
from scheduler.defs.automation.source_raw_backfill import (
    BAOSTOCK_DAILY_KLINE_SCOPE,
    BackfillRunSubmitter,
    BackfillStep,
    BackfillStepResult,
)
from scheduler.defs.automation.source_raw_backfill import (
    EXECUTION_MODE_FULL as SOURCE_RAW_EXECUTION_MODE_FULL,
)
from scheduler.defs.automation.source_to_marts_backfill import (
    ALL_SOURCE_TO_MARTS_SCOPE,
    EXECUTION_MODE_DOWNSTREAM_ONLY,
    EXECUTION_MODE_SOURCE_RAW_ONLY,
    SNAPSHOT_REFERENCE_DATA_SCOPE,
    SOURCE_TO_MARTS_TARGET_SCOPES,
    STAGE_DBT_CALCULATION_WRAPPERS,
    STAGE_DBT_INTERMEDIATE,
    STAGE_DBT_MARTS,
    STAGE_DBT_STAGING,
    STAGE_FURNACE_CALCULATION,
    STAGE_SOURCE_RAW,
    calculation_asset_keys_covered_by_all_source_to_marts_scope,
    dbt_asset_keys_covered_by_all_source_to_marts_scope,
)
from scheduler.defs.daily.definitions import (
    DAILY_SCHEDULE_CRON,
    DAILY_SCHEDULE_NAME,
    daily__fetch_history_sources_to_marts_schedule,
)
from scheduler.defs.daily.source_to_marts import (
    DAILY_CONTROLLER_OP_NAME,
    DAILY_JOB_NAME,
    PORTFOLIO_LIVE_LIQUIDATION_ASSET_KEY,
    STAGE_PORTFOLIO_LIVE_LIQUIDATION,
    STEP_PORTFOLIO_LIVE_LIQUIDATION,
    DailyFetchHistorySourcesToMartsConfig,
    DailyFetchHistorySourcesToMartsRequest,
    DailyPlan,
    DailyStep,
    add_daily_fetch_history_sources_to_marts_controller_run_tags,
    build_daily_fetch_history_sources_to_marts_plan,
    execute_daily_fetch_history_sources_to_marts_controller_request,
    execute_daily_fetch_history_sources_to_marts_plan,
    furnace_modes_for_daily_steps,
)


def test_daily_entrypoint_names_are_explicit() -> None:
    assert DAILY_JOB_NAME == "daily__fetch_history_sources_to_marts_schedule_job"
    assert DAILY_CONTROLLER_OP_NAME == "daily__fetch_history_sources_to_marts_schedule_controller"
    assert DAILY_SCHEDULE_NAME == "daily__fetch_history_sources_to_marts_schedule"


def test_daily_config_requires_target_date_and_defaults_to_all_scope() -> None:
    fields = config_schema_fields(DailyFetchHistorySourcesToMartsConfig)

    assert not fields["target_scope"].is_required
    assert fields["target_date"].is_required
    assert set(
        get_args(DailyFetchHistorySourcesToMartsConfig.model_fields["target_scope"].annotation)
    ) == set(SOURCE_TO_MARTS_TARGET_SCOPES)
    assert set(
        get_args(DailyFetchHistorySourcesToMartsConfig.model_fields["execution_mode"].annotation)
    ) == {
        SOURCE_RAW_EXECUTION_MODE_FULL,
        EXECUTION_MODE_SOURCE_RAW_ONLY,
        EXECUTION_MODE_DOWNSTREAM_ONLY,
    }


def test_daily_plan_maps_target_date_to_single_day_and_uses_append_latest() -> None:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        daily_request(target_date="2026-06-30", dry_run=False),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-001",
    )

    assert plan.target_date == "2026-06-30"
    assert plan.year_partitions == ("2026",)
    assert {step.stage for step in plan.steps} == {
        STAGE_SOURCE_RAW,
        STAGE_DBT_STAGING,
        STAGE_DBT_INTERMEDIATE,
        STAGE_FURNACE_CALCULATION,
        STAGE_DBT_CALCULATION_WRAPPERS,
        STAGE_DBT_MARTS,
        STAGE_PORTFOLIO_LIVE_LIQUIDATION,
    }
    assert furnace_modes_for_daily_steps(plan.steps) == {"append-latest"}

    calculation_step = only_step_for_stage(plan, STAGE_FURNACE_CALCULATION)
    ops = cast(Mapping[str, Mapping[str, Mapping[str, object]]], calculation_step.run_config["ops"])
    assert {
        (op_config["config"]["request_from"], op_config["config"]["request_to"])
        for op_config in ops.values()
    } == {("2026-06-30", "2026-06-30")}

    baostock_daily_step = next(
        step for step in plan.steps if step.label == "baostock daily source 2026"
    )
    assert baostock_daily_step.partition.partition_range_start == "2026-06-30"
    assert baostock_daily_step.partition.partition_range_end == "2026-06-30"


def test_daily_dry_run_keeps_furnace_dry_run_mode_and_does_not_submit() -> None:
    submitter = FakeSubmitter()
    tag_writer = FakeRunTagWriter()

    plan = execute_daily_fetch_history_sources_to_marts_controller_request(
        daily_request(target_date="2026-06-30", dry_run=True),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-002",
        tag_writer=tag_writer,
        log=FakeLog(),
        submitter=submitter,
    )

    assert furnace_modes_for_daily_steps(plan.steps) == {"dry-run"}
    assert plan.steps[-1].stage == STAGE_PORTFOLIO_LIVE_LIQUIDATION
    assert submitter.submitted_steps == []
    assert tag_writer.tags["daily.kind"] == "fetch_history_sources_to_marts_schedule"
    assert tag_writer.tags["daily.target_date"] == "2026-06-30"


def test_daily_plan_reuses_source_to_marts_registry_and_excludes_independent_domains() -> None:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        daily_request(target_date="2026-06-30"),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-003",
    )
    planned_assets = {asset_key for step in plan.steps for asset_key in step.asset_keys}
    planned_dbt_assets = {
        asset_key
        for step in plan.steps
        if step.stage
        in {
            STAGE_DBT_STAGING,
            STAGE_DBT_INTERMEDIATE,
            STAGE_DBT_CALCULATION_WRAPPERS,
            STAGE_DBT_MARTS,
        }
        for asset_key in step.asset_keys
    }
    planned_calculation_assets = {
        asset_key
        for step in plan.steps
        if step.stage == STAGE_FURNACE_CALCULATION
        for asset_key in step.asset_keys
    }

    assert planned_dbt_assets == dbt_asset_keys_covered_by_all_source_to_marts_scope()
    assert (
        planned_calculation_assets == calculation_asset_keys_covered_by_all_source_to_marts_scope()
    )
    assert {
        "int_stock_limit_up_pool_daily",
        "mart_stock_limit_up_pool_daily",
        "mart_government_bond_yields_daily",
        "int_stock_balance_sheet",
        "mart_stock_balance_sheet",
        "int_stock_dividend_plan",
        "mart_stock_dividend_plan",
        "int_stock_free_float_shareholder_top10",
        "mart_stock_free_float_shareholder_top10",
    } <= planned_dbt_assets
    assert not {asset_key for asset_key in planned_assets if "jiuyan" in asset_key}
    assert "fleur_portfolio/portfolio_run_snapshot" not in planned_assets
    assert not {asset_key for asset_key in planned_assets if "calc_portfolio_" in asset_key}
    assert not {asset_key for asset_key in planned_assets if asset_key.startswith("int_portfolio_")}
    assert not {
        asset_key for asset_key in planned_assets if asset_key.startswith("mart_portfolio_")
    }
    assert PORTFOLIO_LIVE_LIQUIDATION_ASSET_KEY in planned_assets
    assert "rearview/strategy_portfolio_daily_runs" not in planned_assets

    terminal_step = plan.steps[-1]
    assert terminal_step.label == "portfolio live nav liquidation"
    assert terminal_step.stage == STAGE_PORTFOLIO_LIVE_LIQUIDATION
    assert terminal_step.step_kind == STEP_PORTFOLIO_LIVE_LIQUIDATION
    assert terminal_step.asset_keys == (PORTFOLIO_LIVE_LIQUIDATION_ASSET_KEY,)
    assert terminal_step.partition.label() == "unpartitioned"
    assert terminal_step.run_config == {}


def test_daily_partial_scope_does_not_append_portfolio_live_terminal_step() -> None:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        daily_request(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            target_date="2026-06-30",
        ),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-003-partial",
    )

    assert STAGE_PORTFOLIO_LIVE_LIQUIDATION not in {step.stage for step in plan.steps}
    assert PORTFOLIO_LIVE_LIQUIDATION_ASSET_KEY not in {
        asset_key for step in plan.steps for asset_key in step.asset_keys
    }


def test_daily_snapshot_scope_requires_target_date_but_source_plan_ignores_date_window() -> None:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        daily_request(
            target_scope=SNAPSHOT_REFERENCE_DATA_SCOPE,
            target_date="2026-06-30",
        ),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-004",
    )

    assert plan.target_date == "2026-06-30"
    assert plan.year_partitions == ()
    assert {step.partition.label() for step in plan.steps} == {"unpartitioned"}
    assert {asset_key for step in plan.steps for asset_key in step.asset_keys} >= {
        "source/sina__trade_calendar",
        "source/baostock__query_stock_basic",
        "clickhouse/raw/sina__trade_calendar",
        "clickhouse/raw/baostock__query_stock_basic",
    }


def test_daily_controller_tags_are_written_to_parent_and_child_steps() -> None:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        daily_request(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            target_date="2026-06-30",
        ),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-005",
    )
    tag_writer = FakeRunTagWriter()

    add_daily_fetch_history_sources_to_marts_controller_run_tags(
        tag_writer,
        run_id="daily-controller-run-005",
        plan=plan,
    )

    assert tag_writer.tags == {
        "daily.kind": "fetch_history_sources_to_marts_schedule",
        "daily.id": "baostock_daily_kline-2026-06-30-dailycontrol",
        "daily.target_scope": BAOSTOCK_DAILY_KLINE_SCOPE,
        "daily.target_date": "2026-06-30",
        "daily.execution_mode": SOURCE_RAW_EXECUTION_MODE_FULL,
    }
    assert all(
        step.tags["daily.parent_run_id"] == "daily-controller-run-005" for step in plan.steps
    )
    assert all("daily.stage" in step.tags for step in plan.steps)


def test_daily_execution_stops_before_downstream_after_source_raw_failure() -> None:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        daily_request(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            target_date="2026-06-30",
            dry_run=False,
        ),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-006",
    )
    submitter = FakeSubmitter(fail_on_label="baostock raw 2026")

    with pytest.raises(RuntimeError, match="Daily source-to-marts step failed"):
        execute_daily_fetch_history_sources_to_marts_plan(plan, submitter=submitter, log=FakeLog())

    assert [step.label for step in submitter.submitted_steps] == [
        "baostock daily source 2026",
        "baostock compacted source 2026",
        "baostock raw 2026",
    ]


def test_daily_execution_stops_before_portfolio_live_after_upstream_failure() -> None:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        daily_request(
            target_date="2026-06-30",
            dry_run=False,
        ),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-006-all",
    )
    terminal_label = plan.steps[-1].label
    assert terminal_label == "portfolio live nav liquidation"
    submitter = FakeSubmitter(fail_on_label=plan.steps[0].label)

    with pytest.raises(RuntimeError, match="Daily source-to-marts step failed"):
        execute_daily_fetch_history_sources_to_marts_plan(plan, submitter=submitter, log=FakeLog())

    assert terminal_label not in [step.label for step in submitter.submitted_steps]


def test_daily_terminal_step_failure_fails_parent_execution() -> None:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        daily_request(
            target_date="2026-06-30",
            dry_run=False,
        ),
        today=date(2026, 6, 30),
        controller_run_id="daily-controller-run-006-terminal",
    )
    submitter = FakeSubmitter(fail_on_label="portfolio live nav liquidation")

    with pytest.raises(RuntimeError, match="Daily source-to-marts step failed"):
        execute_daily_fetch_history_sources_to_marts_plan(plan, submitter=submitter, log=FakeLog())

    assert submitter.submitted_steps[-1].label == "portfolio live nav liquidation"


def test_daily_schedule_is_stopped_and_emits_target_date_config() -> None:
    context = dg.build_schedule_context(
        scheduled_execution_time=datetime(
            2026,
            7,
            1,
            17,
            45,
            tzinfo=ZoneInfo("Asia/Shanghai"),
        )
    )

    tick = daily__fetch_history_sources_to_marts_schedule.evaluate_tick(context)

    assert daily__fetch_history_sources_to_marts_schedule.default_status == (
        dg.DefaultScheduleStatus.STOPPED
    )
    assert daily__fetch_history_sources_to_marts_schedule.cron_schedule == DAILY_SCHEDULE_CRON
    assert DAILY_SCHEDULE_CRON == "45 17 * * *"
    assert tick.run_requests is not None
    assert len(tick.run_requests) == 1
    request = tick.run_requests[0]
    assert request.run_key == "daily__fetch_history_sources_to_marts_schedule:2026-07-01"
    assert request.tags["daily.kind"] == "fetch_history_sources_to_marts_schedule"
    assert request.tags["daily.dry_run"] == "false"
    assert request.run_config == {
        "ops": {
            DAILY_CONTROLLER_OP_NAME: {
                "config": {
                    "target_scope": ALL_SOURCE_TO_MARTS_SCOPE,
                    "target_date": "2026-07-01",
                    "execution_mode": SOURCE_RAW_EXECUTION_MODE_FULL,
                    "dry_run": False,
                    "refresh_prerequisite_snapshots": False,
                    "overwrite_source_partitions": False,
                }
            }
        }
    }


def only_step_for_stage(plan: DailyPlan, stage: str) -> DailyStep:
    matches = [step for step in plan.steps if step.stage == stage]
    assert len(matches) == 1
    return matches[0]


def config_schema_fields(
    config_cls: type[DailyFetchHistorySourcesToMartsConfig],
) -> Mapping[str, Any]:
    config_type = config_cls.to_config_schema().as_field().config_type
    assert hasattr(config_type, "fields"), f"Expected shape config type, got {type(config_type)}"
    config_type_with_fields = cast(ConfigTypeWithFields, config_type)
    fields = config_type_with_fields.fields
    assert isinstance(fields, Mapping), f"Expected mapping fields, got {type(fields)}"
    return fields


class ConfigTypeWithFields(Protocol):
    fields: Mapping[str, Any]


def daily_request(
    *,
    target_scope: str = ALL_SOURCE_TO_MARTS_SCOPE,
    target_date: str = "2026-06-30",
    execution_mode: str = SOURCE_RAW_EXECUTION_MODE_FULL,
    refresh_prerequisite_snapshots: bool = False,
    overwrite_source_partitions: bool = False,
    dry_run: bool = True,
) -> DailyFetchHistorySourcesToMartsRequest:
    return DailyFetchHistorySourcesToMartsRequest(
        target_scope=target_scope,
        target_date=target_date,
        execution_mode=execution_mode,
        refresh_prerequisite_snapshots=refresh_prerequisite_snapshots,
        overwrite_source_partitions=overwrite_source_partitions,
        dry_run=dry_run,
    )


class FakeSubmitter(BackfillRunSubmitter):
    def __init__(self, *, fail_on_label: str | None = None) -> None:
        self._fail_on_label = fail_on_label
        self.submitted_steps: list[BackfillStep] = []

    def submit_step(self, step: BackfillStep) -> BackfillStepResult:
        self.submitted_steps.append(step)
        success = step.label != self._fail_on_label
        return BackfillStepResult(
            run_id=f"run-{len(self.submitted_steps)}",
            status="SUCCESS" if success else "FAILURE",
            success=success,
        )


class FakeRunTagWriter:
    def __init__(self) -> None:
        self.tags: dict[str, str] = {}

    def add_run_tags(self, run_id: str, new_tags: Mapping[str, str]) -> None:
        _ = run_id
        self.tags = dict(new_tags)


class FakeLog:
    def info(self, msg: object, *_args: object) -> None:
        _ = msg
        return None
