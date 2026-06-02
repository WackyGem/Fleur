# Plan 0025: raw source profiling before dbt staging implementation

日期：2026-06-02

状态：Draft

关联文档：

- `docs/RFC/0013-raw-source-profiling-before-dbt-staging.md`
- `docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- `docs/ADR/0008-raw-source-profiling-before-dbt-staging.md`
- `docs/RFC/0012-dbt-field-glossary-and-raw-source-governance.md`
- `docs/plans/0024-dbt-field-glossary-and-raw-source-governance-implementation-plan.md`
- `pipeline/elt/README.md`
- `pipeline/elt/models/sources.yml`
- `pipeline/elt/metadata/field_glossary.yml`
- `docs/skills/fleur-harness/SKILL.md`

相关 dbt skills：

- `using-dbt-for-analytics-engineering`
  - `references/discovering-data.md`：staging 前置 raw discovery/profiling 的核心方法。
  - `references/writing-data-tests.md`：把 profile 结论转成高价值 dbt tests。
  - `references/writing-documentation.md`：把 grain、边界和清洗原因写进 docs/YAML。
  - `references/planning-dbt-models.md`：从目标 staging 输出反推 raw inputs 和 edge cases。
- `running-dbt-commands`
  - 统一 `dbt parse`、`dbt show --inline`、`dbt build --select ...`、selector 和 `--quiet` 用法。
- `fetching-dbt-docs`
  - 需要确认 dbt source tests、source freshness、`dbt show`、YAML properties 或 CLI 语法时使用。
- `adding-dbt-unit-test`
  - 后续对复杂 staging normalization macro 或 transformation 做 TDD 时使用；本计划第一版不强制。

## 1. 背景

Plan 0024 已完成 dbt canonical 字段治理第一版：

```text
pipeline/contracts/datasets/*.yml
  -> generated raw sources.yml

pipeline/elt
  -> field_glossary.yml
  -> docs blocks
  -> staging SQL/YAML
  -> validate_field_glossary.py
```

ADR 0007 明确 staging 可以做 source-local、确定性、低业务口径风险的轻清洗。ADR 0008 和 RFC 0013 进一步要求：新增或重写 staging model 前，必须先对对应 raw source/table 做数据质量和数据特征分析。

当前仓库还缺少这些落地能力：

- `docs/references/raw_profile/` 目录和报告模板。
- 可复用的 stg readiness skill。
- 生成 profiling 查询和报告草稿的脚本。
- 检查 staging model 是否有对应 raw profile report 的门禁脚本。
- 已有首批 staging models 对应的 raw profile reports。
- `pipeline/elt/README.md` 和 `AGENTS.md` 中的 readiness 检查命令。

本计划把 RFC 0013 收敛成可执行阶段，并把 dbt skills 的工作流显式写入每个阶段。

## 2. 目标

完成后应满足：

1. 每个新增或重写的 staging model 都有对应 raw profile report。
2. raw profile report 有固定模板，覆盖 row count、grain、key、null、占位值、枚举、格式、数值范围、recommended staging transformations 和 deferred items。
3. `profile_raw_source.py` 能基于 generated `sources.yml` 和 dbt source 生成 profile report 草稿与 profiling SQL。
4. `validate_staging_readiness.py` 能发现 staging model 使用的 raw table 缺少 profile report。
5. `docs/skills/stg-model-readiness/SKILL.md` 固化 agent 在写 staging 前的工作流。
6. 现有首批 staging models 至少有对应 profile reports，或有明确的 deferred/legacy 处理说明。
7. dbt staging 本地检查加入 readiness 校验，但不和 field glossary lint 混淆。

## 3. 非目标

本计划不做以下事情：

1. 不把 profiling report 写回 `pipeline/contracts/datasets/*.yml`。
2. 不把 `pipeline/contracts` 扩展成 staging 清洗或 canonical 字段治理工具。
3. 不引入外部数据质量平台。
4. 不一次性 profile 所有 raw tables，只覆盖当前或已有 staging model 的 raw inputs。
5. 不自动生成 staging SQL。
6. 不替代 dbt tests、contract validation 或 ClickHouse raw schema validation。
7. 不在第一版实现复杂异常检测、可视化或 profile result 长期存储表。

## 4. 设计边界

### 4.1 职责归属

| 组件 | 职责 |
|------|------|
| `pipeline/contracts` | source/Parquet/ClickHouse raw 字段事实 |
| `pipeline/contract_tools` | contract validate/generate、raw source catalog、data dict |
| `pipeline/elt/models/sources.yml` | dbt 内 raw source catalog，profiling 的输入之一 |
| `docs/references/raw_profile/*.md` | raw 真实数据质量和数据特征观察 |
| `pipeline/elt/scripts/profile_raw_source.py` | 生成或执行 raw profiling 查询，产出报告草稿 |
| `pipeline/elt/scripts/validate_staging_readiness.py` | 检查 staging 是否具备 raw profile 前置文档 |
| `pipeline/elt/scripts/validate_field_glossary.py` | 校验 staging 字段治理、glossary、source_columns 和 tests |
| `docs/skills/stg-model-readiness/SKILL.md` | agent 写 staging 前的操作手册 |

### 4.2 dbt skills 映射

| 工作 | 使用 skill | 约束 |
|------|------------|------|
| 识别目标 raw source/table | `using-dbt-for-analytics-engineering` / discovering-data | 使用 `dbt ls --select "source:raw.*"` 和 `sources.yml` |
| 抽样和 EDA | `using-dbt-for-analytics-engineering` / discovering-data + `running-dbt-commands` | 使用 `dbt show --inline`，limit 用 `--limit` |
| 写 profile report | writing-documentation | 描述 grain、清洗原因和 edge cases，不只复述字段名 |
| 设计 tests | writing-data-tests | 只把 profile 证实的断言转成 tests，避免低价值全字段测试 |
| 运行验证 | running-dbt-commands | 定向 `dbt parse`、`dbt build --select ...`，避免全项目误跑 |
| 查 dbt 最新语法 | fetching-dbt-docs | 仅用于 dbt source/data tests/docs/CLI 语法确认 |

## 5. 目标目录

```text
docs/
  references/
    raw_profile/
      README.md
      _template.md
      <dataset>.md
  skills/
    stg-model-readiness/
      SKILL.md

pipeline/elt/
  scripts/
    profile_raw_source.py
    validate_staging_readiness.py
```

脚本暂放 `pipeline/elt/scripts/`。如果后续 dbt 辅助脚本明显增多，再单独评估 `pipeline/elt_tools`，本计划不提前抽包。

## 6. 实施阶段

### Phase 0: 基线确认和 scope 选择

范围：

- `pipeline/elt/models/staging/**`
- `pipeline/elt/models/sources.yml`
- `pipeline/contracts/datasets/*.yml`
- `docs/references/data_dict/*.md`

动作：

- 列出现有 staging models 和其 raw source inputs：

```bash
rg -n "source\\('raw'|source\\(\"raw\"|source_columns" pipeline/elt/models/staging
```

- 用 dbt 列出 raw sources，确认 manifest 可解析：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt ls --project-dir elt --profiles-dir elt --select "source:raw.*" --output json
```

- 选择第一批 profile 对象，建议覆盖现有 staging inputs：
  - `sina__trade_calendar`
  - `baostock__query_history_k_data_plus_daily`
  - `eastmoney__equity_history`

dbt skill 使用：

- 按 `using-dbt-for-analytics-engineering/references/discovering-data.md` 的 scope strategy，只 profile 当前 staging 会使用的表，不全量扫所有 raw tables。
- 按 `running-dbt-commands` 使用定向命令和 selector。

完成标准：

- 有一份当前 staging -> raw table mapping 清单。
- 第一批 raw profile tables 明确。
- 确认不把 profile 工作扩展到所有 raw tables。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
```

### Phase 1: raw profile 文档模板和目录

范围：

- `docs/references/raw_profile/README.md`
- `docs/references/raw_profile/_template.md`

动作：

- 新增 raw profile 目录说明。
- 把 RFC 0013 的 report 模板落成 `_template.md`。
- README 说明状态语义：
  - `Draft`：已生成但未完成全部 profiling。
  - `Accepted`：可作为 staging 设计依据。
  - `Superseded`：raw schema 或数据特征变化后被新报告替代。
- README 说明报告不写回 contract，不替代 dbt tests。

dbt skill 使用：

- 使用 writing-documentation 原则：报告必须写 grain、清洗依据、edge cases 和 deferred items，不只复述字段名。

完成标准：

- 模板包含 RFC 0013 要求的 9 个章节。
- README 明确报告用途、状态、命名和边界。

验证：

```bash
git diff --check -- docs/references/raw_profile
```

### Phase 2: stg-model-readiness skill

范围：

- `docs/skills/stg-model-readiness/SKILL.md`

动作：

- 新增 repo skill，触发场景：
  - 新增 staging model。
  - 重写 staging model。
  - 讨论 staging 清洗规则。
  - 为 raw table 设计 dbt canonical 字段。
- skill 工作流：
  1. 读取 `AGENTS.md`、ADR 0007、ADR 0008、RFC 0013 和本计划。
  2. 读取 `sources.yml`、contract dataset 和 data dict。
  3. 按 discovering-data 执行 inventory、sample、grain、null、enum、format、numeric profiling。
  4. 写 raw profile report。
  5. 把 report 结论转成 staging transformations、tests、YAML meta 和 deferred items。
  6. 再开始写 staging SQL/YAML。
  7. 运行 readiness、field glossary 和 dbt build 验证。

dbt skill 使用：

- 在 skill 内显式引用 `using-dbt-for-analytics-engineering` 的 discovering-data、writing-data-tests、writing-documentation。
- 在 skill 内显式引用 `running-dbt-commands` 的命令偏好。

完成标准：

- 后续 agent 能从 `docs/skills/stg-model-readiness/SKILL.md` 独立执行 staging 前置准备。
- skill 不要求把脚本放进 `pipeline/contracts`。

验证：

```bash
git diff --check -- docs/skills/stg-model-readiness/SKILL.md
```

### Phase 3: profile_raw_source.py 第一版

范围：

- `pipeline/elt/scripts/profile_raw_source.py`

动作：

- 实现 CLI：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py \
  --source raw \
  --table sina__trade_calendar \
  --output ../docs/references/raw_profile/sina__trade_calendar.md
```

- 支持参数：
  - `--source`，默认 `raw`。
  - `--table`，必填。
  - `--output`，可选；缺省输出到 stdout。
  - `--sample-limit`，默认 50。
  - `--top-n`，默认 20。
  - `--key`，可重复。
  - `--date-column`，可重复。
  - `--enum-column`，可重复。
  - `--format-column`，可重复。
  - `--numeric-column`，可重复。
  - `--execute`，第一版可选实现。
- 读取 `pipeline/elt/models/sources.yml`。
- 校验 source/table 是否存在。
- 从 source catalog 提取：
  - raw columns。
  - data_type。
  - table description。
  - contract metadata。
- 生成 Markdown report 草稿。
- 生成标准 profiling SQL blocks：
  - sample。
  - row count。
  - date range。
  - null/empty counts。
  - enum top N。
  - format regex counts。
  - numeric min/max/negative/zero。

实现约束：

- 使用 Python 3.12 语法。
- 使用 `pathlib`。
- 输出稳定、可 review。
- 不写 `pipeline/contracts`。
- 不手动解析 SQL AST。
- `--execute` 如实现，应通过 `uv run dbt show --inline ... --limit ...`，并遵守 running-dbt-commands 的 limit 规则。

dbt skill 使用：

- discovering-data 提供查询类别和报告字段。
- running-dbt-commands 约束 `dbt show --inline` 调用方式。

完成标准：

- 对存在的 source/table 能生成 report 草稿。
- 对不存在的 source/table 以非 0 退出并输出明确错误。
- 生成的 SQL 使用 `{{ source('raw', '<table>') }}`。

验证：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py --source raw --table sina__trade_calendar
uv run python elt/scripts/profile_raw_source.py --source raw --table __missing__ ; test $? -ne 0
```

### Phase 4: 首批 raw profile reports

范围：

- `docs/references/raw_profile/sina__trade_calendar.md`
- `docs/references/raw_profile/baostock__query_history_k_data_plus_daily.md`
- `docs/references/raw_profile/eastmoney__equity_history.md`

动作：

- 对每个 raw table 按 discovering-data 完成：
  - inventory。
  - sample raw data。
  - grain/key。
  - null/placeholder。
  - enum。
  - code/date/numeric format。
  - recommended staging transformations。
  - deferred items。
- 现有 staging 已使用的字段必须覆盖：
  - `trade_date`
  - `date`
  - `code`
  - `SECUCODE`
  - `END_DATE`
- 如果当前 ClickHouse 环境不可用，报告状态保持 `Draft`，并记录无法执行的命令和原因；不能伪造 profile 结果。

dbt skill 使用：

- discovering-data 的 “Complete all 6 steps for every table you will build models on” 作为该阶段约束。
- writing-data-tests 把已证实的断言转成 tests 建议，不做全字段低价值测试。
- writing-documentation 保证报告说明 grain 和 edge cases。

完成标准：

- 三份报告至少达到 `Draft`。
- 如 ClickHouse 可用，至少 `sina__trade_calendar.md` 达到 `Accepted`。
- 每份报告明确 staging transformations 和 deferred items。

验证：

```bash
git diff --check -- docs/references/raw_profile
```

如数据环境可用，追加定向 `dbt show --inline` 查询记录到报告中。

### Phase 5: validate_staging_readiness.py

范围：

- `pipeline/elt/scripts/validate_staging_readiness.py`

动作：

- 读取 `pipeline/elt/target/manifest.json`。
- 识别 `models/staging/**` 下的 model。
- 解析每个 column 的 `config.meta.source_columns`。
- 对每个 source/table 检查：
  - `docs/references/raw_profile/<table>.md` 是否存在。
  - 报告是否包含 `## 9. Acceptance`。
  - 报告是否包含 `状态：`。
  - 报告状态为 `Draft` 时第一版输出 warning，不阻塞；缺失报告输出 error。
- 对 local/derived 字段：
  - 如果有 `source_columns`，按 source table 检查。
  - 如果只有 `derived_from`，检查 derived_from 指向字段是否能追溯到 source_columns；第一版无法追溯时输出 warning。
- 输出格式包含：
  - rule id。
  - model。
  - column。
  - raw table。
  - message。
  - fix。

建议规则：

| Rule | Severity | 含义 |
|------|----------|------|
| S001 | error | staging column 缺少 `source_columns` 且不是明确 derived/local |
| S002 | error | raw profile report 不存在 |
| S003 | error | raw profile report 缺少 acceptance checklist |
| S004 | warn | raw profile report 状态不是 `Accepted` |
| S005 | warn | derived/local 字段无法追溯到 raw input |

完成标准：

- 当前首批 staging models 能被 readiness 脚本检查。
- 缺失 report 时给出明确失败。
- `validate_field_glossary.py` 职责不变。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
```

### Phase 6: dbt README、AGENTS 和门禁更新

范围：

- `pipeline/elt/README.md`
- `AGENTS.md`
- 必要时更新 `docs/skills/fleur-harness/SKILL.md`

动作：

- 在 dbt 入口写入：
  - 新增/重写 staging 前先执行 stg-model-readiness workflow。
  - 修改 staging 后运行 readiness lint。
- 更新本地检查命令：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --project-dir elt --profiles-dir elt --select staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

dbt skill 使用：

- running-dbt-commands 要求 `dbt build` 使用 selector 和 `--quiet --warn-error-options`。

完成标准：

- repo 入口能指向 ADR 0008、RFC 0013、Plan 0025 和 stg-model-readiness skill。
- dbt README 的 checks 包含 readiness。

验证：

```bash
git diff --check -- AGENTS.md pipeline/elt/README.md docs/skills/fleur-harness/SKILL.md
```

### Phase 7: 测试和质量门禁

范围：

- `pipeline/elt/scripts/profile_raw_source.py`
- `pipeline/elt/scripts/validate_staging_readiness.py`
- 可能新增的脚本 tests

动作：

- 如脚本逻辑超过简单 IO，新增 pytest 覆盖：
  - source/table lookup。
  - missing table error。
  - report path detection。
  - status/checklist parsing。
  - source_columns extraction。
- 考虑将脚本测试放在 `pipeline/elt/tests` 或后续 `pipeline/elt_tools/tests`；本计划第一版可先用 CLI smoke test。
- 运行项目现有相关门禁。

完成标准：

- 文档-only 阶段至少 `git diff --check` 通过。
- 脚本阶段至少 CLI smoke test 通过。
- 如果新增 pytest，纳入后续质量命令。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/profile_raw_source.py --source raw --table sina__trade_calendar
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --project-dir elt --profiles-dir elt --select staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

## 7. 验收清单

### Documentation

- [ ] `docs/references/raw_profile/README.md` 已创建。
- [ ] `docs/references/raw_profile/_template.md` 已创建。
- [ ] `docs/skills/stg-model-readiness/SKILL.md` 已创建。
- [ ] `pipeline/elt/README.md` 包含 readiness 检查命令。
- [ ] `AGENTS.md` 指向 ADR 0008 / RFC 0013 / staging readiness workflow。

### Scripts

- [ ] `profile_raw_source.py` 能生成 report 草稿。
- [ ] `profile_raw_source.py` 能校验 source/table 存在。
- [ ] `profile_raw_source.py` 输出标准 profiling SQL。
- [ ] `validate_staging_readiness.py` 能读取 manifest。
- [ ] `validate_staging_readiness.py` 能检查 staging source_columns 对应 raw profile。
- [ ] readiness lint 和 field glossary lint 职责清晰分离。

### First profiles

- [ ] `sina__trade_calendar` 有 raw profile report。
- [ ] `baostock__query_history_k_data_plus_daily` 有 raw profile report。
- [ ] `eastmoney__equity_history` 有 raw profile report。
- [ ] 每份 report 都列出 recommended staging transformations。
- [ ] 每份 report 都列出 deferred items 或明确无 deferred items。

### Validation

- [ ] `git diff --check` 通过。
- [ ] `uv run dbt parse --project-dir elt --profiles-dir elt` 通过。
- [ ] `uv run python elt/scripts/validate_staging_readiness.py` 通过，或只剩明确记录的 Draft warnings。
- [ ] `uv run python elt/scripts/validate_field_glossary.py` 通过。
- [ ] `uv run dbt build --project-dir elt --profiles-dir elt --select staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'` 通过，或记录无法运行原因。

## 8. 禁止模式

- 不把 `profile_raw_source.py` 或 readiness lint 放到 `pipeline/contracts`。
- 不修改 generated `pipeline/elt/models/sources.yml` 来让 profile 通过。
- 不在 report 中伪造未执行的数据观察。
- 不把 `Draft` report 当成永久完成状态。
- 不用全字段 `not_null` / `accepted_values` tests 代替 profiling。
- 不把跨源去重、主数据修正、业务优先级判断写入 staging 清洗建议。
- 不为了 readiness 通过而给 staging column 写假的 `source_columns`。

## 9. 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| profiling 流程过重 | staging 开发变慢 | 只强制当前 staging inputs，脚本先生成草稿和查询 |
| ClickHouse 本地不可用 | 无法完成 Accepted report | 允许 Draft，并记录无法执行命令；不伪造结果 |
| readiness lint 过早阻塞 | Plan 0024 首批 staging 已存在但 profile 后补 | 第一版 Draft 为 warning，缺失 report 为 error |
| 脚本职责膨胀 | 变成另一个数据质量平台 | 第一版只做 report/query/readiness，不做复杂异常检测 |
| dbt tests 过多 | 成本高且噪声大 | 按 writing-data-tests 分层，只保留 profile 证实的高价值 tests |
| contract 边界回流 | 重新混淆 raw contract 和 dbt staging | 禁止脚本进入 contracts，report 不写回 contract |

## 10. 完成后的维护动作

- 若实施完成，将本计划状态改为 `Completed` 并记录完成日期。
- 若 readiness 成为稳定门禁，更新 `AGENTS.md` 的 dbt 检查命令。
- 若 profile script 或 readiness script 变复杂，后续单独评估 `pipeline/elt_tools`。
- 若发现需要机器可读 profile result，新增 RFC 或 plan，不在本计划内顺手扩展。

## 11. 结论

本计划将 RFC 0013 落地为一套可执行的 staging 前置准备流程：使用 dbt skills 做数据发现、测试设计、文档写作和命令规范；使用项目内 `pipeline/elt/scripts` 做 report 生成和 readiness 校验；使用 `docs/references/raw_profile` 保存事实依据。这样可以在不扩展 raw contract 边界、不引入外部平台的前提下，让 staging 清洗规则有稳定、可 review 的数据基础。
