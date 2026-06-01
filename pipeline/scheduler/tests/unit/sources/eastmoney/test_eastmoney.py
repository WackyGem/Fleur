from __future__ import annotations

import unittest
from datetime import date
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path

import pyarrow as pa
from scheduler.defs.contract_schemas import PARQUET_SCHEMAS
from scheduler.defs.sources.eastmoney.assets import baostock_code_to_eastmoney_code
from scheduler.defs.sources.eastmoney.client import (
    EastmoneyAioHttpClient,
    EastmoneyRequestError,
    build_request_params,
    parse_eastmoney_page,
)
from scheduler.defs.sources.eastmoney.generated import fields as generated_fields
from scheduler.defs.sources.eastmoney.schema import (
    EASTMONEY_SCHEMAS,
    ENDPOINT_CONFIGS,
    EastmoneyFetchedRow,
    eastmoney_business_field_names,
    eastmoney_rows_to_table,
    eastmoney_schema,
    endpoint_by_asset_name,
)
from tests.fakes.http import FakeEastmoneyHttpClient
from tests.helpers.paths import find_repo_root

REPO_ROOT = find_repo_root(Path(__file__).resolve())
EXTRACT_SCRIPT_PATH = REPO_ROOT / "pipeline/scheduler/scripts/extract_eastmoney_schema_fields.py"
EXTRACT_SCRIPT_SPEC = spec_from_file_location(
    "extract_eastmoney_schema_fields",
    EXTRACT_SCRIPT_PATH,
)
if EXTRACT_SCRIPT_SPEC is None or EXTRACT_SCRIPT_SPEC.loader is None:
    msg = f"Could not load {EXTRACT_SCRIPT_PATH}"
    raise RuntimeError(msg)
extract_eastmoney_schema_fields = module_from_spec(EXTRACT_SCRIPT_SPEC)
EXTRACT_SCRIPT_SPEC.loader.exec_module(extract_eastmoney_schema_fields)

GENERATE_SCHEMA_SCRIPT_PATH = (
    REPO_ROOT / "pipeline/scheduler/scripts" / ("generate_" + "eastmoney_schemas.py")
)

EXPECTED_OPENAPI_FIELD_COUNTS = {
    "eastmoney__balance": 319,
    "eastmoney__cashflow_sq": 372,
    "eastmoney__cashflow_ytd": 254,
    "eastmoney__dividend_allotment": 10,
    "eastmoney__dividend_main": 30,
    "eastmoney__equity_history": 69,
    "eastmoney__income_sq": 299,
    "eastmoney__income_ytd": 203,
}

EXPECTED_PHASE_4_SCHEMA_FIELDS = {
    "eastmoney__dividend_allotment": {
        "EX_DIVIDEND_DATEE": (pa.date32(), False),
    },
    "eastmoney__dividend_main": {
        "EQUITY_RECORD_DATE": (pa.date32(), True),
        "EX_DIVIDEND_DATE": (pa.date32(), True),
        "PAY_CASH_DATE": (pa.date32(), True),
        "ASSIGN_OBJECT": (pa.string(), True),
        "GMDECISION_NOTICE_DATE": (pa.date32(), True),
        "INFO_CODE": (pa.string(), True),
        "DAT_YAGGR": (pa.date32(), True),
        "REPORT_TIME": (pa.string(), True),
        "LAST_TRADE_DATE": (pa.date32(), True),
    },
    "eastmoney__balance": {
        "OPINION_TYPE": (pa.string(), True),
        "OSOPINION_TYPE": (pa.string(), True),
    },
    "eastmoney__cashflow_sq": {
        "OPINION_TYPE": (pa.string(), True),
        "OSOPINION_TYPE": (pa.string(), True),
    },
    "eastmoney__cashflow_ytd": {
        "OPINION_TYPE": (pa.string(), True),
        "OSOPINION_TYPE": (pa.string(), True),
    },
    "eastmoney__income_sq": {
        "OPINION_TYPE": (pa.string(), True),
        "OSOPINION_TYPE": (pa.string(), True),
    },
    "eastmoney__income_ytd": {
        "OPINION_TYPE": (pa.string(), True),
    },
}


class EastmoneySchemaTest(unittest.TestCase):
    def test_all_endpoint_schemas_include_complete_openapi_fields_with_correct_types(self) -> None:
        """验证所有端点 schema 包含完整的 OpenAPI 字段，并使用正确的类型。"""
        self.assertEqual(len(ENDPOINT_CONFIGS), 8)

        for endpoint in ENDPOINT_CONFIGS:
            with self.subTest(endpoint=endpoint.asset_name):
                field_names = eastmoney_business_field_names(endpoint.asset_name)
                schema = eastmoney_schema(endpoint)

                self.assertEqual(
                    len(field_names),
                    EXPECTED_OPENAPI_FIELD_COUNTS[endpoint.asset_name],
                )
                self.assertEqual(
                    schema.names,
                    list(field_names),
                )
                # 验证 schema 包含字符串字段
                has_string = any(pa.types.is_string(field.type) for field in schema)
                self.assertTrue(has_string, "Schema should have string fields")

                # 验证 schema 包含日期字段
                has_date = any(pa.types.is_date(field.type) for field in schema)
                self.assertTrue(has_date, "Schema should have date fields")

    def test_static_fields_match_openapi_yaml_extraction(self) -> None:
        extracted = extract_eastmoney_schema_fields.extract_all_field_names(
            REPO_ROOT / "docs/references/openapi"
        )

        for endpoint in ENDPOINT_CONFIGS:
            with self.subTest(endpoint=endpoint.asset_name):
                self.assertEqual(
                    eastmoney_business_field_names(endpoint.asset_name),
                    extracted[endpoint.asset_name],
                )

    def test_generated_modules_are_the_schema_source_of_truth(self) -> None:
        self.assertEqual(
            set(generated_fields.EASTMONEY_FIELD_NAMES), set(EXPECTED_OPENAPI_FIELD_COUNTS)
        )
        self.assertEqual(set(EASTMONEY_SCHEMAS), set(EXPECTED_OPENAPI_FIELD_COUNTS))

    def test_endpoint_schemas_come_from_global_generated_map(self) -> None:
        self.assertFalse(GENERATE_SCHEMA_SCRIPT_PATH.exists())
        for endpoint in ENDPOINT_CONFIGS:
            with self.subTest(endpoint=endpoint.asset_name):
                self.assertIs(eastmoney_schema(endpoint), PARQUET_SCHEMAS[endpoint.asset_name])

    def test_phase_4_field_types_and_nullable_facts(self) -> None:
        for asset_name, expected_fields in EXPECTED_PHASE_4_SCHEMA_FIELDS.items():
            schema = eastmoney_schema(endpoint_by_asset_name(asset_name))
            with self.subTest(asset=asset_name):
                for field_name, (expected_type, expected_nullable) in expected_fields.items():
                    field = schema.field(field_name)
                    self.assertEqual(field.type, expected_type)
                    self.assertEqual(field.nullable, expected_nullable)

    def test_rows_to_table_preserves_missing_fields_as_null_and_counts_unknowns(self) -> None:
        endpoint = endpoint_by_asset_name("eastmoney__dividend_main")
        result = eastmoney_rows_to_table(
            endpoint,
            [
                EastmoneyFetchedRow(
                    data={
                        "SECUCODE": "601088.SH",
                        "SECURITY_CODE": "601088",
                        "TOTAL_DIVIDEND": 19471149555.9,
                        "EXTRA_FIELD": "ignored",
                    }
                )
            ],
        )

        self.assertEqual(result.unknown_field_count, 1)
        self.assertEqual(result.table.num_rows, 1)
        self.assertEqual(result.table["TOTAL_DIVIDEND"].to_pylist(), [19471149555.9])
        self.assertEqual(result.table["NOTICE_DATE"].to_pylist(), [None])
        self.assertNotIn("EXTRA_FIELD", result.table.column_names)
        self.assertNotIn("request_code", result.table.column_names)
        self.assertNotIn("request_start_date", result.table.column_names)
        self.assertNotIn("request_end_date", result.table.column_names)
        self.assertNotIn("partition_year", result.table.column_names)
        self.assertNotIn("source_endpoint", result.table.column_names)
        self.assertNotIn("ingested_at", result.table.column_names)

    def test_dividend_main_report_time_accepts_historical_report_label(self) -> None:
        endpoint = endpoint_by_asset_name("eastmoney__dividend_main")
        result = eastmoney_rows_to_table(
            endpoint,
            [
                EastmoneyFetchedRow(
                    data={
                        "SECUCODE": "600000.SH",
                        "SECURITY_CODE": "600000",
                        "NOTICE_DATE": "1992-01-01 00:00:00",
                        "REPORT_TIME": "1991年报",
                    }
                )
            ],
        )

        self.assertEqual(result.table.num_rows, 1)
        self.assertEqual(result.table["REPORT_TIME"].to_pylist(), ["1991年报"])


class EastmoneyClientTest(unittest.TestCase):
    def test_code_conversion_accepts_shanghai_and_shenzhen_only(self) -> None:
        self.assertEqual(baostock_code_to_eastmoney_code("sh.600000"), "600000.SH")
        self.assertEqual(baostock_code_to_eastmoney_code("sz.000001"), "000001.SZ")
        self.assertIsNone(baostock_code_to_eastmoney_code("bj.430047"))

    def test_data_get_params_use_expected_pagination_and_sort_keys(self) -> None:
        endpoint = endpoint_by_asset_name("eastmoney__balance")
        params = build_request_params(
            endpoint,
            "601088.SH",
            date(2026, 1, 1),
            date(2026, 5, 27),
            page_number=2,
        )

        self.assertEqual(params["type"], "RPT_F10_FINANCE_GBALANCE")
        self.assertEqual(params["sty"], "F10_FINANCE_GBALANCE")
        self.assertEqual(params["p"], "2")
        self.assertEqual(params["ps"], "500")
        self.assertEqual(params["st"], "REPORT_DATE,SECURITY_CODE")
        self.assertEqual(params["sr"], "-1,-1")
        self.assertNotIn("pageNumber", params)
        self.assertIn('(SECUCODE="601088.SH")', params["filter"])
        self.assertIn("(NOTICE_DATE>='2026-01-01')", params["filter"])
        self.assertIn("(NOTICE_DATE<='2026-05-27')", params["filter"])

    def test_data_v1_get_params_use_expected_pagination_and_sort_keys(self) -> None:
        endpoint = endpoint_by_asset_name("eastmoney__dividend_main")
        params = build_request_params(
            endpoint,
            "601088.SH",
            date(2026, 1, 1),
            date(2026, 5, 27),
            page_number=3,
        )

        self.assertEqual(params["reportName"], "RPT_F10_DIVIDEND_MAIN")
        self.assertEqual(params["columns"], "ALL")
        self.assertEqual(params["pageNumber"], "3")
        self.assertEqual(params["pageSize"], "500")
        self.assertEqual(params["sortColumns"], "NOTICE_DATE,SECURITY_CODE")
        self.assertEqual(params["sortTypes"], "-1,-1")
        self.assertNotIn("p", params)

    def test_code_9201_null_result_is_empty_page(self) -> None:
        endpoint = endpoint_by_asset_name("eastmoney__dividend_main")
        page = parse_eastmoney_page(endpoint, {"code": 9201, "result": None})

        self.assertTrue(page.is_empty)
        self.assertEqual(page.rows, [])
        self.assertEqual(page.total_pages, 0)


class EastmoneyPaginationTest(unittest.IsolatedAsyncioTestCase):
    async def test_single_code_paginates_sequentially(self) -> None:
        endpoint = endpoint_by_asset_name("eastmoney__balance")
        http_client = FakeEastmoneyHttpClient(
            {
                1: {"result": {"pages": 2, "data": [{"SECURITY_CODE": "601088"}]}},
                2: {"result": {"pages": 2, "data": [{"SECURITY_CODE": "600000"}]}},
            },
        )
        client = EastmoneyAioHttpClient(http_client=http_client)

        async with client:
            rows = await client.fetch_code_range(
                endpoint,
                "601088.SH",
                date(2026, 1, 1),
                date(2026, 5, 27),
            )

        self.assertEqual(http_client.requested_pages, [1, 2])
        self.assertEqual([row["SECURITY_CODE"] for row in rows], ["601088", "600000"])
        self.assertEqual(client.stats.page_count, 2)

    async def test_duplicate_row_across_pages_fails(self) -> None:
        endpoint = endpoint_by_asset_name("eastmoney__balance")
        http_client = FakeEastmoneyHttpClient(
            {
                1: {"result": {"pages": 2, "data": [{"SECURITY_CODE": "601088"}]}},
                2: {"result": {"pages": 2, "data": [{"SECURITY_CODE": "601088"}]}},
            },
        )
        client = EastmoneyAioHttpClient(http_client=http_client)

        async with client:
            with self.assertRaises(EastmoneyRequestError):
                await client.fetch_code_range(
                    endpoint,
                    "601088.SH",
                    date(2026, 1, 1),
                    date(2026, 5, 27),
                )

        self.assertEqual(http_client.requested_pages, [1, 2])
        self.assertEqual(client.stats.duplicate_page_row_count, 1)
