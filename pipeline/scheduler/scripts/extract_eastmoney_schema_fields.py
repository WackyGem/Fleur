from __future__ import annotations

import argparse
import difflib
from collections.abc import Mapping
from pathlib import Path

import yaml

ASSET_NAMES = (
    "eastmoney__balance",
    "eastmoney__cashflow_sq",
    "eastmoney__cashflow_ytd",
    "eastmoney__dividend_allotment",
    "eastmoney__dividend_main",
    "eastmoney__equity_history",
    "eastmoney__income_sq",
    "eastmoney__income_ytd",
)


def extract_field_names(openapi_path: Path) -> tuple[str, ...]:
    document = yaml.safe_load(openapi_path.read_text(encoding="utf-8"))
    if not isinstance(document, Mapping):
        msg = f"OpenAPI file does not contain a mapping: {openapi_path}"
        raise ValueError(msg)

    properties = _nested_mapping(
        document,
        (
            "paths",
            next(iter(document["paths"])),
            "get",
            "responses",
            "200",
            "content",
            "application/json",
            "schema",
            "properties",
            "result",
            "properties",
            "data",
            "items",
            "properties",
        ),
        openapi_path,
    )
    field_names = tuple(str(field_name) for field_name in properties)
    if not field_names:
        msg = f"OpenAPI file has no result.data item fields: {openapi_path}"
        raise ValueError(msg)
    if len(set(field_names)) != len(field_names):
        msg = f"OpenAPI file contains duplicate result.data item fields: {openapi_path}"
        raise ValueError(msg)
    return field_names


def extract_all_field_names(openapi_dir: Path) -> dict[str, tuple[str, ...]]:
    return {
        asset_name: extract_field_names(openapi_dir / f"{asset_name}.yaml")
        for asset_name in ASSET_NAMES
    }


def render_fields_module(field_names_by_asset: Mapping[str, tuple[str, ...]]) -> str:
    lines = [
        "from __future__ import annotations",
        "",
        "# Generated from docs/references/openapi/eastmoney__*.yaml.",
        "# Update with scripts/extract_eastmoney_schema_fields.py when endpoint schemas change.",
        "",
        "EASTMONEY_FIELD_NAMES: dict[str, tuple[str, ...]] = {",
    ]
    for asset_name in ASSET_NAMES:
        lines.append(f"    {asset_name!r}: (")
        for field_name in field_names_by_asset[asset_name]:
            lines.append(f"        {field_name!r},")
        lines.append("    ),")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Extract EastMoney static schema fields from OpenAPI YAML references.",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[3],
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Fail if the checked-in static fields module is out of date.",
    )
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    openapi_dir = repo_root / "docs/references/openapi"
    fields_module_path = (
        repo_root / "pipeline/scheduler/src/scheduler/defs/eastmoney/fields.py"
    )
    rendered = render_fields_module(extract_all_field_names(openapi_dir))

    if args.check:
        current = fields_module_path.read_text(encoding="utf-8")
        if current != rendered:
            diff = "\n".join(
                difflib.unified_diff(
                    current.splitlines(),
                    rendered.splitlines(),
                    fromfile=str(fields_module_path),
                    tofile="generated",
                    lineterm="",
                )
            )
            raise SystemExit(f"EastMoney static fields are out of date:\n{diff}")
        return

    fields_module_path.write_text(rendered, encoding="utf-8")


def _nested_mapping(
    mapping: Mapping[object, object],
    keys: tuple[object, ...],
    path: Path,
) -> Mapping[object, object]:
    current: object = mapping
    for key in keys:
        if not isinstance(current, Mapping) or key not in current:
            msg = f"OpenAPI file is missing field path at {key!r}: {path}"
            raise ValueError(msg)
        current = current[key]
    if not isinstance(current, Mapping):
        msg = f"OpenAPI field path is not a mapping in {path}"
        raise ValueError(msg)
    return current


if __name__ == "__main__":
    main()
