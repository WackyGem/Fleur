from __future__ import annotations

from collections.abc import Mapping
from typing import Any

import dagster as dg
from dagster_dbt import DbtProject, DbtProjectComponent


def _dbt_model_layer(dbt_resource_props: Mapping[str, Any]) -> str:
    fqn = dbt_resource_props.get("fqn", [])
    if isinstance(fqn, list) and len(fqn) >= 2:
        layer = fqn[1]
        if layer in {"staging", "intermediate", "marts"}:
            return str(layer)

    original_file_path = str(dbt_resource_props.get("original_file_path", ""))
    if original_file_path.startswith("models/staging/"):
        return "staging"
    if original_file_path.startswith("models/intermediate/"):
        return "intermediate"
    if original_file_path.startswith("models/marts/"):
        return "marts"

    return "dbt"


class FleurDbtProjectComponent(DbtProjectComponent):
    def get_asset_spec(
        self,
        manifest: Mapping[str, Any],
        unique_id: str,
        project: DbtProject | None,
    ) -> dg.AssetSpec:
        base_spec = super().get_asset_spec(manifest, unique_id, project)
        dbt_resource_props = self.get_resource_props(manifest, unique_id)

        if dbt_resource_props.get("resource_type") != "model":
            return base_spec

        layer = _dbt_model_layer(dbt_resource_props)
        return base_spec.replace_attributes(
            key=dg.AssetKey([str(dbt_resource_props["name"])]),
            group_name=f"dbt_{layer}",
            tags={
                **base_spec.tags,
                "layer": layer,
                "owner": "dbt",
                "storage": "clickhouse",
            },
        )
