from __future__ import annotations

import dagster as dg

SINA_TRADE_CALENDAR_ASSET_KEY = dg.AssetKey("sina__trade_calendar")
BAOSTOCK_STOCK_BASIC_ASSET_KEY = dg.AssetKey("baostock__query_stock_basic")
BAOSTOCK_DAILY_K_ASSET_KEY = dg.AssetKey("baostock__query_history_k_data_plus_daily")
