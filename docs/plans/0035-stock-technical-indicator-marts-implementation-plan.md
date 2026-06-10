# Plan 0035: 股票技术指标 marts 实施计划

日期：2026-06-10

状态：Proposed

关联文档：

- `docs/design/README.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md`
- `docs/Q&A/int-layer-indicators-2026-06-10.md`
- `docs/RFC/0016-rust-furnace-compute-engine.md`
- `docs/plans/0034-furnace-macd-technical-indicator-implementation-plan.md`
- `docs/plans/archive/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0029-furnace-moving-average-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0030-furnace-bollinger-bands-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0030-furnace-rsi-technical-indicators-implementation-plan.md`
- `pipeline/elt/models/intermediate/int_stock_ma_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_boll_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_rsi_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_kdj_daily.sql`
- `pipeline/elt/models/marts/mart_stock_quotes_daily.sql`

相关 skills：

- `using-dbt-for-analytics-engineering`：mart 模型 grain、字段契约、YAML tests 和定向 dbt build。
- `running-dbt-commands`：dbt parse/build/test/show 命令格式和选择器。
- `rust-best-practices` / `rust-patterns` / `rust-testing`：仅用于阶段 1 前置计划 `0034` 的验收理解；本计划 mart 阶段不写 Rust。
- `dagster-expert`：仅用于阶段 1 前置计划 `0034` 的 Dagster asset 验收理解；本计划 mart 阶段不改 Dagster。

## 1. 目标

设计并创建三个面向消费层的日频股票技术指标 mart：

| mart | 主题 | 目标指标族 | 粒度 |
|---|---|---|---|
| `mart_stock_trend_indicator` | 趋势指标 | MA、MACD、BOLL | 每证券、交易日一行 |
| `mart_stock_momentum_indicator` | 动量指标 | RSI、KDJ | 每证券、交易日一行 |
| `mart_stock_volume_indicator` | 成交量形指标 | 均量 | 每证券、交易日一行 |

完成后应满足：

1. 三个 mart 均位于 dbt `marts` 层，物化到 `fleur_marts` schema。
2. 三个 mart 都只从 dbt intermediate wrapper 读取指标字段，不在 mart SQL 中重写 MA、MACD、BOLL、RSI、KDJ 或均量公式。
3. mart 主键均为 `(security_code, trade_date)`，与当前日频指标 wrapper 保持一致。
4. 指标字段保留上游口径：MA/BOLL/RSI/KDJ 当前均基于 `close_price_forward_adj`，KDJ 固定 canonical `KDJ(9,3,3)`。
5. MACD 目前没有 calculation/source/intermediate 实现；必须先完成 `docs/plans/0034-furnace-macd-technical-indicator-implementation-plan.md`，再把 MACD 纳入趋势指标 mart。
6. 均量字段归属 `mart_stock_volume_indicator`，不放入 `mart_stock_trend_indicator`。
7. 为三个 mart 建立 SQL、YAML、字段文档、模型设计文档和最小数据测试。

## 2. 非目标

本计划不做以下事情：

1. 不改变已有 MA、BOLL、RSI、KDJ 的公式、参数、复权口径或 Furnace 计算逻辑。
2. 不在 mart 层实现 MACD 公式；MACD 必须先进入 Furnace calculation 层和 dbt intermediate wrapper。
3. 不把三个 mart 设计成长表 `indicator_name/value` 结构；第一版使用宽表，便于下游筛选和 join。
4. 不扩展到 WR、CCI、ATR、OBV、DMA、TRIX 等未列入本计划的指标。
5. 不修改 `mart_stock_quotes_daily` 的既有字段集合；该模型可继续保留 KDJ 字段，但本计划不通过行情宽表承载全部技术指标。
6. 不改变 `fleur_calculation` 表的分区替换协议、Dagster 调度策略或 Rust crate 边界，除非 MACD 上游阶段明确需要新增对应实现。

## 3. 当前事实基线

### 3.1 已有 mart

当前 `pipeline/elt/models/marts/` 只有：

```text
mart_stock_quotes_daily
```

该模型物化为 ClickHouse `MergeTree()` table，按 `toYear(trade_date)` 分区，按 `(security_code, trade_date)` 排序。它以 `int_stock_quotes_daily_unadj` 为主事实表，as-of 补充估值，并左连接 `int_stock_kdj_daily` 暴露 KDJ 字段。

新增三个独立指标 mart 的理由：

1. 技术指标字段会继续增长，继续塞入行情宽表会扩大 `mart_stock_quotes_daily` 的消费面和回归面。
2. 趋势类、动量类和成交量形指标有不同下游使用场景，独立 mart 能让消费方按主题选择字段。
3. 均量字段来自成交量序列，虽然当前由 `int_stock_ma_daily` 暴露，但语义上归属成交量形指标，不应混入价格趋势指标 mart。
4. 三个 mart 与行情宽表 grain 一致，但语义焦点不同；这是新 mart 的合理边界，而不是简单给既有 mart 追加字段。

### 3.2 已有 intermediate 指标

当前可直接复用的 intermediate wrapper：

| 模型 | 上游 calculation 表 | 可用指标 |
|---|---|---|
| `int_stock_ma_daily` | `fleur_calculation.calc_stock_ma_daily` | `price_ma_*`、`price_avg_ma_*`、`price_ema2_10`、`volume_ma_*`；其中 `volume_ma_*` 下游归入成交量形 mart |
| `int_stock_boll_daily` | `fleur_calculation.calc_stock_boll_daily` | `boll_mid_*`、`boll_up_*`、`boll_dn_*` |
| `int_stock_rsi_daily` | `fleur_calculation.calc_stock_rsi_daily` | `rsi_6`、`rsi_12`、`rsi_14`、`rsi_24`、`rsi_25`、`rsi_50` |
| `int_stock_kdj_daily` | `fleur_calculation.calc_stock_kdj_daily` | `rsv_window`、`k_smoothing`、`d_smoothing`、`rsv`、`k_value`、`d_value`、`j_value` |

### 3.3 MACD 缺口

当前仓库未发现 `macd` / `MACD` 相关 calculation 表、Furnace 指标模块或 dbt intermediate model。`docs/Q&A/int-layer-indicators-2026-06-10.md` 也明确记录 MACD 当前没有 int 模型。

因此 `mart_stock_trend_indicator` 不能在第一阶段直接完整上线。需要先按 `docs/plans/0034-furnace-macd-technical-indicator-implementation-plan.md` 新增：

```text
fleur_calculation.calc_stock_macd_daily
fleur_intermediate.int_stock_macd_daily
```

再由 mart 引用：

```text
{{ ref('int_stock_macd_daily') }}
```

## 4. 目标输出草案

### 4.1 `mart_stock_trend_indicator`

粒度：每证券、交易日一行。

建议物化：

```text
materialized='table'
engine='MergeTree()'
order_by='(security_code, trade_date)'
partition_by='toYear(trade_date)'
```

建议字段：

| 字段组 | 字段 |
|---|---|
| 主键 | `security_code`, `trade_date` |
| MA 价格均线 | `price_ma_3`, `price_ma_5`, `price_ma_6`, `price_ma_10`, `price_ma_12`, `price_ma_14`, `price_ma_20`, `price_ma_24`, `price_ma_28`, `price_ma_57`, `price_ma_60`, `price_ma_114`, `price_ma_250` |
| MA 组合和 EMA | `price_avg_ma_3_6_12_24`, `price_avg_ma_14_28_57_114`, `price_ema2_10` |
| BOLL | `boll_mid_10_1p5`, `boll_up_10_1p5`, `boll_dn_10_1p5`, `boll_mid_20_2`, `boll_up_20_2`, `boll_dn_20_2`, `boll_mid_50_2p5`, `boll_up_50_2p5`, `boll_dn_50_2p5` |
| MACD | `macd_dif`, `macd_dea`, `macd_histogram` 或实施 MACD 上游时确定的 canonical 字段名 |

基准左表建议使用 `int_stock_ma_daily`，但只选择价格 MA、MA 组合和 EMA 字段，不选择 `volume_ma_*`。BOLL 和 MACD 通过 `(security_code, trade_date)` 左连接，以避免因某类指标缺口丢失已有 MA 行。

### 4.2 `mart_stock_momentum_indicator`

粒度：每证券、交易日一行。

建议物化与排序同 `mart_stock_trend_indicator`。

建议字段：

| 字段组 | 字段 |
|---|---|
| 主键 | `security_code`, `trade_date` |
| RSI | `rsi_6`, `rsi_12`, `rsi_14`, `rsi_24`, `rsi_25`, `rsi_50` |
| KDJ 参数 | `kdj_rsv_window`, `kdj_k_smoothing`, `kdj_d_smoothing` |
| KDJ 指标 | `kdj_rsv`, `kdj_k_value`, `kdj_d_value`, `kdj_j_value` |

基准左表建议使用 `int_stock_rsi_daily`。KDJ 通过 `(security_code, trade_date)` 左连接，并在 mart 层给 KDJ 字段加 `kdj_` 前缀，避免 `rsv`、`k_value` 等通用字段名在消费侧语义不清。

### 4.3 `mart_stock_volume_indicator`

粒度：每证券、交易日一行。

建议物化与排序同 `mart_stock_trend_indicator`。

建议字段：

| 字段组 | 字段 |
|---|---|
| 主键 | `security_code`, `trade_date` |
| 均量 | `volume_ma_5`, `volume_ma_10`, `volume_ma_20`, `volume_ma_60` |

基准左表建议使用 `int_stock_ma_daily`。第一版只选择均量字段，不引入价格 MA、BOLL、MACD、RSI 或 KDJ。虽然当前均量字段由 MA calculation 表提供，但 mart 语义按成交量形指标归类。

## 5. 实施阶段

### 阶段 0：确认模型契约

1. 确认三个 mart 名称是否按本计划使用：
   - `mart_stock_trend_indicator`
   - `mart_stock_momentum_indicator`
   - `mart_stock_volume_indicator`
2. 确认 `mart_stock_trend_indicator` 是否允许先以 MA+BOLL 上线，MACD 待上游完成后再补齐；推荐不拆分上线，避免趋势 mart 第一版字段契约不完整。
3. 确认沿用 `0034` 的 MACD canonical 口径：
   - 参数：`MACD(12,26,9)`。
   - 字段：`macd_dif`、`macd_dea`、`macd_histogram`。
   - `macd_histogram = DIF - DEA`，不使用 `2 * (DIF - DEA)`。

完成标准：

- 计划中的 mart 名称、字段命名和 MACD 口径已确认。
- 如有名称或口径变更，同步更新本计划再实施。

### 阶段 1：完成 MACD 上游前置计划

前置计划：

```text
docs/plans/0034-furnace-macd-technical-indicator-implementation-plan.md
```

实施范围以 `0034` 为准，包括：

1. 在 Furnace 中新增 MACD 计算能力，公式位于 `furnace-core`。
2. 在 `furnace-io` 中新增 ClickHouse DDL、RowBinary I/O、并行调度和 staging/partition replace 支持。
3. 在 `furnace` CLI 中新增 `macd` 子命令。
4. 在 Dagster 中新增 `fleur_calculation.calc_stock_macd_daily` materialization asset 和必要 metadata。
5. 在 dbt 中新增 `sources_fleur_calculation.yml` source table 定义。
6. 新增 `pipeline/elt/models/intermediate/int_stock_macd_daily.sql` 和 `.yml` thin wrapper。

完成标准：

- MACD 公式没有出现在 dbt SQL、Dagster Python 或 ClickHouse SQL 中。
- `int_stock_macd_daily` 与其他技术指标 wrapper 一样，每证券、交易日一行。
- MACD 字段文档明确输入价格口径、参数、SMA 启动阶段 NULL 语义和 histogram 口径。
- `uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_macd_daily` 已通过；否则不得开始 `mart_stock_trend_indicator` 实施。

### 阶段 2：新增三个 mart 设计文档

新增或更新：

```text
docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator.md
docs/design/dbt_layer/fleur_marts/mart_stock_momentum_indicator.md
docs/design/dbt_layer/fleur_marts/mart_stock_volume_indicator.md
```

每个设计文档至少包含：

1. 模型目标和非目标。
2. grain 和主键。
3. 上游模型与 join 方式。
4. 字段分组和字段来源。
5. NULL 语义和指标可用性说明。
6. 与 0034 的依赖关系：`mart_stock_trend_indicator` 必须等待 `int_stock_macd_daily` 可用。
7. 验证命令。

完成标准：

- 设计文档链接到对应 SQL/YAML 路径。
- 设计文档不重写已在 upstream wrapper 文档中定义的公式，只引用上游口径。

### 阶段 3：新增 `mart_stock_trend_indicator`

新增：

```text
pipeline/elt/models/marts/mart_stock_trend_indicator.sql
pipeline/elt/models/marts/mart_stock_trend_indicator.yml
```

SQL 设计：

1. `ma` CTE 从 `{{ ref('int_stock_ma_daily') }}` 选择价格 MA、MA 组合和 EMA 字段。
2. `boll` CTE 从 `{{ ref('int_stock_boll_daily') }}` 选择 BOLL 字段。
3. `macd` CTE 从 `{{ ref('int_stock_macd_daily') }}` 选择 `macd_dif`、`macd_dea`、`macd_histogram`。
4. 以 `ma` 为左表，按 `(security_code, trade_date)` 左连接 BOLL 和 MACD。
5. 不选择 `volume_ma_*` 字段；均量归入 `mart_stock_volume_indicator`。
6. 不引入行情、估值、财务或基础信息字段，保持趋势指标 mart 边界清晰。

YAML 设计：

1. 模型级描述说明趋势指标 mart、grain 和上游 wrapper。
2. 添加 `(security_code, trade_date)` 唯一组合测试。
3. `security_code` 添加 `not_null` 和 `cn_security_code_format`。
4. `trade_date` 添加 `not_null`。
5. BOLL 可添加关系型数据测试或 singular test：非空时 `boll_up >= boll_mid >= boll_dn`。
6. MACD 字段说明参数、输入和 histogram 口径。
7. 明确不暴露 `ema_fast_state_12`、`ema_slow_state_26`、`macd_dea_state` 等 MACD 内部状态列。

完成标准：

- `dbt parse` 通过。
- `dbt build --select mart_stock_trend_indicator` 通过。
- `dbt show` 抽样结果没有 duplicate grain 或 join 放大。
- 抽样结果中 MACD warm-up 期允许为 NULL，但不得出现 mart 层填 0 或重算公式。

### 阶段 4：新增 `mart_stock_momentum_indicator`

新增：

```text
pipeline/elt/models/marts/mart_stock_momentum_indicator.sql
pipeline/elt/models/marts/mart_stock_momentum_indicator.yml
```

SQL 设计：

1. `rsi` CTE 从 `{{ ref('int_stock_rsi_daily') }}` 选择 RSI 字段。
2. `kdj` CTE 从 `{{ ref('int_stock_kdj_daily') }}` 选择 KDJ 字段，并在 select 中重命名为 `kdj_*`。
3. 以 `rsi` 为左表，按 `(security_code, trade_date)` 左连接 KDJ。
4. 不从 `mart_stock_quotes_daily` 读取 KDJ，避免 mart 依赖 mart。

YAML 设计：

1. 模型级描述说明动量指标 mart、grain 和上游 wrapper。
2. 添加 `(security_code, trade_date)` 唯一组合测试。
3. `security_code` 添加 `not_null` 和 `cn_security_code_format`。
4. `trade_date` 添加 `not_null`。
5. RSI 字段添加范围测试或 singular test：非空值必须在 `[0, 100]`。
6. KDJ 参数字段添加 accepted values：`kdj_rsv_window = 9`、`kdj_k_smoothing = 3`、`kdj_d_smoothing = 3`。

完成标准：

- `dbt parse` 通过。
- `dbt build --select mart_stock_momentum_indicator` 通过。
- `dbt show` 抽样结果没有 duplicate grain 或 join 放大。

### 阶段 5：新增 `mart_stock_volume_indicator`

新增：

```text
pipeline/elt/models/marts/mart_stock_volume_indicator.sql
pipeline/elt/models/marts/mart_stock_volume_indicator.yml
```

SQL 设计：

1. `volume_ma` CTE 从 `{{ ref('int_stock_ma_daily') }}` 选择 `security_code`、`trade_date` 和均量字段。
2. 输出字段固定为 `volume_ma_5`、`volume_ma_10`、`volume_ma_20`、`volume_ma_60`。
3. 不引入价格 MA、BOLL、MACD、RSI、KDJ、行情、估值、财务或基础信息字段。

YAML 设计：

1. 模型级描述说明成交量形指标 mart、grain 和上游 wrapper。
2. 添加 `(security_code, trade_date)` 唯一组合测试。
3. `security_code` 添加 `not_null` 和 `cn_security_code_format`。
4. `trade_date` 添加 `not_null`。
5. 均量字段说明基于未复权日行情 `volume`，0 成交量是有效输入，窗口不足或 source 缺口时允许为 NULL。

完成标准：

- `dbt parse` 通过。
- `dbt build --select mart_stock_volume_indicator` 通过。
- `dbt show` 抽样结果没有 duplicate grain 或 join 放大。

### 阶段 6：验证和运行报告

定向验证命令：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator mart_stock_momentum_indicator mart_stock_volume_indicator
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator --limit 20
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_momentum_indicator --limit 20
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_volume_indicator --limit 20
```

阶段 1 必须额外运行 `0034` 中列出的 Rust、Dagster 和定向 dbt source/wrapper 验证。

完成后新增运行报告：

```text
docs/jobs/reports/YYYY-MM-DD-stock-technical-indicator-marts-build.md
```

报告至少记录：

1. 执行日期和环境。
2. dbt 命令、选择器和结果。
3. 三个 mart 的行数、证券数、交易日范围。
4. duplicate grain 检查结果。
5. 关键指标 NULL 分布或 warm-up 缺口说明。
6. `mart_stock_trend_indicator` 中 MACD 字段来自 `int_stock_macd_daily` 的证据，例如 compiled SQL 或 lineage 片段。

## 6. 禁止模式

1. 禁止在 mart SQL 中用 window function 重算 MA、BOLL、RSI、KDJ、MACD 或均量。
2. 禁止让 `mart_stock_momentum_indicator` 从 `mart_stock_quotes_daily` 读取 KDJ。
3. 禁止把 `volume_ma_*` 放入 `mart_stock_trend_indicator`；均量归属 `mart_stock_volume_indicator`。
4. 禁止以 inner join 合并各指标 wrapper，除非已证明所有上游指标表的 grain 和 coverage 完全一致。
5. 禁止为了解决字段缺失在 mart 层 hardcode 默认值；指标不可用应保留 NULL。
6. 禁止把 MACD 内部状态列暴露到 mart，除非先在 intermediate wrapper 中明确成为业务字段。
7. 禁止在没有完成 MACD 上游的情况下伪造 MACD 字段或使用临时 SQL 公式。
8. 禁止在 `mart_stock_trend_indicator` 中选择或透传 MACD 内部状态列。

## 7. 最小验证命令

文档-only 阶段至少运行：

```bash
make docs-check
git diff --check
```

实施 dbt mart 后运行：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator mart_stock_momentum_indicator mart_stock_volume_indicator
```

阶段 1 按 `0034` 额外运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

阶段 1 还必须运行：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_macd_daily
```

## 8. 完成标准

本计划完成时应满足：

1. `mart_stock_trend_indicator`、`mart_stock_momentum_indicator` 和 `mart_stock_volume_indicator` 已在 dbt marts 层实现并文档化。
2. 三个 mart 均通过主键唯一性、主键非空和证券代码格式测试。
3. `mart_stock_trend_indicator` 包含 MA、BOLL、MACD 三类趋势指标；MACD 字段来自 `int_stock_macd_daily`，且不包含 `volume_ma_*`。
4. `mart_stock_momentum_indicator` 包含 RSI 和 KDJ 两类动量指标；KDJ 字段在 mart 层使用 `kdj_` 前缀。
5. `mart_stock_volume_indicator` 包含 `volume_ma_5`、`volume_ma_10`、`volume_ma_20`、`volume_ma_60` 四个均量字段。
6. 三个 mart 均通过定向 `dbt build`。
7. 运行报告记录行数、证券数、交易日范围、关键 NULL 分布和验证命令结果。
8. 完成后将本计划移入 `docs/plans/archive/`，状态改为 `Archived`，并更新 `docs/plans/README.md`。
9. `docs/plans/0034-furnace-macd-technical-indicator-implementation-plan.md` 已完成或归档；若未完成，本计划状态不得进入 `Completed`。
