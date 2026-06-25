from __future__ import annotations

import dagster as dg

from scheduler.defs.rearview.assets import (
    REARVIEW_ASSETS,
    STRATEGY_PORTFOLIO_DAILY_ASSET_KEY,
)
from scheduler.defs.rearview.resources import RearviewApiResource

STRATEGY_PORTFOLIO_DAILY_RUN_JOB = dg.define_asset_job(
    name="strategy_portfolio__daily_run_job",
    selection=dg.AssetSelection.keys(STRATEGY_PORTFOLIO_DAILY_ASSET_KEY),
)

PORTFOLIO_DAILY_RUN_SCHEDULE = dg.build_schedule_from_partitioned_job(
    STRATEGY_PORTFOLIO_DAILY_RUN_JOB,
    name="portfolio__daily_run_schedule",
    minute_of_hour=0,
    hour_of_day=20,
)


def build_rearview_defs(*, base_url: str = "") -> dg.Definitions:
    return dg.Definitions(
        assets=list(REARVIEW_ASSETS),
        jobs=[STRATEGY_PORTFOLIO_DAILY_RUN_JOB],
        schedules=[PORTFOLIO_DAILY_RUN_SCHEDULE],
        resources={"rearview_api": RearviewApiResource(base_url=base_url)},
    )


REARVIEW_DEFS = build_rearview_defs()
