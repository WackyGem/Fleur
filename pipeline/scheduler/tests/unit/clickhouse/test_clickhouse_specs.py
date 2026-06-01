from __future__ import annotations

from pathlib import Path

from scheduler.defs.clickhouse.specs import (
    BAOSTOCK_DAILY_K_SPEC,
    ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
)

REPO_ROOT = Path(__file__).resolve().parents[5]


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


def test_enabled_spec_columns_match_baostock_data_dict() -> None:
    data_dict_columns = _read_clickhouse_columns_from_data_dict(
        REPO_ROOT
        / "docs"
        / "references"
        / "data_dict"
        / ("baostock__query_history_k_data_plus_daily.md")
    )

    assert {
        column.name: (column.pyarrow_type, column.clickhouse_type)
        for column in BAOSTOCK_DAILY_K_SPEC.columns
    } == data_dict_columns


def test_all_enabled_spec_columns_have_data_dict_clickhouse_types() -> None:
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


def _read_clickhouse_columns_from_data_dict(path: Path) -> dict[str, tuple[str, str]]:
    columns: dict[str, tuple[str, str]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped.startswith("|") or "✅" not in stripped:
            continue
        parts = [part.strip() for part in stripped.strip("|").split("|")]
        field_name = parts[1].split(".")[-1]
        pyarrow_type = parts[4]
        clickhouse_type = parts[5]
        columns[field_name] = (pyarrow_type, clickhouse_type)
    return columns
