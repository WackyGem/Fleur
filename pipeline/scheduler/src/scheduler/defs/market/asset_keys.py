from __future__ import annotations

import dagster as dg

SOURCE_ASSET_KEY_PREFIX = "source"

SINA_TRADE_CALENDAR_ASSET_KEY = dg.AssetKey([SOURCE_ASSET_KEY_PREFIX, "sina__trade_calendar"])
BAOSTOCK_STOCK_BASIC_ASSET_KEY = dg.AssetKey(
    [SOURCE_ASSET_KEY_PREFIX, "baostock__query_stock_basic"]
)
BAOSTOCK_DAILY_K_ASSET_KEY = dg.AssetKey(
    [SOURCE_ASSET_KEY_PREFIX, "baostock__query_history_k_data_plus_daily"]
)
BAOSTOCK_DAILY_K_COMPACTED_ASSET_KEY = dg.AssetKey(
    [SOURCE_ASSET_KEY_PREFIX, "baostock__query_history_k_data_plus_daily_compacted"]
)
