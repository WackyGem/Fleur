from __future__ import annotations

from typing import cast

import dagster as dg
import pytest
from scheduler.defs.dbt_jobs import (
    STOCK_JOBS,
    TRANSFORMATION_SCHEDULES,
    TRANSFORMATION_SENSORS,
    baostock_raw_sync_success_triggers_stock_daily_build,
)
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
    FURNACE_MACD_ASSET_KEY,
    FURNACE_MACD_GROUP,
    FURNACE_MACD_UPSTREAM_ASSET_KEY,
    FURNACE_PRICE_PATTERN_ASSET_KEY,
    FURNACE_PRICE_PATTERN_GROUP,
    FURNACE_PRICE_PATTERN_STREAK_UPSTREAM_ASSET_KEY,
    FURNACE_PRICE_PATTERN_STRUCTURE_UPSTREAM_ASSET_KEY,
    FURNACE_RSI_ASSET_KEY,
    FURNACE_RSI_GROUP,
    FURNACE_RSI_UPSTREAM_ASSET_KEY,
    FurnaceBollRunConfig,
    FurnaceKdjRunConfig,
    FurnaceMacdRunConfig,
    FurnaceMaRunConfig,
    FurnacePricePatternRunConfig,
    FurnaceRsiRunConfig,
    _metadata_from_summary,
)
from scheduler.defs.furnace.definitions import build_furnace_defs


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
        (
            FURNACE_MACD_ASSET_KEY,
            FURNACE_MACD_GROUP,
            {FURNACE_MACD_UPSTREAM_ASSET_KEY},
        ),
        (
            FURNACE_PRICE_PATTERN_ASSET_KEY,
            FURNACE_PRICE_PATTERN_GROUP,
            {
                FURNACE_PRICE_PATTERN_STRUCTURE_UPSTREAM_ASSET_KEY,
                FURNACE_PRICE_PATTERN_STREAK_UPSTREAM_ASSET_KEY,
            },
        ),
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


def test_furnace_defs_only_register_assets_and_resource() -> None:
    loaded_defs = build_furnace_defs()

    assert loaded_defs.jobs is None
    assert loaded_defs.schedules is None
    assert loaded_defs.resources is not None
    assert set(loaded_defs.resources) == {"furnace_cli"}


def test_stock_daily_job_selects_dbt_calculation_and_mart_assets() -> None:
    assert {job.name for job in STOCK_JOBS} == {"stock__daily_build_job"}
    selection = str(STOCK_JOBS[0].selection)

    assert 'group:"calculation"' in selection
    assert 'group:"dbt_staging"' not in selection
    assert 'group:"dbt_intermediate"' not in selection
    assert 'group:"dbt_marts"' not in selection
    assert 'key:"int_stock_quotes_daily_unadj"' in selection
    assert 'key:"int_stock_adjustment_factor"' in selection
    assert 'key:"int_stock_quotes_daily_adj"' in selection
    assert 'key:"int_stock_kdj_daily"' in selection
    assert 'key:"mart_stock_quotes_daily"' in selection


def test_baostock_raw_sync_success_sensor_launches_stock_daily_job() -> None:
    assert {sensor.name for sensor in TRANSFORMATION_SENSORS} == {
        "baostock_raw_sync_success_triggers_stock_daily_build"
    }
    assert (
        baostock_raw_sync_success_triggers_stock_daily_build.default_status
        == dg.DefaultSensorStatus.RUNNING
    )
    run = dg.DagsterRun(
        job_name="clickhouse__raw_sync_baostock_job",
        run_id="raw-run-1",
    )
    event = dg.DagsterEvent(
        event_type_value=dg.DagsterEventType.RUN_SUCCESS.value,
        job_name="clickhouse__raw_sync_baostock_job",
    )
    context = dg.build_run_status_sensor_context(
        sensor_name="baostock_raw_sync_success_triggers_stock_daily_build",
        dagster_event=event,
        dagster_instance=dg.DagsterInstance.ephemeral(),
        dagster_run=run,
        partition_key="2026",
    )

    request = cast(dg.RunRequest, baostock_raw_sync_success_triggers_stock_daily_build(context))

    assert request.run_key == "stock-daily-after-baostock-raw-sync:raw-run-1"
    assert request.tags == {
        "trigger": "baostock_raw_sync_success",
        "upstream_job": "clickhouse__raw_sync_baostock_job",
        "upstream_run_id": "raw-run-1",
        "upstream_partition_key": "2026",
        "stock_daily_trade_date": request.tags["stock_daily_trade_date"],
    }
    assert "_sync_at" not in str(request.run_config)
    assert (
        request.run_config["ops"]["fleur_calculation__calc_stock_kdj_daily"]["config"]["mode"]
        == "append-latest"
    )


def test_stock_daily_schedule_uses_append_latest_furnace_config() -> None:
    schedules = {schedule.name: schedule for schedule in TRANSFORMATION_SCHEDULES}

    tick = schedules["stock__daily_build_schedule"].evaluate_tick(dg.build_schedule_context())
    assert tick.run_requests is not None
    run_config = tick.run_requests[0].run_config
    assert (
        run_config["ops"]["fleur_calculation__calc_stock_kdj_daily"]["config"]["mode"]
        == "append-latest"
    )

    ma_config = run_config["ops"]["fleur_calculation__calc_stock_ma_daily"]["config"]
    assert ma_config["mode"] == "append-latest"
    assert ma_config["volume_input_table"] == "fleur_intermediate.int_stock_quotes_daily_unadj"
    assert ma_config["price_column"] == "close_price_forward_adj"
    assert ma_config["volume_column"] == "volume"

    rsi_config = run_config["ops"]["furnace__calc_stock_rsi_daily"]["config"]
    assert rsi_config["mode"] == "append-latest"
    assert rsi_config["price_column"] == "close_price_forward_adj"

    boll_config = run_config["ops"]["furnace__calc_stock_boll_daily"]["config"]
    assert boll_config["mode"] == "append-latest"
    assert boll_config["output_table"] == "fleur_calculation.calc_stock_boll_daily"
    assert boll_config["price_column"] == "close_price_forward_adj"
    assert boll_config["insert_batch_size"] == 10_000

    macd_config = run_config["ops"]["fleur_calculation__calc_stock_macd_daily"]["config"]
    assert macd_config["mode"] == "append-latest"
    assert macd_config["input_table"] == "fleur_intermediate.int_stock_quotes_daily_adj"
    assert macd_config["output_table"] == "fleur_calculation.calc_stock_macd_daily"
    assert macd_config["price_column"] == "close_price_forward_adj"
    assert macd_config["insert_batch_size"] == 10_000

    price_pattern_config = run_config["ops"]["furnace__calc_stock_price_pattern_daily"]["config"]
    assert price_pattern_config["mode"] == "append-latest"
    assert (
        price_pattern_config["structure_input_table"]
        == "fleur_intermediate.int_stock_quotes_daily_adj"
    )
    assert (
        price_pattern_config["streak_input_table"]
        == "fleur_intermediate.int_stock_quotes_daily_unadj"
    )
    assert price_pattern_config["output_table"] == (
        "fleur_calculation.calc_stock_price_pattern_daily"
    )
    assert price_pattern_config["high_column"] == "high_price_forward_adj"
    assert price_pattern_config["low_column"] == "low_price_forward_adj"
    assert price_pattern_config["close_column"] == "close_price"
    assert price_pattern_config["prev_close_column"] == "prev_close_price"
    assert price_pattern_config["insert_batch_size"] == 10_000


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
        (
            FurnaceMacdRunConfig(
                request_from="2026-01-01", request_to="2026-01-02", mode="bad-mode"
            ),
            "MACD",
        ),
        (
            FurnacePricePatternRunConfig(
                request_from="2026-01-01", request_to="2026-01-02", mode="bad-mode"
            ),
            "Price Pattern",
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
    assert metadata["scheduler_version"] == "0.1.0"
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


def test_furnace_metadata_maps_macd_summary_to_materialization_metadata() -> None:
    metadata = _metadata_from_summary(
        {
            "indicator": "macd",
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
            "macd_params": {"fast_window": 12, "slow_window": 26, "signal_window": 9},
            "histogram_mode": "DIF - DEA",
            "macd_state_source": "full-history",
            "incomplete_state_symbols_count": 1,
            "gap_symbols_count": 0,
            "gap_fill_from": None,
            "staging_validation": {"status": "passed"},
            "partition_replace": {"status": "not_applied"},
            "performance_metrics": {"parallelism": "rayon", "compute_ms": 12},
            "writes_applied": False,
        }
    )

    assert metadata["indicator"] == "macd"
    assert metadata["valid_close_rows"] == 18
    assert metadata["macd_params"] == {"fast_window": 12, "slow_window": 26, "signal_window": 9}
    assert metadata["histogram_mode"] == "DIF - DEA"
    assert metadata["macd_state_source"] == "full-history"
    assert metadata["incomplete_state_symbols_count"] == 1
    assert metadata["gap_symbols_count"] == 0


def test_furnace_metadata_maps_price_pattern_summary_to_materialization_metadata() -> None:
    metadata = _metadata_from_summary(
        {
            "indicator": "price_pattern",
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
            "input_valid_streak_rows": 95,
            "input_valid_structure_bar_rows": 96,
            "valid_streak_rows": 18,
            "valid_structure_bar_rows": 19,
            "null_streak_rows": 2,
            "null_second_low_rows": 4,
            "affected_years": [2026],
            "retained_rows": 80,
            "state_source": "full-history",
            "n_structure_window": 20,
            "staging_validation": {"status": "passed"},
            "partition_replace": {"status": "not_applied"},
            "performance_metrics": {"parallelism": "rayon", "compute_ms": 12},
            "writes_applied": False,
        }
    )

    assert metadata["indicator"] == "price_pattern"
    assert metadata["valid_streak_rows"] == 18
    assert metadata["valid_structure_bar_rows"] == 19
    assert metadata["null_streak_rows"] == 2
    assert metadata["null_second_low_rows"] == 4
    assert metadata["state_source"] == "full-history"
    assert metadata["n_structure_window"] == 20
