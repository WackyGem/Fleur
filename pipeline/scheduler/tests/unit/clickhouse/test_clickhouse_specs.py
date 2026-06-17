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

    assert len(raw_asset_keys) == 17
    assert "clickhouse/raw/baostock__query_history_k_data_plus_daily" in raw_asset_keys
    assert "clickhouse/raw/chinabond__government_bond" in raw_asset_keys
    assert "clickhouse/raw/sina__trade_calendar" in raw_asset_keys
    assert "clickhouse/raw/jiuyan__industry_ocr_snapshot" in raw_asset_keys
    assert "clickhouse/raw/jiuyan__action_field_compacted" in raw_asset_keys
    assert "clickhouse/raw/ths__limit_up_pool_compacted" in raw_asset_keys
    assert "clickhouse/raw/eastmoney__balance" in raw_asset_keys
    assert "clickhouse/raw/eastmoney__freeholders" in raw_asset_keys


def test_baostock_spec_uses_year_partition_and_query_driven_order_by() -> None:
    spec = BAOSTOCK_DAILY_K_SPEC

    assert spec.source_asset_key.to_user_string() == (
        "source/baostock__query_history_k_data_plus_daily"
    )
    assert spec.storage_mode == "partitioned"
    assert spec.source_partition_key_name == "year"
    assert spec.partition_strategy == "year"
    assert spec.order_by == ("date", "code")
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
        field.name: (
            parquet_fields[field.from_].type,
            f"Nullable({field.type})" if field.nullable else field.type,
        )
        for field in contract.clickhouse_raw.fields
    }


def test_affected_nullable_date_fields_generate_nullable_clickhouse_types() -> None:
    expected = {
        "baostock__query_stock_basic": {
            "outDate": "Nullable(Date)",
        },
        "eastmoney__balance": {
            "UPDATE_DATE": "Nullable(Date)",
        },
        "eastmoney__cashflow_sq": {
            "UPDATE_DATE": "Nullable(Date)",
        },
        "eastmoney__cashflow_ytd": {
            "UPDATE_DATE": "Nullable(Date)",
        },
        "eastmoney__income_sq": {
            "UPDATE_DATE": "Nullable(Date)",
        },
        "eastmoney__income_ytd": {
            "UPDATE_DATE": "Nullable(Date)",
        },
        "eastmoney__freeholders": {
            "CHANGE_RATIO": "Nullable(Float64)",
        },
        "eastmoney__dividend_main": {
            "EQUITY_RECORD_DATE": "Nullable(Date)",
            "EX_DIVIDEND_DATE": "Nullable(Date)",
            "PAY_CASH_DATE": "Nullable(Date)",
            "GMDECISION_NOTICE_DATE": "Nullable(Date)",
            "DAT_YAGGR": "Nullable(Date)",
            "REPORT_TIME": "Nullable(Date)",
            "LAST_TRADE_DATE": "Nullable(Date)",
        },
        "jiuyan__action_field_compacted": {
            "delete_time": "Nullable(DateTime)",
            "create_time": "DateTime",
            "update_time": "Nullable(DateTime)",
        },
        "jiuyan__industry_list": {
            "delete_time": "Nullable(DateTime)",
            "create_time": "DateTime",
            "update_time": "Nullable(DateTime)",
        },
        "ths__limit_up_pool_compacted": {
            "first_limit_up_time": "DateTime('UTC')",
            "last_limit_up_time": "DateTime('UTC')",
        },
    }

    spec_by_dataset = {spec.contract_dataset: spec for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS}

    for dataset, expected_columns in expected.items():
        column_types = {
            column.name: column.clickhouse_type for column in spec_by_dataset[dataset].columns
        }
        assert {column: column_types[column] for column in expected_columns} == expected_columns


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
