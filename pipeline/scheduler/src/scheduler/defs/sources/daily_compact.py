from __future__ import annotations

from datetime import date, datetime
from zoneinfo import ZoneInfo

import dagster as dg
import pyarrow as pa

from scheduler.defs.config.models import S3Config
from scheduler.defs.market.trade_calendar import read_trade_dates_from_s3
from scheduler.defs.storage.parquet_readers import (
    read_partitioned_parquet_tables_from_s3,
    trade_date_partition_keys_for_year,
)


def compact_daily_asset_by_year(
    context: dg.AssetExecutionContext,
    *,
    raw_asset_key: dg.AssetKey,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    partition_key = context.partition_key
    year = int(partition_key)
    refresh_until = refresh_until_for_year(year, run_tag(context, "market.trade_date"))
    s3_config = S3Config.from_env()
    trade_dates = read_trade_dates_from_s3(s3_config)
    requested_partition_keys = trade_date_partition_keys_for_year(
        year,
        trade_dates=trade_dates,
        refresh_until_trade_date=refresh_until,
    )
    read_result = read_partitioned_parquet_tables_from_s3(
        s3_config,
        raw_asset_key,
        partition_keys=requested_partition_keys,
        partition_key_name="trade_date",
    )
    if not read_result.tables:
        msg = (
            f"No non-empty daily partitions found for {raw_asset_key.to_user_string()} "
            f"in year partition {partition_key}"
        )
        raise RuntimeError(msg)

    table = pa.concat_tables(read_result.tables, promote_options="default")
    return dg.MaterializeResult(
        value={partition_key: table},
        metadata={
            "row_count": table.num_rows,
            "column_count": table.num_columns,
            "input_asset": raw_asset_key.to_user_string(),
            "requested_partition_count": len(requested_partition_keys),
            "read_partition_count": len(read_result.read_partition_keys),
            "missing_partition_count": len(read_result.missing_partition_keys),
            "empty_partition_count": len(read_result.empty_partition_keys),
            "read_partition_keys": dg.MetadataValue.json(read_result.read_partition_keys),
            "missing_partition_keys_sample": dg.MetadataValue.json(
                read_result.missing_partition_keys[:20]
            ),
            "empty_partition_keys_sample": dg.MetadataValue.json(
                read_result.empty_partition_keys[:20]
            ),
            "refresh_until_trade_date": refresh_until.isoformat()
            if refresh_until is not None
            else "full_year",
        },
    )


def refresh_until_for_year(year: int, tagged_trade_date: str | None) -> date | None:
    if tagged_trade_date is not None:
        trade_date = date.fromisoformat(tagged_trade_date)
        if trade_date.year != year:
            msg = f"market.trade_date {tagged_trade_date} is not in year partition {year}"
            raise ValueError(msg)
        return trade_date

    today = datetime.now(ZoneInfo("Asia/Shanghai")).date()
    if today.year == year:
        return today
    return None


def run_tag(context: dg.AssetExecutionContext, key: str) -> str | None:
    try:
        return context.run.tags.get(key)
    except Exception:
        return context.op_execution_context.run_tags.get(key)
