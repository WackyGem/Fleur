from __future__ import annotations

import json
import threading
import time
from collections.abc import Sequence
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import fleur_contracts.clickhouse_layer_migration as migration
import pytest
from fleur_contracts.clickhouse_layer_migration import (
    LAYER_DATABASES,
    confirmation_token,
    execute_migration_command_plan,
    migration_command_plan_from_manifest,
    migration_commands_from_manifest,
    run_migrate,
    run_reset,
)


class FakeQueryResult:
    def __init__(self, rows: Sequence[Sequence[object]]) -> None:
        self._rows = rows

    @property
    def result_rows(self) -> Sequence[Sequence[object]]:
        return self._rows


class FakeClickHouseClient:
    def __init__(self) -> None:
        self.commands: list[str] = []
        self.databases = set(LAYER_DATABASES)

    def command(
        self,
        cmd: str,
        *,
        settings: dict[str, object] | None = None,
    ) -> object:
        del settings
        self.commands.append(cmd)
        prefix = "DROP DATABASE IF EXISTS `"
        if cmd.startswith(prefix) and cmd.endswith("`"):
            self.databases.discard(cmd.removeprefix(prefix).removesuffix("`"))
        return None

    def query(
        self,
        query: str,
        *,
        parameters: dict[str, object] | None = None,
        settings: dict[str, object] | None = None,
    ) -> FakeQueryResult:
        del query, settings
        if parameters is not None and "databases" in parameters:
            databases = parameters["databases"]
            assert isinstance(databases, list)
            return FakeQueryResult(
                [(database,) for database in databases if database in self.databases]
            )
        return FakeQueryResult([])

    def close(self) -> None:
        return None


def test_run_reset_requires_confirmation_token(tmp_path: Path) -> None:
    manifest_path = _write_manifest(tmp_path, _manifest())

    with pytest.raises(RuntimeError, match="Confirmation token"):
        run_reset(
            confirm="wrong-token",
            baseline_manifest_path=manifest_path,
            reports_dir=tmp_path,
            client=FakeClickHouseClient(),
            now=datetime(2026, 6, 2, tzinfo=UTC),
        )


def test_run_reset_drops_only_layer_databases(tmp_path: Path) -> None:
    manifest = _manifest()
    manifest_path = _write_manifest(tmp_path, manifest)
    client = FakeClickHouseClient()

    report = run_reset(
        confirm=confirmation_token(None, manifest),
        baseline_manifest_path=manifest_path,
        reports_dir=tmp_path,
        client=client,
        now=datetime(2026, 6, 2, tzinfo=UTC),
    )

    assert client.commands == [
        f"DROP DATABASE IF EXISTS `{database}`" for database in LAYER_DATABASES
    ]
    assert report.exists()
    assert "Confirmation token hash" in report.read_text(encoding="utf-8")


def test_run_reset_refuses_manifest_with_missing_objects(tmp_path: Path) -> None:
    manifest = _manifest()
    manifest["datasets"][0]["missing_objects"] = ["source/demo/000000_0.parquet"]
    manifest_path = _write_manifest(tmp_path, manifest)

    with pytest.raises(RuntimeError, match="missing S3 objects"):
        run_reset(
            confirm=confirmation_token(None, manifest),
            baseline_manifest_path=manifest_path,
            reports_dir=tmp_path,
            client=FakeClickHouseClient(),
            now=datetime(2026, 6, 2, tzinfo=UTC),
        )


def test_baseline_report_separates_historical_raw_from_migration_target() -> None:
    report = migration.render_baseline_report(
        timestamp=datetime(2026, 6, 2, tzinfo=UTC),
        databases={"raw": True, "fleur_raw": False},
        table_baseline=[
            {
                "dataset": "demo__yearly",
                "historical_target": "raw.demo__yearly",
                "migration_target": "fleur_raw.demo__yearly",
                "historical_raw_rows": 10,
                "schema_fingerprint": "abc123",
                "active_parts": 2,
                "partitions": ["2025", "2026"],
            }
        ],
        manifest={
            "raw_database": "fleur_raw",
            "datasets": [
                {
                    "dataset": "demo__yearly",
                    "partition_strategy": "year",
                    "objects": [
                        {"object_key": "source/demo__yearly/year=2025/000000_0.parquet"},
                        {"object_key": "source/demo__yearly/year=2026/000000_0.parquet"},
                    ],
                    "missing_objects": [],
                }
            ],
        },
        grants=["GRANT SELECT, CREATE DATABASE, DROP DATABASE ON *.* TO default"],
        working_tree="clean",
        confirmation_token="token123",
    )

    assert "| Dataset | Historical raw table | Migration target |" in report
    assert "| `demo__yearly` | `raw.demo__yearly` | `fleur_raw.demo__yearly` |" in report
    assert "| `demo__yearly` | `year` | 2 | 0 |" in report
    assert "- `clickhouse/raw/demo__yearly`" in report
    assert "Required privileges" in report
    assert "GRANT SELECT, CREATE DATABASE, DROP DATABASE" in report


def test_migration_commands_expand_snapshot_and_year_partitions() -> None:
    commands = migration_commands_from_manifest(_manifest())
    labels = [label for label, _, _ in commands]
    command_values = [command for _, _, command in commands]

    assert "raw sync demo__snapshot snapshot" in labels
    assert "raw sync demo__yearly partition 2025" in labels
    assert "raw sync demo__yearly partition 2026" in labels
    assert (
        "uv",
        "run",
        "dg",
        "launch",
        "--target-path",
        "scheduler",
        "--assets",
        "key:clickhouse/raw/demo__yearly",
        "--partition",
        "2026",
    ) in command_values
    assert all("clickhouse__raw_sync_all_job" not in command for command in command_values)


def test_migration_command_plan_groups_raw_sync_by_dataset() -> None:
    plan = migration_command_plan_from_manifest(_manifest())
    raw_sync_commands = [command for command in plan if command.phase == "raw_sync"]

    assert [command.raw_sync_group for command in raw_sync_commands] == [
        "demo__snapshot",
        "demo__yearly",
        "demo__yearly",
    ]


def test_allow_empty_year_manifest_uses_parquet_row_count_for_expected_partitions() -> None:
    manifest_dataset = {
        "dataset": "demo__yearly",
        "partition_strategy": "year",
        "expected_years": ["2024", "2025", "2026"],
        "objects": [
            {
                "object_key": "source/demo__yearly/year=2024/000000_0.parquet",
                "row_count": 0,
            },
            {
                "object_key": "source/demo__yearly/year=2025/000000_0.parquet",
                "row_count": 10,
            },
            {
                "object_key": "source/demo__yearly/year=2026/000000_0.parquet",
                "row_count": 0,
            },
        ],
    }

    assert migration._expected_present_years(manifest_dataset, allow_empty=True) == ["2025"]
    assert migration._expected_present_years(manifest_dataset, allow_empty=False) == [
        "2024",
        "2025",
        "2026",
    ]


def test_migration_runner_parallelizes_across_datasets_not_within_dataset() -> None:
    plan = migration_command_plan_from_manifest(_manifest_with_two_yearly_datasets())
    active_by_dataset: dict[str, int] = {}
    max_active_by_dataset: dict[str, int] = {}
    raw_sync_threads: set[int] = set()
    lock = threading.Lock()

    class Completed:
        returncode = 0
        stdout = ""
        stderr = ""

    def command_runner(
        command: list[str],
        *,
        cwd: Path,
        check: bool,
        capture_output: bool,
        text: bool,
    ) -> Completed:
        del cwd, check, capture_output, text
        dataset = _dataset_from_command(command)
        if dataset is None:
            return Completed()

        thread_id = threading.get_ident()
        with lock:
            active_by_dataset[dataset] = active_by_dataset.get(dataset, 0) + 1
            max_active_by_dataset[dataset] = max(
                max_active_by_dataset.get(dataset, 0),
                active_by_dataset[dataset],
            )
            raw_sync_threads.add(thread_id)
        time.sleep(0.01)
        with lock:
            active_by_dataset[dataset] -= 1
        return Completed()

    results = execute_migration_command_plan(
        plan,
        command_runner=command_runner,
        max_concurrent_raw_sync_datasets=2,
    )

    assert [result.returncode for result in results] == [0] * len(results)
    assert max_active_by_dataset == {"demo__yearly": 1, "other__yearly": 1}
    assert len(raw_sync_threads) == 2


def test_run_migrate_passes_manifest_to_raw_validation_and_writes_report(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    manifest = _manifest()
    manifest_path = _write_manifest(tmp_path, manifest)
    validate_manifest_paths: list[Path | None] = []
    validate_dbt_clients: list[Any] = []
    executed_commands: list[tuple[str, ...]] = []

    class Completed:
        returncode = 0
        stdout = "Started run 12345678-1234-1234-1234-123456789abc"
        stderr = ""

    def command_runner(
        command: list[str],
        *,
        cwd: Path,
        check: bool,
        capture_output: bool,
        text: bool,
    ) -> Completed:
        del cwd, check, capture_output, text
        executed_commands.append(tuple(command))
        return Completed()

    def validate_raw_spy(
        *,
        contract_root: Path,
        baseline_manifest_path: Path | None = None,
        client: Any = None,
    ) -> list[str]:
        del contract_root, client
        validate_manifest_paths.append(baseline_manifest_path)
        return []

    def validate_dbt_layer_spy(client: Any = None) -> list[str]:
        validate_dbt_clients.append(client)
        return []

    monkeypatch.setattr(migration, "validate_raw", validate_raw_spy)
    monkeypatch.setattr(migration, "validate_dbt_layer", validate_dbt_layer_spy)

    clickhouse_client = FakeClickHouseClient()
    report = run_migrate(
        confirm=confirmation_token(None, manifest),
        baseline_manifest_path=manifest_path,
        reports_dir=tmp_path,
        client=clickhouse_client,
        now=datetime(2026, 6, 2, tzinfo=UTC),
        command_runner=command_runner,
        max_concurrent_raw_sync_datasets=1,
    )

    assert validate_manifest_paths == [manifest_path]
    assert validate_dbt_clients == [clickhouse_client]
    assert executed_commands[0] == ("make", "dagster-home")
    report_text = report.read_text(encoding="utf-8")
    assert f"- Partition manifest: `{manifest_path}`" in report_text
    assert "- Reset report:" in report_text
    assert (
        "| dagster home initialization | success | "
        "12345678-1234-1234-1234-123456789abc | `make dagster-home` |"
    ) in report_text
    assert "- `validate_raw` passed." in report_text
    assert "- dbt layer validation passed." in report_text


def test_migration_commands_reject_empty_year_manifest() -> None:
    manifest = _manifest()
    manifest["datasets"][1]["expected_years"] = []

    with pytest.raises(RuntimeError, match="no expected years"):
        migration_commands_from_manifest(manifest)


def _manifest() -> dict[str, Any]:
    return {
        "raw_database": "fleur_raw",
        "datasets": [
            {
                "dataset": "demo__snapshot",
                "partition_strategy": "snapshot",
                "objects": [
                    {
                        "object_key": "source/demo__snapshot/000000_0.parquet",
                        "exists": True,
                    }
                ],
                "missing_objects": [],
            },
            {
                "dataset": "demo__yearly",
                "partition_strategy": "year",
                "expected_years": ["2025", "2026"],
                "objects": [
                    {
                        "object_key": "source/demo__yearly/year=2025/000000_0.parquet",
                        "exists": True,
                    },
                    {
                        "object_key": "source/demo__yearly/year=2026/000000_0.parquet",
                        "exists": True,
                    },
                ],
                "missing_objects": [],
            },
        ],
    }


def _manifest_with_two_yearly_datasets() -> dict[str, Any]:
    manifest = _manifest()
    manifest["datasets"] = [
        manifest["datasets"][1],
        {
            "dataset": "other__yearly",
            "partition_strategy": "year",
            "expected_years": ["2025", "2026"],
            "objects": [
                {
                    "object_key": "source/other__yearly/year=2025/000000_0.parquet",
                    "exists": True,
                },
                {
                    "object_key": "source/other__yearly/year=2026/000000_0.parquet",
                    "exists": True,
                },
            ],
            "missing_objects": [],
        },
    ]
    return manifest


def _dataset_from_command(command: Sequence[str]) -> str | None:
    if "--assets" not in command:
        return None
    asset_index = command.index("--assets")
    asset_selection = command[asset_index + 1]
    prefix = "key:clickhouse/raw/"
    if not asset_selection.startswith(prefix):
        return None
    return asset_selection.removeprefix(prefix)


def _write_manifest(tmp_path: Path, manifest: dict[str, Any]) -> Path:
    path = tmp_path / "partitions.json"
    path.write_text(json.dumps(manifest), encoding="utf-8")
    return path
