from __future__ import annotations

from pathlib import Path

from fleur_contracts.description_quality import validate_description_quality
from fleur_contracts.generate import generate_outputs
from fleur_contracts.loader import load_registry

PIPELINE_ROOT = Path(__file__).resolve().parents[2]


def test_contract_registry_loads_all_raw_datasets() -> None:
    registry = load_registry()

    assert len(registry.datasets) == 15
    assert {dataset.dataset for dataset in registry.datasets} >= {
        "sina__trade_calendar",
        "baostock__query_history_k_data_plus_daily",
        "jiuyan__action_field_compacted",
        "ths__limit_up_pool_compacted",
        "eastmoney__balance",
    }


def test_active_staging_fields_have_glossary_and_valid_raw_references() -> None:
    registry = load_registry()

    active_staging = [
        dataset
        for dataset in registry.datasets
        if dataset.dbt_staging is not None and dataset.dbt_staging.status == "active"
    ]

    assert len(active_staging) == 5
    for dataset in active_staging:
        raw_fields = {field.name for field in dataset.clickhouse_raw.fields}
        assert dataset.dbt_staging is not None
        for field in dataset.dbt_staging.fields:
            assert field.from_ in raw_fields
            assert field.glossary_key in registry.glossary_fields


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


def test_glossary_descriptions_are_quality_checked() -> None:
    registry = load_registry()

    issues = [
        issue
        for issue in validate_description_quality(registry)
        if "glossary/fields.yml" in issue.path
    ]

    assert issues == []
