from __future__ import annotations

from datetime import datetime
from zoneinfo import ZoneInfo

import dagster as dg

from scheduler.defs.automation.source_to_marts_backfill import ALL_SOURCE_TO_MARTS_SCOPE
from scheduler.defs.daily.source_to_marts import (
    DAILY_CONTROLLER_OP_NAME,
    DAILY_JOB_NAME,
    DAILY_KIND,
    daily__fetch_history_sources_to_marts_schedule_job,
)

DAILY_SCHEDULE_NAME = "daily__fetch_history_sources_to_marts_schedule"
DAILY_SCHEDULE_CRON = "30 18 * * *"
DAILY_SCHEDULE_TIMEZONE = "Asia/Shanghai"
DAILY_SCHEDULE_DRY_RUN = True
DAILY_SCHEDULE_EXECUTION_MODE = "full"
DAILY_SCHEDULE_TARGET_SCOPE = ALL_SOURCE_TO_MARTS_SCOPE


def daily_schedule_run_config_for_target_date(target_date: str) -> dict[str, object]:
    return {
        "ops": {
            DAILY_CONTROLLER_OP_NAME: {
                "config": {
                    "target_scope": DAILY_SCHEDULE_TARGET_SCOPE,
                    "target_date": target_date,
                    "execution_mode": DAILY_SCHEDULE_EXECUTION_MODE,
                    "dry_run": DAILY_SCHEDULE_DRY_RUN,
                    "refresh_prerequisite_snapshots": False,
                    "overwrite_source_partitions": False,
                }
            }
        }
    }


def daily_schedule_tags_for_target_date(target_date: str) -> dict[str, str]:
    return {
        "daily.kind": DAILY_KIND,
        "daily.target_scope": DAILY_SCHEDULE_TARGET_SCOPE,
        "daily.target_date": target_date,
        "daily.execution_mode": DAILY_SCHEDULE_EXECUTION_MODE,
        "daily.dry_run": str(DAILY_SCHEDULE_DRY_RUN).lower(),
    }


def daily_schedule_run_request(
    context: dg.ScheduleEvaluationContext,
) -> dg.RunRequest | dg.SkipReason:
    scheduled_time = context.scheduled_execution_time
    if scheduled_time is None:
        return dg.SkipReason("Schedule evaluation did not include scheduled_execution_time")

    target_date = _target_date_from_scheduled_time(scheduled_time)
    return dg.RunRequest(
        run_key=f"{DAILY_SCHEDULE_NAME}:{target_date}",
        run_config=daily_schedule_run_config_for_target_date(target_date),
        tags=daily_schedule_tags_for_target_date(target_date),
    )


daily__fetch_history_sources_to_marts_schedule = dg.ScheduleDefinition(
    name=DAILY_SCHEDULE_NAME,
    job=daily__fetch_history_sources_to_marts_schedule_job,
    cron_schedule=DAILY_SCHEDULE_CRON,
    execution_timezone=DAILY_SCHEDULE_TIMEZONE,
    default_status=dg.DefaultScheduleStatus.STOPPED,
    execution_fn=daily_schedule_run_request,
    description=(
        "Daily incremental source-to-marts controller schedule. Default stopped until "
        "the expanded dry-run plan is accepted for production."
    ),
    tags={
        "daily.kind": DAILY_KIND,
        "daily.job": DAILY_JOB_NAME,
        "daily.target_scope": DAILY_SCHEDULE_TARGET_SCOPE,
    },
)


DAILY_DEFS = dg.Definitions(
    jobs=[daily__fetch_history_sources_to_marts_schedule_job],
    schedules=[daily__fetch_history_sources_to_marts_schedule],
)


def _target_date_from_scheduled_time(scheduled_time: datetime) -> str:
    return scheduled_time.astimezone(ZoneInfo(DAILY_SCHEDULE_TIMEZONE)).date().isoformat()
