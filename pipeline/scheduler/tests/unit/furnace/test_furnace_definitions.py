from __future__ import annotations

from typing import cast

import dagster as dg
import pytest
from scheduler.defs.furnace.assets import (
    FURNACE_BOLL_ASSET_KEY,
    FURNACE_BOLL_GROUP,
    FURNACE_BOLL_UPSTREAM_ASSET_KEY,
    FURNACE_KDJ_ASSET_KEY,
    FURNACE_KDJ_GROUP,
    FURNACE_KDJ_UPSTREAM_ASSET_KEY,
    FURNACE_MA_ASSET_KEY,
    FURNACE_MA_GROUP,
    FURNACE_MA_UPSTREAM_ASSET_KEY,
    FURNACE_MA_VOLUME_UPSTREAM_ASSET_KEY,
    FURNACE_RSI_ASSET_KEY,
    FURNACE_RSI_GROUP,
    FURNACE_RSI_UPSTREAM_ASSET_KEY,
    FurnaceBollRunConfig,
    FurnaceKdjRunConfig,
    FurnaceMaRunConfig,
    FurnaceRsiRunConfig,
    _metadata_from_summary,
)
from scheduler.defs.furnace.definitions import build_furnace_defs, build_furnace_jobs


def _asset_for_key(loaded_defs: dg.Definitions, key: dg.AssetKey) -> dg.AssetsDefinition:
    for candidate in loaded_defs.assets or []:
        asset = cast("dg.AssetsDefinition", candidate)
        if key in asset.keys:
            return asset
    raise AssertionError(f"Furnace asset is not registered: {key.to_user_string()}")


def test_furnace_assets_set_key_group_deps_and_tags() -> None:
    loaded_defs = build_furnace_defs()
    cases = [
        (FURNACE_KDJ_ASSET_KEY, FURNACE_KDJ_GROUP, {FURNACE_KDJ_UPSTREAM_ASSET_KEY}),
        (
            FURNACE_MA_ASSET_KEY,
            FURNACE_MA_GROUP,
            {FURNACE_MA_UPSTREAM_ASSET_KEY, FURNACE_MA_VOLUME_UPSTREAM_ASSET_KEY},
        ),
        (FURNACE_RSI_ASSET_KEY, FURNACE_RSI_GROUP, {FURNACE_RSI_UPSTREAM_ASSET_KEY}),
        (FURNACE_BOLL_ASSET_KEY, FURNACE_BOLL_GROUP, {FURNACE_BOLL_UPSTREAM_ASSET_KEY}),
    ]

    for key, group, deps in cases:
        asset = _asset_for_key(loaded_defs, key)
        assert asset.key == key
        assert asset.group_names_by_key[key] == group
        assert asset.dependency_keys == deps
        assert {
            tag_key: asset.tags_by_key[key][tag_key]
            for tag_key in ("owner", "layer", "storage", "modality")
        } == {
            "owner": "furnace",
            "layer": "calculation",
            "storage": "clickhouse",
            "modality": "batch",
        }
        assert asset.partitions_def is None


def test_furnace_jobs_select_expected_assets() -> None:
    jobs = build_furnace_jobs()

    assert {job.name for job in jobs} == {
        "furnace__kdj_daily_job",
        "furnace__kdj_backfill_job",
        "furnace__kdj_dry_run_job",
        "furnace__ma_daily_job",
        "furnace__ma_backfill_job",
        "furnace__ma_dry_run_job",
        "furnace__rsi_daily_job",
        "furnace__rsi_backfill_job",
        "furnace__rsi_dry_run_job",
        "furnace__boll_daily_job",
        "furnace__boll_backfill_job",
        "furnace__boll_dry_run_job",
    }
    selections_by_name = {job.name: str(job.selection) for job in jobs}
    assert selections_by_name["furnace__kdj_daily_job"] == (
        'key:"fleur_calculation/calc_stock_kdj_daily"'
    )
    assert selections_by_name["furnace__ma_daily_job"] == (
        'key:"fleur_calculation/calc_stock_ma_daily"'
    )
    assert selections_by_name["furnace__rsi_daily_job"] == (
        'key:"fleur_calculation/calc_stock_rsi_daily"'
    )
    assert selections_by_name["furnace__boll_daily_job"] == (
        'key:"fleur_calculation/calc_stock_boll_daily"'
    )


def test_furnace_daily_schedules_use_append_latest_config() -> None:
    loaded_defs = build_furnace_defs()
    schedules = {
        schedule.name: cast("dg.ScheduleDefinition", schedule)
        for schedule in loaded_defs.schedules or []
    }

    kdj_tick = schedules["furnace__kdj_daily_schedule"].evaluate_tick(dg.build_schedule_context())
    assert kdj_tick.run_requests is not None
    assert (
        kdj_tick.run_requests[0].run_config["ops"]["fleur_calculation__calc_stock_kdj_daily"][
            "config"
        ]["mode"]
        == "append-latest"
    )

    ma_tick = schedules["furnace__ma_daily_schedule"].evaluate_tick(dg.build_schedule_context())
    assert ma_tick.run_requests is not None
    ma_config = ma_tick.run_requests[0].run_config["ops"]["fleur_calculation__calc_stock_ma_daily"][
        "config"
    ]
    assert ma_config["mode"] == "append-latest"
    assert ma_config["volume_input_table"] == "fleur_intermediate.int_stock_quotes_daily_unadj"
    assert ma_config["price_column"] == "close_price_forward_adj"
    assert ma_config["volume_column"] == "volume"

    rsi_tick = schedules["furnace__rsi_daily_schedule"].evaluate_tick(dg.build_schedule_context())
    assert rsi_tick.run_requests is not None
    rsi_config = rsi_tick.run_requests[0].run_config["ops"]["furnace__calc_stock_rsi_daily"][
        "config"
    ]
    assert rsi_config["mode"] == "append-latest"
    assert rsi_config["price_column"] == "close_price_forward_adj"

    boll_tick = schedules["furnace__boll_daily_schedule"].evaluate_tick(dg.build_schedule_context())
    assert boll_tick.run_requests is not None
    boll_config = boll_tick.run_requests[0].run_config["ops"]["furnace__calc_stock_boll_daily"][
        "config"
    ]
    assert boll_config["mode"] == "append-latest"
    assert boll_config["output_table"] == "fleur_calculation.calc_stock_boll_daily"
    assert boll_config["price_column"] == "close_price_forward_adj"
    assert boll_config["insert_batch_size"] == 10_000


def test_furnace_configs_reject_unknown_mode() -> None:
    cases = [
        (
            FurnaceKdjRunConfig(
                request_from="2026-01-01", request_to="2026-01-02", mode="bad-mode"
            ),
            "KDJ",
        ),
        (
            FurnaceMaRunConfig(request_from="2026-01-01", request_to="2026-01-02", mode="bad-mode"),
            "MA",
        ),
        (
            FurnaceRsiRunConfig(
                request_from="2026-01-01", request_to="2026-01-02", mode="bad-mode"
            ),
            "RSI",
        ),
        (
            FurnaceBollRunConfig(
                request_from="2026-01-01", request_to="2026-01-02", mode="bad-mode"
            ),
            "BOLL",
        ),
    ]

    for config, name in cases:
        with pytest.raises(ValueError, match=f"Unsupported Furnace {name} mode"):
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
            "valid_volume_rows": 17,
            "null_indicator_rows": 4,
            "affected_years": [2026],
            "retained_rows": 80,
            "price_ma_windows": [3, 5, 6, 10, 12, 14, 20, 24, 28, 57, 60, 114, 250],
            "volume_ma_windows": [5, 10, 20, 60],
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
    assert metadata["valid_volume_rows"] == 17
    assert metadata["ema_state_source"] == "full-history"
    assert metadata["price_ma_windows"][-1] == 250
    assert metadata["volume_ma_windows"] == [5, 10, 20, 60]


def test_furnace_metadata_maps_rsi_summary_to_materialization_metadata() -> None:
    metadata = _metadata_from_summary(
        {
            "indicator": "rsi",
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
            "rsi_windows": [6, 12, 14, 24, 25, 50],
            "rsi_state_source": "full-history",
            "gap_symbols_count": 0,
            "gap_fill_from": None,
            "staging_validation": {"status": "passed"},
            "partition_replace": {"status": "not_applied"},
            "performance_metrics": {"parallelism": "rayon", "compute_ms": 12},
            "writes_applied": False,
        }
    )

    assert metadata["indicator"] == "rsi"
    assert metadata["valid_close_rows"] == 18
    assert metadata["rsi_state_source"] == "full-history"
    assert metadata["rsi_windows"][-1] == 50
    assert metadata["gap_symbols_count"] == 0


def test_furnace_metadata_maps_boll_summary_to_materialization_metadata() -> None:
    metadata = _metadata_from_summary(
        {
            "indicator": "boll",
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
            "input_valid_close_rows": 95,
            "output_valid_close_rows": 18,
            "null_indicator_rows": 4,
            "affected_years": [2026],
            "retained_rows": 80,
            "boll_configs": [{"window": 20, "multiplier": 2.0, "field_suffix": "20_2"}],
            "max_window": 50,
            "stddev_ddof": 0,
            "state_source": "rolling-lookback",
            "staging_validation": {"status": "passed"},
            "partition_replace": {"status": "not_applied"},
            "performance_metrics": {"parallelism": "rayon", "compute_ms": 12},
            "writes_applied": False,
        }
    )

    assert metadata["indicator"] == "boll"
    assert metadata["output_valid_close_rows"] == 18
    assert metadata["max_window"] == 50
    assert metadata["stddev_ddof"] == 0
    assert metadata["boll_configs"][0]["field_suffix"] == "20_2"
