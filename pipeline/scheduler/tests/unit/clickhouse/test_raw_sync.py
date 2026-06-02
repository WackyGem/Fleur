from __future__ import annotations

from collections.abc import Sequence
from dataclasses import replace

import pytest
from scheduler.defs.clickhouse import sql
from scheduler.defs.clickhouse.raw_sync import RawSyncRequest, RawSyncService
from scheduler.defs.clickhouse.specs import BAOSTOCK_DAILY_K_SPEC


class FakeQueryResult:
    def __init__(self, rows: Sequence[Sequence[object]]) -> None:
        self._rows = rows

    @property
    def result_rows(self) -> Sequence[Sequence[object]]:
        return self._rows


class FakeClickHouseClient:
    def __init__(self) -> None:
        self.commands: list[str] = []
        self.queries: list[str] = []
        self.insert_error: RuntimeError | None = None
        self.partition_validation_row: tuple[int, int, int] = (5, 2026, 2026)
        self.raw_year_partition_count = 5
        self.unique_count = 5_000

    @property
    def server_version(self) -> str:
        return "25.1.1"

    def ping(self) -> bool:
        return True

    def command(
        self,
        cmd: str,
        *,
        settings: dict[str, object] | None = None,
    ) -> object:
        del settings
        self.commands.append(cmd)
        if cmd.startswith("INSERT INTO") and self.insert_error is not None:
            raise self.insert_error
        return None

    def query(
        self,
        query: str,
        *,
        settings: dict[str, object] | None = None,
    ) -> FakeQueryResult:
        del settings
        self.queries.append(query)
        if "FROM system.columns" in query:
            return FakeQueryResult(
                [
                    (column.name, column.clickhouse_type)
                    for column in BAOSTOCK_DAILY_K_SPEC.table_columns
                ]
            )
        if "min(`year`)" in query:
            return FakeQueryResult([self.partition_validation_row])
        if "uniq(`code`)" in query:
            return FakeQueryResult([(self.unique_count,)])
        if "WHERE `year` = 2026" in query:
            return FakeQueryResult([(self.raw_year_partition_count,)])
        raise AssertionError(f"Unexpected query: {query}")

    def close(self) -> None:
        return None


def test_raw_sync_success_path_replaces_partition_after_validation() -> None:
    client = FakeClickHouseClient()

    result = RawSyncService(client).sync(_request(partition_key="2026"))

    assert result.loaded_row_count == 5
    assert result.raw_row_count_after_replace == 5
    assert result.s3_object_key == (
        "source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet"
    )
    assert result.metadata()["clickhouse_database"] == "fleur_raw"
    assert any(
        command.startswith("CREATE DATABASE IF NOT EXISTS `fleur_raw`")
        for command in client.commands
    )
    assert any("REPLACE PARTITION 2026" in command for command in client.commands)
    assert client.commands.index(
        next(command for command in client.commands if command.startswith("INSERT INTO"))
    ) < client.commands.index(
        next(command for command in client.commands if "REPLACE PARTITION" in command)
    )


def test_raw_sync_does_not_replace_when_staging_insert_fails() -> None:
    client = FakeClickHouseClient()
    client.insert_error = RuntimeError("insert failed")

    with pytest.raises(RuntimeError, match="insert failed"):
        RawSyncService(client).sync(_request(partition_key="2026"))

    assert not any("REPLACE PARTITION" in command for command in client.commands)


def test_raw_sync_does_not_replace_when_partition_validation_fails() -> None:
    client = FakeClickHouseClient()
    client.partition_validation_row = (5, 2025, 2026)

    with pytest.raises(RuntimeError, match="contains partition range"):
        RawSyncService(client).sync(_request(partition_key="2026"))

    assert not any("REPLACE PARTITION" in command for command in client.commands)


def test_raw_sync_allows_empty_year_partition_when_spec_allows_empty() -> None:
    client = FakeClickHouseClient()
    client.partition_validation_row = (0, 0, 0)
    client.raw_year_partition_count = 0
    spec = replace(BAOSTOCK_DAILY_K_SPEC, allow_empty=True)

    result = RawSyncService(client).sync(_request(partition_key="2026", spec=spec))

    assert result.loaded_row_count == 0
    assert result.raw_row_count_after_replace == 0
    assert any("REPLACE PARTITION 2026" in command for command in client.commands)


def test_raw_sync_rejects_high_cardinality_low_cardinality_column() -> None:
    client = FakeClickHouseClient()
    client.unique_count = 10_001

    with pytest.raises(RuntimeError, match="above LowCardinality limit"):
        RawSyncService(client).sync(_request(partition_key="2026"))

    assert not any("REPLACE PARTITION" in command for command in client.commands)


def test_raw_sync_requires_four_digit_year_partition() -> None:
    client = FakeClickHouseClient()

    with pytest.raises(RuntimeError, match="four-digit year"):
        RawSyncService(client).sync(_request(partition_key="20260"))


def _request(
    *,
    partition_key: str,
    spec=BAOSTOCK_DAILY_K_SPEC,
) -> RawSyncRequest:
    return RawSyncRequest(
        spec=spec,
        s3_input=sql.ClickHouseS3InputConfig(
            endpoint="http://127.0.0.1:9000",
            bucket="bucket",
            access_key="access",
            secret_key="secret",
        ),
        partition_key=partition_key,
    )
