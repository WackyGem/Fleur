from __future__ import annotations

from scheduler.defs.baostock.assets import (
    baostock__query_history_k_data_plus_daily,
    baostock__query_history_k_data_plus_daily_compacted,
    baostock__query_stock_basic,
)
from scheduler.defs.baostock.backfill_controller import (
    baostock__history_k_data_year_range_backfill_job,
)
from scheduler.defs.baostock.schedules import (
    baostock__daily_job,
    baostock__daily_schedule,
    baostock__query_history_k_data_plus_daily_compacted_job,
)
from scheduler.defs.source_bundle import SourceBundle

baostock_bundle = SourceBundle(
    name="baostock",
    assets=(
        baostock__query_stock_basic,
        baostock__query_history_k_data_plus_daily,
        baostock__query_history_k_data_plus_daily_compacted,
    ),
    jobs=(
        baostock__daily_job,
        baostock__query_history_k_data_plus_daily_compacted_job,
        baostock__history_k_data_year_range_backfill_job,
    ),
    schedules=(baostock__daily_schedule,),
)
