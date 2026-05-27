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
from scheduler.defs.http_resources.eastmoney.assets import EASTMONEY_ASSETS
from scheduler.defs.http_resources.eastmoney.schedules import (
    eastmoney__daily_job,
    eastmoney__daily_schedule,
)
from scheduler.defs.http_resources.jiuyan__action_field import jiuyan__action_field
from scheduler.defs.http_resources.jiuyan__industry_list import jiuyan__industry_list
from scheduler.defs.http_resources.schedules import (
    http_resources__market_event_daily_job,
    http_resources__market_event_daily_schedule,
    jiuyan__industry_list_snapshot_job,
    jiuyan__industry_list_snapshot_schedule,
    sina__trade_calendar_dynamic_partitions_sensor,
    sina__trade_calendar_job,
    sina__trade_calendar_schedule,
)
from scheduler.defs.http_resources.sina__trade_calendar import sina__trade_calendar
from scheduler.defs.http_resources.ths__limit_up_pool import ths__limit_up_pool
from scheduler.defs.io_managers.s3_io_manager import S3IOManager


@dg.definitions
def defs() -> dg.Definitions:
    return dg.Definitions(
        assets=[
            sina__trade_calendar,
            jiuyan__action_field,
            ths__limit_up_pool,
            jiuyan__industry_list,
            baostock__query_stock_basic,
            baostock__query_history_k_data_plus_daily,
            *EASTMONEY_ASSETS,
        ],
        jobs=[
            sina__trade_calendar_job,
            http_resources__market_event_daily_job,
            jiuyan__industry_list_snapshot_job,
            baostock__daily_job,
            eastmoney__daily_job,
        ],
        schedules=[
            sina__trade_calendar_schedule,
            http_resources__market_event_daily_schedule,
            jiuyan__industry_list_snapshot_schedule,
            baostock__daily_schedule,
            eastmoney__daily_schedule,
        ],
        sensors=[
            sina__trade_calendar_dynamic_partitions_sensor,
        ],
        resources={"s3_io_manager": S3IOManager()},
    )
