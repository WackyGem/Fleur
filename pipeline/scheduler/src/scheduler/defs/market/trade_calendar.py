from __future__ import annotations

from datetime import date

from scheduler.defs.common.dates import is_trade_date
from scheduler.defs.config.models import S3Config
from scheduler.defs.storage.parquet_readers import read_sina_trade_calendar_dates_from_s3


def read_trade_dates_from_s3(config: S3Config) -> set[date]:
    return read_sina_trade_calendar_dates_from_s3(config)


def is_market_trade_date(candidate: date, trade_dates: set[date]) -> bool:
    return is_trade_date(candidate, trade_dates)
