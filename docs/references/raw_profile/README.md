# Raw Source 画像

本目录记录 ClickHouse raw source/table 的数据质量和数据特征分析。新增或重写 dbt staging model 前，必须先为其直接使用的 raw table 创建 profile report。

用途：

- 为 staging 字段命名、类型转换、格式标准化、tests 和例外说明提供数据依据。
- 明确哪些问题适合在 staging 处理，哪些必须推迟到 intermediate/mart。
- 给后续 review、debug 和字段治理提供可追溯事实。

边界：

- profile report 不写回 `pipeline/contracts/datasets/*.yml`。
- profile report 不替代 dbt tests、contract validation 或 ClickHouse raw schema validation。
- profile report 不管理 dbt canonical 字段；canonical 字段事实源仍是 `pipeline/elt/metadata/field_glossary.yml`。

状态：

- `Draft`：报告已创建，但 profiling 未全部执行或仍有未确认问题。
- `Accepted`：报告可作为 staging 设计依据。
- `Superseded`：raw schema 或数据特征变化后被新报告替代。

命名：

```text
docs/references/raw_profile/<dataset>.md
```

创建报告：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py \
  --source raw \
  --table sina__trade_calendar \
  --output ../docs/references/raw_profile/sina__trade_calendar.md
```

如果本地 ClickHouse 可用，可以追加 `--execute` 把基础查询结果写入报告草稿。报告正文使用中文；字段名、SQL、dbt 命令输出和枚举原值保持原样，便于复现和核对。
