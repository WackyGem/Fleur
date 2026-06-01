from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Literal

import dagster as dg
from fleur_contracts.adapters.clickhouse import build_scheduler_specs
from fleur_contracts.loader import load_registry

from scheduler.defs.storage.s3 import StorageMode

PartitionStrategy = Literal["snapshot", "year"]

CLICKHOUSE_RAW_ASSET_PREFIX = ("clickhouse", "raw")
CLICKHOUSE_RAW_GROUP = "clickhouse_raw"
LOW_CARDINALITY_UNIQUE_LIMIT = 10_000


@dataclass(frozen=True)
class ClickHouseColumnSpec:
    name: str
    pyarrow_type: str
    clickhouse_type: str

    def __post_init__(self) -> None:
        validate_identifier(self.name, field_name="column name")
        if not self.pyarrow_type:
            msg = f"Column {self.name!r} is missing a PyArrow type"
            raise ValueError(msg)
        if not self.clickhouse_type:
            msg = f"Column {self.name!r} is missing a ClickHouse type"
            raise ValueError(msg)


@dataclass(frozen=True)
class ClickHouseRawTableSpec:
    contract_dataset: str
    contract_version: int
    contract_schema_hash: str
    source_schema_hash: str
    clickhouse_schema_hash: str
    source_asset_key: dg.AssetKey
    raw_asset_key: dg.AssetKey
    storage_mode: StorageMode
    source_partition_key_name: str | None
    clickhouse_database: str
    clickhouse_table: str
    staging_table: str
    partition_strategy: PartitionStrategy
    order_by: tuple[str, ...]
    columns: tuple[ClickHouseColumnSpec, ...]
    allow_empty: bool
    sync_enabled: bool
    low_cardinality_unique_limit: int = LOW_CARDINALITY_UNIQUE_LIMIT

    def __post_init__(self) -> None:
        if tuple(self.raw_asset_key.path[:2]) != CLICKHOUSE_RAW_ASSET_PREFIX:
            msg = f"Raw asset key must start with clickhouse/raw: {self.raw_asset_key}"
            raise ValueError(msg)
        if self.partition_strategy == "year":
            if self.storage_mode != "partitioned":
                msg = "Year partition raw sync specs must use partitioned storage mode"
                raise ValueError(msg)
            if self.source_partition_key_name != "year":
                msg = "Year partition raw sync specs must use source_partition_key_name='year'"
                raise ValueError(msg)
            if not self.order_by:
                msg = "Year partition raw sync specs must define ORDER BY columns"
                raise ValueError(msg)
        if self.partition_strategy == "snapshot" and self.storage_mode != "latest_snapshot":
            msg = "Snapshot raw sync specs must use latest_snapshot storage mode"
            raise ValueError(msg)
        if not self.columns:
            msg = f"Raw sync spec {self.raw_asset_key.to_user_string()} has no columns"
            raise ValueError(msg)

        validate_identifier(self.clickhouse_database, field_name="database")
        validate_identifier(self.clickhouse_table, field_name="table")
        validate_identifier(self.staging_table, field_name="staging table")
        for order_by_column in self.order_by:
            validate_identifier(order_by_column, field_name="ORDER BY column")

    @property
    def raw_asset_table_name(self) -> str:
        return self.raw_asset_key.path[-1]

    @property
    def partition_column(self) -> ClickHouseColumnSpec | None:
        if self.partition_strategy != "year":
            return None
        return ClickHouseColumnSpec(
            name="year",
            pyarrow_type="partition",
            clickhouse_type="UInt16",
        )

    @property
    def table_columns(self) -> tuple[ClickHouseColumnSpec, ...]:
        partition_column = self.partition_column
        if partition_column is None:
            return self.columns
        return (*self.columns, partition_column)

    @property
    def low_cardinality_columns(self) -> tuple[ClickHouseColumnSpec, ...]:
        return tuple(
            column
            for column in self.columns
            if column.clickhouse_type.startswith("LowCardinality(")
        )


def validate_identifier(value: str, *, field_name: str) -> None:
    if not value:
        msg = f"ClickHouse {field_name} is empty"
        raise ValueError(msg)
    if value[0].isdigit():
        msg = f"ClickHouse {field_name} cannot start with a digit: {value!r}"
        raise ValueError(msg)
    if not all(character == "_" or character.isalnum() for character in value):
        msg = f"ClickHouse {field_name} contains unsupported characters: {value!r}"
        raise ValueError(msg)


def raw_asset_key(table_name: str) -> dg.AssetKey:
    return dg.AssetKey([*CLICKHOUSE_RAW_ASSET_PREFIX, table_name])


def enabled_specs(specs: Sequence[ClickHouseRawTableSpec]) -> tuple[ClickHouseRawTableSpec, ...]:
    return tuple(spec for spec in specs if spec.sync_enabled)


CLICKHOUSE_RAW_TABLE_SPECS: tuple[ClickHouseRawTableSpec, ...] = build_scheduler_specs(
    load_registry().datasets,
    asset_key_factory=dg.AssetKey,
    table_spec_factory=ClickHouseRawTableSpec,
    column_spec_factory=ClickHouseColumnSpec,
)
BAOSTOCK_DAILY_K_SPEC = next(
    spec
    for spec in CLICKHOUSE_RAW_TABLE_SPECS
    if spec.clickhouse_table == "baostock__query_history_k_data_plus_daily"
)
ENABLED_CLICKHOUSE_RAW_TABLE_SPECS = enabled_specs(CLICKHOUSE_RAW_TABLE_SPECS)
