# Data Dictionary

本目录下的数据字典 Markdown 文件由 `pipeline/contracts/datasets/*.yml` 生成。字段事实以 contract registry 为准，不应手工修改生成后的字段表。

常用命令：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```

字段事实入口：

- Dataset contracts：`pipeline/contracts/datasets/`
- 字段 glossary：`pipeline/contracts/glossary/fields.yml`
- 表 glossary：`pipeline/contracts/glossary/tables.yml`
- 生成器：`pipeline/contract_tools/src/fleur_contracts/`
