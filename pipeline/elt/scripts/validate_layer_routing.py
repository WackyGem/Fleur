from __future__ import annotations

import json
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import click
import yaml

ELT_ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ELT_ROOT / "target" / "manifest.json"
SOURCES_PATH = ELT_ROOT / "models" / "sources.yml"

EXPECTED_MODEL_SCHEMAS = {
    "staging": "fleur_staging",
    "intermediate": "fleur_intermediate",
    "marts": "fleur_marts",
}
RAW_SOURCE_NAME = "raw"
RAW_SOURCE_SCHEMA = "fleur_raw"
FORBIDDEN_MODEL_SCHEMA = "analytics"


@dataclass(frozen=True)
class RoutingIssue:
    resource: str
    message: str

    def format(self) -> str:
        return f"{self.resource}: {self.message}"


def main() -> None:
    issues = validate()
    if issues:
        click.echo("Layer routing validation failed:", err=True)
        for issue in issues:
            click.echo(f"- {issue.format()}", err=True)
        raise SystemExit(1)
    click.echo("Layer routing validation passed.")


def validate() -> list[RoutingIssue]:
    manifest = _load_json(MANIFEST_PATH)
    sources = _load_yaml(SOURCES_PATH)
    issues: list[RoutingIssue] = []
    issues.extend(_validate_sources_yaml(sources))
    issues.extend(_validate_manifest_sources(manifest))
    issues.extend(_validate_manifest_models(manifest))
    return issues


def _validate_sources_yaml(sources: Mapping[str, Any]) -> list[RoutingIssue]:
    raw_sources = [
        source
        for source in sources.get("sources", [])
        if isinstance(source, Mapping) and source.get("name") == RAW_SOURCE_NAME
    ]
    if len(raw_sources) != 1:
        return [
            RoutingIssue(
                resource="models/sources.yml",
                message="expected exactly one source named raw",
            )
        ]
    raw_source = raw_sources[0]
    schema = raw_source.get("schema")
    if schema != RAW_SOURCE_SCHEMA:
        return [
            RoutingIssue(
                resource="source.raw",
                message=f"schema is {schema!r}, expected {RAW_SOURCE_SCHEMA!r}",
            )
        ]

    issues: list[RoutingIssue] = []
    for table in raw_source.get("tables", []):
        if not isinstance(table, Mapping):
            continue
        table_name = str(table.get("name", "<unknown>"))
        meta = _config_meta(table)
        raw_table = str(meta.get("clickhouse_raw_table", ""))
        if not raw_table.startswith(f"{RAW_SOURCE_SCHEMA}."):
            issues.append(
                RoutingIssue(
                    resource=f"source.raw.{table_name}",
                    message=(
                        "meta.clickhouse_raw_table must point to "
                        f"{RAW_SOURCE_SCHEMA}, got {raw_table!r}"
                    ),
                )
            )
    return issues


def _validate_manifest_sources(manifest: Mapping[str, Any]) -> list[RoutingIssue]:
    issues: list[RoutingIssue] = []
    for source in manifest.get("sources", {}).values():
        if not isinstance(source, Mapping) or source.get("source_name") != RAW_SOURCE_NAME:
            continue
        schema = source.get("schema")
        if schema != RAW_SOURCE_SCHEMA:
            issues.append(
                RoutingIssue(
                    resource=str(source.get("unique_id", "source.raw")),
                    message=f"schema is {schema!r}, expected {RAW_SOURCE_SCHEMA!r}",
                )
            )
    return issues


def _validate_manifest_models(manifest: Mapping[str, Any]) -> list[RoutingIssue]:
    issues: list[RoutingIssue] = []
    for node in manifest.get("nodes", {}).values():
        if not isinstance(node, Mapping) or node.get("resource_type") != "model":
            continue
        name = str(node.get("name", "<unknown>"))
        schema = str(node.get("schema", ""))
        if schema == FORBIDDEN_MODEL_SCHEMA:
            issues.append(
                RoutingIssue(
                    resource=name,
                    message=f"model points to forbidden schema {FORBIDDEN_MODEL_SCHEMA!r}",
                )
            )
        layer = _model_layer(node)
        expected_schema = EXPECTED_MODEL_SCHEMAS.get(layer)
        if expected_schema is not None and schema != expected_schema:
            issues.append(
                RoutingIssue(
                    resource=name,
                    message=f"{layer} model schema is {schema!r}, expected {expected_schema!r}",
                )
            )
    return issues


def _model_layer(node: Mapping[str, Any]) -> str:
    path = str(node.get("original_file_path", ""))
    if path.startswith("models/staging/"):
        return "staging"
    if path.startswith("models/intermediate/"):
        return "intermediate"
    if path.startswith("models/marts/"):
        return "marts"
    return ""


def _config_meta(value: Mapping[str, Any]) -> Mapping[str, Any]:
    config = value.get("config", {})
    if isinstance(config, Mapping):
        meta = config.get("meta", {})
        if isinstance(meta, Mapping):
            return meta
    return {}


def _load_json(path: Path) -> Mapping[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_yaml(path: Path) -> Mapping[str, Any]:
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(data, Mapping):
        msg = f"{path} must contain a YAML mapping"
        raise RuntimeError(msg)
    return data


if __name__ == "__main__":
    main()
