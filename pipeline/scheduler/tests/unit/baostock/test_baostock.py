from __future__ import annotations

import asyncio
import contextlib
from dataclasses import replace
from datetime import date
from typing import Any, cast

import dagster as dg
import pytest
from scheduler.defs.baostock import assets
from scheduler.defs.baostock.client import BaostockAioTcpClient
from scheduler.defs.baostock.protocol import (
    MESSAGE_END,
    MESSAGE_SPLIT,
    BaostockAuthenticationError,
    BaostockNetworkError,
    BaostockProtocolError,
    BaostockResponseError,
    aggregate_responses,
    decode_response,
    encode_request,
)
from scheduler.defs.baostock.schemas import (
    K_HISTORY_DAILY_FIELDS,
    STOCK_BASIC_FIELDS,
    k_history_daily_response_to_table,
    response_to_table,
    stock_basic_response_to_table,
)
from scheduler.defs.common.retry import ExponentialBackoffPolicy
from scheduler.defs.resources.baostock import BaostockClientFactoryResource
from scheduler.defs.resources.s3 import S3SettingsResource
from scheduler.defs.storage.dataset_service import DatasetLocation, DatasetWriteOptions
from scheduler.defs.storage.dataset_writer import DatasetPartitionWriteError, DatasetWriteResult
from tests.fakes.baostock import (
    FakeBaostockAssetClient,
    FakeBaostockClientFactory,
    baostock_response,
    client_config,
    queued_baostock_client,
    response_message,
    retrying_baostock_client,
)


def test_encode_request_includes_pagination_for_data_apis_but_not_login() -> None:
    data_request = encode_request(
        request_code="45",
        api_name="query_stock_basic",
        user_id="user",
        params=["", ""],
        page=2,
        page_size=500,
    ).decode("utf-8")
    login_request = encode_request(
        request_code="00",
        api_name="login",
        user_id="user",
        params=["password", "0"],
    ).decode("utf-8")

    assert "query_stock_basic\x01user\x012\x01500" in data_request
    assert "login\x01user\x01password\x010" in login_request
    assert "login\x01user\x011\x01" not in login_request


def test_decode_stock_basic_response_and_convert_to_table() -> None:
    response = decode_response(
        response_message(
            [
                "0",
                "",
                "query_stock_basic",
                "user",
                "1",
                "2",
                '{"record":[["sh.600000","浦发银行","1999-11-10","","1","1"]]}',
                "",
                "",
                ",".join(STOCK_BASIC_FIELDS),
            ]
        )
    )

    assert response.api_name == "query_stock_basic"
    assert response.field_names == STOCK_BASIC_FIELDS
    assert response.records == [["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]]
    table = stock_basic_response_to_table(response)
    assert table.column_names == STOCK_BASIC_FIELDS
    assert table["code"].to_pylist() == ["sh.600000"]


def test_k_history_daily_converts_is_st_to_bool() -> None:
    import pyarrow as pa

    response = baostock_response(
        api_name="query_history_k_data_plus",
        records=[
            [
                "2026-05-25",
                "sh.600000",
                "10.0",
                "11.0",
                "9.5",
                "10.5",
                "10.1",
                "1000",
                "10500.0",
                "3",
                "1.23",
                "1",
                "3.96",
                "1",
            ],
            [
                "2026-05-26",
                "sh.600000",
                "10.5",
                "10.8",
                "10.2",
                "10.3",
                "10.5",
                "900",
                "9270.0",
                "3",
                "1.10",
                "1",
                "-1.90",
                "0",
            ],
        ],
        field_names=K_HISTORY_DAILY_FIELDS,
    )

    table = k_history_daily_response_to_table(response)

    assert table.schema.field("isST").type == pa.bool_()
    assert table["isST"].to_pylist() == [True, False]


def test_decode_compressed_login_response() -> None:
    response = decode_response(
        response_message(
            ["0", "", "login", "user"],
            response_code="96",
            compressed=True,
        )
    )

    assert response.response_code == "96"
    assert response.api_name == "login"
    assert response.error_code == "0"


def test_decode_response_rejects_bad_crc_and_malformed_records() -> None:
    valid_message = response_message(
        [
            "0",
            "",
            "query_stock_basic",
            "user",
            "1",
            "2",
            '{"record":[]}',
            "",
            "",
            ",".join(STOCK_BASIC_FIELDS),
        ]
    )
    bad_crc_message = (
        valid_message.rsplit(MESSAGE_SPLIT.encode(), 1)[0]
        + f"{MESSAGE_SPLIT}999999".encode()
        + MESSAGE_END
    )

    with pytest.raises(BaostockProtocolError, match="CRC mismatch"):
        decode_response(bad_crc_message)
    with pytest.raises(BaostockProtocolError, match="record JSON"):
        decode_response(
            response_message(
                [
                    "0",
                    "",
                    "query_stock_basic",
                    "user",
                    "1",
                    "2",
                    '{"not_record":[]}',
                    "",
                    "",
                    ",".join(STOCK_BASIC_FIELDS),
                ]
            )
        )
    with pytest.raises(BaostockProtocolError, match="page is invalid"):
        decode_response(
            response_message(
                [
                    "0",
                    "",
                    "query_stock_basic",
                    "user",
                    "x",
                    "2",
                    '{"record":[]}',
                    "",
                    "",
                    ",".join(STOCK_BASIC_FIELDS),
                ]
            )
        )


def test_aggregate_responses_combines_records_and_rejects_empty_input() -> None:
    first = baostock_response(records=[["a"]], field_names=["code"])
    second = baostock_response(records=[["b"]], field_names=["code"])

    combined = aggregate_responses([first, second])

    assert combined.records == [["a"], ["b"]]
    assert combined.field_names == ["code"]
    with pytest.raises(ValueError, match="empty"):
        aggregate_responses([])


def test_baostock_schema_converters_validate_columns_and_record_width() -> None:
    import pyarrow as pa

    response = baostock_response(
        api_name="query_stock_basic",
        records=[["sh.600000", "浦发银行"]],
        field_names=["code", "code_name"],
    )

    # 创建一个简单的 schema 用于测试
    test_schema = pa.schema(
        [
            pa.field("code", pa.string()),
            pa.field("code_name", pa.string()),
        ]
    )
    table = response_to_table(response, test_schema)
    assert table.column_names == ["code", "code_name"]

    with pytest.raises(BaostockProtocolError, match="returned 2 values for 6 fields"):
        stock_basic_response_to_table(response)
    with pytest.raises(BaostockProtocolError, match="returned 1 values"):
        response_to_table(
            baostock_response(records=[["only-one"]], field_names=["a", "b"]),
            pa.schema([pa.field("a", pa.string()), pa.field("b", pa.string())]),
        )

    empty_k_table = k_history_daily_response_to_table(
        baostock_response(
            api_name="query_history_k_data_plus",
            records=[],
            field_names=[],
        )
    )
    assert empty_k_table.column_names == K_HISTORY_DAILY_FIELDS
    assert empty_k_table.num_rows == 0


def test_query_history_daily_rejects_reversed_date_range() -> None:
    client = BaostockAioTcpClient(config=client_config())

    with pytest.raises(ValueError, match="start_date"):
        asyncio.run(
            client.query_history_k_data_plus_daily(
                "sh.600000",
                date(2026, 5, 9),
                date(2026, 5, 8),
            )
        )


def test_baostock_client_uses_configured_request_and_login_timeouts() -> None:
    config = replace(
        client_config(),
        request_timeout_seconds=7,
        login_timeout_seconds=3,
    )
    sender_responses = [
        baostock_response(api_name="login", field_names=[]),
        baostock_response(api_name="query_stock_basic"),
    ]
    sender = queued_baostock_client(sender_responses)[1]
    client = BaostockAioTcpClient(
        config=config,
        retry_policy=ExponentialBackoffPolicy(jitter=False, base_delay=0),
        max_attempts=1,
        send_once=sender,
    )

    asyncio.run(client.query_stock_basic())

    assert sender.timeout_seconds == [3, 7]


def test_request_api_refreshes_login_after_no_login_response() -> None:
    initial_login_success = baostock_response(api_name="login", field_names=[])
    success = baostock_response(api_name="query_stock_basic")
    no_login = baostock_response(
        api_name="query_stock_basic",
        error_code="10001001",
        error_message="not login",
    )
    refreshed_login_success = baostock_response(api_name="login", field_names=[])
    client, sender = queued_baostock_client(
        [initial_login_success, no_login, refreshed_login_success, success]
    )

    response = asyncio.run(client.query_stock_basic())

    assert response == success
    assert len(sender.payloads) == 4


def test_request_api_raises_for_persistent_auth_and_business_errors() -> None:
    initial_login_success = baostock_response(api_name="login", field_names=[])
    no_login = baostock_response(
        api_name="query_stock_basic",
        error_code="10001001",
        error_message="not login",
    )
    refreshed_login_success = baostock_response(api_name="login", field_names=[])
    client, _sender = queued_baostock_client(
        [initial_login_success, no_login, refreshed_login_success, no_login]
    )

    with pytest.raises(BaostockAuthenticationError, match="still reported not logged in"):
        asyncio.run(client.query_stock_basic())

    business_error_client, _business_error_sender = queued_baostock_client(
        [
            baostock_response(api_name="login", field_names=[]),
            baostock_response(
                api_name="query_stock_basic",
                error_code="500",
                error_message="business failed",
            ),
        ]
    )
    with pytest.raises(BaostockResponseError, match="business failed"):
        asyncio.run(business_error_client.query_stock_basic())


def test_client_start_retries_network_failures() -> None:
    client, sender = retrying_baostock_client(failures_before_success=2)

    asyncio.run(client.start())

    assert sender.send_count == 3

    failing_client, _failing_sender = retrying_baostock_client(failures_before_success=3)
    with pytest.raises(BaostockNetworkError, match="failed after 3 attempts"):
        asyncio.run(failing_client.start())


def test_connection_pool_logs_in_each_tcp_connection_before_data_request() -> None:
    async def run_scenario() -> list[list[str]]:
        recorder = _RecordingBaostockTcpServer(
            expected_connections=4,
            wait_for_expected_connections_before_data=True,
        )
        server = await asyncio.start_server(recorder.handle, "127.0.0.1", 0)
        try:
            port = cast(Any, server.sockets[0]).getsockname()[1]
            config = replace(
                client_config(max_connections=4),
                host="127.0.0.1",
                port=port,
            )
            async with BaostockAioTcpClient(
                config=config,
                retry_policy=ExponentialBackoffPolicy(jitter=False, base_delay=0),
                max_attempts=1,
            ) as client:
                await asyncio.gather(*(client.query_stock_basic() for _ in range(4)))
        finally:
            server.close()
            await server.wait_closed()

        return recorder.api_names_by_connection()

    sequences = asyncio.run(run_scenario())

    assert len(sequences) == 4
    assert sum(sequence.count("login") for sequence in sequences) == 4
    assert all(sequence[:2] == ["login", "query_stock_basic"] for sequence in sequences)


def test_no_login_response_refreshes_only_the_current_connection() -> None:
    async def run_scenario() -> list[list[str]]:
        recorder = _RecordingBaostockTcpServer(first_data_request_returns_no_login=True)
        server = await asyncio.start_server(recorder.handle, "127.0.0.1", 0)
        try:
            port = cast(Any, server.sockets[0]).getsockname()[1]
            config = replace(
                client_config(max_connections=1),
                host="127.0.0.1",
                port=port,
            )
            async with BaostockAioTcpClient(
                config=config,
                retry_policy=ExponentialBackoffPolicy(jitter=False, base_delay=0),
                max_attempts=1,
            ) as client:
                await client.query_stock_basic()
        finally:
            server.close()
            await server.wait_closed()

        return recorder.api_names_by_connection()

    assert asyncio.run(run_scenario()) == [
        ["login", "query_stock_basic", "login", "query_stock_basic"]
    ]


def test_empty_k_history_table_uses_daily_schema() -> None:
    table = assets.empty_k_history_table()

    assert table.column_names == K_HISTORY_DAILY_FIELDS
    assert table.num_rows == 0


def test_fetch_stock_basic_table_uses_client_and_converts_response() -> None:
    factory = FakeBaostockClientFactory()

    table, metadata = asyncio.run(assets.fetch_stock_basic_table(factory))

    assert table.column_names == STOCK_BASIC_FIELDS
    assert table["code"].to_pylist() == ["sh.600000"]
    assert factory.created_max_connections == [None]
    assert set(metadata) == {
        "baostock_client_start_seconds",
        "baostock_query_seconds",
        "table_convert_seconds",
        "baostock_client_close_seconds",
        "asset_function_seconds",
    }


def test_fetch_k_history_table_for_trade_date_filters_active_securities_and_builds_metadata() -> (
    None
):
    factory = FakeBaostockClientFactory()
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[
                ["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"],
                ["sh.000001", "上证指数", "1991-07-15", "", "2", "1"],
                ["sh.510300", "ETF", "2012-05-28", "", "5", "1"],
                ["sh.600001", "退市", "1999-01-01", "2000-01-01", "1", "0"],
            ],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    table, metadata = asyncio.run(
        assets.fetch_k_history_table_for_trade_date(
            stock_basic,
            date(2026, 5, 8),
            factory,
        )
    )

    assert factory.created_max_connections == [4]
    assert table.num_rows == 2
    assert table.column_names == K_HISTORY_DAILY_FIELDS
    assert metadata["candidate_security_count"] == 4
    assert metadata["selected_security_count"] == 2
    assert metadata["skipped_security_count"] == 2
    assert cast(Any, metadata["selected_security_types"]).data == ["1", "2"]
    assert cast(Any, metadata["allowed_security_types"]).data == ["1", "2"]
    assert metadata["requested_trade_date"] == "2026-05-08"
    assert metadata["max_connections"] == 4
    assert metadata["max_concurrent_security_requests"] == 4
    assert cast(FakeBaostockAssetClient, factory._client).history_calls == [
        ("sh.600000", date(2026, 5, 8), date(2026, 5, 8)),
        ("sh.000001", date(2026, 5, 8), date(2026, 5, 8)),
    ]


def test_fetch_k_history_table_for_trade_date_rejects_partial_security_failures() -> None:
    class PartiallyFailingBaostockClient(FakeBaostockAssetClient):
        async def query_history_k_data_plus_daily(
            self,
            code: str,
            start_date: date,
            end_date: date,
        ) -> Any:
            if code == "sz.159001":
                self.history_calls.append((code, start_date, end_date))
                msg = "remote rejected security"
                raise RuntimeError(msg)
            return await super().query_history_k_data_plus_daily(code, start_date, end_date)

    client = PartiallyFailingBaostockClient()
    factory = FakeBaostockClientFactory(client)
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[
                ["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"],
                ["sz.159001", "基金", "2005-01-01", "", "2", "1"],
            ],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    with pytest.raises(RuntimeError, match="requests failed for 1 securities"):
        asyncio.run(
            assets.fetch_k_history_table_for_trade_date(
                stock_basic,
                date(2026, 5, 8),
                factory,
            )
        )

    assert factory.created_max_connections == [4]
    assert [call[0] for call in client.history_calls] == ["sh.600000", "sz.159001"]


def test_baostock_client_factory_resource_defaults_to_single_connection() -> None:
    resource = BaostockClientFactoryResource(
        host="baostock.test",
        port=10030,
        username="user",
        password="password",
        connect_timeout_seconds=15,
        request_timeout_seconds=20,
        login_timeout_seconds=15,
        max_request_attempts=4,
    )

    assert resource.config().max_connections == 1
    assert resource.config(max_connections=2).max_connections == 2
    assert resource.config().connect_timeout_seconds == 15
    assert resource.config().request_timeout_seconds == 20
    assert resource.config().login_timeout_seconds == 15
    assert resource.config().max_request_attempts == 4


def test_fetch_k_history_table_for_trade_date_returns_empty_metadata_when_no_security_is_selected() -> (
    None
):
    factory = FakeBaostockClientFactory()
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["bj.430047", "北交所", "2020-01-01", "", "9", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    table, metadata = asyncio.run(
        assets.fetch_k_history_table_for_trade_date(
            stock_basic,
            date(2026, 5, 8),
            factory,
        )
    )

    assert table.num_rows == 0
    assert table.column_names == K_HISTORY_DAILY_FIELDS
    assert factory.created_max_connections == []
    assert metadata["candidate_security_count"] == 1
    assert metadata["selected_security_count"] == 0
    assert metadata["skipped_security_count"] == 1


def test_fetch_k_history_range_backfill_requests_security_ranges_and_splits_dates() -> None:
    class RangeBaostockClient(FakeBaostockAssetClient):
        async def query_history_k_data_plus_daily(
            self,
            code: str,
            start_date: date,
            end_date: date,
        ) -> Any:
            self.history_calls.append((code, start_date, end_date))
            return baostock_response(
                api_name="query_history_k_data_plus",
                records=[
                    [
                        start_date.isoformat(),
                        code,
                        "1",
                        "2",
                        "1",
                        "2",
                        "1",
                        "100",
                        "200",
                        "3",
                        "1.0",
                        "1",
                        "10.0",
                        "0",
                    ]
                ],
                field_names=K_HISTORY_DAILY_FIELDS,
            )

    client = RangeBaostockClient()
    factory = FakeBaostockClientFactory(client)
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[
                ["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"],
                ["sh.000001", "上证指数", "2026-01-05", "", "2", "1"],
                ["sh.510300", "ETF", "2012-05-28", "", "5", "1"],
            ],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    result = asyncio.run(
        assets.baostock_services.fetch_k_history_tables_for_trade_date_range(
            stock_basic,
            date(2026, 1, 2),
            date(2026, 1, 5),
            [date(2026, 1, 2), date(2026, 1, 5)],
            factory,
        )
    )

    assert client.history_calls == [
        ("sh.600000", date(2026, 1, 2), date(2026, 1, 5)),
        ("sh.000001", date(2026, 1, 5), date(2026, 1, 5)),
    ]
    assert sorted(result.tables) == ["2026-01-02", "2026-01-05"]
    assert result.tables["2026-01-02"].num_rows == 1
    assert result.tables["2026-01-05"].num_rows == 1
    assert result.metadata["selected_security_count"] == 2
    assert cast(Any, result.metadata["empty_partition_keys"]).data == []
    assert result.metadata["duplicate_key_count"] == 0


def test_fetch_k_history_range_backfill_writes_empty_tables_for_empty_trade_dates() -> None:
    factory = FakeBaostockClientFactory(FakeBaostockAssetClient())
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    result = asyncio.run(
        assets.baostock_services.fetch_k_history_tables_for_trade_date_range(
            stock_basic,
            date(2026, 1, 2),
            date(2026, 1, 5),
            [date(2026, 1, 2), date(2026, 1, 5)],
            factory,
        )
    )

    assert result.tables["2026-01-02"].num_rows == 1
    assert result.tables["2026-01-05"].num_rows == 0
    assert cast(Any, result.metadata["empty_partition_keys"]).data == ["2026-01-05"]


def test_fetch_k_history_range_backfill_skips_when_no_target_trade_dates() -> None:
    client = FakeBaostockAssetClient()
    factory = FakeBaostockClientFactory(client)
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    result = asyncio.run(
        assets.baostock_services.fetch_k_history_tables_for_trade_date_range(
            stock_basic,
            date(2026, 1, 3),
            date(2026, 1, 4),
            [],
            factory,
        )
    )

    assert result.tables == {}
    assert client.history_calls == []
    assert factory.created_max_connections == []
    assert result.metadata["processed_trade_date_count"] == 0


def test_fetch_k_history_range_backfill_rejects_duplicate_keys() -> None:
    class DuplicateBaostockClient(FakeBaostockAssetClient):
        async def query_history_k_data_plus_daily(
            self,
            code: str,
            start_date: date,
            end_date: date,
        ) -> Any:
            self.history_calls.append((code, start_date, end_date))
            record = [
                start_date.isoformat(),
                code,
                "1",
                "2",
                "1",
                "2",
                "1",
                "100",
                "200",
                "3",
                "1.0",
                "1",
                "10.0",
                "0",
            ]
            return baostock_response(
                api_name="query_history_k_data_plus",
                records=[record, record],
                field_names=K_HISTORY_DAILY_FIELDS,
            )

    factory = FakeBaostockClientFactory(DuplicateBaostockClient())
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    with pytest.raises(RuntimeError, match="duplicate keys"):
        asyncio.run(
            assets.baostock_services.fetch_k_history_tables_for_trade_date_range(
                stock_basic,
                date(2026, 1, 2),
                date(2026, 1, 2),
                [date(2026, 1, 2)],
                factory,
            )
        )


def test_fetch_k_history_range_backfill_rejects_partial_security_failures() -> None:
    class PartiallyFailingBaostockClient(FakeBaostockAssetClient):
        async def query_history_k_data_plus_daily(
            self,
            code: str,
            start_date: date,
            end_date: date,
        ) -> Any:
            if code == "sh.000001":
                self.history_calls.append((code, start_date, end_date))
                msg = "remote rejected security"
                raise RuntimeError(msg)
            return await super().query_history_k_data_plus_daily(code, start_date, end_date)

    client = PartiallyFailingBaostockClient()
    factory = FakeBaostockClientFactory(client)
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[
                ["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"],
                ["sh.000001", "上证指数", "1991-07-15", "", "2", "1"],
            ],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    with pytest.raises(RuntimeError, match="requests failed for 1 securities"):
        asyncio.run(
            assets.baostock_services.fetch_k_history_tables_for_trade_date_range(
                stock_basic,
                date(2026, 1, 2),
                date(2026, 1, 5),
                [date(2026, 1, 2), date(2026, 1, 5)],
                factory,
            )
        )

    assert [call[0] for call in client.history_calls] == ["sh.600000", "sh.000001"]


def test_fetch_k_history_range_backfill_circuit_breaker_stops_network_failures() -> None:
    class NetworkFailingBaostockClient(FakeBaostockAssetClient):
        async def query_history_k_data_plus_daily(
            self,
            code: str,
            start_date: date,
            end_date: date,
        ) -> Any:
            self.history_calls.append((code, start_date, end_date))
            msg = "tcp timeout"
            raise BaostockNetworkError(msg)

    client = NetworkFailingBaostockClient()
    factory = FakeBaostockClientFactory(client)
    records = [
        [f"sh.{600000 + index:06d}", f"证券{index}", "1999-11-10", "", "1", "1"]
        for index in range(40)
    ]
    stock_basic = stock_basic_response_to_table(
        baostock_response(records=records, field_names=STOCK_BASIC_FIELDS)
    )

    with pytest.raises(RuntimeError, match="circuit breaker stopped scheduling"):
        asyncio.run(
            assets.baostock_services.fetch_k_history_tables_for_trade_date_range(
                stock_basic,
                date(2026, 1, 2),
                date(2026, 1, 2),
                [date(2026, 1, 2)],
                factory,
            )
        )

    assert len(client.history_calls) < 40


def test_fetch_k_history_range_backfill_rejects_non_trade_date_rows() -> None:
    class NonTradeDateBaostockClient(FakeBaostockAssetClient):
        async def query_history_k_data_plus_daily(
            self,
            code: str,
            start_date: date,
            end_date: date,
        ) -> Any:
            self.history_calls.append((code, start_date, end_date))
            return baostock_response(
                api_name="query_history_k_data_plus",
                records=[
                    [
                        "2026-01-03",
                        code,
                        "1",
                        "2",
                        "1",
                        "2",
                        "1",
                        "100",
                        "200",
                        "3",
                        "1.0",
                        "1",
                        "10.0",
                        "0",
                    ]
                ],
                field_names=K_HISTORY_DAILY_FIELDS,
            )

    factory = FakeBaostockClientFactory(NonTradeDateBaostockClient())
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    with pytest.raises(RuntimeError, match="outside the requested trade-date set"):
        asyncio.run(
            assets.baostock_services.fetch_k_history_tables_for_trade_date_range(
                stock_basic,
                date(2026, 1, 2),
                date(2026, 1, 2),
                [date(2026, 1, 2)],
                factory,
            )
        )


def test_range_backfill_records_cutoff_metadata_and_skips_after_cutoff(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class FakeTradeCalendarReader:
        @classmethod
        def from_s3_config(cls, config: object) -> FakeTradeCalendarReader:
            return cls()

        def read_trade_dates(self) -> set[date]:
            return {date(2026, 1, 2), date(2026, 1, 5)}

    class FakeDatasetService:
        written_partition_keys: list[str] = []

        def __init__(self, *, s3_config: object) -> None:
            self.s3_config = s3_config

        def existing_partition_keys(
            self,
            location: DatasetLocation,
            *,
            partition_keys: list[str],
            partition_key_name: str,
        ) -> list[str]:
            return []

        def write_partitioned(
            self,
            location: DatasetLocation,
            tables: dict[str, object],
            options: DatasetWriteOptions,
        ) -> DatasetWriteResult:
            self.__class__.written_partition_keys = sorted(tables)
            return DatasetWriteResult([], 1, 14, {"2026-01-02": 1})

        def object_keys(self, result: DatasetWriteResult) -> list[str]:
            return []

        def metadata(
            self,
            *,
            result: DatasetWriteResult,
            options: DatasetWriteOptions,
        ) -> dict[str, object]:
            return {
                "row_count": result.row_count,
                "column_count": result.column_count,
                "partition_row_counts": dg.MetadataValue.json(result.partition_row_counts),
                "empty_partition_keys": dg.MetadataValue.json(result.empty_partition_keys),
            }

    monkeypatch.setattr(assets, "S3TradeCalendarReader", FakeTradeCalendarReader)
    monkeypatch.setattr(assets, "S3DatasetService", FakeDatasetService)
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    result = asyncio.run(
        assets._materialize_daily_kline_partition_selection(
            cast(
                dg.AssetExecutionContext,
                _FakeAssetContext(["2026-01-02", "2026-01-05"]),
            ),
            config=assets.BaostockDailyKlineRunConfig(
                cutoff_trade_date="2026-01-02",
            ),
            stock_basic=stock_basic,
            s3_settings=S3SettingsResource(
                endpoint="http://localhost:9000",
                bucket="bucket",
                access_key="access",
                secret_key="secret",
            ),
            baostock_client_factory=cast(
                BaostockClientFactoryResource,
                FakeBaostockClientFactory(),
            ),
        )
    )

    assert FakeDatasetService.written_partition_keys == ["2026-01-02"]
    assert result.metadata["request_end_date"] == "2026-01-02"
    assert result.metadata["cutoff_trade_date"] == "2026-01-02"
    assert result.metadata["effective_cutoff_trade_date"] == "2026-01-02"
    assert result.metadata["skipped_after_cutoff_partition_count"] == 1
    assert cast(Any, result.metadata["skipped_after_cutoff_partition_keys_sample"]).data == [
        "2026-01-05"
    ]


def test_range_backfill_rejects_cutoff_after_partition_range() -> None:
    with pytest.raises(RuntimeError, match="cutoff_trade_date cannot be later"):
        asyncio.run(
            assets._materialize_daily_kline_partition_selection(
                cast(dg.AssetExecutionContext, _FakeAssetContext(["2026-01-02"])),
                config=assets.BaostockDailyKlineRunConfig(
                    cutoff_trade_date="2026-01-05",
                ),
                stock_basic=assets.empty_k_history_table(),
                s3_settings=S3SettingsResource(
                    endpoint="http://localhost:9000",
                    bucket="bucket",
                    access_key="access",
                    secret_key="secret",
                ),
                baostock_client_factory=cast(
                    BaostockClientFactoryResource,
                    FakeBaostockClientFactory(),
                ),
            )
        )


def test_range_backfill_refuses_existing_partitions_by_default(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class FakeTradeCalendarReader:
        @classmethod
        def from_s3_config(cls, config: object) -> FakeTradeCalendarReader:
            return cls()

        def read_trade_dates(self) -> set[date]:
            return {date(2026, 1, 2)}

    class FakeDatasetService:
        write_calls = 0

        def __init__(self, *, s3_config: object) -> None:
            self.s3_config = s3_config

        def existing_partition_keys(
            self,
            location: DatasetLocation,
            *,
            partition_keys: list[str],
            partition_key_name: str,
        ) -> list[str]:
            return ["2026-01-02"]

        def write_partitioned(
            self,
            location: DatasetLocation,
            tables: object,
            options: DatasetWriteOptions,
        ) -> DatasetWriteResult:
            self.__class__.write_calls += 1
            return DatasetWriteResult([], 0, 0)

    monkeypatch.setattr(assets, "S3TradeCalendarReader", FakeTradeCalendarReader)
    monkeypatch.setattr(assets, "S3DatasetService", FakeDatasetService)
    context = _FakeAssetContext(["2026-01-02"])
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    with pytest.raises(RuntimeError, match="refuses to overwrite"):
        asyncio.run(
            assets._materialize_daily_kline_partition_selection(
                cast(dg.AssetExecutionContext, context),
                config=assets.BaostockDailyKlineRunConfig(),
                stock_basic=stock_basic,
                s3_settings=S3SettingsResource(
                    endpoint="http://localhost:9000",
                    bucket="bucket",
                    access_key="access",
                    secret_key="secret",
                ),
                baostock_client_factory=cast(
                    BaostockClientFactoryResource,
                    FakeBaostockClientFactory(),
                ),
            )
        )

    assert FakeDatasetService.write_calls == 0


def test_range_backfill_allows_explicit_overwrite_and_records_metadata(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class FakeTradeCalendarReader:
        @classmethod
        def from_s3_config(cls, config: object) -> FakeTradeCalendarReader:
            return cls()

        def read_trade_dates(self) -> set[date]:
            return {date(2026, 1, 2)}

    class FakeDatasetService:
        written_partition_keys: list[str] = []

        def __init__(self, *, s3_config: object) -> None:
            self.s3_config = s3_config

        def existing_partition_keys(
            self,
            location: DatasetLocation,
            *,
            partition_keys: list[str],
            partition_key_name: str,
        ) -> list[str]:
            return ["2026-01-02"]

        def write_partitioned(
            self,
            location: DatasetLocation,
            tables: dict[str, object],
            options: DatasetWriteOptions,
        ) -> DatasetWriteResult:
            self.__class__.written_partition_keys = sorted(tables)
            return DatasetWriteResult(
                [
                    "bucket/source/baostock__query_history_k_data_plus_daily/trade_date=2026-01-02/000000_0.parquet"
                ],
                1,
                14,
                {"2026-01-02": 1},
            )

        def object_keys(self, result: DatasetWriteResult) -> list[str]:
            return result.object_keys("bucket")

        def metadata(
            self,
            *,
            result: DatasetWriteResult,
            options: DatasetWriteOptions,
        ) -> dict[str, object]:
            return {
                "row_count": result.row_count,
                "column_count": result.column_count,
                "partition_row_counts": dg.MetadataValue.json(result.partition_row_counts),
                "empty_partition_keys": dg.MetadataValue.json(result.empty_partition_keys),
            }

    monkeypatch.setattr(assets, "S3TradeCalendarReader", FakeTradeCalendarReader)
    monkeypatch.setattr(assets, "S3DatasetService", FakeDatasetService)
    context = _FakeAssetContext(["2026-01-02"])
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    result = asyncio.run(
        assets._materialize_daily_kline_partition_selection(
            cast(dg.AssetExecutionContext, context),
            config=assets.BaostockDailyKlineRunConfig(
                overwrite_existing_partitions=True,
            ),
            stock_basic=stock_basic,
            s3_settings=S3SettingsResource(
                endpoint="http://localhost:9000",
                bucket="bucket",
                access_key="access",
                secret_key="secret",
            ),
            baostock_client_factory=cast(
                BaostockClientFactoryResource,
                FakeBaostockClientFactory(),
            ),
        )
    )

    assert FakeDatasetService.written_partition_keys == ["2026-01-02"]
    assert result.metadata["overwrite_existing_partitions"] is True
    assert cast(Any, result.metadata["overwritten_partition_keys"]).data == ["2026-01-02"]


def test_range_backfill_reports_partial_write_repair_context(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class FakeTradeCalendarReader:
        @classmethod
        def from_s3_config(cls, config: object) -> FakeTradeCalendarReader:
            return cls()

        def read_trade_dates(self) -> set[date]:
            return {date(2026, 1, 2), date(2026, 1, 5)}

    class FakeDatasetService:
        def __init__(self, *, s3_config: object) -> None:
            self.s3_config = s3_config

        def existing_partition_keys(
            self,
            location: DatasetLocation,
            *,
            partition_keys: list[str],
            partition_key_name: str,
        ) -> list[str]:
            return []

        def write_partitioned(
            self,
            location: DatasetLocation,
            tables: dict[str, object],
            options: DatasetWriteOptions,
        ) -> DatasetWriteResult:
            raise DatasetPartitionWriteError(
                attempted_partition_keys=["2026-01-02", "2026-01-05"],
                written_partition_keys=["2026-01-02"],
                failed_partition_keys=["2026-01-05"],
                written_paths=[],
                cause=RuntimeError("s3 write failed"),
            )

    monkeypatch.setattr(assets, "S3TradeCalendarReader", FakeTradeCalendarReader)
    monkeypatch.setattr(assets, "S3DatasetService", FakeDatasetService)
    context = _FakeAssetContext(["2026-01-02", "2026-01-05"])
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    with pytest.raises(RuntimeError, match="written_partition_keys=\\['2026-01-02'\\]"):
        asyncio.run(
            assets._materialize_daily_kline_partition_selection(
                cast(dg.AssetExecutionContext, context),
                config=assets.BaostockDailyKlineRunConfig(),
                stock_basic=stock_basic,
                s3_settings=S3SettingsResource(
                    endpoint="http://localhost:9000",
                    bucket="bucket",
                    access_key="access",
                    secret_key="secret",
                ),
                baostock_client_factory=cast(
                    BaostockClientFactoryResource,
                    FakeBaostockClientFactory(),
                ),
            )
        )
    assert context.log.errors
    assert context.log.errors[0][1] == (
        ["2026-01-02", "2026-01-05"],
        ["2026-01-02"],
        ["2026-01-05"],
    )


def test_multi_partition_selection_runs_without_mode(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class FakeTradeCalendarReader:
        @classmethod
        def from_s3_config(cls, config: object) -> FakeTradeCalendarReader:
            return cls()

        def read_trade_dates(self) -> set[date]:
            return {date(2026, 1, 2), date(2026, 1, 5)}

    class FakeDatasetService:
        written_partition_keys: list[str] = []

        def __init__(self, *, s3_config: object) -> None:
            self.s3_config = s3_config

        def existing_partition_keys(
            self,
            location: DatasetLocation,
            *,
            partition_keys: list[str],
            partition_key_name: str,
        ) -> list[str]:
            return []

        def write_partitioned(
            self,
            location: DatasetLocation,
            tables: dict[str, object],
            options: DatasetWriteOptions,
        ) -> DatasetWriteResult:
            self.__class__.written_partition_keys = sorted(tables)
            return DatasetWriteResult([], 2, 14, {"2026-01-02": 1, "2026-01-05": 1})

        def object_keys(self, result: DatasetWriteResult) -> list[str]:
            return []

        def metadata(
            self,
            *,
            result: DatasetWriteResult,
            options: DatasetWriteOptions,
        ) -> dict[str, object]:
            return {
                "row_count": result.row_count,
                "column_count": result.column_count,
                "partition_row_counts": dg.MetadataValue.json(result.partition_row_counts),
                "empty_partition_keys": dg.MetadataValue.json(result.empty_partition_keys),
            }

    monkeypatch.setattr(assets, "S3TradeCalendarReader", FakeTradeCalendarReader)
    monkeypatch.setattr(assets, "S3DatasetService", FakeDatasetService)
    client = FakeBaostockAssetClient()
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    result = asyncio.run(
        assets._materialize_daily_kline_partition_selection(
            cast(
                dg.AssetExecutionContext,
                _FakeAssetContext(["2026-01-02", "2026-01-05"]),
            ),
            config=assets.BaostockDailyKlineRunConfig(),
            stock_basic=stock_basic,
            s3_settings=S3SettingsResource(
                endpoint="http://localhost:9000",
                bucket="bucket",
                access_key="access",
                secret_key="secret",
            ),
            baostock_client_factory=cast(
                BaostockClientFactoryResource,
                FakeBaostockClientFactory(client),
            ),
        )
    )

    assert FakeDatasetService.written_partition_keys == ["2026-01-02", "2026-01-05"]
    assert client.history_calls == [("sh.600000", date(2026, 1, 2), date(2026, 1, 5))]
    assert result.metadata["request_start_date"] == "2026-01-02"
    assert result.metadata["request_end_date"] == "2026-01-05"


class _FakeAssetContext:
    def __init__(self, partition_keys: list[str]) -> None:
        self.partition_keys = partition_keys
        self.asset_key = assets.baostock__query_history_k_data_plus_daily.key
        self.log = _FakeLog()


class _FakeLog:
    def __init__(self) -> None:
        self.errors: list[tuple[str, tuple[object, ...]]] = []

    def error(self, message: str, *args: object) -> None:
        self.errors.append((message, args))


class _RecordingBaostockTcpServer:
    def __init__(
        self,
        *,
        expected_connections: int = 1,
        wait_for_expected_connections_before_data: bool = False,
        first_data_request_returns_no_login: bool = False,
    ) -> None:
        self.expected_connections = expected_connections
        self.wait_for_expected_connections_before_data = wait_for_expected_connections_before_data
        self.first_data_request_returns_no_login = first_data_request_returns_no_login
        self.payloads_by_connection: list[list[bytes]] = []
        self._expected_connections_seen = asyncio.Event()
        self._data_request_count = 0

    async def handle(
        self,
        reader: asyncio.StreamReader,
        writer: asyncio.StreamWriter,
    ) -> None:
        payloads: list[bytes] = []
        self.payloads_by_connection.append(payloads)
        if len(self.payloads_by_connection) >= self.expected_connections:
            self._expected_connections_seen.set()

        try:
            while True:
                payload = await reader.readuntil(b"\n")
                payloads.append(payload)
                api_name = _payload_api_name(payload)
                if api_name == "login":
                    writer.write(_login_success_message())
                elif api_name == "query_stock_basic":
                    if self.wait_for_expected_connections_before_data:
                        await asyncio.wait_for(self._expected_connections_seen.wait(), timeout=2)
                    self._data_request_count += 1
                    if self.first_data_request_returns_no_login and self._data_request_count == 1:
                        writer.write(_stock_basic_no_login_message())
                    else:
                        writer.write(_stock_basic_success_message())
                else:
                    raise AssertionError(f"Unexpected BaoStock API request: {api_name}")
                await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionResetError):
            return
        finally:
            writer.close()
            with contextlib.suppress(OSError):
                await writer.wait_closed()

    def api_names_by_connection(self) -> list[list[str]]:
        return [
            [_payload_api_name(payload) for payload in payloads]
            for payloads in self.payloads_by_connection
        ]


def _payload_api_name(payload: bytes) -> str:
    text = payload.decode("utf-8", errors="ignore")
    if "login" in text:
        return "login"
    if "query_stock_basic" in text:
        return "query_stock_basic"
    if "query_history_k_data_plus" in text:
        return "query_history_k_data_plus"
    return "unknown"


def _login_success_message() -> bytes:
    return response_message(["0", "", "login", "user"])


def _stock_basic_success_message() -> bytes:
    return response_message(
        [
            "0",
            "",
            "query_stock_basic",
            "user",
            "1",
            "1000",
            '{"record":[]}',
            "",
            "",
            ",".join(STOCK_BASIC_FIELDS),
        ]
    )


def _stock_basic_no_login_message() -> bytes:
    return response_message(
        [
            "10001001",
            "not login",
            "query_stock_basic",
            "user",
            "1",
            "1000",
            '{"record":[]}',
            "",
            "",
            ",".join(STOCK_BASIC_FIELDS),
        ]
    )
