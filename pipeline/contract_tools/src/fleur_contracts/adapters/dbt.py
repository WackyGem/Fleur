from __future__ import annotations

from typing import Any

from fleur_contracts.schema import ContractRegistry, DatasetContract


def render_sources_yaml(registry: ContractRegistry) -> str:
    tables = [
        {
            "name": dataset.dataset,
            "description": _table_description(registry, dataset),
            "meta": {
                "contract_dataset": dataset.dataset,
                "contract_version": dataset.version,
                "upstream_raw_asset": "/".join(dataset.raw_asset_key),
            },
        }
        for dataset in registry.datasets
    ]
    payload: dict[str, Any] = {
        "version": 2,
        "sources": [
            {
                "name": "raw",
                "schema": "raw",
                "description": (
                    "ClickHouse raw tables synchronized from Dagster-published S3 Parquet assets."
                ),
                "tables": tables,
            }
        ],
    }
    return _dump_yaml(payload)


def render_staging_yaml(registry: ContractRegistry) -> str:
    models: list[dict[str, Any]] = []
    for dataset in registry.datasets:
        staging = dataset.dbt_staging
        if staging is None or staging.status != "active":
            continue
        columns = []
        for field in staging.fields:
            glossary = None
            if field.glossary_key is not None:
                glossary = registry.glossary_fields[field.glossary_key]
            column: dict[str, Any] = {
                "name": field.name,
                "description": glossary.description
                if glossary is not None
                else field.exempt_reason,
            }
            if field.tests:
                column["tests"] = field.tests
            columns.append(column)
        models.append(
            {
                "name": staging.model,
                "description": _table_description(registry, dataset),
                "config": {
                    "materialized": staging.materialized,
                },
                "meta": {
                    "contract_dataset": dataset.dataset,
                    "contract_version": dataset.version,
                    "upstream_raw_asset": "/".join(dataset.raw_asset_key),
                },
                "columns": columns,
            }
        )
    return _dump_yaml({"version": 2, "models": models})


def _table_description(registry: ContractRegistry, dataset: DatasetContract) -> str:
    table = registry.glossary_tables.get(dataset.dataset)
    if table is not None:
        return table.description
    return dataset.external.source_table_name


def _dump_yaml(payload: dict[str, Any]) -> str:
    import yaml

    return yaml.safe_dump(
        payload,
        allow_unicode=True,
        default_flow_style=False,
        sort_keys=False,
    )
