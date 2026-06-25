# Plan 0045: Racingline 策略选股 Step 1 缺口填补实施计划

日期：2026-06-21

状态：Completed

关联文档：

- [RFC 0024: Racingline 策略选股 Step 1 实现切入方案](../../RFC/archive/0024-racingline-strategy-selection-step1.md)
- [RFC 0023: Racingline 前端原型驱动开发流程](../../RFC/archive/0023-racingline-frontend-prototype-led-development.md)
- [ADR 0011: Racingline 前端技术栈和工程边界](../../ADR/0011-racingline-frontend-technology-stack.md)
- [ADR 0013: Racingline UI 栈变体评估](../../ADR/0013-racingline-ui-stack-variant-evaluation.md)
- [ADR 0010: 技术指标字段命名区分窗口参数和算子重数](../../ADR/0010-technical-indicator-field-naming.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)
- [mart_stock_trend_indicator_daily 设计](../../design/dbt_layer/fleur_marts/mart_stock_trend_indicator_daily.md)
- [验收报告：Racingline Strategy Step 1 Gap Closure](../../jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md)

## 目标

1. 将 RFC 0024 的 Step 1 vertical slice 落成可执行计划：`GET /rearview/metrics -> /strategies 策略选股草稿 -> RuleVersionSpec -> POST /rearview/explain -> 可定位反馈`。
2. 优先填补现有实现缺口，而不是扩大策略创建范围。
3. 让趋势指标 `crosses_above` / `crosses_below` 具备统一的数据层、catalog、后端校验和前端展示契约。
4. 为后续权重配置、股池预览、模拟建仓、回测和正式工程迁移留下稳定输入。

## 非目标

1. 不发布规则集或规则版本，不调用 `POST /rearview/rule-sets` 或 rule version API。
2. 不发起真实 run、portfolio run、股池预览、模拟建仓或回测。
3. 不把 `app/racingline_new` 直接升级为正式生产工程；它仍按 RFC 0023 和 ADR 0013 作为原型/验证工程处理。
4. 不在浏览器内计算选股结果、技术指标、成交、持仓或净值。
5. 不在 Rearview request path 里用动态 `lag()`、自连接或 ad hoc SQL 临时计算 crossing 前值。
6. 不把前端静态 catalog、UI overlay 或 display hint 当作指标字段事实源。

## 当前事实基线

### 已有能力

| 领域 | 当前事实 |
|---|---|
| 原型 Step 流程 | `app/racingline_new/src/routes/strategy-page.tsx` 已有 `策略选股 -> 权重配置 -> 股池预览 -> 模拟建仓 -> 策略回测` 的页面节奏。 |
| 条件 UI | `app/racingline_new/src/features/strategy/components/condition-groups-panel.tsx` 和 `comparison-fields.tsx` 已支持指标组、条件增删、比较对象、数值/指标比较和 AND/OR 控件。 |
| 静态 catalog | `app/racingline_new/src/features/strategy/catalog.ts` 仍是本地 mock，且含旧字段名和旧 mart 表名。 |
| 正式 Racingline API 经验 | `app/racingline/src/api/client.ts`、`api/rearview.ts`、`api/hooks.ts`、`types/rearview.ts` 和 `store/workbench.ts` 已有 Rearview API、TanStack Query hook、TS 类型和 `RuleVersionSpec` builder 经验。 |
| Rearview catalog | `engines/crates/rearview-core/src/domain/catalog_policy.rs` 已能从 `metric_policy.yml` 读取 policy，并对照 dbt mart YAML 检查表字段和 `data_type`。 |
| Rearview explain | `POST /rearview/explain` 已存在，Planner 能编译普通比较、`between` 和 `is_null`，且空 `scoring.rules` 当前编译为 score `0`。 |
| 趋势 mart | `pipeline/elt/models/marts/mart_stock_trend_indicator_daily.sql` 当前输出 MA、组合 MA、EMA2、BOLL 和 MACD 当期值，粒度为每证券、交易日一行。 |

### 已确认缺口

| 缺口 ID | 缺口 | 影响 | 填补方向 |
|---|---|---|---|
| G1 | 趋势 mart 没有 `prev_*` 前值字段。 | 上穿/下穿不能用单行谓词稳定表达。 | 在 `mart_stock_trend_indicator_daily` 输出同一证券上一交易行前值，并同步 YAML 和设计文档。 |
| G2 | `metric_policy.yml` 仍有旧 mart 表名和旧字段名，且缺少 crossing 元数据、display hint、ignored fields 和 coverage 检查。 | catalog 容易漂移，前端不能知道哪些指标可 crossing。 | 重构 policy 结构，增加 `cross.previous_metric`、operator profiles、display、ignored fields 和 coverage/check 命令。 |
| G3 | Rearview `Operator` 只支持 `eq/ne/lt/lte/gt/gte/between/is_null`。 | `crosses_above` / `crosses_below` 不能通过 validation 或 SQL planner。 | 扩展 domain operator、catalog policy、rule validation 和 planner 编译。 |
| G4 | `MetricDefinition` API 没有暴露 `cross.previous_metric` 或 display hint。 | 前端只能猜测 crossing 能力和中文展示。 | API response 增加非事实展示字段和 crossing 能力字段，字段事实仍来自 dbt YAML + policy。 |
| G5 | `app/racingline_new` 没有 API client、hooks、`QueryClientProvider` 和 base URL 读取约定。 | `/strategies` 不能连 Rearview，也没有服务端状态缓存和错误态。 | 从 `app/racingline` 迁移最小 API 子集，并挂载 query client。 |
| G6 | 原型 UI catalog 与 Rearview `MetricDefinition` 没有 adapter。 | 真实 catalog 无法进入现有指标类型、指标、操作符和比较对象控件。 | 新增 catalog adapter，输出按展示组组织的 filter/scoring option。 |
| G7 | `StrategyConditionGroup[]` 没有转换到 `FilterExpr` / `RuleVersionSpec`，组内 AND/OR 混排也没有明确 AST 语义。 | Step 1 无法生成后端权威草案，混排条件容易被前端、后端或 SQL 优先级解释成不同含义。 | 新增 `buildStrategySelectionRuleSpec()`，覆盖条件组混排解析、operand、operator、output metrics 和 top N。 |
| G8 | explain 结果和错误没有接入 Step 1。 | 用户无法知道草稿是否被 Rearview 接受。 | 增加校验动作、结果面板、失败摘要和条件行级映射。 |
| G9 | 缺少针对 mock -> API 的迁移验收样本和浏览器基线。 | 容易实现成“看起来能点”，但没有证明 vertical slice 真的闭环。 | 补单元测试、后端 planner 测试、catalog 命令、dbt build、Playwright CDP 截图/交互验收报告。 |
| G10 | `app/racingline_new` 和 `app/racingline` 的正式边界未在实施阶段落地。 | 原型代码可能绕过 ADR 0011 直接沉淀为正式工程。 | 本计划只验证原型 vertical slice；正式迁移需另起计划或更新 ADR。 |

## 填补原则

1. 数据事实先行：上穿/下穿依赖的上一期值必须先进入 mart contract，再进入 metric catalog。
2. 后端能力是前端可选项的上限：前端只展示 Rearview catalog 明确允许的指标和操作符。
3. UI 草稿模型保留，但只通过 adapter 生成 `RuleVersionSpec`；组件内不拼后端 AST。
4. explain 是 Step 1 的唯一权威反馈；本阶段不让“下一步模拟结果”承担规则正确性判断。
5. 每个阶段都带测试或机械检查，不把测试集中留到最后。

## 实施阶段

### 阶段 1：趋势前值字段和 dbt 契约补齐

目标：让 crossing 所需的“上一交易行值”成为 mart 层稳定字段。

任务：

1. 在 `mart_stock_trend_indicator_daily.sql` 中为 crossing-eligible 数值趋势字段增加 `prev_*` 字段，使用 `lag(field) over (partition by security_code order by trade_date)`。
2. 初始覆盖 RFC 0024 列出的 MA、组合 MA、`price_ema2_10`、BOLL 和 MACD 字段。
3. 同步 `mart_stock_trend_indicator_daily.yml`，为每个 `prev_*` 声明 `Nullable(Float64)` 和“同一证券上一交易行”的描述。
4. 更新 `docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator_daily.md`，记录 NULL 语义、停牌/非交易日语义和 crossing 消费边界。
5. 保持 `(security_code, trade_date)` 唯一粒度不变。

测试策略：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator_daily
```

完成标准：

- mart SQL、YAML 和设计文档同时包含当前值与 `prev_*` 字段。
- 首个交易行、warm-up NULL 和上一交易行字段为 NULL 时，`prev_*` 保持 NULL，不做填充。
- dbt build 通过，唯一粒度测试仍通过。

### 阶段 2：Rearview metric policy 和 catalog 门禁补齐

目标：把 metric catalog 从“能列指标”提升为“能声明可筛选、可评分、可 crossing、可展示，并能防漂移”的运行时契约。

任务：

1. 修正 `engines/crates/rearview-core/config/metric_policy.yml` 中的旧 mart 表名和旧字段名，例如 `mart_stock_trend_indicator`、`boll_dn_20_2`。
2. 增加 operator profile，避免每个 metric 重复完整 `allowed_ops`。
3. 增加 `cross.previous_metric`，并要求它引用同一 catalog 内存在的 `prev_*` metric 或可校验字段。
4. 增加 `display.group`、`display.label_zh`、`display.unit`、`display.sort_order` 等展示 hint；这些字段只影响前端展示。
5. 增加 `ignored_fields`，要求 eligible mart 中不进入 Rearview 的字段写明排除原因。
6. 扩展 `catalog check`，继续检查 dbt 字段存在性和类型兼容，同时检查 crossing 前值引用。
7. 新增 `catalog coverage`，检查 eligible mart 非主键字段必须出现在 `metrics` 或 `ignored_fields`。
8. 新增或保留稳定序列化检查，避免 YAML 排序和 profile 展开造成无意义 diff。

测试策略：

```bash
cd engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
cargo test -p rearview-core catalog_policy
```

完成标准：

- `GET /rearview/metrics` 的来源可追溯到 dbt mart YAML + policy overlay。
- policy 引用不存在字段、旧 mart 表名、旧字段名、类型不兼容和无效 `previous_metric` 时会失败。
- eligible mart 新增字段未处理时，coverage 会失败。

### 阶段 3：Rearview crossing operator 和 explain 编译补齐

目标：让后端能够校验并编译趋势上穿/下穿。

任务：

1. 在 Rearview domain `Operator` 中新增 `crosses_above` 和 `crosses_below`。
2. 扩展 `MetricDefinition`，增加 crossing 元数据，例如 `cross.previous_metric`。
3. Rule validation 增加 crossing 规则：
   - 左操作数必须是 metric。
   - 左 metric 必须配置 `previous_metric`。
   - 右操作数为 metric 时，也必须配置 `previous_metric`。
   - 右操作数为常量时，previous right 等于同一常量。
   - crossing 只允许数值/整数 metric。
4. Planner 将 crossing 展开为当前谓词和前值谓词：
   - `crosses_above(left, right)` 编译为 `left > right AND prev_left <= prev_right`。
   - `crosses_below(left, right)` 编译为 `left < right AND prev_left >= prev_right`。
5. `required_metrics` 和 `required_columns` 必须包含当前指标和前值指标/字段。
6. explain 错误信息至少能区分：不支持 operator、缺少 previous metric、类型不兼容、右侧 operand 错误。

测试策略：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

关键测试用例：

- `price_ma_5 crosses_above price_ma_20` explain 成功。
- `macd_dif crosses_above macd_dea` explain 成功。
- `price_ma_20 crosses_above 0` explain 成功。
- 非趋势 metric 或缺少 `previous_metric` 的 crossing 被拒绝。
- crossing 依赖出现在 dependency snapshot、required metrics 和 required columns 中。

完成标准：

- `POST /rearview/explain` 接受 RFC 0024 定义的 crossing 规则。
- 非 crossing 的现有规则不发生行为回归。
- 错误响应足以让前端展示明确失败原因；字段级路径不足时，先保留摘要映射方案，并记录下一阶段 error contract 增强项。

### 阶段 4：`app/racingline_new` API 运行时补齐

目标：让原型工程具备调用 Rearview 的最小正式运行时。

任务：

1. 从 `app/racingline` 迁移最小 API 子集：
   - request client
   - `listMetrics`
   - `explainRule`
   - query keys
   - `useMetricsQuery`
   - `useExplainMutation`
   - `MetricDefinition`、`RuleVersionSpec`、`FilterExpr`、`Operand`、`ExplainResponse` 等类型
2. 在 `main.tsx` 挂载 `QueryClientProvider`，沿用 TanStack Query 作为服务端状态缓存。
3. 按 ADR 0011 约定读取仓库根目录 `.env` 中的 `VITE_REARVIEW_API_BASE_URL`，不在 `app/racingline_new` 下新增 `.env*`。
4. API 不可用时，`/strategies` 保留页面壳和可理解错误态，不退回静态 catalog 作为成功状态。
5. 保留静态 catalog 只作为测试 fixture 或明确标注的 dev fallback，不参与 explain 成功路径。

测试策略：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm run build
```

完成标准：

- `/strategies` 能发起 `GET /rearview/metrics`。
- 无 Rearview 服务时，用户看到 metrics 加载失败，不误以为规则已经可提交。
- TanStack Query provider 和 hook 使用不报 runtime error。

### 阶段 5：Metric catalog 到 Step 1 UI adapter 补齐

目标：用真实 catalog 驱动指标类型、指标列表、操作符和比较对象。

任务：

1. 新增 `buildStrategyMetricCatalog()` 或同等 adapter，把 flat `MetricDefinition[]` 转为 UI option：
   - 按 `display.group` 或 `mart_table` 分组。
   - 只展示 `allow_filter = true` 的指标。
   - 按 `value_kind` 映射 number/boolean/string/date UI 输入。
   - 按 `allowed_ops` 过滤操作符。
   - 仅当 metric 暴露 crossing operator 且 `cross.previous_metric` 有效时展示 `crosses_above` / `crosses_below`。
2. 对 `neq` 做兼容迁移：UI 内部统一为后端 `ne`，不再生成 `neq`。
3. 处理加载中、空 catalog、错误、搜索无结果和字段描述缺失状态。
4. 保持 Step 1 的高密度编辑体验，避免把 catalog 变成长列表堆叠。

测试策略：

```bash
cd app/racingline_new
npm run typecheck
npm run lint
```

关键测试用例：

- `allow_filter = false` 的 metric 不出现在过滤条件选择器中。
- `boolean` metric 只展示 `eq/ne/is_null` 等兼容操作符。
- 非 crossing metric 不展示上穿/下穿。
- crossing metric 与非 crossing metric 比较时，UI 阻止生成无效条件。

完成标准：

- Step 1 选择器不再依赖 `features/strategy/catalog.ts` 的静态事实。
- 用户能从真实 metrics 中配置数值、布尔、between、指标比较和 crossing 条件。

### 阶段 6：RuleVersionSpec adapter 补齐

目标：把 Step 1 草稿转换为 Rearview 可 explain 的最小规则草案。

任务：

1. 新增 `buildStrategySelectionRuleSpec(conditionGroups, catalog, options)`。
2. 生成默认 universe：
   - `base = "all_a_shares"`
   - `exclude_st = true`
   - `exclude_suspend = true`
   - include/exclude 证券代码第一阶段留空
3. 将组间固定 AND 生成 `FilterExpr { type: "all" }`。
4. 组内 AND/OR 混排必须支持，采用 `AND` 高于 `OR` 的固定优先级，并生成 nested `all` / `any` AST；不得降级为统一 AND/OR。
5. operand 生成规则：
   - 指标比较：`{ type: "metric", name }`
   - 数值：`{ type: "number", value }`
   - 布尔：`{ type: "bool", value }`
   - 字符串/日期：按后端支持情况生成 string，未支持的输入在 UI 禁用。
   - between：右侧生成 `range` operand。
   - is_null：不发送 right side。
6. `scoring.rules` 第一阶段允许为空，依赖后端当前 `compile_score()` 行为输出 `0`；如果后端 validation 收紧，再使用第一个 `allow_scoring` numeric metric 作为 placeholder，并在 UI 标记。
7. `top_n_default` 默认使用页面 TopN 或 `20`。
8. `output_metrics` 默认取 catalog 中 `default_output = true` 的指标，并强制包含过滤条件使用的指标和 crossing 前值依赖。

#### 组内混排解析设计

现有草稿模型中，`StrategyCondition.logic` 只在第二个及后续条件前显示，语义是“前一个条件到当前条件的连接符”。第一个条件的 `logic` 字段不得参与 AST 生成，避免隐藏状态影响规则。

适配器新增两个纯函数：

```text
buildGroupFilterExpr(group: StrategyConditionGroup, catalog)
buildMixedLogicFilterExpr(conditions: StrategyCondition[], catalog)
```

`buildMixedLogicFilterExpr()` 使用下面的文法：

```text
group_expr := or_segment ("or" or_segment)*
or_segment := condition ("and" condition)*
```

扫描规则：

1. 将第一个条件转换为第一个 `and` segment 的首个 leaf。
2. 从第二个条件开始读取 `condition.logic`：
   - `and`：把当前 condition leaf 追加到当前 segment。
   - `or`：关闭当前 segment，开启新的 segment，并把当前 condition leaf 放入新 segment。
3. segment 内有多个 leaf 时生成 `{ type: "all", conditions: [...] }`；只有一个 leaf 时直接使用该 leaf。
4. group 内有多个 segment 时生成 `{ type: "any", conditions: [segmentExpr...] }`；只有一个 segment 时直接使用该 segment。
5. 外层 `pool_filters` 始终生成 `{ type: "all", conditions: groupExprs }`，表达指标组之间固定 AND。
6. 所有 `all` / `any` 的 `conditions` 必须非空；空 group 在 adapter validation 阶段失败，不发送 explain。

示例映射：

| UI 组内条件 | Group AST |
|---|---|
| `A and B and C` | `all([A, B, C])` |
| `A or B or C` | `any([A, B, C])` |
| `A and B or C` | `any([all([A, B]), C])` |
| `A or B and C` | `any([A, all([B, C])])` |
| `A and B or C and D` | `any([all([A, B]), all([C, D])])` |

多组示例：

```text
Group 1: A and B or C
Group 2: D or E and F

pool_filters =
  all([
    any([all([A, B]), C]),
    any([D, all([E, F])])
  ])
```

实现约束：

1. 生成的是结构化 `FilterExpr`，不得先拼字符串再解析。
2. `AND` 优先级必须由 nested AST 表达，不能依赖 ClickHouse SQL 默认优先级。
3. adapter 应保留 `conditionId -> filter path` 的本地映射，用于 explain 失败后尽量定位到条件行；该映射不写入 `RuleVersionSpec`。
4. UI 若后续展示规则摘要，应按同一 AST 加括号渲染，例如 `(A AND B) OR C`，避免用户误读。

测试策略：

```bash
cd app/racingline_new
npm run typecheck
npm run lint
```

关键测试用例：

- 数值比较。
- 指标比较。
- `between`。
- boolean `eq/ne/is_null`。
- `is_null` 无 right side。
- 趋势 metric-vs-metric crossing。
- 趋势 metric-vs-constant crossing。
- 组内 `A and B or C` 生成 `any([all([A, B]), C])`。
- 组内 `A or B and C` 生成 `any([A, all([B, C])])`。
- 组内 `A and B or C and D` 生成 `any([all([A, B]), all([C, D])])`。
- 多组混排生成顶层 `all([group1Expr, group2Expr])`。
- 第一个 condition 的隐藏 `logic` 字段不影响 AST。
- 空条件组被拒绝，不发送 explain。

完成标准：

- 生成的 JSON 可直接提交给 `POST /rearview/explain`。
- adapter 测试覆盖主要 operand、operator 和组内 AND/OR 混排，不依赖组件点击才能验证 AST 正确性。

### 阶段 7：Explain 反馈闭环补齐

目标：让用户在 Step 1 获得 Rearview 的确定反馈，并让后续步骤只消费已校验草案。

任务：

1. 在 Step 1 footer 或右侧面板增加“校验规则”动作；第一阶段不把主按钮直接跳到 Step 2。
2. 提交 `RuleVersionSpec` 到 `POST /rearview/explain`，日期区间暂不传。
3. 展示 explain summary：
   - `sql_hash` 或 `compiled_sql_hash`
   - `required_metrics`
   - `required_marts`
   - `required_columns`
   - 可选 `chunk_plan` 数量
   - 只读 `RuleVersionSpec` JSON
4. explain 失败时展示错误摘要，并尽量映射到条件行；无法定位到字段时，明确是后端错误 contract 限制。
5. 成功 explain 后保存 `lastRuleSpec`、`lastExplainResult` 和 `lastExplainAt` 到 strategy draft state。
6. 修改任何 Step 1 条件后，将 explain 状态置为 stale，禁用后续真实 API 行为。
7. “配置权重”可以保留为原型导航，但真实后续步骤必须显示未接入状态或只消费已 explain 成功的草案。

测试策略：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm run build
```

浏览器验收：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

完成标准：

- 成功 explain 后，用户能看到依赖的 metrics/marts/columns。
- crossing explain 结果能看到当前字段和 `prev_*` 字段依赖。
- 失败 explain 能区分指标、操作符、值类型、缺少前值字段和空条件。
- 修改条件后，页面明确要求重新校验。

### 阶段 8：验收基线和后续交接补齐

目标：把 Step 1 vertical slice 的真实状态沉淀为可复查记录。

任务：

1. 新增 job report，记录：
   - Rearview catalog check / coverage 命令和结果。
   - dbt trend mart build 命令和结果。
   - 前端 lint/typecheck/build 命令和结果。
   - 浏览器验收路径、截图或关键观察项。
2. 保存至少四组 explain 样本：
   - 普通数值比较成功。
   - 指标比较成功。
   - crossing 成功。
   - crossing 缺失前值或非法 operator 失败。
3. 在报告中列出仍未接入的后续步骤：权重配置、股池预览、模拟建仓、回测、正式工程迁移。
4. 若决定把 `racingline_new` 的体验迁回 `app/racingline`，另起迁移计划，并遵守 ADR 0011/0013。

完成标准：

- `docs/jobs/reports/` 有一份可追溯验收报告。
- Step 1 的完成状态不依赖聊天记录或手工描述。
- 后续计划能直接消费本计划的 `RuleVersionSpec` 和 explain 结果，不重新定义 Step 1 契约。

## 依赖顺序

| 先决项 | 解锁项 | 原因 |
|---|---|---|
| 阶段 1 | 阶段 2、3 | crossing 必须先有 `prev_*` 数据字段，catalog 和 planner 才能引用。 |
| 阶段 2 | 阶段 3、5 | 后端和前端都需要知道 metric 能力边界。 |
| 阶段 3 | 阶段 7 | explain 必须先支持 crossing，前端反馈才有权威结果。 |
| 阶段 4 | 阶段 5、7 | API runtime 是 metrics query 和 explain mutation 的前提。 |
| 阶段 5、6 | 阶段 7 | UI adapter 和 RuleVersionSpec adapter 是 explain 请求体前提。 |
| 阶段 7 | 阶段 8 | 验收报告必须基于实际 browser/API 闭环。 |

## 禁止模式

1. 禁止用 `app/racingline_new/src/features/strategy/catalog.ts` 的静态字段作为 explain 成功路径的指标来源。
2. 禁止在前端用硬编码规则猜测某个 metric 是否支持 crossing。
3. 禁止为了支持 crossing 在 Planner 中临时拼 `lag()` 或 self join。
4. 禁止在 dbt mart 中新增 ADR 0010 明确禁止的旧字段别名，例如 `boll_dn_20_2` 或裸 `ma_5`。
5. 禁止绕过 `POST /rearview/explain` 直接让 Step 1 进入真实 run、portfolio run 或回测。
6. 禁止把 `racingline_new` 的 UI 栈变体作为正式工程默认决策。
7. 禁止把组内 AND/OR 混排降级为统一 AND、统一 OR 或左到右无优先级求值。

## 允许保留的例外

1. `app/racingline_new` 可以保留静态 catalog fixture，用于单元测试和 Rearview 不可用时的明确 dev fallback，但 UI 必须标记为非真实后端状态。
2. 第一阶段可暂不做字段级 error path contract；如果后端只返回 summary error，前端先做摘要展示，并把字段路径作为后续 Rearview error contract 缺口。
3. `scoring.rules` 可以为空，前提是 Rearview validation 和 planner 明确允许；否则使用最小 placeholder 并在 UI 标记为 explain placeholder。

## 总体验证命令

文档变更：

```bash
make docs-check
git diff --check
```

数据层：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator_daily
```

Rearview：

```bash
cd engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

前端：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm run build
```

浏览器：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

## 完成标准

1. `mart_stock_trend_indicator_daily` 有 crossing 所需的 `prev_*` 字段和文档契约。
2. Rearview catalog 能暴露真实 metrics、allowed ops、display hint 和 crossing 能力，并能通过 check/coverage 发现漂移。
3. `POST /rearview/explain` 能校验并编译普通过滤、between、is_null、指标比较和趋势 crossing。
4. `/strategies` Step 1 使用真实 `GET /rearview/metrics`，不再用静态 catalog 作为权威来源。
5. Step 1 能生成包含组内 AND/OR 混排 nested `all` / `any` 的 `RuleVersionSpec` 并提交 explain。
6. 成功和失败 explain 都有明确 UI 反馈；修改条件会让 explain 状态失效。
7. 文档、dbt、Rearview、前端和浏览器验收结果写入 job report。
8. 完成后将本计划状态改为 `Completed` 并移入 `docs/plans/archive/`，同时同步 `docs/plans/README.md` 和相关系统地图。
