from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
from collections.abc import Mapping, Sequence
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Literal, Protocol, cast
from urllib.parse import urlparse

import pyarrow.fs as pafs
import pyarrow.parquet as pq

from fleur_contracts.clickhouse_types import effective_clickhouse_type
from fleur_contracts.env import REPO_ROOT, load_repo_dotenv_if_present
from fleur_contracts.loader import DEFAULT_CONTRACT_ROOT, clickhouse_schema_hash, load_registry
from fleur_contracts.schema import DatasetContract
from fleur_contracts.validate_clickhouse import build_client_from_env

LAYER_DATABASES: tuple[str, ...] = (
    "raw",
    "analytics",
    "fleur_raw",
    "fleur_staging",
    "fleur_intermediate",
    "fleur_marts",
)
RAW_TARGET_DATABASE = "fleur_raw"
REPORTS_DIR = REPO_ROOT / "docs" / "jobs" / "reports"
PIPELINE_ROOT = REPO_ROOT / "pipeline"
DEFAULT_MAX_CONCURRENT_RAW_SYNC_DATASETS = 4
LOW_CARDINALITY_UNIQUE_LIMIT = 10_000
MigrationCommandPhase = Literal["preflight", "raw_sync", "validation"]


class ClickHouseQueryResult(Protocol):
    @property
    def result_rows(self) -> Sequence[Sequence[object]]: ...


class ClickHouseMigrationClient(Protocol):
    def command(self, cmd: str, *, settings: dict[str, object] | None = None) -> object: ...

    def query(
        self,
        query: str,
        *,
        parameters: dict[str, object] | None = None,
        settings: dict[str, object] | None = None,
    ) -> ClickHouseQueryResult: ...

    def close(self) -> None: ...


@dataclass(frozen=True)
class BaselineArtifacts:
    baseline_report: Path
    partition_manifest: Path
    confirmation_token: str


@dataclass(frozen=True)
class MigrationStepResult:
    label: str
    command: tuple[str, ...]
    cwd: Path
    returncode: int
    stdout: str
    stderr: str

    @property
    def status(self) -> str:
        if self.returncode == 0:
            return "success"
        return "failed"


@dataclass(frozen=True)
class MigrationCommand:
    label: str
    cwd: Path
    command: tuple[str, ...]
    phase: MigrationCommandPhase
    raw_sync_group: str | None = None


def run_baseline(
    *,
    contract_root: Path = DEFAULT_CONTRACT_ROOT,
    reports_dir: Path = REPORTS_DIR,
    client: ClickHouseMigrationClient | None = None,
    now: datetime | None = None,
) -> BaselineArtifacts:
    load_repo_dotenv_if_present()
    registry = load_registry(contract_root)
    timestamp = now or datetime.now(UTC)
    owns_client = client is None
    active_client: ClickHouseMigrationClient = _build_migration_client(client)

    try:
        databases = _database_existence(active_client)
        raw_datasets = _raw_datasets(registry.datasets)
        table_baseline = _table_baseline(active_client, raw_datasets)
        manifest = _partition_manifest(raw_datasets)
        grants = _clickhouse_grants(active_client)
        working_tree = _working_tree_status()
        token = confirmation_token(timestamp, manifest)
        report = render_baseline_report(
            timestamp=timestamp,
            databases=databases,
            table_baseline=table_baseline,
            manifest=manifest,
            grants=grants,
            working_tree=working_tree,
            confirmation_token=token,
        )

        reports_dir.mkdir(parents=True, exist_ok=True)
        date_prefix = timestamp.strftime("%Y-%m-%d")
        report_path = reports_dir / f"{date_prefix}-clickhouse-layered-database-baseline.md"
        manifest_path = reports_dir / f"{date_prefix}-clickhouse-layered-database-partitions.json"
        report_path.write_text(report, encoding="utf-8")
        manifest_path.write_text(
            json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return BaselineArtifacts(
            baseline_report=report_path,
            partition_manifest=manifest_path,
            confirmation_token=token,
        )
    finally:
        if owns_client:
            active_client.close()


def run_reset(
    *,
    confirm: str,
    baseline_manifest_path: Path,
    reports_dir: Path = REPORTS_DIR,
    client: ClickHouseMigrationClient | None = None,
    now: datetime | None = None,
) -> Path:
    load_repo_dotenv_if_present()
    timestamp = now or datetime.now(UTC)
    manifest = json.loads(baseline_manifest_path.read_text(encoding="utf-8"))
    missing_objects = _manifest_missing_objects(manifest)
    if missing_objects:
        msg = "Partition manifest has missing S3 objects; reset is not allowed"
        raise RuntimeError(msg)
    expected_token = confirmation_token(timestamp=None, manifest=manifest)
    if confirm != expected_token:
        msg = "Confirmation token does not match the baseline partition manifest"
        raise RuntimeError(msg)

    owns_client = client is None
    active_client: ClickHouseMigrationClient = _build_migration_client(client)

    try:
        for database in LAYER_DATABASES:
            active_client.command(f"DROP DATABASE IF EXISTS `{database}`")
        remaining = _database_existence(active_client)
        not_dropped = [database for database in LAYER_DATABASES if remaining.get(database)]
        if not_dropped:
            msg = f"Databases still exist after reset: {', '.join(not_dropped)}"
            raise RuntimeError(msg)

        reports_dir.mkdir(parents=True, exist_ok=True)
        date_prefix = timestamp.strftime("%Y-%m-%d")
        report_path = reports_dir / f"{date_prefix}-clickhouse-layered-database-reset.md"
        report_path.write_text(
            render_reset_report(
                timestamp=timestamp,
                baseline_manifest_path=baseline_manifest_path,
                confirmation_token=confirm,
            ),
            encoding="utf-8",
        )
        return report_path
    finally:
        if owns_client:
            active_client.close()


def run_migrate(
    *,
    confirm: str,
    baseline_manifest_path: Path,
    contract_root: Path = DEFAULT_CONTRACT_ROOT,
    reports_dir: Path = REPORTS_DIR,
    client: ClickHouseMigrationClient | None = None,
    now: datetime | None = None,
    command_runner: Any = subprocess.run,
    max_concurrent_raw_sync_datasets: int = DEFAULT_MAX_CONCURRENT_RAW_SYNC_DATASETS,
) -> Path:
    timestamp = now or datetime.now(UTC)
    manifest = json.loads(baseline_manifest_path.read_text(encoding="utf-8"))
    reset_report: Path | None = None
    results: list[MigrationStepResult] = []
    raw_validation_issues: list[str] = []
    dbt_validation_issues: list[str] = []
    failure: str | None = None

    try:
        reset_report = run_reset(
            confirm=confirm,
            baseline_manifest_path=baseline_manifest_path,
            reports_dir=reports_dir,
            client=client,
            now=timestamp,
        )
        command_plan = migration_command_plan_from_manifest(manifest)
        results = execute_migration_command_plan(
            command_plan,
            command_runner=command_runner,
            max_concurrent_raw_sync_datasets=max_concurrent_raw_sync_datasets,
        )
        failed_result = next((result for result in results if result.returncode != 0), None)
        if failed_result is not None:
            failure = f"{failed_result.label} failed with exit code {failed_result.returncode}"

        if failure is None:
            raw_validation_issues = validate_raw(
                contract_root=contract_root,
                baseline_manifest_path=baseline_manifest_path,
                client=client,
            )
            if raw_validation_issues:
                failure = "Raw validation failed"
        if failure is None:
            dbt_validation_issues = validate_dbt_layer(client=client)
            if dbt_validation_issues:
                failure = "dbt layer validation failed"
    except Exception as error:
        failure = str(error)
        raise
    finally:
        report_path = write_migration_report(
            timestamp=timestamp,
            reports_dir=reports_dir,
            baseline_manifest_path=baseline_manifest_path,
            reset_report=reset_report,
            results=results,
            raw_validation_issues=raw_validation_issues,
            dbt_validation_issues=dbt_validation_issues,
            failure=failure,
        )

    if failure is not None:
        msg = f"Migration failed; report written to {report_path}: {failure}"
        raise RuntimeError(msg)

    return report_path


def migration_commands_from_manifest(
    manifest: Mapping[str, object],
) -> list[tuple[str, Path, tuple[str, ...]]]:
    return [
        (command.label, command.cwd, command.command)
        for command in migration_command_plan_from_manifest(manifest)
    ]


def migration_command_plan_from_manifest(
    manifest: Mapping[str, object],
) -> list[MigrationCommand]:
    missing_objects = _manifest_missing_objects(manifest)
    if missing_objects:
        msg = f"Partition manifest has missing S3 objects: {missing_objects}"
        raise RuntimeError(msg)

    commands: list[MigrationCommand] = [
        MigrationCommand(
            label="dagster home initialization",
            cwd=REPO_ROOT,
            command=("make", "dagster-home"),
            phase="preflight",
        ),
        MigrationCommand(
            label="dagster definitions check",
            cwd=PIPELINE_ROOT,
            command=("uv", "run", "dg", "check", "defs", "--target-path", "scheduler"),
            phase="preflight",
        ),
    ]
    for dataset in _manifest_datasets(manifest):
        dataset_name = str(dataset["dataset"])
        asset_selection = f"key:clickhouse/raw/{dataset_name}"
        years = dataset.get("expected_years")
        if isinstance(years, list):
            if not years:
                msg = f"Year-partitioned dataset has no expected years: {dataset_name}"
                raise RuntimeError(msg)
            for year in years:
                commands.append(
                    MigrationCommand(
                        label=f"raw sync {dataset_name} partition {year}",
                        cwd=PIPELINE_ROOT,
                        command=(
                            "uv",
                            "run",
                            "dg",
                            "launch",
                            "--target-path",
                            "scheduler",
                            "--assets",
                            asset_selection,
                            "--partition",
                            str(year),
                        ),
                        phase="raw_sync",
                        raw_sync_group=dataset_name,
                    )
                )
            continue
        commands.append(
            MigrationCommand(
                label=f"raw sync {dataset_name} snapshot",
                cwd=PIPELINE_ROOT,
                command=(
                    "uv",
                    "run",
                    "dg",
                    "launch",
                    "--target-path",
                    "scheduler",
                    "--assets",
                    asset_selection,
                ),
                phase="raw_sync",
                raw_sync_group=dataset_name,
            )
        )

    commands.extend(
        [
            MigrationCommand(
                label="dbt parse",
                cwd=PIPELINE_ROOT,
                command=(
                    "uv",
                    "run",
                    "dbt",
                    "parse",
                    "--project-dir",
                    "elt",
                    "--profiles-dir",
                    "elt",
                ),
                phase="validation",
            ),
            MigrationCommand(
                label="dbt list raw sources",
                cwd=PIPELINE_ROOT,
                command=(
                    "uv",
                    "run",
                    "dbt",
                    "list",
                    "--project-dir",
                    "elt",
                    "--profiles-dir",
                    "elt",
                    "--select",
                    "source:raw.*",
                    "--output",
                    "json",
                ),
                phase="validation",
            ),
            MigrationCommand(
                label="dbt build staging",
                cwd=PIPELINE_ROOT,
                command=(
                    "uv",
                    "run",
                    "dbt",
                    "build",
                    "--project-dir",
                    "elt",
                    "--profiles-dir",
                    "elt",
                    "--select",
                    "staging",
                    "--quiet",
                    "--warn-error-options",
                    '{"error": ["NoNodesForSelectionCriteria"]}',
                ),
                phase="validation",
            ),
            MigrationCommand(
                label="dbt layer routing validation",
                cwd=PIPELINE_ROOT,
                command=("uv", "run", "python", "elt/scripts/validate_layer_routing.py"),
                phase="validation",
            ),
            MigrationCommand(
                label="field glossary validation",
                cwd=PIPELINE_ROOT,
                command=("uv", "run", "python", "elt/scripts/validate_field_glossary.py"),
                phase="validation",
            ),
            MigrationCommand(
                label="staging readiness validation",
                cwd=PIPELINE_ROOT,
                command=("uv", "run", "python", "elt/scripts/validate_staging_readiness.py"),
                phase="validation",
            ),
        ]
    )
    return commands


def execute_migration_command_plan(
    commands: Sequence[MigrationCommand],
    *,
    command_runner: Any,
    max_concurrent_raw_sync_datasets: int,
) -> list[MigrationStepResult]:
    if max_concurrent_raw_sync_datasets < 1:
        msg = "max_concurrent_raw_sync_datasets must be positive"
        raise ValueError(msg)

    results: list[MigrationStepResult] = []
    preflight_commands = [command for command in commands if command.phase == "preflight"]
    raw_sync_commands = [command for command in commands if command.phase == "raw_sync"]
    validation_commands = [command for command in commands if command.phase == "validation"]

    results.extend(_execute_serial_commands(preflight_commands, command_runner=command_runner))
    if _has_failed_result(results):
        return results

    results.extend(
        _execute_raw_sync_commands(
            raw_sync_commands,
            command_runner=command_runner,
            max_concurrent_raw_sync_datasets=max_concurrent_raw_sync_datasets,
        )
    )
    if _has_failed_result(results):
        return results

    results.extend(_execute_serial_commands(validation_commands, command_runner=command_runner))
    return results


def _execute_serial_commands(
    commands: Sequence[MigrationCommand],
    *,
    command_runner: Any,
) -> list[MigrationStepResult]:
    results: list[MigrationStepResult] = []
    for command in commands:
        result = _execute_command(command, command_runner=command_runner)
        results.append(result)
        if result.returncode != 0:
            break
    return results


def _execute_raw_sync_commands(
    commands: Sequence[MigrationCommand],
    *,
    command_runner: Any,
    max_concurrent_raw_sync_datasets: int,
) -> list[MigrationStepResult]:
    grouped_commands = _raw_sync_commands_by_group(commands)
    if not grouped_commands:
        return []

    with ThreadPoolExecutor(max_workers=max_concurrent_raw_sync_datasets) as executor:
        futures = [
            executor.submit(
                _execute_serial_commands,
                group_commands,
                command_runner=command_runner,
            )
            for group_commands in grouped_commands.values()
        ]
        results_by_label = {
            result.label: result for future in futures for result in future.result()
        }

    return [
        results_by_label[command.label] for command in commands if command.label in results_by_label
    ]


def _raw_sync_commands_by_group(
    commands: Sequence[MigrationCommand],
) -> dict[str, list[MigrationCommand]]:
    grouped_commands: dict[str, list[MigrationCommand]] = {}
    for command in commands:
        if command.raw_sync_group is None:
            msg = f"Raw sync command is missing a concurrency group: {command.label}"
            raise RuntimeError(msg)
        if command.raw_sync_group not in grouped_commands:
            grouped_commands[command.raw_sync_group] = []
        grouped_commands[command.raw_sync_group].append(command)
    return grouped_commands


def _execute_command(
    command: MigrationCommand,
    *,
    command_runner: Any,
) -> MigrationStepResult:
    completed = command_runner(
        list(command.command),
        cwd=command.cwd,
        check=False,
        capture_output=True,
        text=True,
    )
    return MigrationStepResult(
        label=command.label,
        command=command.command,
        cwd=command.cwd,
        returncode=int(completed.returncode),
        stdout=str(completed.stdout),
        stderr=str(completed.stderr),
    )


def _has_failed_result(results: Sequence[MigrationStepResult]) -> bool:
    return any(result.returncode != 0 for result in results)


def _manifest_datasets(manifest: Mapping[str, object]) -> list[Mapping[str, object]]:
    raw_database = manifest.get("raw_database")
    if raw_database != RAW_TARGET_DATABASE:
        msg = f"Partition manifest raw_database must be {RAW_TARGET_DATABASE!r}"
        raise RuntimeError(msg)
    datasets = manifest.get("datasets")
    if not isinstance(datasets, list):
        msg = "Partition manifest must contain a datasets list"
        raise RuntimeError(msg)
    typed_datasets: list[Mapping[str, object]] = []
    for dataset in datasets:
        if not isinstance(dataset, Mapping):
            msg = "Partition manifest datasets entries must be mappings"
            raise RuntimeError(msg)
        if not str(dataset.get("dataset", "")).strip():
            msg = "Partition manifest dataset entry is missing dataset"
            raise RuntimeError(msg)
        typed_datasets.append(dataset)
    return typed_datasets


def _manifest_missing_objects(manifest: Mapping[str, object]) -> list[str]:
    missing: list[str] = []
    for dataset in _manifest_datasets(manifest):
        missing_objects = dataset.get("missing_objects", [])
        if not isinstance(missing_objects, list):
            msg = "Partition manifest missing_objects must be a list"
            raise RuntimeError(msg)
        missing.extend(str(object_key) for object_key in missing_objects)
    return missing


def validate_empty(client: ClickHouseMigrationClient | None = None) -> list[str]:
    load_repo_dotenv_if_present()
    owns_client = client is None
    active_client: ClickHouseMigrationClient = _build_migration_client(client)
    try:
        databases = _database_existence(active_client)
        return [database for database in LAYER_DATABASES if databases.get(database)]
    finally:
        if owns_client:
            active_client.close()


def validate_raw(
    *,
    contract_root: Path = DEFAULT_CONTRACT_ROOT,
    baseline_manifest_path: Path | None = None,
    client: ClickHouseMigrationClient | None = None,
) -> list[str]:
    load_repo_dotenv_if_present()
    registry = load_registry(contract_root)
    raw_datasets = _raw_datasets(registry.datasets)
    manifest = _load_manifest(baseline_manifest_path)
    manifest_by_dataset = _manifest_by_dataset(manifest)
    owns_client = client is None
    active_client: ClickHouseMigrationClient = _build_migration_client(client)

    issues: list[str] = []
    try:
        issues.extend(_validate_manifest_contract_coverage(raw_datasets, manifest_by_dataset))
        for dataset in raw_datasets:
            raw = dataset.clickhouse_raw
            if raw is None:
                continue
            rows = active_client.query(
                """
                SELECT name, type
                FROM system.columns
                WHERE database = {database:String}
                  AND table = {table:String}
                ORDER BY position
                """,
                parameters={"database": raw.database, "table": raw.table},
            ).result_rows
            if not rows:
                issues.append(f"missing table: {raw.database}.{raw.table}")
                continue
            actual = [(str(row[0]), str(row[1])) for row in rows]
            expected = [
                (field.name, effective_clickhouse_type(field.type, nullable=field.nullable))
                for field in raw.fields
            ]
            if raw.partition_strategy == "year":
                expected.append(("year", "UInt16"))
            if actual != expected:
                issues.append(f"schema mismatch: {raw.database}.{raw.table}")
                continue
            actual_schema_hash = _schema_fingerprint(active_client, raw.database, raw.table)
            expected_schema_hash = clickhouse_schema_hash(dataset)
            if actual_schema_hash != expected_schema_hash:
                issues.append(
                    "schema hash mismatch: "
                    f"{raw.database}.{raw.table} expected {expected_schema_hash}, "
                    f"got {actual_schema_hash}"
                )
            row_count = _first_int(
                active_client.query(
                    f"SELECT count() FROM `{raw.database}`.`{raw.table}`",
                )
            )
            if row_count <= 0 and not raw.allow_empty:
                issues.append(f"empty table: {raw.database}.{raw.table}")
            active_parts = _active_parts(active_client, raw.database, raw.table)
            if active_parts > 3_000:
                issues.append(
                    f"excessive active parts: {raw.database}.{raw.table} has {active_parts}"
                )
            issues.extend(_validate_low_cardinality_columns(active_client, dataset))
            if raw.partition_strategy == "year":
                manifest_dataset = manifest_by_dataset.get(dataset.dataset)
                if manifest_dataset is not None:
                    issues.extend(
                        _validate_year_manifest(
                            active_client,
                            dataset,
                            manifest_dataset=manifest_dataset,
                        )
                    )
        issues.extend(_validate_staging_tables(active_client, raw_datasets))
        return issues
    finally:
        if owns_client:
            active_client.close()


def validate_dbt_layer(client: ClickHouseMigrationClient | None = None) -> list[str]:
    load_repo_dotenv_if_present()
    owns_client = client is None
    active_client: ClickHouseMigrationClient = _build_migration_client(client)
    issues: list[str] = []
    try:
        databases = _database_existence(active_client)
        for database in ("fleur_staging", "fleur_intermediate", "fleur_marts"):
            if not databases.get(database):
                issues.append(f"missing dbt layer database: {database}")
        if databases.get("analytics"):
            analytics_models = active_client.query(
                """
                SELECT name
                FROM system.tables
                WHERE database = 'analytics'
                ORDER BY name
                """
            ).result_rows
            if analytics_models:
                names = ", ".join(str(row[0]) for row in analytics_models[:20])
                issues.append(f"analytics contains model relations: {names}")
        return issues
    finally:
        if owns_client:
            active_client.close()


def _load_manifest(path: Path | None) -> Mapping[str, object] | None:
    if path is None:
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def _manifest_by_dataset(
    manifest: Mapping[str, object] | None,
) -> dict[str, Mapping[str, object]]:
    if manifest is None:
        return {}
    return {str(dataset["dataset"]): dataset for dataset in _manifest_datasets(manifest)}


def _validate_manifest_contract_coverage(
    raw_datasets: Sequence[DatasetContract],
    manifest_by_dataset: Mapping[str, Mapping[str, object]],
) -> list[str]:
    if not manifest_by_dataset:
        return []
    expected = {dataset.dataset for dataset in raw_datasets}
    actual = set(manifest_by_dataset)
    issues: list[str] = []
    for dataset in sorted(expected - actual):
        issues.append(f"manifest missing raw dataset: {dataset}")
    for dataset in sorted(actual - expected):
        issues.append(f"manifest contains non-contract dataset: {dataset}")
    return issues


def _validate_year_manifest(
    client: ClickHouseMigrationClient,
    dataset: DatasetContract,
    *,
    manifest_dataset: Mapping[str, object],
) -> list[str]:
    raw = dataset.clickhouse_raw
    if raw is None:
        return []
    years_obj = manifest_dataset.get("expected_years")
    if not isinstance(years_obj, list):
        return [f"manifest missing expected_years for year dataset: {dataset.dataset}"]
    expected_years = [str(year) for year in years_obj]
    expected_present_years = _expected_present_years(manifest_dataset, allow_empty=raw.allow_empty)
    actual_years = _year_partitions(client, raw.database, raw.table)
    issues: list[str] = []
    if actual_years != expected_present_years:
        issues.append(
            f"year partition mismatch: {raw.database}.{raw.table} "
            f"expected {expected_present_years}, got {actual_years}"
        )
    for year in expected_years:
        row = _first_row(
            client.query(
                f"""
                SELECT count(), min(`year`), max(`year`)
                FROM `{raw.database}`.`{raw.table}`
                WHERE `year` = {{year:UInt16}}
                """,
                parameters={"year": int(year)},
            )
        )
        if len(row) != 3:
            issues.append(f"year validation query returned {len(row)} values for {dataset.dataset}")
            continue
        row_count = int(str(row[0]))
        min_year = int(str(row[1])) if row[1] is not None else 0
        max_year = int(str(row[2])) if row[2] is not None else 0
        if row_count <= 0 and not raw.allow_empty:
            issues.append(f"empty year partition: {raw.database}.{raw.table} year={year}")
        if row_count > 0 and (min_year != int(year) or max_year != int(year)):
            issues.append(
                f"year partition contains wrong range: {raw.database}.{raw.table} "
                f"year={year} range={min_year}..{max_year}"
            )
    return issues


def _expected_present_years(
    manifest_dataset: Mapping[str, object],
    *,
    allow_empty: bool,
) -> list[str]:
    years_obj = manifest_dataset.get("expected_years", [])
    years = [str(year) for year in years_obj] if isinstance(years_obj, list) else []
    if not allow_empty:
        return years

    row_count_by_year = _manifest_row_count_by_year(manifest_dataset)
    if not row_count_by_year:
        return years
    return [year for year in years if row_count_by_year.get(year, 1) > 0]


def _manifest_row_count_by_year(manifest_dataset: Mapping[str, object]) -> dict[str, int]:
    objects = manifest_dataset.get("objects", [])
    if not isinstance(objects, list):
        return {}

    row_counts: dict[str, int] = {}
    for item in objects:
        if not isinstance(item, Mapping):
            continue
        row_count = item.get("row_count")
        if not isinstance(row_count, int):
            continue
        object_key = str(item.get("object_key", ""))
        marker = "/year="
        if marker not in object_key:
            continue
        year = object_key.split(marker, maxsplit=1)[1].split("/", maxsplit=1)[0]
        row_counts[year] = row_count
    return row_counts


def _validate_low_cardinality_columns(
    client: ClickHouseMigrationClient,
    dataset: DatasetContract,
) -> list[str]:
    raw = dataset.clickhouse_raw
    if raw is None:
        return []
    issues: list[str] = []
    for field in raw.fields:
        if not field.type.startswith("LowCardinality("):
            continue
        unique_count = _first_int(
            client.query(f"SELECT uniq(`{field.name}`) FROM `{raw.database}`.`{raw.table}`")
        )
        if unique_count > LOW_CARDINALITY_UNIQUE_LIMIT:
            issues.append(
                f"low cardinality limit exceeded: {raw.database}.{raw.table}.{field.name} "
                f"has {unique_count} unique values"
            )
    return issues


def _validate_staging_tables(
    client: ClickHouseMigrationClient,
    raw_datasets: Sequence[DatasetContract],
) -> list[str]:
    issues: list[str] = []
    for dataset in raw_datasets:
        raw = dataset.clickhouse_raw
        if raw is None:
            continue
        staging_table = f"{dataset.dataset}__stage"
        if not _table_exists(client, raw.database, staging_table):
            continue
        row_count = _first_int(
            client.query(f"SELECT count() FROM `{raw.database}`.`{staging_table}`")
        )
        issues.append(f"residual staging table: {raw.database}.{staging_table} rows={row_count}")
    return issues


def _year_partitions(
    client: ClickHouseMigrationClient,
    database: str,
    table: str,
) -> list[str]:
    rows = client.query(
        f"""
        SELECT toString(`year`)
        FROM `{database}`.`{table}`
        GROUP BY `year`
        ORDER BY `year`
        """
    ).result_rows
    return [str(row[0]) for row in rows]


def render_migration_report_skeleton(
    *,
    reports_dir: Path = REPORTS_DIR,
    now: datetime | None = None,
) -> Path:
    timestamp = now or datetime.now(UTC)
    reports_dir.mkdir(parents=True, exist_ok=True)
    date_prefix = timestamp.strftime("%Y-%m-%d")
    path = reports_dir / f"{date_prefix}-clickhouse-layered-database-migration-report.md"
    path.write_text(
        "\n".join(
            (
                "# ClickHouse Layered Database Migration Report",
                "",
                f"日期：{timestamp.isoformat()}",
                "执行人：",
                f"Git commit / working tree：{_git_head()} / {_working_tree_status()}",
                "环境：",
                "",
                "## 1. Scope",
                "",
                "## 2. Baseline Summary",
                "",
                "## 3. Reset Summary",
                "",
                "## 4. Dagster Rematerialization Runs",
                "",
                "## 5. Raw Table Validation",
                "",
                "## 6. dbt Layer Validation",
                "",
                "## 7. Failures / Exceptions",
                "",
                "## 8. Acceptance Checklist",
                "",
                "## 9. Follow-ups",
                "",
            )
        ),
        encoding="utf-8",
    )
    return path


def write_migration_report(
    *,
    timestamp: datetime,
    reports_dir: Path,
    baseline_manifest_path: Path,
    reset_report: Path | None,
    results: Sequence[MigrationStepResult],
    raw_validation_issues: Sequence[str],
    dbt_validation_issues: Sequence[str],
    failure: str | None,
) -> Path:
    reports_dir.mkdir(parents=True, exist_ok=True)
    date_prefix = timestamp.strftime("%Y-%m-%d")
    path = reports_dir / f"{date_prefix}-clickhouse-layered-database-migration-report.md"
    lines = [
        "# ClickHouse Layered Database Migration Report",
        "",
        f"日期：{timestamp.isoformat()}",
        "执行人：fleur-contracts clickhouse-layer migrate",
        f"Git commit / working tree：{_git_head()} / {_working_tree_status()}",
        "环境：ClickHouse + Dagster + dbt local runner",
        "",
        "## 1. Scope",
        "",
        "- Reset databases: `raw`, `analytics`, `fleur_raw`, `fleur_staging`, "
        "`fleur_intermediate`, `fleur_marts`.",
        "- Rematerialize all `clickhouse/raw/*` assets from the partition manifest.",
        "",
        "## 2. Baseline Summary",
        "",
        f"- Partition manifest: `{baseline_manifest_path}`",
        "",
        "## 3. Reset Summary",
        "",
        f"- Reset report: `{reset_report}`"
        if reset_report is not None
        else "- Reset did not complete.",
        "",
        "## 4. Dagster Rematerialization Runs",
        "",
        "| Step | Status | Run IDs | Command |",
        "| --- | --- | --- | --- |",
    ]
    for result in results:
        command = " ".join(result.command)
        run_ids = ", ".join(_extract_run_ids(result.stdout)) or "n/a"
        lines.append(f"| {result.label} | {result.status} | {run_ids} | `{command}` |")
    lines.extend(
        (
            "",
            "## 5. Raw Table Validation",
            "",
        )
    )
    if raw_validation_issues:
        lines.extend(f"- {issue}" for issue in raw_validation_issues)
    elif results and failure is None:
        lines.append("- `validate_raw` passed.")
    else:
        lines.append("- Raw validation did not run.")
    lines.extend(
        (
            "",
            "## 6. dbt Layer Validation",
            "",
        )
    )
    if dbt_validation_issues:
        lines.extend(f"- {issue}" for issue in dbt_validation_issues)
    elif results and failure is None:
        lines.append("- dbt layer validation passed.")
    else:
        lines.append("- dbt layer validation did not run.")
    lines.extend(
        (
            "- See `dbt parse`, `dbt build staging`, and validation steps above.",
            "",
            "## 7. Failures / Exceptions",
            "",
            f"- {failure}" if failure else "- None recorded.",
            "",
            "## 8. Acceptance Checklist",
            "",
            "- Confirm all step statuses above are `success`.",
            "- Confirm raw validation has no issues.",
            "- Confirm dbt layer validation has no issues.",
            "",
            "## 9. Follow-ups",
            "",
            "- Remove or explain any residual staging tables after acceptance.",
            "",
        )
    )
    path.write_text("\n".join(lines), encoding="utf-8")
    return path


def confirmation_token(
    timestamp: datetime | None,
    manifest: Mapping[str, object],
) -> str:
    del timestamp
    payload = json.dumps(manifest, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()[:16]


def render_baseline_report(
    *,
    timestamp: datetime,
    databases: Mapping[str, bool],
    table_baseline: Sequence[Mapping[str, object]],
    manifest: Mapping[str, object],
    grants: Sequence[str],
    working_tree: str,
    confirmation_token: str,
) -> str:
    dataset_count = len(table_baseline)
    manifest_datasets_obj = manifest.get("datasets", [])
    manifest_datasets = manifest_datasets_obj if isinstance(manifest_datasets_obj, list) else []
    missing_objects = [
        item
        for item in manifest_datasets
        if isinstance(item, Mapping) and item.get("missing_objects")
    ]
    lines = [
        "# ClickHouse Layered Database Baseline",
        "",
        f"日期：{timestamp.isoformat()}",
        f"Git working tree：{working_tree}",
        "",
        "## 1. Database Scope",
        "",
        "| Database | Exists |",
        "| --- | --- |",
    ]
    lines.extend(
        f"| `{database}` | {databases.get(database, False)} |" for database in LAYER_DATABASES
    )
    lines.extend(
        (
            "",
            "## 2. Raw Dataset Baseline",
            "",
            f"- Raw-enabled datasets：{dataset_count}",
            "",
            "| Dataset | Historical raw table | Migration target | Historical raw rows | Schema fingerprint | Active parts | Partitions |",
            "| --- | --- | --- | ---: | --- | ---: | --- |",
        )
    )
    for item in table_baseline:
        partition_values = item.get("partitions", [])
        partition_list = partition_values if isinstance(partition_values, list) else []
        partitions = ", ".join(str(value) for value in partition_list)
        lines.append(
            "| `{dataset}` | `{historical_target}` | `{migration_target}` | {rows} | `{schema}` | {parts} | {partitions} |".format(
                dataset=item["dataset"],
                historical_target=item["historical_target"],
                migration_target=item["migration_target"],
                rows=item["historical_raw_rows"],
                schema=item["schema_fingerprint"],
                parts=item["active_parts"],
                partitions=partitions or "n/a",
            )
        )
    lines.extend(
        (
            "",
            "## 3. S3 Coverage",
            "",
            f"- Manifest datasets：{len(manifest_datasets)}",
            f"- Datasets with missing objects：{len(missing_objects)}",
            "",
            "| Dataset | Strategy | Expected objects | Missing objects |",
            "| --- | --- | ---: | ---: |",
        )
    )
    for dataset in manifest_datasets:
        if not isinstance(dataset, Mapping):
            continue
        objects = dataset.get("objects", [])
        missing = dataset.get("missing_objects", [])
        object_count = len(objects) if isinstance(objects, list) else 0
        missing_count = len(missing) if isinstance(missing, list) else 0
        lines.append(
            "| `{dataset}` | `{strategy}` | {object_count} | {missing_count} |".format(
                dataset=dataset.get("dataset", "unknown"),
                strategy=dataset.get("partition_strategy", "unknown"),
                object_count=object_count,
                missing_count=missing_count,
            )
        )
    lines.extend(
        (
            "",
            "## 4. Dagster / Contract Scope",
            "",
            f"- Raw sync asset keys：{len(manifest_datasets)}",
            "",
        )
    )
    for dataset in manifest_datasets:
        if isinstance(dataset, Mapping):
            lines.append(f"- `clickhouse/raw/{dataset.get('dataset', 'unknown')}`")
    lines.extend(
        (
            "",
            "## 5. ClickHouse Privilege Evidence",
            "",
            "- Required privileges: `DROP DATABASE`, `CREATE DATABASE`, `CREATE TABLE`, "
            "`INSERT`, `ALTER TABLE REPLACE PARTITION`, `EXCHANGE TABLES`, `SELECT`.",
            "",
        )
    )
    if grants:
        lines.extend(f"- `{grant}`" for grant in grants)
    else:
        lines.append("- No grant rows returned by `SHOW GRANTS`.")
    lines.extend(
        (
            "",
            "## 6. Safety Gate",
            "",
            f"- Confirmation token：`{confirmation_token}`",
            "- Reset command：`uv run fleur-contracts clickhouse-layer reset "
            "--manifest <partition-manifest.json> --confirm <token>`",
            "",
        )
    )
    return "\n".join(lines)


def render_reset_report(
    *,
    timestamp: datetime,
    baseline_manifest_path: Path,
    confirmation_token: str,
) -> str:
    token_hash = hashlib.sha256(confirmation_token.encode("utf-8")).hexdigest()
    lines = [
        "# ClickHouse Layered Database Reset Report",
        "",
        f"日期：{timestamp.isoformat()}",
        f"Baseline manifest：`{baseline_manifest_path}`",
        f"Confirmation token hash：`{token_hash}`",
        "",
        "## Dropped Databases",
        "",
    ]
    lines.extend(f"- `{database}`" for database in LAYER_DATABASES)
    lines.append("")
    return "\n".join(lines)


def _raw_datasets(datasets: Sequence[DatasetContract]) -> list[DatasetContract]:
    raw_datasets = [dataset for dataset in datasets if dataset.clickhouse_raw is not None]
    bad_databases = {
        dataset.dataset: dataset.clickhouse_raw.database
        for dataset in raw_datasets
        if dataset.clickhouse_raw is not None
        and dataset.clickhouse_raw.database != RAW_TARGET_DATABASE
    }
    if bad_databases:
        msg = f"Raw datasets must target {RAW_TARGET_DATABASE}: {bad_databases}"
        raise RuntimeError(msg)
    return raw_datasets


def _build_migration_client(
    client: ClickHouseMigrationClient | None,
) -> ClickHouseMigrationClient:
    if client is not None:
        return client
    return cast(ClickHouseMigrationClient, build_client_from_env(database="default"))


def _database_existence(client: ClickHouseMigrationClient) -> dict[str, bool]:
    rows = client.query(
        """
        SELECT name
        FROM system.databases
        WHERE name IN {databases:Array(String)}
        """,
        parameters={"databases": list(LAYER_DATABASES)},
    ).result_rows
    existing = {str(row[0]) for row in rows}
    return {database: database in existing for database in LAYER_DATABASES}


def _table_baseline(
    client: ClickHouseMigrationClient,
    raw_datasets: Sequence[DatasetContract],
) -> list[dict[str, object]]:
    baseline: list[dict[str, object]] = []
    for dataset in raw_datasets:
        raw = dataset.clickhouse_raw
        if raw is None:
            continue
        baseline.append(
            {
                "dataset": dataset.dataset,
                "historical_target": f"raw.{raw.table}",
                "migration_target": f"{raw.database}.{raw.table}",
                "historical_raw_rows": _count_table_if_exists(client, "raw", raw.table),
                "schema_fingerprint": _schema_fingerprint(client, "raw", raw.table),
                "active_parts": _active_parts(client, "raw", raw.table),
                "partitions": _partitions(client, "raw", raw.table),
            }
        )
    return baseline


def _partition_manifest(raw_datasets: Sequence[DatasetContract]) -> dict[str, object]:
    s3 = _s3_file_system_from_env()
    bucket = _required_env("RUSTFS_BUCKET")
    datasets: list[dict[str, object]] = []
    for dataset in raw_datasets:
        raw = dataset.clickhouse_raw
        if raw is None:
            continue
        if dataset.parquet.storage_mode == "latest_snapshot":
            object_key = _parquet_object_key(dataset)
            datasets.append(
                {
                    "dataset": dataset.dataset,
                    "partition_strategy": raw.partition_strategy,
                    "objects": [_object_manifest_entry(s3, bucket, object_key)],
                    "missing_objects": _missing_objects(s3, bucket, [object_key]),
                }
            )
            continue

        years = _s3_partition_years(s3, bucket, dataset)
        object_keys = [_parquet_object_key(dataset, partition_key=year) for year in years]
        datasets.append(
            {
                "dataset": dataset.dataset,
                "partition_strategy": raw.partition_strategy,
                "expected_years": years,
                "objects": [
                    _object_manifest_entry(s3, bucket, object_key) for object_key in object_keys
                ],
                "missing_objects": _missing_objects(s3, bucket, object_keys),
            }
        )
    return {
        "generated_at": datetime.now(UTC).isoformat(),
        "raw_database": RAW_TARGET_DATABASE,
        "datasets": datasets,
    }


def _clickhouse_grants(client: ClickHouseMigrationClient) -> list[str]:
    rows = client.query("SHOW GRANTS").result_rows
    return [str(row[0]) for row in rows if row]


def _extract_run_ids(text: str) -> list[str]:
    matches = re.findall(
        r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-"
        r"[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b",
        text,
    )
    seen: set[str] = set()
    run_ids: list[str] = []
    for match in matches:
        normalized = match.lower()
        if normalized in seen:
            continue
        seen.add(normalized)
        run_ids.append(normalized)
    return run_ids


def _s3_file_system_from_env() -> Any:
    endpoint = _required_env("RUSTFS_ENDPOINT")
    scheme = None
    if "://" in endpoint:
        parsed_endpoint = urlparse(endpoint)
        scheme = parsed_endpoint.scheme
        endpoint = parsed_endpoint.netloc

    s3_filesystem_factory = cast(Any, pafs).S3FileSystem
    return s3_filesystem_factory(
        access_key=_required_env("RUSTFS_ACCESS_KEY"),
        secret_key=_required_env("RUSTFS_SECRET_KEY"),
        endpoint_override=endpoint,
        scheme=scheme,
        region=os.environ.get("RUSTFS_REGION_NAME", "us-east-1"),
    )


def _s3_partition_years(s3: Any, bucket: str, dataset: DatasetContract) -> list[str]:
    prefix = "/".join((*dataset.source_asset_key, ""))
    file_selector_factory = cast(Any, pafs).FileSelector
    selector = file_selector_factory(f"{bucket}/{prefix}", recursive=True)
    years: set[str] = set()
    for info in s3.get_file_info(selector):
        path = str(info.path)
        marker = "/year="
        if marker not in path or not path.endswith("/000000_0.parquet"):
            continue
        year = path.split(marker, maxsplit=1)[1].split("/", maxsplit=1)[0]
        if len(year) == 4 and year.isdigit():
            years.add(year)
    return sorted(years)


def _object_manifest_entry(s3: Any, bucket: str, object_key: str) -> dict[str, object]:
    info = s3.get_file_info(f"{bucket}/{object_key}")
    file_type = cast(Any, pafs).FileType
    exists = info.type == file_type.File
    return {
        "object_key": object_key,
        "exists": exists,
        "size": info.size if exists else None,
        "mtime": info.mtime.isoformat() if info.mtime is not None else None,
        "row_count": _parquet_row_count(s3, bucket, object_key) if exists else None,
    }


def _parquet_row_count(s3: Any, bucket: str, object_key: str) -> int:
    parquet_file = pq.ParquetFile(f"{bucket}/{object_key}", filesystem=s3)
    metadata = parquet_file.metadata
    if metadata is None:
        return 0
    return int(metadata.num_rows)


def _missing_objects(s3: Any, bucket: str, object_keys: Sequence[str]) -> list[str]:
    missing: list[str] = []
    file_type = cast(Any, pafs).FileType
    for object_key in object_keys:
        info = s3.get_file_info(f"{bucket}/{object_key}")
        if info.type != file_type.File:
            missing.append(object_key)
    return missing


def _parquet_object_key(dataset: DatasetContract, partition_key: str | None = None) -> str:
    path_parts = list(dataset.source_asset_key)
    if partition_key is not None:
        partition_key_name = dataset.parquet.partition_key_name
        if partition_key_name is None:
            msg = f"{dataset.dataset} has no partition key name"
            raise RuntimeError(msg)
        path_parts.append(f"{partition_key_name}={partition_key}")
    path_parts.append("000000_0.parquet")
    return "/".join(path_parts)


def _count_table_if_exists(
    client: ClickHouseMigrationClient, database: str, table: str
) -> int | None:
    if not _table_exists(client, database, table):
        return None
    return _first_int(client.query(f"SELECT count() FROM `{database}`.`{table}`"))


def _schema_fingerprint(
    client: ClickHouseMigrationClient,
    database: str,
    table: str,
) -> str:
    rows = client.query(
        """
        SELECT name, type
        FROM system.columns
        WHERE database = {database:String}
          AND table = {table:String}
        ORDER BY position
        """,
        parameters={"database": database, "table": table},
    ).result_rows
    schema_text = "\n".join(f"{row[0]}:{row[1]}" for row in rows)
    return hashlib.sha256(schema_text.encode("utf-8")).hexdigest() if schema_text else ""


def _active_parts(client: ClickHouseMigrationClient, database: str, table: str) -> int:
    return _first_int(
        client.query(
            """
            SELECT count()
            FROM system.parts
            WHERE active
              AND database = {database:String}
              AND table = {table:String}
            """,
            parameters={"database": database, "table": table},
        )
    )


def _partitions(client: ClickHouseMigrationClient, database: str, table: str) -> list[str]:
    rows = client.query(
        """
        SELECT partition
        FROM system.parts
        WHERE active
          AND database = {database:String}
          AND table = {table:String}
        GROUP BY partition
        ORDER BY partition
        """,
        parameters={"database": database, "table": table},
    ).result_rows
    return [str(row[0]) for row in rows]


def _table_exists(client: ClickHouseMigrationClient, database: str, table: str) -> bool:
    return (
        _first_int(
            client.query(
                """
            SELECT count()
            FROM system.tables
            WHERE database = {database:String}
              AND name = {table:String}
            """,
                parameters={"database": database, "table": table},
            )
        )
        > 0
    )


def _first_int(result: ClickHouseQueryResult) -> int:
    if not result.result_rows or not result.result_rows[0]:
        return 0
    return int(str(result.result_rows[0][0]))


def _first_row(result: ClickHouseQueryResult) -> Sequence[object]:
    if not result.result_rows:
        return ()
    return result.result_rows[0]


def _required_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        msg = f"{name} is required for ClickHouse layer migration"
        raise RuntimeError(msg)
    return value


def _working_tree_status() -> str:
    result = subprocess.run(
        ["git", "status", "--short"],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    status = result.stdout.strip()
    return status or "clean"


def _git_head() -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip() or "unknown"
