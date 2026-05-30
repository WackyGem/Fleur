from __future__ import annotations

from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field

import dagster as dg


@dataclass(frozen=True)
class SourceBundle:
    name: str
    assets: Sequence[dg.AssetsDefinition] = field(default_factory=tuple)
    jobs: Sequence[dg.UnresolvedAssetJobDefinition | dg.JobDefinition] = field(
        default_factory=tuple
    )
    schedules: Sequence[dg.ScheduleDefinition] = field(default_factory=tuple)


def bundle_assets(bundles: Iterable[SourceBundle]) -> list[dg.AssetsDefinition]:
    return [asset for bundle in bundles for asset in bundle.assets]


def bundle_jobs(
    bundles: Iterable[SourceBundle],
) -> list[dg.UnresolvedAssetJobDefinition | dg.JobDefinition]:
    return [job for bundle in bundles for job in bundle.jobs]


def bundle_schedules(bundles: Iterable[SourceBundle]) -> list[dg.ScheduleDefinition]:
    return [schedule for bundle in bundles for schedule in bundle.schedules]
