from __future__ import annotations

import re
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

AssetKeyPath = list[str]
PartitionStrategy = Literal["snapshot", "year"]
StorageMode = Literal["latest_snapshot", "partitioned"]
PayloadFormat = Literal["json", "tabular"]
Protocol = Literal["http", "tcp", "generated"]

IDENTIFIER_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
CANONICAL_RE = re.compile(r"^[a-z][a-z0-9_]*$")


class ContractModel(BaseModel):
    model_config = ConfigDict(extra="forbid")


class ExternalSpec(ContractModel):
    provider: str
    source_table_name: str
    source_description_zh: str


class SourceField(ContractModel):
    name: str
    type: str
    required: bool = True
    external_description_zh: str


class SourceSpec(ContractModel):
    protocol: Protocol
    payload_format: PayloadFormat
    fields: list[SourceField]


class ParquetField(ContractModel):
    name: str
    type: str
    nullable: bool = False


class ParquetSpec(ContractModel):
    storage_mode: StorageMode
    partition_key_name: str | None = None
    fields: list[ParquetField]


class ClickHouseRawField(ContractModel):
    name: str
    type: str
    from_: str = Field(alias="from")
    nullable: bool = False
    default: str | None = None
    reason: str | None = None


class ClickHouseRawSpec(ContractModel):
    database: str
    table: str
    partition_strategy: PartitionStrategy
    engine: Literal["MergeTree"] = "MergeTree"
    partition_by: str
    order_by: list[str]
    allow_empty: bool = False
    sync_enabled: bool = True
    fields: list[ClickHouseRawField]


class DatasetContract(ContractModel):
    dataset: str
    version: int
    owner: str
    grain: str
    source_asset_key: AssetKeyPath
    raw_asset_key: AssetKeyPath | None = None
    external: ExternalSpec
    source: SourceSpec
    parquet: ParquetSpec
    clickhouse_raw: ClickHouseRawSpec | None = None
    dataset_note_zh: str | None = None
    validation_notes: list[str] = Field(default_factory=list)

    @field_validator("dataset")
    @classmethod
    def validate_dataset_name(cls, value: str) -> str:
        if not CANONICAL_RE.fullmatch(value):
            msg = f"dataset must be lower snake case with source prefix: {value!r}"
            raise ValueError(msg)
        return value

    @field_validator("source_asset_key", "raw_asset_key")
    @classmethod
    def validate_asset_key(cls, value: AssetKeyPath | None) -> AssetKeyPath | None:
        if value is None:
            return None
        if not value or any(not part for part in value):
            msg = "asset keys must be non-empty string path lists"
            raise ValueError(msg)
        return value

    @model_validator(mode="after")
    def validate_references(self) -> DatasetContract:
        parquet_fields = {field.name for field in self.parquet.fields}

        if self.parquet.storage_mode == "partitioned" and not self.parquet.partition_key_name:
            msg = "partitioned parquet storage requires partition_key_name"
            raise ValueError(msg)
        if self.parquet.storage_mode == "latest_snapshot" and self.parquet.partition_key_name:
            msg = "latest_snapshot parquet storage must not define partition_key_name"
            raise ValueError(msg)
        if self.parquet.partition_key_name == "":
            msg = "parquet.partition_key_name must be non-empty when provided"
            raise ValueError(msg)

        if self.clickhouse_raw is None:
            if self.raw_asset_key is not None:
                msg = "source-only dataset must not define raw_asset_key without clickhouse_raw"
                raise ValueError(msg)
            return self

        raw_fields = {field.name for field in self.clickhouse_raw.fields}

        if self.raw_asset_key is None:
            msg = "clickhouse_raw dataset requires raw_asset_key"
            raise ValueError(msg)
        if self.clickhouse_raw.table != self.dataset:
            msg = "clickhouse_raw.table must match dataset"
            raise ValueError(msg)
        if self.raw_asset_key != ["clickhouse", "raw", self.dataset]:
            msg = "raw_asset_key must be ['clickhouse', 'raw', dataset]"
            raise ValueError(msg)
        if self.clickhouse_raw.partition_strategy == "year":
            if self.parquet.storage_mode != "partitioned":
                msg = "year partition strategy requires partitioned parquet storage"
                raise ValueError(msg)
            if self.parquet.partition_key_name != "year":
                msg = "year partition strategy requires parquet.partition_key_name='year'"
                raise ValueError(msg)
        if (
            self.clickhouse_raw.partition_strategy == "snapshot"
            and self.parquet.storage_mode != "latest_snapshot"
        ):
            msg = "snapshot partition strategy requires latest_snapshot parquet storage"
            raise ValueError(msg)

        for field in self.clickhouse_raw.fields:
            if field.from_ not in parquet_fields:
                msg = f"ClickHouse field {field.name!r} references missing parquet field {field.from_!r}"
                raise ValueError(msg)
            if field.type.startswith("LowCardinality(") and not field.reason:
                msg = f"LowCardinality field {field.name!r} must include reason"
                raise ValueError(msg)

        for order_by_column in self.clickhouse_raw.order_by:
            if order_by_column not in raw_fields:
                msg = f"ORDER BY column {order_by_column!r} is not a ClickHouse raw field"
                raise ValueError(msg)

        return self


class GlossaryTable(ContractModel):
    name: str
    description_zh: str
    description: str


class ContractRegistry(ContractModel):
    datasets: list[DatasetContract]
    glossary_tables: dict[str, GlossaryTable]

    @model_validator(mode="after")
    def validate_registry(self) -> ContractRegistry:
        seen_datasets: set[str] = set()
        seen_tables: set[str] = set()
        seen_raw_assets: set[tuple[str, ...]] = set()

        for dataset in self.datasets:
            if dataset.dataset in seen_datasets:
                msg = f"duplicate dataset contract: {dataset.dataset}"
                raise ValueError(msg)
            seen_datasets.add(dataset.dataset)

            if dataset.clickhouse_raw is not None:
                table = dataset.clickhouse_raw.table
                if table in seen_tables:
                    msg = f"duplicate ClickHouse raw table: {table}"
                    raise ValueError(msg)
                seen_tables.add(table)

                if dataset.raw_asset_key is None:
                    msg = f"{dataset.dataset} clickhouse_raw dataset requires raw_asset_key"
                    raise ValueError(msg)
                raw_asset_key = tuple(dataset.raw_asset_key)
                if raw_asset_key in seen_raw_assets:
                    msg = f"duplicate raw asset key: {'/'.join(raw_asset_key)}"
                    raise ValueError(msg)
                seen_raw_assets.add(raw_asset_key)

        return self
