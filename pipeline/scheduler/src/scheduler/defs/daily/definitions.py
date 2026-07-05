from __future__ import annotations

from collections.abc import Callable
from datetime import date, datetime
from zoneinfo import ZoneInfo

import dagster as dg

from scheduler.defs.automation.source_to_marts_backfill import ALL_SOURCE_TO_MARTS_SCOPE
from scheduler.defs.config.models import S3Config
from scheduler.defs.daily.source_to_marts import (
    DAILY_CONTROLLER_OP_NAME,
    DAILY_JOB_NAME,
    DAILY_KIND,
    daily__fetch_history_sources_to_marts_schedule_job,
)
from scheduler.defs.market.readers import S3TradeCalendarReader, TradeCalendarReader
from scheduler.defs.market.trade_calendar import is_market_trade_date

DAILY_SCHEDULE_NAME = "daily__fetch_history_sources_to_marts_schedule"
DAILY_SCHEDULE_CRON = "45 17 * * *"
DAILY_SCHEDULE_TIMEZONE = "Asia/Shanghai"
DAILY_SCHEDULE_DRY_RUN = False
DAILY_SCHEDULE_EXECUTION_MODE = "full"
DAILY_SCHEDULE_TARGET_SCOPE = ALL_SOURCE_TO_MARTS_SCOPE
TradeCalendarReaderFactory = Callable[[], TradeCalendarReader]


def _default_trade_calendar_reader_factory() -> TradeCalendarReader:
    return S3TradeCalendarReader.from_s3_config(S3Config.from_env())


daily_trade_calendar_reader_factory: TradeCalendarReaderFactory = (
    _default_trade_calendar_reader_factory
)


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

    target_trade_date = _target_date_from_scheduled_time(scheduled_time)
    try:
        trade_dates = daily_trade_calendar_reader_factory().read_trade_dates()
    except Exception as error:
        return dg.SkipReason(
            "Sina trade calendar parquet is unavailable; "
            f"materialize sina__trade_calendar first: {error}"
        )

    if not is_market_trade_date(target_trade_date, trade_dates):
        return dg.SkipReason(f"{target_trade_date.isoformat()} is not an A-share trade date")

    target_date = target_trade_date.isoformat()
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
        "Daily incremental source-to-marts controller schedule. Default stopped; when "
        "enabled it submits real daily source-to-marts runs."
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


def _target_date_from_scheduled_time(scheduled_time: datetime) -> date:
    return scheduled_time.astimezone(ZoneInfo(DAILY_SCHEDULE_TIMEZONE)).date()
