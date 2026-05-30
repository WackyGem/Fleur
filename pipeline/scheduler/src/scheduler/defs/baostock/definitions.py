from __future__ import annotations

from scheduler.defs.baostock.assets import (
    baostock__query_history_k_data_plus_daily,
    baostock__query_stock_basic,
)
from scheduler.defs.baostock.schedules import baostock__daily_job, baostock__daily_schedule
from scheduler.defs.source_bundle import SourceBundle

baostock_bundle = SourceBundle(
    name="baostock",
    assets=(baostock__query_stock_basic, baostock__query_history_k_data_plus_daily),
    jobs=(baostock__daily_job,),
    schedules=(baostock__daily_schedule,),
)
