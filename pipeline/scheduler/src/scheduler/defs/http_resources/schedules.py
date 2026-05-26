from __future__ import annotations

import dagster as dg

from scheduler.defs.http_resources.sina__trade_calendar import sina__trade_calendar

sina__trade_calendar_job = dg.define_asset_job(
    name="sina__trade_calendar_job",
    selection=[sina__trade_calendar],
)

sina__trade_calendar_schedule = dg.ScheduleDefinition(
    name="sina__trade_calendar_last_week_of_year",
    job=sina__trade_calendar_job,
    cron_schedule="0 9 25-31 12 *",
    execution_timezone="Asia/Shanghai",
    description="Refresh Sina A-share trade calendar during the final week of each year.",
)
