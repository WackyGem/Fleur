from __future__ import annotations

from fleur_contracts.adapters.parquet import (
    parquet_schema_contracts,
    pyarrow_schema_from_contract,
)
from fleur_contracts.loader import load_registry
from scheduler.defs import contract_schemas


def test_contract_schemas_cover_contract_datasets_exactly() -> None:
    contract_datasets = {dataset.dataset for dataset in load_registry().datasets}

    assert set(contract_schemas.PARQUET_SCHEMAS) == contract_datasets
    assert set(contract_schemas.PARQUET_SCHEMA_HASHES) == contract_datasets
    assert set(contract_schemas.CONTRACT_SCHEMA_HASHES) == contract_datasets
    assert set(contract_schemas.SOURCE_SCHEMA_HASHES) == contract_datasets
    assert set(contract_schemas.CONTRACT_VERSIONS) == contract_datasets
    assert set(contract_schemas.SOURCE_ASSET_KEYS) == contract_datasets
    assert set(contract_schemas.STORAGE_MODES) == contract_datasets
    assert set(contract_schemas.PARTITION_KEY_NAMES) == contract_datasets


def test_contract_schemas_match_parquet_adapter_outputs() -> None:
    contracts = parquet_schema_contracts(load_registry().datasets)

    for contract in contracts:
        assert contract_schemas.PARQUET_SCHEMAS[contract.dataset] == pyarrow_schema_from_contract(
            contract
        )
        assert (
            contract_schemas.PARQUET_SCHEMA_HASHES[contract.dataset] == contract.parquet_schema_hash
        )
        assert contract_schemas.CONTRACT_SCHEMA_HASHES[contract.dataset] == contract.schema_hash
        assert (
            contract_schemas.SOURCE_SCHEMA_HASHES[contract.dataset] == contract.source_schema_hash
        )
        assert contract_schemas.CONTRACT_VERSIONS[contract.dataset] == contract.version
        assert contract_schemas.SOURCE_ASSET_KEYS[contract.dataset] == contract.source_asset_key
        assert contract_schemas.STORAGE_MODES[contract.dataset] == contract.storage_mode
        assert contract_schemas.PARTITION_KEY_NAMES[contract.dataset] == contract.partition_key_name


def test_contract_schema_source_asset_keys_end_with_dataset_name() -> None:
    for dataset, source_asset_key in contract_schemas.SOURCE_ASSET_KEYS.items():
        assert source_asset_key[-1] == dataset
