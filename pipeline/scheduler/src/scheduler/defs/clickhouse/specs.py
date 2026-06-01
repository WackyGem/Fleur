from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Literal

import dagster as dg

from scheduler.defs.market.asset_keys import (
    BAOSTOCK_DAILY_K_ASSET_KEY,
    BAOSTOCK_STOCK_BASIC_ASSET_KEY,
    SINA_TRADE_CALENDAR_ASSET_KEY,
    SOURCE_ASSET_KEY_PREFIX,
)
from scheduler.defs.storage.s3 import StorageMode

PartitionStrategy = Literal["snapshot", "year"]

CLICKHOUSE_RAW_ASSET_PREFIX = ("clickhouse", "raw")
CLICKHOUSE_RAW_GROUP = "clickhouse_raw"
LOW_CARDINALITY_UNIQUE_LIMIT = 10_000
REPO_ROOT = Path(__file__).resolve().parents[6]
DATA_DICT_DIR = REPO_ROOT / "docs" / "references" / "data_dict"


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


def source_asset_key(asset_name: str) -> dg.AssetKey:
    return dg.AssetKey([SOURCE_ASSET_KEY_PREFIX, asset_name])


def enabled_specs(specs: Sequence[ClickHouseRawTableSpec]) -> tuple[ClickHouseRawTableSpec, ...]:
    return tuple(spec for spec in specs if spec.sync_enabled)


def columns_from_data_dict(data_dict_name: str) -> tuple[ClickHouseColumnSpec, ...]:
    path = DATA_DICT_DIR / f"{data_dict_name}.md"
    columns_by_name: dict[str, tuple[ClickHouseColumnSpec, bool]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped.startswith("|") or "✅" not in stripped:
            continue

        parts = [part.strip() for part in stripped.strip("|").split("|")]
        original_field_name = parts[1]
        field_name = original_field_name.split(".")[-1]
        is_exact_field = "." not in original_field_name
        pyarrow_type = parts[4]
        clickhouse_type = parts[5]
        if clickhouse_type == "-":
            msg = f"{path} has enabled field {field_name!r} without ClickHouse type"
            raise ValueError(msg)

        column = ClickHouseColumnSpec(
            name=field_name,
            pyarrow_type=pyarrow_type,
            clickhouse_type=clickhouse_type,
        )
        existing = columns_by_name.get(field_name)
        if existing is not None:
            existing_column, existing_is_exact = existing
            if existing_column == column:
                continue
            if existing_is_exact and not is_exact_field:
                continue
            if not existing_is_exact and is_exact_field:
                columns_by_name[field_name] = (column, is_exact_field)
                continue
            msg = (
                f"{path} has conflicting data_dict entries for {field_name!r}: "
                f"{existing_column} != {column}"
            )
            raise ValueError(msg)
        columns_by_name[field_name] = (column, is_exact_field)

    if not columns_by_name:
        msg = f"{path} did not yield any enabled data_dict columns"
        raise ValueError(msg)
    return tuple(column for column, _ in columns_by_name.values())


def build_snapshot_spec(
    *,
    asset_name: str,
    order_by: tuple[str, ...],
    source_asset_key_override: dg.AssetKey | None = None,
    sync_enabled: bool = True,
) -> ClickHouseRawTableSpec:
    source_key = source_asset_key_override or source_asset_key(asset_name)
    return ClickHouseRawTableSpec(
        source_asset_key=source_key,
        raw_asset_key=raw_asset_key(asset_name),
        storage_mode="latest_snapshot",
        source_partition_key_name=None,
        clickhouse_database="raw",
        clickhouse_table=asset_name,
        staging_table=f"{asset_name}__stage",
        partition_strategy="snapshot",
        order_by=order_by,
        columns=columns_from_data_dict(asset_name),
        allow_empty=False,
        sync_enabled=sync_enabled,
    )


def build_year_spec(
    *,
    asset_name: str,
    data_dict_name: str,
    order_by: tuple[str, ...],
    source_asset_key_override: dg.AssetKey | None = None,
    allow_empty: bool = False,
    sync_enabled: bool = True,
) -> ClickHouseRawTableSpec:
    source_key = source_asset_key_override or source_asset_key(asset_name)
    return ClickHouseRawTableSpec(
        source_asset_key=source_key,
        raw_asset_key=raw_asset_key(asset_name),
        storage_mode="partitioned",
        source_partition_key_name="year",
        clickhouse_database="raw",
        clickhouse_table=asset_name,
        staging_table=f"{asset_name}__stage",
        partition_strategy="year",
        order_by=order_by,
        columns=columns_from_data_dict(data_dict_name),
        allow_empty=allow_empty,
        sync_enabled=sync_enabled,
    )


SNAPSHOT_SPECS: tuple[ClickHouseRawTableSpec, ...] = (
    build_snapshot_spec(
        asset_name="sina__trade_calendar",
        source_asset_key_override=SINA_TRADE_CALENDAR_ASSET_KEY,
        order_by=("trade_date",),
    ),
    build_snapshot_spec(
        asset_name="baostock__query_stock_basic",
        source_asset_key_override=BAOSTOCK_STOCK_BASIC_ASSET_KEY,
        order_by=("code",),
    ),
    build_snapshot_spec(
        asset_name="jiuyan__industry_list",
        order_by=("industry_id",),
    ),
    build_snapshot_spec(
        asset_name="jiuyan__industry_ocr_snapshot",
        order_by=("industry_id", "image_filename", "ocr_row_index"),
    ),
)

BAOSTOCK_DAILY_K_SPEC = ClickHouseRawTableSpec(
    source_asset_key=BAOSTOCK_DAILY_K_ASSET_KEY,
    raw_asset_key=raw_asset_key("baostock__query_history_k_data_plus_daily"),
    storage_mode="partitioned",
    source_partition_key_name="year",
    clickhouse_database="raw",
    clickhouse_table="baostock__query_history_k_data_plus_daily",
    staging_table="baostock__query_history_k_data_plus_daily__stage",
    partition_strategy="year",
    order_by=("code", "date"),
    columns=columns_from_data_dict("baostock__query_history_k_data_plus_daily"),
    allow_empty=False,
    sync_enabled=True,
)

COMPACTED_YEAR_SPECS: tuple[ClickHouseRawTableSpec, ...] = (
    build_year_spec(
        asset_name="jiuyan__action_field_compacted",
        data_dict_name="jiuyan__action_field",
        order_by=("date", "code"),
    ),
    build_year_spec(
        asset_name="ths__limit_up_pool_compacted",
        data_dict_name="ths__limit_up_pool",
        order_by=("date", "code"),
    ),
)


def order_by_for_data_dict(data_dict_name: str) -> tuple[str, ...]:
    column_names = {column.name for column in columns_from_data_dict(data_dict_name)}
    if {"SECUCODE", "REPORT_DATE"} <= column_names:
        return ("SECUCODE", "REPORT_DATE")
    if {"SECUCODE", "END_DATE"} <= column_names:
        return ("SECUCODE", "END_DATE")
    if {"SECUCODE", "NOTICE_DATE"} <= column_names:
        return ("SECUCODE", "NOTICE_DATE")

    msg = f"Cannot infer EastMoney ORDER BY columns for {data_dict_name}"
    raise ValueError(msg)


EASTMONEY_YEAR_SPECS: tuple[ClickHouseRawTableSpec, ...] = tuple(
    build_year_spec(
        asset_name=asset_name,
        data_dict_name=asset_name,
        order_by=order_by_for_data_dict(asset_name),
        allow_empty=True,
    )
    for asset_name in (
        "eastmoney__balance",
        "eastmoney__cashflow_sq",
        "eastmoney__cashflow_ytd",
        "eastmoney__dividend_allotment",
        "eastmoney__dividend_main",
        "eastmoney__equity_history",
        "eastmoney__income_sq",
        "eastmoney__income_ytd",
    )
)

CLICKHOUSE_RAW_TABLE_SPECS: tuple[ClickHouseRawTableSpec, ...] = (
    BAOSTOCK_DAILY_K_SPEC,
    *SNAPSHOT_SPECS,
    *COMPACTED_YEAR_SPECS,
    *EASTMONEY_YEAR_SPECS,
)
ENABLED_CLICKHOUSE_RAW_TABLE_SPECS = enabled_specs(CLICKHOUSE_RAW_TABLE_SPECS)
