# RFC 0018: Rust Rearview 规则选股服务与 mart 指标库

状态：Archived（2026-06-25；归档前状态：草案 / 构想（2026-06-12））

## 摘要

本文档记录 mono-fleur 规则选股器的初始 RFC 构想。服务名定为 `rearview`。目标是在现有 mart 层日频行情和技术指标已经成型的基础上，新增一个 Rust HTTP 选股服务：

```text
PostgreSQL instance
  database: rearview
    规则、版本、运行、结果摘要和业务审计
      ↓
Rust rearview service
  HTTP API、规则校验、规则编译、ClickHouse 查询执行、运行状态管理
      ↓
ClickHouse fleur_marts
  mart_stock_quotes_daily
  mart_stock_trend_indicator
  mart_stock_momentum_indicator
  mart_stock_volume_indicator
  mart_stock_price_pattern_daily
```

第一版定位是“规则驱动的区间选股和买入信号生成服务”，不是新的指标计算引擎。指标仍由 dbt/Furnace 维护并物化到 ClickHouse mart 层；选股器只消费稳定 mart 接口。

第一版业务分为两段：

1. **Part 1：日股票池生成。** 用户给定交易日期区间，例如 `2021-01-01` 到 `2025-12-31`，服务对区间内每个交易日使用同一规则版本进行过滤，产出每日股票池。
2. **Part 2：日内评分与 TopN 买入信号。** 服务对每日股票池内个股使用当日指标打分，再按用户给定 `top_n` 选择每日买入信号个股。

## 背景

当前 mono-fleur 已有一批可支持选股的 ClickHouse mart 表：

| Mart | 当前职责 | 主要字段组 | 当前物化特征 |
|---|---|---|---|
| `fleur_marts.mart_stock_quotes_daily` | A 股日频行情、交易指标、市值、估值、KDJ 消费字段 | OHLC、成交量、换手率、涨跌幅、市值、PE/PB、股息率、ST/停牌、KDJ | `MergeTree`，`PARTITION BY toYear(trade_date)`，`ORDER BY (security_code, trade_date)` |
| `fleur_marts.mart_stock_trend_indicator` | 趋势类技术指标 | 价格 MA、组合 MA、双重 EMA、BOLL、MACD | `MergeTree`，`PARTITION BY toYear(trade_date)`，`ORDER BY (trade_date, security_code)` |
| `fleur_marts.mart_stock_momentum_indicator` | 动量类技术指标 | RSI、KDJ | `MergeTree`，`PARTITION BY toYear(trade_date)`，`ORDER BY (trade_date, security_code)` |
| `fleur_marts.mart_stock_volume_indicator` | 成交量形指标 | `volume_ma_5`、`volume_ma_10`、`volume_ma_20`、`volume_ma_60` | `MergeTree`，`PARTITION BY toYear(trade_date)`，`ORDER BY (trade_date, security_code)` |
| `fleur_marts.mart_stock_price_pattern_daily` | 价格行为与前低-次低结构 | `close_direction`、`close_up_streak_days`、`close_down_streak_days`、`n_structure_20_*` | `MergeTree`，`PARTITION BY toYear(trade_date)`，`ORDER BY (trade_date, security_code)` |

这些 mart 表已遵守当前分层边界：

- Furnace 负责计算 MA、RSI、BOLL、MACD、KDJ 等技术指标。
- `fleur_calculation` 是外部计算产物层。
- dbt intermediate thin wrapper 暴露 `int_*` 稳定语义。
- `fleur_marts` 提供面向应用、BI 和下游服务的稳定消费接口。

规则选股器应优先消费 `fleur_marts`，避免直接读取 `fleur_calculation`、`fleur_intermediate`、`fleur_staging` 或 `fleur_raw`。

## 工作负载画像

| 维度 | 判断 |
|---|---|
| workload | market data / financial services，混合 OLAP 与小规模点查 |
| 典型数据粒度 | 每证券、交易日一行 |
| 主要查询模式 | 给定日期区间，对每个交易日在证券 universe 内按指标谓词过滤，随后按当日指标评分并取 TopN |
| 延迟目标 | 短区间交互式运行应在秒级内返回；多年区间批量运行可以异步执行 |
| 业务状态 | 规则定义、版本、运行状态、审计和结果摘要放 PostgreSQL `rearview` 数据库 |
| 指标状态 | 日频指标事实放 ClickHouse mart 层，只读消费 |

## 目标

1. 支持版本化规则集，保证同一次选股运行可复现到具体规则版本。
2. 支持基于 mart 指标的布尔筛选，例如区间、比较、字段间比较、空值策略、`AND` / `OR` / `NOT` 组合。
3. 支持基于 mart 指标的当日评分、排名和每日 TopN 买入信号生成。
4. 支持按 `start_date`、`end_date`、证券 universe、规则版本和 `top_n` 发起区间选股运行。
5. 使用 PostgreSQL `rearview` 数据库记录规则、规则版本、区间运行状态、每日股票池、每日买入信号、编译 SQL hash、ClickHouse query id、结果摘要和错误信息。
6. 使用 Rust 服务负责规则校验、查询规划、SQL 编译、ClickHouse 查询执行和 PostgreSQL 状态更新。
7. 让 ClickHouse 查询只读取必要 mart 表、必要列和请求日期区间。
8. 第一版提供 HTTP API，后续为 UI、定时运行和策略研究留出边界。

## 非目标

1. 第一版不实现交易、下单、风控或组合调仓。
2. 第一版不实现完整回测引擎；区间逐日股票池和买入信号只是信号生成，不模拟持仓、资金、成交、滑点、调仓或收益。
3. 第一版不在 Rust 选股服务中重算任何技术指标。
4. 第一版不允许用户提交任意 SQL。
5. 第一版不直接读取 raw、staging、intermediate 或 `fleur_calculation` 表。
6. 第一版不要求建设 Web UI；服务入口固定为 HTTP API，CLI 只作为可选运维工具。
7. 第一版不把 ClickHouse 作为规则业务数据库；业务状态以 PostgreSQL `rearview` 数据库为准。

## 目标架构

```text
Operator / API / Schedule
    |
    v
Rust rearview service
    |
    | read/write business state
    v
PostgreSQL instance
  database: pipeline
    existing OCR / pipeline business state
  database: rearview
    rule_set
    rule_version
    run
    run_day
    pool_member
    buy_signal
    metric_catalog
    ^
    |
    | read-only analytical queries
    |
ClickHouse fleur_marts
  mart_stock_quotes_daily
  mart_stock_trend_indicator
  mart_stock_momentum_indicator
  mart_stock_volume_indicator
  mart_stock_price_pattern_daily
```

### PostgreSQL 职责

PostgreSQL 是选股器业务数据库，保存需要事务、版本、审计和应用查询的状态。第一版采用“同一个 PostgreSQL 实例、不同 database”的隔离方式：

| Database | 职责 | 运行时使用方 |
|---|---|---|
| `pipeline` | 现有 pipeline 业务状态，例如 OCR 相关表 | Dagster / pipeline 组件 |
| `rearview` | Rearview 规则、版本、运行、结果、指标目录和审计 | Rust `rearview` 服务 |

拆分 database 后，`rearview` 库内表名不再附带服务名前缀或 `screening_` 前缀。文档中写到 `rule_set`、`run`、`buy_signal` 等表名时，均指 PostgreSQL `rearview` database 内的表。跨库引用时使用“`rearview` database 的 `rule_set` 表”这种描述，避免把 database 名误写成 PostgreSQL schema 名。

建议第一版业务表：

| 表 | 职责 |
|---|---|
| `rule_set` | 规则集业务实体，保存名称、描述、状态、owner、tags 和 `current_version_id` |
| `rule_version` | 不可变规则版本，保存规则 AST、universe snapshot、输出字段、打分配置、metric dependency snapshot 和 `rule_hash` |
| `metric_catalog` | 可被规则引用的指标目录，映射 logical metric、mart table、column、类型、允许操作符、默认空值策略 |
| `run` | 一次区间选股运行，保存 rule version、`start_date`、`end_date`、`top_n`、universe snapshot、状态、compiled SQL hash、ClickHouse query id、耗时、结果摘要 |
| `run_day` | 区间运行内的日粒度状态，保存 `trade_date`、universe 数量、股票池数量、信号数量、耗时和错误摘要 |
| `pool_member` | Part 1 结果，保存每个交易日通过过滤的证券、filter snapshot 和轻量 `selected_metrics` |
| `buy_signal` | Part 2 结果，保存每个交易日 TopN 买入信号证券、rank、score、`score_breakdown` 和 `selected_metrics` |

第一版默认把股票池和买入信号写回 PostgreSQL。A 股单日 universe 规模有限，即使 5 年区间逐日保存也仍适合先满足业务读取、审计和 API 查询。如果后续需要保存大量规则、多年重跑或跨规则 OLAP，再单独评估 ClickHouse 结果事实表。

### PostgreSQL 迁移边界

PostgreSQL DDL 仍由现有 `pipeline/migrate` Alembic 统一管理，不为 Rust 服务建立独立迁移机制。但 `pipeline/migrate` 需要从当前单 database 迁移入口改造成多 database target：

| Target | Database | URL 环境变量 | 迁移内容 |
|---|---|---|---|
| `pipeline` | `pipeline` | `PIPELINE_DATABASE_URL` | 现有 OCR / pipeline 业务表 |
| `rearview` | `rearview` | `REARVIEW_DATABASE_URL` | `rule_set`、`rule_version`、`run`、`run_day`、`pool_member`、`buy_signal`、`metric_catalog` 等 Rearview 表 |

约束：

1. `PIPELINE_DATABASE_URL` 和 `REARVIEW_DATABASE_URL` 指向同一个 PostgreSQL 实例的不同 database。
2. `pipeline/migrate` 是 PostgreSQL schema 变更的唯一权威入口；Rust `rearview` 服务不自动建表、不自动执行 DDL migration。
3. 迁移账号可以拥有创建 database 和 DDL 权限；运行时账号按 database 分离授权，pipeline 组件不写 `rearview` 库，`rearview` 服务不写 `pipeline` 库。
4. `pipeline/migrate` 可以通过 Alembic branch label、version locations 或等价机制区分 `pipeline` 与 `rearview` target；每个 database 维护自己的 Alembic version 状态。
5. 部署顺序固定为先运行 PostgreSQL 迁移，再启动或重启 Rust `rearview` 服务。
6. Rust `rearview` 服务启动时可以做只读 schema readiness check；若 `rearview` database 缺表或版本不兼容，应 fail fast，而不是尝试自修复 DDL。

### 规则集、版本和 hash

规则集分为两层：

| 层 | 角色 | 是否影响结果 |
|---|---|---|
| `rule_set` | 策略容器，负责名称、描述、owner、状态、分类和当前默认版本 | 不直接影响结果 |
| `rule_version` | 可执行规则快照，负责 universe、过滤、评分、TopN 默认值、输出指标和 metric dependency | 直接影响结果 |

`rule_set` 建议字段：

| 字段 | 作用 |
|---|---|
| `rule_set_id` | 规则集唯一标识 |
| `name` | 策略名称，例如“趋势动量选股” |
| `description` | 策略说明 |
| `owner` / `created_by` | 归属人和创建人 |
| `status` | `draft`、`active`、`archived` 等生命周期 |
| `tags` / `category` | 策略分类和检索 |
| `current_version_id` | 当前默认运行的规则版本指针 |
| `created_at` / `updated_at` | 审计时间 |

`current_version_id` 只表示“现在默认运行哪个版本”。如果 API 只传 `rule_set_id` 而不传 `rule_version_id`，服务可以使用 `current_version_id`；但历史运行必须保存实际使用的 `rule_version_id` 和 `rule_hash`，不能只保存会变化的 `current_version_id`。切换、灰度和回滚规则时，只更新 `current_version_id`，不修改历史版本。

`rule_hash` 是规则版本内容的稳定指纹。它用于证明某次运行使用的规则内容是否完全一致。第一版建议对会影响结果的字段做 canonical JSON，再计算 SHA-256：

```text
rule_hash = sha256(canonical_json({
  universe_snapshot,
  pool_filters,
  scoring,
  top_n_default,
  output_metrics,
  metric_dependency_snapshot
}))
```

不影响结果的展示字段不进入 `rule_hash`，例如 `name`、`description`、tags 和 UI 排版字段。规则内容任何变化都应生成新的 `rule_version` 和新的 `rule_hash`。

### Universe 存储

第一版 `universe` 使用结构化 JSONB 存储，不保存 SQL。

示例：

```json
{
  "base": "all_a_shares",
  "filters": {
    "exclude_st": true,
    "exclude_suspend": true,
    "min_listing_days": 120,
    "boards": ["main", "chi_next"],
    "market_cap": {
      "min": 5000000000,
      "max": null
    }
  },
  "include_security_codes": [],
  "exclude_security_codes": ["000001.SZ"]
}
```

建议字段：

| 存储位置 | 内容 | 作用 |
|---|---|---|
| `rule_version.universe_snapshot` | 规则版本默认 universe JSONB | 规则版本可复现 |
| `run.universe_snapshot` | 运行时实际使用的 universe JSONB | 保存运行时覆盖后的事实 |
| `run.resolved_universe_hash` | 解析后股票范围 hash | 审计和排查 |
| `run_day.universe_count` | 每日实际 universe 数量 | 观察每日参与筛选的基数 |

第一版不需要把 universe 单独建成复杂表。若后续需要用户维护可复用股票池，再新增：

| 表 | 作用 |
|---|---|
| `universe_set` | 股票池容器，例如“沪深300候选池” |
| `universe_version` | 不可变股票池版本 |
| `universe_member` | 每个版本包含的 `security_code` |

此时规则版本里的 universe 可以引用 `universe_version_id`，但 `run` 仍必须保存运行时 universe snapshot，避免历史运行随当前股票池变化。

### ClickHouse 职责

ClickHouse 是选股器的指标库，只提供已物化的 mart 指标事实。

第一版读取范围：

| 允许读取 | 用途 |
|---|---|
| `fleur_marts.mart_stock_quotes_daily` | universe 基础过滤、行情、估值、市值、ST/停牌 |
| `fleur_marts.mart_stock_trend_indicator` | 趋势类技术指标筛选 |
| `fleur_marts.mart_stock_momentum_indicator` | 动量类技术指标筛选 |
| `fleur_marts.mart_stock_volume_indicator` | 成交量形指标筛选 |
| `fleur_marts.mart_stock_price_pattern_daily` | 价格方向、连涨连跌和 20 根窗口结构筛选 |

默认禁止：

- 选股服务直接读取 `fleur_calculation.*`。
- 选股服务直接读取 `fleur_intermediate.*`、`fleur_staging.*` 或 `fleur_raw.*`。
- 选股服务向 mart 层写入数据。

## 规则表达

规则应以结构化 AST 存储，而不是保存任意 SQL 字符串。服务将 AST 编译为受控 ClickHouse SQL。

示例草案：

```yaml
name: trend_and_momentum_screen
date_range:
  start_date: 2021-01-01
  end_date: 2025-12-31
universe:
  base: all_a_shares
  filters:
    exclude_suspend: true
    exclude_st: true
    min_listing_days: 120
  include_security_codes: []
  exclude_security_codes: []
pool_filters:
  all:
    - metric: close_price
      op: gt_field
      rhs_metric: price_ma_20
    - metric: price_ma_5
      op: gt_field
      rhs_metric: price_ma_20
    - metric: rsi_6
      op: between
      value: [20, 65]
    - metric: macd_histogram
      op: gt
      value: 0
scoring:
  method: weighted_sum
  factors:
    - metric: amount
      transform: rank_pct_by_day
      direction: desc
      weight: 0.3
    - metric: rsi_6
      transform: rank_pct_by_day
      direction: asc
      weight: 0.2
    - metric: macd_histogram
      transform: rank_pct_by_day
      direction: desc
      weight: 0.5
top_n: 20
output_metrics:
  - close_price
  - pct_change
  - price_ma_5
  - price_ma_20
  - rsi_6
  - macd_histogram
```

第一版操作符建议：

| 类型 | 操作符 |
|---|---|
| 数值比较 | `gt`、`gte`、`lt`、`lte`、`eq`、`neq`、`between` |
| 字段间比较 | `gt_field`、`gte_field`、`lt_field`、`lte_field` |
| 表达式比较 | `gt_expr`、`gte_expr`、`lt_expr`、`lte_expr`、`eq_expr`、`neq_expr` |
| 空值判断 | `is_null`、`is_not_null` |
| 集合判断 | `in`、`not_in` |
| 逻辑组合 | `all`、`any`、`not` |

条件表达式必须支持字段、字面量和受控算术表达式，不能把任意 SQL 片段塞进规则。第一版为了覆盖选股常见口径，至少支持：

| 表达式 | 示例 | 编译含义 |
|---|---|---|
| 字面量 | `-10`、`true` | 参数绑定值 |
| 指标字段 | `{metric: prev_volume}` | catalog allowlist 中的字段 |
| 字段乘常数 | `{metric: prev_volume, mul: 0.8}` | `prev_volume * 0.8` |
| 字段加常数 | `{metric: kdj_j_value, add: 5}` | `kdj_j_value + 5` |
| 字段间比较 | `price_ema2_10 > price_avg_ma_14_28_57_114` | 两个 allowlist 字段比较 |
| 区间比较 | `-15 <= kdj_j_value < -10` | lower inclusive、upper exclusive 的复合条件 |

这些表达式只允许由 metric catalog 中的字段、数字/布尔字面量和白名单算子组成。若后续要支持更复杂公式，应扩展 AST，而不是开放 SQL。

第一版评分建议：

| 配置 | 语义 |
|---|---|
| `method: weighted_sum` | 对每个交易日股票池内个股按多个因子加权求和 |
| `method: conditional_points` | 对每个交易日股票池内个股按条件命中累加或扣减固定分 |
| `transform: raw` | 直接使用原始指标值，只适合同量纲或明确可比指标 |
| `transform: rank_pct_by_day` | 在同一交易日股票池内按指标排序后转成百分位，便于不同量纲指标相加 |
| `direction: desc` | 指标越大得分越高 |
| `direction: asc` | 指标越小得分越高 |
| `weight` | 因子权重；第一版要求权重和为 1 或由服务归一化，并记录归一化结果 |
| `points` | 条件命中时加减的固定分值，例如 `+25` 或 `-15` |
| `clamp` | 对最终分数做上下限裁剪，例如 `{min: 0, max: 99}` |

`conditional_points` 的规则：

1. 每条评分规则包含 `id`、`when` 和 `points`。
2. `when` 使用与 `pool_filters` 相同的条件 AST。
3. 同一股票同一交易日可以命中多条评分规则，分值累加。
4. 分值可以为负数。
5. 最终分值必须按 `clamp.min` / `clamp.max` 裁剪。业务上写作 `(0,99)` 时，第一版按闭区间 `[0,99]` 处理；如果后续要求严格开区间，再引入 epsilon 规则。
6. `score_breakdown` 必须记录每条评分规则是否命中、贡献分和参与比较的原始指标值。

### 代表性用例：N 结构低位企稳信号

该用例用于验证第一版规则语言必须能同时支持：

- 多 mart 字段联合过滤。
- 数值比较、布尔比较、字段间比较和字段乘常数。
- 条件命中加分、扣分、分段加分。
- 最终分数 clamp 到 `[0,99]`。

规则草案：

```yaml
name: n_structure_low_reversal_screen
date_range:
  start_date: 2021-01-01
  end_date: 2025-12-31
universe:
  base: all_a_shares
  filters:
    exclude_suspend: true
    exclude_st: true
pool_filters:
  all:
    - id: kdj_j_oversold
      metric: kdj_j_value
      op: lt
      value: -10
    - id: down_streak_not_too_long
      metric: close_down_streak_days
      op: lt
      value: 4
    - id: ema_above_long_avg
      metric: price_ema2_10
      op: gt_field
      rhs_metric: price_avg_ma_14_28_57_114
    - id: volume_above_prev_80pct
      metric: volume
      op: gt_expr
      rhs:
        metric: prev_volume
        mul: 0.8
    - id: n_structure_valid
      metric: n_structure_20_is_valid
      op: eq
      value: true
scoring:
  method: conditional_points
  base_score: 0
  clamp:
    min: 0
    max: 99
  rules:
    - id: near_boll_dn_20_2
      when:
        metric: close_price
        op: lte_expr
        rhs:
          metric: boll_dn_20_2
          mul: 1.02
      points: 0.25
    - id: rsi_6_deep_oversold
      when:
        metric: rsi_6
        op: lt
        value: 25
      points: 10
    - id: kdj_j_deep_oversold
      when:
        metric: kdj_j_value
        op: lt
        value: -15
      points: 35
    - id: kdj_j_mild_oversold
      when:
        all:
          - metric: kdj_j_value
            op: gte
            value: -15
          - metric: kdj_j_value
            op: lt
            value: -10
      points: 25
    - id: volume_below_ma5_half
      when:
        metric: volume
        op: lt_expr
        rhs:
          metric: volume_ma_5
          mul: 0.5
      points: 20
    - id: close_below_short_avg
      when:
        metric: close_price
        op: lt_field
        rhs_metric: price_avg_ma_3_6_12_24
      points: 15
    - id: close_between_ma20_ma60
      when:
        all:
          - metric: close_price
            op: gt_field
            rhs_metric: price_ma_20
          - metric: close_price
            op: lt_field
            rhs_metric: price_ma_60
      points: 15
    - id: close_too_far_above_short_avg
      when:
        metric: close_price
        op: gt_expr
        rhs:
          metric: price_avg_ma_3_6_12_24
          mul: 1.05
      points: -15
top_n: 20
output_metrics:
  - close_price
  - prev_volume
  - volume
  - volume_ma_5
  - kdj_j_value
  - rsi_6
  - boll_dn_20_2
  - price_ema2_10
  - price_avg_ma_14_28_57_114
  - price_avg_ma_3_6_12_24
  - price_ma_20
  - price_ma_60
  - close_down_streak_days
  - n_structure_20_is_valid
```

该用例依赖的 mart 输入：

| 字段 | 来源 mart |
|---|---|
| `close_price`, `prev_volume`, `volume` | `mart_stock_quotes_daily` |
| `kdj_j_value`, `rsi_6` | `mart_stock_momentum_indicator` |
| `price_ema2_10`, `price_avg_ma_14_28_57_114`, `price_avg_ma_3_6_12_24`, `price_ma_20`, `price_ma_60`, `boll_dn_20_2` | `mart_stock_trend_indicator` |
| `volume_ma_5` | `mart_stock_volume_indicator` |
| `close_down_streak_days`, `n_structure_20_is_valid` | `mart_stock_price_pattern_daily` |

实施时，`metric_catalog` 必须登记上表所有字段及类型、来源 mart、允许操作符和默认 NULL 语义。第一版基础字段事实应从 dbt mart YAML 读取或校验，`mart_stock_price_pattern_daily` 的 YAML 必须包含这些字段；Rearview 只在 policy overlay 中维护可过滤、可评分、允许操作符和 NULL 策略等规则引擎策略。

该用例的 `score_breakdown` 至少应记录每个 `scoring.rules[].id` 的：

| 字段 | 说明 |
|---|---|
| `rule_id` | 评分规则 ID |
| `matched` | 是否命中 |
| `points` | 命中时贡献分；未命中为 0 |
| `raw_values` | 条件中使用的原始指标值 |
| `condition` | 规则版本中的条件快照或条件 hash |

最终 `score = clamp(sum(points), 0, 99)`。在该用例当前互斥分段下，理论最大正向分为 `95.25`；clamp 仍作为统一评分协议保留，确保后续规则组合或权重调整后分数不会越界。

### 用例支持性审查

原草案不能完整支持该用例，缺口如下。本 RFC 已按下表补齐第一版必须支持的能力：

| 缺口 | 影响 | 补充设计 |
|---|---|---|
| 未把 `mart_stock_price_pattern_daily` 纳入允许读取范围 | 无法使用 `close_down_streak_days` 和 `n_structure_20_is_valid` | 已加入目标架构、mart 背景表和 ClickHouse 读取范围 |
| 仅有简单字段比较 | 无法表达 `volume > prev_volume * 0.8`、`close_price <= boll_dn_20_2 * 1.02` | 条件 AST 增加受控 RHS 表达式，支持字段乘常数 |
| 没有布尔字段过滤说明 | 无法表达 `n_structure_20_is_valid = true` | 操作符保留 `eq`，metric catalog 需标记 Boolean 字段可用 `eq` |
| 评分只描述 `weighted_sum` | 无法表达固定加分、扣分和分段加分 | 新增 `conditional_points` 评分方法 |
| 没有分数裁剪 | 无法保证分数控制在 `(0,99)` | 新增 `clamp`，第一版按 `[0,99]` 实现 |
| `score_breakdown` 结构不够具体 | 无法解释每个加分项来源 | 明确每条 scoring rule 记录 matched、points、raw_values 和 condition |

补齐以上能力后，该用例可以由第一版 rearview 编译为受控 ClickHouse 查询，并把完整股票池与 TopN 买入信号写回 PostgreSQL。

规则编译约束：

1. 规则只能引用 `metric_catalog` 中登记的 metric。
2. 每个 metric 必须有明确来源 mart、列名、类型、允许操作符和空值策略。
3. 字面量通过参数绑定或等价安全机制传入，不拼接用户输入。
4. 字段名、表名和排序方向只能来自 allowlist。
5. 编译 SQL 必须记录 hash，用于运行复现和审计。
6. 除 `is_null` 外，任一比较条件遇到 NULL 默认为不命中；如需不同 NULL 语义，必须在规则 AST 中显式配置。
7. `conditional_points` 中多条规则默认全部独立求值并累加；如果要表达分段互斥，需要像 `kdj_j_mild_oversold` 一样写成互斥条件。

## HTTP API 草案

第一版服务入口固定为 HTTP API。API 只接收结构化规则、运行参数和查询参数，不接收任意 SQL。

候选接口：

| Method | Path | 用途 |
|---|---|---|
| `POST` | `/rearview/rule-sets` | 创建规则集 |
| `POST` | `/rearview/rule-sets/{rule_set_id}/versions` | 创建不可变规则版本 |
| `POST` | `/rearview/runs` | 发起区间选股运行 |
| `GET` | `/rearview/runs/{run_id}` | 查询运行状态和汇总 |
| `GET` | `/rearview/runs/{run_id}/chunks` | 查询 chunk 日期范围、状态、ClickHouse query id 和耗时 |
| `GET` | `/rearview/runs/{run_id}/days` | 查询每日股票池数量、信号数量和日状态 |
| `GET` | `/rearview/runs/{run_id}/pool?trade_date=...` | 查询某日股票池 |
| `GET` | `/rearview/runs/{run_id}/signals?trade_date=...` | 查询某日 TopN 买入信号 |
| `POST` | `/rearview/explain` | 只编译和解释规则，不写结果；请求带日期区间时返回 chunk plan |

`POST /rearview/runs` 请求草案：

```json
{
  "rule_version_id": "rv_001",
  "start_date": "2021-01-01",
  "end_date": "2025-12-31",
  "top_n": 20,
  "mode": "async"
}
```

请求可以直接传 `rule_version_id`，也可以传 `rule_set_id`。当只传 `rule_set_id` 时，服务使用 `rule_set.current_version_id` 解析出实际运行版本，并把实际 `rule_version_id` 和 `rule_hash` 写入 `run`。

多年区间默认异步执行，HTTP 立即返回 `run_id`。短区间可以后续支持同步模式，但同步模式也必须落 PostgreSQL run 记录。

## 业务流程

### Part 1：日股票池生成

输入：

- `rule_version_id`
- `start_date`
- `end_date`
- universe 配置
- `pool_filters`

处理：

1. 服务校验规则版本和 metric catalog。
2. 服务将日期区间解析为运行范围。第一版可从 anchor mart 的 `trade_date` 推导实际交易日集合；如果后续需要完整交易所日历，应新增 mart 层交易日历消费接口。
3. 服务为 anchor mart 中覆盖到的每个实际交易日创建 `run_day` 占位；即使某日没有任何证券入池，也必须保留 `run_day` 并把 `pool_count` / `signal_count` 记为 0。
4. 服务编译 ClickHouse 查询，对区间内每个交易日使用同一过滤规则生成股票池。
5. 服务把每个交易日的入池证券写入 `pool_member`，并更新 `run_day.pool_count`。

输出：

```text
run_id, trade_date, security_code, selected_metrics, filter_snapshot
```

`pool_member.selected_metrics` 保存轻量指标快照，只包含规则版本 `output_metrics` 中声明的展示和排查字段，不保存 ClickHouse 查询中参与 join 的全部指标。这样可以让完整股票池具备基本解释能力，同时控制 PostgreSQL 存储膨胀。

### Part 2：日内评分与 TopN 买入信号

输入：

- Part 1 每日股票池
- `scoring` 配置
- `top_n`

处理：

1. 每个交易日只在当日股票池内部评分，不能跨日期比较原始指标值。
2. 评分因子默认使用 `rank_pct_by_day`，避免不同指标量纲直接相加。
3. 服务按 `score DESC, security_code ASC` 生成稳定排名。
4. 每个交易日取 `rank <= top_n` 写入 `buy_signal`。

输出：

```text
run_id, trade_date, security_code, rank, score, score_breakdown, selected_metrics
```

如果某个交易日股票池数量小于 `top_n`，该日买入信号数量等于实际股票池数量。

`buy_signal.score_breakdown` 第一版建议使用 JSONB，而不是为每个评分因子建固定列。这样可以保留解释性，同时避免评分规则变化时频繁迁移表结构。

`buy_signal` 必须保存买入信号解释所需的结果快照：

1. `score_breakdown.raw_values` 保存评分条件实际用到的原始指标值，保证历史分数可以解释。
2. `selected_metrics` 保存规则版本 `output_metrics` 中声明的展示指标，服务查询历史买入信号时不需要再依赖 ClickHouse 当前 mart 状态。
3. 不把所有可用 mart 字段都写入 PostgreSQL；未声明为 `output_metrics` 且未参与评分解释的字段，需要时再查询 ClickHouse 当前值，并在 API 中标记为非运行时快照。

示例：

```json
{
  "method": "weighted_sum",
  "factors": [
    {
      "metric": "amount",
      "raw_value": 123456789.0,
      "transform": "rank_pct_by_day",
      "direction": "desc",
      "normalized_value": 0.92,
      "weight": 0.3,
      "contribution": 0.276
    },
    {
      "metric": "rsi_6",
      "raw_value": 42.5,
      "transform": "rank_pct_by_day",
      "direction": "asc",
      "normalized_value": 0.61,
      "weight": 0.2,
      "contribution": 0.122
    }
  ],
  "final_score": 0.713
}
```

建议字段：

| 字段 | 作用 |
|---|---|
| `score` | 最终总分，直接用于排序和结果展示 |
| `rank` | 当日排名 |
| `score_breakdown` JSONB | 每个评分因子的原始值、归一化值、权重和贡献分 |
| `selected_metrics` JSONB | 买入信号展示和排查用的运行时指标快照 |
| `scoring_method` | 例如 `weighted_sum` |
| `score_version_hash` | 可选，用于确认评分公式版本 |

## 流程与产物矩阵

第一版把一次区间选股运行拆成可审计的固定流程。每个流程都应有明确输入、输出产物和持久化落点，避免后续实现把临时状态隐式藏在内存或日志里。

总体产物流：

```text
RuleDraft
  -> rule_version
  -> ScreeningRunRequest
  -> run
  -> RearviewPlan / CompiledQuery
  -> ClickHouse ranked pool rows
  -> pool_member
  -> buy_signal
  -> run_day + run summary
```

| 流程 | 输入 | 处理 | 输出产物 | 落点 |
|---|---|---|---|---|
| 1. 创建规则集 | HTTP `POST /rearview/rule-sets` 请求，名称、描述、owner | 创建可被多版本规则复用的业务实体 | `rule_set` | PostgreSQL |
| 2. 创建规则版本 | 规则 AST 草案、`pool_filters`、`scoring`、`output_metrics`、当前 metric catalog | 校验 metric 引用、操作符、评分权重和空值策略；计算 `rule_hash` | `rule_version`，包含不可变 rule snapshot、metric dependency snapshot、`rule_hash` | PostgreSQL |
| 3. 设置当前版本 | `rule_set_id`、目标 `rule_version_id` | 将规则集默认版本指针切换到目标版本；不修改历史版本 | `rule_set.current_version_id` | PostgreSQL |
| 4. 发起区间运行 | HTTP `POST /rearview/runs` 请求，`rule_version_id` 或 `rule_set_id`、`start_date`、`end_date`、`top_n`、`mode`、可选 universe override | 如果只给 `rule_set_id`，解析 `current_version_id`；冻结运行参数，创建运行记录，返回 `run_id` | `run` 初始记录，状态为 `created` | PostgreSQL |
| 5. 运行校验 | `run`、`rule_version`、`metric_catalog` | 校验日期区间、`top_n`、规则版本状态、metric catalog 兼容性 | validation result；失败时写 `failed_validation` | PostgreSQL `run.error_message` |
| 6. 交易日解析 | `start_date`、`end_date`、anchor mart 中的 `trade_date` | 推导实际覆盖交易日集合；记录每个交易日的日粒度占位 | `run_day` rows，状态为 `pending`；`trading_day_count` | PostgreSQL |
| 7. 查询计划编译 | rule AST、metric dependency snapshot、日期区间、`top_n` | 推导所需 mart、列、join、where、score expression、rank expression 和 chunk 策略 | `RearviewPlan`、`CompiledQuery`、`compiled_sql_hash`、required mart/column list | PostgreSQL `run` metadata；服务内执行对象 |
| 8. ClickHouse 执行 | `CompiledQuery`、chunk 日期范围、ClickHouse query id | 对区间或 chunk 内每个交易日生成 pool rows、score、rank | ranked pool row stream；ClickHouse query stats | ClickHouse query result；PostgreSQL `run` / `run_day` metadata |
| 9. 写入日股票池 | ranked pool row stream 中的全部 rows | 按 `run_id, trade_date, security_code` 写入 Part 1 结果，并保存轻量 `selected_metrics` | `pool_member` rows；每日 `pool_count` | PostgreSQL |
| 10. 写入买入信号 | ranked pool rows 中 `signal_rank <= top_n` 的 rows | 按日保存 TopN 买入信号、rank、score、score breakdown 和运行时 `selected_metrics` | `buy_signal` rows；每日 `signal_count` | PostgreSQL |
| 11. 汇总运行 | `run_day`、pool rows、signal rows、query stats | 汇总总交易日数、总股票池行数、总信号行数、耗时和失败日 | `run` 终态 `succeeded` 或 `failed_*` | PostgreSQL |
| 12. 查询结果 | HTTP `GET /rearview/runs/{run_id}`、`/days`、`/pool`、`/signals` | 从 PostgreSQL 读取运行摘要、日状态、股票池和买入信号 | API response DTO | HTTP response，不作为新事实源 |
| 13. Explain | HTTP `POST /rearview/explain` 请求、规则草案或规则版本；可选 `start_date`、`end_date`、`top_n` | 只做校验、计划编译、chunk plan 和可选 ClickHouse `EXPLAIN indexes = 1`，不写股票池和信号 | explain report、required marts、compiled SQL、chunk plan、index usage summary | HTTP response；可选写审计日志 |

### 核心产物定义

| 产物 | 粒度 | 最小字段 | 用途 |
|---|---|---|---|
| `rule_version` | 每规则版本一行 | `rule_version_id`、`rule_set_id`、rule AST、`universe_snapshot`、metric dependency snapshot、`rule_hash`、状态 | 保证历史运行可复现 |
| `run` | 每次区间运行一行 | `run_id`、`rule_version_id`、`start_date`、`end_date`、`top_n`、`universe_snapshot`、`resolved_universe_hash`、状态、`compiled_sql_hash`、汇总数量、错误摘要 | 运行主事实和 API 查询入口 |
| `run_day` | 每次运行、每交易日一行 | `run_id`、`trade_date`、状态、`universe_count`、`pool_count`、`signal_count`、chunk/query metadata | 支持断点恢复、日粒度排查和结果浏览 |
| `pool_member` | 每次运行、每交易日、每入池证券一行 | `run_id`、`trade_date`、`security_code`、`score`、`signal_rank`、`selected_metrics` JSONB、filter snapshot | Part 1 股票池事实；`selected_metrics` 只保存 `output_metrics` |
| `buy_signal` | 每次运行、每交易日、每 TopN 证券一行 | `run_id`、`trade_date`、`security_code`、`rank`、`score`、`score_breakdown` JSONB、`selected_metrics` JSONB、`scoring_method`、可选 `score_version_hash` | Part 2 买入信号事实；保存买入信号解释所需快照 |
| `RearviewPlan` | 每次运行或 chunk 一个执行计划 | required mart list、required column list、join plan、filter expression、score expression、chunk range | 服务内执行对象；摘要写入 `run` |
| `CompiledQuery` | 每次运行或 chunk 一个 SQL 产物 | parameterized SQL、bind parameters、`compiled_sql_hash`、ClickHouse query id | 审计、排查和 ClickHouse 执行 |

### 产物依赖关系

```text
rule_set
    └── rule_version
            └── run
                    ├── run_day
                    ├── pool_member
                    └── buy_signal
```

约束：

1. `rule_version` 创建后不可修改；任何规则变更都生成新版本。
2. `run` 必须保存发起时的 `top_n`、日期区间和 universe snapshot，不能只引用当前规则集配置。
3. `pool_member` 是完整股票池，`buy_signal` 是从同一批 ranked pool rows 中派生的 TopN 子集。
4. `buy_signal` 不应独立重新计算评分；它必须能追溯到同一 `run_id`、`trade_date` 下的 `pool_member`。
5. `buy_signal.score_breakdown.raw_values` 是评分解释的运行时事实；历史解释不能依赖回查 ClickHouse 当前 mart 值。
6. `pool_member.selected_metrics` 和 `buy_signal.selected_metrics` 只保存规则版本声明的 `output_metrics`，不把所有可用 mart 字段都写入 PostgreSQL。
7. HTTP response DTO 只是读取视图，不是新的事实源；事实源仍是 PostgreSQL 表和 ClickHouse mart 输入。

## 查询规划草案

第一版查询规划遵守以下原则：

1. 从规则 AST 中收集所需 metric，推导最小 mart 表集合。
2. 每个 mart 表先按 `trade_date BETWEEN <start_date> AND <end_date>` 过滤，再参与 join。
3. 只选择规则筛选、排序和输出需要的列，不使用 `SELECT *`。
4. 优先以 `mart_stock_quotes_daily` 作为 universe 左表；如果规则不需要行情、估值或 universe 条件，可以选择指标 mart 作为 anchor。
5. join key 固定为 `security_code, trade_date`。
6. 逻辑上按每个交易日独立过滤和评分；物理执行上优先编译为区间查询和 `PARTITION BY trade_date` 的日内排名，避免对 5 年区间发起上千个单日查询。
7. 多年区间默认按自然年 chunk 执行，降低单次查询峰值内存并便于断点恢复；短区间可以使用单次 range query，月度 chunk 仅作为年度 chunk 超时或内存压力过高时的降级策略。
8. 结果保留 pool count、signal count、scanned mart tables 和 query id。

`conditional_points` 编译要求：

1. 每条评分规则编译为受控条件表达式和固定 `points`。
2. `raw_score` 是所有命中规则贡献分之和。
3. `score` 是 `raw_score` 经过 `clamp.min` / `clamp.max` 裁剪后的值。
4. `score_breakdown` 由 Rust 根据同一批查询结果和同一份评分规则生成，不允许使用另一套评分逻辑。
5. `rank` / `signal_rank` 使用裁剪后的 `score` 排序；同分时使用 `security_code ASC` 保持稳定。

SQL 形态示例：

```sql
WITH
quotes AS (
    SELECT
        security_code,
        trade_date,
        close_price,
        amount,
        pct_change,
        is_suspend,
        is_st
    FROM fleur_marts.mart_stock_quotes_daily
    WHERE trade_date BETWEEN {start_date:Date} AND {end_date:Date}
),
trend AS (
    SELECT
        security_code,
        trade_date,
        price_ma_5,
        price_ma_20,
        macd_histogram
    FROM fleur_marts.mart_stock_trend_indicator
    WHERE trade_date BETWEEN {start_date:Date} AND {end_date:Date}
),
momentum AS (
    SELECT
        security_code,
        trade_date,
        rsi_6
    FROM fleur_marts.mart_stock_momentum_indicator
    WHERE trade_date BETWEEN {start_date:Date} AND {end_date:Date}
),
pool AS (
    SELECT
        quotes.security_code,
        quotes.trade_date,
        quotes.close_price,
        quotes.pct_change,
        quotes.amount,
        trend.price_ma_5,
        trend.price_ma_20,
        momentum.rsi_6,
        trend.macd_histogram
    FROM quotes
    INNER JOIN trend
        ON quotes.security_code = trend.security_code
        AND quotes.trade_date = trend.trade_date
    INNER JOIN momentum
        ON quotes.security_code = momentum.security_code
        AND quotes.trade_date = momentum.trade_date
    WHERE quotes.is_suspend = false
      AND quotes.is_st = false
      AND quotes.close_price > trend.price_ma_20
      AND trend.price_ma_5 > trend.price_ma_20
      AND momentum.rsi_6 BETWEEN 20 AND 65
      AND trend.macd_histogram > 0
),
scored AS (
    SELECT
        *,
        (
            0.3 * percent_rank() OVER (PARTITION BY trade_date ORDER BY amount ASC)
            + 0.2 * percent_rank() OVER (PARTITION BY trade_date ORDER BY rsi_6 DESC)
            + 0.5 * percent_rank() OVER (PARTITION BY trade_date ORDER BY macd_histogram ASC)
        ) AS score
    FROM pool
),
ranked AS (
    SELECT
        *,
        row_number() OVER (
            PARTITION BY trade_date
            ORDER BY score DESC, security_code ASC
        ) AS signal_rank
    FROM scored
)
SELECT
    security_code,
    trade_date,
    close_price,
    pct_change,
    price_ma_5,
    price_ma_20,
    rsi_6,
    macd_histogram,
    score,
    signal_rank
FROM ranked
ORDER BY trade_date ASC, signal_rank ASC
```

实际实现可以让同一次 ClickHouse 查询返回全部 pool rows、score 和 rank；Rust 将全部 rows 写入 `pool_member`，并将 `signal_rank <= top_n` 的 rows 写入 `buy_signal`。如果 pool 规模过大，则改为两段查询。多年区间按自然年 chunk 依次执行，每个 chunk 独立记录 ClickHouse query id 和写入结果。

## ClickHouse 设计依据

### 运行时 join 控制

ClickHouse 官方建议在延迟敏感的分析查询中减少 join 数量，并优先考虑反范式、dictionary、materialized view 或 `IN` 子查询等方式降低查询时成本。第一版可以直接 join 现有 mart 表，因为 A 股日频数据规模有限；但生产规则如果长期跨多年、跨多张 mart join，应评估专用筛选宽表。

分类：official / derived。

依据：

- ClickHouse docs：`Minimize and optimize JOINs`
- ClickHouse docs：`Joining tables`
- Per `query-join-filter-before`：join 前先过滤各表，必要时用子查询或 CTE 显式过滤。
- Per `query-join-consider-alternatives`：重复低延迟 lookup 或高频 join 应评估 dictionary、denormalization 或 materialized view。

验证：

```sql
EXPLAIN indexes = 1
SELECT ...
```

同时记录 `system.query_log` 中的 read rows、read bytes、memory usage 和 query duration。

### 专用筛选宽表

如果规则选股成为高频交互式路径，或 `2021-01-01` 到 `2025-12-31` 这类多年区间成为常规运行形态，建议新增一个 dbt-owned mart，例如：

```text
fleur_marts.mart_stock_rearview_metric_daily
```

该表按 `security_code, trade_date` 粒度整合常用行情、估值、趋势、动量、成交量和价格形态指标，作为选股服务的主输入。这样把多 mart join 从查询时转移到 dbt 物化时。

分类：derived。

依据：

- ClickHouse docs：denormalization can shift work from query time to insert or pre-processing time.
- ClickHouse docs：materialized views can shift expensive aggregation or transformation work out of query time.
- Per `query-join-consider-alternatives`：高频生产查询不应长期依赖重复 runtime join。

第一版不强制新增该表。触发条件：

1. 单次选股经常跨 3 张以上 mart 表。
2. 多年区间运行需要频繁重复执行。
3. p95 查询延迟无法满足交互需求。
4. `EXPLAIN indexes = 1` 或 `system.query_log` 显示读取行数明显超过目标日期区间 universe。
5. 规则 UI 或 API 需要稳定的一表式指标目录。

### 区间运行与分区

当前 mart 表按 `toYear(trade_date)` 分区，适合 `2021-01-01` 到 `2025-12-31` 这类年边界或跨年区间扫描。第一版不需要为了选股服务引入日分区；日分区会显著增加 partition 数量，除非后续有非常短保留周期和严格按日生命周期操作。

分类：official / derived。

依据：

- ClickHouse docs：partitioning should support lifecycle management and bounded pruning.
- Per `decision-partitioning-timeseries`：时间序列分区应对齐保留和批量操作，不应随意使用过细分区。

处理建议：

1. 区间查询必须带 `trade_date BETWEEN start_date AND end_date`。
2. 多年运行默认按自然年 chunk 执行，使查询和 PostgreSQL 写入都有可恢复边界，并降低单次查询内存峰值。
3. 小区间可以单次 range query 执行；第一版建议阈值为不超过 90 个交易日。
4. 月度 chunk 不作为默认策略，只在年度 chunk 超时、内存压力过高或单年结果写入过大时作为 fallback。
5. 每个 chunk 记录独立 ClickHouse query id、日期范围、状态、耗时和错误摘要，并汇总到同一个 `run`。

### ORDER BY 与日期过滤

选股查询的首要过滤条件通常是日期范围。当前趋势、动量和成交量 mart 已使用 `ORDER BY (trade_date, security_code)`，与按日或按区间筛选模式匹配。`mart_stock_quotes_daily` 当前为 `ORDER BY (security_code, trade_date)`，对 date-only 全市场筛选不是最优前缀。

分类：official / derived。

依据：

- Per `schema-pk-filter-on-orderby`：查询过滤应尽量使用 `ORDER BY` 前缀列。
- ClickHouse docs：primary key / sparse index 使用情况可通过 `EXPLAIN indexes = 1` 验证。

处理建议：

1. 第一版可以继续读取 `mart_stock_quotes_daily`，但必须对真实区间查询做 `EXPLAIN indexes = 1` 和 query log 观测。
2. 如果行情/估值/universe 条件成为每次选股必用输入，优先新增 date-first 的 `mart_stock_rearview_metric_daily`，而不是让选股服务绕过 mart 层读 intermediate。
3. 不为了选股器直接修改现有 mart 主键；如要调整现有 mart 表设计，应另起 dbt model 设计或迁移计划。

### Dictionary 使用边界

ClickHouse dictionary 适合小规模、低变化、唯一 key 的 lookup，例如证券静态分类、交易所、板块或外部标签。规则版本、运行状态和用户业务状态不应放入 ClickHouse dictionary；这些属于 PostgreSQL 业务事务边界。

分类：official / field。

依据：

- ClickHouse docs：dictionaries are a ClickHouse feature for direct joins and key-value lookups.
- Per `query-join-consider-alternatives`：dictionary 可用于频繁 key-based enrichment，但 duplicate keys 会被静默去重，必须保证 key 唯一。

## Rust 服务边界

建议第一版新增单一 Rust crate：`engines/crates/rearview/`，不放入 `pipeline/` 的 Python uv workspace。当前没有复用需求，不需要把 `core`、`io`、service 拆成多个 crate；包内用模块和 `pub(crate)` 边界表达分层即可。

建议结构：

```text
engines/crates/rearview/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── app.rs              # HTTP app/router/bootstrap
    ├── config.rs
    ├── error.rs
    ├── domain/             # 规则、universe、score、run 状态等领域类型
    ├── api/                # request/response DTO, handlers
    ├── planner/            # AST 校验、metric 依赖、查询计划
    ├── clickhouse/         # SQL 编译、query executor、row mapping
    ├── postgres/           # repository、事务、结果写入
    └── service/            # create run, execute run, explain 等编排逻辑
```

约束：

1. `domain` 不依赖 PostgreSQL、ClickHouse、HTTP 框架、环境变量或具体部署方式。
2. `planner` 负责 AST 校验、metric 依赖和查询计划，不直接写 PostgreSQL。
3. `clickhouse` 负责编译和执行 ClickHouse 查询，不知道 HTTP request 结构。
4. `postgres` 负责 repository、事务和结果写入，不做规则计算。
5. `service` 负责编排 create run、execute run 和 explain 流程。
6. 服务 stdout/stderr 或 HTTP response 不暴露未脱敏数据库连接信息。
7. 每次 ClickHouse 查询设置可追踪 query id，建议包含 `run.id`。

后续只有出现明确复用或边界压力时再拆 crate，例如：

1. 规则 AST / SQL planner 要被其他服务复用。
2. 需要单独发布 SDK 或 CLI library。
3. 测试或编译成本明显需要隔离。
4. IO 层和领域层依赖开始互相污染，单 crate 模块边界已经管不住。

## 运行语义

一次选股运行建议状态流转：

```text
created
  -> validating
  -> compiling
  -> running_clickhouse
  -> writing_pool
  -> writing_signals
  -> succeeded
```

失败状态：

```text
failed_validation
failed_compile
failed_clickhouse
failed_write
cancelled
```

运行记录至少保存：

| 字段 | 说明 |
|---|---|
| `run_id` | PostgreSQL 运行主键 |
| `rule_version_id` | 不可变规则版本 |
| `start_date` / `end_date` | 请求运行日期区间 |
| `trading_day_count` | 实际覆盖交易日数量 |
| `top_n` | 每日买入信号数量上限 |
| `universe_snapshot` | 运行时实际使用的 universe 配置快照 |
| `resolved_universe_hash` | 解析后股票范围 hash，用于审计和排查 |
| `rule_hash` | 规则 AST hash |
| `compiled_sql_hash` | 编译 SQL hash |
| `clickhouse_query_id` | ClickHouse 查询追踪 id；分 chunk 时保存主 query id 或摘要 |
| `pool_count` | 区间内股票池总行数 |
| `signal_count` | 区间内买入信号总行数 |
| `started_at` / `finished_at` | 运行时间 |
| `error_message` | 失败摘要 |

## 指标目录

`metric_catalog` 应避免成为新的指标事实源。它只记录“选股器可引用哪些 mart 字段，以及这些字段在规则引擎中如何使用”，字段存在性、字段描述和字段语义仍以 dbt mart YAML 和设计文档为准。

第一版采用半自动方案：**dbt mart YAML 生成或校验基础字段事实，Rearview 手工维护 curated policy overlay**。不采用纯手工 catalog，也不把所有 mart 字段自动开放给规则引擎。

```text
pipeline/elt/models/marts/*.yml
    ↓
candidate mart field facts
    +
Rearview metric policy overlay
    ↓
metric catalog sync / validation
    ↓
PostgreSQL database `rearview`: table `metric_catalog`
```

职责拆分：

| 来源 | 负责内容 | 不负责内容 |
|---|---|---|
| dbt mart YAML | mart 表、物理字段名、字段描述、字段类型和字段语义来源 | 是否允许用户过滤、评分和输出 |
| Rearview policy overlay | `logical_metric`、canonical 来源、`allow_filter`、`allow_scoring`、`allowed_ops`、`null_policy`、`default_output`、`value_kind` | 重新定义 mart 字段语义 |
| PostgreSQL `rearview.metric_catalog` | 运行时 allowlist 和规则校验输入 | 成为第二套长期字段字典 |

第一版只纳入代表性用例和近期规则需要的字段。新增 metric 必须先在 dbt mart YAML 中存在，再通过 Rearview policy overlay 显式开放。overlay 引用不存在的 mart 字段、字段类型不兼容、或 canonical 来源冲突时，catalog sync / validation 必须失败。

需要解决的目录问题：

1. 同一逻辑指标可能在多个 mart 暴露，例如 KDJ 同时出现在 quotes mart 和 momentum mart。catalog 必须指定 canonical 来源。
2. 每个 metric 需要类型、单位、空值语义和允许操作符。
3. 每个 metric 需要默认输出名称，避免前端或 API 直接暴露物理列冲突。
4. catalog 变更需要版本化或至少记录生效时间，避免历史规则不可解释。
5. Boolean 指标必须显式标记可用操作符，例如 `n_structure_20_is_valid` 允许 `eq true/false`，不允许数值比较。
6. 表达式比较中的 RHS metric 也必须经过 catalog allowlist 校验，不能只校验左侧 metric。

## 一期实施建议

第一阶段只做最小可用选股闭环：

1. 改造 `pipeline/migrate`，支持同一 PostgreSQL 实例下的 `pipeline` 与 `rearview` 两个 database target。
2. 在 PostgreSQL `rearview` database 新增规则、版本、运行和结果表，表名使用 `rule_set`、`run`、`buy_signal` 等短表名。
3. Rust 实现规则 AST、metric allowlist 校验和 SQL 编译。
4. 建立 metric catalog 半自动维护流程：从 dbt mart YAML 读取或校验基础字段事实，用 Rearview policy overlay 显式开放可过滤、可评分和可输出字段。
5. 提供 HTTP API 创建规则版本、发起运行、查询每日股票池和每日买入信号。
6. 支持读取现有五张 `fleur_marts` 指标和形态表：quotes、trend、momentum、volume、price pattern。
7. 支持日期区间运行，逻辑上对每个交易日独立过滤；多年区间默认按自然年 chunk 执行。
8. 支持 `AND` 规则、数值比较、布尔比较、字段间比较、字段乘常数表达式、`weighted_sum`、`conditional_points`、clamp 和每日 TopN。
9. 运行股票池和买入信号写 PostgreSQL `rearview` database。
10. 提供 `explain` 模式，输出编译 SQL、所需 mart、输出列、日期区间 chunk plan 和 ClickHouse `EXPLAIN indexes = 1` 摘要。

第二阶段再评估：

1. 鉴权、用户隔离和 UI。
2. 定时运行和 Dagster 编排。
3. `mart_stock_rearview_metric_daily` 专用筛选宽表。
4. 信号序列分析和轻量回测。
5. 结果集 ClickHouse 化，用于大规模历史分析。

## 验收标准

RFC 落地后的第一版应满足：

1. PostgreSQL 使用同一个实例下的 `pipeline` 与 `rearview` 两个 database；现有 OCR 相关表留在 `pipeline` database，Rearview 表只在 `rearview` database。
2. PostgreSQL DDL 由 `pipeline/migrate` Alembic 管理；Rust `rearview` 服务不自动建表、不执行 DDL migration。
3. `rearview` database 内表名不带服务名前缀或 `screening_` 前缀。
4. 规则版本不可变；历史 run 可以定位到规则 AST、metric catalog 版本和 compiled SQL hash。
5. 任意用户输入不能直接拼接为 SQL 标识符或 SQL 片段。
6. metric catalog 基础字段事实来自 dbt mart YAML 或由其校验；Rearview policy overlay 只维护规则引擎策略，不重新定义字段语义。
7. overlay 引用不存在的 mart 字段、字段类型不兼容、或 canonical 来源冲突时，catalog sync / validation 必须失败。
8. 规则只能引用 catalog allowlist 中的 metric。
9. ClickHouse 查询默认按 `trade_date BETWEEN start_date AND end_date` 限定日期区间，并只读取必要列。
10. 每次运行都有 PostgreSQL run 记录、日粒度运行摘要和 ClickHouse query id。
11. Part 1 结果行包含 `run_id`、`trade_date`、`security_code` 和轻量 `selected_metrics`；`selected_metrics` 只保存规则版本声明的 `output_metrics`。
12. Part 2 买入信号行包含 `run_id`、`trade_date`、`security_code`、rank、score、score breakdown 和运行时 `selected_metrics`。
13. `buy_signal.score_breakdown.raw_values` 保存评分条件实际使用的原始指标值，历史信号解释不依赖回查 ClickHouse 当前 mart 值。
14. 代表性用例 `n_structure_low_reversal_screen` 可以通过规则校验、查询计划编译和 `score_breakdown` 生成。
15. 多年区间运行默认按自然年 chunk 执行；每个 chunk 记录日期范围、ClickHouse query id、状态、耗时和错误摘要。
16. 对代表性多年区间规则运行 `EXPLAIN indexes = 1`，确认筛选 mart 使用目标索引路径或记录无法使用的原因。
17. 服务对 ClickHouse 超时、PostgreSQL 写入失败和规则校验失败有明确错误状态。

## 待决问题

1. 是否需要在第一版就新增 `mart_stock_rearview_metric_daily`，还是先用现有 mart runtime join 验证需求？
2. universe 规则是否只支持 ST/停牌/上市状态，还是要支持行业、板块、市值区间和自定义股票池？
3. 评分规则在 `weighted_sum` 和 `conditional_points` 之外，是否还要支持 veto 因子、上限封顶分组或跨因子依赖？

## 相关文档

- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/ADR/0009-clickhouse-layered-databases.md`
- `docs/RFC/archive/0014-clickhouse-layered-database-migration.md`
- `docs/RFC/archive/0016-rust-furnace-compute-engine.md`
- `engines/README.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_momentum_indicator.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_volume_indicator.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_price_pattern_daily.md`

## 外部依据

- ClickHouse docs: `Minimize and optimize JOINs`
- ClickHouse docs: `Joining tables`
- ClickHouse docs: `Query optimization`
- ClickHouse docs: `Materialized views versus projections`
- ClickHouse docs: `Dictionaries`
- ClickHouse docs: `Partitions`
- ClickHouse best-practice rules checked:
  - `query-join-filter-before`
  - `query-join-consider-alternatives`
  - `schema-pk-filter-on-orderby`
  - `query-index-skipping-indices`
  - `decision-partitioning-timeseries`
