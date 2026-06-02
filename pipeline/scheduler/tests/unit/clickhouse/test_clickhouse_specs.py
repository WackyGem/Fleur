from __future__ import annotations

from fleur_contracts.loader import load_registry
from scheduler.defs.clickhouse.specs import (
    BAOSTOCK_DAILY_K_SPEC,
    ENABLED_CLICKHOUSE_RAW_POOL_NAMES,
    ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
)


def test_enabled_specs_cover_initial_snapshot_compacted_and_eastmoney_groups() -> None:
    raw_asset_keys = {
        spec.raw_asset_key.to_user_string() for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
    }

    assert len(raw_asset_keys) == 15
    assert "clickhouse/raw/baostock__query_history_k_data_plus_daily" in raw_asset_keys
    assert "clickhouse/raw/sina__trade_calendar" in raw_asset_keys
    assert "clickhouse/raw/jiuyan__industry_ocr_snapshot" in raw_asset_keys
    assert "clickhouse/raw/jiuyan__action_field_compacted" in raw_asset_keys
    assert "clickhouse/raw/ths__limit_up_pool_compacted" in raw_asset_keys
    assert "clickhouse/raw/eastmoney__balance" in raw_asset_keys


def test_baostock_spec_uses_year_partition_and_query_driven_order_by() -> None:
    spec = BAOSTOCK_DAILY_K_SPEC

    assert spec.source_asset_key.to_user_string() == (
        "source/baostock__query_history_k_data_plus_daily"
    )
    assert spec.storage_mode == "partitioned"
    assert spec.source_partition_key_name == "year"
    assert spec.partition_strategy == "year"
    assert spec.order_by == ("code", "date")
    assert spec.partition_column is not None
    assert spec.partition_column.clickhouse_type == "UInt16"


def test_enabled_spec_columns_match_baostock_contract() -> None:
    contract = next(
        dataset
        for dataset in load_registry().datasets
        if dataset.dataset == "baostock__query_history_k_data_plus_daily"
    )
    parquet_fields = {field.name: field for field in contract.parquet.fields}

    assert contract.clickhouse_raw is not None
    assert {
        column.name: (column.pyarrow_type, column.clickhouse_type)
        for column in BAOSTOCK_DAILY_K_SPEC.columns
    } == {
        field.name: (parquet_fields[field.from_].type, field.type)
        for field in contract.clickhouse_raw.fields
    }


def test_all_enabled_spec_columns_have_contract_clickhouse_types() -> None:
    for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS:
        for column in spec.columns:
            assert column.clickhouse_type != "-"
            assert column.pyarrow_type != "-"


def test_sparse_daily_assets_are_not_enabled_for_raw_sync() -> None:
    enabled_source_assets = {
        spec.source_asset_key.to_user_string() for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
    }

    assert "source/jiuyan__action_field" not in enabled_source_assets
    assert "source/ths__limit_up_pool" not in enabled_source_assets
    assert "source/jiuyan__industry_ocr" not in enabled_source_assets


def test_enabled_raw_sync_pool_names_are_dataset_level() -> None:
    assert len(ENABLED_CLICKHOUSE_RAW_POOL_NAMES) == len(ENABLED_CLICKHOUSE_RAW_TABLE_SPECS)
    assert len(set(ENABLED_CLICKHOUSE_RAW_POOL_NAMES)) == len(ENABLED_CLICKHOUSE_RAW_POOL_NAMES)
    assert (
        "clickhouse_raw_baostock__query_history_k_data_plus_daily_pool"
        in ENABLED_CLICKHOUSE_RAW_POOL_NAMES
    )
    assert "clickhouse_raw_eastmoney__balance_pool" in ENABLED_CLICKHOUSE_RAW_POOL_NAMES
