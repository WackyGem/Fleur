# Plan 0045: Int/Mart 层字段风格与一致性债务清偿实施计划

日期：2026-06-21
状态：Completed
分支：`refactor/int-mart-field-consistency`

关联文档：

- `docs/issues/archive/debt/0002-2026-06-21-int-field-style-consistency.md`（int 层债务，13 项）
- `docs/issues/archive/debt/0003-2026-06-21-mart-field-style-consistency.md`（mart 层债务，12 项）
- `docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- `docs/ADR/0008-raw-source-profiling-before-dbt-staging.md`

## 目标

1. 清偿 int 层 13 项字段风格与一致性债务（0002 文档 IA1–IA13）
2. 清偿 mart 层 12 项字段风格与一致性债务（0003 文档 A1–D2）
3. int 层先改，mart 层随后透传对齐，不在 mart 层用 alias 掩盖 int 层不一致
4. 所有改动通过 dbt parse、field_glossary 校验和定向 dbt build 验证

## 非目标

- 不修改 staging 层透传字段（`listed_a_shares`/`b_free_share`/`h_free_share` 等源端多市场标记保留），但 IA13 的 stg 层日期字段纠正命名除外
- 不修改 `sources.yml`（contract 生成物，由 `fleur-contracts generate` 管理）
- 不修改 `field_glossary.yml` 的 canonical 字段定义（只可能新增 glossary_key）
- 不考虑下游 Rearview/Racingline/Dagster 消费方影响（本次聚焦 dbt 项目内部）
- 不做历史数据回填或 ClickHouse 物理表迁移（dropped+recreated by dbt run）

## 债务覆盖矩阵

实施阶段与债务编号的映射（确保无遗漏）：

| 阶段 | int 债务 | mart 债务 | 说明 |
|---|---|---|---|
| Phase 1 | IA1 | — | 补全 3 个模型的 config() 块 |
| Phase 2 | IA2 | A2 | order_by 统一为 `(trade_date, security_code)` |
| Phase 3 | IB1 | A3 | BOLL `boll_up/dn` → `boll_upper/lower` |
| Phase 4 | IB3 | A5 | 去除 `a_` 前缀 |
| Phase 5 | IB4, IA6 | A6 | `turnover_rate_actual` → `turnover_rate_free_float` |
| Phase 6 | IB2, IA4, IA11, IE3 | A4 | 百分比口径 `_pct` 后缀统一（政府债已符合，IA11/IE3 随决策关闭） |
| Phase 7 | IA13 | — | stg 日期字段纠正命名 + int 同步 |
| Phase 8 | IA9b/IE1a | — | `int_index_quotes_daily` 移除 `is_suspend` |
| Phase 9 | — | A1 | mart 表名加 `_daily` 后缀 |
| Phase 10 | IA10, IE2 | — | partition_by / unique_key 配置统一 |
| Phase 11 | IC1 | B2 | portfolio 系列 YAML 补全列文档 |
| Phase 12 | ID1, ID2 | C1 | 描述语言统一为中文 |
| Phase 14 | IA12, IE4 | D1, D2 | 测试加固（not_null + 范围测试） |
| Phase 15 | — | B1 | KDJ 参数字段暴露一致性收敛 |

## 设计原则

- **int 先于 mart**：所有字段重命名在 int 层完成，mart 层透传对齐，不在 mart 层保留旧名 alias
- **机械重命名优先**：先做不影响 SQL 逻辑的纯重命名（config、order_by、字段名），后做文档和测试
- **每阶段独立可验证**：每个 Phase 完成后运行 dbt parse + 定向 build，确保不引入编译错误
- **stg 层最小改动**：只有 IA13 需要 stg 层改动（日期字段纠正命名），其余 stg 层不动

## 实施阶段

### Phase 1: 补全缺失的 config() 块（IA1）

**目标**：3 个缺失 config 的 int 模型补全显式声明。

| 模型 | 新增 config |
|---|---|
| `int_government_bond_yields_daily.sql` | `materialized='table'`, `engine='MergeTree()'`, `order_by='trade_date'`, `partition_by='toYear(trade_date)'` |
| `int_stock_basic_snapshot.sql` | `materialized='table'`, `engine='MergeTree()'`, `order_by='security_code'` |
| `int_trade_calendar.sql` | `materialized='table'`, `engine='MergeTree()'`, `order_by='trade_date'` |

**改动文件**：3 个 `.sql` 文件

**验证**：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --select int_government_bond_yields_daily int_stock_basic_snapshot int_trade_calendar --project-dir elt --profiles-dir elt
```

### Phase 2: order_by 统一为 (trade_date, security_code)（IA2 + A2）

**目标**：mart 层 2 张表 order_by 从 `(security_code, trade_date)` 改为 `(trade_date, security_code)`。int 层已符合，无需改动。

| 文件 | 改动 |
|---|---|
| `mart_stock_quotes_daily.sql` | `order_by='(security_code, trade_date)'` → `'(trade_date, security_code)'` |
| `mart_benchmark_returns_daily.sql` | 同上 |

**验证**：

```bash
uv run dbt build --select mart_stock_quotes_daily mart_benchmark_returns_daily --project-dir elt --profiles-dir elt
```

### Phase 3: BOLL 上下轨缩写对称化（IB1 + A3）

**目标**：`boll_up_*` → `boll_upper_*`，`boll_dn_*` → `boll_lower_*`。

**改动文件**（int 先改，mart 透传）：

- `int_stock_boll_daily.sql` + `.yml`（6 个字段重命名）
- `mart_stock_trend_indicator.sql` + `.yml`（9 个 BOLL 字段透传重命名）

**字段映射**：

| 旧名 | 新名 |
|---|---|
| `boll_up_10_1p5` | `boll_upper_10_1p5` |
| `boll_dn_10_1p5` | `boll_lower_10_1p5` |
| `boll_up_20_2` | `boll_upper_20_2` |
| `boll_dn_20_2` | `boll_lower_20_2` |
| `boll_up_50_2p5` | `boll_upper_50_2p5` |
| `boll_dn_50_2p5` | `boll_lower_50_2p5` |

**验证**：

```bash
uv run dbt build --select int_stock_boll_daily mart_stock_trend_indicator --project-dir elt --profiles-dir elt
```

### Phase 4: 去除 a_ 前缀（IB3 + A5）

**目标**：6 个 `a_` 前缀字段去除前缀。语义评估已完成（0002 文档 IB3），安全可行。

**改动文件**（6 个文件）：

| 文件 | 改动 |
|---|---|
| `int_stock_shares_history.sql` | `a_shares`/`a_float_shares`/`a_free_float_shares` 定义改名，上游 `listed_a_shares` 保持不变 |
| `int_stock_shares_history.yml` | 同步列名 + description（明确 `shares` vs `total_shares` 对照） |
| `int_stock_quotes_daily_unadj.sql` | 引用改名后的 shares 字段 + `a_market_cap`/`a_float_market_cap`/`a_free_float_market_cap` 派生改名（CTE alias、派生表达式、final SELECT） |
| `int_stock_quotes_daily_unadj.yml` | 同步 6 个列名 |
| `mart_stock_quotes_daily.sql` | 透传 6 个改名后的字段 |
| `mart_stock_quotes_daily.yml` | 同步 6 个列名 |

**字段映射**：

| 旧名 | 新名 |
|---|---|
| `a_shares` | `shares` |
| `a_float_shares` | `float_shares` |
| `a_free_float_shares` | `free_float_shares` |
| `a_market_cap` | `market_cap` |
| `a_float_market_cap` | `float_market_cap` |
| `a_free_float_market_cap` | `free_float_market_cap` |

**⚠️ 关键**：`int_stock_shares_history.yml` 的 `shares` description 必须明确"A 股股本（流通+限售）"，与 `total_shares`（全市场总股本）区分。`int_stock_financial_valuation` 只引用 `total_shares`，无需改动。

**验证**：

```bash
uv run dbt build --select int_stock_shares_history int_stock_quotes_daily_unadj mart_stock_quotes_daily --project-dir elt --profiles-dir elt
```

### Phase 5: turnover_rate_actual 改名（IB4/IA6 + A6）

**目标**：`turnover_rate_actual` → `turnover_rate_free_float`。影响面 4 个文件（已确认无其他引用）。

**改动文件**：

- `int_stock_quotes_daily_unadj.sql`（定义处 + CTE 引用 + final SELECT）
- `int_stock_quotes_daily_unadj.yml`
- `mart_stock_quotes_daily.sql`（透传）
- `mart_stock_quotes_daily.yml`

**验证**：

```bash
uv run dbt build --select int_stock_quotes_daily_unadj mart_stock_quotes_daily --project-dir elt --profiles-dir elt
```

### Phase 6: 百分比口径 _pct 后缀统一（IB2/IA4 + A4）

**目标**：百分数口径字段统一加 `_pct` 后缀；小数比例口径不加后缀。采用后缀方案与项目现有主流惯例（staging `free_float_holdnum_ratio_pct`、政府债 `*_yield_pct`）对齐。

**字段映射**（int 层定义，mart 透传）：

| 旧名 | 新名 | 口径 |
|---|---|---|
| `pct_amplitude` | `amplitude_pct` | 百分数 |
| `pct_change` | `change_pct` | 百分数 |
| `turnover_rate` | `turnover_rate_pct` | 百分数 |
| `turnover_rate_free_float`（Phase 5 已改名） | `turnover_rate_free_float_pct` | 百分数 |
| `dy_static` | `dy_static_pct` | 百分数 |
| `dy_ttm` | `dy_ttm_pct` | 百分数 |

**保持不变**（小数比例口径，已有 description 标注）：`roe`/`roa`/`roaa`/`roae`/`annual_rate`/`daily_rate`/`return_daily`。政府债 `*_yield_pct` 系列（12 个）已符合后缀方案，无需改动。

**改动文件**：

- `int_stock_quotes_daily_unadj.sql` + `.yml`（6 个字段重命名 + SQL 内引用，含 `pct_amplitude`→`amplitude_pct`、`pct_change`→`change_pct`）
- `int_risk_free_rate_daily.sql` + `.yml`（引用不变，`one_year_yield_pct` 已符合）
- `mart_stock_quotes_daily.sql` + `.yml`（透传 6 个改名后的字段）

**验证**：

```bash
uv run dbt build --select int_stock_quotes_daily_unadj int_government_bond_yields_daily int_risk_free_rate_daily mart_stock_quotes_daily --project-dir elt --profiles-dir elt
```

### Phase 7: stg 日期字段纠正命名（IA13/IE5）

**目标**：纠正 staging 层日期字段命名与事实不符的问题。

**stg 层改动**：

| 文件 | 改动 |
|---|---|
| `stg_eastmoney__equity_history.sql` | `END_DATE as report_date` → `END_DATE as end_date`（股本变动截止日，非报告期） |
| `stg_eastmoney__equity_history.yml` | 列名 `report_date` → `end_date`，description 改为"股本变动截止日，非财报报告期"，移除 `glossary_key: report_date`；unique_combination_of_columns 同步改 `end_date` |
| `stg_eastmoney__freeholders.sql` | `END_DATE as end_date` → `END_DATE as report_date`（报告期） |
| `stg_eastmoney__freeholders.yml` | 列名 `end_date` → `report_date`，description 复用 `{{ doc('field_report_date') }}`，加 `glossary_key: report_date`；natural_key 和 unique_combination_of_columns 同步改 `report_date` |

**int 层同步**：

| 文件 | 改动 |
|---|---|
| `int_stock_shares_history.sql` | equity_history CTE：`report_date as end_date` → `end_date as end_date`；freeholders CTE 及 `freeholders_report_aggregates`：`end_date` → `report_date as end_date`，group by 同步 |
| `int_stock_shares_history.yml` | `source_equity_end_date` / `source_freeholders_end_date` description 保持不变 |

**验证**：

```bash
uv run dbt build --select stg_eastmoney__equity_history stg_eastmoney__freeholders int_stock_shares_history --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
```

### Phase 8: int_index_quotes_daily 移除 is_suspend（IA9b/IE1a）

**目标**：指数无停牌概念，移除无语义字段。

**改动文件**：

- `int_index_quotes_daily.sql`：删除 SELECT 中的 `quotes.is_suspend` 和 final SELECT 的 `is_suspend`
- `int_index_quotes_daily.yml`：删除 `is_suspend` 列定义和 `not_null` 测试

**验证**：

```bash
uv run dbt build --select int_index_quotes_daily int_benchmark_returns_daily --project-dir elt --profiles-dir elt
```

### Phase 9: mart 表名加 _daily 后缀（A1）

**目标**：3 张日频 mart 表统一加 `_daily` 后缀。

| 旧表名 | 新表名 |
|---|---|
| `mart_stock_momentum_indicator` | `mart_stock_momentum_indicator_daily` |
| `mart_stock_trend_indicator` | `mart_stock_trend_indicator_daily` |
| `mart_stock_volume_indicator` | `mart_stock_volume_indicator_daily` |

**改动**：重命名 `.sql` + `.yml` 文件 + model name（YAML `name:` 字段）。

**注意**：需检查是否有其他 mart/int 模型 `ref()` 这 3 张表（预期无，因它们是终端消费层）。

**验证**：

```bash
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --select mart_stock_momentum_indicator_daily mart_stock_trend_indicator_daily mart_stock_volume_indicator_daily --project-dir elt --profiles-dir elt
```

### Phase 10: partition_by / unique_key 配置统一（IA10/IE2）

**目标**：table 物化模型补全 partition_by；snapshot 系列不加 partition（数据量小，合理）。

**改动**：

- `int_stock_shares_history.sql`：加 `partition_by='toYear(effective_date)'`
- 评估是否需要 `unique_key`（低优先级，可作为可选增强）

**验证**：

```bash
uv run dbt build --select int_stock_shares_history --project-dir elt --profiles-dir elt
```

### Phase 11: portfolio 系列 YAML 补全列文档（IC1 + B2）

**目标**：4 个 portfolio int wrapper + 2 个 mart rank 表补全缺失列文档（共缺 53 + 13 列）。

**int 层补全**（IC1）：

| 模型 | SQL 输出 | YAML 已文档化 | 需补 |
|---|---|---|---|
| `int_portfolio_closed_trade` | 21 | 4 | 17（含 `security_code`, `realized_pnl`, `total_fee` 等核心字段） |
| `int_portfolio_performance_metric` | 25 | 7 | 18（含全部 12 个绩效指标值字段） |
| `int_portfolio_performance_metric_status` | 8 | 5 | 3（含 `security_code`, `window_key`, `computed_at`） |
| `int_portfolio_trade_metric` | 18 | 3 | 15（含全部交易指标值字段） |

**mart 层补全**（B2）：

| 模型 | 需补 |
|---|---|
| `mart_portfolio_performance_metric_rank` | 10 列 |
| `mart_portfolio_trade_metric_rank` | 6 列 |

**要求**：所有列补 `name`/`data_type`/`description`（中文）；主键和 status 列加 `not_null`。

**验证**：

```bash
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --select int_portfolio_closed_trade int_portfolio_performance_metric int_portfolio_performance_metric_status int_portfolio_trade_metric --project-dir elt --profiles-dir elt
```

### Phase 12: 描述语言统一为中文（ID1/ID2 + C1）

**目标**：所有模型描述和列描述统一为中文。

**改动**：

- int 层：`int_stock_kdj_daily.yml`、`int_stock_ma_daily.yml`、`int_stock_rsi_daily.yml`、`int_stock_price_pattern_daily.yml` 列描述英文 → 中文
- int 层：portfolio 系列 4 个 `.yml` 模型描述和列描述英文 → 中文
- mart 层：`mart_portfolio_performance_metric_rank.yml`、`mart_portfolio_trade_metric_rank.yml`、`mart_stock_momentum_indicator.yml`、`mart_stock_price_pattern_daily.yml`、`mart_stock_trend_indicator.yml`、`mart_stock_volume_indicator.yml` 列描述英文 → 中文

**验证**：

```bash
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt docs generate --project-dir elt --profiles-dir elt
```

### Phase 14: 测试加固（IA12/IE4 + D1/D2）

**目标**：补全 not_null 测试 + 关键度量范围测试。

**改动**：

- portfolio rank 表主键列加 `not_null`（D1）
- RSI 字段加 `accepted_range [0, 100]`（IE4）
- `return_daily`/`pct_change` 加合理区间告警测试（D2）

**验证**：

```bash
uv run dbt build --select mart_portfolio_performance_metric_rank mart_portfolio_trade_metric_rank int_stock_rsi_daily --project-dir elt --profiles-dir elt
```

### Phase 15: KDJ 参数字段暴露一致性收敛（B1）

**目标**：统一 KDJ 参数字段（`kdj_rsv_window`/`kdj_k_smoothing`/`kdj_d_smoothing`）在 mart 层的暴露策略。

**现状**：`mart_stock_momentum_indicator` 暴露了 3 个参数字段，`mart_stock_quotes_daily` 的 KDJ 部分只有值字段无参数字段。

**建议**：采用"mart 只暴露值，参数口径收敛到 int wrapper 文档 + accepted_values 测试"策略 —— 从 `mart_stock_momentum_indicator` 移除 `kdj_rsv_window`/`kdj_k_smoothing`/`kdj_d_smoothing` 3 个参数字段，参数口径由 `int_stock_kdj_daily.yml` 的 `accepted_values` 测试保证。

**改动文件**：

- `mart_stock_momentum_indicator.sql`：删除 SELECT 中的 `kdj_rsv_window`/`kdj_k_smoothing`/`kdj_d_smoothing` 及对应 join alias
- `mart_stock_momentum_indicator.yml`：删除 3 个参数列定义和 `accepted_values` 测试（int 层已有同名测试保证）

**验证**：

```bash
uv run dbt build --select mart_stock_momentum_indicator_daily --project-dir elt --profiles-dir elt
```

## 禁止模式

- 不在 mart 层用 `as old_name` alias 掩盖 int 层重命名
- 不手工编辑 `sources.yml`（contract 生成物）
- 不修改 staging 层透传字段名（IA13 的日期字段纠正除外）
- 不在 int/mart 层重写指标公式（Furnace 边界）
- 不跳过 dbt parse 和 field_glossary 校验

## 验证命令

### 每阶段后运行

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --select <affected_models> --project-dir elt --profiles-dir elt
```

### 全量验证（所有 Phase 完成后）

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
uv run python elt/scripts/validate_staging_readiness.py
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run ruff format scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests

# 文档检查
make docs-check
git diff --check
```

## 完成标准

1. 两份 debt 文档（0002、0003）中所有"待落地"债务项状态更新为"已关闭"
2. `dbt parse` 无错误
3. `validate_field_glossary.py` 无错误
4. 所有受影响模型的 `dbt build` 通过（含 data_tests）
5. int 层字段重命名后 mart 层透传对齐，无残留旧名 alias
6. portfolio 系列 YAML 列文档完整（SQL 输出列 = YAML 文档化列）
7. 所有模型描述和列描述统一为中文
8. `make docs-check` 和 `git diff --check` 通过

## 建议执行顺序

Phase 1 → 2 → 3 → 4 → 5 → 7 → 8 → 6 → 9 → 10 → 11 → 12 → 15 → 14

理由：

- Phase 1-2 是纯 config 改动，最安全，先做
- Phase 3-5 是机械字段重命名，int 先 mart 后
- Phase 7-8 是独立改动（stg 日期纠正 + is_suspend 移除），可并行
- Phase 6 百分比口径 `_pct` 后缀统一（政府债已符合，无需额外改动）
- Phase 9 表名重命名在字段重命名之后，避免中间状态混乱
- Phase 10-12 是配置和文档补全
- Phase 15 在 Phase 9 之后（表名已改），收敛 KDJ 参数字段暴露
- Phase 14 测试加固放最后，避免被前面重命名干扰
