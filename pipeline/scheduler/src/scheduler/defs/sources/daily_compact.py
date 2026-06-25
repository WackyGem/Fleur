from __future__ import annotations

from collections.abc import Sequence
from datetime import date, datetime
from zoneinfo import ZoneInfo

import dagster as dg
import pyarrow as pa

from scheduler.defs.contract_schemas import PARQUET_SCHEMAS
from scheduler.defs.market.readers import S3TradeCalendarReader
from scheduler.defs.market.trade_calendar import trade_date_partition_keys_for_year
from scheduler.defs.resources.s3 import S3SettingsResource
from scheduler.defs.storage.dataset_service import DatasetLocation, S3DatasetService


def compact_daily_asset_by_year(
    context: dg.AssetExecutionContext,
    *,
    raw_asset_key: dg.AssetKey,
    output_dataset: str,
    s3_settings: S3SettingsResource,
    require_complete_partitions: bool = False,
    unique_key_columns: Sequence[str] = (),
    sort_key_columns: Sequence[str] = (),
    refresh_until_trade_date: date | None = None,
    use_latest_existing_partition_as_refresh_until: bool = False,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    partition_key = context.partition_key
    year = int(partition_key)
    tagged_trade_date = run_tag(context, "market.trade_date")
    refresh_until_source = "explicit_config" if refresh_until_trade_date is not None else "default"
    refresh_until = refresh_until_trade_date or refresh_until_for_year(year, tagged_trade_date)
    if refresh_until_trade_date is None and tagged_trade_date is not None:
        refresh_until_source = "run_tag"
    if refresh_until is not None and refresh_until.year != year:
        msg = (
            f"refresh_until_trade_date {refresh_until.isoformat()} is not in year partition {year}"
        )
        raise ValueError(msg)
    s3_config = s3_settings.config()
    dataset_service = S3DatasetService(s3_config=s3_config)
    trade_dates = S3TradeCalendarReader.from_s3_config(s3_config).read_trade_dates()
    location = DatasetLocation(
        bucket=s3_config.bucket, object_prefix="source", asset_key=raw_asset_key
    )
    if (
        refresh_until_trade_date is None
        and tagged_trade_date is None
        and use_latest_existing_partition_as_refresh_until
        and refresh_until is not None
    ):
        candidate_partition_keys = trade_date_partition_keys_for_year(
            year,
            trade_dates=trade_dates,
            refresh_until_trade_date=refresh_until,
        )
        existing_partition_keys = dataset_service.existing_partition_keys(
            location,
            partition_keys=candidate_partition_keys,
            partition_key_name="trade_date",
        )
        if existing_partition_keys:
            refresh_until = date.fromisoformat(existing_partition_keys[-1])
            refresh_until_source = "latest_existing_partition"

    requested_partition_keys = trade_date_partition_keys_for_year(
        year,
        trade_dates=trade_dates,
        refresh_until_trade_date=refresh_until,
    )
    read_result = dataset_service.read_partitioned(
        location,
        partition_keys=requested_partition_keys,
        partition_key_name="trade_date",
    )
    if not read_result.tables:
        msg = (
            f"No non-empty daily partitions found for {raw_asset_key.to_user_string()} "
            f"in year partition {partition_key}"
        )
        raise RuntimeError(msg)
    if require_complete_partitions and read_result.missing_partition_keys:
        msg = (
            f"Compacted dataset {output_dataset} is missing required daily partitions "
            f"for year {partition_key}: {read_result.missing_partition_keys[:20]}"
        )
        raise RuntimeError(msg)

    table = _compact_table_for_output_schema(
        pa.concat_tables(read_result.tables, promote_options="default"),
        dataset=output_dataset,
    )
    duplicate_key_count = _duplicate_key_count(table, unique_key_columns)
    if duplicate_key_count:
        msg = (
            f"Compacted dataset {output_dataset} contains {duplicate_key_count} "
            f"duplicate keys for columns {list(unique_key_columns)}"
        )
        raise RuntimeError(msg)
    table = _sort_table(table, sort_key_columns)
    return dg.MaterializeResult(
        value={partition_key: table},
        metadata={
            "row_count": table.num_rows,
            "column_count": table.num_columns,
            "input_asset": raw_asset_key.to_user_string(),
            "expected_partition_count": len(requested_partition_keys),
            "requested_partition_count": len(requested_partition_keys),
            "read_partition_count": len(read_result.read_partition_keys),
            "missing_partition_count": len(read_result.missing_partition_keys),
            "empty_partition_count": len(read_result.empty_partition_keys),
            "duplicate_key_count": duplicate_key_count,
            "completeness_required": require_complete_partitions,
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
            "refresh_until_source": refresh_until_source,
        },
    )


def _duplicate_key_count(table: pa.Table, key_columns: Sequence[str]) -> int:
    if not key_columns:
        return 0
    missing_columns = [column for column in key_columns if column not in table.column_names]
    if missing_columns:
        msg = f"Cannot validate duplicate keys; missing columns: {missing_columns}"
        raise RuntimeError(msg)
    seen: set[tuple[object, ...]] = set()
    duplicate_count = 0
    for row in table.select(list(key_columns)).to_pylist():
        key = tuple(row[column] for column in key_columns)
        if key in seen:
            duplicate_count += 1
        else:
            seen.add(key)
    return duplicate_count


def _sort_table(table: pa.Table, sort_key_columns: Sequence[str]) -> pa.Table:
    if not sort_key_columns:
        return table
    missing_columns = [column for column in sort_key_columns if column not in table.column_names]
    if missing_columns:
        msg = f"Cannot sort compacted table; missing columns: {missing_columns}"
        raise RuntimeError(msg)
    return table.sort_by([(column, "ascending") for column in sort_key_columns])


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


def _compact_table_for_output_schema(table: pa.Table, *, dataset: str) -> pa.Table:
    expected_schema = PARQUET_SCHEMAS[dataset]
    table = _normalize_compacted_table(table, dataset=dataset)
    if table.schema == expected_schema:
        return table
    missing_fields = [
        field_name for field_name in expected_schema.names if field_name not in table.schema.names
    ]
    if missing_fields:
        msg = f"Compacted dataset {dataset} is missing fields: {missing_fields}"
        raise RuntimeError(msg)
    return table.select(expected_schema.names).cast(expected_schema)


def _normalize_compacted_table(table: pa.Table, *, dataset: str) -> pa.Table:
    if dataset != "jiuyan__action_field_compacted":
        return table
    if "reason" not in table.schema.names:
        return table

    normalized_reason = pa.array(
        [None if value == "" else value for value in table["reason"].to_pylist()],
        type=pa.string(),
    )
    return table.set_column(
        table.schema.get_field_index("reason"),
        "reason",
        normalized_reason,
    )
