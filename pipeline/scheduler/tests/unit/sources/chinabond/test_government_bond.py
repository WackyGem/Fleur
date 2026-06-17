from __future__ import annotations

import unittest
from datetime import date

import pyarrow as pa
from scheduler.defs.contract_schemas import PARQUET_SCHEMAS
from scheduler.defs.http.client import HttpFetchStats
from scheduler.defs.sources.chinabond.client import ChinabondRequestError
from scheduler.defs.sources.chinabond.services import (
    CHINABOND_DATASET,
    build_year_ranges,
    chinabond_rows_to_table,
    normalize_chinabond_row,
    parse_chinabond_rows,
)


def sample_row(**overrides: object) -> dict[str, object]:
    row: dict[str, object] = {
        "workTime": "2026-06-16",
        "threeMonth": "1.09",
        "sixMonth": "1.13",
        "oneYear": "1.20",
        "twoYear": "1.28",
        "threeYear": "1.31",
        "fiveYear": "1.46",
        "sevenYear": "1.58",
        "tenYear": "1.73",
        "fifteenYear": None,
        "twentyYear": " ",
        "thirtyYear": "2.22",
        "qxmc": "中债国债收益率曲线",
    }
    row.update(overrides)
    return row


class ChinabondYearRangeTest(unittest.TestCase):
    def test_2006_partition_starts_at_first_available_data_date(self) -> None:
        ranges = build_year_ranges(["2006"])

        self.assertEqual(ranges["2006"], (date(2006, 3, 1), date(2006, 12, 31)))

    def test_regular_partition_uses_natural_year(self) -> None:
        ranges = build_year_ranges(["2025"])

        self.assertEqual(ranges["2025"], (date(2025, 1, 1), date(2025, 12, 31)))

    def test_refresh_until_date_limits_single_partition(self) -> None:
        ranges = build_year_ranges(["2026"], refresh_until_date="2026-06-16")

        self.assertEqual(ranges["2026"], (date(2026, 1, 1), date(2026, 6, 16)))

    def test_refresh_until_date_must_match_partition(self) -> None:
        with self.assertRaises(ValueError):
            build_year_ranges(["2025"], refresh_until_date="2026-06-16")


class ChinabondParserTest(unittest.TestCase):
    def test_normalize_row_converts_dates_and_optional_yields(self) -> None:
        row = normalize_chinabond_row(sample_row())

        self.assertEqual(row["work_date"], date(2026, 6, 16))
        self.assertEqual(row["curve_name"], "中债国债收益率曲线")
        self.assertEqual(row["three_month_yield_pct"], 1.09)
        self.assertIsNone(row["fifteen_year_yield_pct"])
        self.assertIsNone(row["twenty_year_yield_pct"])
        self.assertEqual(row["thirty_year_yield_pct"], 2.22)

    def test_parse_rows_sorts_by_work_date(self) -> None:
        rows = parse_chinabond_rows(
            {
                "flag": "0",
                "heList": [
                    sample_row(workTime="2026-06-16"),
                    sample_row(workTime="2026-01-04"),
                ],
            },
            partition_key="2026",
        )

        self.assertEqual([row["work_date"] for row in rows], [date(2026, 1, 4), date(2026, 6, 16)])

    def test_parse_rows_rejects_business_failure_flag(self) -> None:
        with self.assertRaises(ChinabondRequestError):
            parse_chinabond_rows({"flag": "1", "heList": None}, partition_key="2026")

    def test_parse_rows_rejects_duplicate_work_date(self) -> None:
        with self.assertRaises(ChinabondRequestError):
            parse_chinabond_rows(
                {"flag": "0", "heList": [sample_row(), sample_row()]},
                partition_key="2026",
            )

    def test_parse_rows_rejects_unexpected_curve_name(self) -> None:
        with self.assertRaises(ChinabondRequestError):
            normalize_chinabond_row(sample_row(qxmc="????"))

    def test_rows_to_table_uses_contract_schema(self) -> None:
        rows = parse_chinabond_rows(
            {"flag": "0", "heList": [sample_row()]},
            partition_key="2026",
        )

        table = chinabond_rows_to_table(rows)

        self.assertEqual(table.schema, PARQUET_SCHEMAS[CHINABOND_DATASET])
        self.assertEqual(table.num_rows, 1)
        self.assertTrue(pa.types.is_date32(table.schema.field("work_date").type))


class FakeChinabondClient:
    def __init__(self) -> None:
        self.stats = HttpFetchStats()
