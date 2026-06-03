from __future__ import annotations

from collections.abc import Callable, Sequence
from dataclasses import dataclass
from typing import Any

from fleur_contracts.clickhouse_types import effective_clickhouse_type
from fleur_contracts.loader import clickhouse_schema_hash, dataset_schema_hash, source_schema_hash
from fleur_contracts.schema import DatasetContract


@dataclass(frozen=True)
class ClickHouseColumnContract:
    name: str
    pyarrow_type: str
    clickhouse_type: str


@dataclass(frozen=True)
class ClickHouseRawTableContract:
    dataset: str
    version: int
    schema_hash: str
    source_schema_hash: str
    clickhouse_schema_hash: str
    source_asset_key: tuple[str, ...]
    raw_asset_key: tuple[str, ...]
    storage_mode: str
    source_partition_key_name: str | None
    clickhouse_database: str
    clickhouse_table: str
    staging_table: str
    partition_strategy: str
    order_by: tuple[str, ...]
    columns: tuple[ClickHouseColumnContract, ...]
    allow_empty: bool
    sync_enabled: bool


def raw_table_contract_from_dataset(contract: DatasetContract) -> ClickHouseRawTableContract:
    if contract.clickhouse_raw is None or contract.raw_asset_key is None:
        msg = f"{contract.dataset} does not define a ClickHouse raw table"
        raise ValueError(msg)
    parquet_fields = {field.name: field for field in contract.parquet.fields}
    columns = tuple(
        ClickHouseColumnContract(
            name=field.name,
            pyarrow_type=parquet_fields[field.from_].type,
            clickhouse_type=effective_clickhouse_type(
                field.type,
                nullable=field.nullable,
            ),
        )
        for field in contract.clickhouse_raw.fields
    )
    return ClickHouseRawTableContract(
        dataset=contract.dataset,
        version=contract.version,
        schema_hash=dataset_schema_hash(contract),
        source_schema_hash=source_schema_hash(contract),
        clickhouse_schema_hash=clickhouse_schema_hash(contract),
        source_asset_key=tuple(contract.source_asset_key),
        raw_asset_key=tuple(contract.raw_asset_key),
        storage_mode=contract.parquet.storage_mode,
        source_partition_key_name=contract.parquet.partition_key_name,
        clickhouse_database=contract.clickhouse_raw.database,
        clickhouse_table=contract.clickhouse_raw.table,
        staging_table=f"{contract.dataset}__stage",
        partition_strategy=contract.clickhouse_raw.partition_strategy,
        order_by=tuple(contract.clickhouse_raw.order_by),
        columns=columns,
        allow_empty=contract.clickhouse_raw.allow_empty,
        sync_enabled=contract.clickhouse_raw.sync_enabled,
    )


def raw_table_contracts(
    contracts: Sequence[DatasetContract],
) -> tuple[ClickHouseRawTableContract, ...]:
    return tuple(
        raw_table_contract_from_dataset(contract)
        for contract in contracts
        if contract.clickhouse_raw is not None
    )


def build_scheduler_specs(
    contracts: Sequence[DatasetContract],
    *,
    asset_key_factory: Callable[[Sequence[str]], Any],
    table_spec_factory: Callable[..., Any],
    column_spec_factory: Callable[..., Any],
) -> tuple[Any, ...]:
    specs = []
    for contract in raw_table_contracts(contracts):
        specs.append(
            table_spec_factory(
                contract_dataset=contract.dataset,
                contract_version=contract.version,
                contract_schema_hash=contract.schema_hash,
                source_schema_hash=contract.source_schema_hash,
                clickhouse_schema_hash=contract.clickhouse_schema_hash,
                source_asset_key=asset_key_factory(contract.source_asset_key),
                raw_asset_key=asset_key_factory(contract.raw_asset_key),
                storage_mode=contract.storage_mode,
                source_partition_key_name=contract.source_partition_key_name,
                clickhouse_database=contract.clickhouse_database,
                clickhouse_table=contract.clickhouse_table,
                staging_table=contract.staging_table,
                partition_strategy=contract.partition_strategy,
                order_by=contract.order_by,
                columns=tuple(
                    column_spec_factory(
                        name=column.name,
                        pyarrow_type=column.pyarrow_type,
                        clickhouse_type=column.clickhouse_type,
                    )
                    for column in contract.columns
                ),
                allow_empty=contract.allow_empty,
                sync_enabled=contract.sync_enabled,
            )
        )
    return tuple(specs)
