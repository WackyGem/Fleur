from __future__ import annotations

import hashlib
import json
from collections.abc import Sequence
from dataclasses import dataclass

import pyarrow as pa

from fleur_contracts.loader import dataset_schema_hash, source_schema_hash
from fleur_contracts.schema import DatasetContract


@dataclass(frozen=True)
class ParquetFieldContract:
    name: str
    type: str
    nullable: bool


@dataclass(frozen=True)
class ParquetSchemaContract:
    dataset: str
    version: int
    schema_hash: str
    source_schema_hash: str
    parquet_schema_hash: str
    source_asset_key: tuple[str, ...]
    storage_mode: str
    partition_key_name: str | None
    fields: tuple[ParquetFieldContract, ...]


def parquet_schema_hash(contract: DatasetContract) -> str:
    payload = [
        {
            "name": field.name,
            "type": field.type,
            "nullable": field.nullable,
        }
        for field in contract.parquet.fields
    ]
    encoded = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


def parquet_schema_contract_from_dataset(contract: DatasetContract) -> ParquetSchemaContract:
    return ParquetSchemaContract(
        dataset=contract.dataset,
        version=contract.version,
        schema_hash=dataset_schema_hash(contract),
        source_schema_hash=source_schema_hash(contract),
        parquet_schema_hash=parquet_schema_hash(contract),
        source_asset_key=tuple(contract.source_asset_key),
        storage_mode=contract.parquet.storage_mode,
        partition_key_name=contract.parquet.partition_key_name,
        fields=tuple(
            ParquetFieldContract(
                name=field.name,
                type=field.type,
                nullable=field.nullable,
            )
            for field in contract.parquet.fields
        ),
    )


def parquet_schema_contracts(
    contracts: Sequence[DatasetContract],
) -> tuple[ParquetSchemaContract, ...]:
    return tuple(parquet_schema_contract_from_dataset(contract) for contract in contracts)


def pyarrow_type_from_contract(type_text: str) -> pa.DataType:
    if type_text == "string":
        return pa.string()
    if type_text in {"date32", "date32[day]"}:
        return pa.date32()
    if type_text == "bool":
        return pa.bool_()
    if type_text == "int8":
        return pa.int8()
    if type_text == "int32":
        return pa.int32()
    if type_text == "int64":
        return pa.int64()
    if type_text in {"float64", "double"}:
        return pa.float64()
    if type_text == "timestamp[s]":
        return pa.timestamp("s")
    if type_text == "timestamp[s, tz=UTC]":
        return pa.timestamp("s", tz="UTC")
    if type_text == "timestamp[ms]":
        return pa.timestamp("ms")
    if type_text == "timestamp[ms, tz=UTC]":
        return pa.timestamp("ms", tz="UTC")
    if type_text == "timestamp[ns]":
        return pa.timestamp("ns")
    if type_text == "timestamp[ns, tz=UTC]":
        return pa.timestamp("ns", tz="UTC")
    if type_text == "time32[ms]":
        return pa.time32("ms")

    msg = f"Unsupported Parquet type: {type_text!r}"
    raise ValueError(msg)


def pyarrow_type_expression_from_contract(type_text: str) -> str:
    pyarrow_type_from_contract(type_text)
    if type_text == "string":
        return "pa.string()"
    if type_text in {"date32", "date32[day]"}:
        return "pa.date32()"
    if type_text == "bool":
        return "pa.bool_()"
    if type_text == "int8":
        return "pa.int8()"
    if type_text == "int32":
        return "pa.int32()"
    if type_text == "int64":
        return "pa.int64()"
    if type_text in {"float64", "double"}:
        return "pa.float64()"
    if type_text == "timestamp[s]":
        return 'pa.timestamp("s")'
    if type_text == "timestamp[s, tz=UTC]":
        return 'pa.timestamp("s", tz="UTC")'
    if type_text == "timestamp[ms]":
        return 'pa.timestamp("ms")'
    if type_text == "timestamp[ms, tz=UTC]":
        return 'pa.timestamp("ms", tz="UTC")'
    if type_text == "timestamp[ns]":
        return 'pa.timestamp("ns")'
    if type_text == "timestamp[ns, tz=UTC]":
        return 'pa.timestamp("ns", tz="UTC")'
    if type_text == "time32[ms]":
        return 'pa.time32("ms")'
    msg = f"Unsupported Parquet type: {type_text!r}"
    raise ValueError(msg)


def pyarrow_schema_from_contract(contract: ParquetSchemaContract) -> pa.Schema:
    fields = []
    for field in contract.fields:
        try:
            dtype = pyarrow_type_from_contract(field.type)
        except ValueError as error:
            msg = (
                f"Unsupported Parquet type for dataset={contract.dataset!r}, "
                f"field={field.name!r}, type={field.type!r}"
            )
            raise ValueError(msg) from error
        fields.append(pa.field(field.name, dtype, nullable=field.nullable))
    return pa.schema(fields)


def pyarrow_schema_by_dataset(
    contracts: Sequence[DatasetContract],
) -> dict[str, pa.Schema]:
    return {
        contract.dataset: pyarrow_schema_from_contract(contract)
        for contract in parquet_schema_contracts(contracts)
    }
