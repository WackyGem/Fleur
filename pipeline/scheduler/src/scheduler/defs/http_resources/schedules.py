from __future__ import annotations

from zoneinfo import ZoneInfo

import dagster as dg

from scheduler.defs.config import S3Config
from scheduler.defs.http_resources.eastmoney import EASTMONEY_ASSETS
from scheduler.defs.http_resources.jiuyan__action_field import jiuyan__action_field
from scheduler.defs.http_resources.jiuyan__industry_list import jiuyan__industry_list
from scheduler.defs.http_resources.jiuyan__industry_ocr import (
    jiuyan__industry_images,
    jiuyan__industry_ocr,
)
from scheduler.defs.http_resources.sina__trade_calendar import sina__trade_calendar
from scheduler.defs.http_resources.ths__limit_up_pool import ths__limit_up_pool
from scheduler.defs.util import is_trade_date, read_sina_trade_calendar_dates_from_s3

EASTMONEY_DAILY_ASSET_NAMES = [asset.key.path[-1] for asset in EASTMONEY_ASSETS]

sina__trade_calendar_job = dg.define_asset_job(
    name="sina__trade_calendar_job",
    selection=[sina__trade_calendar],
)

sina__trade_calendar_schedule = dg.ScheduleDefinition(
    name="sina__trade_calendar_schedule",
    job=sina__trade_calendar_job,
    cron_schedule="0 9 25-31 12 *",
    execution_timezone="Asia/Shanghai",
    description="Refresh Sina A-share trade calendar during the final week of each year.",
)

jiuyan__action_field_daily_job = dg.define_asset_job(
    name="jiuyan__action_field_daily_job",
    selection=[jiuyan__action_field],
)

ths__limit_up_pool_daily_job = dg.define_asset_job(
    name="ths__limit_up_pool_daily_job",
    selection=[ths__limit_up_pool],
)

jiuyan__industry_list_snapshot_job = dg.define_asset_job(
    name="jiuyan__industry_list_snapshot_job",
    selection=[jiuyan__industry_list],
)

jiuyan__industry_ocr_pipeline_job = dg.define_asset_job(
    name="jiuyan__industry_ocr_pipeline_job",
    selection=[jiuyan__industry_list, jiuyan__industry_images, jiuyan__industry_ocr],
)

eastmoney__daily_job = dg.define_asset_job(
    name="eastmoney__daily_job",
    selection=EASTMONEY_ASSETS,
)


def _evaluate_trade_date_daily_schedule(
    context: dg.ScheduleEvaluationContext,
    *,
    source: str,
) -> dg.RunRequest | dg.SkipReason:
    scheduled_time = context.scheduled_execution_time
    if scheduled_time is None:
        return dg.SkipReason("Schedule evaluation did not include scheduled_execution_time")

    trade_date = scheduled_time.astimezone(ZoneInfo("Asia/Shanghai")).date()
    try:
        trade_dates = read_sina_trade_calendar_dates_from_s3(S3Config.from_env())
    except Exception as error:
        return dg.SkipReason(
            "Sina trade calendar parquet is unavailable; "
            f"materialize sina__trade_calendar first: {error}"
        )

    if not is_trade_date(trade_date, trade_dates):
        return dg.SkipReason(f"{trade_date.isoformat()} is not an A-share trade date")

    partition_key = trade_date.isoformat()
    return dg.RunRequest(
        partition_key=partition_key,
        tags={
            "market.trade_date": partition_key,
            "source": source,
        },
    )


def _evaluate_jiuyan_action_field_daily_schedule(
    context: dg.ScheduleEvaluationContext,
) -> dg.RunRequest | dg.SkipReason:
    return _evaluate_trade_date_daily_schedule(context, source="jiuyan")


def _evaluate_ths_limit_up_pool_daily_schedule(
    context: dg.ScheduleEvaluationContext,
) -> dg.RunRequest | dg.SkipReason:
    return _evaluate_trade_date_daily_schedule(context, source="ths")


def _evaluate_eastmoney_daily_schedule(
    context: dg.ScheduleEvaluationContext,
) -> dg.RunRequest | dg.SkipReason:
    scheduled_time = context.scheduled_execution_time
    if scheduled_time is None:
        return dg.SkipReason("Schedule evaluation did not include scheduled_execution_time")

    natural_date = scheduled_time.astimezone(ZoneInfo("Asia/Shanghai")).date()
    return dg.RunRequest(
        partition_key=str(natural_date.year),
        run_config={
            "ops": {
                asset_name: {"config": {"refresh_until_date": natural_date.isoformat()}}
                for asset_name in EASTMONEY_DAILY_ASSET_NAMES
            }
        },
        tags={
            "market.natural_date": natural_date.isoformat(),
            "market.year": str(natural_date.year),
            "source": "eastmoney",
        },
    )


jiuyan__action_field_daily_schedule = dg.ScheduleDefinition(
    name="jiuyan__action_field_daily_schedule",
    job=jiuyan__action_field_daily_job,
    cron_schedule="45 16 * * *",
    execution_timezone="Asia/Shanghai",
    execution_fn=_evaluate_jiuyan_action_field_daily_schedule,
)

ths__limit_up_pool_daily_schedule = dg.ScheduleDefinition(
    name="ths__limit_up_pool_daily_schedule",
    job=ths__limit_up_pool_daily_job,
    cron_schedule="45 16 * * *",
    execution_timezone="Asia/Shanghai",
    execution_fn=_evaluate_ths_limit_up_pool_daily_schedule,
)

jiuyan__industry_list_snapshot_schedule = dg.ScheduleDefinition(
    name="jiuyan__industry_list_snapshot_schedule",
    job=jiuyan__industry_list_snapshot_job,
    cron_schedule="30 17 * * *",
    execution_timezone="Asia/Shanghai",
    description="Refresh the latest JiuYan industry-list snapshot.",
)

jiuyan__industry_ocr_pipeline_schedule = dg.ScheduleDefinition(
    name="jiuyan__industry_ocr_pipeline_schedule",
    job=jiuyan__industry_ocr_pipeline_job,
    cron_schedule="35 17 * * *",
    execution_timezone="Asia/Shanghai",
    description="Refresh JiuYan industry list, download images, and process OCR.",
)

eastmoney__daily_schedule = dg.ScheduleDefinition(
    name="eastmoney__daily_schedule",
    job=eastmoney__daily_job,
    cron_schedule="0 16 * * *",
    execution_timezone="Asia/Shanghai",
    execution_fn=_evaluate_eastmoney_daily_schedule,
)
