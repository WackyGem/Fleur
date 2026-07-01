from __future__ import annotations

import dagster as dg

from scheduler.defs.furnace.assets import FURNACE_ASSETS
from scheduler.defs.resources.furnace import DEFAULT_FURNACE_BINARY_PATH, FurnaceCliResource


def build_furnace_defs(
    *,
    binary_path: str = DEFAULT_FURNACE_BINARY_PATH,
    working_dir: str = ".",
    rayon_num_threads: int | None = 8,
) -> dg.Definitions:
    return dg.Definitions(
        assets=list(FURNACE_ASSETS),
        resources={
            "furnace_cli": FurnaceCliResource(
                binary_path=binary_path,
                working_dir=working_dir,
                rayon_num_threads=rayon_num_threads,
            )
        },
    )


FURNACE_DEFS = build_furnace_defs()
