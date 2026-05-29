import dagster as dg
import pyarrow as pa

from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.sources.daily_compact import compact_daily_asset_by_year
from scheduler.defs.sources.sina.trade_calendar import sina__trade_calendar
from scheduler.defs.sources.ths.limit_up_pool import ths__limit_up_pool

ths_limit_up_pool_compacted_year_partitions = dg.TimeWindowPartitionsDefinition(
    start="2025",
    fmt="%Y",
    cron_schedule="0 0 1 1 *",
    timezone="Asia/Shanghai",
    end_offset=1,
)


@dg.asset(
    name="ths__limit_up_pool_compacted",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    partitions_def=ths_limit_up_pool_compacted_year_partitions,
    deps=[
        dg.AssetDep(ths__limit_up_pool, partition_mapping=dg.TimeWindowPartitionMapping()),
        sina__trade_calendar,
    ],
    io_manager_key="s3_io_manager",
    backfill_policy=dg.BackfillPolicy.multi_run(max_partitions_per_run=1),
    automation_condition=dg.AutomationCondition.eager(),
    metadata={
        "storage_mode": "partitioned",
        "partition_key_name": "year",
        "partitions_def": "year_partitions",
        "input_partition_key_name": "trade_date",
        "input_asset": ths__limit_up_pool.key.to_user_string(),
    },
    tags={"source": "ths", "layer": "compacted", "storage": "s3"},
)
def ths__limit_up_pool_compacted(
    context: dg.AssetExecutionContext,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    """TongHuaShun limit-up pool daily parquet compacted by natural-year partition."""

    return compact_daily_asset_by_year(
        context,
        raw_asset_key=ths__limit_up_pool.key,
    )
