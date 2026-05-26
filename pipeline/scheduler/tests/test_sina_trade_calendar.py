from __future__ import annotations

import unittest

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
)
from scheduler.defs.io_managers.s3_io_manager import trade_calendar_rows_to_parquet_bytes


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
        rows = SinaCalendarParser().parse(SAMPLE_RESPONSE)

        self.assertGreater(len(rows), 1)
        self.assertEqual(rows[0], ["1990-12-19"])
        self.assertIn(["1992-05-04"], rows)

    def test_parser_returns_empty_rows_for_missing_datelist(self) -> None:
        rows = SinaCalendarParser().parse("var KLC_TD_SH='invalid';")

        self.assertEqual(rows, [])

    def test_parquet_writer_uses_trade_date_column(self) -> None:
        parquet_bytes = trade_calendar_rows_to_parquet_bytes(
            [["1990-12-19"], ["1990-12-20"]]
        )

        table = pq.read_table(pa.BufferReader(parquet_bytes))

        self.assertEqual(table.column_names, ["trade_date"])
        self.assertEqual(table.num_rows, 2)


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


if __name__ == "__main__":
    unittest.main()
