from __future__ import annotations

from fleur_contracts.clickhouse_types import effective_clickhouse_type
from fleur_contracts.schema import ContractRegistry, DatasetContract


def render_data_dict_markdown(
    registry: ContractRegistry,
    contract: DatasetContract,
) -> str:
    table = registry.glossary_tables.get(contract.dataset)
    title = table.description_zh if table is not None else contract.external.source_description_zh
    raw_by_from = (
        {field.from_: field for field in contract.clickhouse_raw.fields}
        if contract.clickhouse_raw is not None
        else {}
    )

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
        *_raw_metadata_lines(contract),
        "",
        "## 字段链路",
        "",
        _field_lineage_header(contract),
        _field_lineage_separator(contract),
    ]

    parquet_by_name = {field.name: field for field in contract.parquet.fields}
    for index, source_field in enumerate(contract.source.fields, start=1):
        parquet_field = parquet_by_name.get(source_field.name)
        raw_field = raw_by_from.get(source_field.name)
        lines.append(
            _field_lineage_row(
                contract=contract,
                index=index,
                source_field_name=source_field.name,
                source_field_type=source_field.type,
                parquet_field_type=parquet_field.type if parquet_field is not None else "-",
                raw_field_name=raw_field.name if raw_field is not None else "-",
                raw_field_type=(
                    effective_clickhouse_type(raw_field.type, nullable=raw_field.nullable)
                    if raw_field is not None
                    else "-"
                ),
                description=source_field.external_description_zh,
            )
        )

    if contract.dataset_note_zh:
        lines.extend(["", "## 数据集备注", "", contract.dataset_note_zh])
    if contract.validation_notes:
        lines.extend(["", "## 校验记录", ""])
        lines.extend(f"- {note}" for note in contract.validation_notes)
    lines.append("")
    return "\n".join(lines)


def _raw_metadata_lines(contract: DatasetContract) -> list[str]:
    if contract.clickhouse_raw is None:
        return [
            "- Raw asset：不适用",
            "- ClickHouse raw：不适用",
        ]
    if contract.raw_asset_key is None:
        msg = f"{contract.dataset} defines clickhouse_raw without raw_asset_key"
        raise ValueError(msg)
    return [
        f"- Raw asset：`{'/'.join(contract.raw_asset_key)}`",
        f"- ClickHouse raw：`{contract.clickhouse_raw.database}.{contract.clickhouse_raw.table}`",
        f"- 分区策略：`{contract.clickhouse_raw.partition_strategy}`",
        f"- ORDER BY：`({', '.join(contract.clickhouse_raw.order_by)})`",
    ]


def _field_lineage_header(contract: DatasetContract) -> str:
    if contract.clickhouse_raw is None:
        return "| # | 外源字段 | 外源类型 | Parquet 类型 | 中文描述 |"
    return (
        "| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | "
        "ClickHouse 类型 | 中文描述 |"
    )


def _field_lineage_separator(contract: DatasetContract) -> str:
    if contract.clickhouse_raw is None:
        return "|---|----------|----------|--------------|----------|"
    return "|---|----------|----------|--------------|---------------------|-----------------|----------|"


def _field_lineage_row(
    *,
    contract: DatasetContract,
    index: int,
    source_field_name: str,
    source_field_type: str,
    parquet_field_type: str,
    raw_field_name: str,
    raw_field_type: str,
    description: str,
) -> str:
    if contract.clickhouse_raw is None:
        return (
            "| "
            f"{index} | "
            f"`{source_field_name}` | "
            f"`{source_field_type}` | "
            f"`{parquet_field_type}` | "
            f"{description} |"
        )
    return (
        "| "
        f"{index} | "
        f"`{source_field_name}` | "
        f"`{source_field_type}` | "
        f"`{parquet_field_type}` | "
        f"`{raw_field_name}` | "
        f"`{raw_field_type}` | "
        f"{description} |"
    )
