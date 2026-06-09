from __future__ import annotations

from datetime import UTC, datetime

import dagster as dg

from scheduler.defs.furnace.assets import (
    FURNACE_ASSETS,
    FURNACE_BOLL_ASSET_KEY,
    FURNACE_KDJ_ASSET_KEY,
    FURNACE_MA_ASSET_KEY,
    FURNACE_PRICE_PATTERN_ASSET_KEY,
    FURNACE_RSI_ASSET_KEY,
)
from scheduler.defs.resources.furnace import FurnaceCliResource


def build_furnace_defs(
    *,
    binary_path: str = "engines/target/debug/furnace",
    working_dir: str = ".",
    daily_cron_schedule: str = "45 18 * * *",
    rayon_num_threads: int | None = 8,
) -> dg.Definitions:
    jobs = build_furnace_jobs()
    return dg.Definitions(
        assets=list(FURNACE_ASSETS),
        jobs=list(jobs),
        schedules=[
            dg.ScheduleDefinition(
                name="furnace__kdj_daily_schedule",
                job=jobs[0],
                cron_schedule=daily_cron_schedule,
                run_config_fn=_kdj_daily_run_config,
            ),
            dg.ScheduleDefinition(
                name="furnace__ma_daily_schedule",
                job=jobs[3],
                cron_schedule=daily_cron_schedule,
                run_config_fn=_ma_daily_run_config,
            ),
            dg.ScheduleDefinition(
                name="furnace__rsi_daily_schedule",
                job=jobs[6],
                cron_schedule=daily_cron_schedule,
                run_config_fn=_rsi_daily_run_config,
            ),
            dg.ScheduleDefinition(
                name="furnace__boll_daily_schedule",
                job=jobs[9],
                cron_schedule=daily_cron_schedule,
                run_config_fn=_boll_daily_run_config,
            ),
            dg.ScheduleDefinition(
                name="furnace__price_pattern_daily_schedule",
                job=jobs[12],
                cron_schedule=daily_cron_schedule,
                run_config_fn=_price_pattern_daily_run_config,
            ),
        ],
        resources={
            "furnace_cli": FurnaceCliResource(
                binary_path=binary_path,
                working_dir=working_dir,
                rayon_num_threads=rayon_num_threads,
            )
        },
    )


def build_furnace_jobs() -> tuple[dg.UnresolvedAssetJobDefinition, ...]:
    kdj_selection = dg.AssetSelection.assets(FURNACE_KDJ_ASSET_KEY)
    ma_selection = dg.AssetSelection.assets(FURNACE_MA_ASSET_KEY)
    rsi_selection = dg.AssetSelection.assets(FURNACE_RSI_ASSET_KEY)
    boll_selection = dg.AssetSelection.assets(FURNACE_BOLL_ASSET_KEY)
    price_pattern_selection = dg.AssetSelection.assets(FURNACE_PRICE_PATTERN_ASSET_KEY)
    return (
        dg.define_asset_job(name="furnace__kdj_daily_job", selection=kdj_selection),
        dg.define_asset_job(name="furnace__kdj_backfill_job", selection=kdj_selection),
        dg.define_asset_job(name="furnace__kdj_dry_run_job", selection=kdj_selection),
        dg.define_asset_job(name="furnace__ma_daily_job", selection=ma_selection),
        dg.define_asset_job(name="furnace__ma_backfill_job", selection=ma_selection),
        dg.define_asset_job(name="furnace__ma_dry_run_job", selection=ma_selection),
        dg.define_asset_job(name="furnace__rsi_daily_job", selection=rsi_selection),
        dg.define_asset_job(name="furnace__rsi_backfill_job", selection=rsi_selection),
        dg.define_asset_job(name="furnace__rsi_dry_run_job", selection=rsi_selection),
        dg.define_asset_job(name="furnace__boll_daily_job", selection=boll_selection),
        dg.define_asset_job(name="furnace__boll_backfill_job", selection=boll_selection),
        dg.define_asset_job(name="furnace__boll_dry_run_job", selection=boll_selection),
        dg.define_asset_job(
            name="furnace__price_pattern_daily_job", selection=price_pattern_selection
        ),
        dg.define_asset_job(
            name="furnace__price_pattern_backfill_job", selection=price_pattern_selection
        ),
        dg.define_asset_job(
            name="furnace__price_pattern_dry_run_job", selection=price_pattern_selection
        ),
    )


def _kdj_daily_run_config(context: dg.ScheduleEvaluationContext) -> dict[str, object]:
    try:
        scheduled_time = context.scheduled_execution_time
    except Exception:
        scheduled_time = datetime.now(tz=UTC)
    trade_date = scheduled_time.date().isoformat()
    return {
        "ops": {
            "fleur_calculation__calc_stock_kdj_daily": {
                "config": {
                    "request_from": trade_date,
                    "request_to": trade_date,
                    "mode": "append-latest",
                    "symbols": [],
                    "rsv_window": 9,
                    "k_smoothing": 3,
                    "d_smoothing": 3,
                    "insert_batch_size": 10_000,
                }
            }
        }
    }


def _ma_daily_run_config(context: dg.ScheduleEvaluationContext) -> dict[str, object]:
    try:
        scheduled_time = context.scheduled_execution_time
    except Exception:
        scheduled_time = datetime.now(tz=UTC)
    trade_date = scheduled_time.date().isoformat()
    return {
        "ops": {
            "fleur_calculation__calc_stock_ma_daily": {
                "config": {
                    "request_from": trade_date,
                    "request_to": trade_date,
                    "mode": "append-latest",
                    "symbols": [],
                    "input_table": "fleur_intermediate.int_stock_quotes_daily_adj",
                    "volume_input_table": "fleur_intermediate.int_stock_quotes_daily_unadj",
                    "output_table": "fleur_calculation.calc_stock_ma_daily",
                    "price_column": "close_price_forward_adj",
                    "volume_column": "volume",
                    "insert_batch_size": 10_000,
                }
            }
        }
    }


def _rsi_daily_run_config(context: dg.ScheduleEvaluationContext) -> dict[str, object]:
    try:
        scheduled_time = context.scheduled_execution_time
    except Exception:
        scheduled_time = datetime.now(tz=UTC)
    trade_date = scheduled_time.date().isoformat()
    return {
        "ops": {
            "furnace__calc_stock_rsi_daily": {
                "config": {
                    "request_from": trade_date,
                    "request_to": trade_date,
                    "mode": "append-latest",
                    "symbols": [],
                    "input_table": "fleur_intermediate.int_stock_quotes_daily_adj",
                    "output_table": "fleur_calculation.calc_stock_rsi_daily",
                    "price_column": "close_price_forward_adj",
                    "insert_batch_size": 10_000,
                }
            }
        }
    }


def _boll_daily_run_config(context: dg.ScheduleEvaluationContext) -> dict[str, object]:
    try:
        scheduled_time = context.scheduled_execution_time
    except Exception:
        scheduled_time = datetime.now(tz=UTC)
    trade_date = scheduled_time.date().isoformat()
    return {
        "ops": {
            "furnace__calc_stock_boll_daily": {
                "config": {
                    "request_from": trade_date,
                    "request_to": trade_date,
                    "mode": "append-latest",
                    "symbols": [],
                    "input_table": "fleur_intermediate.int_stock_quotes_daily_adj",
                    "output_table": "fleur_calculation.calc_stock_boll_daily",
                    "price_column": "close_price_forward_adj",
                    "insert_batch_size": 10_000,
                }
            }
        }
    }


def _price_pattern_daily_run_config(context: dg.ScheduleEvaluationContext) -> dict[str, object]:
    try:
        scheduled_time = context.scheduled_execution_time
    except Exception:
        scheduled_time = datetime.now(tz=UTC)
    trade_date = scheduled_time.date().isoformat()
    return {
        "ops": {
            "furnace__calc_stock_price_pattern_daily": {
                "config": {
                    "request_from": trade_date,
                    "request_to": trade_date,
                    "mode": "append-latest",
                    "symbols": [],
                    "structure_input_table": "fleur_intermediate.int_stock_quotes_daily_adj",
                    "streak_input_table": "fleur_intermediate.int_stock_quotes_daily_unadj",
                    "output_table": "fleur_calculation.calc_stock_price_pattern_daily",
                    "high_column": "high_price_forward_adj",
                    "low_column": "low_price_forward_adj",
                    "close_column": "close_price",
                    "prev_close_column": "prev_close_price",
                    "insert_batch_size": 10_000,
                }
            }
        }
    }


FURNACE_DEFS = build_furnace_defs()
