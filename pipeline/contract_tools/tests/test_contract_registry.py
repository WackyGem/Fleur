from __future__ import annotations

from pathlib import Path

import pytest
from fleur_contracts.adapters.clickhouse import raw_table_contracts
from fleur_contracts.adapters.data_dict import render_data_dict_markdown
from fleur_contracts.adapters.dbt import render_sources_yaml
from fleur_contracts.description_quality import validate_description_quality
from fleur_contracts.generate import generate_outputs
from fleur_contracts.loader import load_registry
from fleur_contracts.schema import (
    ContractRegistry,
    DatasetContract,
)

PIPELINE_ROOT = Path(__file__).resolve().parents[2]


def test_contract_registry_loads_raw_and_source_only_datasets() -> None:
    registry = load_registry()

    raw_datasets = [dataset for dataset in registry.datasets if dataset.clickhouse_raw is not None]
    source_only_datasets = [
        dataset for dataset in registry.datasets if dataset.clickhouse_raw is None
    ]

    assert len(registry.datasets) == 18
    assert len(raw_datasets) == 15
    assert {dataset.dataset for dataset in source_only_datasets} == {
        "jiuyan__action_field",
        "jiuyan__industry_ocr",
        "ths__limit_up_pool",
    }
    assert {dataset.dataset for dataset in registry.datasets} >= {
        "sina__trade_calendar",
        "baostock__query_history_k_data_plus_daily",
        "jiuyan__action_field_compacted",
        "ths__limit_up_pool_compacted",
        "eastmoney__balance",
    }


def test_generated_outputs_are_current() -> None:
    assert generate_outputs(check=True) == []


def test_source_external_descriptions_are_quality_checked() -> None:
    registry = load_registry()

    issues = [
        issue
        for issue in validate_description_quality(registry)
        if ".external_description_zh" in issue.path
    ]

    assert issues == []


def test_source_only_dataset_contract_does_not_require_clickhouse_raw() -> None:
    contract = _source_only_contract()

    assert contract.raw_asset_key is None
    assert contract.clickhouse_raw is None


def test_source_only_dataset_rejects_raw_asset_key_without_clickhouse_raw() -> None:
    payload = _source_only_payload()
    payload["raw_asset_key"] = ["clickhouse", "raw", "demo__source_only"]

    with pytest.raises(ValueError, match="must not define raw_asset_key"):
        DatasetContract.model_validate(payload)


def test_source_only_dataset_is_excluded_from_dbt_sources_and_clickhouse_specs() -> None:
    source_only = _source_only_contract()
    raw_dataset = _raw_contract()
    registry = _registry_with(source_only, raw_dataset)

    sources_yaml = render_sources_yaml(registry)
    raw_contracts = raw_table_contracts(registry.datasets)

    assert "demo__source_only" not in sources_yaml
    assert "demo__raw_table" in sources_yaml
    assert [contract.dataset for contract in raw_contracts] == ["demo__raw_table"]


def test_dbt_sources_include_raw_column_catalog() -> None:
    registry = _registry_with(_raw_contract())

    sources_yaml = render_sources_yaml(registry)

    assert "name: raw" in sources_yaml
    assert "schema: fleur_raw" in sources_yaml
    assert "clickhouse_raw_table: fleur_raw.demo__raw_table" in sources_yaml
    assert "source_schema_hash:" in sources_yaml
    assert "parquet_schema_hash:" in sources_yaml
    assert "clickhouse_schema_hash:" in sources_yaml
    assert "columns:" in sources_yaml
    assert "name: demo_id" in sources_yaml
    assert "data_type: String" in sources_yaml
    assert "source_field: demo_id" in sources_yaml
    assert "parquet_field: demo_id" in sources_yaml
    assert "clickhouse_raw_field: demo_id" in sources_yaml
    assert "external_description_zh: 演示记录唯一标识" in sources_yaml


def test_source_only_data_dict_omits_clickhouse_columns() -> None:
    contract = _source_only_contract()
    registry = _registry_with(contract)

    markdown = render_data_dict_markdown(registry, contract)

    assert "- Raw asset：不适用" in markdown
    assert "- ClickHouse raw：不适用" in markdown
    assert "ClickHouse raw 字段" not in markdown
    assert "| # | 外源字段 | 外源类型 | Parquet 类型 | 中文描述 |" in markdown


def test_raw_data_dict_omits_staging_columns() -> None:
    contract = _raw_contract()
    registry = _registry_with(contract)

    markdown = render_data_dict_markdown(registry, contract)

    assert "ClickHouse 类型 | stg" not in markdown
    assert (
        "| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | "
        "ClickHouse 类型 | 中文描述 |"
    ) in markdown


def _source_only_payload() -> dict[str, object]:
    return {
        "dataset": "demo__source_only",
        "version": 1,
        "owner": "data",
        "grain": "one row per demo id",
        "source_asset_key": ["source", "demo__source_only"],
        "external": {
            "provider": "demo",
            "source_table_name": "demo__source_only",
            "source_description_zh": "演示 source-only 数据集",
        },
        "source": {
            "protocol": "http",
            "payload_format": "tabular",
            "fields": [
                {
                    "name": "demo_id",
                    "type": "string",
                    "external_description_zh": "演示记录唯一标识",
                }
            ],
        },
        "parquet": {
            "storage_mode": "latest_snapshot",
            "fields": [
                {
                    "name": "demo_id",
                    "type": "string",
                    "nullable": False,
                }
            ],
        },
    }


def _source_only_contract() -> DatasetContract:
    return DatasetContract.model_validate(_source_only_payload())


def _raw_contract() -> DatasetContract:
    return DatasetContract.model_validate(
        {
            "dataset": "demo__raw_table",
            "version": 1,
            "owner": "data",
            "grain": "one row per demo id",
            "source_asset_key": ["source", "demo__raw_table"],
            "raw_asset_key": ["clickhouse", "raw", "demo__raw_table"],
            "external": {
                "provider": "demo",
                "source_table_name": "demo__raw_table",
                "source_description_zh": "演示 raw 数据集",
            },
            "source": {
                "protocol": "http",
                "payload_format": "tabular",
                "fields": [
                    {
                        "name": "demo_id",
                        "type": "string",
                        "external_description_zh": "演示记录唯一标识",
                    }
                ],
            },
            "parquet": {
                "storage_mode": "latest_snapshot",
                "fields": [
                    {
                        "name": "demo_id",
                        "type": "string",
                        "nullable": False,
                    }
                ],
            },
            "clickhouse_raw": {
                "database": "fleur_raw",
                "table": "demo__raw_table",
                "partition_strategy": "snapshot",
                "partition_by": "tuple()",
                "order_by": ["demo_id"],
                "fields": [
                    {
                        "name": "demo_id",
                        "type": "String",
                        "from": "demo_id",
                        "nullable": False,
                    }
                ],
            },
        }
    )


def _registry_with(*datasets: DatasetContract) -> ContractRegistry:
    return ContractRegistry(
        datasets=list(datasets),
        glossary_tables={},
    )
