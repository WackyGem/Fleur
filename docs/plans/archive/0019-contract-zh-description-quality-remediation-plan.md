# Plan 0019: Contract 中文字段描述质量修复计划

日期：2026-06-01

关联文档：

- `docs/plans/0018-data-contract-registry-and-staging-layer-plan.md`
- `docs/RFC/0010-data-contract-registry-and-contract-tools.md`
- `pipeline/contracts/README.md`
- `docs/references/data_dict/README.md`

## 1. 背景

Plan 0018 已经把字段事实源收敛到 `pipeline/contracts`，并让 dbt YAML 与 `docs/references/data_dict/*.md` 由 contract 生成。但当前 contract 的中文描述质量没有达到 0018 的验收目标：

- `source.fields[].external_description_zh` 大量只是原始字段名本身，例如 `SECUCODE`、`TOTAL_ASSETS`、`open_num`。
- `pipeline/contracts/glossary/fields.yml` 中大量 `description_zh` 仍是 canonical 字段名，例如 `open`、`volume`、`pct_chg`。
- 生成后的 data_dict 会把这些占位内容继续扩散到人工阅读文档。
- 当前 `fleur-contracts validate` 只能校验结构和引用关系，不能阻止“字段名伪装成中文描述”的低质量内容进入仓库。

本计划专门修复 contract 中文描述质量，并把质量要求沉淀为可执行校验，避免后续新增字段再次退化。

## 2. 当前量化基线

基于 2026-06-01 当前 `pipeline/contracts` 的抽查脚本统计：

| 范围 | 字段总数 | 不合格字段 | 不合格率 | 主要问题 |
|------|----------|------------|----------|----------|
| `baostock__query_history_k_data_plus_daily` | 14 | 12 | 85.7% | 描述等于字段名 |
| `baostock__query_stock_basic` | 6 | 4 | 66.7% | 描述等于字段名 |
| EastMoney 8 个数据集 | 1556 | 1556 | 100.0% | 描述等于字段名 |
| `jiuyan__action_field_compacted` | 18 | 16 | 88.9% | 描述等于字段名 |
| `jiuyan__industry_list` | 17 | 16 | 94.1% | 描述等于字段名 |
| `jiuyan__industry_ocr_snapshot` | 8 | 1 | 12.5% | 描述过短 |
| `sina__trade_calendar` | 1 | 0 | 0.0% | 当前合格 |
| `ths__limit_up_pool_compacted` | 22 | 20 | 90.9% | 描述等于字段名 |
| `glossary/fields.yml` | 43 | 29 | 67.4% | 描述等于字段名或过短 |

结论：这不是少量漏填，而是 contract 迁移时把字段名批量写入 `external_description_zh`，需要一次性治理。

## 3. 目标

本计划完成后应满足：

- 所有 `source.fields[].external_description_zh` 都是中文自然语言描述，不允许为空、不允许等于字段名、不允许只有英文/拼音/缩写。
- 所有 `glossary.fields[].description_zh` 都表达 mono-fleur canonical 字段语义，不允许等于 canonical 字段名。
- data_dict 中展示的中文描述来自高质量 contract/glossary，而不是字段名占位。
- `uv run fleur-contracts validate` 能直接失败并指出低质量描述的位置。
- 后续新增 dataset 或字段时，低质量中文描述会在本地测试或 CI 阶段被拦截。

## 4. 非目标

- 不把 raw/stg contract 改造成业务指标语义层。
- 不要求一次性完成 mart 层或未来语义层字段口径。
- 不强行把供应商所有英文缩写翻译成唯一中文名；不能确认含义的字段必须标记为待核实，而不是伪造语义。
- 不手工修改 generated data_dict 作为事实源；生成物只能由 contract 重新生成。

## 5. 描述质量标准

### 5.1 `external_description_zh`

`external_description_zh` 表达外源或供应商语境下的字段含义，写作标准：

- 必须包含中文字符。
- 必须是字段语义描述，不能只是字段名、英文缩写、拼音或数据库列名。
- 对金额、比例、日期、状态、布尔字段应尽量说明单位或取值语境。
- 对 EastMoney 财务报表字段应保留报表语境，例如“资产负债表”“利润表”“现金流量表”“本期”“同比”“单季度”“年初至报告期末”。
- 对不能确认业务含义的字段，使用明确占位格式：`待核实：供应商字段 <FIELD_NAME>，当前仅确认来自 <dataset> 原始响应。` 这种占位允许短期存在，但必须在 `validation_notes` 或修复报告中统计。

不合格示例：

```yaml
external_description_zh: TOTAL_ASSETS
external_description_zh: open_num
external_description_zh: status
```

合格示例：

```yaml
external_description_zh: 资产负债表披露的资产总计金额。
external_description_zh: 当日涨停开板次数。
external_description_zh: 证券上市状态。
```

### 5.2 `glossary.description_zh`

`glossary.description_zh` 表达 mono-fleur 系统内统一语义，写作标准：

- 必须说明 canonical 字段在系统内代表什么。
- 不写供应商特有字段名，供应商别名留在 dataset contract 中体现。
- 可复用字段要比单个数据集更抽象，例如 `trade_date` 不绑定 BaoStock 或 Sina。

不合格示例：

```yaml
open:
  description_zh: open
```

合格示例：

```yaml
open:
  description_zh: 交易标的在交易日内的开盘价格。
```

### 5.3 `dataset_note_zh`

`dataset_note_zh` 用来记录单个数据集特有的异常值、转换限制、供应商口径差异。不能把单个字段的基础翻译堆到 `dataset_note_zh` 里逃避字段级描述。

## 6. 修复策略

### 6.1 先加可解释审计，再批量修内容

先扩展 `contract_tools` 的静态校验能力，让工具输出明确问题清单：

- dataset、字段名、描述字段路径。
- 问题类型：`missing`、`same_as_field_name`、`identifier_only`、`no_cjk`、`too_short`、`known_placeholder`。
- 当前描述值。

实现位置：

- `pipeline/contract_tools/src/fleur_contracts/description_quality.py`
- `pipeline/contract_tools/src/fleur_contracts/validate.py`
- `pipeline/contract_tools/tests/test_contract_registry.py`

`validate_contracts()` 仍返回 dataset 数量；质量失败通过抛出 `ValueError` 或聚合异常让 CLI 非零退出。

### 6.2 分数据源修复字段描述

按数据源分批修复，避免一个超大 diff 难以 review。

#### Batch A：glossary 和小表

范围：

- `pipeline/contracts/glossary/fields.yml`
- `sina__trade_calendar`
- `baostock__query_stock_basic`
- `baostock__query_history_k_data_plus_daily`
- `jiuyan__industry_list`
- `jiuyan__industry_ocr_snapshot`
- `jiuyan__action_field_compacted`
- `ths__limit_up_pool_compacted`

完成标准：

- glossary 不再有字段名占位。
- 7 个非 EastMoney dataset 不再有 `same_as_field_name`、`identifier_only`、`no_cjk`。
- 已知含义字段写成真实中文语义；不确定字段用 `待核实：...` 格式并在 `validation_notes` 记录。

#### Batch B：EastMoney 证券基础和权益/分红表

范围：

- `eastmoney__dividend_allotment`
- `eastmoney__dividend_main`
- `eastmoney__equity_history`

修复依据：

- 优先使用东方财富接口返回字段名、现有 raw data_dict、字段所在表语境。
- 对日期字段区分公告日、股权登记日、除权除息日、派息日、上市日。
- 对比例字段明确“比例”而非金额。
- 对股本字段区分限售股、无限售流通股、A/B/H 股、变动数和占比。

完成标准：

- 109 个 source 字段中文描述全部通过质量校验。
- 分红配股和股本历史字段的日期、金额、数量、比例语义可读。

#### Batch C：EastMoney 三大财务报表

范围：

- `eastmoney__balance`
- `eastmoney__income_sq`
- `eastmoney__income_ytd`
- `eastmoney__cashflow_sq`
- `eastmoney__cashflow_ytd`

修复依据：

- 按财务报表类型建立术语映射，不逐字段临时翻译：
  - 资产负债表：资产、负债、所有者权益、流动/非流动、合计、期末余额。
  - 利润表：营业收入、营业成本、费用、利润、净利润、每股收益、其他综合收益。
  - 现金流量表：经营/投资/筹资活动现金流入流出、现金及现金等价物、补充资料。
- 识别后缀规则：
  - `_QOQ`：单季度口径或环比/季度派生字段，必须结合当前 contract 的 sq/ytd 语境确认，不可机械写“环比”。
  - `_YOY`：同比变动字段。
  - `_BALANCE`、`_OTHER`、`_OTHERNOTE`：平衡项、其他项或附注项。
- 共用字段如 `SECUCODE`、`SECURITY_CODE`、`REPORT_DATE`、`NOTICE_DATE` 在多个 EastMoney dataset 中保持一致中文描述。

完成标准：

- 1447 个 EastMoney 财务报表 source 字段全部通过质量校验。
- 不确定字段集中保留为 `待核实：...`，比例不得超过 5%；超过 5% 时不能标记计划完成。

### 6.3 重新生成下游文档

每批 contract 修复后运行：

```bash
cd pipeline
uv run fleur-contracts generate
uv run fleur-contracts generate --check
```

生成物包括：

- `pipeline/elt/models/sources.yml`
- `pipeline/elt/models/staging/staging.yml`
- `docs/references/data_dict/*.md`

禁止手工编辑 generated data_dict 修复中文描述。

## 7. 机械校验设计

新增描述质量校验规则：

| 规则 | 失败条件 |
|------|----------|
| `required` | 描述为空、缺失或 YAML null |
| `has_cjk` | 描述不含中文字符 |
| `not_same_as_field` | 描述与字段名大小写无关相等 |
| `not_identifier_only` | 描述整体是单个 identifier，例如 `TOTAL_ASSETS` |
| `min_length` | 描述过短，少于 3 个中文字符 |
| `no_known_placeholders` | 描述为 `TODO`、`TBD`、`未知`、`待补充` 等无上下文占位 |
| `verified_unknown_format` | 允许不确定字段，但必须使用 `待核实：供应商字段 <FIELD_NAME>，...` 格式 |

新增测试：

```text
pipeline/contract_tools/tests/test_contract_registry.py
  - test_source_external_descriptions_are_quality_checked
  - test_glossary_descriptions_are_quality_checked
```

CLI 行为：

```bash
cd pipeline
uv run fleur-contracts validate
```

若失败，应输出类似：

```text
Description quality failed:
- datasets/eastmoney__balance.yml source.fields[TOTAL_ASSETS].external_description_zh: same_as_field_name
- glossary/fields.yml fields.open.description_zh: same_as_field_name
```

## 8. 验收命令

每个 batch 完成后至少运行：

```bash
cd pipeline

uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests
git diff --check
```

全部完成后运行：

```bash
cd pipeline

uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run ruff check contract_tools/src contract_tools/tests
uv run ruff format --check contract_tools/src contract_tools/tests
uv run pyright contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest contract_tools/tests
git diff --check
```

如果本次同时触碰 scheduler/dbt 生成接口，再追加 0018 的完整质量门禁。

## 9. 完成标准

计划完成时应满足：

- 当前 15 个 dataset contract 的 `external_description_zh` 全部通过描述质量校验。
- 当前 glossary 的 `description_zh` 全部通过描述质量校验。
- `uv run fleur-contracts validate` 已经包含描述质量门禁。
- `docs/references/data_dict/*.md` 由修复后的 contract 重新生成且无 diff。
- 修复报告或 PR 描述中列出仍保留 `待核实：...` 的字段数量和原因；该比例不得超过全部 source 字段的 5%。
- 后续新增低质量中文描述时，contract_tools 单元测试和 CLI 校验会失败。

## 10. 禁止模式

- 禁止用字段名、英文缩写或机器翻译痕迹填充中文描述。
- 禁止为了通过校验写无意义中文，例如“字段值”“数据字段”“相关信息”。
- 禁止在 generated data_dict 中手工修中文描述。
- 禁止把不确定字段伪装成确定语义；必须显式写 `待核实：...` 并后续收敛。
- 禁止把 dataset 特例写进全局 glossary。
