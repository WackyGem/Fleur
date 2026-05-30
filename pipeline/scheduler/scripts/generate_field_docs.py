"""生成 OpenAPI 字段校对文档。

从 OpenAPI YAML 文件中提取字段名和 JSON 类型，
与资产中实际使用的字段进行对比，生成 Markdown 文档。

Usage:
    cd /storage/program/mono-fleur/pipeline
    uv run python scheduler/scripts/generate_field_docs.py
"""

from __future__ import annotations

import re
from datetime import datetime, timezone
from pathlib import Path

import yaml

# 项目根目录
PROJECT_ROOT = Path(__file__).parent.parent.parent.parent
OPENAPI_DIR = PROJECT_ROOT / "docs" / "references" / "openapi"
OUTPUT_DIR = PROJECT_ROOT / "docs" / "references" / "data_dict"

# OpenAPI 文件到资产名的映射
OPENAPI_TO_ASSET = {
    "eastmoney__balance.yaml": "eastmoney__balance",
    "eastmoney__cashflow_sq.yaml": "eastmoney__cashflow_sq",
    "eastmoney__cashflow_ytd.yaml": "eastmoney__cashflow_ytd",
    "eastmoney__dividend_allotment.yaml": "eastmoney__dividend_allotment",
    "eastmoney__dividend_main.yaml": "eastmoney__dividend_main",
    "eastmoney__equity_history.yaml": "eastmoney__equity_history",
    "eastmoney__income_sq.yaml": "eastmoney__income_sq",
    "eastmoney__income_ytd.yaml": "eastmoney__income_ytd",
    "ths__limit_up_pool.yaml": "ths__limit_up_pool",
    "jiuyan__action_field.yaml": "jiuyan__action_field",
    "jiuyan__industry_ocr.yaml": "jiuyan__industry_ocr",
    "jiuyan__industry_list.yaml": "jiuyan__industry_list",
    "sina__calendar.yaml": "sina__trade_calendar",
}


def extract_openapi_fields_eastmoney(spec: dict) -> dict[str, str]:
    """从东方财富 OpenAPI 规范中提取字段名和 JSON 类型。"""
    fields = {}
    try:
        # 路径: paths -> [first path] -> get -> responses -> 200 -> content -> application/json -> schema -> properties -> result -> properties -> data -> items -> properties
        paths = spec.get("paths", {})
        first_path = next(iter(paths.values()), {})
        get_op = first_path.get("get", first_path.get("post", {}))
        response_200 = get_op.get("responses", {}).get("200", {})
        content = response_200.get("content", {}).get("application/json", {})
        schema = content.get("schema", {})
        result_props = schema.get("properties", {}).get("result", {}).get("properties", {})
        data_items = result_props.get("data", {}).get("items", {})
        field_props = data_items.get("properties", {})

        for field_name, field_def in field_props.items():
            json_type = field_def.get("type", "unknown")
            fields[field_name] = json_type
    except (KeyError, StopIteration, AttributeError):
        pass
    return fields


def extract_openapi_fields_ths(spec: dict) -> dict[str, str]:
    """从同花顺 OpenAPI 规范中提取字段名和 JSON 类型。"""
    fields = {}
    try:
        # 路径: paths -> [first path] -> get -> responses -> 200 -> content -> application/json -> schema -> properties -> data -> properties -> info -> items -> properties
        paths = spec.get("paths", {})
        first_path = next(iter(paths.values()), {})
        get_op = first_path.get("get", first_path.get("post", {}))
        response_200 = get_op.get("responses", {}).get("200", {})
        content = response_200.get("content", {}).get("application/json", {})
        schema = content.get("schema", {})
        data_props = schema.get("properties", {}).get("data", {}).get("properties", {})
        info_items = data_props.get("info", {}).get("items", {})
        field_props = info_items.get("properties", {})

        for field_name, field_def in field_props.items():
            json_type = field_def.get("type", "unknown")
            fields[field_name] = json_type
    except (KeyError, StopIteration, AttributeError):
        pass
    return fields


def extract_openapi_fields_jiuyan(spec: dict) -> dict[str, str]:
    """从韭研 OpenAPI 规范中提取字段名和 JSON 类型。"""
    fields = {}
    try:
        # 路径: paths -> [first path] -> post -> responses -> 200 -> content -> application/json -> schema -> properties
        paths = spec.get("paths", {})
        first_path = next(iter(paths.values()), {})
        get_op = first_path.get("get", first_path.get("post", {}))
        response_200 = get_op.get("responses", {}).get("200", {})
        content = response_200.get("content", {}).get("application/json", {})
        schema = content.get("schema", {})

        # 韭研有三种结构：
        # 1. data 是数组: data -> items -> properties (异动字段)
        # 2. data 是对象: data -> result -> items -> properties (行业列表)
        # 3. 直接是 StockThemeItem (OCR)

        data_prop = schema.get("properties", {}).get("data", {})

        if data_prop.get("type") == "array":
            # 结构1: data 是数组
            items = data_prop.get("items", {})
            item_props = items.get("properties", {})
            _extract_jiuyan_fields(item_props, fields, "")
        elif data_prop.get("type") == "object":
            # 结构2: data 是对象，包含 result 数组
            data_props = data_prop.get("properties", {})
            result_prop = data_props.get("result", {})
            if result_prop.get("type") == "array":
                result_items = result_prop.get("items", {})
                result_item_props = result_items.get("properties", {})
                _extract_jiuyan_fields(result_item_props, fields, "")
        else:
            # 结构3: 检查 components/schemas 中的 StockThemeItem
            components = spec.get("components", {}).get("schemas", {})
            stock_theme_item = components.get("StockThemeItem", {})
            if stock_theme_item:
                item_props = stock_theme_item.get("properties", {})
                _extract_jiuyan_fields(item_props, fields, "")
    except (KeyError, StopIteration, AttributeError):
        pass
    return fields


def _extract_jiuyan_fields(props: dict, fields: dict, prefix: str):
    """递归提取韭研字段。"""
    for field_name, field_def in props.items():
        json_type = field_def.get("type", "unknown")
        full_name = f"{prefix}{field_name}" if prefix else field_name
        fields[full_name] = json_type

        if json_type == "object":
            nested_props = field_def.get("properties", {})
            _extract_jiuyan_fields(nested_props, fields, f"{full_name}.")
        elif json_type == "array":
            list_items = field_def.get("items", {})
            list_props = list_items.get("properties", {})
            _extract_jiuyan_fields(list_props, fields, f"{full_name}.")


def extract_openapi_fields_sina(spec: dict) -> dict[str, str]:
    """从新浪 OpenAPI 规范中提取字段名和 JSON 类型。"""
    fields = {}
    try:
        paths = spec.get("paths", {})
        first_path = next(iter(paths.values()), {})
        get_op = first_path.get("get", first_path.get("post", {}))
        response_200 = get_op.get("responses", {}).get("200", {})
        content = response_200.get("content", {})

        # 新浪交易日历是 text/plain 格式，不是 JSON
        if "text/plain" in content:
            # 返回空字段，因为这不是 JSON API
            return {}

        json_content = content.get("application/json", {})
        schema = json_content.get("schema", {})
        field_props = schema.get("properties", {})

        for field_name, field_def in field_props.items():
            json_type = field_def.get("type", "unknown")
            fields[field_name] = json_type
    except (KeyError, StopIteration, AttributeError):
        pass
    return fields


def extract_openapi_fields(openapi_file: Path, asset_name: str) -> dict[str, str]:
    """根据资产类型提取 OpenAPI 字段。"""
    with open(openapi_file, encoding="utf-8") as f:
        spec = yaml.safe_load(f)

    if asset_name.startswith("eastmoney__"):
        return extract_openapi_fields_eastmoney(spec)
    if asset_name.startswith("ths__"):
        return extract_openapi_fields_ths(spec)
    if asset_name.startswith("jiuyan__"):
        return extract_openapi_fields_jiuyan(spec)
    if asset_name.startswith("sina__"):
        return extract_openapi_fields_sina(spec)
    return {}


def get_asset_fields_eastmoney(asset_name: str) -> tuple[str, ...]:
    """获取东方财富资产的字段列表。"""
    # 动态导入以避免循环依赖
    import sys

    sys.path.insert(0, str(PROJECT_ROOT / "pipeline" / "scheduler" / "src"))
    from scheduler.defs.sources.eastmoney.fields import EASTMONEY_FIELD_NAMES

    return EASTMONEY_FIELD_NAMES.get(asset_name, ())


def get_asset_fields_ths() -> tuple[str, ...]:
    """获取同花顺资产的字段列表。"""
    import sys

    sys.path.insert(0, str(PROJECT_ROOT / "pipeline" / "scheduler" / "src"))
    from scheduler.defs.http.schemas import THS_LIMIT_UP_POOL_COLUMNS

    return THS_LIMIT_UP_POOL_COLUMNS


def get_asset_fields_jiuyan_action_field() -> tuple[str, ...]:
    """获取韭研异动字段资产的字段列表。"""
    import sys

    sys.path.insert(0, str(PROJECT_ROOT / "pipeline" / "scheduler" / "src"))
    from scheduler.defs.http.schemas import JIUYAN_ACTION_FIELD_COLUMNS

    # 韭研的字段是从嵌套结构扁平化出来的
    # 需要将嵌套字段名映射到扁平化后的字段名
    # 例如: list.code -> code, list.article.action_info.time -> time
    return JIUYAN_ACTION_FIELD_COLUMNS


def get_asset_fields_jiuyan_industry_list() -> tuple[str, ...]:
    """获取韭研行业列表资产的字段列表。"""
    import sys

    sys.path.insert(0, str(PROJECT_ROOT / "pipeline" / "scheduler" / "src"))
    from scheduler.defs.http.schemas import JIUYAN_INDUSTRY_LIST_COLUMNS

    return JIUYAN_INDUSTRY_LIST_COLUMNS


def get_asset_fields_jiuyan_industry_ocr() -> tuple[str, ...]:
    """获取韭研行业 OCR 资产的字段列表。"""
    import sys

    sys.path.insert(0, str(PROJECT_ROOT / "pipeline" / "scheduler" / "src"))
    from scheduler.defs.sources.jiuyan.ocr_schema import JIUYAN_INDUSTRY_OCR_SCHEMA

    return tuple(JIUYAN_INDUSTRY_OCR_SCHEMA.names)


def get_asset_fields_sina() -> tuple[str, ...]:
    """获取新浪交易日历资产的字段列表。"""
    return ("trade_date",)


def get_asset_fields(asset_name: str) -> tuple[str, ...]:
    """获取资产的字段列表。"""
    if asset_name.startswith("eastmoney__"):
        return get_asset_fields_eastmoney(asset_name)
    if asset_name == "ths__limit_up_pool":
        return get_asset_fields_ths()
    if asset_name == "jiuyan__action_field":
        return get_asset_fields_jiuyan_action_field()
    if asset_name == "jiuyan__industry_list":
        return get_asset_fields_jiuyan_industry_list()
    if asset_name == "jiuyan__industry_ocr":
        return get_asset_fields_jiuyan_industry_ocr()
    if asset_name == "sina__trade_calendar":
        return get_asset_fields_sina()
    return ()


def get_pyarrow_type_mapping(asset_name: str) -> dict[str, str]:
    """获取资产的 PyArrow 类型映射。"""
    import sys

    sys.path.insert(0, str(PROJECT_ROOT / "pipeline" / "scheduler" / "src"))

    if asset_name.startswith("eastmoney__"):
        from scheduler.defs.sources.eastmoney.schema import eastmoney_field_type

        # 获取字段列表
        fields = get_asset_fields_eastmoney(asset_name)
        return {field: str(eastmoney_field_type(field)) for field in fields}

    if asset_name == "ths__limit_up_pool":
        from scheduler.defs.http.schemas import THS_LIMIT_UP_POOL_SCHEMA

        return {field.name: str(field.type) for field in THS_LIMIT_UP_POOL_SCHEMA}

    if asset_name == "jiuyan__action_field":
        from scheduler.defs.http.schemas import JIUYAN_ACTION_FIELD_SCHEMA

        return {field.name: str(field.type) for field in JIUYAN_ACTION_FIELD_SCHEMA}

    if asset_name == "jiuyan__industry_list":
        from scheduler.defs.http.schemas import JIUYAN_INDUSTRY_LIST_SCHEMA

        return {field.name: str(field.type) for field in JIUYAN_INDUSTRY_LIST_SCHEMA}

    if asset_name == "jiuyan__industry_ocr":
        from scheduler.defs.sources.jiuyan.ocr_schema import JIUYAN_INDUSTRY_OCR_SCHEMA

        return {field.name: str(field.type) for field in JIUYAN_INDUSTRY_OCR_SCHEMA}

    if asset_name == "sina__trade_calendar":
        return {"trade_date": "date32[day]"}

    return {}


def generate_markdown(asset_name: str, openapi_fields: dict[str, str], asset_fields: tuple[str, ...], pa_type_mapping: dict[str, str]) -> str:
    """生成 Markdown 文档。"""
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
    openapi_file = OPENAPI_TO_ASSET_REVERSE.get(asset_name, asset_name)

    lines = [
        f"# {asset_name} 字段校对",
        "",
        f"> 生成时间: {now}",
        f"> OpenAPI 文档: {openapi_file}.yaml",
        "",
        "## 字段对比",
        "",
        "| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 |",
        "|---|--------|-------------|---------|-------------|",
    ]

    # 合并所有字段名（OpenAPI + 资产）
    all_fields = list(openapi_fields.keys())
    for field in asset_fields:
        if field not in openapi_fields:
            all_fields.append(field)

    used_count = 0
    unused_count = 0

    for i, field_name in enumerate(all_fields, 1):
        json_type = openapi_fields.get(field_name, "N/A")

        # 对于韭研，需要将嵌套字段名映射到扁平化后的字段名
        is_used = field_name in asset_fields
        if not is_used and asset_name.startswith("jiuyan__"):
            # 检查是否是嵌套字段的最后一部分
            parts = field_name.split(".")
            if len(parts) > 1:
                flat_name = parts[-1]
                is_used = flat_name in asset_fields

        is_used_str = "✅" if is_used else "❌"
        pa_type = pa_type_mapping.get(field_name, "-")

        # 对于韭研，也检查扁平化后的字段名
        if pa_type == "-" and asset_name.startswith("jiuyan__"):
            parts = field_name.split(".")
            if len(parts) > 1:
                flat_name = parts[-1]
                pa_type = pa_type_mapping.get(flat_name, "-")

        if is_used:
            used_count += 1
        else:
            unused_count += 1

        lines.append(f"| {i} | {field_name} | {json_type} | {is_used_str} | {pa_type} |")

    lines.extend(
        [
            "",
            "## 统计",
            "",
            f"- OpenAPI 字段总数: {len(openapi_fields)}",
            f"- 资产使用字段数: {used_count}",
            f"- 未使用字段数: {unused_count}",
            "",
        ]
    )

    return "\n".join(lines)


# 反向映射：资产名 -> OpenAPI 文件名
OPENAPI_TO_ASSET_REVERSE = {v: k.replace(".yaml", "") for k, v in OPENAPI_TO_ASSET.items()}


def main():
    """主函数。"""
    # 创建输出目录
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    for openapi_filename, asset_name in OPENAPI_TO_ASSET.items():
        openapi_file = OPENAPI_DIR / openapi_filename
        if not openapi_file.exists():
            print(f"⚠️  OpenAPI 文件不存在: {openapi_file}")
            continue

        print(f"📄 处理 {asset_name}...")

        # 提取 OpenAPI 字段
        openapi_fields = extract_openapi_fields(openapi_file, asset_name)

        # 获取资产字段
        asset_fields = get_asset_fields(asset_name)

        # 获取 PyArrow 类型映射
        pa_type_mapping = get_pyarrow_type_mapping(asset_name)

        # 生成 Markdown
        markdown = generate_markdown(asset_name, openapi_fields, asset_fields, pa_type_mapping)

        # 写入文件
        output_file = OUTPUT_DIR / f"{asset_name}.md"
        with open(output_file, "w", encoding="utf-8") as f:
            f.write(markdown)

        print(f"✅ 已生成: {output_file}")

    print(f"\n🎉 完成！文档已生成到 {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
