from __future__ import annotations

import asyncio
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
from tests.fakes.baostock import (
    FakeBaostockClientFactory,
    baostock_response,
    client_config,
    queued_baostock_client,
    response_message,
    retrying_baostock_client,
)
from tests.fakes.dagster import FakeAssetContext

FAKE_BAOSTOCK_DAILY_K_ASSET_KEY = dg.AssetKey(["baostock", "query_history_k_data_plus_daily"])


def fake_asset_context(partition_keys: list[str]) -> FakeAssetContext:
    return FakeAssetContext(
        partition_keys=partition_keys,
        asset_key=FAKE_BAOSTOCK_DAILY_K_ASSET_KEY,
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


def test_build_year_ranges_for_full_year_and_refresh_until() -> None:
    trade_dates = {date(2025, 1, 2), date(2026, 5, 8)}

    assert assets.build_year_ranges(
        cast(Any, fake_asset_context(["2025", "2026"])),
        assets.KLineDailyYearConfig(),
        trade_dates,
    ) == {
        "2025": (date(2025, 1, 1), date(2025, 12, 31)),
        "2026": (date(2026, 1, 1), date(2026, 12, 31)),
    }
    assert assets.build_year_ranges(
        cast(Any, fake_asset_context(["2026"])),
        assets.KLineDailyYearConfig(refresh_until_trade_date="2026-05-08"),
        trade_dates,
    ) == {"2026": (date(2026, 1, 1), date(2026, 5, 8))}


def test_build_year_ranges_rejects_invalid_partition_requests() -> None:
    with pytest.raises(RuntimeError, match="requires at least one"):
        assets.build_year_ranges(
            cast(Any, fake_asset_context([])),
            assets.KLineDailyYearConfig(),
            {date(2026, 5, 8)},
        )
    with pytest.raises(ValueError, match="single year partition"):
        assets.build_year_ranges(
            cast(Any, fake_asset_context(["2025", "2026"])),
            assets.KLineDailyYearConfig(refresh_until_trade_date="2026-05-08"),
            {date(2026, 5, 8)},
        )
    with pytest.raises(ValueError, match="is not a trade date"):
        assets.build_year_ranges(
            cast(Any, fake_asset_context(["2026"])),
            assets.KLineDailyYearConfig(refresh_until_trade_date="2026-05-09"),
            {date(2026, 5, 8)},
        )
    with pytest.raises(ValueError, match="is not in partition"):
        assets.build_year_ranges(
            cast(Any, fake_asset_context(["2025"])),
            assets.KLineDailyYearConfig(refresh_until_trade_date="2026-05-08"),
            {date(2026, 5, 8)},
        )
    with pytest.raises(ValueError, match="has no trade dates"):
        assets.build_year_ranges(
            cast(Any, fake_asset_context(["2024"])),
            assets.KLineDailyYearConfig(),
            {date(2026, 5, 8)},
        )


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


def test_fetch_k_history_tables_filters_active_securities_and_builds_metadata() -> None:
    factory = FakeBaostockClientFactory()
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[
                ["sh.600000", "浦发银行", "1999-11-10", "", "1", "1"],
                ["sz.159001", "基金", "2005-01-01", "", "2", "1"],
                ["sh.600001", "退市", "1999-01-01", "2000-01-01", "1", "0"],
            ],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    tables, metadata = asyncio.run(
        assets.fetch_k_history_tables(
            stock_basic,
            {"2026": (date(2026, 1, 1), date(2026, 12, 31))},
            factory,
        )
    )

    assert factory.created_max_connections == [30]
    assert set(tables) == {"2026"}
    assert tables["2026"].num_rows == 2
    assert tables["2026"].column_names == K_HISTORY_DAILY_FIELDS
    assert metadata["candidate_security_count"] == 3
    assert cast(Any, metadata["selected_security_count"]).data == {"2026": 2}
    assert cast(Any, metadata["skipped_security_count"]).data == {"2026": 1}
    assert cast(Any, metadata["selected_security_types"]).data == ["1", "2"]
    assert cast(Any, metadata["requested_ranges"]).data == {
        "2026": {"start_date": "2026-01-01", "end_date": "2026-12-31"}
    }


def test_fetch_k_history_tables_returns_empty_metadata_when_no_security_is_selected() -> None:
    factory = FakeBaostockClientFactory()
    stock_basic = stock_basic_response_to_table(
        baostock_response(
            records=[["bj.430047", "北交所", "2020-01-01", "", "9", "1"]],
            field_names=STOCK_BASIC_FIELDS,
        )
    )

    tables, metadata = asyncio.run(
        assets.fetch_k_history_tables(
            stock_basic,
            {"2026": (date(2026, 1, 1), date(2026, 12, 31))},
            factory,
        )
    )

    assert tables == {}
    assert metadata["candidate_security_count"] == 1
    assert cast(Any, metadata["selected_security_count"]).data == {"2026": 0}
    assert cast(Any, metadata["skipped_security_count"]).data == {"2026": 1}
