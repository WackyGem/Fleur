from __future__ import annotations

from datetime import date

from scheduler.defs.automation import schedules as automation_schedules
from scheduler.defs.market import schedules as market_schedules
from scheduler.defs.source_bundle import SourceBundle
from scheduler.defs.sources.jiuyan.action_field import jiuyan__action_field
from scheduler.defs.sources.jiuyan.action_field_compact import jiuyan__action_field_compacted
from scheduler.defs.sources.jiuyan.industry_list import jiuyan__industry_list
from scheduler.defs.sources.jiuyan.industry_ocr import (
    jiuyan__industry_images,
    jiuyan__industry_ocr,
)


def _trade_date_tags(trade_date: date) -> dict[str, str]:
    return {"market.trade_date": trade_date.isoformat()}


jiuyan__action_field_daily_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="jiuyan__action_field_daily_job",
        selection=[jiuyan__action_field],
    )
)

jiuyan__action_field_compacted_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="jiuyan__action_field_compacted_job",
        selection=[jiuyan__action_field_compacted],
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

jiuyan__action_field_daily_schedule = market_schedules.build_trade_date_schedule(
    name="jiuyan__action_field_daily_schedule",
    job=jiuyan__action_field_daily_job,
    cron_schedule="45 16 * * *",
    source="jiuyan",
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

jiuyan_bundle = SourceBundle(
    name="jiuyan",
    assets=(
        jiuyan__action_field,
        jiuyan__action_field_compacted,
        jiuyan__industry_list,
        jiuyan__industry_images,
        jiuyan__industry_ocr,
    ),
    jobs=(
        jiuyan__action_field_daily_job,
        jiuyan__action_field_compacted_job,
        jiuyan__industry_list_snapshot_job,
        jiuyan__industry_ocr_pipeline_job,
    ),
    schedules=(
        jiuyan__action_field_daily_schedule,
        jiuyan__industry_list_snapshot_schedule,
        jiuyan__industry_ocr_pipeline_schedule,
    ),
)
