from __future__ import annotations

from collections.abc import Mapping
from typing import Any

import dagster as dg
from dagster_dbt import DbtProject, DbtProjectComponent

DBT_MODEL_RESOURCE_TYPES = {"model"}


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


def _uses_flat_model_asset_key(dbt_resource_props: Mapping[str, Any]) -> bool:
    return dbt_resource_props.get("resource_type") in DBT_MODEL_RESOURCE_TYPES


def _flat_model_asset_key(dbt_resource_props: Mapping[str, Any]) -> dg.AssetKey:
    return dg.AssetKey([str(dbt_resource_props["name"])])


def _rewrite_model_deps(
    base_spec: dg.AssetSpec,
    manifest: Mapping[str, Any],
) -> list[dg.AssetDep]:
    resources_by_default_key = {
        dg.AssetKey([str(resource["config"]["schema"]), str(resource["name"])]): resource
        for resource in manifest.get("nodes", {}).values()
        if _uses_flat_model_asset_key(resource) and resource.get("config", {}).get("schema")
    }

    deps: list[dg.AssetDep] = []
    for dep in base_spec.deps:
        resource = resources_by_default_key.get(dep.asset_key)
        dep_key = _flat_model_asset_key(resource) if resource is not None else dep.asset_key
        deps.append(
            dg.AssetDep(
                dep_key,
                partition_mapping=dep.partition_mapping,
                metadata=dep.metadata,
            )
        )

    return deps


class FleurDbtProjectComponent(DbtProjectComponent):
    def get_asset_spec(
        self,
        manifest: Mapping[str, Any],
        unique_id: str,
        project: DbtProject | None,
    ) -> dg.AssetSpec:
        base_spec = super().get_asset_spec(manifest, unique_id, project)
        dbt_resource_props = self.get_resource_props(manifest, unique_id)

        if not _uses_flat_model_asset_key(dbt_resource_props):
            return base_spec

        layer = _dbt_model_layer(dbt_resource_props)
        return base_spec.replace_attributes(
            key=_flat_model_asset_key(dbt_resource_props),
            deps=_rewrite_model_deps(base_spec, manifest),
            group_name=f"dbt_{layer}",
            tags={
                **base_spec.tags,
                "layer": layer,
                "owner": "dbt",
                "storage": "clickhouse",
            },
        )
