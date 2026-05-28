from __future__ import annotations

import random
from collections.abc import Callable
from dataclasses import dataclass, field
from datetime import date
from typing import Literal
from urllib.parse import urlparse

import dagster as dg
import pyarrow as pa
import pyarrow.compute as pc
import pyarrow.dataset as ds
import pyarrow.fs as pafs
import pyarrow.parquet as pq

from scheduler.defs.config import S3Config

StorageMode = Literal["partitioned", "latest_snapshot"]
RandomUniform = Callable[[float, float], float]
DEFAULT_OBJECT_PREFIX = "raw"
SINA_TRADE_CALENDAR_ASSET_KEY = dg.AssetKey("sina__trade_calendar")
BAOSTOCK_STOCK_BASIC_ASSET_KEY = dg.AssetKey("baostock__query_stock_basic")
BAOSTOCK_DAILY_K_ASSET_KEY = dg.AssetKey("baostock__query_history_k_data_plus_daily")
BAOSTOCK_SECURITY_TYPE_DATA_START_DATES = {
    "1": date(1990, 12, 19),
    "2": date(2006, 1, 1),
    "5": date(2026, 1, 5),
}


@dataclass(frozen=True)
class ExponentialBackoffPolicy:
    """Configurable exponential backoff schedule for transient remote failures."""

    base_delay: float = 1.0
    factor: float = 2.0
    max_delay: float = 60.0
    jitter: bool = True
    jitter_ratio: float = 0.25
    random_uniform: RandomUniform = field(default=random.uniform, repr=False, compare=False)

    def delays(self, max_attempts: int) -> list[float]:
        if max_attempts < 1:
            msg = "max_attempts must be positive"
            raise ValueError(msg)

        delays = []
        for attempt in range(max_attempts - 1):
            delay = self.base_delay * (self.factor**attempt)
            delay = min(delay, self.max_delay)
            if self.jitter:
                delay = self.random_uniform(
                    delay * (1 - self.jitter_ratio),
                    delay * (1 + self.jitter_ratio),
                )
            delays.append(delay)
        return delays

    def metadata(self, max_attempts: int) -> dict[str, object]:
        return {
            "type": "exponential_backoff",
            "base_delay": self.base_delay,
            "factor": self.factor,
            "max_delay": self.max_delay,
            "jitter": self.jitter,
            "jitter_ratio": self.jitter_ratio,
            "max_attempts": max_attempts,
            "max_retries": max_attempts - 1,
            "nominal_delays": ExponentialBackoffPolicy(
                base_delay=self.base_delay,
                factor=self.factor,
                max_delay=self.max_delay,
                jitter=False,
            ).delays(max_attempts),
        }


DEFAULT_RETRY_POLICY = ExponentialBackoffPolicy(jitter=False)


@dataclass(frozen=True)
class SecurityDateRange:
    code: str
    security_type: str
    start_date: date
    end_date: date


def asset_key_to_parquet_object_key(
    asset_key: dg.AssetKey,
    object_prefix: str = DEFAULT_OBJECT_PREFIX,
    partition_key: str | None = None,
    partition_key_name: str | None = None,
    storage_mode: StorageMode = "partitioned",
) -> str:
    asset_path = "/".join(asset_key.path)
    stripped_prefix = object_prefix.strip("/")
    path_parts = [part for part in (stripped_prefix, asset_path) if part]

    if storage_mode == "partitioned" and partition_key is not None:
        if partition_key_name is None:
            msg = "partition_key_name is required when partition_key is provided"
            raise ValueError(msg)
        path_parts.append(f"{partition_key_name}={partition_key}")
    elif storage_mode not in {"partitioned", "latest_snapshot"}:
        msg = f"Unsupported storage mode: {storage_mode}"
        raise ValueError(msg)

    path_parts.append("000000_0.parquet")
    return "/".join(path_parts)


def build_s3_filesystem(config: S3Config) -> pafs.S3FileSystem:
    endpoint = config.endpoint
    scheme = None
    if "://" in endpoint:
        parsed_endpoint = urlparse(endpoint)
        scheme = parsed_endpoint.scheme
        endpoint = parsed_endpoint.netloc

    return pafs.S3FileSystem(
        access_key=config.access_key,
        secret_key=config.secret_key,
        endpoint_override=endpoint,
        scheme=scheme,
        region=config.region_name,
        allow_bucket_creation=True,
    )


def write_bytes_to_filesystem(
    filesystem: pafs.FileSystem,
    path: str,
    data: bytes,
) -> None:
    with filesystem.open_output_stream(path) as sink:
        sink.write(data)


def read_bytes_from_filesystem(
    filesystem: pafs.FileSystem,
    path: str,
) -> bytes:
    with filesystem.open_input_file(path) as source:
        return source.read()


def write_parquet_dataset(
    table: pa.Table,
    base_dir: str,
    filesystem: pafs.FileSystem,
    *,
    partition_key: str | None = None,
    partition_key_name: str | None = None,
    allow_empty: bool = False,
) -> list[str]:
    if table.num_rows == 0 and not allow_empty:
        msg = "Refusing to write an empty pyarrow.Table"
        raise ValueError(msg)

    if partition_key_name is not None or partition_key is not None:
        if partition_key_name is None or partition_key is None:
            msg = "partition_key and partition_key_name must be provided together"
            raise ValueError(msg)
        base_dir = f"{base_dir.rstrip('/')}/{partition_key_name}={partition_key}"

    if table.num_rows == 0:
        filesystem.delete_dir_contents(base_dir, missing_dir_ok=True)
        filesystem.create_dir(base_dir, recursive=True)
        path = f"{base_dir}/000000_0.parquet"
        with filesystem.open_output_stream(path) as sink:
            pq.write_table(table, sink, compression="zstd")
        return [path]

    written_paths: list[str] = []

    def visit_file(written_file: ds.WrittenFile) -> None:
        written_paths.append(written_file.path)

    ds.write_dataset(
        table,
        base_dir=base_dir,
        filesystem=filesystem,
        format="parquet",
        basename_template="000000_{i}.parquet",
        existing_data_behavior="delete_matching",
        use_threads=True,
        max_rows_per_file=max(table.num_rows, 1),
        max_rows_per_group=max(table.num_rows, 1),
        file_visitor=visit_file,
    )

    extra_files = [
        path
        for path in written_paths
        if not path.endswith("/000000_0.parquet") and path != f"{base_dir}/000000_0.parquet"
    ]
    if extra_files:
        msg = f"PyArrow wrote unexpected parquet files: {extra_files}"
        raise RuntimeError(msg)

    return sorted(written_paths)


def read_parquet_table_from_s3(
    config: S3Config,
    asset_key: dg.AssetKey,
    *,
    partition_key: str | None = None,
    partition_key_name: str | None = None,
    storage_mode: StorageMode = "partitioned",
) -> pa.Table:
    object_key = asset_key_to_parquet_object_key(
        asset_key,
        partition_key=partition_key,
        partition_key_name=partition_key_name,
        storage_mode=storage_mode,
    )
    path = f"{config.bucket}/{object_key}"
    filesystem = build_s3_filesystem(config)
    try:
        with filesystem.open_input_file(path) as source:
            return pq.read_table(source)
    except Exception as error:
        msg = f"Failed to read parquet table from s3://{path}"
        raise RuntimeError(msg) from error


def read_sina_trade_calendar_dates_from_s3(config: S3Config) -> set[date]:
    table = read_parquet_table_from_s3(
        config,
        SINA_TRADE_CALENDAR_ASSET_KEY,
        storage_mode="latest_snapshot",
    )
    if "trade_date" not in table.column_names:
        msg = "Sina trade calendar parquet is missing the trade_date column"
        raise ValueError(msg)
    if table.num_rows == 0:
        msg = "Sina trade calendar parquet is empty"
        raise ValueError(msg)

    values = table.column("trade_date").to_pylist()
    trade_dates = {value for value in values if isinstance(value, date)}
    if not trade_dates:
        msg = "Sina trade calendar parquet contains no valid trade_date values"
        raise ValueError(msg)
    return trade_dates


def read_baostock_stock_basic_from_s3(config: S3Config) -> pa.Table:
    return read_parquet_table_from_s3(
        config,
        BAOSTOCK_STOCK_BASIC_ASSET_KEY,
        storage_mode="latest_snapshot",
    )


def is_trade_date(candidate: date, trade_dates: set[date]) -> bool:
    return candidate in trade_dates


def filter_active_security_ranges(
    stock_basic: pa.Table,
    requested_start_date: date,
    requested_end_date: date,
    allowed_security_types: frozenset[str] = frozenset({"1", "2", "5"}),
) -> list[SecurityDateRange]:
    if requested_start_date > requested_end_date:
        msg = "requested_start_date must be less than or equal to requested_end_date"
        raise ValueError(msg)

    required_columns = {"code", "ipoDate", "outDate", "type"}
    missing_columns = required_columns - set(stock_basic.column_names)
    if missing_columns:
        msg = f"stock_basic is missing required columns: {sorted(missing_columns)}"
        raise ValueError(msg)

    selected = stock_basic.select(["code", "ipoDate", "outDate", "type"])
    ranges: list[SecurityDateRange] = []
    for row in selected.to_pylist():
        code = _clean_optional_string(row["code"])
        security_type = _clean_optional_string(row["type"])
        if code is None or security_type is None:
            continue
        if security_type not in allowed_security_types:
            continue
        if security_type not in BAOSTOCK_SECURITY_TYPE_DATA_START_DATES:
            continue

        ipo_date = _parse_required_date(row["ipoDate"])
        if ipo_date is None:
            continue
        out_date = _parse_optional_date(row["outDate"])
        if out_date is not None and out_date < ipo_date:
            continue

        security_start = max(
            ipo_date,
            BAOSTOCK_SECURITY_TYPE_DATA_START_DATES[security_type],
        )
        effective_start = max(requested_start_date, security_start)
        effective_end = requested_end_date
        if out_date is not None:
            effective_end = min(effective_end, out_date)
        if effective_start > effective_end:
            continue

        ranges.append(
            SecurityDateRange(
                code=code,
                security_type=security_type,
                start_date=effective_start,
                end_date=effective_end,
            )
        )

    return ranges


def _clean_optional_string(value: object) -> str | None:
    if value is None:
        return None
    cleaned = str(value).strip()
    if not cleaned:
        return None
    return cleaned


def _parse_required_date(value: object) -> date | None:
    cleaned = _clean_optional_string(value)
    if cleaned is None:
        return None
    try:
        return date.fromisoformat(cleaned)
    except ValueError:
        return None


def _parse_optional_date(value: object) -> date | None:
    cleaned = _clean_optional_string(value)
    if cleaned is None:
        return None
    try:
        return date.fromisoformat(cleaned)
    except ValueError:
        return None


def table_row_count_by_string_column(table: pa.Table, column_name: str) -> dict[str, int]:
    if column_name not in table.column_names:
        msg = f"Column {column_name!r} is missing from table"
        raise ValueError(msg)

    counts = pc.value_counts(table[column_name]).to_pylist()
    return {row["values"]: row["counts"] for row in counts}
