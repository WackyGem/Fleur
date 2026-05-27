from __future__ import annotations

import dagster as dg

from scheduler.defs.baostock.assets import (
    baostock__query_history_k_data_plus_daily,
    baostock__query_stock_basic,
)
from scheduler.defs.baostock.schedules import (
    baostock__daily_job,
    baostock__daily_schedule,
)
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
