from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import click
import yaml

ELT_ROOT = Path(__file__).resolve().parents[1]
MODELS_ROOT = ELT_ROOT / "models"
MANIFEST_PATH = ELT_ROOT / "target" / "manifest.json"


@dataclass(frozen=True)
class PassthroughModel:
    staging: str
    intermediate: str
    mart: str
    derived_columns: tuple[str, ...] = ()
    required_staging_columns: tuple[str, ...] = ()
    forbidden_columns: tuple[str, ...] = ()


F10_PASSTHROUGH_MODELS: tuple[PassthroughModel, ...] = (
    PassthroughModel(
        staging="stg_eastmoney__balance",
        intermediate="int_stock_balance_sheet",
        mart="mart_stock_balance_sheet",
    ),
    PassthroughModel(
        staging="stg_eastmoney__cashflow_sq",
        intermediate="int_stock_cashflow_statement_quarterly",
        mart="mart_stock_cashflow_statement_quarterly",
    ),
    PassthroughModel(
        staging="stg_eastmoney__cashflow_ytd",
        intermediate="int_stock_cashflow_statement_ytd",
        mart="mart_stock_cashflow_statement_ytd",
    ),
    PassthroughModel(
        staging="stg_eastmoney__dividend_allotment",
        intermediate="int_stock_allotment_event",
        mart="mart_stock_allotment_event",
    ),
    PassthroughModel(
        staging="stg_eastmoney__dividend_main",
        intermediate="int_stock_dividend_plan",
        mart="mart_stock_dividend_plan",
        derived_columns=("dividend_plan_record_key", "dividend_plan_group_key"),
        required_staging_columns=("announcement_identifier",),
        forbidden_columns=("info_code",),
    ),
    PassthroughModel(
        staging="stg_eastmoney__equity_history",
        intermediate="int_stock_share_capital_history",
        mart="mart_stock_share_capital_history",
    ),
    PassthroughModel(
        staging="stg_eastmoney__freeholders",
        intermediate="int_stock_free_float_shareholder_top10",
        mart="mart_stock_free_float_shareholder_top10",
        required_staging_columns=("holder_identifier",),
        forbidden_columns=("holder_eastmoney_code",),
    ),
    PassthroughModel(
        staging="stg_eastmoney__income_sq",
        intermediate="int_stock_income_statement_quarterly",
        mart="mart_stock_income_statement_quarterly",
    ),
    PassthroughModel(
        staging="stg_eastmoney__income_ytd",
        intermediate="int_stock_income_statement_ytd",
        mart="mart_stock_income_statement_ytd",
    ),
)


def load_manifest_model_names() -> set[str]:
    if not MANIFEST_PATH.exists():
        msg = f"Missing dbt manifest: {MANIFEST_PATH}. Run dbt parse before this validator."
        raise ValueError(msg)
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    nodes = manifest.get("nodes")
    if not isinstance(nodes, dict):
        msg = f"Invalid dbt manifest nodes payload: {type(nodes)}"
        raise TypeError(msg)
    model_names: set[str] = set()
    for node in nodes.values():
        if not isinstance(node, dict):
            continue
        if node.get("resource_type") != "model":
            continue
        name = node.get("name")
        if isinstance(name, str):
            model_names.add(name)
    return model_names


def model_yaml_path(model_name: str) -> Path:
    matches = sorted(MODELS_ROOT.glob(f"**/{model_name}.yml"))
    if len(matches) != 1:
        msg = f"Expected one YAML file for {model_name}, found {len(matches)}"
        raise ValueError(msg)
    return matches[0]


def load_model(model_name: str) -> dict[str, Any]:
    path = model_yaml_path(model_name)
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        msg = f"Invalid YAML document in {path}"
        raise TypeError(msg)
    models = data.get("models")
    if not isinstance(models, list):
        msg = f"Missing models list in {path}"
        raise TypeError(msg)
    for model in models:
        if isinstance(model, dict) and model.get("name") == model_name:
            return model
    msg = f"Model {model_name} not found in {path}"
    raise ValueError(msg)


def column_names(model: dict[str, Any]) -> list[str]:
    columns = model.get("columns")
    if not isinstance(columns, list):
        msg = f"Missing columns list for {model.get('name')}"
        raise TypeError(msg)
    names: list[str] = []
    for column in columns:
        if not isinstance(column, dict):
            msg = f"Invalid column entry for {model.get('name')}: {column!r}"
            raise TypeError(msg)
        name = column.get("name")
        if not isinstance(name, str):
            msg = f"Invalid column name for {model.get('name')}: {name!r}"
            raise TypeError(msg)
        names.append(name)
    return names


def expected_downstream_columns(
    staging_columns: list[str],
    *,
    derived_columns: tuple[str, ...],
) -> list[str]:
    return [*derived_columns, *staging_columns]


def validate_no_downstream_source_marker(model_name: str, columns: list[str]) -> list[str]:
    failures: list[str] = []
    if "_eastmoney" in model_name:
        failures.append(f"{model_name}: downstream model name contains _eastmoney")
    for column in columns:
        if "_eastmoney" in column:
            failures.append(f"{model_name}.{column}: downstream column contains _eastmoney")
    return failures


def validate_passthrough(spec: PassthroughModel, manifest_model_names: set[str]) -> list[str]:
    failures: list[str] = []
    for model_name in (spec.staging, spec.intermediate, spec.mart):
        if model_name not in manifest_model_names:
            failures.append(f"{model_name}: missing from dbt manifest")

    staging_model = load_model(spec.staging)
    intermediate_model = load_model(spec.intermediate)
    mart_model = load_model(spec.mart)

    staging_columns = column_names(staging_model)
    intermediate_columns = column_names(intermediate_model)
    mart_columns = column_names(mart_model)
    expected_columns = expected_downstream_columns(
        staging_columns,
        derived_columns=spec.derived_columns,
    )

    if intermediate_columns != expected_columns:
        failures.append(
            f"{spec.intermediate}: expected columns {expected_columns!r}, "
            f"got {intermediate_columns!r}"
        )
    if mart_columns != intermediate_columns:
        failures.append(f"{spec.mart}: mart columns differ from {spec.intermediate}")

    failures.extend(validate_no_downstream_source_marker(spec.intermediate, intermediate_columns))
    failures.extend(validate_no_downstream_source_marker(spec.mart, mart_columns))

    for column in spec.required_staging_columns:
        if column not in staging_columns:
            failures.append(f"{spec.staging}.{column}: required canonical staging column missing")
    for column in spec.forbidden_columns:
        for model_name, columns in (
            (spec.staging, staging_columns),
            (spec.intermediate, intermediate_columns),
            (spec.mart, mart_columns),
        ):
            if column in columns:
                failures.append(f"{model_name}.{column}: forbidden non-canonical column present")
    return failures


@click.command()
def main() -> None:
    """Validate EastMoney F10 staging -> intermediate -> mart passthrough coverage."""
    manifest_model_names = load_manifest_model_names()
    failures: list[str] = []
    for spec in F10_PASSTHROUGH_MODELS:
        failures.extend(validate_passthrough(spec, manifest_model_names))
    if failures:
        for failure in failures:
            click.echo(f"ERROR: {failure}", err=True)
        raise SystemExit(1)
    click.echo("F10 passthrough coverage validation passed.")


if __name__ == "__main__":
    main()
