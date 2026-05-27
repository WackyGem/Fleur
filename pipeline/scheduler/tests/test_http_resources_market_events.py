from __future__ import annotations

import os
import time
import unittest
from datetime import date
from unittest.mock import patch

import dagster as dg

from scheduler.defs.http_resources.client import HttpFetchStats
from scheduler.defs.http_resources.flatten import flatten_content_object
from scheduler.defs.http_resources.jiuyan__action_field import (
    _fetch_action_field_table,
    jiuyan_header_factory,
)
from scheduler.defs.http_resources.jiuyan__industry_list import (
    _fetch_industry_list_table_with_client,
)
from scheduler.defs.http_resources.partitioned import (
    sync_trade_date_dynamic_partitions,
    trade_date_dynamic_partitions,
)
from scheduler.defs.http_resources.schemas import (
    jiuyan_action_field_to_table,
    jiuyan_industry_list_to_table,
    ths_limit_up_pool_to_table,
)
from scheduler.defs.http_resources.ths__limit_up_pool import (
    _fetch_limit_up_pool_table,
    _limit_up_pool_params,
)


class FakeJsonClient:
    def __init__(self, payloads: list[dict[str, object]]) -> None:
        self.payloads = payloads
        self.requests: list[object] = []
        self.stats = HttpFetchStats()

    async def request_json_object(self, request: object) -> dict[str, object]:
        self.requests.append(request)
        self.stats.request_count += 1
        return self.payloads.pop(0)


class JiuYanHeaderTest(unittest.TestCase):
    def test_header_factory_reads_credentials_and_generates_dynamic_timestamp(self) -> None:
        with patch.dict(
            os.environ,
            {"JIUYAN_TOKEN": "token-value", "JIUYAN_COOKIE": "SESSION=session-value"},
        ):
            with patch("time.time", side_effect=[1778309697.0, 1778309698.0]):
                headers = jiuyan_header_factory()
                first = headers()
                second = headers()

        self.assertEqual(first["token"], "token-value")
        self.assertEqual(first["cookie"], "SESSION=session-value")
        self.assertEqual(first["platform"], "3")
        self.assertEqual(first["timestamp"], "1778309697000")
        self.assertEqual(second["timestamp"], "1778309698000")

    def test_header_factory_requires_jiuyan_credentials(self) -> None:
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaises(RuntimeError):
                jiuyan_header_factory()


class MarketEventSchemaTest(unittest.TestCase):
    def test_sync_trade_date_dynamic_partitions_adds_only_missing_keys(self) -> None:
        instance = dg.DagsterInstance.ephemeral()

        first_sync = sync_trade_date_dynamic_partitions(
            instance,
            {date(2026, 5, 8), date(2026, 5, 11)},
        )
        second_sync = sync_trade_date_dynamic_partitions(
            instance,
            {date(2026, 5, 8), date(2026, 5, 11), date(2026, 5, 12)},
        )

        self.assertEqual(first_sync, ["2026-05-08", "2026-05-11"])
        self.assertEqual(second_sync, ["2026-05-12"])
        self.assertEqual(
            sorted(instance.get_dynamic_partitions(trade_date_dynamic_partitions.name)),
            ["2026-05-08", "2026-05-11", "2026-05-12"],
        )

    def test_flatten_content_object_uses_shortest_leaf_naming(self) -> None:
        flattened = flatten_content_object(
            {
                "list": [
                    {
                        "article": {
                            "action_info": {"time": "09:25:00"},
                        }
                    }
                ],
                "result": {"imgs": '["https://example.test/a.png"]'},
            }
        )

        self.assertEqual(flattened["time"], ["09:25:00"])
        self.assertEqual(flattened["imgs"], '["https://example.test/a.png"]')
        self.assertNotIn("list[].article.action_info.time", flattened)

    def test_action_field_table_uses_short_leaf_columns_and_selected_stock_rows(self) -> None:
        result = jiuyan_action_field_to_table(
            [
                {
                    "action_field_id": "field-1",
                    "name": "公告",
                    "date": "2026-05-08",
                    "reason": "板块原因",
                    "status": 0,
                    "list": [
                        {
                            "code": "sh603045",
                            "name": "福达合金",
                            "article": {
                                "article_id": "article-1",
                                "action_info": {
                                    "time": "09:25:00",
                                    "num": "4天4板",
                                    "reason": None,
                                    "expound": "个股原因",
                                    "stock_id": "stock-1",
                                },
                                "user": {"nickname": "韭菜团子"},
                            },
                        }
                    ],
                }
            ]
        )
        table = result.table

        self.assertEqual(table.num_rows, 1)
        self.assertIn("time", table.column_names)
        self.assertIn("name", table.column_names)
        self.assertIn("reason", table.column_names)
        self.assertNotIn("article.action_info.time", table.column_names)
        self.assertNotIn("article.user.nickname", table.column_names)
        self.assertNotIn("stock.name", table.column_names)
        self.assertNotIn("list[].article.action_info.time", table.column_names)
        self.assertNotIn("status", table.column_names)
        self.assertNotIn("article_id", table.column_names)
        self.assertNotIn("stock_id", table.column_names)
        self.assertNotIn("errCode", table.column_names)
        self.assertNotIn("serverTime", table.column_names)
        self.assertNotIn("source_endpoint", table.column_names)
        self.assertEqual(table["time"].to_pylist(), ["09:25:00"])
        self.assertEqual(table["name"].to_pylist(), ["福达合金"])
        self.assertEqual(table["reason"].to_pylist(), ["板块原因"])
        self.assertGreater(result.unknown_field_count, 0)

    def test_ths_table_uses_info_rows_and_drops_page_and_count_columns(self) -> None:
        table = ths_limit_up_pool_to_table(
            [
                {
                    "date": "20260508",
                    "page": {"count": 1, "page": 1},
                    "info": [
                        {
                            "code": "000001",
                            "name": "平安银行",
                            "latest": 10.1,
                            "time_preview": ["09:25", "09:30"],
                        }
                    ],
                    "limit_up_count": {"today": {"num": 1}},
                    "limit_down_count": {"today": {"num": 0}},
                    "trade_status": {"id": "open"},
                }
            ]
        ).table

        self.assertEqual(table.num_rows, 1)
        self.assertIn("date", table.column_names)
        self.assertIn("code", table.column_names)
        self.assertIn("latest", table.column_names)
        self.assertNotIn("page.count", table.column_names)
        self.assertNotIn("time_preview", table.column_names)
        self.assertNotIn("time_preview[]", table.column_names)
        self.assertNotIn("info[].code", table.column_names)
        self.assertNotIn("limit_up_count.today.num", table.column_names)
        self.assertNotIn("limit_down_count.today.num", table.column_names)
        self.assertNotIn("trade_status", table.column_names)
        self.assertNotIn("status_code", table.column_names)
        self.assertNotIn("status_msg", table.column_names)
        self.assertEqual(table["date"].to_pylist(), ["20260508"])
        self.assertEqual(table["latest"].to_pylist(), ["10.1"])

    def test_industry_table_keeps_only_result_rows_and_imgs_string(self) -> None:
        table = jiuyan_industry_list_to_table(
            [
                {
                    "pageNo": 1,
                    "hasNext": False,
                    "nextPage": 1,
                    "result": [
                        {
                            "industry_id": "industry-1",
                            "imgs": '["https://example.test/a.png"]',
                        }
                    ],
                }
            ]
        ).table

        self.assertEqual(
            table["imgs"].to_pylist(),
            ['["https://example.test/a.png"]'],
        )
        self.assertNotIn("pageNo", table.column_names)
        self.assertNotIn("hasNext", table.column_names)
        self.assertNotIn("result[].imgs", table.column_names)
        self.assertNotIn("ingested_at", table.column_names)


class MarketEventFetchTest(unittest.IsolatedAsyncioTestCase):
    async def test_action_field_fetch_validates_success_and_returns_empty_table(self) -> None:
        client = FakeJsonClient([{"errCode": "0", "data": []}])

        table, metadata = await _fetch_action_field_table(
            client,
            trade_date=date(2026, 5, 8),
        )

        self.assertEqual(table.num_rows, 0)
        self.assertEqual(metadata["empty_response_count"], 1)

    async def test_ths_limit_up_pool_uses_page_count_and_detects_duplicates(self) -> None:
        client = FakeJsonClient(
            [
                {
                    "status_code": 0,
                    "status_msg": "success",
                    "data": {
                        "page": {"count": 2, "page": 1},
                        "date": "20260508",
                        "info": [{"code": "000001"}],
                    },
                },
                {
                    "status_code": 0,
                    "status_msg": "success",
                    "data": {
                        "page": {"count": 2, "page": 2},
                        "date": "20260508",
                        "info": [{"code": "000001"}],
                    },
                },
            ]
        )

        with self.assertRaises(RuntimeError):
            await _fetch_limit_up_pool_table(
                client,
                trade_date=date(2026, 5, 8),
            )

        self.assertEqual([request.params["page"] for request in client.requests], ["1", "2"])

    async def test_industry_list_paginates_by_has_next_and_next_page(self) -> None:
        client = FakeJsonClient(
            [
                {
                    "errCode": "0",
                    "data": {
                        "pageNo": 1,
                        "hasNext": True,
                        "nextPage": 2,
                        "result": [{"industry_id": "a"}],
                    },
                },
                {
                    "errCode": "0",
                    "data": {
                        "pageNo": 2,
                        "hasNext": False,
                        "nextPage": 2,
                        "result": [{"industry_id": "b"}],
                    },
                },
            ]
        )

        table, metadata = await _fetch_industry_list_table_with_client(
            client,
            started_at=time.perf_counter(),
        )

        self.assertEqual(table.num_rows, 2)
        self.assertEqual(metadata["industry_total_rows"], 2)
        self.assertEqual(
            [request.json_body["start"] for request in client.requests],
            ["0", "2"],
        )

    def test_ths_params_use_trade_date_and_timestamp(self) -> None:
        with patch("time.time", return_value=1778299947.223):
            params = _limit_up_pool_params(
                trade_date=date(2026, 4, 29),
                page_number=1,
            )

        self.assertEqual(params["date"], "20260429")
        self.assertEqual(params["_"], "1778299947223")
        self.assertEqual(params["limit"], "200")
        self.assertEqual(params["filter"], "HS,GEM2STAR")
