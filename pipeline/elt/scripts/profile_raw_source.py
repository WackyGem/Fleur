from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

import click
import yaml

ELT_ROOT = Path(__file__).resolve().parents[1]
PIPELINE_ROOT = ELT_ROOT.parent
REPO_ROOT = PIPELINE_ROOT.parent
SOURCES_PATH = ELT_ROOT / "models" / "sources.yml"
RAW_PROFILE_ROOT = REPO_ROOT / "docs" / "references" / "raw_profile"
ANSI_ESCAPE_PATTERN = re.compile(r"\x1b\[[0-9;]*m")


@dataclass(frozen=True)
class SourceColumn:
    name: str
    data_type: str
    description: str


@dataclass(frozen=True)
class SourceTable:
    source_name: str
    table_name: str
    description: str
    meta: dict[str, object]
    columns: list[SourceColumn]


@dataclass(frozen=True)
class QueryBlock:
    title: str
    sql: str
    limit: int


@dataclass(frozen=True)
class QueryResult:
    title: str
    sql: str
    succeeded: bool
    output: str


def main() -> None:
    try:
        _main()
    except ValueError as exc:
        click.echo(f"Error: {exc}", err=True)
        raise SystemExit(1) from exc


@click.command()
@click.option("--source", "source_name", default="raw", show_default=True)
@click.option("--table", "table_name", required=True)
@click.option("--output", "output_path", type=click.Path(path_type=Path))
@click.option("--sample-limit", default=50, show_default=True, type=int)
@click.option("--top-n", default=20, show_default=True, type=int)
@click.option("--key", "keys", multiple=True)
@click.option("--date-column", "date_columns", multiple=True)
@click.option("--enum-column", "enum_columns", multiple=True)
@click.option("--format-column", "format_columns", multiple=True)
@click.option("--numeric-column", "numeric_columns", multiple=True)
@click.option("--execute", is_flag=True)
@click.option(
    "--status",
    "report_status",
    type=click.Choice(["Draft", "Accepted", "Superseded"]),
    default="Draft",
    show_default=True,
)
def _main(
    *,
    source_name: str,
    table_name: str,
    output_path: Path | None,
    sample_limit: int,
    top_n: int,
    keys: tuple[str, ...],
    date_columns: tuple[str, ...],
    enum_columns: tuple[str, ...],
    format_columns: tuple[str, ...],
    numeric_columns: tuple[str, ...],
    execute: bool,
    report_status: str,
) -> None:
    if sample_limit < 1:
        raise ValueError("--sample-limit must be a positive integer")
    if top_n < 1:
        raise ValueError("--top-n must be a positive integer")

    table = _load_source_table(source_name=source_name, table_name=table_name)
    selected = _selected_columns(
        table=table,
        keys=keys,
        date_columns=date_columns,
        enum_columns=enum_columns,
        format_columns=format_columns,
        numeric_columns=numeric_columns,
    )
    queries = _build_queries(table=table, selected=selected, sample_limit=sample_limit, top_n=top_n)
    results = _execute_queries(queries=queries, sample_limit=sample_limit) if execute else []
    markdown = _render_report(
        table=table,
        selected=selected,
        queries=queries,
        results=results,
        execute=execute,
        report_status=report_status,
    )

    if output_path is None:
        click.echo(markdown)
        return

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(markdown, encoding="utf-8")
    click.echo(f"Wrote {output_path}")


def _load_source_table(*, source_name: str, table_name: str) -> SourceTable:
    payload = yaml.safe_load(SOURCES_PATH.read_text(encoding="utf-8"))
    sources = payload.get("sources", [])
    if not isinstance(sources, list):
        raise ValueError(f"{SOURCES_PATH} does not contain a valid sources list")

    source = next(
        (item for item in sources if isinstance(item, dict) and item.get("name") == source_name),
        None,
    )
    if source is None:
        raise ValueError(f"source `{source_name}` is not defined in {SOURCES_PATH}")

    tables = source.get("tables", [])
    if not isinstance(tables, list):
        raise ValueError(f"source `{source_name}` does not contain a valid tables list")

    table = next(
        (item for item in tables if isinstance(item, dict) and item.get("name") == table_name),
        None,
    )
    if table is None:
        raise ValueError(f"source table `{source_name}.{table_name}` is not defined")

    columns_payload = table.get("columns", [])
    if not isinstance(columns_payload, list):
        raise ValueError(f"source table `{source_name}.{table_name}` has invalid columns")

    columns: list[SourceColumn] = []
    for column in columns_payload:
        if not isinstance(column, dict):
            continue
        columns.append(
            SourceColumn(
                name=str(column.get("name", "")),
                data_type=str(column.get("data_type", "")),
                description=str(column.get("description", "")),
            )
        )

    config = table.get("config", {})
    meta = config.get("meta", {}) if isinstance(config, dict) else {}
    return SourceTable(
        source_name=source_name,
        table_name=table_name,
        description=str(table.get("description", "")),
        meta=dict(meta) if isinstance(meta, dict) else {},
        columns=columns,
    )


def _selected_columns(
    *,
    table: SourceTable,
    keys: tuple[str, ...],
    date_columns: tuple[str, ...],
    enum_columns: tuple[str, ...],
    format_columns: tuple[str, ...],
    numeric_columns: tuple[str, ...],
) -> dict[str, list[str]]:
    column_names = {column.name for column in table.columns}
    selected = {
        "keys": list(keys),
        "date_columns": list(date_columns),
        "enum_columns": list(enum_columns),
        "format_columns": list(format_columns),
        "numeric_columns": list(numeric_columns),
    }
    for group, names in selected.items():
        missing = [name for name in names if name not in column_names]
        if missing:
            raise ValueError(f"{group} contains columns not in source table: {', '.join(missing)}")

    if not selected["date_columns"]:
        selected["date_columns"] = [
            column.name
            for column in table.columns
            if column.data_type.lower().startswith("date") or "date" in column.name.lower()
        ][:4]
    if not selected["format_columns"]:
        selected["format_columns"] = [
            column.name
            for column in table.columns
            if _is_string_type(column.data_type)
            and any(token in column.name.lower() for token in ("code", "secucode"))
        ][:4]
    if not selected["enum_columns"]:
        selected["enum_columns"] = [
            column.name
            for column in table.columns
            if _is_low_cardinality_type(column.data_type)
            or column.data_type.lower() in {"bool", "boolean", "int8", "uint8"}
        ][:6]
    if not selected["numeric_columns"]:
        selected["numeric_columns"] = [
            column.name for column in table.columns if _is_numeric_type(column.data_type)
        ][:8]

    return selected


def _build_queries(
    *,
    table: SourceTable,
    selected: dict[str, list[str]],
    sample_limit: int,
    top_n: int,
) -> list[QueryBlock]:
    relation = _source_relation(table)
    queries = [
        QueryBlock(
            title="样例行",
            sql=f"select *\nfrom {relation}",
            limit=sample_limit,
        ),
        QueryBlock(
            title="行数统计",
            sql=f"select count(*) as row_count\nfrom {relation}",
            limit=1,
        ),
    ]

    if selected["date_columns"]:
        expressions = []
        for column in selected["date_columns"]:
            quoted = _quote_identifier(column)
            safe = _safe_alias(column)
            expressions.extend(
                [
                    f"min({quoted}) as min_{safe}",
                    f"max({quoted}) as max_{safe}",
                    f"countIf(isNull({quoted})) as null_{safe}",
                ]
            )
        queries.append(
            QueryBlock(
                title="日期范围",
                sql=f"select\n    {',\n    '.join(expressions)}\nfrom {relation}",
                limit=1,
            )
        )

    if selected["keys"]:
        key_expr = ", ".join(_quote_identifier(column) for column in selected["keys"])
        queries.append(
            QueryBlock(
                title="候选键重复检查",
                sql=(
                    f"select\n    {key_expr},\n    count(*) as row_count\n"
                    f"from {relation}\n"
                    f"group by {key_expr}\n"
                    "having row_count > 1\n"
                    "order by row_count desc"
                ),
                limit=sample_limit,
            )
        )

    for column in selected["format_columns"]:
        quoted = _quote_identifier(column)
        queries.append(
            QueryBlock(
                title=f"格式分布：{column}",
                sql=(
                    "select\n"
                    f"    countIf(match(toString({quoted}), '^[0-9]{{6}}\\\\.(SH|SZ|BJ)$')) "
                    "as canonical_suffix,\n"
                    f"    countIf(match(toString({quoted}), '^(sh|sz|bj)\\\\.[0-9]{{6}}$')) "
                    "as vendor_prefix,\n"
                    f"    countIf(match(toString({quoted}), '^[0-9]{{6}}$')) as numeric_only,\n"
                    f"    countIf(isNull({quoted}) or toString({quoted}) = '') as empty_or_null,\n"
                    f"    count(*) as row_count\n"
                    f"from {relation}"
                ),
                limit=1,
            )
        )

    for column in selected["enum_columns"]:
        quoted = _quote_identifier(column)
        queries.append(
            QueryBlock(
                title=f"高频取值：{column}",
                sql=(
                    f"select\n    {quoted} as value,\n    count(*) as row_count\n"
                    f"from {relation}\n"
                    f"group by {quoted}\n"
                    "order by row_count desc"
                ),
                limit=top_n,
            )
        )

    for column in selected["numeric_columns"]:
        quoted = _quote_identifier(column)
        queries.append(
            QueryBlock(
                title=f"数值范围：{column}",
                sql=(
                    "select\n"
                    f"    min({quoted}) as min_value,\n"
                    f"    max({quoted}) as max_value,\n"
                    f"    countIf({quoted} = 0) as zero_count,\n"
                    f"    countIf({quoted} < 0) as negative_count,\n"
                    f"    countIf(isNull({quoted})) as null_count,\n"
                    "    count(*) as row_count\n"
                    f"from {relation}"
                ),
                limit=1,
            )
        )

    return queries


def _execute_queries(*, queries: list[QueryBlock], sample_limit: int) -> list[QueryResult]:
    results: list[QueryResult] = []
    for query in queries:
        command = [
            "uv",
            "run",
            "dbt",
            "show",
            "--project-dir",
            "elt",
            "--profiles-dir",
            "elt",
            "--inline",
            query.sql,
            "--limit",
            str(query.limit),
        ]
        completed = subprocess.run(
            command,
            cwd=PIPELINE_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        output = completed.stdout.strip()
        if completed.stderr.strip():
            output = f"{output}\n{completed.stderr.strip()}".strip()
        output = _strip_ansi(output)
        results.append(
            QueryResult(
                title=query.title,
                sql=query.sql,
                succeeded=completed.returncode == 0,
                output=output,
            )
        )
    return results


def _render_report(
    *,
    table: SourceTable,
    selected: dict[str, list[str]],
    queries: list[QueryBlock],
    results: list[QueryResult],
    execute: bool,
    report_status: str,
) -> str:
    today = datetime.now(UTC).date().isoformat()
    failed_queries = execute and results and any(not result.succeeded for result in results)
    status = "Draft" if failed_queries else report_status
    contract_dataset = str(table.meta.get("contract_dataset", table.table_name))
    command = (
        "cd pipeline && uv run python elt/scripts/profile_raw_source.py "
        f"--source {table.source_name} --table {table.table_name} --execute "
        f"--output ../docs/references/raw_profile/{table.table_name}.md"
    )
    column_rows = "\n".join(
        "| "
        + " | ".join(
            [
                column.name,
                column.data_type,
                "待补充",
                "待补充",
                "待补充",
                _truncate(_localize_column_description(column.description), 120),
            ]
        )
        + " |"
        for column in table.columns
    )
    query_section = _render_query_section(queries=queries, results=results)

    return f"""# Raw 数据画像：{table.table_name}

日期：{today}

状态：{status}

关联：

- 数据契约：`pipeline/contracts/datasets/{contract_dataset}.yml`
- dbt source：`source('{table.source_name}', '{table.table_name}')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`{table.source_name}`
- raw 表：`{table.table_name}`
- profiling 命令：`{command}`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`{contract_dataset}`
- ClickHouse raw 表：`{table.meta.get("clickhouse_raw_table", "待补充")}`
- 表说明：{table.description or "待补充"}

## 2. 数据分析发现

基于当前 raw 表的现状分析：

- 数据量与覆盖
  - 总记录数：待补充
  - 覆盖主体数：待补充
  - 日期 / 分区范围：待补充
- 粒度与候选键
  - 观察到的粒度：待补充
  - 候选自然键去重结果：{_format_list(selected["keys"])}
  - 旧候选键或备选键对比：待补充
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：待补充
  - 占位值：待补充
  - 预期缺失：待补充
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：待补充
  - 直接 raw input 参照命中情况：待补充
- 分布与相关性
  - 枚举 top values：待补充
  - 少量值 / 长尾文本：待补充
  - 字段间强相关：待补充
- 时间字段合理性
  - 日期范围：待补充
  - 日期先后关系异常：待补充
  - 批次时间范围：待补充
- 数值字段合理性
  - 负数 / 零值 / 极端值：待补充
  - 单位判断：待补充
- 其他观察
  - 对 staging 设计有影响、但不应在 staging 静默修正的事实：待补充

## 3. 粒度与键

- 观察到的粒度：待补充
- 候选自然键：{_format_list(selected["keys"])}
- 重复检查：待补充
- 粒度注意事项：待补充

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
{column_rows}

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：{_format_list(selected["format_columns"])}
- 观察到的格式：待补充
- 无效样例：待补充
- 建议 staging 处理：待补充

### 日期与时间字段

- 已画像字段：{_format_list(selected["date_columns"])}
- 范围：待补充
- 无效值或占位值：待补充
- 建议 staging 处理：待补充

### 枚举字段

- 已画像字段：{_format_list(selected["enum_columns"])}
- 取值：待补充
- 未知或异常取值：待补充
- 建议 staging 处理：待补充

### 数值字段

- 已画像字段：{_format_list(selected["numeric_columns"])}
- 最小/最大值：待补充
- 负数/零值/极端值：待补充
- 单位假设：待补充
- 建议 staging 处理：待补充

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 待补充 | 待补充 | 待补充 | 待补充 | 待补充 |

## 7. Staging 设计决策

- 重命名：待补充
- 类型转换：待补充
- 标准化：待补充
- NULL 处理：待补充
- 测试：待补充
- YAML 元数据：待补充

## 8. 延后到 Intermediate/Mart

- 跨源 join：待补充
- 需要优先级判断的去重：待补充
- 主数据修正：待补充
- 粒度变化：待补充
- 业务指标逻辑：待补充

## 待确认问题

- [ ] 确认画像发现，并在依赖该报告开展新 staging 工作前更新报告状态。

## 关键 SQL 证据摘要

- 行数：待补充
- 日期 / 分区范围：待补充
- 候选键重复：待补充
- 关键 NULL / 占位值：待补充
- 枚举 / 文本分布：待补充
- 数值范围：待补充

## 9. 验收清单

- [ ] 已抽样 raw source。
- [ ] 已记录行数和日期/分区范围。
- [ ] 已评估粒度和候选键。
- [ ] 已完成关键字段画像。
- [ ] 已列出 staging 转换建议。
- [ ] 已列出延后处理事项。
- [ ] 已提出测试或明确豁免。

## Profiling SQL 与结果

{query_section}
"""


def _render_query_section(*, queries: list[QueryBlock], results: list[QueryResult]) -> str:
    result_by_title = {result.title: result for result in results}
    blocks: list[str] = []
    for query in queries:
        blocks.append(f"### {query.title}\n\n```sql\n{query.sql}\n```")
        result = result_by_title.get(query.title)
        if result is not None:
            status = "成功" if result.succeeded else "失败"
            blocks.append(f"\n结果（{status}）：\n\n```text\n{result.output}\n```")
    return "\n\n".join(blocks)


def _source_relation(table: SourceTable) -> str:
    return "{{ source('" + table.source_name + "', '" + table.table_name + "') }}"


def _quote_identifier(name: str) -> str:
    escaped = name.replace("`", "``")
    return f"`{escaped}`"


def _safe_alias(name: str) -> str:
    return "".join(character.lower() if character.isalnum() else "_" for character in name).strip(
        "_"
    )


def _format_list(values: list[str]) -> str:
    if not values:
        return "待补充"
    return ", ".join(f"`{value}`" for value in values)


def _localize_column_description(description: str) -> str:
    normalized = description.replace("\n", " ")
    match = re.match(r"Raw source column from `([^`]+)` field `([^`]+)`\.(.*)", normalized)
    if match is None:
        return normalized

    source, field, suffix = match.groups()
    return f"来自 `{source}` 原始字段 `{field}`。{suffix}"


def _truncate(value: str, max_length: int) -> str:
    if len(value) <= max_length:
        return value
    return f"{value[: max_length - 3]}..."


def _strip_ansi(value: str) -> str:
    return ANSI_ESCAPE_PATTERN.sub("", value)


def _is_string_type(data_type: str) -> bool:
    lowered = data_type.lower()
    return "string" in lowered or "fixedstring" in lowered


def _is_low_cardinality_type(data_type: str) -> bool:
    return data_type.lower().startswith("lowcardinality")


def _is_numeric_type(data_type: str) -> bool:
    lowered = data_type.lower()
    return any(token in lowered for token in ("int", "float", "decimal"))


if __name__ == "__main__":
    main()
