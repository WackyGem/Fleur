from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from datetime import date
from typing import Protocol
from zoneinfo import ZoneInfo

import dagster as dg

from scheduler.defs.automation.schedules import ScheduleSpec, build_schedule
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.trade_calendar import is_market_trade_date, read_trade_dates_from_s3


class TradeCalendarReader(Protocol):
    def read_trade_dates(self) -> set[date]: ...


@dataclass(frozen=True)
class S3TradeCalendarReader:
    s3_config: S3Config

    def read_trade_dates(self) -> set[date]:
        return read_trade_dates_from_s3(self.s3_config)


def build_trade_date_schedule(
    *,
    name: str,
    job: dg.UnresolvedAssetJobDefinition,
    cron_schedule: str,
    source: str | None = None,
    partition_key_fn: Callable[[date], str | None] | None = None,
    run_config_fn: Callable[[date], dict[str, object]] | None = None,
    tags_fn: Callable[[date], dict[str, str]] | None = None,
    execution_timezone: str = "Asia/Shanghai",
    trade_calendar_reader_factory: Callable[[], TradeCalendarReader] | None = None,
) -> dg.ScheduleDefinition:
    timezone = ZoneInfo(execution_timezone)
    reader_factory = trade_calendar_reader_factory or (
        lambda: S3TradeCalendarReader(S3Config.from_env())
    )

    def evaluate_trade_date_schedule(
        context: dg.ScheduleEvaluationContext,
    ) -> dg.RunRequest | dg.SkipReason:
        scheduled_time = context.scheduled_execution_time
        if scheduled_time is None:
            return dg.SkipReason("Schedule evaluation did not include scheduled_execution_time")

        trade_date = scheduled_time.astimezone(timezone).date()
        try:
            trade_dates = reader_factory().read_trade_dates()
        except Exception as error:
            return dg.SkipReason(
                "Sina trade calendar parquet is unavailable; "
                f"materialize sina__trade_calendar first: {error}"
            )

        if not is_market_trade_date(trade_date, trade_dates):
            return dg.SkipReason(f"{trade_date.isoformat()} is not an A-share trade date")

        partition_key = (
            partition_key_fn(trade_date) if partition_key_fn is not None else trade_date.isoformat()
        )
        tags = tags_fn(trade_date) if tags_fn is not None else {}
        if source is not None:
            tags = {"source": source, **tags}
        return dg.RunRequest(
            partition_key=partition_key,
            run_config=run_config_fn(trade_date) if run_config_fn is not None else {},
            tags=tags,
        )

    return build_schedule(
        ScheduleSpec(
            name=name,
            job=job,
            cron_schedule=cron_schedule,
            execution_timezone=execution_timezone,
            execution_fn=evaluate_trade_date_schedule,
        )
    )
