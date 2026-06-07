from __future__ import annotations

import dagster as dg

DBT_JOBS: tuple[dg.UnresolvedAssetJobDefinition, ...] = (
    dg.define_asset_job(
        name="dbt__staging_build_job",
        selection=dg.AssetSelection.groups("dbt_staging"),
    ),
    dg.define_asset_job(
        name="dbt__marts_build_job",
        selection=(
            dg.AssetSelection.groups("dbt_staging")
            | dg.AssetSelection.groups("dbt_intermediate")
            | dg.AssetSelection.groups("dbt_marts")
        ),
    ),
    dg.define_asset_job(
        name="dbt__daily_build_job",
        selection=(
            dg.AssetSelection.groups("dbt_staging")
            | dg.AssetSelection.groups("dbt_intermediate")
            | dg.AssetSelection.groups("dbt_marts")
        ),
    ),
)

DBT_SCHEDULES: tuple[dg.ScheduleDefinition, ...] = (
    dg.ScheduleDefinition(
        name="dbt__daily_build_schedule",
        job=DBT_JOBS[-1],
        cron_schedule="30 18 * * *",
    ),
)
