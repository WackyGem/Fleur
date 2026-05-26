from __future__ import annotations

import unittest
from datetime import date

import dagster as dg
import pyarrow as pa
import pyarrow.parquet as pq
import requests

from scheduler.defs.http_resources.sina__trade_calendar import (
    CHROME_USER_AGENT,
    REQUEST_TIMEOUT_SECONDS,
    SINA_TRADE_CALENDAR_URL,
    ExponentialBackoffPolicy,
    SinaCalendarParser,
    fetch_sina_trade_calendar,
    sina__trade_calendar,
    trade_calendar_dates_to_table,
)
from scheduler.defs.io_managers.s3_io_manager import (
    asset_key_to_parquet_object_key,
    table_to_parquet_bytes,
)


SAMPLE_RESPONSE = (
    'var datelist="LC/AAApNDXCw6mHbaPgkryxXv10eAJP1LW0SD39aT7+NV44Xba3PxCgTdrp5Bk'
    "YVAc11hWvg0c/19UAc7jNtHQyWBAu2xmGuZI1NVAc3FepphjnTBw1X4hmGu+ypVAcvFenpB"
    "XPqCc6F4ZmGueLFwbIN8QTDXPsCc1FepphjvOoCc8FepphjvcgFO3CP00wxXXWhrkUdZrI"
    "Jpw9X3ThrlEp6hlGc88Kcem0VeFpZM46VV4MrTC2KScKc811U4aLXUdlzINc9lTrwFW3T5"
    "2KPj0mDueVFuUR1RtiEoCXfdgFOOSGRXnUhrXWhb0kt6Rk2pU44JV4SrTyU9wSDHPwCnXd"
    "P1FuiUM44r7qwdKqcYrIZpw1DqgrlU5IrHRawxjrwBaqcbrIt9gr3UhDtOpyVNjEnCHPnC"
    '3royNWvi0gjHXBXYdRlLbFpdJFueSFcqkK30sSDO+68K46IVOwVkaBX/";var KLC_TD_SH=datelist;'
)


class FakeResponse:
    def __init__(self, text: str, status_code: int = 200) -> None:
        self.text = text
        self.status_code = status_code

    def raise_for_status(self) -> None:
        if self.status_code >= 400:
            raise requests.HTTPError(f"HTTP {self.status_code}")


class SinaCalendarParserTest(unittest.TestCase):
    def test_parser_decodes_sample_and_adds_known_missing_date(self) -> None:
        dates = SinaCalendarParser().parse(SAMPLE_RESPONSE)

        self.assertGreater(len(dates), 1)
        self.assertEqual(dates[0], date(1990, 12, 19))
        self.assertIn(date(1992, 5, 4), dates)

    def test_parser_returns_empty_dates_for_missing_datelist(self) -> None:
        dates = SinaCalendarParser().parse("var KLC_TD_SH='invalid';")

        self.assertEqual(dates, [])

    def test_trade_calendar_table_uses_trade_date_column(self) -> None:
        table = trade_calendar_dates_to_table(
            [date(1990, 12, 19), date(1990, 12, 20)]
        )

        self.assertEqual(table.column_names, ["trade_date"])
        self.assertEqual(table.num_rows, 2)
        self.assertTrue(pa.types.is_date32(table.schema.field("trade_date").type))


class FetchSinaTradeCalendarTest(unittest.TestCase):
    def test_fetch_retries_transient_errors_then_returns_response_text(self) -> None:
        sleep_calls: list[float] = []
        request_calls: list[dict[str, object]] = []

        def fake_get(url: str, **kwargs: object) -> FakeResponse:
            request_calls.append({"url": url, **kwargs})
            if len(request_calls) < 3:
                raise requests.Timeout("temporary timeout")
            return FakeResponse("calendar payload")

        result = fetch_sina_trade_calendar(
            request_get=fake_get,
            sleep=sleep_calls.append,
            retry_policy=ExponentialBackoffPolicy(jitter=False),
        )

        self.assertEqual(result, "calendar payload")
        self.assertEqual(sleep_calls, [1, 2])
        self.assertEqual(len(request_calls), 3)
        self.assertEqual(request_calls[0]["url"], SINA_TRADE_CALENDAR_URL)
        self.assertEqual(request_calls[0]["timeout"], REQUEST_TIMEOUT_SECONDS)
        self.assertEqual(
            request_calls[0]["headers"],
            {"User-Agent": CHROME_USER_AGENT, "Accept": "text/plain,*/*"},
        )

    def test_fetch_exhausts_retry_policy_before_raising(self) -> None:
        sleep_calls: list[float] = []
        request_count = 0

        def fake_get(url: str, **kwargs: object) -> FakeResponse:
            nonlocal request_count
            request_count += 1
            raise requests.ConnectionError("connection refused")

        with self.assertRaises(RuntimeError):
            fetch_sina_trade_calendar(
                request_get=fake_get,
                sleep=sleep_calls.append,
                retry_policy=ExponentialBackoffPolicy(jitter=False),
            )

        self.assertEqual(request_count, 4)
        self.assertEqual(sleep_calls, [1, 2, 4])

    def test_backoff_policy_calculates_delays_from_parameters(self) -> None:
        policy = ExponentialBackoffPolicy(
            base_delay=0.5,
            factor=3.0,
            max_delay=4.0,
            jitter=False,
        )

        self.assertEqual(policy.delays(max_attempts=5), [0.5, 1.5, 4.0, 4.0])

    def test_backoff_policy_applies_jitter(self) -> None:
        policy = ExponentialBackoffPolicy(
            base_delay=1.0,
            factor=2.0,
            max_delay=60.0,
            jitter=True,
            random_uniform=lambda lower, upper: upper,
        )

        self.assertEqual(policy.delays(max_attempts=3), [1.25, 2.5])


class S3TableIOTest(unittest.TestCase):
    def test_asset_key_to_parquet_object_key_uses_raw_prefix_by_default(self) -> None:
        key = asset_key_to_parquet_object_key(
            dg.AssetKey(["sina__trade_calendar"]),
            object_prefix="raw",
        )

        self.assertEqual(key, "raw/sina__trade_calendar/000000_0.parquet")

    def test_table_to_parquet_bytes_round_trips_pyarrow_table(self) -> None:
        table = trade_calendar_dates_to_table(
            [date(1990, 12, 19), date(1990, 12, 20)]
        )

        parquet_bytes = table_to_parquet_bytes(table)
        round_tripped = pq.read_table(pa.BufferReader(parquet_bytes))

        self.assertEqual(round_tripped.column_names, ["trade_date"])
        self.assertEqual(round_tripped.num_rows, 2)
        self.assertTrue(pa.types.is_date32(round_tripped.schema.field("trade_date").type))


class SinaTradeCalendarAutomationTest(unittest.TestCase):
    def test_automation_requests_never_materialized_calendar_once(self) -> None:
        instance = dg.DagsterInstance.ephemeral()
        defs = dg.Definitions(
            assets=[sina__trade_calendar],
            resources={"s3_io_manager": dg.InMemoryIOManager()},
        )

        result = dg.evaluate_automation_conditions(
            defs=defs,
            instance=instance,
            asset_selection=dg.AssetSelection.assets(sina__trade_calendar),
        )

        self.assertEqual(result.total_requested, 1)

        next_result = dg.evaluate_automation_conditions(
            defs=defs,
            instance=instance,
            asset_selection=dg.AssetSelection.assets(sina__trade_calendar),
            cursor=result.cursor,
        )

        self.assertEqual(next_result.total_requested, 0)


if __name__ == "__main__":
    unittest.main()
