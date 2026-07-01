from __future__ import annotations

from collections.abc import Mapping
from datetime import date
from typing import Any, Protocol, cast, get_args

import pytest
from scheduler.defs.automation.source_raw_backfill import (
    BAOSTOCK_DAILY_KLINE_SCOPE,
    CHINABOND_SCOPE,
    EASTMONEY_F10_SCOPE,
    EXECUTION_MODE_FULL,
    MARKET_EVENTS_SCOPE,
    SNAPSHOT_REFERENCE_DATA_SCOPE,
    BackfillRunSubmitter,
    BackfillStep,
    BackfillStepResult,
)
from scheduler.defs.automation.source_to_marts_backfill import (
    ALL_SOURCE_TO_MARTS_SCOPE,
    BACKFILL_CONTROLLER_OP_NAME,
    BACKFILL_JOB_NAME,
    EXECUTION_MODE_DOWNSTREAM_ONLY,
    EXECUTION_MODE_SOURCE_RAW_ONLY,
    SOURCE_TO_MARTS_TARGET_SCOPES,
    STAGE_DBT_STAGING,
    STAGE_FURNACE_CALCULATION,
    STAGE_SOURCE_RAW,
    SourceToMartsBackfillConfig,
    SourceToMartsControllerRequest,
    add_source_to_marts_controller_run_tags,
    build_source_to_marts_plan,
    calculation_asset_keys_covered_by_all_source_to_marts_scope,
    dbt_asset_keys_covered_by_all_source_to_marts_scope,
    execute_source_to_marts_plan,
)
from scheduler.defs.definitions import defs as scheduler_defs


def test_source_to_marts_backfill_entrypoint_names_are_explicit() -> None:
    assert BACKFILL_JOB_NAME == "backfill__fetch_history_sources_to_marts_job"
    assert BACKFILL_CONTROLLER_OP_NAME == "backfill__fetch_history_sources_to_marts_controller"


def test_config_schema_requires_dates_for_all_scopes() -> None:
    fields = config_schema_fields(SourceToMartsBackfillConfig)

    assert fields["target_scope"].is_required
    assert fields["start_date"].is_required
    assert fields["end_date"].is_required
    assert set(
        get_args(SourceToMartsBackfillConfig.model_fields["target_scope"].annotation)
    ) == set(SOURCE_TO_MARTS_TARGET_SCOPES)
    assert set(get_args(SourceToMartsBackfillConfig.model_fields["execution_mode"].annotation)) == {
        EXECUTION_MODE_FULL,
        EXECUTION_MODE_SOURCE_RAW_ONLY,
        EXECUTION_MODE_DOWNSTREAM_ONLY,
    }


def test_baostock_plan_splits_source_raw_dbt_furnace_and_marts() -> None:
    plan = build_source_to_marts_plan(
        source_to_marts_request(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            start_date="2024-01-01",
            end_date="2024-12-31",
            dry_run=False,
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-001",
    )

    assert [step.tags["backfill.stage"] for step in plan.steps] == [
        STAGE_SOURCE_RAW,
        STAGE_SOURCE_RAW,
        STAGE_SOURCE_RAW,
        "dbt_staging",
        "dbt_intermediate",
        STAGE_FURNACE_CALCULATION,
        "dbt_calculation_wrappers",
        "dbt_marts",
    ]
    calculation_step = next(
        step for step in plan.steps if step.tags["backfill.stage"] == STAGE_FURNACE_CALCULATION
    )
    assert calculation_step.asset_keys == (
        "fleur_calculation/calc_stock_kdj_daily",
        "fleur_calculation/calc_stock_ma_daily",
        "fleur_calculation/calc_stock_rsi_daily",
        "fleur_calculation/calc_stock_boll_daily",
        "fleur_calculation/calc_stock_macd_daily",
        "fleur_calculation/calc_stock_price_pattern_daily",
    )
    ops = cast(Mapping[str, Mapping[str, Mapping[str, object]]], calculation_step.run_config["ops"])
    assert set(ops) == {
        "fleur_calculation__calc_stock_kdj_daily",
        "fleur_calculation__calc_stock_ma_daily",
        "fleur_calculation__calc_stock_rsi_daily",
        "fleur_calculation__calc_stock_boll_daily",
        "fleur_calculation__calc_stock_macd_daily",
        "fleur_calculation__calc_stock_price_pattern_daily",
    }
    assert {
        tuple(
            (
                op_config["config"]["request_from"],
                op_config["config"]["request_to"],
                op_config["config"]["mode"],
            )
        )
        for op_config in ops.values()
    } == {("2024-01-01", "2024-12-31", "replace-cascade")}


def test_controller_dry_run_sets_furnace_child_config_to_dry_run() -> None:
    plan = build_source_to_marts_plan(
        source_to_marts_request(target_scope=BAOSTOCK_DAILY_KLINE_SCOPE),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-001b",
    )

    calculation_step = next(
        step for step in plan.steps if step.tags["backfill.stage"] == STAGE_FURNACE_CALCULATION
    )
    ops = cast(Mapping[str, Mapping[str, Mapping[str, object]]], calculation_step.run_config["ops"])

    assert {op_config["config"]["mode"] for op_config in ops.values()} == {"dry-run"}


def test_source_raw_only_and_downstream_only_modes_split_execution_surface() -> None:
    source_raw_plan = build_source_to_marts_plan(
        source_to_marts_request(
            target_scope=EASTMONEY_F10_SCOPE,
            execution_mode=EXECUTION_MODE_SOURCE_RAW_ONLY,
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-002",
    )
    downstream_plan = build_source_to_marts_plan(
        source_to_marts_request(
            target_scope=EASTMONEY_F10_SCOPE,
            execution_mode=EXECUTION_MODE_DOWNSTREAM_ONLY,
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-003",
    )

    assert {step.tags["backfill.stage"] for step in source_raw_plan.steps} == {STAGE_SOURCE_RAW}
    assert STAGE_SOURCE_RAW not in {step.tags["backfill.stage"] for step in downstream_plan.steps}
    assert downstream_plan.steps[0].tags["backfill.stage"] == STAGE_DBT_STAGING


def test_market_events_scope_filters_jiuyan_and_keeps_ths_staging() -> None:
    plan = build_source_to_marts_plan(
        source_to_marts_request(target_scope=MARKET_EVENTS_SCOPE),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-004",
    )
    planned_assets = {asset_key for step in plan.steps for asset_key in step.asset_keys}

    assert "source/ths__limit_up_pool" in planned_assets
    assert "source/ths__limit_up_pool_compacted" in planned_assets
    assert "clickhouse/raw/ths__limit_up_pool_compacted" in planned_assets
    assert "stg_ths__limit_up_pool_compacted" in planned_assets
    assert not {asset_key for asset_key in planned_assets if "jiuyan" in asset_key}


def test_snapshot_reference_scope_ignores_required_dates_and_filters_jiuyan() -> None:
    plan = build_source_to_marts_plan(
        source_to_marts_request(
            target_scope=SNAPSHOT_REFERENCE_DATA_SCOPE,
            start_date="2026-12-30",
            end_date="2026-12-31",
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-005",
    )
    planned_assets = {asset_key for step in plan.steps for asset_key in step.asset_keys}

    assert plan.start_date is None
    assert plan.end_date is None
    assert plan.year_partitions == ()
    assert not {asset_key for asset_key in planned_assets if "jiuyan" in asset_key}
    assert "source/sina__trade_calendar" in planned_assets
    assert "source/baostock__query_stock_basic" in planned_assets
    assert "clickhouse/raw/sina__trade_calendar" in planned_assets
    assert "clickhouse/raw/baostock__query_stock_basic" in planned_assets


def test_all_source_to_marts_scope_excludes_jiuyan_portfolio_and_covers_expected_assets() -> None:
    plan = build_source_to_marts_plan(
        source_to_marts_request(target_scope=ALL_SOURCE_TO_MARTS_SCOPE),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-006",
    )
    planned_assets = {asset_key for step in plan.steps for asset_key in step.asset_keys}

    assert not {asset_key for asset_key in planned_assets if "jiuyan" in asset_key}
    assert not {asset_key for asset_key in planned_assets if "portfolio" in asset_key}
    assert dbt_asset_keys_covered_by_all_source_to_marts_scope() == expected_dbt_coverage()
    assert calculation_asset_keys_covered_by_all_source_to_marts_scope() == {
        "fleur_calculation/calc_stock_kdj_daily",
        "fleur_calculation/calc_stock_ma_daily",
        "fleur_calculation/calc_stock_rsi_daily",
        "fleur_calculation/calc_stock_boll_daily",
        "fleur_calculation/calc_stock_macd_daily",
        "fleur_calculation/calc_stock_price_pattern_daily",
    }


def test_invalid_scope_execution_mode_and_date_range_fail_explicitly() -> None:
    with pytest.raises(ValueError, match="Unsupported target_scope"):
        build_source_to_marts_plan(
            source_to_marts_request(target_scope="unknown"),
            today=date(2026, 6, 30),
            controller_run_id="controller-run-007",
        )

    with pytest.raises(ValueError, match="Unsupported execution_mode"):
        build_source_to_marts_plan(
            source_to_marts_request(
                target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
                execution_mode="raw_only",
            ),
            today=date(2026, 6, 30),
            controller_run_id="controller-run-008",
        )

    with pytest.raises(ValueError, match="start_date cannot be later"):
        build_source_to_marts_plan(
            source_to_marts_request(
                target_scope=CHINABOND_SCOPE,
                start_date="2025-01-01",
                end_date="2024-01-01",
            ),
            today=date(2026, 6, 30),
            controller_run_id="controller-run-009",
        )

    with pytest.raises(ValueError, match="end_date cannot be later"):
        build_source_to_marts_plan(
            source_to_marts_request(
                target_scope=CHINABOND_SCOPE,
                start_date="2026-01-01",
                end_date="2026-07-01",
            ),
            today=date(2026, 6, 30),
            controller_run_id="controller-run-010",
        )


def test_controller_tags_use_source_to_marts_kind() -> None:
    plan = build_source_to_marts_plan(
        source_to_marts_request(target_scope=CHINABOND_SCOPE),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-011",
    )
    tag_writer = FakeRunTagWriter()

    add_source_to_marts_controller_run_tags(tag_writer, run_id="controller-run-011", plan=plan)

    assert tag_writer.tags == {
        "backfill.kind": "fetch_history_sources_to_marts",
        "backfill.id": "chinabond-2024-01-01-2024-12-31-controllerru",
        "backfill.target_scope": CHINABOND_SCOPE,
        "backfill.start_date": "2024-01-01",
        "backfill.end_date": "2024-12-31",
    }
    assert all(step.tags["backfill.parent_run_id"] == "controller-run-011" for step in plan.steps)
    assert all("backfill.stage" in step.tags for step in plan.steps)


def test_execute_source_to_marts_plan_stops_before_downstream_after_source_raw_failure() -> None:
    plan = build_source_to_marts_plan(
        source_to_marts_request(target_scope=BAOSTOCK_DAILY_KLINE_SCOPE),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-012",
    )
    submitter = FakeSubmitter(fail_on_label="baostock raw 2024")

    with pytest.raises(RuntimeError, match="Source-to-marts backfill step failed"):
        execute_source_to_marts_plan(plan, submitter=submitter, log=FakeLog())

    assert [step.label for step in submitter.submitted_steps] == [
        "baostock daily source 2024",
        "baostock compacted source 2024",
        "baostock raw 2024",
    ]


def expected_dbt_coverage() -> set[str]:
    loaded_defs = scheduler_defs.load_fn()
    dbt_asset_def = next(asset for asset in loaded_defs.assets or [] if len(asset.keys) == 52)
    expected: set[str] = set()
    for key in dbt_asset_def.keys:
        key_str = key.to_user_string()
        group = dbt_asset_def.specs_by_key[key].group_name
        if group not in {"dbt_staging", "dbt_intermediate", "dbt_marts"}:
            continue
        if key_str.startswith("stg_jiuyan__"):
            continue
        if key_str.startswith("int_portfolio_") or key_str.startswith("mart_portfolio_"):
            continue
        expected.add(key_str)
    return expected


def config_schema_fields(config_cls: type[SourceToMartsBackfillConfig]) -> Mapping[str, Any]:
    config_type = config_cls.to_config_schema().as_field().config_type
    assert hasattr(config_type, "fields"), f"Expected shape config type, got {type(config_type)}"
    config_type_with_fields = cast(ConfigTypeWithFields, config_type)
    fields = config_type_with_fields.fields
    assert isinstance(fields, Mapping), f"Expected mapping fields, got {type(fields)}"
    return fields


class ConfigTypeWithFields(Protocol):
    fields: Mapping[str, Any]


def source_to_marts_request(
    *,
    target_scope: str,
    start_date: str = "2024-01-01",
    end_date: str = "2024-12-31",
    execution_mode: str = EXECUTION_MODE_FULL,
    refresh_prerequisite_snapshots: bool = False,
    overwrite_source_partitions: bool = False,
    dry_run: bool = True,
) -> SourceToMartsControllerRequest:
    return SourceToMartsControllerRequest(
        target_scope=target_scope,
        start_date=start_date,
        end_date=end_date,
        execution_mode=execution_mode,
        refresh_prerequisite_snapshots=refresh_prerequisite_snapshots,
        overwrite_source_partitions=overwrite_source_partitions,
        dry_run=dry_run,
    )


class FakeSubmitter(BackfillRunSubmitter):
    def __init__(self, *, fail_on_label: str) -> None:
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
