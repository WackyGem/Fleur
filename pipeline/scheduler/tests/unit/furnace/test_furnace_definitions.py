from __future__ import annotations

from typing import cast

import dagster as dg
import pytest
from scheduler.defs.furnace.assets import (
    FURNACE_KDJ_ASSET_KEY,
    FURNACE_KDJ_GROUP,
    FURNACE_KDJ_UPSTREAM_ASSET_KEY,
    FURNACE_MA_ASSET_KEY,
    FURNACE_MA_GROUP,
    FURNACE_MA_UPSTREAM_ASSET_KEY,
    FurnaceKdjRunConfig,
    FurnaceMaRunConfig,
    _metadata_from_summary,
)
from scheduler.defs.furnace.definitions import build_furnace_defs, build_furnace_jobs


def _furnace_kdj_asset(loaded_defs: dg.Definitions) -> dg.AssetsDefinition:
    for candidate in loaded_defs.assets or []:
        asset = cast("dg.AssetsDefinition", candidate)
        if FURNACE_KDJ_ASSET_KEY in asset.keys:
            return asset
    raise AssertionError("Furnace KDJ asset is not registered")


def _furnace_ma_asset(loaded_defs: dg.Definitions) -> dg.AssetsDefinition:
    for candidate in loaded_defs.assets or []:
        asset = cast("dg.AssetsDefinition", candidate)
        if FURNACE_MA_ASSET_KEY in asset.keys:
            return asset
    raise AssertionError("Furnace MA asset is not registered")


def test_furnace_kdj_asset_sets_key_group_deps_and_tags() -> None:
    loaded_defs = build_furnace_defs()
    asset = _furnace_kdj_asset(loaded_defs)

    assert asset.key == FURNACE_KDJ_ASSET_KEY
    assert asset.group_names_by_key[FURNACE_KDJ_ASSET_KEY] == FURNACE_KDJ_GROUP
    assert asset.dependency_keys == {FURNACE_KDJ_UPSTREAM_ASSET_KEY}
    assert {
        key: asset.tags_by_key[FURNACE_KDJ_ASSET_KEY][key]
        for key in ("owner", "layer", "storage", "modality")
    } == {
        "owner": "furnace",
        "layer": "calculation",
        "storage": "clickhouse",
        "modality": "batch",
    }


def test_furnace_kdj_asset_is_not_partitioned() -> None:
    loaded_defs = build_furnace_defs()
    asset = _furnace_kdj_asset(loaded_defs)

    assert asset.partitions_def is None


def test_furnace_ma_asset_sets_key_group_deps_and_tags() -> None:
    loaded_defs = build_furnace_defs()
    asset = _furnace_ma_asset(loaded_defs)

    assert asset.key == FURNACE_MA_ASSET_KEY
    assert asset.group_names_by_key[FURNACE_MA_ASSET_KEY] == FURNACE_MA_GROUP
    assert asset.dependency_keys == {FURNACE_MA_UPSTREAM_ASSET_KEY}
    assert {
        key: asset.tags_by_key[FURNACE_MA_ASSET_KEY][key]
        for key in ("owner", "layer", "storage", "modality")
    } == {
        "owner": "furnace",
        "layer": "calculation",
        "storage": "clickhouse",
        "modality": "batch",
    }


def test_furnace_jobs_select_expected_assets() -> None:
    jobs = build_furnace_jobs()

    assert {job.name for job in jobs} == {
        "furnace__kdj_daily_job",
        "furnace__kdj_backfill_job",
        "furnace__kdj_dry_run_job",
        "furnace__ma_daily_job",
        "furnace__ma_backfill_job",
        "furnace__ma_dry_run_job",
    }
    selections_by_name = {job.name: str(job.selection) for job in jobs}
    assert selections_by_name["furnace__kdj_daily_job"] == (
        'key:"fleur_calculation/calc_stock_kdj_daily"'
    )
    assert selections_by_name["furnace__ma_daily_job"] == (
        'key:"fleur_calculation/calc_stock_ma_daily"'
    )


def test_furnace_daily_schedule_uses_append_latest_config() -> None:
    loaded_defs = build_furnace_defs()
    schedule = next(
        schedule
        for schedule in loaded_defs.schedules or []
        if schedule.name == "furnace__kdj_daily_schedule"
    )
    schedule = cast("dg.ScheduleDefinition", schedule)

    tick = schedule.evaluate_tick(dg.build_schedule_context())

    assert tick.run_requests is not None
    assert (
        tick.run_requests[0].run_config["ops"]["furnace__calc_stock_kdj_daily"]["config"]["mode"]
        == "append-latest"
    )


def test_furnace_ma_daily_schedule_uses_append_latest_config() -> None:
    loaded_defs = build_furnace_defs()
    schedule = next(
        schedule
        for schedule in loaded_defs.schedules or []
        if schedule.name == "furnace__ma_daily_schedule"
    )
    schedule = cast("dg.ScheduleDefinition", schedule)

    tick = schedule.evaluate_tick(dg.build_schedule_context())

    assert tick.run_requests is not None
    config = tick.run_requests[0].run_config["ops"]["furnace__calc_stock_ma_daily"]["config"]
    assert config["mode"] == "append-latest"
    assert config["price_column"] == "close_price_forward_adj"


def test_furnace_kdj_config_rejects_unknown_mode() -> None:
    config = FurnaceKdjRunConfig(
        request_from="2026-01-01",
        request_to="2026-01-02",
        mode="bad-mode",
    )

    with pytest.raises(ValueError, match="Unsupported Furnace KDJ mode"):
        config.to_cli_request(run_id="run-1")


def test_furnace_ma_config_rejects_unknown_mode() -> None:
    config = FurnaceMaRunConfig(
        request_from="2026-01-01",
        request_to="2026-01-02",
        mode="bad-mode",
    )

    with pytest.raises(ValueError, match="Unsupported Furnace MA mode"):
        config.to_cli_request(run_id="run-1")


def test_furnace_metadata_maps_cli_summary_to_materialization_metadata() -> None:
    metadata = _metadata_from_summary(
        {
            "request_from": "2026-01-01",
            "request_to": "2026-01-02",
            "effective_output_from": "2026-01-01",
            "effective_output_to": "2026-01-02",
            "input_from": "2025-12-01",
            "input_to": "2026-01-02",
            "mode": "dry-run",
            "symbols_count": 2,
            "input_rows": 100,
            "output_rows": 20,
            "null_indicator_rows": 4,
            "affected_years": [2026],
            "retained_rows": 80,
            "kdj_params": {"rsv_window": 9, "k_smoothing": 3, "d_smoothing": 3},
            "state_source": "previous_materialization",
            "staging_validation": {"status": "passed"},
            "partition_replace": {"status": "not_applied"},
            "performance_metrics": {"parallelism": "rayon", "compute_ms": 12},
            "writes_applied": False,
        }
    )

    assert metadata["effective_output_range"] == {"from": "2026-01-01", "to": "2026-01-02"}
    assert metadata["output_rows"] == 20
    assert metadata["performance_metrics"] == {"parallelism": "rayon", "compute_ms": 12}


def test_furnace_metadata_maps_ma_summary_to_materialization_metadata() -> None:
    metadata = _metadata_from_summary(
        {
            "indicator": "ma",
            "request_from": "2026-01-01",
            "request_to": "2026-01-02",
            "effective_output_from": "2026-01-01",
            "effective_output_to": "2026-01-02",
            "input_from": "1990-12-19",
            "input_to": "2026-01-02",
            "mode": "dry-run",
            "symbols_count": 2,
            "input_rows": 100,
            "output_rows": 20,
            "valid_close_rows": 18,
            "null_indicator_rows": 4,
            "affected_years": [2026],
            "retained_rows": 80,
            "ma_windows": [3, 5, 6, 10, 12, 14, 20, 24, 28, 57, 60, 114, 250],
            "ema_window": 10,
            "ema_state_source": "full-history",
            "staging_validation": {"status": "passed"},
            "partition_replace": {"status": "not_applied"},
            "performance_metrics": {"parallelism": "rayon", "compute_ms": 12},
            "writes_applied": False,
        }
    )

    assert metadata["indicator"] == "ma"
    assert metadata["valid_close_rows"] == 18
    assert metadata["ema_state_source"] == "full-history"
    assert metadata["ma_windows"][-1] == 250
