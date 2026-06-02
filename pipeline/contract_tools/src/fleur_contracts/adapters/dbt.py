from __future__ import annotations

from typing import Any

from fleur_contracts.adapters.parquet import parquet_schema_hash
from fleur_contracts.loader import clickhouse_schema_hash, source_schema_hash
from fleur_contracts.schema import ContractRegistry, DatasetContract


def render_sources_yaml(registry: ContractRegistry) -> str:
    raw_datasets = _raw_datasets_for_sources(registry)
    raw_databases = {
        dataset.clickhouse_raw.database
        for dataset in raw_datasets
        if dataset.clickhouse_raw is not None
    }
    if len(raw_databases) != 1:
        msg = f"dbt raw source requires exactly one physical database, got {sorted(raw_databases)}"
        raise ValueError(msg)
    raw_database = next(iter(raw_databases))

    tables = []
    for dataset in raw_datasets:
        raw = dataset.clickhouse_raw
        raw_asset_key = dataset.raw_asset_key
        if raw is None or raw_asset_key is None:
            continue
        tables.append(
            {
                "name": dataset.dataset,
                "description": _table_description(registry, dataset),
                "config": {
                    "meta": {
                        "contract_dataset": dataset.dataset,
                        "contract_version": dataset.version,
                        "upstream_raw_asset": "/".join(raw_asset_key),
                        "clickhouse_raw_table": f"{raw.database}.{raw.table}",
                        "source_schema_hash": source_schema_hash(dataset),
                        "parquet_schema_hash": parquet_schema_hash(dataset),
                        "clickhouse_schema_hash": clickhouse_schema_hash(dataset),
                    },
                },
                "columns": _source_columns(dataset),
            }
        )
    payload: dict[str, Any] = {
        "version": 2,
        "sources": [
            {
                "name": "raw",
                "schema": raw_database,
                "description": (
                    "ClickHouse raw tables synchronized from Dagster-published S3 Parquet assets."
                ),
                "tables": tables,
            }
        ],
    }
    return _dump_yaml(payload)


def _raw_datasets_for_sources(registry: ContractRegistry) -> list[DatasetContract]:
    datasets: list[DatasetContract] = []
    for dataset in registry.datasets:
        if dataset.clickhouse_raw is None or dataset.raw_asset_key is None:
            continue
        datasets.append(dataset)
    return datasets


def _table_description(registry: ContractRegistry, dataset: DatasetContract) -> str:
    table = registry.glossary_tables.get(dataset.dataset)
    if table is not None:
        return table.description
    return dataset.external.source_table_name


def _source_columns(dataset: DatasetContract) -> list[dict[str, Any]]:
    if dataset.clickhouse_raw is None:
        return []

    parquet_fields = {field.name: field for field in dataset.parquet.fields}
    source_fields = {field.name: field for field in dataset.source.fields}
    columns: list[dict[str, Any]] = []

    for raw_field in dataset.clickhouse_raw.fields:
        parquet_field = parquet_fields.get(raw_field.from_)
        if parquet_field is None:
            msg = (
                f"{dataset.dataset} ClickHouse raw field {raw_field.name!r} references "
                f"missing parquet field {raw_field.from_!r}"
            )
            raise ValueError(msg)

        source_field = source_fields.get(parquet_field.name)
        if source_field is None:
            msg = (
                f"{dataset.dataset} parquet field {parquet_field.name!r} has no matching "
                "source field for dbt source catalog lineage"
            )
            raise ValueError(msg)

        columns.append(
            {
                "name": raw_field.name,
                "description": (
                    f"Raw source column from `{dataset.external.provider}` field "
                    f"`{source_field.name}`. 原始字段说明："
                    f"{source_field.external_description_zh}"
                ),
                "data_type": raw_field.type,
                "config": {
                    "meta": {
                        "source_field": source_field.name,
                        "parquet_field": parquet_field.name,
                        "clickhouse_raw_field": raw_field.name,
                        "external_description_zh": source_field.external_description_zh,
                    },
                },
            }
        )

    return columns


def _dump_yaml(payload: dict[str, Any]) -> str:
    import yaml

    return yaml.safe_dump(
        payload,
        allow_unicode=True,
        default_flow_style=False,
        sort_keys=False,
    )
