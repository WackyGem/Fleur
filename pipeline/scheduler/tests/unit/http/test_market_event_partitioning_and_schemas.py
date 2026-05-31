from __future__ import annotations

import os
import time
import unittest
from datetime import date
from datetime import time as dt_time
from tempfile import TemporaryDirectory
from typing import cast
from unittest.mock import patch

import dagster as dg
import pyarrow as pa
import pyarrow.parquet as pq
from dagster._core.definitions.metadata.metadata_value import JsonMetadataValue
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.config.models import S3Config
from scheduler.defs.http.flatten import flatten_content_object
from scheduler.defs.http.partitioning import (
    JIUYAN_BACKFILL_MAX_TRADE_DATES,
    THS_BACKFILL_MAX_NATURAL_DAYS,
    jiuyan_action_field_daily_partitions,
    materialize_trade_date_range,
    ths_limit_up_pool_daily_partitions,
)
from scheduler.defs.http.schemas import (
    jiuyan_action_field_to_table,
    jiuyan_industry_list_to_table,
    ths_limit_up_pool_to_table,
)
from scheduler.defs.sources.jiuyan.action_field import (
    fetch_action_field_table_with_client,
    jiuyan_header_factory,
)
from scheduler.defs.sources.jiuyan.industry_list import (
    fetch_industry_list_table_with_client,
)
from scheduler.defs.sources.ths.limit_up_pool import (
    fetch_limit_up_pool_table_with_client,
    limit_up_pool_params,
)
from tests.fakes.dagster import FakeAssetContext
from tests.fakes.http import FakeJsonClient
from tests.fakes.storage import local_filesystem


def fake_s3_config(bucket: str) -> S3Config:
    return S3Config(
        endpoint="http://localhost:9000",
        bucket=bucket,
        access_key="access",
        secret_key="secret",
    )


class JiuYanHeaderTest(unittest.TestCase):
    def test_header_factory_reads_credentials_and_generates_dynamic_timestamp(self) -> None:
        with (
            patch.dict(
                os.environ,
                {"JIUYAN_TOKEN": "token-value", "JIUYAN_COOKIE": "SESSION=session-value"},
            ),
            patch("time.time", side_effect=[1778309697.0, 1778309698.0]),
        ):
            headers = jiuyan_header_factory()
            first = headers()
            second = headers()

        self.assertEqual(first["token"], "token-value")
        self.assertEqual(first["cookie"], "SESSION=session-value")
        self.assertEqual(first["platform"], "3")
        self.assertEqual(first["timestamp"], "1778309697000")
        self.assertEqual(second["timestamp"], "1778309698000")

    def test_header_factory_requires_jiuyan_credentials(self) -> None:
        with patch.dict(os.environ, {}, clear=True), self.assertRaises(RuntimeError):
            jiuyan_header_factory()


class MarketEventSchemaTest(unittest.TestCase):
    def test_market_event_assets_use_natural_day_partition_starts(self) -> None:
        self.assertEqual(
            jiuyan_action_field_daily_partitions.get_first_partition_key(), "2021-01-01"
        )
        self.assertEqual(ths_limit_up_pool_daily_partitions.get_first_partition_key(), "2025-01-01")

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
        self.assertEqual(table["time"].to_pylist(), [dt_time(9, 25)])
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
        self.assertEqual(table["date"].to_pylist(), [date(2026, 5, 8)])
        self.assertEqual(table["latest"].to_pylist(), [10.1])

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
    async def test_trade_date_range_skips_non_trade_dates_without_fetching_or_writing(self) -> None:
        fetched_dates: list[date] = []

        async def fetch_table_for_trade_date(
            trade_date: date,
        ) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
            fetched_dates.append(trade_date)
            return pa.table({"value": [trade_date.isoformat()]}), {"row_count": 1}

        with TemporaryDirectory() as bucket:
            context = FakeAssetContext(
                partition_keys=["2026-05-08", "2026-05-09", "2026-05-10"],
                asset_key=dg.AssetKey(["source", "jiuyan__action_field"]),
            )
            s3_config = fake_s3_config(bucket)

            with (
                patch(
                    "scheduler.defs.storage.dataset_writer.build_s3_filesystem",
                    return_value=local_filesystem(),
                ),
                patch(
                    "scheduler.defs.http.partitioning.read_trade_dates_from_s3",
                    return_value={date(2026, 5, 8)},
                ),
            ):
                result = await materialize_trade_date_range(
                    context,
                    max_concurrent_trade_dates=2,
                    fetch_table_for_trade_date=fetch_table_for_trade_date,
                    s3_config=s3_config,
                )

            written_table = pq.read_table(
                f"{bucket}/source/jiuyan__action_field/trade_date=2026-05-08/000000_0.parquet"
            )

        self.assertEqual(fetched_dates, [date(2026, 5, 8)])
        self.assertEqual(written_table.num_rows, 1)
        self.assertEqual(result.metadata["processed_trade_date_count"], 1)
        self.assertEqual(result.metadata["skipped_non_trade_date_count"], 2)

    async def test_action_field_fetch_validates_success_and_returns_empty_table(self) -> None:
        client = FakeJsonClient([{"errCode": "0", "data": []}])

        table, metadata = await fetch_action_field_table_with_client(
            client,
            trade_date=date(2026, 5, 8),
        )

        self.assertEqual(table.num_rows, 0)
        self.assertEqual(metadata["empty_response_count"], 1)

    async def test_action_field_fetch_retries_rate_limit_then_succeeds(self) -> None:
        client = FakeJsonClient(
            [
                {"errCode": "9", "msg": "服务繁忙"},
                {"errCode": "9", "msg": "服务繁忙"},
                {"errCode": "0", "data": []},
            ]
        )

        with patch("scheduler.defs.sources.jiuyan.action_field.asyncio.sleep") as sleep:
            table, metadata = await fetch_action_field_table_with_client(
                client,
                trade_date=date(2026, 5, 8),
            )

        # 第三次请求成功后，结果必须被返回，而不是抛出此前的限流错误
        self.assertEqual(table.num_rows, 0)
        self.assertEqual(len(client.requests), 3)
        self.assertEqual(metadata["retry_rounds"], 2)
        self.assertEqual(metadata["source_err_code"], "0")
        # 两次限流 -> 两次退避，且为指数递增
        self.assertEqual([call.args[0] for call in sleep.await_args_list], [1.0, 2.0])

    async def test_action_field_fetch_raises_after_rate_limit_exhausted(self) -> None:
        client = FakeJsonClient([{"errCode": "9", "msg": "服务繁忙"}] * 6)

        with (
            patch("scheduler.defs.sources.jiuyan.action_field.asyncio.sleep"),
            self.assertRaises(RuntimeError) as ctx,
        ):
            await fetch_action_field_table_with_client(client, trade_date=date(2026, 5, 8))

        self.assertEqual(len(client.requests), 6)
        self.assertIn("still rate-limited", str(ctx.exception))

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
            await fetch_limit_up_pool_table_with_client(
                client,
                trade_date=date(2026, 5, 8),
            )

        self.assertEqual(
            [request.params["page"] for request in client.requests if request.params is not None],
            ["1", "2"],
        )

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

        table, metadata = await fetch_industry_list_table_with_client(
            client,
            started_at=time.perf_counter(),
        )

        self.assertEqual(table.num_rows, 2)
        self.assertEqual(metadata["industry_total_rows"], 2)
        self.assertEqual(
            [
                request.json_body["start"]
                for request in client.requests
                if isinstance(request.json_body, dict)
            ],
            ["0", "2"],
        )

    def test_ths_params_use_trade_date_and_timestamp(self) -> None:
        with patch("time.time", return_value=1778299947.223):
            params = limit_up_pool_params(
                trade_date=date(2026, 4, 29),
                page_number=1,
            )

        self.assertEqual(params["date"], "20260429")
        self.assertEqual(params["_"], "1778299947223")
        self.assertEqual(params["limit"], "200")
        self.assertEqual(params["filter"], "HS,GEM2STAR")

    def test_backfill_constants_are_defined(self) -> None:
        """验证回填窗口限制常量已定义。"""
        self.assertEqual(THS_BACKFILL_MAX_NATURAL_DAYS, 380)
        self.assertEqual(JIUYAN_BACKFILL_MAX_TRADE_DATES, 80)


class BackfillWindowLimitTest(unittest.IsolatedAsyncioTestCase):
    async def test_backfill_window_limit_truncates_partitions(self) -> None:
        """测试回填窗口按交易日数量裁切。"""
        fetched_dates: list[date] = []

        async def fetch_table_for_trade_date(
            trade_date: date,
        ) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
            fetched_dates.append(trade_date)
            return pa.table({"value": [trade_date.isoformat()]}), {"row_count": 1}

        # 创建 100 个分区
        partition_keys = [f"2026-01-{i:02d}" for i in range(1, 32)]
        partition_keys += [f"2026-02-{i:02d}" for i in range(1, 29)]
        partition_keys += [f"2026-03-{i:02d}" for i in range(1, 32)]
        partition_keys += [f"2026-04-{i:02d}" for i in range(1, 11)]

        with TemporaryDirectory() as bucket:
            context = FakeAssetContext(
                partition_keys=partition_keys,
                asset_key=dg.AssetKey(["source", "jiuyan__action_field"]),
            )
            s3_config = fake_s3_config(bucket)

            with (
                patch(
                    "scheduler.defs.storage.dataset_writer.build_s3_filesystem",
                    return_value=local_filesystem(),
                ),
                patch(
                    "scheduler.defs.http.partitioning.read_trade_dates_from_s3",
                    return_value={date(2026, 1, d) for d in range(1, 32)}
                    | {date(2026, 2, d) for d in range(1, 29)}
                    | {date(2026, 3, d) for d in range(1, 32)}
                    | {date(2026, 4, d) for d in range(1, 11)},
                ),
            ):
                result = await materialize_trade_date_range(
                    context,
                    max_concurrent_trade_dates=4,
                    fetch_table_for_trade_date=fetch_table_for_trade_date,
                    backfill_window_limit=50,
                    s3_config=s3_config,
                )

        # 应该保留完整自然日请求范围，但只处理最后 50 个交易日
        self.assertEqual(len(fetched_dates), 50)
        self.assertEqual(result.metadata["requested_natural_date_count"], 100)
        self.assertEqual(result.metadata["requested_trade_date_count"], 100)
        self.assertEqual(result.metadata["processed_trade_date_count"], 50)
        self.assertEqual(result.metadata["skipped_window_trade_date_count"], 50)
        self.assertEqual(fetched_dates[0], date(2026, 2, 20))
        self.assertEqual(fetched_dates[-1], date(2026, 4, 10))

    async def test_backfill_window_limit_counts_trade_dates_not_natural_dates(self) -> None:
        fetched_dates: list[date] = []

        async def fetch_table_for_trade_date(
            trade_date: date,
        ) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
            fetched_dates.append(trade_date)
            return pa.table({"value": [trade_date.isoformat()]}), {"row_count": 1}

        partition_keys = [f"2026-05-{i:02d}" for i in range(1, 11)]

        with TemporaryDirectory() as bucket:
            context = FakeAssetContext(
                partition_keys=partition_keys,
                asset_key=dg.AssetKey(["source", "jiuyan__action_field"]),
            )
            s3_config = fake_s3_config(bucket)

            with (
                patch(
                    "scheduler.defs.storage.dataset_writer.build_s3_filesystem",
                    return_value=local_filesystem(),
                ),
                patch(
                    "scheduler.defs.http.partitioning.read_trade_dates_from_s3",
                    return_value={
                        date(2026, 5, 1),
                        date(2026, 5, 4),
                        date(2026, 5, 8),
                    },
                ),
            ):
                result = await materialize_trade_date_range(
                    context,
                    max_concurrent_trade_dates=2,
                    fetch_table_for_trade_date=fetch_table_for_trade_date,
                    backfill_window_limit=2,
                    s3_config=s3_config,
                )

        self.assertEqual(fetched_dates, [date(2026, 5, 4), date(2026, 5, 8)])
        self.assertEqual(result.metadata["requested_natural_date_count"], 10)
        self.assertEqual(result.metadata["requested_trade_date_count"], 3)
        self.assertEqual(result.metadata["processed_trade_date_count"], 2)
        self.assertEqual(result.metadata["skipped_window_trade_date_count"], 1)
        self.assertEqual(result.metadata["skipped_non_trade_date_count"], 7)

    async def test_all_partition_failures_raise_error(self) -> None:
        async def fetch_table_for_trade_date(
            trade_date: date,
        ) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
            msg = f"boom for {trade_date.isoformat()}"
            raise RuntimeError(msg)

        with TemporaryDirectory() as bucket:
            context = FakeAssetContext(
                partition_keys=["2026-05-08"],
                asset_key=dg.AssetKey(["source", "jiuyan__action_field"]),
            )
            s3_config = fake_s3_config(bucket)

            with (
                patch(
                    "scheduler.defs.storage.dataset_writer.build_s3_filesystem",
                    return_value=local_filesystem(),
                ),
                patch(
                    "scheduler.defs.http.partitioning.read_trade_dates_from_s3",
                    return_value={date(2026, 5, 8)},
                ),
                self.assertRaises(RuntimeError) as ctx,
            ):
                await materialize_trade_date_range(
                    context,
                    max_concurrent_trade_dates=1,
                    fetch_table_for_trade_date=fetch_table_for_trade_date,
                    s3_config=s3_config,
                )

        self.assertIn("All 1 bounded tasks failed", str(ctx.exception))

    async def test_partial_partition_failures_include_runner_error_metadata(self) -> None:
        async def fetch_table_for_trade_date(
            trade_date: date,
        ) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
            if trade_date == date(2026, 5, 8):
                raise RuntimeError("boom for 2026-05-08")
            return pa.table({"value": [trade_date.isoformat()]}), {"row_count": 1}

        with TemporaryDirectory() as bucket:
            context = FakeAssetContext(
                partition_keys=["2026-05-08", "2026-05-11"],
                asset_key=dg.AssetKey(["source", "jiuyan__action_field"]),
            )
            s3_config = fake_s3_config(bucket)

            with (
                patch(
                    "scheduler.defs.storage.dataset_writer.build_s3_filesystem",
                    return_value=local_filesystem(),
                ),
                patch(
                    "scheduler.defs.http.partitioning.read_trade_dates_from_s3",
                    return_value={date(2026, 5, 8), date(2026, 5, 11)},
                ),
            ):
                result = await materialize_trade_date_range(
                    context,
                    max_concurrent_trade_dates=1,
                    fetch_table_for_trade_date=fetch_table_for_trade_date,
                    s3_config=s3_config,
                )

        self.assertEqual(result.metadata["failed_partition_count"], 1)
        failed_partition_errors = cast(
            JsonMetadataValue,
            result.metadata["failed_partition_errors"],
        )
        self.assertIsInstance(failed_partition_errors, JsonMetadataValue)
        self.assertEqual(
            failed_partition_errors.data,
            {
                "2026-05-08": {
                    "type": "RuntimeError",
                    "message": "boom for 2026-05-08",
                }
            },
        )

    async def test_no_limit_when_backfill_window_limit_is_none(self) -> None:
        """测试不设置限制时的行为。"""
        fetched_dates: list[date] = []

        async def fetch_table_for_trade_date(
            trade_date: date,
        ) -> tuple[pa.Table, dict[str, RawMetadataValue]]:
            fetched_dates.append(trade_date)
            return pa.table({"value": [trade_date.isoformat()]}), {"row_count": 1}

        # 创建 10 个分区
        partition_keys = [f"2026-05-{i:02d}" for i in range(1, 11)]

        with TemporaryDirectory() as bucket:
            context = FakeAssetContext(
                partition_keys=partition_keys,
                asset_key=dg.AssetKey(["source", "jiuyan__action_field"]),
            )
            s3_config = fake_s3_config(bucket)

            with (
                patch(
                    "scheduler.defs.storage.dataset_writer.build_s3_filesystem",
                    return_value=local_filesystem(),
                ),
                patch(
                    "scheduler.defs.http.partitioning.read_trade_dates_from_s3",
                    return_value={date(2026, 5, d) for d in range(1, 11)},
                ),
            ):
                result = await materialize_trade_date_range(
                    context,
                    max_concurrent_trade_dates=4,
                    fetch_table_for_trade_date=fetch_table_for_trade_date,
                    backfill_window_limit=None,
                    s3_config=s3_config,
                )

        # 应该处理了所有 10 个分区
        self.assertEqual(len(fetched_dates), 10)
        self.assertEqual(result.metadata["requested_natural_date_count"], 10)
