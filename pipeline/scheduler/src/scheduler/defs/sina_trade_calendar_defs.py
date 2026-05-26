from __future__ import annotations

import dagster as dg

from scheduler.defs.http_resources.schedules import (
    sina__trade_calendar_job,
    sina__trade_calendar_schedule,
)
from scheduler.defs.http_resources.sina__trade_calendar import sina__trade_calendar
from scheduler.defs.io_managers.s3_io_manager import S3IOManager


@dg.definitions
def defs() -> dg.Definitions:
    return dg.Definitions(
        assets=[sina__trade_calendar],
        jobs=[sina__trade_calendar_job],
        schedules=[sina__trade_calendar_schedule],
        resources={"s3_io_manager": S3IOManager()},
    )
