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
FIELD_GLOSSARY_PATH = ELT_ROOT / "metadata" / "field_glossary.yml"
SOURCES_PATH = ELT_ROOT / "models" / "sources.yml"


@dataclass(frozen=True)
class LintIssue:
    rule: str
    model: str
    column: str
    message: str
    fix: str

    def format(self) -> str:
        location = self.model if self.column == "<model>" else f"{self.model}.{self.column}"
        return f"{self.rule} {location}: {self.message} Fix: {self.fix}"


def main() -> None:
    issues = validate()
    if issues:
        click.echo("Field glossary lint failed:", err=True)
        for issue in issues:
            click.echo(f"- {issue.format()}", err=True)
        raise SystemExit(1)
    click.echo("Field glossary lint passed.")


def validate() -> list[LintIssue]:
    manifest = _load_json(MANIFEST_PATH)
    glossary = _load_yaml(FIELD_GLOSSARY_PATH)
    raw_columns = _load_raw_source_columns(SOURCES_PATH)
    glossary_fields = _load_glossary_fields(glossary)
    tests_by_column = _tests_by_model_column(manifest)

    issues: list[LintIssue] = []
    staging_models = [
        node for node in manifest.get("nodes", {}).values() if _is_staging_model(node)
    ]

    for node in staging_models:
        issues.extend(
            _validate_model(
                node=node,
                glossary_fields=glossary_fields,
                raw_columns=raw_columns,
                tests_by_column=tests_by_column,
            )
        )

    return issues


def _validate_model(
    *,
    node: Mapping[str, Any],
    glossary_fields: Mapping[str, Mapping[str, Any]],
    raw_columns: set[tuple[str, str, str]],
    tests_by_column: Mapping[tuple[str, str], set[str]],
) -> list[LintIssue]:
    model_name = str(node.get("name", "<unknown>"))
    model_id = str(node.get("unique_id", model_name))
    columns = node.get("columns", {})
    if not isinstance(columns, Mapping) or not columns:
        return [
            LintIssue(
                rule="R001",
                model=model_name,
                column="<model>",
                message="staging model has no YAML-declared columns",
                fix="Declare every staging output column in the model YAML file.",
            )
        ]

    issues: list[LintIssue] = []
    deprecated_names = _deprecated_names(glossary_fields)

    for column_name, column in columns.items():
        if not isinstance(column, Mapping):
            continue
        column_name = str(column_name)
        meta = _column_meta(column)
        tests = tests_by_column.get((model_id, column_name), set())
        glossary_key = meta.get("glossary_key")
        dictionary_scope = meta.get("dictionary_scope")

        if column_name in deprecated_names:
            issues.append(
                LintIssue(
                    rule="R009",
                    model=model_name,
                    column=column_name,
                    message="column name is deprecated in the dbt field glossary",
                    fix=f"Rename the staging output column to `{deprecated_names[column_name]}`.",
                )
            )

        if not str(column.get("description", "")).strip():
            issues.append(
                LintIssue(
                    rule="R002",
                    model=model_name,
                    column=column_name,
                    message="column is missing a description",
                    fix="Add a YAML description, preferably with a shared docs block.",
                )
            )

        if not str(column.get("data_type", "")).strip():
            issues.append(
                LintIssue(
                    rule="R003",
                    model=model_name,
                    column=column_name,
                    message="column is missing data_type",
                    fix="Set the dbt column data_type.",
                )
            )

        if glossary_key is None:
            issues.extend(
                _validate_local_or_derived_column(
                    model_name=model_name,
                    column_name=column_name,
                    meta=meta,
                    dictionary_scope=dictionary_scope,
                    raw_columns=raw_columns,
                )
            )
            continue

        glossary_key = str(glossary_key)
        field = glossary_fields.get(glossary_key)
        if field is None:
            issues.append(
                LintIssue(
                    rule="R005",
                    model=model_name,
                    column=column_name,
                    message=f"glossary_key `{glossary_key}` is not defined",
                    fix="Add the key to metadata/field_glossary.yml or mark the column local.",
                )
            )
            continue

        issues.extend(
            _validate_glossary_column(
                model_name=model_name,
                column_name=column_name,
                column=column,
                meta=meta,
                glossary_key=glossary_key,
                field=field,
                raw_columns=raw_columns,
                tests=tests,
            )
        )

    return issues


def _validate_glossary_column(
    *,
    model_name: str,
    column_name: str,
    column: Mapping[str, Any],
    meta: Mapping[str, Any],
    glossary_key: str,
    field: Mapping[str, Any],
    raw_columns: set[tuple[str, str, str]],
    tests: set[str],
) -> list[LintIssue]:
    issues: list[LintIssue] = []

    if column_name != glossary_key and not _name_allowed_by_rule(column_name, glossary_key, field):
        issues.append(
            LintIssue(
                rule="R006",
                model=model_name,
                column=column_name,
                message=f"column name does not match glossary_key `{glossary_key}`",
                fix="Use the glossary key as the canonical staging column name or add an allowed rule.",
            )
        )

    issues.extend(
        _validate_source_columns(
            model_name=model_name,
            column_name=column_name,
            meta=meta,
            raw_columns=raw_columns,
        )
    )

    description = str(column.get("description", ""))
    doc_reference = f"doc('field_{glossary_key}')"
    doc_reference_double = f'doc("field_{glossary_key}")'
    doc_blocks = column.get("doc_blocks", [])
    rendered_doc_reference = f"doc.elt.field_{glossary_key}"
    if (
        doc_reference not in description
        and doc_reference_double not in description
        and (
            not isinstance(doc_blocks, list)
            or rendered_doc_reference not in [str(doc_block) for doc_block in doc_blocks]
        )
        and "description_exempt_reason_zh" not in meta
    ):
        issues.append(
            LintIssue(
                rule="R010",
                model=model_name,
                column=column_name,
                message="shared glossary column description does not reference the docs block",
                fix=f"Use `{{{{ doc('field_{glossary_key}') }}}}` or add description_exempt_reason_zh.",
            )
        )

    normalization_macro = field.get("normalization_macro")
    if normalization_macro is not None:
        normalization = meta.get("normalization")
        if not isinstance(normalization, Mapping):
            if "normalization_exempt_reason_zh" not in meta:
                issues.append(
                    LintIssue(
                        rule="R011",
                        model=model_name,
                        column=column_name,
                        message="normalization metadata is missing",
                        fix="Add config.meta.normalization.macro and input_format.",
                    )
                )
        elif (
            normalization.get("macro") != normalization_macro
            and "normalization_exempt_reason_zh" not in meta
        ):
            issues.append(
                LintIssue(
                    rule="R012",
                    model=model_name,
                    column=column_name,
                    message="normalization macro does not match the glossary field",
                    fix=f"Set normalization.macro to `{normalization_macro}`.",
                )
            )
        if (
            isinstance(normalization, Mapping)
            and not str(normalization.get("input_format", "")).strip()
        ):
            issues.append(
                LintIssue(
                    rule="R011",
                    model=model_name,
                    column=column_name,
                    message="normalization input_format is missing",
                    fix="Set config.meta.normalization.input_format.",
                )
            )

    if (
        _requires_data_test(field)
        and not _has_required_data_test(field, tests)
        and "data_test_exempt_reason_zh" not in meta
    ):
        issues.append(
            LintIssue(
                rule="R013",
                model=model_name,
                column=column_name,
                message="required generic data test is missing",
                fix="Add the required data_tests entry or record data_test_exempt_reason_zh.",
            )
        )

    return issues


def _validate_local_or_derived_column(
    *,
    model_name: str,
    column_name: str,
    meta: Mapping[str, Any],
    dictionary_scope: Any,
    raw_columns: set[tuple[str, str, str]],
) -> list[LintIssue]:
    if dictionary_scope != "local":
        return [
            LintIssue(
                rule="R004",
                model=model_name,
                column=column_name,
                message="column has neither glossary_key nor dictionary_scope: local",
                fix="Add config.meta.glossary_key or mark the field as local with lineage.",
            )
        ]

    if "derived_from" in meta:
        if "derivation_note_zh" in meta:
            return []
        return [
            LintIssue(
                rule="R004",
                model=model_name,
                column=column_name,
                message="derived local column is missing derivation_note_zh",
                fix="Explain the derivation in config.meta.derivation_note_zh.",
            )
        ]

    return _validate_source_columns(
        model_name=model_name,
        column_name=column_name,
        meta=meta,
        raw_columns=raw_columns,
    )


def _validate_source_columns(
    *,
    model_name: str,
    column_name: str,
    meta: Mapping[str, Any],
    raw_columns: set[tuple[str, str, str]],
) -> list[LintIssue]:
    source_columns = meta.get("source_columns")
    if not isinstance(source_columns, list) or not source_columns:
        return [
            LintIssue(
                rule="R008",
                model=model_name,
                column=column_name,
                message="source_columns lineage is missing",
                fix="Add config.meta.source_columns with raw source/table/column entries.",
            )
        ]

    issues: list[LintIssue] = []
    for source_column in source_columns:
        if not isinstance(source_column, Mapping):
            issues.append(
                LintIssue(
                    rule="R008",
                    model=model_name,
                    column=column_name,
                    message="source_columns entry must be a mapping",
                    fix="Use source/table/column keys for each source_columns entry.",
                )
            )
            continue

        raw_column_key = (
            str(source_column.get("source", "")),
            str(source_column.get("table", "")),
            str(source_column.get("column", "")),
        )
        if raw_column_key not in raw_columns:
            issues.append(
                LintIssue(
                    rule="R008",
                    model=model_name,
                    column=column_name,
                    message=f"source column {raw_column_key!r} is absent from generated sources.yml",
                    fix="Point source_columns to an existing generated raw source column.",
                )
            )

    return issues


def _column_meta(column: Mapping[str, Any]) -> Mapping[str, Any]:
    meta = column.get("meta")
    if isinstance(meta, Mapping):
        return meta
    config = column.get("config")
    if isinstance(config, Mapping):
        config_meta = config.get("meta")
        if isinstance(config_meta, Mapping):
            return config_meta
    return {}


def _is_staging_model(node: Mapping[str, Any]) -> bool:
    if node.get("resource_type") != "model":
        return False
    original_file_path = str(node.get("original_file_path", ""))
    path = str(node.get("path", ""))
    return original_file_path.startswith("models/staging/") or path.startswith("staging/")


def _load_raw_source_columns(path: Path) -> set[tuple[str, str, str]]:
    payload = _load_yaml(path)
    raw_columns: set[tuple[str, str, str]] = set()
    for source in payload.get("sources", []):
        source_name = str(source.get("name", ""))
        for table in source.get("tables", []):
            table_name = str(table.get("name", ""))
            for column in table.get("columns", []):
                raw_columns.add((source_name, table_name, str(column.get("name", ""))))
    return raw_columns


def _load_glossary_fields(payload: Mapping[str, Any]) -> Mapping[str, Mapping[str, Any]]:
    fields = payload.get("fields", {})
    if not isinstance(fields, Mapping):
        msg = f"{FIELD_GLOSSARY_PATH} fields must be a mapping"
        raise ValueError(msg)
    return {str(key): value for key, value in fields.items() if isinstance(value, Mapping)}


def _tests_by_model_column(manifest: Mapping[str, Any]) -> dict[tuple[str, str], set[str]]:
    tests_by_column: dict[tuple[str, str], set[str]] = {}
    for node in manifest.get("nodes", {}).values():
        if not isinstance(node, Mapping) or node.get("resource_type") != "test":
            continue
        test_name = _test_name(node)
        column_name = _test_column_name(node)
        if column_name is None:
            continue
        for model_id in _test_model_ids(node):
            tests_by_column.setdefault((model_id, column_name), set()).add(test_name)
    return tests_by_column


def _test_name(node: Mapping[str, Any]) -> str:
    test_metadata = node.get("test_metadata")
    if isinstance(test_metadata, Mapping) and test_metadata.get("name") is not None:
        return str(test_metadata["name"])
    return str(node.get("name", ""))


def _test_column_name(node: Mapping[str, Any]) -> str | None:
    if node.get("column_name") is not None:
        return str(node["column_name"])
    test_metadata = node.get("test_metadata")
    if not isinstance(test_metadata, Mapping):
        return None
    kwargs = test_metadata.get("kwargs")
    if isinstance(kwargs, Mapping) and kwargs.get("column_name") is not None:
        return str(kwargs["column_name"])
    return None


def _test_model_ids(node: Mapping[str, Any]) -> list[str]:
    attached_node = node.get("attached_node")
    if isinstance(attached_node, str) and attached_node.startswith("model."):
        return [attached_node]
    depends_on = node.get("depends_on")
    if not isinstance(depends_on, Mapping):
        return []
    nodes = depends_on.get("nodes", [])
    if not isinstance(nodes, list):
        return []
    return [str(item) for item in nodes if str(item).startswith("model.")]


def _deprecated_names(
    glossary_fields: Mapping[str, Mapping[str, Any]],
) -> dict[str, str]:
    deprecated: dict[str, str] = {}
    for key, field in glossary_fields.items():
        names = field.get("deprecated_names", [])
        if isinstance(names, list):
            for name in names:
                deprecated[str(name)] = key
    return deprecated


def _name_allowed_by_rule(
    column_name: str,
    glossary_key: str,
    field: Mapping[str, Any],
) -> bool:
    prefixes = field.get("allowed_name_prefixes", [])
    suffixes = field.get("allowed_name_suffixes", [])
    if isinstance(prefixes, list) and any(
        column_name == f"{prefix}{glossary_key}" for prefix in prefixes
    ):
        return True
    return isinstance(suffixes, list) and any(
        column_name == f"{glossary_key}{suffix}" for suffix in suffixes
    )


def _requires_data_test(field: Mapping[str, Any]) -> bool:
    return "regex" in field or "value_domain" in field or bool(field.get("required_data_tests"))


def _has_required_data_test(field: Mapping[str, Any], tests: set[str]) -> bool:
    required_tests = field.get("required_data_tests")
    if isinstance(required_tests, list) and required_tests:
        return any(str(test_name) in tests for test_name in required_tests)
    if "value_domain" in field:
        return "accepted_values" in tests
    return bool(tests)


def _load_json(path: Path) -> Mapping[str, Any]:
    if not path.exists():
        msg = f"{path} does not exist. Run dbt parse first."
        raise FileNotFoundError(msg)
    return json.loads(path.read_text(encoding="utf-8"))


def _load_yaml(path: Path) -> Mapping[str, Any]:
    if not path.exists():
        msg = f"{path} does not exist"
        raise FileNotFoundError(msg)
    payload = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        msg = f"{path} must contain a YAML mapping"
        raise ValueError(msg)
    return payload


if __name__ == "__main__":
    main()
