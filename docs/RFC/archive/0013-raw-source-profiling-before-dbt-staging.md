# RFC 0013: dbt staging 前置 raw source profiling 工作流

状态：草案（2026-06-02）

## 摘要

本文档把 ADR 0008 的长期决策展开为可实施的工作流：新增或重写 dbt staging model 前，必须先对对应 ClickHouse raw source/table 做数据质量与数据特征分析，并把结果记录成可复用报告。

第一版采用项目内轻量方案：

```text
generated raw source catalog
  -> raw profiling query/script
  -> docs/references/raw_profile/<dataset>.md
  -> staging SQL / YAML / tests / glossary updates
  -> manifest lint and dbt build
```

核心目标不是引入新平台，而是让每个 stg model 的清洗规则、字段命名、tests 和例外说明都有可追溯的数据依据。

## 背景

当前链路已经稳定为：

```text
source payload
  -> Dagster source asset
  -> S3 Parquet
  -> ClickHouse raw table
  -> dbt source()
  -> dbt staging
  -> dbt intermediate / mart
```

已有治理边界：

- `pipeline/contracts/datasets/*.yml` 管理 source、Parquet 和 ClickHouse raw 字段事实。
- `pipeline/contract_tools` 生成 `pipeline/elt/models/sources.yml` 和数据字典文档。
- `pipeline/elt/metadata/field_glossary.yml` 管理 dbt canonical 字段。
- `pipeline/elt/scripts/validate_field_glossary.py` 校验 staging YAML、manifest 和 raw source catalog 的一致性。
- ADR 0007 明确 staging 可以做 source-local、确定性、低业务口径风险的轻清洗。

问题在于：contract 和 generated source catalog 能说明 raw 表“应当有什么字段”，但不能说明真实 raw 数据的分布、异常、重复、占位值和格式漂移。如果 stg 建模只依赖字段名和描述，容易出现以下问题：

- 证券代码、日期、枚举值、金额单位等清洗规则基于猜测。
- tests 只覆盖理想情况，没有覆盖实际脏值。
- 重复、grain 和自然键问题过早或错误地在 staging 处理。
- 本应推迟到 intermediate/mart 的跨源判断被放入 staging。
- 后续 review 无法判断某个清洗规则来自真实数据还是经验假设。

## 目标

1. 为每个新增或重写的 staging model 建立 raw source profiling 前置步骤。
2. 固定 raw profile report 的目录、模板和最低内容要求。
3. 提供可脚本化的 profiling 查询生成或报告草稿生成方式。
4. 让 staging SQL/YAML/tests/glossary 更新显式引用 profiling 结论。
5. 提供 readiness 校验，防止 staging model 绕过 raw profiling。
6. 保持 raw contract 与 dbt staging 治理边界，不把 profiling 结论写回 contract。

## 非目标

1. 不引入 Great Expectations、Soda、OpenMetadata、DataHub 等外部平台。
2. 不要求一次性 profile 所有 raw tables。
3. 不把 profiling report 作为持续数据质量测试的替代品。
4. 不把 profiling 结论写入 `pipeline/contracts/datasets/*.yml`。
5. 不让 `pipeline/contracts` 管理 staging 清洗、canonical 字段或 data tests。
6. 不自动生成 staging SQL；profiling 只提供设计依据。
7. 不在第一版实现复杂统计画像或自动异常检测。

## 关联文档

- `docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- `docs/ADR/0008-raw-source-profiling-before-dbt-staging.md`
- `docs/RFC/archive/0012-dbt-field-glossary-and-raw-source-governance.md`
- `docs/plans/archive/0024-dbt-field-glossary-and-raw-source-governance-implementation-plan.md`
- `pipeline/contracts/README.md`
- `pipeline/elt/README.md`
- `pipeline/elt/models/sources.yml`
- `pipeline/elt/metadata/field_glossary.yml`

## 设计原则

### profiling 是 staging 设计输入

raw profile report 必须在 staging SQL/YAML 定稿前完成。它回答：

- 这个 raw 表真实 grain 是什么？
- 哪些字段能作为候选自然键？
- 哪些字段存在空值、占位值、异常枚举或格式漂移？
- 哪些转换适合 staging 处理？
- 哪些问题必须推迟到 intermediate/mart？
- 哪些字段语义仍不确定，不能为了通过 lint 伪造 canonical 含义？

### dbt 是执行和校验底座

第一版 profiling 优先通过 dbt/ClickHouse 查询执行：

- 用 `dbt show --inline` 做抽样和聚合。
- 用 `source('raw', '<table>')` 保持 dbt lineage 和 profile 一致。
- 用 generated `sources.yml` 获取 raw 字段、类型、描述和 contract metadata。
- 已确认的 source 断言可沉淀为 source-level dbt data tests。
- staging 输出继续由 manifest lint、generic tests 和 `dbt build --select staging` 校验。

### 报告是事实记录，不是 contract

profiling report 保存真实数据观察和建模建议。它不改变 raw contract 的事实源职责，不替代 contract validate，也不把 dbt canonical 字段规则回写到 `pipeline/contracts`。

### 只 profile 当前要建模的 inputs

面对很多 raw tables 时，不做“全表轻扫”。只对当前 staging model 直接使用的 raw table 做完整 profiling。暂不建模的 raw tables 可以明确 deferred。

## 目标目录

第一版新增目录和脚本：

```text
docs/
  references/
    raw_profile/
      README.md
      <dataset>.md

pipeline/elt/
  scripts/
    profile_raw_source.py
    validate_staging_readiness.py

docs/skills/
  stg-model-readiness/
    SKILL.md
```

`profile_raw_source.py` 和 `validate_staging_readiness.py` 暂放 `pipeline/elt/scripts/`。如果后续 dbt 辅助脚本明显增多，再单独评估是否拆出 `pipeline/elt_tools`，不要放入 `pipeline/contracts`。

## Raw Profile Report 模板

每份报告文件命名：

```text
docs/references/raw_profile/<dataset>.md
```

模板：

```markdown
# Raw Profile: <dataset>

日期：YYYY-MM-DD

状态：Draft | Accepted | Superseded

关联：

- Contract: `pipeline/contracts/datasets/<dataset>.yml`
- dbt source: `source('raw', '<dataset>')`
- Generated source catalog: `pipeline/elt/models/sources.yml`
- Planned staging model: `pipeline/elt/models/staging/<source>/stg_<source>__<entity>.sql`

## 1. Scope

- Source name:
- Raw table:
- Profiling command:
- Row count:
- Data range:
- Partition range:

## 2. Grain and Keys

- Observed grain:
- Candidate natural key:
- Duplicate check:
- Grain caveats:

## 3. Column Profile

| Column | Type | Nulls | Empty/placeholder | Distinct/sample | Notes |
|--------|------|-------|-------------------|-----------------|-------|

## 4. Key Field Findings

### Security Code Fields

- Observed formats:
- Invalid samples:
- Recommended staging handling:

### Date and Time Fields

- Range:
- Invalid or placeholder values:
- Recommended staging handling:

### Enum Fields

- Values:
- Unknown or unexpected values:
- Recommended staging handling:

### Numeric Fields

- Min/max:
- Negative/zero/extreme values:
- Unit assumptions:
- Recommended staging handling:

## 5. Data Quality Issues

| Issue | Severity | Evidence | Staging action | Deferred action |
|-------|----------|----------|----------------|-----------------|

## 6. Recommended Staging Transformations

- Rename:
- Cast:
- Normalize:
- Null handling:
- Tests:
- YAML metadata:

## 7. Deferred to Intermediate/Mart

- Cross-source joins:
- Deduplication requiring priority:
- Master-data fixes:
- Grain changes:
- Business metric logic:

## 8. Open Questions

- [ ] Question:

## 9. Acceptance

- [ ] Raw source sampled.
- [ ] Row count and date/partition range recorded.
- [ ] Grain and candidate keys evaluated.
- [ ] Key fields profiled.
- [ ] Staging transformations listed.
- [ ] Deferred items listed.
- [ ] Tests or explicit exemptions proposed.
```

## Profiling 查询范围

第一版 profiling 至少执行以下类别查询。

### Inventory

- `count(*)`
- 数据分区范围，例如 `min(year)` / `max(year)` 或等价字段。
- 日期字段范围，例如 `min(trade_date)` / `max(trade_date)`。
- `limit 50` 抽样。

### Grain and Duplicates

- 候选自然键分组重复数。
- 按日期/分区的行数分布。
- 如果无明显自然键，记录无法确认原因。

### Nulls and Placeholders

关键字段：

- null count / null rate。
- 空字符串 count。
- 常见占位值 count，例如 `--`、`N/A`、`0000-00-00`、`1970-01-01`。

### Enums

低基数字段：

- distinct values。
- value counts top N。
- unexpected values。

### Formats

高价值字符串字段：

- 长度分布。
- regex 分组计数。
- 异常样本。

证券代码字段至少检查：

- `^[0-9]{6}\.(SH|SZ|BJ)$`
- `^(sh|sz|bj)\.[0-9]{6}$`
- `^[0-9]{6}$`
- 其他格式。

### Numeric Ranges

数值字段：

- min / max。
- zero count。
- negative count。
- quantiles 或 top extreme samples。
- 单位判断证据。

## CLI 设计

### profile_raw_source.py

目标：生成 profiling 查询和报告草稿。第一版可以先输出 Markdown 到 stdout 或写入目标文件。

命令：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py \
  --source raw \
  --table baostock__query_history_k_data_plus_daily \
  --output ../docs/references/raw_profile/baostock__query_history_k_data_plus_daily.md
```

可选参数：

- `--source`：dbt source name，默认 `raw`。
- `--table`：dbt source table name，必填。
- `--output`：报告输出路径。
- `--sample-limit`：抽样行数，默认 50。
- `--top-n`：枚举 top N，默认 20。
- `--key`：候选自然键，可重复。
- `--date-column`：重点日期字段，可重复。
- `--enum-column`：重点枚举字段，可重复。
- `--format-column`：重点格式字段，可重复。
- `--numeric-column`：重点数值字段，可重复。
- `--execute`：实际运行 profiling 查询；未提供时只生成查询和报告框架。

第一版实现可以只依赖：

- `target/manifest.json`
- `models/sources.yml`
- `dbt show --inline`
- Python 标准库和 PyYAML

### validate_staging_readiness.py

目标：检查 staging model 是否有对应 raw profile report。

命令：

```bash
cd pipeline
uv run python elt/scripts/validate_staging_readiness.py
```

校验规则：

1. 每个 `models/staging/**/stg_*.sql` 必须有 YAML column metadata。
2. 每个 staging column 的 `config.meta.source_columns` 指向的 raw table 必须有 `docs/references/raw_profile/<table>.md`。
3. raw profile report 必须包含 acceptance checklist。
4. 如 report 状态不是 `Accepted`，可以先 warn；后续再升级为 error。
5. 如果 staging column 是 derived/local，仍需检查其直接 raw inputs 是否有 profile。

该脚本不校验数据内容，只校验 readiness 文档是否存在和结构是否完整。字段治理仍由 `validate_field_glossary.py` 负责。

## Skill 设计

新增 repo skill：

```text
docs/skills/stg-model-readiness/SKILL.md
```

触发场景：

- 新增 staging model。
- 重写 staging model。
- 讨论 staging 清洗规则。
- 为 raw table 设计 dbt canonical 字段。

工作流：

1. 读取 `AGENTS.md`、ADR 0007、ADR 0008 和本 RFC。
2. 读取 `pipeline/elt/models/sources.yml` 中目标 raw table metadata。
3. 读取对应 `pipeline/contracts/datasets/<dataset>.yml` 和 data dict。
4. 运行或生成 profiling 查询。
5. 写入 `docs/references/raw_profile/<dataset>.md`。
6. 从 report 中提取 staging transformations、tests 和 deferred items。
7. 再开始写 staging SQL/YAML。
8. 运行 `dbt parse`、`validate_field_glossary.py`、`validate_staging_readiness.py` 和定向 `dbt build`。

## 与现有治理的关系

### 与 contracts

`pipeline/contracts` 仍只管理 raw 字段事实。profiling report 可以引用 contract metadata，但不能回写 staging 清洗、canonical 字段或 tests。

### 与 generated sources.yml

`sources.yml` 是 profiling 的输入之一。profile 脚本可以读取它来获取 raw columns、types 和 metadata，但不能手改该文件。

### 与 field_glossary

profiling 发现新的公共 canonical 字段时，字段定义应进入 `pipeline/elt/metadata/field_glossary.yml`。如果字段仅在单个 staging model 内局部使用，应使用 `dictionary_scope: local` 或 derived 例外。

### 与 ADR 0007

profiling report 必须明确区分：

- staging 可处理的 source-local 清洗。
- intermediate/mart 才能处理的跨源、主数据、去重优先级或 grain 改造。

## 实施阶段

### Phase 1: 文档和模板

范围：

- `docs/references/raw_profile/README.md`
- `docs/references/raw_profile/_template.md`
- `docs/skills/stg-model-readiness/SKILL.md`

完成标准：

- raw profile report 模板可直接复制使用。
- skill 明确 staging 前置 profiling 工作流。
- `AGENTS.md` 或 `pipeline/elt/README.md` 有最小入口指针。

验证：

```bash
git diff --check
```

### Phase 2: profile_raw_source.py

范围：

- `pipeline/elt/scripts/profile_raw_source.py`

完成标准：

- 能读取 generated `sources.yml`。
- 能根据目标 raw table 生成 report 草稿。
- 能生成标准 profiling SQL。
- `--execute` 能通过 dbt 或 ClickHouse 运行基础查询。
- 对缺失 source/table 给出明确错误。

验证：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py --source raw --table sina__trade_calendar
```

### Phase 3: 首份 raw profile report

范围：

- `docs/references/raw_profile/sina__trade_calendar.md` 或其他小表。

完成标准：

- 报告包含模板所有必需章节。
- 明确推荐 staging transformations。
- 明确 deferred items 或说明无 deferred items。

验证：

```bash
git diff --check
```

### Phase 4: validate_staging_readiness.py

范围：

- `pipeline/elt/scripts/validate_staging_readiness.py`

完成标准：

- 能读取 dbt manifest 或 staging YAML。
- 能解析 staging columns 的 `meta.source_columns`。
- 能检查对应 raw profile report 是否存在。
- 输出包含 model、column、raw table、缺失原因和修复建议。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
```

### Phase 5: 纳入 dbt staging 开发门禁

范围：

- `pipeline/elt/README.md`
- `AGENTS.md`
- 相关 skills 或 plans

完成标准：

- 修改 staging model 后的本地检查包含 readiness 校验。
- agent 路由明确：写 staging 前先执行 stg-model-readiness skill。

建议命令：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --project-dir elt --profiles-dir elt --select staging
```

## 验收标准

- 新 staging model 不能在没有 raw profile report 的情况下进入完成状态。
- raw profile report 能解释 staging SQL 中每个非平凡清洗规则。
- report 能明确列出不进入 staging 的问题。
- readiness 脚本能发现缺失 profile report 的 staging model。
- field glossary lint 和 readiness lint 职责不混淆。
- contract registry 不重新持有 staging 清洗或 canonical 字段事实。

## 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| profiling 变成形式主义 | 报告存在但不指导 stg 设计 | readiness 只做最低门禁，review 检查清洗规则是否引用 report 证据 |
| 报告维护成本高 | stg 开发变慢 | 只强制当前建模 raw inputs，不做全量 raw profiling |
| 脚本过早复杂化 | 维护成本超过收益 | 第一版只生成报告草稿和标准查询，复杂统计后置 |
| contract 边界回流 | raw contract 被迫理解 stg 清洗 | 明确脚本放在 `pipeline/elt/scripts`，report 放 docs，不写回 contracts |
| 数据量大导致 profile 查询昂贵 | ClickHouse 查询影响开发效率 | 默认抽样和 top N；大表按分区范围或日期窗口 profile |

## 开放问题

1. raw profile report 状态从 `Draft` 升级到 `Accepted` 的责任人是谁。
2. readiness 校验第一版对 `Draft` report 是 warn 还是 error。
3. 是否需要把 profiling 查询结果同时保存为机器可读 JSON。
4. 是否需要为 ClickHouse 大表增加默认采样策略。
5. 是否在 `field_glossary.yml` 中记录 profile report 反向链接。

## 结论

mono-fleur 第一版采用项目内轻量 raw profiling 工作流：报告在 `docs/references/raw_profile/`，脚本在 `pipeline/elt/scripts/`，skill 在 `docs/skills/stg-model-readiness/`。dbt 提供 source metadata、查询执行和 tests 校验，项目脚本和报告模板负责把 staging 前置分析固化为可审查、可复用的工程流程。
