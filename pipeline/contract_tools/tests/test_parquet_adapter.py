from __future__ import annotations

from copy import deepcopy

import pyarrow as pa
import pytest
from fleur_contracts.adapters.parquet import (
    parquet_schema_contract_from_dataset,
    parquet_schema_contracts,
    parquet_schema_hash,
    pyarrow_schema_by_dataset,
    pyarrow_schema_from_contract,
    pyarrow_type_expression_from_contract,
    pyarrow_type_from_contract,
)
from fleur_contracts.loader import load_registry
from fleur_contracts.schema import DatasetContract


def test_parquet_adapter_loads_all_contract_datasets() -> None:
    registry = load_registry()

    contracts = parquet_schema_contracts(registry.datasets)
    schemas = pyarrow_schema_by_dataset(registry.datasets)

    assert len(contracts) == 19
    assert set(schemas) == {contract.dataset for contract in registry.datasets}


def test_parquet_schema_matches_contract_fields_exactly() -> None:
    registry = load_registry()

    for dataset in registry.datasets:
        schema = pyarrow_schema_from_contract(parquet_schema_contract_from_dataset(dataset))

        assert [(field.name, str(field.type), field.nullable) for field in schema] == [
            (
                field.name,
                str(pyarrow_type_from_contract(field.type)),
                field.nullable,
            )
            for field in dataset.parquet.fields
        ]


def test_unsupported_type_error_includes_dataset_field_and_type() -> None:
    payload = load_registry().datasets[0].model_dump(mode="json", by_alias=True)
    payload["parquet"]["fields"][0]["type"] = "unsupported_type"
    contract = DatasetContract.model_validate(payload)

    with pytest.raises(
        ValueError,
        match="dataset='baostock__query_history_k_data_plus_daily'.*field='date'.*unsupported_type",
    ):
        pyarrow_schema_from_contract(parquet_schema_contract_from_dataset(contract))


def test_parquet_schema_hash_is_sensitive_to_schema_facts() -> None:
    contract = load_registry().datasets[0]
    baseline = parquet_schema_hash(contract)

    renamed_payload = contract.model_dump(mode="json", by_alias=True)
    renamed_payload["parquet"]["fields"][0]["name"] = "renamed_date"
    renamed_payload["clickhouse_raw"]["fields"][0]["from"] = "renamed_date"

    retyped_payload = contract.model_dump(mode="json", by_alias=True)
    retyped_payload["parquet"]["fields"][0]["type"] = "string"

    nullable_payload = contract.model_dump(mode="json", by_alias=True)
    nullable_payload["parquet"]["fields"][0]["nullable"] = True

    reordered_payload = deepcopy(contract.model_dump(mode="json", by_alias=True))
    first = reordered_payload["parquet"]["fields"].pop(0)
    reordered_payload["parquet"]["fields"].append(first)

    assert parquet_schema_hash(DatasetContract.model_validate(renamed_payload)) != baseline
    assert parquet_schema_hash(DatasetContract.model_validate(retyped_payload)) != baseline
    assert parquet_schema_hash(DatasetContract.model_validate(nullable_payload)) != baseline
    assert parquet_schema_hash(DatasetContract.model_validate(reordered_payload)) != baseline


def test_supported_parquet_type_parser() -> None:
    assert pyarrow_type_from_contract("string") == pa.string()
    assert pyarrow_type_from_contract("date32") == pa.date32()
    assert pyarrow_type_from_contract("date32[day]") == pa.date32()
    assert pyarrow_type_from_contract("bool") == pa.bool_()
    assert pyarrow_type_from_contract("int8") == pa.int8()
    assert pyarrow_type_from_contract("int32") == pa.int32()
    assert pyarrow_type_from_contract("int64") == pa.int64()
    assert pyarrow_type_from_contract("float64") == pa.float64()
    assert pyarrow_type_from_contract("double") == pa.float64()
    assert pyarrow_type_from_contract("timestamp[s]") == pa.timestamp("s")
    assert pyarrow_type_from_contract("timestamp[s, tz=UTC]") == pa.timestamp("s", tz="UTC")
    assert pyarrow_type_from_contract("timestamp[ms]") == pa.timestamp("ms")
    assert pyarrow_type_from_contract("timestamp[ms, tz=UTC]") == pa.timestamp("ms", tz="UTC")
    assert pyarrow_type_from_contract("timestamp[ns]") == pa.timestamp("ns")
    assert pyarrow_type_from_contract("timestamp[ns, tz=UTC]") == pa.timestamp("ns", tz="UTC")
    assert pyarrow_type_from_contract("time32[ms]") == pa.time32("ms")
    assert pyarrow_type_expression_from_contract("timestamp[s]") == 'pa.timestamp("s")'
    assert pyarrow_type_expression_from_contract("timestamp[s, tz=UTC]") == (
        'pa.timestamp("s", tz="UTC")'
    )
    assert pyarrow_type_expression_from_contract("timestamp[ms]") == 'pa.timestamp("ms")'
    assert pyarrow_type_expression_from_contract("timestamp[ms, tz=UTC]") == (
        'pa.timestamp("ms", tz="UTC")'
    )
    assert pyarrow_type_expression_from_contract("timestamp[ns, tz=UTC]") == (
        'pa.timestamp("ns", tz="UTC")'
    )
