from __future__ import annotations

from zoneinfo import ZoneInfo

import dagster as dg

from scheduler.defs.http_resources.eastmoney.assets import EASTMONEY_ASSETS

EASTMONEY_DAILY_ASSET_NAMES = [asset.key.path[-1] for asset in EASTMONEY_ASSETS]

eastmoney__daily_job = dg.define_asset_job(
    name="eastmoney__daily_job",
    selection=EASTMONEY_ASSETS,
)


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


eastmoney__daily_schedule = dg.ScheduleDefinition(
    name="eastmoney__daily_schedule",
    job=eastmoney__daily_job,
    cron_schedule="0 16 * * *",
    execution_timezone="Asia/Shanghai",
    execution_fn=_evaluate_eastmoney_daily_schedule,
)
