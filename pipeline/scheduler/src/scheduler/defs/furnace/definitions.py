from __future__ import annotations

from datetime import UTC, datetime

import dagster as dg

from scheduler.defs.furnace.assets import FURNACE_KDJ_ASSET_KEY, FURNACE_KDJ_ASSETS
from scheduler.defs.resources.furnace import FurnaceCliResource


def build_furnace_defs(
    *,
    binary_path: str = "engines/target/debug/furnace",
    working_dir: str = ".",
    daily_cron_schedule: str = "45 18 * * *",
) -> dg.Definitions:
    jobs = build_furnace_jobs()
    return dg.Definitions(
        assets=list(FURNACE_KDJ_ASSETS),
        jobs=list(jobs),
        schedules=[
            dg.ScheduleDefinition(
                name="furnace__kdj_daily_schedule",
                job=jobs[0],
                cron_schedule=daily_cron_schedule,
                run_config_fn=_daily_run_config,
            )
        ],
        resources={
            "furnace_cli": FurnaceCliResource(
                binary_path=binary_path,
                working_dir=working_dir,
            )
        },
    )


def build_furnace_jobs() -> tuple[dg.UnresolvedAssetJobDefinition, ...]:
    selection = dg.AssetSelection.assets(FURNACE_KDJ_ASSET_KEY)
    return (
        dg.define_asset_job(name="furnace__kdj_daily_job", selection=selection),
        dg.define_asset_job(name="furnace__kdj_backfill_job", selection=selection),
        dg.define_asset_job(name="furnace__kdj_dry_run_job", selection=selection),
    )


def _daily_run_config(context: dg.ScheduleEvaluationContext) -> dict[str, object]:
    try:
        scheduled_time = context.scheduled_execution_time
    except Exception:
        scheduled_time = datetime.now(tz=UTC)
    trade_date = scheduled_time.date().isoformat()
    return {
        "ops": {
            "furnace__calc_stock_kdj_daily": {
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


FURNACE_DEFS = build_furnace_defs()
