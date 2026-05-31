from __future__ import annotations

from datetime import date, datetime
from zoneinfo import ZoneInfo

import dagster as dg
import pyarrow as pa

from scheduler.defs.market.readers import S3TradeCalendarReader
from scheduler.defs.market.trade_calendar import trade_date_partition_keys_for_year
from scheduler.defs.resources.s3 import S3SettingsResource
from scheduler.defs.storage.dataset_service import DatasetLocation, S3DatasetService


def compact_daily_asset_by_year(
    context: dg.AssetExecutionContext,
    *,
    raw_asset_key: dg.AssetKey,
    s3_settings: S3SettingsResource,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    partition_key = context.partition_key
    year = int(partition_key)
    refresh_until = refresh_until_for_year(year, run_tag(context, "market.trade_date"))
    s3_config = s3_settings.config()
    dataset_service = S3DatasetService(s3_config=s3_config)
    trade_dates = S3TradeCalendarReader.from_s3_config(s3_config).read_trade_dates()
    requested_partition_keys = trade_date_partition_keys_for_year(
        year,
        trade_dates=trade_dates,
        refresh_until_trade_date=refresh_until,
    )
    read_result = dataset_service.read_partitioned(
        DatasetLocation(bucket=s3_config.bucket, object_prefix="source", asset_key=raw_asset_key),
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
