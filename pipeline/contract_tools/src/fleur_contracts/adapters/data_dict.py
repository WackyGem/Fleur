from __future__ import annotations

from fleur_contracts.schema import ContractRegistry, DatasetContract


def render_data_dict_markdown(
    registry: ContractRegistry,
    contract: DatasetContract,
) -> str:
    table = registry.glossary_tables.get(contract.dataset)
    title = table.description_zh if table is not None else contract.external.source_description_zh
    raw_by_from = {field.from_: field for field in contract.clickhouse_raw.fields}
    stg_by_from = {}
    if contract.dbt_staging is not None and contract.dbt_staging.status == "active":
        stg_by_from = {field.from_: field for field in contract.dbt_staging.fields}

    lines = [
        f"# {contract.dataset} 数据字典",
        "",
        f"本文件由 `pipeline/contracts/datasets/{contract.dataset}.yml` 生成。字段事实以 contract 为准。",
        "",
        f"- 数据集：`{contract.dataset}`",
        f"- 版本：`{contract.version}`",
        f"- 说明：{title}",
        f"- 粒度：{contract.grain}",
        f"- Source asset：`{'/'.join(contract.source_asset_key)}`",
        f"- Raw asset：`{'/'.join(contract.raw_asset_key)}`",
        f"- ClickHouse raw：`{contract.clickhouse_raw.database}.{contract.clickhouse_raw.table}`",
        f"- 分区策略：`{contract.clickhouse_raw.partition_strategy}`",
        f"- ORDER BY：`({', '.join(contract.clickhouse_raw.order_by)})`",
        "",
        "## 字段链路",
        "",
        (
            "| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | "
            "ClickHouse 类型 | stg 字段 | 中文描述 |"
        ),
        "|---|----------|----------|--------------|---------------------|-----------------|----------|----------|",
    ]

    parquet_by_name = {field.name: field for field in contract.parquet.fields}
    for index, source_field in enumerate(contract.source.fields, start=1):
        parquet_field = parquet_by_name.get(source_field.name)
        raw_field = raw_by_from.get(source_field.name)
        stg_field = stg_by_from.get(raw_field.name) if raw_field is not None else None
        glossary_description = ""
        if stg_field is not None and stg_field.glossary_key is not None:
            glossary_description = registry.glossary_fields[stg_field.glossary_key].description_zh
        description = glossary_description or source_field.external_description_zh
        lines.append(
            "| "
            f"{index} | "
            f"`{source_field.name}` | "
            f"`{source_field.type}` | "
            f"`{parquet_field.type if parquet_field is not None else '-'}` | "
            f"`{raw_field.name if raw_field is not None else '-'}` | "
            f"`{raw_field.type if raw_field is not None else '-'}` | "
            f"`{stg_field.name if stg_field is not None else '-'}` | "
            f"{description} |"
        )

    if contract.dataset_note_zh:
        lines.extend(["", "## 数据集备注", "", contract.dataset_note_zh])
    if contract.validation_notes:
        lines.extend(["", "## 校验记录", ""])
        lines.extend(f"- {note}" for note in contract.validation_notes)
    lines.append("")
    return "\n".join(lines)
