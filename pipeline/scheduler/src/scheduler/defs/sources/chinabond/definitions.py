from __future__ import annotations

from scheduler.defs.automation import schedules as automation_schedules
from scheduler.defs.source_bundle import SourceBundle
from scheduler.defs.sources.chinabond.assets import chinabond__government_bond

CHINABOND_ASSETS = (chinabond__government_bond,)
CHINABOND_DAILY_OP_NAMES = [asset.node_def.name for asset in CHINABOND_ASSETS]

chinabond__government_bond_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="chinabond__government_bond_job",
        selection=CHINABOND_ASSETS,
    )
)

chinabond__government_bond_schedule = automation_schedules.build_year_refresh_schedule(
    name="chinabond__government_bond_schedule",
    job=chinabond__government_bond_job,
    cron_schedule="0 16 * * *",
    asset_names=CHINABOND_DAILY_OP_NAMES,
    source="chinabond",
)

chinabond_bundle = SourceBundle(
    name="chinabond",
    assets=CHINABOND_ASSETS,
    jobs=(chinabond__government_bond_job,),
    schedules=(chinabond__government_bond_schedule,),
)
