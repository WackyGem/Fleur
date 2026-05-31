from __future__ import annotations

from datetime import date

from scheduler.defs.common.dates import is_trade_date
from scheduler.defs.config.models import S3Config
from scheduler.defs.market.readers import S3TradeCalendarReader


def read_trade_dates_from_s3(config: S3Config) -> set[date]:
    return S3TradeCalendarReader.from_s3_config(config).read_trade_dates()


def is_market_trade_date(candidate: date, trade_dates: set[date]) -> bool:
    return is_trade_date(candidate, trade_dates)


def trade_date_partition_keys_for_year(
    year: int,
    *,
    trade_dates: set[date],
    refresh_until_trade_date: date | None = None,
) -> list[str]:
    start_date = date(year, 1, 1)
    end_date = date(year, 12, 31)
    if refresh_until_trade_date is not None:
        end_date = min(end_date, refresh_until_trade_date)
    return [
        trade_date.isoformat()
        for trade_date in sorted(trade_dates)
        if start_date <= trade_date <= end_date
    ]
