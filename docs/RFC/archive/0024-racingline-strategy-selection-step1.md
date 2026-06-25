# RFC 0024: Racingline 策略选股 Step 1 实现切入方案

状态：Archived（2026-06-25；归档前状态：Proposed（2026-06-20））
领域：racingline, rearview
关联系统：racingline, rearview
代码根：app/racingline_new/, app/racingline/, engines/crates/rearview-core/
系统地图：docs/systems/racingline.md

路径说明：本文写于 Plan 0053 迁移前；文中的 `app/racingline_new/` 均为历史实现路径，当前 Racingline 前端代码根为 `app/racingline/`。

## 摘要

本文档定义从 `app/racingline_new` 的 `/strategies` Step 1「策略选股」切入，逐步完善 Racingline 前后端策略创建闭环的第一阶段方案。

第一阶段不追求一次性完成完整策略创建、股池预览、模拟建仓和回测，而是先完成一个可验证的 vertical slice：

```text
指标目录
  -> 策略选股表单
  -> RuleVersionSpec 草案
  -> POST /rearview/explain
  -> explain 结果和字段级错误反馈
```

该切口验证“用户选股想法能否被结构化表达，并被 Rearview 后端解释和校验”。只有这条链路稳定后，后续的权重配置、股池预览、模拟建仓、回测和运行策略才有可靠输入。

## 背景

`app/racingline_new/` 当前原型已经形成 `看板 -> 选股 -> 回测 -> 运行策略 -> 回到看板` 的业务闭环，但 `/strategies` 中的 Step 1 仍是本地 mock 驱动：

- 指标 catalog 来自 `app/racingline_new/src/features/strategy/catalog.ts`。
- 条件组、比较字段和操作符只在前端本地状态里组合。
- 还没有生成 Rearview 权威 `RuleVersionSpec`。
- 还没有调用 `POST /rearview/explain` 做后端校验。
- 还没有从真实 `GET /rearview/metrics` 读取可用指标、操作符和过滤/评分能力。

与此同时，旧 `app/racingline/` 已经有一套可用但信息架构偏旧的 Rearview API 集成和规则工作台：

- `app/racingline/src/api/rearview.ts`
- `app/racingline/src/api/hooks.ts`
- `app/racingline/src/types/rearview.ts`
- `app/racingline/src/store/workbench.ts`
- `app/racingline/src/features/rules/components/rule-workbench.tsx`

因此第一步不应从零重写所有规则能力，而应把旧工程已有的 API client、类型和 `RuleVersionSpec` builder 经验迁移到新原型的信息架构中。

## 目标

1. 把 `/strategies` Step 1 从静态 mock catalog 迁移为真实 `GET /rearview/metrics` 驱动。
2. 支持用户用表单配置基础选股条件组，并生成后端兼容的 `RuleVersionSpec.pool_filters`。
3. 保留 `app/racingline_new` 当前更适合策略研究工作流的页面结构和交互节奏。
4. 第一阶段只接通 `POST /rearview/explain`，不急于发布规则版本或创建 run。
5. 明确前端草稿模型到 Rearview `RuleVersionSpec` 的适配层边界。
6. 在趋势指标范围内支持 `crosses_above` / `crosses_below`，并通过趋势 mart 预计算前值保证 explain 和后续执行路径一致。
7. 盘点现有资源和欠缺资源，为后续 plan 拆分提供依据。

## 非目标

1. 不在第一阶段完成完整策略创建发布。
2. 不在第一阶段发起 `POST /rearview/runs` 或组合运行。
3. 不实现股池预览、模拟建仓和回测的真实 API 接入。
4. 不把上穿/下穿泛化到全部指标；第一阶段只支持趋势指标 mart 中具备前值字段的指标。
5. 不在浏览器内重算技术指标、选股结果、成交、持仓或净值。
6. 不把 `app/racingline_new` 的静态指标 catalog 当成长期字段事实源。
7. 不在 request path 中用动态 `lag()` 或自连接临时计算前值。

## 当前资源盘点

### 前端原型资源

| 资源 | 路径 | 当前价值 | 问题 |
|---|---|---|---|
| Step 流程和页面骨架 | `app/racingline_new/src/routes/strategy-page.tsx` | 已有 `策略选股 -> 权重配置 -> 股池预览 -> 模拟建仓 -> 策略回测` 的页面节奏 | 仍是 mock 数据和本地状态 |
| 条件组 UI | `app/racingline_new/src/features/strategy/components/condition-groups-panel.tsx` | 支持指标组、组间固定 AND、组内 AND/OR、添加/删除条件 | 还未绑定真实 metric catalog 和后端错误 |
| 比较字段 UI | `app/racingline_new/src/features/strategy/components/comparison-fields.tsx` | 支持指标、操作符、数值/指标比较 | 需要把 `neq` 归一到后端 `ne`，并仅对具备前值字段的趋势指标打开 `crosses_above` / `crosses_below` |
| 原型类型 | `app/racingline_new/src/features/strategy/types.ts` | 有 `StrategyConditionGroup`、`ComparableIndicator` 等适合 UI 编辑的草稿模型 | 不是 Rearview `RuleVersionSpec` |
| 静态指标 catalog | `app/racingline_new/src/features/strategy/catalog.ts` | 用于原型演示和布局验证 | 不能作为权威指标来源 |

### 旧 Racingline 可复用资源

| 资源 | 路径 | 当前价值 | 复用方式 |
|---|---|---|---|
| Rearview API client | `app/racingline/src/api/rearview.ts` | 已封装 `listMetrics`、`explainRule`、rule set/version/run/portfolio APIs | 迁移或复制到 `racingline_new`，保持请求语义一致 |
| Query hooks | `app/racingline/src/api/hooks.ts` | 已有 TanStack Query hooks 和 mutation 模式 | 迁移 `useMetricsQuery`、`useExplainMutation` 的最小子集 |
| Rearview TS 类型 | `app/racingline/src/types/rearview.ts` | 已定义 `MetricDefinition`、`RuleVersionSpec`、`FilterExpr`、`ExplainResponse` | 第一阶段直接复用字段语义 |
| RuleVersionSpec builder | `app/racingline/src/store/workbench.ts` | 已能从旧草稿构造 `RuleVersionSpec`，含代表性低位反转规则 | 提炼为新 `strategy` feature 的 adapter，而不是复用旧 UI 状态 |
| Explain 展示经验 | `app/racingline/src/features/rules/components/rule-workbench.tsx` | 已展示 required metrics/marts/columns/chunk plan 和 JSON preview | 新页面可复用信息结构，但 UI 重新贴合 Step 1 |

### Rearview 后端资源

| 资源 | 路径 | 当前能力 |
|---|---|---|
| Metric catalog API | `engines/crates/rearview-core/src/api/mod.rs` | `GET /rearview/metrics`，支持 `mart_table`、`value_kind`、`allow_filter`、`allow_scoring`、`keyword` |
| Explain API | `engines/crates/rearview-core/src/api/mod.rs` | `POST /rearview/explain`，支持直接提交 `RuleVersionSpec`，可选日期区间返回 chunk plan |
| Rule DSL | `engines/crates/rearview-core/src/domain/rule.rs` | `RuleVersionSpec`、`UniverseSpec`、`FilterExpr`、`Operand`、`ScoringSpec`、`ScoringRule` |
| Planner | `engines/crates/rearview-core/src/planner/sql.rs` | 校验依赖 metrics，编译 SQL，返回 required metrics/marts/columns |
| API 路由 | `engines/crates/rearview-core/src/api/mod.rs` | 已注册 `/rearview/metrics`、`/rearview/explain`、rule sets、runs、portfolio runs |

### 数据层资源

| 资源 | 路径 | 当前能力 | 欠缺 |
|---|---|---|---|
| 趋势指标 mart | `pipeline/elt/models/marts/mart_stock_trend_indicator_daily.sql` | 每证券、交易日输出 MA、组合 MA、EMA、BOLL 和 MACD 当期值 | 尚无 `prev_*` 前值字段，无法稳定表达上穿/下穿 |
| 趋势指标设计文档 | `docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator_daily.md` | 已记录趋势 mart 粒度、字段分组、NULL 语义和验证命令 | 需要补充前值字段语义：同一证券上一交易行，不是自然日前一日 |
| 趋势指标 YAML | `pipeline/elt/models/marts/mart_stock_trend_indicator_daily.yml` | 已声明趋势 mart 字段描述和主键测试 | 需要补充 `prev_*` 字段描述和必要测试 |

## 欠缺资源

### 前端欠缺

1. `app/racingline_new` 没有 Rearview API client、query hooks 和 `VITE_REARVIEW_API_BASE_URL` 读取约定。
2. 新原型没有 `MetricDefinition` 到 UI catalog/group 的转换层。
3. 新原型没有 `StrategyConditionGroup[]` 到 `FilterExpr` 的 adapter。
4. 新原型没有 `RuleVersionSpec` 草案预览和 explain 结果面板。
5. 新原型没有后端 validation error 的定位和展示策略。
6. 当前操作符集合需要按后端能力和 metric 能力裁剪；`crosses_above` / `crosses_below` 只对趋势指标中配置了 `previous_metric` 的字段可用。
7. `output_metrics` 和 `top_n_default` 在新原型的 Step 1 中尚无明确归属。

### 后端/API 欠缺或待确认

1. `POST /rearview/explain` 的错误响应当前主要是通用 validation message，字段路径粒度可能不足。
2. `GET /rearview/metrics` 返回的是 flat list；前端需要本地分组、搜索和筛选策略。
3. 后端当前 `Operator` 支持 `eq`、`ne`、`lt`、`lte`、`gt`、`gte`、`between`、`is_null`；需要新增 `crosses_above` / `crosses_below`，并限制为具备前值字段的趋势指标。
4. `GET /rearview/metrics` 需要暴露或可由 policy 推导 crossing 能力，例如 `cross.previous_metric` 和 crossing operator。
5. 指标字段的用户友好中文名称、分组名和说明需要从 `MetricDefinition.description` 或 UI overlay 补足。
6. 如果需要跨指标比较和二元表达式，前端必须只生成后端已支持的 `Operand.metric`、`Operand.range` 和 `Operand.binary.multiply`。

### 数据层欠缺

1. `mart_stock_trend_indicator_daily` 尚未输出趋势指标前值，无法用单行谓词判断上穿/下穿。
2. 当前 mart 字段只有当期值；Planner 若在 request path 动态 `lag()` 或自连接，会把规则解释和执行复杂度推高。
3. 需要为所有允许 crossing 的趋势字段建立稳定命名：`prev_<current_metric_column>`。
4. 需要定义前值缺失、warm-up NULL 和跨停牌/非交易日场景的匹配语义。

### 文档/验收欠缺

1. 尚无针对 `strategies` Step 1 的独立实施计划。
2. 尚无 mock 到真实 Rearview API 的迁移 checklist。
3. 尚无 step 1 浏览器验收截图基线。
4. 尚无从 explain 失败到 UI 字段错误的示例错误样本。

## 第一阶段设计

### D1: Step 1 只做规则草案和 explain

第一阶段完成标准不是“创建策略成功”，而是：

> 用户可以在 `/strategies` 的「策略选股」中配置一个基础规则草案，并得到 Rearview `explain` 的确定反馈。

这意味着：

- 不创建 `rule_set`。
- 不创建 `rule_version`。
- 不发起 `run`。
- 不进入真实股池预览。
- 只提交 `RuleVersionSpec` 给 explain。

### D2: 前端保留 UI 草稿模型，新增 adapter 输出 RuleVersionSpec

`StrategyConditionGroup` 适合编辑 UI，不应强行改成后端 AST。新增 adapter：

```text
StrategyConditionGroup[]
  -> FilterExpr
  -> RuleVersionSpec
```

初始映射：

| UI 概念 | 后端表达 |
|---|---|
| 组间固定 AND | `FilterExpr { type: "all" }` |
| 组内 AND | `FilterExpr { type: "all" }` |
| 组内 OR | `FilterExpr { type: "any" }` |
| 指标与数值比较 | `Operand.metric` + numeric/bool/string `Operand` |
| 指标与指标比较 | `Operand.metric` + `Operand.metric` |
| 区间比较 | `Operator.between` + `Operand.range` |

组内同时存在 AND 和 OR 时，第一阶段必须支持混排，不允许降级为统一 AND 或统一 OR。实施计划以 [Plan 0045](../../plans/archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md) 的阶段 6 为准：

1. `AND` 高于 `OR`。
2. 按连续 AND segment 生成 nested `all` / `any`。
3. 例如 `A and B or C and D` 必须生成 `any([all([A, B]), all([C, D])])`。
4. 第一个 condition 的隐藏 `logic` 字段不参与 AST 生成。

### D3: 操作符按 Rearview 能力和 metric 能力收敛

第一阶段 UI 只展示后端支持且 metric 类型兼容的操作符。基础比较操作符对普通可过滤指标开放：

| UI 显示 | Rearview `Operator` |
|---|---|
| `=` | `eq` |
| `!=` | `ne` |
| `<` | `lt` |
| `<=` | `lte` |
| `>` | `gt` |
| `>=` | `gte` |
| `区间内` | `between` |
| `为空` | `is_null` |

上穿/下穿进入第一阶段真实 explain 链路，但只对趋势指标开放：

| UI 显示 | Rearview `Operator` | 开放条件 |
|---|---|---|
| `上穿` | `crosses_above` | 左操作数 metric 配置了 `previous_metric`；右操作数为常量，或右操作数 metric 也配置了 `previous_metric` |
| `下穿` | `crosses_below` | 左操作数 metric 配置了 `previous_metric`；右操作数为常量，或右操作数 metric 也配置了 `previous_metric` |

语义定义：

```text
crosses_above(left, right):
  current(left) > current(right)
  AND previous(left) <= previous(right)

crosses_below(left, right):
  current(left) < current(right)
  AND previous(left) >= previous(right)
```

当右操作数是常量时，`previous(right)` 等于该常量。`previous` 指同一 `security_code` 的上一交易行，不是自然日前一日。任一当前值或前值为 NULL 时，crossing 条件默认 `no_match`。

### D4: Metric catalog 是唯一指标来源

`app/racingline_new/src/features/strategy/catalog.ts` 只能作为原型 fallback，不作为正式实现来源。第一阶段必须从 `GET /rearview/metrics` 读取：

- `logical_metric`
- `mart_table`
- `column_name`
- `value_kind`
- `allow_filter`
- `allow_scoring`
- `allowed_ops`
- `null_policy`
- `default_output`
- `description`

前端可以做 UI 分组 overlay，但 overlay 不能改变 metric 事实；只负责展示名、排序和分组。

### D4.1: `metric_policy.yml` 短期治理方案

第一阶段继续维护 `engines/crates/rearview-core/config/metric_policy.yml`，不把 metric catalog 立即迁到 `pipeline/migrate` 生成 SQL 或 PostgreSQL runtime source。原因是 Step 1 的主要风险在字段漂移、遗漏和操作符能力不一致；这些问题可以先通过 YAML 结构优化和机械检查收敛，避免同时改动 dbt metadata、Alembic seed、PostgreSQL catalog 读取和 Rearview runtime bootstrap。

短期分层：

| 层级 | 职责 | 不承担 |
|---|---|---|
| dbt mart YAML | 字段存在性、`data_type`、字段描述、mart 粒度和主键测试 | Rearview 是否允许筛选、评分或 crossing |
| `metric_policy.yml` | Rearview policy overlay：logical metric、来源字段、过滤/评分能力、操作符、NULL 策略、默认输出和前端展示 hint | 重新定义 mart 字段语义 |
| PostgreSQL `metric_catalog` | `catalog sync` 后的运行时快照和 API 查询来源候选 | 长期手工维护的字段事实源 |

`metric_policy.yml` 优化方向：

1. 增加操作符 profile，减少每个 metric 重复声明完整数组。
2. 按 mart/domain 分组并保持稳定排序，降低 review 成本。
3. 对常见数值、布尔、日期字段提供默认能力模板；单个 metric 只覆盖差异。
4. 将 crossing 能力表达为 `cross.previous_metric` + `allowed_ops`，`supports_cross` 可由 catalog 构建阶段推导，避免双写。
5. 增加 `display` hint，例如 `group`、`label_zh`、`unit`、`sort_order`；这些只影响前端展示，不改变字段事实。
6. 增加 `ignored_fields`，要求 eligible mart 中不进入 Rearview 的字段写明排除原因。

示例结构：

```yaml
version: 1

op_profiles:
  numeric_filter: [lt, lte, gt, gte, between, eq, ne, is_null]
  boolean_filter: [eq, ne, is_null]

defaults:
  numeric_metric:
    value_kind: numeric
    allow_filter: true
    allow_scoring: true
    allowed_ops_profile: numeric_filter
    null_policy: no_match

metrics:
  - logical_metric: change_pct
    source:
      mart_table: mart_stock_quotes_daily
      column_name: change_pct
    extends: numeric_metric
    default_output: true
    display:
      group: quotes
      label_zh: 涨跌幅
      unit: pct

  - logical_metric: price_ma_20
    source:
      mart_table: mart_stock_trend_indicator_daily
      column_name: price_ma_20
    extends: numeric_metric
    allowed_ops_profile: numeric_filter
    cross:
      previous_metric: prev_price_ma_20
      allowed_ops: [crosses_above, crosses_below]
    display:
      group: trend
      label_zh: MA20

ignored_fields:
  mart_stock_quotes_daily:
    forward_adjustment_factor: 解释前复权价格口径，不作为策略筛选指标。
```

机械门禁：

- `catalog check`：policy 引用的 mart 表、字段和 `value_kind` 必须与 dbt mart YAML 一致；旧字段名、旧 mart 表名或类型不兼容必须失败。
- `catalog coverage`：所有标记为 Rearview eligible 的 mart 非主键字段，必须在 `metrics` 或 `ignored_fields` 中出现；否则失败，防止新增指标字段被静默遗漏。
- `catalog format` 或稳定序列化检查：生成规范化 YAML diff，避免人工排序和 profile 展开造成无意义变更。

PostgreSQL 生成 SQL 方案保留为后续阶段：当 metric 数量或 mart 数量超过单文件 YAML 可维护边界时，再把 dbt mart YAML / manifest 与 policy overlay 生成 SQL seed，交由 `pipeline/migrate` 持久化到 PostgreSQL，并让 Rearview runtime 从 PostgreSQL 读取 catalog。第一阶段不把这个迁移作为 Step 1 前置条件。

### D5: Explain 面板是 Step 1 的主要反馈

Step 1 右侧或底部应展示 explain 结果：

- `compiled_sql_hash` 或 `sql_hash`
- `required_metrics`
- `required_marts`
- `required_columns`
- 可选 `chunk_plan` 数量
- 只读 `RuleVersionSpec` JSON
- explain 失败时展示错误摘要和可定位字段

第一阶段不展示完整 SQL，除非作为折叠的 debug 区域。

### D6: 趋势指标上穿/下穿的数据契约

上穿/下穿需要同时参考当期值和前一期值。第一阶段不在 Rearview request path 中动态计算前值，而是在趋势 mart 中输出前值字段：

```text
mart_stock_trend_indicator_daily
  price_ma_20
  prev_price_ma_20
  macd_dif
  prev_macd_dif
  macd_dea
  prev_macd_dea
  ...
```

字段范围：为 `mart_stock_trend_indicator_daily` 中允许过滤且数值型的趋势字段补充 `prev_*`。初始覆盖当前 mart 暴露的 MA、组合 MA、双重 EMA、BOLL 和 MACD 字段：

- `price_ma_3`, `price_ma_5`, `price_ma_6`, `price_ma_10`, `price_ma_12`, `price_ma_14`, `price_ma_20`, `price_ma_24`, `price_ma_28`, `price_ma_30`, `price_ma_57`, `price_ma_60`, `price_ma_114`, `price_ma_250`
- `price_avg_ma_3_6_12_24`, `price_avg_ma_14_28_57_114`, `price_ema2_10`
- `boll_mid_10_1p5`, `boll_upper_10_1p5`, `boll_lower_10_1p5`, `boll_mid_20_2`, `boll_upper_20_2`, `boll_lower_20_2`, `boll_mid_50_2p5`, `boll_upper_50_2p5`, `boll_lower_50_2p5`
- `macd_dif`, `macd_dea`, `macd_histogram`

实现建议：

1. 在 `mart_stock_trend_indicator_daily.sql` 中以窗口函数生成前值：`lag(field) over (partition by security_code order by trade_date)`。
2. 因 mart 粒度是每证券、交易日一行，前值语义自然是上一交易行；若上一交易行对应字段为 NULL，则 `prev_*` 也为 NULL。
3. `mart_stock_trend_indicator_daily.yml` 和设计文档同步声明每个 `prev_*` 字段。
4. Rearview metric policy 为可 crossing 的趋势指标配置 `cross.previous_metric = "prev_<column_name>"`，crossing 能力由 catalog 构建阶段推导。
5. Planner 编译 crossing 时把一个条件展开成当前谓词和前值谓词，并把 `required_columns` 同时加入当前字段和 `prev_*` 字段。

这个方案把“上一期”事实沉淀在 mart 层，避免每次 explain 或执行规则时临时拼 `lag()` / self join，也让前端、后端 explain 和未来 run 使用同一套字段契约。

## 初步实现路径

### Phase 0: 趋势 crossing 数据契约

目标：让趋势指标具备可被 Rearview 单行谓词消费的前值字段，并先把 `metric_policy.yml` 治理到可被机械校验。

任务：

1. 在 `mart_stock_trend_indicator_daily.sql` 为 crossing-eligible 趋势字段增加 `prev_*`。
2. 更新 `mart_stock_trend_indicator_daily.yml` 和 `docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator_daily.md`。
3. 修正 `metric_policy.yml` 中的旧 mart 表名、旧字段名和 BOLL 上下轨命名，使其与当前 dbt mart YAML 一致。
4. 在 Rearview metric policy/catalog 中登记 `cross.previous_metric`，并由 catalog 构建阶段推导 crossing 能力。
5. 增加操作符 profile、display hint 和 `ignored_fields` 结构，减少重复并明确未开放字段原因。
6. 增加 `catalog coverage` 检查，确保 eligible mart 字段不会被静默遗漏。
7. 确保 dbt build 后 `(security_code, trade_date)` 粒度不变。

完成标准：

- `mart_stock_trend_indicator_daily` 输出当前值和前值。
- `prev_*` 的语义明确为同一证券上一交易行。
- Rearview 能在 metric catalog 中识别哪些趋势指标允许 crossing。
- `cargo run -p rearview-server -- catalog check` 能发现 policy 引用不存在字段、表名漂移和类型不匹配。
- `cargo run -p rearview-server -- catalog coverage` 能发现 eligible mart 新增字段未被 policy 或 `ignored_fields` 处理。

### Phase 1: 前端契约迁移

目标：让 `app/racingline_new` 能调用 Rearview API。

任务：

1. 从 `app/racingline/src/api/client.ts`、`api/rearview.ts`、`api/hooks.ts` 和 `types/rearview.ts` 迁移最小子集。
2. 支持 `VITE_REARVIEW_API_BASE_URL`。
3. 增加 `useMetricsQuery` 和 `useExplainMutation`。
4. 确保 `npm run typecheck` 和 `npm run lint` 通过。

完成标准：

- `/strategies` 能请求 `GET /rearview/metrics`。
- API 不可用时页面显示可理解错误，不阻塞整个原型页面壳。

### Phase 2: Metric catalog 驱动 Step 1

目标：用真实 metric catalog 替换静态 catalog。

任务：

1. 建立 `MetricDefinition` 到 UI options 的映射。
2. 按 `allow_filter` 过滤可选指标。
3. 按 `value_kind` 和 `allowed_ops` 过滤操作符。
4. 仅在 metric 暴露 crossing operator 且 `cross.previous_metric` 有效时展示 `crosses_above` / `crosses_below`。
5. 保留搜索和分组能力，避免长列表不可用。
6. 对无指标、加载中、失败状态提供空态。

完成标准：

- 用户选择的指标都来自 Rearview catalog。
- 不允许选择 `allow_filter = false` 的指标作为过滤条件。
- 不允许选择后端 `allowed_ops` 不支持的操作符。
- 不允许非趋势指标或缺少前值字段的指标选择 crossing 操作符。

### Phase 3: RuleVersionSpec adapter

目标：把 Step 1 草稿转换成最小可 explain 的 `RuleVersionSpec`。

任务：

1. 新增 `buildStrategySelectionRuleSpec()`。
2. 生成默认 `UniverseSpec`：
   - `base = "all_a_shares"`
   - `exclude_st = true`
   - `exclude_suspend = true`
   - include/exclude 证券代码第一阶段可留空。
3. 生成 `pool_filters`。
4. 生成最小 `scoring`：
   - 若还未进入权重配置，第一阶段可用一个稳定默认：空 `rules` + clamp `[0, 99]`，前提是后端允许。
   - 如果后端要求评分规则，使用第一个 numeric allow_scoring metric 作为临时 `weighted_metric`，并在 UI 标记为 explain placeholder。
5. 生成 `top_n_default`，默认 `20` 或页面已有 TopN。
6. 生成 `output_metrics`，默认使用 catalog 中 `default_output = true` 的指标，并确保包含过滤条件中使用的指标。

完成标准：

- 生成的 JSON 能被 `POST /rearview/explain` 接受。
- adapter 有单元测试覆盖数值比较、指标比较、between、boolean、is_null、趋势 metric-vs-metric crossing 和趋势 metric-vs-constant crossing。

### Phase 4: Rearview crossing 编译

目标：让 `POST /rearview/explain` 能校验并编译趋势 crossing。

任务：

1. 扩展 Rearview `Operator`，新增 `crosses_above` 和 `crosses_below`。
2. 校验 crossing 左操作数必须是配置了 `previous_metric` 的 metric。
3. 若右操作数是 metric，也必须配置 `previous_metric`；若右操作数是常量，则复用常量作为 previous 值。
4. Planner 把 crossing 编译为当前谓词 + 前值谓词。
5. `required_metrics` / `required_columns` 包含前值字段，错误信息能说明缺少 `previous_metric`。

完成标准：

- explain 支持 `price_ma_5 crosses_above price_ma_20`。
- explain 支持 `macd_dif crosses_above macd_dea`。
- explain 支持趋势 metric 上穿/下穿常量阈值。
- 非趋势指标或缺少 `previous_metric` 的 crossing 被拒绝，并返回可理解错误。

### Phase 5: Explain 反馈闭环

目标：用户点击“校验规则”后获得后端确定反馈。

任务：

1. 在 Step 1 增加 `校验规则` 动作。
2. 请求体第一阶段只发送 rule；日期区间可等后续预览/回测阶段再加。
3. 展示 explain summary。
4. explain 失败时展示错误摘要，并尽量映射到条件行。
5. 禁用后续 Step 的真实 API 行为，直到 explain 成功。

完成标准：

- 成功 explain 后用户能看到 required metrics/marts/columns。
- crossing 规则的 explain 结果能展示当前字段和 `prev_*` 字段依赖。
- 失败 explain 后用户能知道是指标、操作符、值类型、缺少前值字段还是空条件问题。
- 不再依赖静态 mock 判断规则是否有效。

### Phase 6: 为后续步骤留下接口

目标：Step 1 完成后能自然接到 Step 2/3。

任务：

1. 把成功 explain 的 `RuleVersionSpec` 存入 strategy draft state。
2. 记录 `lastExplainResult` 和 `lastExplainAt`。
3. Step 2 权重配置只能基于 `allow_scoring = true` 的 metric。
4. Step 3 股池预览后续复用同一个 `RuleVersionSpec` 创建临时 run 或 explain-with-range。

完成标准：

- 后续步骤只消费已 explain 成功的规则草案。
- 修改 Step 1 条件后，后续 explain 状态失效并提示重新校验。

## API 依赖矩阵

| API | 当前状态 | Step 1 用途 |
|---|---|---|
| `GET /healthz` | 已存在 | 可选健康检查 |
| `GET /rearview/metrics` | 已存在，需扩展 crossing 能力元数据 | 指标目录、操作符能力和 `cross.previous_metric` |
| `POST /rearview/explain` | 已存在，需扩展 crossing operator | 规则草案校验和编译摘要 |
| `POST /rearview/rule-sets` | 已存在 | 后续发布阶段使用，Step 1 不调用 |
| `POST /rearview/rule-sets/{rule_set_id}/versions` | 已存在 | 后续发布阶段使用，Step 1 不调用 |
| `POST /rearview/runs` | 已存在 | 后续股池预览/回测阶段使用，Step 1 不调用 |

## 风险与待决问题

1. 后端是否允许 `scoring.rules = []`？如果不允许，Step 1 需要默认 scoring placeholder。
2. 组内 AND/OR 混排已由 Plan 0045 阶段 6 固定为 `AND` 高于 `OR` 的 nested `all` / `any` AST；后续待决项是 UI 是否需要在摘要中显式渲染括号。
3. `MetricDefinition.description` 是否足以支持中文 UI？如果不足，需要 UI overlay，但 overlay 只能补展示，不改事实。
4. explain 错误是否包含字段路径？如果没有，第一阶段只能做摘要提示；字段级定位需要 Rearview error contract 增强。
5. `app/racingline_new` 是否继续作为原型工程，还是开始承接正式实现？本 RFC 默认仍按 RFC 0023 的原型隔离规则执行。
6. crossing 是否需要支持跨 mart 指标组合，例如行情价格上穿趋势均线？第一阶段不主动打开，除非两个操作数都能通过 metric catalog 提供稳定前值字段。

## 验收建议

文档和代码实现阶段分别验收。

文档阶段：

```bash
make docs-check
git diff --check
```

前端实现阶段：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm run build
```

涉及 Rearview API 或错误响应增强时追加：

```bash
cd engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及趋势 mart 前值字段时追加：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator_daily
```

浏览器验收：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

至少检查：

- metrics 加载成功和失败态。
- 新增/删除指标条件。
- 数值、布尔、between、指标比较。
- 趋势指标上穿/下穿可选，非趋势指标不展示上穿/下穿。
- explain 成功态。
- explain 失败态。
- 修改条件后 explain 状态失效。

## 相关文档

- [用户逻辑：Racingline 策略研究工作台](../../Q&A/user-logic.md)
- [Q&A 0004: Racingline 原型看板到策略创建闭环用户故事](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)
- [RFC 0018: Rust Rearview 规则选股服务与 mart 指标库](0018-rust-stock-screening-service.md)
- [RFC 0019: Racingline Rearview 前端工作台](0019-racingline-rearview-frontend-workbench.md)
- [RFC 0023: Racingline 前端原型驱动开发流程](0023-racingline-frontend-prototype-led-development.md)
- [Plan 0045: Racingline 策略选股 Step 1 缺口填补实施计划](../../plans/archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)
