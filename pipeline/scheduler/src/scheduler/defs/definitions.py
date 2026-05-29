from __future__ import annotations

import dagster as dg

from scheduler.defs.automation import schedules as automation_schedules
from scheduler.defs.baostock.assets import (
    baostock__query_history_k_data_plus_daily,
    baostock__query_stock_basic,
)
from scheduler.defs.baostock.schedules import (
    baostock__daily_job,
    baostock__daily_schedule,
)
from scheduler.defs.http.schedules import (
    eastmoney__daily_job,
    eastmoney__daily_schedule,
    jiuyan__action_field_daily_job,
    jiuyan__action_field_daily_schedule,
    jiuyan__industry_list_snapshot_job,
    jiuyan__industry_list_snapshot_schedule,
    jiuyan__industry_ocr_pipeline_job,
    jiuyan__industry_ocr_pipeline_schedule,
    sina__trade_calendar_job,
    sina__trade_calendar_schedule,
    ths__limit_up_pool_daily_job,
    ths__limit_up_pool_daily_schedule,
)
from scheduler.defs.io_managers.s3_io_manager import S3IOManager
from scheduler.defs.sources.eastmoney.assets import EASTMONEY_ASSETS
from scheduler.defs.sources.jiuyan.action_field import jiuyan__action_field
from scheduler.defs.sources.jiuyan.action_field_compact import jiuyan__action_field_compacted
from scheduler.defs.sources.jiuyan.industry_list import jiuyan__industry_list
from scheduler.defs.sources.jiuyan.industry_ocr import (
    jiuyan__industry_images,
    jiuyan__industry_ocr,
)
from scheduler.defs.sources.sina.trade_calendar import sina__trade_calendar
from scheduler.defs.sources.ths.limit_up_pool import ths__limit_up_pool
from scheduler.defs.sources.ths.limit_up_pool_compact import ths__limit_up_pool_compacted

jiuyan__action_field_compacted_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="jiuyan__action_field_compacted_job",
        selection=[jiuyan__action_field_compacted],
    )
)

ths__limit_up_pool_compacted_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="ths__limit_up_pool_compacted_job",
        selection=[ths__limit_up_pool_compacted],
    )
)


@dg.definitions
def defs() -> dg.Definitions:
    return dg.Definitions(
        assets=[
            sina__trade_calendar,
            jiuyan__action_field,
            jiuyan__action_field_compacted,
            ths__limit_up_pool,
            ths__limit_up_pool_compacted,
            jiuyan__industry_list,
            jiuyan__industry_images,
            jiuyan__industry_ocr,
            baostock__query_stock_basic,
            baostock__query_history_k_data_plus_daily,
            *EASTMONEY_ASSETS,
        ],
        jobs=[
            sina__trade_calendar_job,
            jiuyan__action_field_daily_job,
            jiuyan__action_field_compacted_job,
            ths__limit_up_pool_daily_job,
            ths__limit_up_pool_compacted_job,
            jiuyan__industry_list_snapshot_job,
            jiuyan__industry_ocr_pipeline_job,
            baostock__daily_job,
            eastmoney__daily_job,
        ],
        schedules=[
            sina__trade_calendar_schedule,
            jiuyan__action_field_daily_schedule,
            ths__limit_up_pool_daily_schedule,
            jiuyan__industry_list_snapshot_schedule,
            jiuyan__industry_ocr_pipeline_schedule,
            baostock__daily_schedule,
            eastmoney__daily_schedule,
        ],
        resources={"s3_io_manager": S3IOManager()},
    )
