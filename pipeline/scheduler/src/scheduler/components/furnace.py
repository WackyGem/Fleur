from __future__ import annotations

import dagster as dg

from scheduler.defs.furnace.definitions import build_furnace_defs


class FurnaceKdjComponent(dg.Component, dg.Resolvable, dg.Model):
    binary_path: str = "engines/target/debug/furnace"
    working_dir: str = "."
    daily_cron_schedule: str = "45 18 * * *"

    def build_defs(self, context: dg.ComponentLoadContext) -> dg.Definitions:
        return build_furnace_defs(
            binary_path=self.binary_path,
            working_dir=self.working_dir,
            daily_cron_schedule=self.daily_cron_schedule,
        )
