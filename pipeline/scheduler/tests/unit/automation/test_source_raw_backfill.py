from __future__ import annotations

from collections.abc import Mapping
from datetime import date

import pytest
from scheduler.defs.automation.source_raw_backfill import (
    ALL_FETCH_SOURCES_TO_RAW_SCOPE,
    ALL_RAW_YEARLY_SCOPE,
    BAOSTOCK_DAILY_KLINE_SCOPE,
    DEFAULT_JIUYAN_OCR_LIMIT,
    EXECUTION_MODE_RAW_ONLY,
    JIUYAN_OCR_PIPELINE_SCOPE,
    STEP_RAW,
    BackfillControllerConfig,
    BackfillRunSubmitter,
    BackfillStep,
    BackfillStepResult,
    add_controller_run_tags,
    all_registered_raw_asset_keys,
    all_registered_source_asset_keys,
    build_backfill_plan,
    execute_backfill_plan,
    raw_asset_keys_covered_by_all_fetch_scope,
    source_asset_keys_covered_by_all_fetch_scope,
    source_raw_op_config_mappings,
)


def test_all_fetch_scope_covers_all_current_source_and_raw_assets() -> None:
    assert len(all_registered_source_asset_keys()) == 22
    assert len(all_registered_raw_asset_keys()) == 17
    assert source_asset_keys_covered_by_all_fetch_scope() == all_registered_source_asset_keys()
    assert raw_asset_keys_covered_by_all_fetch_scope() == all_registered_raw_asset_keys()


def test_all_raw_yearly_scope_excludes_snapshot_and_ocr_pipeline_assets() -> None:
    plan = build_backfill_plan(
        BackfillControllerConfig(
            target_scope=ALL_RAW_YEARLY_SCOPE,
            start_date="2024-01-01",
            end_date="2024-12-31",
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-001",
    )
    planned_assets = {asset_key for step in plan.steps for asset_key in step.asset_keys}

    assert "source/jiuyan__industry_images" not in planned_assets
    assert "source/jiuyan__industry_ocr" not in planned_assets
    assert "clickhouse/raw/sina__trade_calendar" not in planned_assets
    assert "clickhouse/raw/baostock__query_stock_basic" not in planned_assets
    assert "clickhouse/raw/jiuyan__industry_list" not in planned_assets
    assert "clickhouse/raw/jiuyan__industry_ocr_snapshot" not in planned_assets


def test_baostock_daily_kline_plan_matches_expected_year_order() -> None:
    plan = build_backfill_plan(
        BackfillControllerConfig(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            start_date="2024-01-01",
            end_date="2026-06-30",
            refresh_prerequisite_snapshots=True,
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-002",
    )

    assert [step.label for step in plan.steps] == [
        "snapshot prerequisites",
        "baostock daily source 2024",
        "baostock compacted source 2024",
        "baostock raw 2024",
        "baostock daily source 2025",
        "baostock compacted source 2025",
        "baostock raw 2025",
        "baostock daily source 2026",
        "baostock compacted source 2026",
        "baostock raw 2026",
    ]
    assert plan.year_partitions == ("2024", "2025", "2026")
    assert plan.steps[0].asset_keys == (
        "source/sina__trade_calendar",
        "source/baostock__query_stock_basic",
    )
    assert plan.steps[7].partition.partition_range_start == "2026-01-01"
    assert plan.steps[7].partition.partition_range_end == "2026-06-30"
    assert plan.steps[7].run_config == {
        "ops": {
            "source__baostock__query_history_k_data_plus_daily": {
                "config": {"cutoff_trade_date": "2026-06-30"}
            }
        }
    }
    assert plan.steps[8].run_config == {
        "ops": {
            "source__baostock__query_history_k_data_plus_daily_compacted": {
                "config": {"cutoff_trade_date": "2026-06-30"}
            }
        }
    }


def test_raw_only_plan_contains_only_raw_steps() -> None:
    plan = build_backfill_plan(
        BackfillControllerConfig(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            start_date="2024-01-01",
            end_date="2025-12-31",
            execution_mode=EXECUTION_MODE_RAW_ONLY,
            refresh_prerequisite_snapshots=True,
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-003",
    )

    assert [step.step_kind for step in plan.steps] == [STEP_RAW, STEP_RAW]
    assert all(
        step.asset_keys == ("clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted",)
        for step in plan.steps
    )


def test_all_fetch_dedupes_snapshot_source_materialization() -> None:
    plan = build_backfill_plan(
        BackfillControllerConfig(
            target_scope=ALL_FETCH_SOURCES_TO_RAW_SCOPE,
            start_date="2026-01-01",
            end_date="2026-06-30",
            refresh_prerequisite_snapshots=True,
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-004",
    )
    non_raw_asset_keys = [
        asset_key
        for step in plan.steps
        if step.step_kind != STEP_RAW
        for asset_key in step.asset_keys
    ]

    assert non_raw_asset_keys.count("source/sina__trade_calendar") == 1
    assert non_raw_asset_keys.count("source/baostock__query_stock_basic") == 1
    assert non_raw_asset_keys.count("source/jiuyan__industry_list") == 1


def test_jiuyan_ocr_plan_uses_safe_default_limit() -> None:
    plan = build_backfill_plan(
        BackfillControllerConfig(target_scope=JIUYAN_OCR_PIPELINE_SCOPE),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-005",
    )

    image_step = next(
        step for step in plan.steps if step.asset_keys == ("source/jiuyan__industry_images",)
    )
    ocr_step = next(
        step for step in plan.steps if step.asset_keys == ("source/jiuyan__industry_ocr",)
    )

    assert image_step.run_config == {
        "ops": {
            "source__jiuyan__industry_images": {
                "config": {
                    "limit": DEFAULT_JIUYAN_OCR_LIMIT,
                    "force_download": False,
                }
            }
        }
    }
    assert ocr_step.run_config == {
        "ops": {
            "source__jiuyan__industry_ocr": {
                "config": {
                    "limit": DEFAULT_JIUYAN_OCR_LIMIT,
                    "force_ocr": False,
                }
            }
        }
    }


def test_op_config_mappings_resolve_real_op_names() -> None:
    mappings = source_raw_op_config_mappings()

    assert mappings
    assert {mapping.asset_key for mapping in mappings} >= {
        "source/baostock__query_history_k_data_plus_daily",
        "source/baostock__query_history_k_data_plus_daily_compacted",
        "source/jiuyan__industry_images",
        "source/jiuyan__industry_ocr",
    }
    assert all(mapping.op_name for mapping in mappings)
    assert all(mapping.config_keys for mapping in mappings)


def test_execute_backfill_plan_stops_after_failed_step() -> None:
    plan = build_backfill_plan(
        BackfillControllerConfig(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            start_date="2024-01-01",
            end_date="2024-12-31",
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-006",
    )
    submitter = FakeSubmitter(fail_on_label="baostock compacted source 2024")

    with pytest.raises(RuntimeError, match="Backfill step failed"):
        execute_backfill_plan(plan, submitter=submitter, log=FakeLog())

    assert [step.label for step in submitter.submitted_steps] == [
        "baostock daily source 2024",
        "baostock compacted source 2024",
    ]


def test_execute_backfill_plan_rejects_non_terminal_status() -> None:
    plan = build_backfill_plan(
        BackfillControllerConfig(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            start_date="2024-01-01",
            end_date="2024-12-31",
            execution_mode=EXECUTION_MODE_RAW_ONLY,
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-006b",
    )

    with pytest.raises(RuntimeError, match="non-terminal status"):
        execute_backfill_plan(plan, submitter=NonTerminalSubmitter(), log=FakeLog())


def test_add_controller_run_tags_writes_common_backfill_tags() -> None:
    plan = build_backfill_plan(
        BackfillControllerConfig(
            target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
            start_date="2024-01-01",
            end_date="2024-12-31",
        ),
        today=date(2026, 6, 30),
        controller_run_id="controller-run-006c",
    )
    tag_writer = FakeRunTagWriter()

    add_controller_run_tags(tag_writer, run_id="controller-run-006c", plan=plan)

    assert tag_writer.run_id == "controller-run-006c"
    assert tag_writer.tags == {
        "backfill.kind": "fetch_sources_to_raw",
        "backfill.id": "baostock_daily_kline-2024-01-01-2024-12-31-controllerru",
        "backfill.target_scope": BAOSTOCK_DAILY_KLINE_SCOPE,
        "backfill.start_date": "2024-01-01",
        "backfill.end_date": "2024-12-31",
    }


def test_invalid_scope_and_date_range_fail_explicitly() -> None:
    with pytest.raises(ValueError, match="Unsupported target_scope"):
        build_backfill_plan(
            BackfillControllerConfig(target_scope="unknown"),
            today=date(2026, 6, 30),
            controller_run_id="controller-run-006",
        )

    with pytest.raises(ValueError, match="requires start_date and end_date"):
        build_backfill_plan(
            BackfillControllerConfig(target_scope=BAOSTOCK_DAILY_KLINE_SCOPE),
            today=date(2026, 6, 30),
            controller_run_id="controller-run-007",
        )

    with pytest.raises(ValueError, match="start_date cannot be later"):
        build_backfill_plan(
            BackfillControllerConfig(
                target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
                start_date="2025-01-01",
                end_date="2024-01-01",
            ),
            today=date(2026, 6, 30),
            controller_run_id="controller-run-008",
        )

    with pytest.raises(ValueError, match="end_date cannot be later"):
        build_backfill_plan(
            BackfillControllerConfig(
                target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
                start_date="2026-01-01",
                end_date="2026-07-01",
            ),
            today=date(2026, 6, 30),
            controller_run_id="controller-run-009",
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


class NonTerminalSubmitter(BackfillRunSubmitter):
    def submit_step(self, step: BackfillStep) -> BackfillStepResult:
        _ = step
        return BackfillStepResult(run_id="run-started", status="STARTED", success=False)


class FakeRunTagWriter:
    def __init__(self) -> None:
        self.run_id: str | None = None
        self.tags: dict[str, str] = {}

    def add_run_tags(self, run_id: str, new_tags: Mapping[str, str]) -> None:
        self.run_id = run_id
        self.tags = dict(new_tags)


class FakeLog:
    def info(self, msg: object, *_args: object) -> None:
        _ = msg
        return None
