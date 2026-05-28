from __future__ import annotations

from zoneinfo import ZoneInfo

import dagster as dg

from scheduler.defs.config import S3Config
from scheduler.defs.http_resources.jiuyan__action_field import jiuyan__action_field
from scheduler.defs.http_resources.jiuyan__industry_list import jiuyan__industry_list
from scheduler.defs.http_resources.partitioned import (
    sync_trade_date_dynamic_partitions,
    trade_date_dynamic_partitions,
)
from scheduler.defs.http_resources.sina__trade_calendar import sina__trade_calendar
from scheduler.defs.http_resources.ths__limit_up_pool import ths__limit_up_pool
from scheduler.defs.util import is_trade_date, read_sina_trade_calendar_dates_from_s3

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

http_resources__market_event_daily_job = dg.define_asset_job(
    name="http_resources__market_event_daily_job",
    selection=[jiuyan__action_field, ths__limit_up_pool],
    partitions_def=trade_date_dynamic_partitions,
)

jiuyan__industry_list_snapshot_job = dg.define_asset_job(
    name="jiuyan__industry_list_snapshot_job",
    selection=[jiuyan__industry_list],
)


def _evaluate_market_event_daily_schedule(
    context: dg.ScheduleEvaluationContext,
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

    sync_trade_date_dynamic_partitions(context.instance, trade_dates)
    partition_key = trade_date.isoformat()
    return dg.RunRequest(
        partition_key=partition_key,
        tags={
            "market.trade_date": partition_key,
            "source": "http_resources",
        },
    )


@dg.asset_sensor(
    asset_key=dg.AssetKey("sina__trade_calendar"),
    name="sina__trade_calendar_dynamic_partitions_sensor",
    minimum_interval_seconds=30,
    default_status=dg.DefaultSensorStatus.RUNNING,
    description=("Sync trade_date dynamic partitions after sina__trade_calendar materializes."),
)
def sina__trade_calendar_dynamic_partitions_sensor(
    context: dg.SensorEvaluationContext,
    _asset_event: dg.EventLogEntry,
) -> dg.SkipReason:
    try:
        trade_dates = read_sina_trade_calendar_dates_from_s3(S3Config.from_env())
    except Exception as error:
        return dg.SkipReason(
            f"Sina trade calendar parquet is unavailable after materialization: {error}"
        )

    new_partition_keys = sync_trade_date_dynamic_partitions(
        context.instance,
        trade_dates,
    )
    context.log.info(
        "Synced %s new trade_date dynamic partitions after sina__trade_calendar materialization",
        len(new_partition_keys),
    )
    return dg.SkipReason(
        "Synced "
        f"{len(new_partition_keys)} new trade_date dynamic partitions "
        "from sina__trade_calendar"
    )


http_resources__market_event_daily_schedule = dg.ScheduleDefinition(
    name="http_resources__market_event_daily_schedule",
    job=http_resources__market_event_daily_job,
    cron_schedule="45 16 * * *",
    execution_timezone="Asia/Shanghai",
    execution_fn=_evaluate_market_event_daily_schedule,
)

jiuyan__industry_list_snapshot_schedule = dg.ScheduleDefinition(
    name="jiuyan__industry_list_snapshot_schedule",
    job=jiuyan__industry_list_snapshot_job,
    cron_schedule="30 17 * * *",
    execution_timezone="Asia/Shanghai",
    description="Refresh the latest JiuYan industry-list snapshot.",
)
