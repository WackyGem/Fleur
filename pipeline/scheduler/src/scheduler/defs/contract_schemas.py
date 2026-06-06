from __future__ import annotations

import pyarrow as pa
from fleur_contracts.adapters.parquet import (
    parquet_schema_contracts,
    pyarrow_schema_from_contract,
)
from fleur_contracts.loader import load_registry

_DATASET_CONTRACTS = tuple(load_registry().datasets)
_PARQUET_SCHEMA_CONTRACTS = parquet_schema_contracts(_DATASET_CONTRACTS)

PARQUET_SCHEMAS: dict[str, pa.Schema] = {
    contract.dataset: pyarrow_schema_from_contract(contract)
    for contract in _PARQUET_SCHEMA_CONTRACTS
}

PARQUET_SCHEMA_HASHES: dict[str, str] = {
    contract.dataset: contract.parquet_schema_hash for contract in _PARQUET_SCHEMA_CONTRACTS
}

CONTRACT_SCHEMA_HASHES: dict[str, str] = {
    contract.dataset: contract.schema_hash for contract in _PARQUET_SCHEMA_CONTRACTS
}

SOURCE_SCHEMA_HASHES: dict[str, str] = {
    contract.dataset: contract.source_schema_hash for contract in _PARQUET_SCHEMA_CONTRACTS
}

SOURCE_FIELD_NAMES: dict[str, tuple[str, ...]] = {
    contract.dataset: tuple(field.name for field in contract.source.fields)
    for contract in _DATASET_CONTRACTS
}

CONTRACT_VERSIONS: dict[str, int] = {
    contract.dataset: contract.version for contract in _PARQUET_SCHEMA_CONTRACTS
}

SOURCE_ASSET_KEYS: dict[str, tuple[str, ...]] = {
    contract.dataset: contract.source_asset_key for contract in _PARQUET_SCHEMA_CONTRACTS
}

STORAGE_MODES: dict[str, str] = {
    contract.dataset: contract.storage_mode for contract in _PARQUET_SCHEMA_CONTRACTS
}

PARTITION_KEY_NAMES: dict[str, str | None] = {
    contract.dataset: contract.partition_key_name for contract in _PARQUET_SCHEMA_CONTRACTS
}

BAOSTOCK_QUERY_HISTORY_K_DATA_PLUS_DAILY_SCHEMA = PARQUET_SCHEMAS[
    "baostock__query_history_k_data_plus_daily"
]
BAOSTOCK_QUERY_STOCK_BASIC_SCHEMA = PARQUET_SCHEMAS["baostock__query_stock_basic"]
EASTMONEY_FREEHOLDERS_SCHEMA = PARQUET_SCHEMAS["eastmoney__freeholders"]
JIUYAN_ACTION_FIELD_SCHEMA = PARQUET_SCHEMAS["jiuyan__action_field"]
JIUYAN_INDUSTRY_LIST_SCHEMA = PARQUET_SCHEMAS["jiuyan__industry_list"]
JIUYAN_INDUSTRY_OCR_SCHEMA = PARQUET_SCHEMAS["jiuyan__industry_ocr"]
JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA = PARQUET_SCHEMAS["jiuyan__industry_ocr_snapshot"]
SINA_TRADE_CALENDAR_SCHEMA = PARQUET_SCHEMAS["sina__trade_calendar"]
THS_LIMIT_UP_POOL_SCHEMA = PARQUET_SCHEMAS["ths__limit_up_pool"]
