from __future__ import annotations

import asyncio
import contextlib
from dataclasses import replace
from datetime import date
from typing import Any, cast

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

    with pytest.raises(RuntimeError, match="failure rate exceeded 0%"):
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
    )

    assert resource.config().max_connections == 1
    assert resource.config(max_connections=2).max_connections == 2


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
