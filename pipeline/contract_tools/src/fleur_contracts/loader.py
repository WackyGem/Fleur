from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, Protocol

import yaml

from fleur_contracts.clickhouse_types import effective_clickhouse_type
from fleur_contracts.schema import (
    ContractRegistry,
    DatasetContract,
    GlossaryTable,
)

PIPELINE_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_CONTRACT_ROOT = PIPELINE_ROOT / "contracts"


def load_registry(contract_root: Path = DEFAULT_CONTRACT_ROOT) -> ContractRegistry:
    datasets = [
        load_dataset_contract(path) for path in sorted((contract_root / "datasets").glob("*.yml"))
    ]
    glossary_tables = _load_mapping(
        contract_root / "glossary" / "tables.yml",
        model=GlossaryTable,
    )
    return ContractRegistry(
        datasets=datasets,
        glossary_tables=glossary_tables,
    )


def load_dataset_contract(path: Path) -> DatasetContract:
    contract = DatasetContract.model_validate(_load_yaml(path))
    if path.stem != contract.dataset:
        msg = f"{path} stem must match dataset {contract.dataset!r}"
        raise ValueError(msg)
    return contract


def dataset_schema_hash(contract: DatasetContract) -> str:
    payload = contract.model_dump(mode="json", by_alias=True, exclude_none=True)
    encoded = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


def clickhouse_schema_hash(contract: DatasetContract) -> str:
    if contract.clickhouse_raw is None:
        return ""
    schema_text = "\n".join(
        f"{field.name}:{effective_clickhouse_type(field.type, nullable=field.nullable)}"
        for field in contract.clickhouse_raw.fields
    )
    if contract.clickhouse_raw.partition_strategy == "year":
        schema_text = f"{schema_text}\nyear:UInt16"
    return hashlib.sha256(schema_text.encode("utf-8")).hexdigest()


def source_schema_hash(contract: DatasetContract) -> str:
    schema_text = "\n".join(f"{field.name}:{field.type}" for field in contract.source.fields)
    return hashlib.sha256(schema_text.encode("utf-8")).hexdigest()


class PydanticModel[T](Protocol):
    @classmethod
    def model_validate(cls, obj: Any) -> T: ...


def _load_mapping[T](path: Path, *, model: PydanticModel[T]) -> dict[str, T]:
    raw = _load_yaml(path)
    if not isinstance(raw, dict):
        msg = f"{path} must contain a mapping"
        raise ValueError(msg)
    return {str(key): model.model_validate(value) for key, value in raw.items()}


def _load_yaml(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        data = yaml.safe_load(handle)
    if data is None:
        msg = f"{path} is empty"
        raise ValueError(msg)
    return data
