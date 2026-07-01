from __future__ import annotations

import dagster as dg

# Legacy transformation jobs were removed from the production Dagster surface by
# Plan 0067. Keep these empty tuples as the migration boundary for old imports.
DBT_JOBS: tuple[dg.UnresolvedAssetJobDefinition, ...] = ()
STOCK_JOBS: tuple[dg.UnresolvedAssetJobDefinition, ...] = ()
TRANSFORMATION_JOBS: tuple[dg.UnresolvedAssetJobDefinition, ...] = ()
TRANSFORMATION_SCHEDULES: tuple[dg.ScheduleDefinition, ...] = ()
TRANSFORMATION_SENSORS: tuple[dg.RunStatusSensorDefinition, ...] = ()
