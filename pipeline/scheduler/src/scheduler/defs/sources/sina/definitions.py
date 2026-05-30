from __future__ import annotations

from scheduler.defs.automation import schedules as automation_schedules
from scheduler.defs.source_bundle import SourceBundle
from scheduler.defs.sources.sina.trade_calendar import sina__trade_calendar

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

sina_bundle = SourceBundle(
    name="sina",
    assets=(sina__trade_calendar,),
    jobs=(sina__trade_calendar_job,),
    schedules=(sina__trade_calendar_schedule,),
)
