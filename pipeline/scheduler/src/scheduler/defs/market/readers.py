from __future__ import annotations

from dataclasses import dataclass
from datetime import date
from typing import Protocol

import dagster as dg
import pyarrow as pa

from scheduler.defs.config.models import S3Config
from scheduler.defs.market.asset_keys import (
    BAOSTOCK_STOCK_BASIC_ASSET_KEY,
    SINA_TRADE_CALENDAR_ASSET_KEY,
)
from scheduler.defs.storage.dataset_service import (
    DatasetLocation,
    DatasetReader,
    S3DatasetService,
)


class TradeCalendarReader(Protocol):
    def read_trade_dates(self) -> set[date]: ...


class SecurityUniverseReader(Protocol):
    def read_stock_basic(self) -> pa.Table: ...


@dataclass(frozen=True)
class S3TradeCalendarReader:
    dataset_reader: DatasetReader
    bucket: str

    @classmethod
    def from_s3_config(cls, s3_config: S3Config) -> S3TradeCalendarReader:
        return cls(dataset_reader=S3DatasetService(s3_config=s3_config), bucket=s3_config.bucket)

    def read_trade_dates(self) -> set[date]:
        table = self.dataset_reader.read_latest_snapshot(
            _source_location(bucket=self.bucket, asset_key=SINA_TRADE_CALENDAR_ASSET_KEY)
        )
        return trade_dates_from_table(table)


@dataclass(frozen=True)
class S3SecurityUniverseReader:
    dataset_reader: DatasetReader
    bucket: str

    @classmethod
    def from_s3_config(cls, s3_config: S3Config) -> S3SecurityUniverseReader:
        return cls(dataset_reader=S3DatasetService(s3_config=s3_config), bucket=s3_config.bucket)

    def read_stock_basic(self) -> pa.Table:
        return self.dataset_reader.read_latest_snapshot(
            _source_location(bucket=self.bucket, asset_key=BAOSTOCK_STOCK_BASIC_ASSET_KEY)
        )


def trade_dates_from_table(table: pa.Table) -> set[date]:
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


def _source_location(*, bucket: str, asset_key: dg.AssetKey) -> DatasetLocation:
    return DatasetLocation(bucket=bucket, object_prefix="source", asset_key=asset_key)
