from __future__ import annotations

from collections.abc import Callable, Sequence
from dataclasses import dataclass
from zoneinfo import ZoneInfo

import dagster as dg


@dataclass(frozen=True)
class AssetJobSpec:
    name: str
    selection: Sequence[dg.AssetsDefinition]


@dataclass(frozen=True)
class ScheduleSpec:
    name: str
    job: dg.UnresolvedAssetJobDefinition
    cron_schedule: str
    execution_timezone: str = "Asia/Shanghai"
    description: str | None = None
    execution_fn: Callable[[dg.ScheduleEvaluationContext], dg.RunRequest | dg.SkipReason] | None = (
        None
    )


def build_asset_job(spec: AssetJobSpec) -> dg.UnresolvedAssetJobDefinition:
    return dg.define_asset_job(name=spec.name, selection=list(spec.selection))


def build_schedule(spec: ScheduleSpec) -> dg.ScheduleDefinition:
    return dg.ScheduleDefinition(
        name=spec.name,
        job=spec.job,
        cron_schedule=spec.cron_schedule,
        execution_timezone=spec.execution_timezone,
        description=spec.description,
        execution_fn=spec.execution_fn,
    )


def build_year_refresh_schedule(
    *,
    name: str,
    job: dg.UnresolvedAssetJobDefinition,
    cron_schedule: str,
    asset_names: Sequence[str],
    source: str,
    execution_timezone: str = "Asia/Shanghai",
) -> dg.ScheduleDefinition:
    timezone = ZoneInfo(execution_timezone)

    def evaluate_year_refresh_schedule(
        context: dg.ScheduleEvaluationContext,
    ) -> dg.RunRequest | dg.SkipReason:
        scheduled_time = context.scheduled_execution_time
        if scheduled_time is None:
            return dg.SkipReason("Schedule evaluation did not include scheduled_execution_time")

        natural_date = scheduled_time.astimezone(timezone).date()
        return dg.RunRequest(
            partition_key=str(natural_date.year),
            run_config={
                "ops": {
                    asset_name: {"config": {"refresh_until_date": natural_date.isoformat()}}
                    for asset_name in asset_names
                }
            },
            tags={
                "market.natural_date": natural_date.isoformat(),
                "market.year": str(natural_date.year),
                "source": source,
            },
        )

    return build_schedule(
        ScheduleSpec(
            name=name,
            job=job,
            cron_schedule=cron_schedule,
            execution_timezone=execution_timezone,
            execution_fn=evaluate_year_refresh_schedule,
        )
    )
