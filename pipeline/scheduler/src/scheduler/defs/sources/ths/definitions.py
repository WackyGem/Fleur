from __future__ import annotations

from datetime import date

from scheduler.defs.automation import schedules as automation_schedules
from scheduler.defs.market import schedules as market_schedules
from scheduler.defs.source_bundle import SourceBundle
from scheduler.defs.sources.ths.limit_up_pool import ths__limit_up_pool
from scheduler.defs.sources.ths.limit_up_pool_compact import ths__limit_up_pool_compacted


def _trade_date_tags(trade_date: date) -> dict[str, str]:
    return {"market.trade_date": trade_date.isoformat()}


ths__limit_up_pool_daily_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="ths__limit_up_pool_daily_job",
        selection=[ths__limit_up_pool],
    )
)

ths__limit_up_pool_compacted_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="ths__limit_up_pool_compacted_job",
        selection=[ths__limit_up_pool_compacted],
    )
)

ths__limit_up_pool_daily_schedule = market_schedules.build_trade_date_schedule(
    name="ths__limit_up_pool_daily_schedule",
    job=ths__limit_up_pool_daily_job,
    cron_schedule="45 16 * * *",
    source="ths",
    tags_fn=_trade_date_tags,
)

ths_bundle = SourceBundle(
    name="ths",
    assets=(ths__limit_up_pool, ths__limit_up_pool_compacted),
    jobs=(ths__limit_up_pool_daily_job, ths__limit_up_pool_compacted_job),
    schedules=(ths__limit_up_pool_daily_schedule,),
)
