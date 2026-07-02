from __future__ import annotations

import dagster as dg

from scheduler.defs.rearview.assets import (
    EXAMPLE_0051_PORTFOLIO_LIVE_ASSET_KEY,
    REARVIEW_ASSETS,
)
from scheduler.defs.rearview.resources import RearviewApiResource

EXAMPLE_PORTFOLIO_LIVE_JOB = dg.define_asset_job(
    name="example__portfolio_live_job",
    selection=dg.AssetSelection.keys(EXAMPLE_0051_PORTFOLIO_LIVE_ASSET_KEY),
)


def build_rearview_defs(*, base_url: str = "") -> dg.Definitions:
    return dg.Definitions(
        assets=list(REARVIEW_ASSETS),
        jobs=[EXAMPLE_PORTFOLIO_LIVE_JOB],
        schedules=[],
        resources={"rearview_api": RearviewApiResource(base_url=base_url)},
    )


REARVIEW_DEFS = build_rearview_defs()
