"""从 dataset contract 生成 EastMoney 显式 schema 定义。

用法:
  cd pipeline
  uv run python scheduler/scripts/generate_eastmoney_schemas.py
"""

from __future__ import annotations

import argparse
import difflib
import json
from collections.abc import Mapping
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parents[3]
CONTRACT_DIR = REPO_ROOT / "pipeline" / "contracts" / "datasets"
SCHEMAS_MODULE_PATH = (
    REPO_ROOT / "pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py"
)

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

PA_TYPE_MAP = {
    "string": "pa.string()",
    "date32[day]": "pa.date32()",
    "date32": "pa.date32()",
    "bool": "pa.bool_()",
    "int64": "pa.int64()",
    "float64": "pa.float64()",
    "double": "pa.float64()",
    "timestamp[ns]": 'pa.timestamp("ns")',
    "timestamp[ns, tz=UTC]": 'pa.timestamp("ns", tz="UTC")',
    "time32[ms]": 'pa.time32("ms")',
}


def load_parquet_fields(asset_name: str) -> list[tuple[str, str, bool]]:
    """读取 dataset contract 的 parquet 字段，返回字段名、PyArrow 类型代码和 nullable。"""
    contract_path = CONTRACT_DIR / f"{asset_name}.yml"
    document = yaml.safe_load(contract_path.read_text(encoding="utf-8"))
    if not isinstance(document, Mapping):
        msg = f"Dataset contract does not contain a mapping: {contract_path}"
        raise ValueError(msg)

    parquet = document.get("parquet")
    if not isinstance(parquet, Mapping):
        msg = f"Dataset contract is missing parquet mapping: {contract_path}"
        raise ValueError(msg)

    raw_fields = parquet.get("fields")
    if not isinstance(raw_fields, list) or not raw_fields:
        msg = f"Dataset contract has no parquet fields: {contract_path}"
        raise ValueError(msg)

    fields: list[tuple[str, str, bool]] = []
    for raw_field in raw_fields:
        if not isinstance(raw_field, Mapping):
            msg = f"Parquet field is not a mapping in {contract_path}: {raw_field!r}"
            raise ValueError(msg)
        field_name = raw_field.get("name")
        field_type = raw_field.get("type")
        nullable = raw_field.get("nullable", False)
        if not isinstance(field_name, str) or not isinstance(field_type, str):
            msg = f"Parquet field needs string name and type in {contract_path}: {raw_field!r}"
            raise ValueError(msg)
        if not isinstance(nullable, bool):
            msg = f"Parquet field nullable must be bool in {contract_path}: {field_name}"
            raise ValueError(msg)

        pa_type_code = PA_TYPE_MAP.get(field_type)
        if pa_type_code is None:
            msg = f"Unsupported parquet type for {asset_name}.{field_name}: {field_type!r}"
            raise ValueError(msg)
        fields.append((field_name, pa_type_code, nullable))
    return fields


def generate_schema_code(asset_name: str, fields: list[tuple[str, str, bool]]) -> str:
    """生成一个 pa.schema([...]) 定义。"""
    schema_var = asset_name.upper().replace("__", "_") + "_SCHEMA"
    lines = [f"{schema_var} = pa.schema(["]
    for field_name, pa_type_code, nullable in fields:
        lines.append(
            f"    pa.field({json.dumps(field_name)}, {pa_type_code}, nullable={nullable}),"
        )
    lines.append("])")
    return "\n".join(lines)


def render_schemas_module() -> str:
    lines = [
        '"""EastMoney 逐字段显式 schema 定义。',
        "",
        "由 scripts/generate_eastmoney_schemas.py 从 dataset contract 自动生成。",
        '"""',
        "",
        "from __future__ import annotations",
        "",
        "import pyarrow as pa",
        "",
    ]

    for asset_name in ASSET_NAMES:
        fields = load_parquet_fields(asset_name)
        if not fields:
            lines.append(f"# {asset_name}: 无字段（跳过）")
            continue
        lines.append(f"# {asset_name}: {len(fields)} 字段")
        lines.append(generate_schema_code(asset_name, fields))
        lines.append("")

    lines.append("# 端点 asset_name → schema 查表")
    lines.append("EASTMONEY_SCHEMAS: dict[str, pa.Schema] = {")
    for asset_name in ASSET_NAMES:
        fields = load_parquet_fields(asset_name)
        if fields:
            schema_var = asset_name.upper().replace("__", "_") + "_SCHEMA"
            lines.append(f'    "{asset_name}": {schema_var},')
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate EastMoney PyArrow schemas from dataset contracts.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Fail if the checked-in generated schemas module is out of date.",
    )
    args = parser.parse_args()

    rendered = render_schemas_module()
    if args.check:
        current = SCHEMAS_MODULE_PATH.read_text(encoding="utf-8")
        if current != rendered:
            diff = "\n".join(
                difflib.unified_diff(
                    current.splitlines(),
                    rendered.splitlines(),
                    fromfile=str(SCHEMAS_MODULE_PATH),
                    tofile="generated",
                    lineterm="",
                )
            )
            raise SystemExit(f"EastMoney generated schemas are out of date:\n{diff}")
        return

    SCHEMAS_MODULE_PATH.write_text(rendered, encoding="utf-8")


if __name__ == "__main__":
    main()
