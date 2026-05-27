from __future__ import annotations

import unittest
import zlib
from datetime import date
from tempfile import TemporaryDirectory

import dagster as dg
import pyarrow as pa
import pyarrow.fs as pafs
import pyarrow.parquet as pq

from scheduler.defs.baostock.protocol import decode_response, encode_request
from scheduler.defs.http_resources.client import HttpTextResponse
from scheduler.defs.http_resources.sina__trade_calendar import (
    SINA_TRADE_CALENDAR_URL,
    SinaCalendarParser,
    fetch_sina_trade_calendar,
    sina__trade_calendar,
    trade_calendar_dates_to_table,
)
from scheduler.defs.io_managers.s3_io_manager import S3IOManager
from scheduler.defs.util import (
    asset_key_to_parquet_object_key,
    filter_active_security_ranges,
    write_parquet_dataset,
)
from scheduler.defs.util import ExponentialBackoffPolicy


SAMPLE_RESPONSE = (
    'var datelist="LC/AAApNDXCw6mHbaPgkryxXv10eAJP1LW0SD39aT7+NV44Xba3PxCgTdrp5Bk'
    "YVAc11hWvg0c/19UAc7jNtHQyWBAu2xmGuZI1NVAc3FepphjnTBw1X4hmGu+ypVAcvFenpB"
    "XPqCc6F4ZmGueLFwbIN8QTDXPsCc1FepphjvOoCc8FepphjvcgFO3CP00wxXXWhrkUdZrI"
    "Jpw9X3ThrlEp6hlGc88Kcem0VeFpZM46VV4MrTC2KScKc811U4aLXUdlzINc9lTrwFW3T5"
    "2KPj0mDueVFuUR1RtiEoCXfdgFOOSGRXnUhrXWhb0kt6Rk2pU44JV4SrTyU9wSDHPwCnXd"
    "P1FuiUM44r7qwdKqcYrIZpw1DqgrlU5IrHRawxjrwBaqcbrIt9gr3UhDtOpyVNjEnCHPnC"
    '3royNWvi0gjHXBXYdRlLbFpdJFueSFcqkK30sSDO+68K46IVOwVkaBX/";var KLC_TD_SH=datelist;'
)


class FakeSinaHttpClient:
    def __init__(self, body: str) -> None:
        self.body = body
        self.requests: list[object] = []

    async def request_text(self, request: object) -> HttpTextResponse:
        self.requests.append(request)
        return HttpTextResponse(status=200, headers={}, body=self.body)


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


class FetchSinaTradeCalendarTest(unittest.IsolatedAsyncioTestCase):
    async def test_fetch_uses_shared_http_client_and_returns_response_text(self) -> None:
        http_client = FakeSinaHttpClient("calendar payload")

        result = await fetch_sina_trade_calendar(http_client)

        self.assertEqual(result, "calendar payload")
        self.assertEqual(len(http_client.requests), 1)
        self.assertEqual(http_client.requests[0].url, SINA_TRADE_CALENDAR_URL)

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
            storage_mode="latest_snapshot",
        )

        self.assertEqual(key, "raw/sina__trade_calendar/000000_0.parquet")

    def test_asset_key_to_parquet_object_key_supports_hive_partition_path(self) -> None:
        key = asset_key_to_parquet_object_key(
            dg.AssetKey(["baostock__query_history_k_data_plus_daily"]),
            partition_key="2026",
            partition_key_name="year",
        )

        self.assertEqual(
            key,
            "raw/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet",
        )

    def test_write_parquet_dataset_round_trips_unpartitioned_table(self) -> None:
        table = trade_calendar_dates_to_table(
            [date(1990, 12, 19), date(1990, 12, 20)]
        )

        with TemporaryDirectory() as tmpdir:
            paths = write_parquet_dataset(
                table,
                f"{tmpdir}/raw/sina__trade_calendar",
                pafs.LocalFileSystem(),
            )
            round_tripped = pq.read_table(f"{tmpdir}/raw/sina__trade_calendar/000000_0.parquet")

        self.assertEqual(paths, [f"{tmpdir}/raw/sina__trade_calendar/000000_0.parquet"])
        self.assertEqual(round_tripped.column_names, ["trade_date"])
        self.assertEqual(round_tripped.num_rows, 2)
        self.assertTrue(pa.types.is_date32(round_tripped.schema.field("trade_date").type))

    def test_write_parquet_dataset_round_trips_partitioned_table(self) -> None:
        table = pa.table(
            {
                "date": ["2026-05-25", "2026-05-26"],
                "code": ["sh.600000", "sh.600001"],
            }
        )

        with TemporaryDirectory() as tmpdir:
            paths = write_parquet_dataset(
                table,
                f"{tmpdir}/raw/baostock__query_history_k_data_plus_daily",
                pafs.LocalFileSystem(),
                partition_key="2026",
                partition_key_name="year",
            )
            parquet_file = pq.ParquetFile(
                f"{tmpdir}/raw/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet"
            )
            round_tripped = parquet_file.read()

        self.assertEqual(
            paths,
            [
                f"{tmpdir}/raw/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet"
            ],
        )
        self.assertEqual(round_tripped.column_names, ["date", "code"])
        self.assertEqual(parquet_file.schema_arrow.names, ["date", "code"])
        self.assertEqual(round_tripped.num_rows, 2)

    def test_write_parquet_dataset_rejects_empty_table_by_default(self) -> None:
        table = pa.table({"date": []}, schema=pa.schema([("date", pa.string())]))

        with TemporaryDirectory() as tmpdir:
            with self.assertRaises(ValueError):
                write_parquet_dataset(
                    table,
                    f"{tmpdir}/raw/empty",
                    pafs.LocalFileSystem(),
                )

    def test_write_parquet_dataset_round_trips_empty_table_when_allowed(self) -> None:
        table = pa.table({"date": []}, schema=pa.schema([("date", pa.string())]))

        with TemporaryDirectory() as tmpdir:
            paths = write_parquet_dataset(
                table,
                f"{tmpdir}/raw/empty",
                pafs.LocalFileSystem(),
                partition_key="2026",
                partition_key_name="year",
                allow_empty=True,
            )
            parquet_file = pq.ParquetFile(f"{tmpdir}/raw/empty/year=2026/000000_0.parquet")
            round_tripped = parquet_file.read()

        self.assertEqual(paths, [f"{tmpdir}/raw/empty/year=2026/000000_0.parquet"])
        self.assertEqual(round_tripped.column_names, ["date"])
        self.assertEqual(parquet_file.schema_arrow.names, ["date"])
        self.assertEqual(round_tripped.num_rows, 0)

    def test_s3_io_manager_empty_table_validation_requires_allow_empty(self) -> None:
        table = pa.table({"date": []}, schema=pa.schema([("date", pa.string())]))
        io_manager = S3IOManager()

        with self.assertRaises(ValueError):
            io_manager._validate_table(table)

        self.assertIs(io_manager._validate_table(table, allow_empty=True), table)


class BaoStockProtocolTest(unittest.TestCase):
    def test_encode_stock_basic_request_matches_reference_shape(self) -> None:
        request = encode_request(
            request_code="45",
            api_name="query_stock_basic",
            user_id="anonymous",
            params=["sh.601088", ""],
        )

        self.assertTrue(request.startswith(b"00.9.10\x0145\x010000000046"))
        self.assertTrue(request.endswith(b"\n"))
        self.assertIn(b"query_stock_basic\x01anonymous\x011\x0110000\x01sh.601088\x01", request)

    def test_decode_plain_stock_basic_response(self) -> None:
        body = (
            '0\x01success\x01query_stock_basic\x01anonymous\x011\x0110000\x01'
            '{"record":[["sh.600000","浦发银行","1999-11-10","","1","1"]]}\x01'
            'sh.600000\x01\x01code, code_name, ipoDate, outDate, type, status'
        )
        header = f"00.9.00\x0146\x01{len(body):010d}"
        head_body = f"{header}{body}"
        crc = zlib.crc32(head_body.encode("utf-8"))
        response = decode_response(f"{head_body}\x01{crc}<![CDATA[]]>\n".encode("utf-8"))

        self.assertEqual(response.error_code, "0")
        self.assertEqual(response.api_name, "query_stock_basic")
        self.assertEqual(response.records[0][0], "sh.600000")
        self.assertEqual(response.field_names, ["code", "code_name", "ipoDate", "outDate", "type", "status"])

    def test_decode_compressed_history_response_keeps_adjustflag_param(self) -> None:
        body = (
            '0\x01success\x01query_history_k_data_plus\x01anonymous\x011\x0110000\x01'
            '{"record":[["2026-05-25","sh.600000","8.9400","9.1200","8.9100","9.0800",'
            '"8.9600","92598961","839416293.2400","3","0.278000","1","1.339300","0"]]}\x01'
            'sh.600000\x01date,code,open,high,low,close,preclose,volume,amount,adjustflag,'
            'turn,tradestatus,pctChg,isST\x012026-05-25\x012026-05-25\x01d\x013'
        )
        compressed_body = zlib.compress(body.encode("utf-8"))
        header = f"00.9.00\x0196\x01{len(compressed_body):010d}".encode("utf-8")
        response = decode_response(header + compressed_body + b"<![CDATA[]]>\n")

        self.assertEqual(response.error_code, "0")
        self.assertEqual(response.api_name, "query_history_k_data_plus")
        self.assertEqual(response.params, ["sh.600000", "2026-05-25", "2026-05-25", "d", "3"])
        self.assertEqual(response.field_names[-1], "isST")
        self.assertEqual(response.records[0][0], "2026-05-25")


class BaoStockSecurityRangeTest(unittest.TestCase):
    def test_filter_active_security_ranges_intersects_requested_dates(self) -> None:
        stock_basic = pa.table(
            {
                "code": ["sh.600000", "sh.000001", "sh.510300", "sh.unknown", "bad"],
                "ipoDate": ["1999-11-10", "1991-07-15", "2012-05-28", "", "2020-01-01"],
                "outDate": ["", "", "", "", "2019-01-01"],
                "type": ["1", "2", "5", "1", "1"],
            }
        )

        ranges = filter_active_security_ranges(
            stock_basic,
            requested_start_date=date(2026, 1, 1),
            requested_end_date=date(2026, 5, 25),
        )

        self.assertEqual([item.code for item in ranges], ["sh.600000", "sh.000001", "sh.510300"])
        self.assertEqual(ranges[2].start_date, date(2026, 1, 5))


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
