from __future__ import annotations

import json
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import click

ELT_ROOT = Path(__file__).resolve().parents[1]
PIPELINE_ROOT = ELT_ROOT.parent
REPO_ROOT = PIPELINE_ROOT.parent
MANIFEST_PATH = ELT_ROOT / "target" / "manifest.json"
RAW_PROFILE_ROOT = REPO_ROOT / "docs" / "references" / "raw_profile"


@dataclass(frozen=True)
class ReadinessIssue:
    rule: str
    severity: str
    model: str
    column: str
    raw_table: str
    message: str
    fix: str

    def format(self) -> str:
        location = self.model if self.column == "<model>" else f"{self.model}.{self.column}"
        table = self.raw_table if self.raw_table else "<unknown>"
        return (
            f"{self.severity.upper()} {self.rule} {location} [{table}]: "
            f"{self.message} Fix: {self.fix}"
        )


def main() -> None:
    issues = validate()
    errors = [issue for issue in issues if issue.severity == "error"]
    warnings = [issue for issue in issues if issue.severity == "warn"]

    if warnings:
        click.echo("Staging readiness warnings:", err=True)
        for issue in warnings:
            click.echo(f"- {issue.format()}", err=True)

    if errors:
        click.echo("Staging readiness failed:", err=True)
        for issue in errors:
            click.echo(f"- {issue.format()}", err=True)
        raise SystemExit(1)

    click.echo("Staging readiness passed.")


def validate() -> list[ReadinessIssue]:
    manifest = _load_json(MANIFEST_PATH)
    issues: list[ReadinessIssue] = []
    staging_models = [
        node for node in manifest.get("nodes", {}).values() if _is_staging_model(node)
    ]

    for node in staging_models:
        if isinstance(node, Mapping):
            issues.extend(_validate_model(node))

    return issues


def _validate_model(node: Mapping[str, Any]) -> list[ReadinessIssue]:
    model_name = str(node.get("name", "<unknown>"))
    columns = node.get("columns", {})
    if not isinstance(columns, Mapping) or not columns:
        return [
            ReadinessIssue(
                rule="S001",
                severity="error",
                model=model_name,
                column="<model>",
                raw_table="",
                message="staging model has no YAML-declared columns",
                fix="Declare staging output columns and config.meta.source_columns.",
            )
        ]

    issues: list[ReadinessIssue] = []
    for column_name, column in columns.items():
        if isinstance(column, Mapping):
            issues.extend(_validate_column(model_name, str(column_name), column))

    return issues


def _validate_column(
    model_name: str,
    column_name: str,
    column: Mapping[str, Any],
) -> list[ReadinessIssue]:
    meta = _column_meta(column)
    source_columns = meta.get("source_columns")
    dictionary_scope = meta.get("dictionary_scope")
    derived_from = meta.get("derived_from")

    if not isinstance(source_columns, list) or not source_columns:
        if dictionary_scope == "local" or derived_from is not None:
            return [
                ReadinessIssue(
                    rule="S005",
                    severity="warn",
                    model=model_name,
                    column=column_name,
                    raw_table="",
                    message="local or derived column has no direct source_columns lineage",
                    fix="Ensure derived_from ultimately traces to a profiled raw input.",
                )
            ]
        return [
            ReadinessIssue(
                rule="S001",
                severity="error",
                model=model_name,
                column=column_name,
                raw_table="",
                message="staging column is missing config.meta.source_columns",
                fix="Add source_columns or mark the field as local/derived with lineage.",
            )
        ]

    issues: list[ReadinessIssue] = []
    seen_tables: set[str] = set()
    for source_column in source_columns:
        if not isinstance(source_column, Mapping):
            issues.append(
                ReadinessIssue(
                    rule="S001",
                    severity="error",
                    model=model_name,
                    column=column_name,
                    raw_table="",
                    message="source_columns entry is not a mapping",
                    fix="Use source/table/column keys for each source_columns entry.",
                )
            )
            continue

        raw_table = str(source_column.get("table", ""))
        if not raw_table:
            issues.append(
                ReadinessIssue(
                    rule="S001",
                    severity="error",
                    model=model_name,
                    column=column_name,
                    raw_table="",
                    message="source_columns entry has no table",
                    fix="Set config.meta.source_columns[].table.",
                )
            )
            continue

        if raw_table in seen_tables:
            continue
        seen_tables.add(raw_table)
        issues.extend(_validate_report(model_name, column_name, raw_table))

    return issues


def _validate_report(model_name: str, column_name: str, raw_table: str) -> list[ReadinessIssue]:
    report_path = RAW_PROFILE_ROOT / f"{raw_table}.md"
    if not report_path.exists():
        return [
            ReadinessIssue(
                rule="S002",
                severity="error",
                model=model_name,
                column=column_name,
                raw_table=raw_table,
                message="raw profile report does not exist",
                fix=f"Create docs/references/raw_profile/{raw_table}.md before staging work.",
            )
        ]

    text = report_path.read_text(encoding="utf-8")
    issues: list[ReadinessIssue] = []
    if "## 9. 验收清单" not in text and "## 9. Acceptance" not in text:
        issues.append(
            ReadinessIssue(
                rule="S003",
                severity="error",
                model=model_name,
                column=column_name,
                raw_table=raw_table,
                message="raw profile report has no acceptance checklist",
                fix="Add the standard ## 9. 验收清单 section.",
            )
        )

    status = _report_status(text)
    if status is None:
        issues.append(
            ReadinessIssue(
                rule="S003",
                severity="error",
                model=model_name,
                column=column_name,
                raw_table=raw_table,
                message="raw profile report has no status",
                fix="Add a status line such as `状态：Draft` or `状态：Accepted`.",
            )
        )
    elif status != "Accepted":
        issues.append(
            ReadinessIssue(
                rule="S004",
                severity="warn",
                model=model_name,
                column=column_name,
                raw_table=raw_table,
                message=f"raw profile report status is `{status}`",
                fix="Complete profiling and update status to `Accepted` when ready.",
            )
        )

    return issues


def _column_meta(column: Mapping[str, Any]) -> Mapping[str, Any]:
    config = column.get("config", {})
    if isinstance(config, Mapping):
        meta = config.get("meta", {})
        if isinstance(meta, Mapping):
            return meta
    meta = column.get("meta", {})
    if isinstance(meta, Mapping):
        return meta
    return {}


def _report_status(text: str) -> str | None:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("状态："):
            return stripped.removeprefix("状态：").strip()
        if stripped.startswith("Status:"):
            return stripped.removeprefix("Status:").strip()
    return None


def _is_staging_model(node: Mapping[str, Any]) -> bool:
    if node.get("resource_type") != "model":
        return False
    original_file_path = str(node.get("original_file_path", ""))
    return original_file_path.startswith("models/staging/")


def _load_json(path: Path) -> Mapping[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


if __name__ == "__main__":
    main()
