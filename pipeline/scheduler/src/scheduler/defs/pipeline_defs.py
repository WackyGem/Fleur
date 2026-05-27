from __future__ import annotations

import dagster as dg

from scheduler.defs.baostock.assets import (
    baostock__query_history_k_data_plus_daily,
    baostock__query_stock_basic,
)
from scheduler.defs.baostock.schedules import build_trade_day_schedule
from scheduler.defs.eastmoney.assets import EASTMONEY_ASSETS
from scheduler.defs.eastmoney.schedules import (
    eastmoney__daily_job,
    eastmoney__daily_schedule,
)
from scheduler.defs.http_resources.schedules import (
    sina__trade_calendar_job,
    sina__trade_calendar_schedule,
)
from scheduler.defs.http_resources.sina__trade_calendar import sina__trade_calendar
from scheduler.defs.io_managers.s3_io_manager import S3IOManager

baostock__daily_job = dg.define_asset_job(
    name="baostock__daily_job",
    selection=[
        baostock__query_stock_basic,
        baostock__query_history_k_data_plus_daily,
    ],
)

baostock__daily_schedule = build_trade_day_schedule(
    name="baostock__daily_schedule",
    job=baostock__daily_job,
    cron_schedule="35 17 * * *",
    partition_key_fn=lambda trade_date: str(trade_date.year),
    run_config_fn=lambda trade_date: {
        "ops": {
            "baostock__query_history_k_data_plus_daily": {
                "config": {
                    "refresh_until_trade_date": trade_date.isoformat(),
                }
            }
        }
    },
    tags_fn=lambda trade_date: {
        "market.trade_date": trade_date.isoformat(),
        "market.year": str(trade_date.year),
    },
)


@dg.definitions
def defs() -> dg.Definitions:
    return dg.Definitions(
        assets=[
            sina__trade_calendar,
            baostock__query_stock_basic,
            baostock__query_history_k_data_plus_daily,
            *EASTMONEY_ASSETS,
        ],
        jobs=[
            sina__trade_calendar_job,
            baostock__daily_job,
            eastmoney__daily_job,
        ],
        schedules=[
            sina__trade_calendar_schedule,
            baostock__daily_schedule,
            eastmoney__daily_schedule,
        ],
        resources={"s3_io_manager": S3IOManager()},
    )
