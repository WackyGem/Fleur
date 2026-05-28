from __future__ import annotations

from collections.abc import Callable
from datetime import date
from zoneinfo import ZoneInfo
import dagster as dg
from scheduler.defs.baostock.assets import (
    baostock__query_history_k_data_plus_daily,
    baostock__query_stock_basic,
)
from scheduler.defs.config import S3Config
from scheduler.defs.util import is_trade_date, read_sina_trade_calendar_dates_from_s3


def build_trade_date_schedule(
    name: str,
    job: dg.UnresolvedAssetJobDefinition,
    cron_schedule: str,
    partition_key_fn: Callable[[date], str | None],
    run_config_fn: Callable[[date], dict[str, object]] | None = None,
    tags_fn: Callable[[date], dict[str, str]] | None = None,
    dynamic_partitions: list[tuple[dg.DynamicPartitionsDefinition, Callable[[date], str]]]
    | None = None,
    execution_timezone: str = "Asia/Shanghai",
) -> dg.ScheduleDefinition:
    timezone = ZoneInfo(execution_timezone)

    def evaluate_trade_date_schedule(
        context: dg.ScheduleEvaluationContext,
    ) -> dg.RunRequest | dg.SkipReason:
        scheduled_time = context.scheduled_execution_time
        if scheduled_time is None:
            msg = "Schedule evaluation did not include scheduled_execution_time"
            return dg.SkipReason(msg)

        trade_date = scheduled_time.astimezone(timezone).date()
        try:
            trade_dates = read_sina_trade_calendar_dates_from_s3(S3Config.from_env())
        except Exception as error:
            return dg.SkipReason(
                "Sina trade calendar parquet is unavailable; "
                f"materialize sina__trade_calendar first: {error}"
            )

        if not is_trade_date(trade_date, trade_dates):
            return dg.SkipReason(f"{trade_date.isoformat()} is not an A-share trade date")

        for partitions_def, key_fn in dynamic_partitions or []:
            partition_key = key_fn(trade_date)
            context.instance.add_dynamic_partitions(
                partitions_def.name,
                [partition_key],
            )

        partition_key = partition_key_fn(trade_date)
        return dg.RunRequest(
            partition_key=partition_key,
            run_config=run_config_fn(trade_date) if run_config_fn else {},
            tags=tags_fn(trade_date) if tags_fn else {},
        )

    return dg.ScheduleDefinition(
        name=name,
        job=job,
        cron_schedule=cron_schedule,
        execution_timezone=execution_timezone,
        execution_fn=evaluate_trade_date_schedule,
    )


baostock__daily_job = dg.define_asset_job(
    name="baostock__daily_job",
    selection=[
        baostock__query_stock_basic,
        baostock__query_history_k_data_plus_daily,
    ],
)

baostock__daily_schedule = build_trade_date_schedule(
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
