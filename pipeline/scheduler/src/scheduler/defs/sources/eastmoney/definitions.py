from __future__ import annotations

from scheduler.defs.automation import schedules as automation_schedules
from scheduler.defs.source_bundle import SourceBundle
from scheduler.defs.sources.eastmoney.assets import EASTMONEY_ASSETS

EASTMONEY_DAILY_OP_NAMES = [asset.node_def.name for asset in EASTMONEY_ASSETS]

eastmoney__daily_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(name="eastmoney__daily_job", selection=EASTMONEY_ASSETS)
)

eastmoney__daily_schedule = automation_schedules.build_year_refresh_schedule(
    name="eastmoney__daily_schedule",
    job=eastmoney__daily_job,
    cron_schedule="0 16 * * *",
    asset_names=EASTMONEY_DAILY_OP_NAMES,
    source="eastmoney",
)

eastmoney_bundle = SourceBundle(
    name="eastmoney",
    assets=tuple(EASTMONEY_ASSETS),
    jobs=(eastmoney__daily_job,),
    schedules=(eastmoney__daily_schedule,),
)
