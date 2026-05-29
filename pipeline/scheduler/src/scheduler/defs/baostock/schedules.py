from __future__ import annotations

from scheduler.defs.automation.schedules import AssetJobSpec, build_asset_job
from scheduler.defs.baostock.assets import (
    baostock__query_history_k_data_plus_daily,
    baostock__query_stock_basic,
)
from scheduler.defs.market.schedules import build_trade_date_schedule

baostock__daily_job = build_asset_job(
    AssetJobSpec(
        name="baostock__daily_job",
        selection=[
            baostock__query_stock_basic,
            baostock__query_history_k_data_plus_daily,
        ],
    )
)

baostock__daily_schedule = build_trade_date_schedule(
    name="baostock__daily_schedule",
    job=baostock__daily_job,
    cron_schedule="35 17 * * *",
    partition_key_fn=lambda trade_date: str(trade_date.year),
    run_config_fn=lambda trade_date: {
        "ops": {
            baostock__query_history_k_data_plus_daily.node_def.name: {
                "config": {
                    "refresh_until_trade_date": trade_date.isoformat(),
                }
            }
        }
    },
    tags_fn=lambda trade_date: {
        "market.trade_date": trade_date.isoformat(),
        "market.year": str(trade_date.year),
    },
)
