"""从 data_dict MD 文件生成 EastMoney 显式 schema 定义。

用法:
  cd pipeline
  uv run python scheduler/scripts/generate_eastmoney_schemas.py \
    > scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py
"""
from __future__ import annotations

import re
from pathlib import Path

DATA_DICT_DIR = Path(__file__).resolve().parents[3] / "docs" / "references" / "data_dict"

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

# data_dict 中的 PyArrow 类型名 → Python 代码
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

# 行匹配: | # | field_name | openapi_type | used | pa_type |
ROW_RE = re.compile(
    r"^\|\s*\d+\s*\|\s*(\S+)\s*\|\s*\S+\s*\|\s*[✅❌]\s*\|\s*(\S+)\s*\|$"
)


def parse_data_dict(asset_name: str) -> list[tuple[str, str]]:
    """解析 data_dict MD 文件，返回 [(field_name, pa_type_code), ...]。"""
    md_path = DATA_DICT_DIR / f"{asset_name}.md"
    fields: list[tuple[str, str]] = []
    for line in md_path.read_text(encoding="utf-8").splitlines():
        m = ROW_RE.match(line)
        if m:
            field_name = m.group(1)
            pa_type_raw = m.group(2)
            if pa_type_raw == "-":
                continue  # 未使用的字段
            pa_type_code = PA_TYPE_MAP.get(pa_type_raw)
            if pa_type_code is None:
                print(f"WARNING: unknown type '{pa_type_raw}' for {asset_name}.{field_name}")
                pa_type_code = "pa.string()"
            fields.append((field_name, pa_type_code))
    return fields


def generate_schema_code(asset_name: str, fields: list[tuple[str, str]]) -> str:
    """生成一个 pa.schema([...]) 定义。"""
    schema_var = asset_name.upper().replace("__", "_") + "_SCHEMA"
    lines = [f"{schema_var} = pa.schema(["]
    for field_name, pa_type_code in fields:
        lines.append(f'    pa.field("{field_name}", {pa_type_code}),')
    lines.append("])")
    return "\n".join(lines)


def main() -> None:
    print('"""EastMoney 逐字段显式 schema 定义。')
    print("")
    print("由 scripts/generate_eastmoney_schemas.py 从 data_dict 自动生成。")
    print('"""')
    print("")
    print("from __future__ import annotations")
    print("")
    print("import pyarrow as pa")
    print("")

    for asset_name in ASSET_NAMES:
        fields = parse_data_dict(asset_name)
        if not fields:
            print(f"# {asset_name}: 无字段（跳过）")
            continue
        print(f"# {asset_name}: {len(fields)} 字段")
        print(generate_schema_code(asset_name, fields))
        print("")

    # 生成查表字典
    print("# 端点 asset_name → schema 查表")
    print("EASTMONEY_SCHEMAS: dict[str, pa.Schema] = {")
    for asset_name in ASSET_NAMES:
        fields = parse_data_dict(asset_name)
        if fields:
            schema_var = asset_name.upper().replace("__", "_") + "_SCHEMA"
            print(f'    "{asset_name}": {schema_var},')
    print("}")


if __name__ == "__main__":
    main()
