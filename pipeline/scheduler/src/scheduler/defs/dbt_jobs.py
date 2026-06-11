from __future__ import annotations

from datetime import UTC, datetime

import dagster as dg

DBT_STAGING_SELECTION = dg.AssetSelection.groups("dbt_staging")
DBT_MODEL_SELECTION = (
    dg.AssetSelection.groups("dbt_staging")
    | dg.AssetSelection.groups("dbt_intermediate")
    | dg.AssetSelection.groups("dbt_marts")
)
STOCK_DAILY_SELECTION = DBT_MODEL_SELECTION | dg.AssetSelection.groups("calculation")

DBT_JOBS: tuple[dg.UnresolvedAssetJobDefinition, ...] = (
    dg.define_asset_job(
        name="dbt__staging_build_job",
        selection=DBT_STAGING_SELECTION,
    ),
    dg.define_asset_job(
        name="dbt__marts_build_job",
        selection=DBT_MODEL_SELECTION,
    ),
)

STOCK_JOBS: tuple[dg.UnresolvedAssetJobDefinition, ...] = (
    dg.define_asset_job(
        name="stock__daily_build_job",
        selection=STOCK_DAILY_SELECTION,
    ),
)

TRANSFORMATION_JOBS: tuple[dg.UnresolvedAssetJobDefinition, ...] = (*DBT_JOBS, *STOCK_JOBS)


def stock_daily_run_config(context: dg.ScheduleEvaluationContext) -> dict[str, object]:
    try:
        scheduled_time = context.scheduled_execution_time
    except Exception:
        scheduled_time = datetime.now(tz=UTC)
    if scheduled_time is None:
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
            },
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
            },
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
            },
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
            },
            "fleur_calculation__calc_stock_macd_daily": {
                "config": {
                    "request_from": trade_date,
                    "request_to": trade_date,
                    "mode": "append-latest",
                    "symbols": [],
                    "input_table": "fleur_intermediate.int_stock_quotes_daily_adj",
                    "output_table": "fleur_calculation.calc_stock_macd_daily",
                    "price_column": "close_price_forward_adj",
                    "insert_batch_size": 10_000,
                }
            },
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
            },
        }
    }


TRANSFORMATION_SCHEDULES: tuple[dg.ScheduleDefinition, ...] = (
    dg.ScheduleDefinition(
        name="stock__daily_build_schedule",
        job=STOCK_JOBS[0],
        cron_schedule="30 18 * * *",
        run_config_fn=stock_daily_run_config,
    ),
)
