from __future__ import annotations

from datetime import date

from scheduler.defs.automation import schedules as automation_schedules
from scheduler.defs.market import schedules as market_schedules
from scheduler.defs.sources.eastmoney.assets import EASTMONEY_ASSETS
from scheduler.defs.sources.jiuyan.action_field import jiuyan__action_field
from scheduler.defs.sources.jiuyan.industry_list import jiuyan__industry_list
from scheduler.defs.sources.jiuyan.industry_ocr import (
    jiuyan__industry_images,
    jiuyan__industry_ocr,
)
from scheduler.defs.sources.sina.trade_calendar import sina__trade_calendar
from scheduler.defs.sources.ths.limit_up_pool import ths__limit_up_pool

EASTMONEY_DAILY_OP_NAMES = [asset.node_def.name for asset in EASTMONEY_ASSETS]


sina__trade_calendar_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="sina__trade_calendar_job",
        selection=[sina__trade_calendar],
    )
)

sina__trade_calendar_schedule = automation_schedules.build_schedule(
    automation_schedules.ScheduleSpec(
        name="sina__trade_calendar_schedule",
        job=sina__trade_calendar_job,
        cron_schedule="0 9 25-31 12 *",
        description="Refresh Sina A-share trade calendar during the final week of each year.",
    )
)

jiuyan__action_field_daily_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="jiuyan__action_field_daily_job",
        selection=[jiuyan__action_field],
    )
)

ths__limit_up_pool_daily_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="ths__limit_up_pool_daily_job",
        selection=[ths__limit_up_pool],
    )
)

jiuyan__industry_list_snapshot_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="jiuyan__industry_list_snapshot_job",
        selection=[jiuyan__industry_list],
    )
)

jiuyan__industry_ocr_pipeline_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="jiuyan__industry_ocr_pipeline_job",
        selection=[jiuyan__industry_list, jiuyan__industry_images, jiuyan__industry_ocr],
    )
)

eastmoney__daily_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(name="eastmoney__daily_job", selection=EASTMONEY_ASSETS)
)


def _trade_date_tags(trade_date: date) -> dict[str, str]:
    return {"market.trade_date": trade_date.isoformat()}


jiuyan__action_field_daily_schedule = market_schedules.build_trade_date_schedule(
    name="jiuyan__action_field_daily_schedule",
    job=jiuyan__action_field_daily_job,
    cron_schedule="45 16 * * *",
    source="jiuyan",
    tags_fn=_trade_date_tags,
)

ths__limit_up_pool_daily_schedule = market_schedules.build_trade_date_schedule(
    name="ths__limit_up_pool_daily_schedule",
    job=ths__limit_up_pool_daily_job,
    cron_schedule="45 16 * * *",
    source="ths",
    tags_fn=_trade_date_tags,
)

jiuyan__industry_list_snapshot_schedule = automation_schedules.build_schedule(
    automation_schedules.ScheduleSpec(
        name="jiuyan__industry_list_snapshot_schedule",
        job=jiuyan__industry_list_snapshot_job,
        cron_schedule="30 17 * * *",
        description="Refresh the latest JiuYan industry-list snapshot.",
    )
)

jiuyan__industry_ocr_pipeline_schedule = automation_schedules.build_schedule(
    automation_schedules.ScheduleSpec(
        name="jiuyan__industry_ocr_pipeline_schedule",
        job=jiuyan__industry_ocr_pipeline_job,
        cron_schedule="35 17 * * *",
        description="Refresh JiuYan industry list, download images, and process OCR.",
    )
)

eastmoney__daily_schedule = automation_schedules.build_year_refresh_schedule(
    name="eastmoney__daily_schedule",
    job=eastmoney__daily_job,
    cron_schedule="0 16 * * *",
    asset_names=EASTMONEY_DAILY_OP_NAMES,
    source="eastmoney",
)
