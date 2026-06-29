from __future__ import annotations

from datetime import UTC, datetime

import dagster as dg

from scheduler.defs.clickhouse.definitions import CLICKHOUSE_RAW_JOBS

DBT_STAGING_SELECTION = dg.AssetSelection.groups("dbt_staging")
DBT_MODEL_SELECTION = (
    dg.AssetSelection.groups("dbt_staging")
    | dg.AssetSelection.groups("dbt_intermediate")
    | dg.AssetSelection.groups("dbt_marts")
)
CLICKHOUSE_RAW_SYNC_BAOSTOCK_JOB = next(
    job for job in CLICKHOUSE_RAW_JOBS if job.name == "clickhouse__raw_sync_baostock_job"
)
STOCK_DAILY_DBT_QUOTE_REBUILD_SELECTION = dg.AssetSelection.assets(
    "int_stock_quotes_daily_unadj",
    "int_stock_adjustment_factor",
    "int_stock_quotes_daily_adj",
)
STOCK_DAILY_CALCULATION_SELECTION = dg.AssetSelection.groups("calculation")
STOCK_DAILY_MART_REBUILD_SELECTION = dg.AssetSelection.assets(
    "int_stock_kdj_daily",
    "mart_stock_quotes_daily",
)
STOCK_DAILY_SELECTION = (
    STOCK_DAILY_DBT_QUOTE_REBUILD_SELECTION
    | STOCK_DAILY_CALCULATION_SELECTION
    | STOCK_DAILY_MART_REBUILD_SELECTION
)

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


def stock_daily_run_config_for_trade_date(trade_date: str) -> dict[str, object]:
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


def stock_daily_run_config(context: dg.ScheduleEvaluationContext) -> dict[str, object]:
    try:
        scheduled_time = context.scheduled_execution_time
    except Exception:
        scheduled_time = datetime.now(tz=UTC)
    if scheduled_time is None:
        scheduled_time = datetime.now(tz=UTC)
    return stock_daily_run_config_for_trade_date(scheduled_time.date().isoformat())


TRANSFORMATION_SCHEDULES: tuple[dg.ScheduleDefinition, ...] = (
    dg.ScheduleDefinition(
        name="stock__daily_build_schedule",
        job=STOCK_JOBS[0],
        cron_schedule="30 18 * * *",
        run_config_fn=stock_daily_run_config,
    ),
)


@dg.run_status_sensor(
    run_status=dg.DagsterRunStatus.SUCCESS,
    name="baostock_raw_sync_success_triggers_stock_daily_build",
    monitored_jobs=[CLICKHOUSE_RAW_SYNC_BAOSTOCK_JOB],
    request_job=STOCK_JOBS[0],
    default_status=dg.DefaultSensorStatus.RUNNING,
    minimum_interval_seconds=30,
)
def baostock_raw_sync_success_triggers_stock_daily_build(
    context: dg.RunStatusSensorContext,
) -> dg.RunRequest:
    trade_date = datetime.now(tz=UTC).date().isoformat()
    raw_run_id = context.dagster_run.run_id
    raw_partition_key = context.partition_key or ""
    return dg.RunRequest(
        run_key=f"stock-daily-after-baostock-raw-sync:{raw_run_id}",
        run_config=stock_daily_run_config_for_trade_date(trade_date),
        tags={
            "trigger": "baostock_raw_sync_success",
            "upstream_job": context.dagster_run.job_name,
            "upstream_run_id": raw_run_id,
            "upstream_partition_key": raw_partition_key,
            "stock_daily_trade_date": trade_date,
        },
    )


TRANSFORMATION_SENSORS: tuple[dg.RunStatusSensorDefinition, ...] = (
    baostock_raw_sync_success_triggers_stock_daily_build,
)
