# Plan 0047: Racingline 股池预览 Step 3 实施计划

日期：2026-06-22

状态：Completed

关联文档：

- [RFC 0026: Racingline 股池预览 Step 3 实现方案](../../RFC/archive/0026-racingline-strategy-pool-preview-step3.md)
- [RFC 0025: Racingline 策略权重配置 Step 2 实现方案](../../RFC/archive/0025-racingline-strategy-weight-configuration-step2.md)
- [RFC 0024: Racingline 策略选股 Step 1 实现切入方案](../../RFC/archive/0024-racingline-strategy-selection-step1.md)
- [Plan 0046: Racingline 策略权重配置 Step 2 实施计划](0046-racingline-strategy-weight-configuration-step2-implementation-plan.md)
- [Racingline Strategy Step 2 Preview Implementation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md)
- [Racingline Strategy Step 3 Preview Implementation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step3-preview.md)
- [Q&A 0004: Racingline 原型看板到策略创建闭环用户故事](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)

## 目标

1. 将 `/strategies` Step 3 股池预览从“能展示真实 preview rows”推进为可检查候选池、评分排名、分数解释和个股上下文的稳定页面。
2. 保证 Step 1、Step 2、Step 3 的用户成功路径全部使用 Rearview 真实接口数据，不保留 mock 成功路径。
3. 引入 `PreviewSnapshot`，明确 applied preview 与 draft rule 的状态边界，防止用户修改草稿后继续误用旧结果。
4. 让 Step 3 使用真实 `POST /rearview/strategy-preview` response 展示候选池数量、rank、score、score breakdown、selected metrics 和 raw values。
5. 补齐证券显示信息，但只要求 `security_code`、`security_name` 和 `exchange_code`；不要求、不返回、不展示行业或板块字段。
6. 用真实 preview security analysis 替代固定 mock K 线和 mock 行情快照。
7. 将 full pool 分页、preview row limit、error/empty/loading 状态和浏览器验收沉淀为可执行检查。

## 非目标

1. 不发布 rule set 或 rule version。
2. 不创建正式 `run`、回测 run、portfolio run 或 PostgreSQL run snapshot。
3. 不在浏览器内计算权威股票池、score、rank、K 线、成交、持仓、费用、滑点或净值。
4. 不在 Step 3 处理建仓参数；Step 3 只提供已预览规则和结果上下文。
5. 不调整 Step 1、Step 2、Step 3 的前端可见文案。
6. 不引入新的评分归一化、仓位优化或交易撮合规则。
7. 不展示行业、板块或同类分组信息。
8. 不把 `app/racingline_new/src/features/strategy/catalog.ts` 作为真实数据源。
9. 不允许 Rearview 接口失败时用 mock 数据伪造成成功状态。

## 当前事实基线

### 已有能力

| 领域 | 当前事实 |
|---|---|
| Step 1 catalog | `app/racingline_new` 已有 `GET /rearview/metrics` 接入，能从真实 metric catalog 构造筛选指标选项。 |
| Step 1 rule adapter | `app/racingline_new/src/features/strategy/adapters.ts` 已能生成 `RuleVersionSpec.pool_filters`，支持组内 AND/OR 混排、between、is_null、crossing 和指标比较。 |
| Step 1 explain | `/strategies` 已能调用 `POST /rearview/explain` 做规则校验。 |
| Step 2 scoring catalog | Step 2 已能基于真实 `allow_scoring = true` metrics 构造评分指标选项。 |
| Step 2 scoring adapter | `WeightIndicator[]` 已能生成 `RuleVersionSpec.scoring.rules`，并固定 `scoring.clamp = { min: 0, max: 100 }`。 |
| Preview rule composer | 点击「股池预览」时已能组合 Step 1 + Step 2 草稿生成完整 `RuleVersionSpec`。 |
| Rearview preview API | `POST /rearview/strategy-preview` 已存在，能执行完整规则，返回 `preview_id`、`sql_hash`、required metrics/marts/columns、date range 和按交易日组织的 preview rows。 |
| Step 3 最小展示 | `stock-pool-preview-workbench.tsx` 已能从真实 preview response 展示日期、pool count、rank、score 和 score breakdown。 |

### 已确认缺口

| 缺口 ID | 缺口 | 影响 | 填补方向 |
|---|---|---|---|
| G1 | Step 1/2/3 仍可能存在 prototype fallback 或 mock 成功路径。 | Rearview 失败时用户可能看到假成功结果。 | 审计并移除或隔离所有成功路径 mock；失败只显示 loading/error/empty。 |
| G2 | Step 3 缺少独立 `PreviewSnapshot`。 | applied result、draft rule、stale 和后续阶段 gate 分散在多个 state 中。 | 新增 snapshot 模型和 reducer/状态更新规则。 |
| G3 | Step 3 仍混用 mock K 线、mock 股票名称和 mock 行情快照。 | 用户看到的个股上下文可能与真实 preview row 不一致。 | 用 security display lookup 和 preview security analysis 替换。 |
| G4 | Preview response 缺少 `security_name` 和 `exchange_code`。 | 表格只能显示证券代码，检查效率低。 | Rearview 在 preview rows 合并证券显示信息，只返回证券名称和交易所代码。 |
| G5 | `selected_metrics` 和 `raw_values` 未成为一等展示模型。 | 用户难以解释筛选和评分为什么命中。 | 新增 presenter adapter，按 metric catalog label 格式化展示。 |
| G6 | score breakdown 与 Step 2 权重行映射散落在组件中。 | 权重顺序变化或 rule name 变化时解释易退化。 | 在 snapshot 中保存 scoring rule label map，并集中到 presenter adapter。 |
| G7 | Preview API 当前由 API 层裁剪展示行数。 | 大候选池多日预览可能传输和内存成本过高。 | 后端支持 preview row limit 或单日分页 query，只传页面所需 rows 与 pool count。 |
| G8 | full pool 浏览缺少单日分页 contract。 | 用户只能看首屏 preview rows，无法检查完整候选池。 | 新增 stateless `strategy-preview/pool-page`。 |
| G9 | preview security analysis 缺失。 | Step 3 不能加载选中证券真实 K 线和指标上下文。 | 新增 stateless `strategy-preview/security-analysis`，并校验证券属于当前 rule 的当日候选池。 |
| G10 | 错误状态粒度不足。 | 用户无法区分 adapter、validation、empty pool、backend execution 和 catalog unavailable。 | 统一 Step 3 状态模型和错误分类。 |
| G11 | 浏览器验收还没有覆盖断开 Rearview 后无 mock 成功结果。 | mock fallback 容易回归。 | 增加 live smoke 和 negative smoke。 |

## 填补原则

1. 真实接口优先：Step 1/2/3 成功状态必须能对应到 Rearview network request。
2. 无 mock 成功路径：mock 只允许存在于测试 fixture、离线原型或明确不可交互的开发样例。
3. Applied preview 不实时联动 draft：修改 Step 1、Step 2 或预览日期后只标记 stale，不静默重算。
4. 后端是结果真相：前端不计算权威 pool、score、rank、K 线或 selected metrics。
5. Preview-only 不污染正式状态：不创建 rule set、rule version、run 或 portfolio run。
6. 展示字段收敛：Step 3 只展示证券代码、证券名称、交易所、评分和指标解释；不展示行业或板块。
7. 每阶段先定义测试策略，避免最后补测试。

## 实施阶段

### 阶段 1：真实接口基线收敛

目标：确保 Step 1、Step 2、Step 3 在用户成功路径中全部由 Rearview 真实接口数据驱动。

任务：

1. 审计 `/strategies` Step 1：
   - 指标类型、指标名、操作符和中文 label 必须来自 `GET /rearview/metrics`。
   - 规则校验成功必须来自 `POST /rearview/explain`。
   - metrics 或 explain 失败时不展示 mock 成功结果。
2. 审计 Step 2：
   - 可评分指标必须来自真实 `allow_scoring = true` catalog。
   - scoring adapter 输入不得来自静态 catalog fallback。
3. 审计 Step 3：
   - candidate pool count、rank、score、score breakdown、selected metrics 和 raw values 必须来自 `POST /rearview/strategy-preview`。
   - 本地生成股票池、mock 分数、mock 排名和 mock score breakdown 不得进入成功路径。
4. 隔离 `app/racingline_new/src/features/strategy/catalog.ts`：
   - 只允许测试 fixture 或明确 prototype-only 使用。
   - 不允许 `/strategies` 正式成功路径引用。
5. 补齐 loading/error/empty 状态：
   - Rearview loading：显示 loading。
   - Rearview error：显示 error 和重试入口。
   - Rearview empty：显示 empty，不使用 mock 兜底。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

浏览器验收：

- 正常 Rearview：Step 1/2/3 network 中能看到真实 Rearview 请求。
- 错误 API base URL：Step 1/2/3 不出现 mock 成功结果。

完成标准：

- Step 1/2/3 的成功状态均能追溯到 Rearview response。
- 断开 Rearview 后不会展示可提交或可继续的 mock 成功状态。

### 阶段 2：PreviewSnapshot 状态模型

目标：建立 Step 3 applied preview snapshot，明确 draft 与已应用结果边界。

任务：

1. 新增 `PreviewSnapshot` 类型：
   - `previewId`
   - `appliedRuleSpec`
   - `createdAt`
   - `range.startDate`
   - `range.endDate`
   - `result`
   - `labels.scoringRules`
   - `labels.metrics`
   - `stale`
2. 将现有 `lastPreviewRuleSpec`、`lastPreviewResult`、`lastPreviewAt`、`isPreviewStale` 收敛到 snapshot 或 snapshot-like reducer。
3. 点击「股池预览」成功后创建 snapshot。
4. 点击「更新股池」成功后替换 snapshot。
5. 修改 Step 1 条件、Step 2 权重或 Step 3 日期后，只标记 snapshot stale。
6. 后续阶段 gate 只读取 `stale = false` 的 applied snapshot。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
```

关键测试用例：

- preview success 创建 snapshot。
- 修改 Step 1 后 snapshot stale。
- 修改 Step 2 后 snapshot stale。
- 修改日期后 snapshot stale。
- stale snapshot 不能通过后续阶段 gate。

完成标准：

- Step 3 表格和解释始终展示 applied snapshot，不会静默混入 draft。
- stale 状态清晰、可测试、可阻断后续阶段。

### 阶段 3：Preview presenter adapter

目标：把 `StrategyPreviewResponse` 转成稳定展示模型，移除组件内散落的映射逻辑。

任务：

1. 新增 `buildPreviewPresentation()`：
   - 输入 `StrategyPreviewResponse`、metric catalog、applied weight labels。
   - 输出 trade date rows、candidate pool count、ranked preview rows、score items、selected metrics rows、raw values rows。
2. score breakdown 映射：
   - 优先使用 snapshot 中的 scoring rule label map。
   - 缺失时回退 rule name。
3. selected metrics 映射：
   - 使用 `MetricDefinition.display.label_zh`。
   - 使用 `display.unit` 和 value kind 做数值格式化。
4. raw values 默认折叠，用于解释和调试。
5. 未识别 key 保留原始 key，不丢弃数据。
6. `StockPoolPreviewWorkbench` 只消费 presentation model，不直接解析 backend JSON。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

关键测试用例：

- score breakdown 显示 Step 2 权重行 label。
- selected metrics 使用中文 metric label。
- raw values 保留未识别 key。
- 空 trade_dates 显示 empty。
- response 中没有 `security_name` 时回退 `security_code`。

完成标准：

- Step 3 不再从 mock 计算股票池、分数或排名。
- 解释展示逻辑集中在 presenter adapter，有单元测试覆盖。

### 阶段 4：证券显示信息补齐

目标：让 preview rows 显示证券名称和交易所代码，不展示行业或板块。

任务：

1. 确认或新增 marts 层证券基础快照，字段只要求：
   - `security_code`
   - `security_name`
   - `exchange_code`
2. Rearview 新增 security display lookup helper：
   - 按 `security_code IN (...)` 批量查询。
   - 查询失败不影响 preview 主结果。
3. `POST /rearview/strategy-preview` 在返回 rows 前合并 display snapshot。
4. `POST /rearview/strategy-preview/pool-page` 同样合并 display snapshot。
5. 前端展示：
   - 有 `security_name` 时显示名称 + 代码。
   - 无 `security_name` 时回退代码。
   - 不展示行业或板块占位。

测试策略：

Rearview：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core api
cargo test -p rearview-core clickhouse
```

若新增 mart，追加：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select <security-basic-mart-selector>
uv run python elt/scripts/validate_field_glossary.py
```

前端：

```bash
cd app/racingline_new
npm test
npm run typecheck
```

完成标准：

- Step 3 表格可展示证券名称、证券代码和交易所代码。
- 页面不展示行业、板块或同类分组字段。
- display lookup 失败时 preview 主结果仍可用。

### 阶段 5：Preview SQL 裁剪和全池分页

目标：降低首屏 preview 传输成本，并允许用户检查完整候选池。

任务：

1. Rearview planner 或 API 支持 preview row limit：
   - 首屏只返回页面所需 ranked preview rows。
   - 同时返回每个交易日的完整 `pool_count`。
2. 新增 stateless 单日分页 API：

```http
POST /rearview/strategy-preview/pool-page
```

3. Request 包含：
   - `rule`
   - `trade_date`
   - `limit`
   - `offset`
   - `sort`
   - 可选 `security_code`
4. Response 包含：
   - `trade_date`
   - `pool_count`
   - `items`
   - `limit`
   - `offset`
   - `has_more`
5. 排序固定为 `score DESC, security_code ASC`。
6. 前端 Step 3 支持 selected trade date 下查看 preview rows 和 full pool page。

测试策略：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

前端：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

完成标准：

- 首屏 preview 不再需要把全池 rows 传回浏览器。
- full pool 可以按单日分页检查。
- full pool 和首屏 preview 使用同一条 rule spec 语义。

### 阶段 6：Preview security analysis

目标：用真实个股上下文替代固定 mock K 线和 mock 行情快照。

任务：

1. 新增 stateless API：

```http
POST /rearview/strategy-preview/security-analysis
```

2. Request 包含：
   - `rule`
   - `trade_date`
   - `security_code`
   - `adjustment`
   - `lookback_trading_days`
3. Backend 校验：
   - `security_code` 必须属于当前 rule 在 `trade_date` 的候选池。
   - 不属于候选池时返回 validation error 或 404。
4. Response 复用 RFC 0020 个股分析结构的核心字段：
   - `source = "preview"`
   - `trade_date`
   - `security_code`
   - `security_name`
   - `result_snapshot`
   - `chart_window`
   - `chart`
   - `quote_rows`
   - `indicator_sections`
5. 前端点击 Step 3 表格行后请求 preview security analysis。
6. 图表和右侧数据全部来自真实 response；查询失败时表格和 preview result 仍可用。

测试策略：

Rearview：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core api
cargo test -p rearview-core clickhouse
```

前端：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

完成标准：

- Step 3 不再展示固定 mock K 线。
- 选中证券的图表、行情和指标上下文来自 Rearview。
- 非候选池证券无法获取 preview analysis。

### 阶段 7：错误状态、空状态和 gate 收敛

目标：让 Step 3 在失败、空结果、stale 和后续阶段进入条件上具备明确行为。

任务：

1. 统一错误分类：
   - adapter validation error
   - preview request validation error
   - catalog unavailable
   - backend execution error
   - empty pool
   - empty preview rows
   - security analysis error
2. Step 3 保留 applied snapshot：
   - 新请求失败不覆盖旧成功结果。
   - 若 draft 已变化，旧结果保持 stale。
3. 后续阶段 gate：
   - 需要存在 snapshot。
   - snapshot 必须非 stale。
   - 至少一个交易日 `candidatePoolCount > 0`。
   - 至少一个交易日有 ranked preview rows。
4. Date range 变化只标记 stale，不本地重算。
5. preview row limit 或 pagination 变化不改变建仓语义，只改变展示。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

关键测试用例：

- preview API 失败时旧 snapshot 不被清空。
- empty pool 显示 empty。
- stale snapshot 阻断后续阶段。
- date range change 标记 stale。

完成标准：

- 用户能区分失败、空结果和过期结果。
- 后续阶段不会读取未预览或已过期的规则上下文。

### 阶段 8：浏览器验收和运行报告

目标：用 live Rearview + Racingline 验收 Step 1/2/3 真实接口闭环，并沉淀报告。

任务：

1. 启动 Rearview 和 Racingline dev server。
2. 浏览器验收：
   - Step 1 指标类型、指标名和操作符来自 `GET /rearview/metrics`。
   - Step 1 校验规则调用 `POST /rearview/explain`。
   - Step 2 可评分指标来自真实 `allow_scoring = true` catalog。
   - Step 3 候选池数量、rank、score、score breakdown、selected metrics 和 raw values 来自 `POST /rearview/strategy-preview`。
   - 断开 Rearview 或指向错误 API base URL 后，Step 1/2/3 不出现 mock 成功结果。
   - Step 3 不展示行业或板块信息。
   - Step 1/2/日期修改后 Step 3 标记 stale。
   - 更新股池后 stale 清除，表格使用新 applied result。
   - 选中证券后加载真实 preview security analysis。
3. 增加 job report：
   - 记录命令、URL、API sample、浏览器观察项、失败样本和剩余限制。
4. 同步系统地图或计划归档状态：
   - 实施完成后把本计划移入 `docs/plans/archive/`。
   - 更新 `docs/plans/README.md`。
   - 如新增 API，更新 `docs/systems/rearview.md` 和 `docs/systems/racingline.md`。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
npm run build

cd ../../engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
```

浏览器工具：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

完成标准：

- Step 1/2/3 真实接口闭环有浏览器证据。
- 断开 Rearview 的 negative smoke 证明没有 mock 成功路径。
- job report 可复查。

## 禁止模式

1. 不在 Step 3 本地生成候选池、分数、排名、score breakdown 或 selected metrics。
2. 不在 Rearview 接口失败时显示 mock 成功状态。
3. 不把 preview result 写入正式 run 或 portfolio run 状态。
4. 不从 Step 3 输出建仓参数。
5. 不展示行业或板块字段。
6. 不直接从前端访问 ClickHouse 或 PostgreSQL。
7. 不让 `catalog.ts` 参与用户成功路径。
8. 不把 security analysis 查询失败升级为 preview 主结果失败。

## 最小验证命令

文档阶段：

```bash
make docs-check
git diff --check
```

前端阶段：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
npm run build
```

Rearview 阶段：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
```

涉及 dbt mart 时追加：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select <security-basic-mart-selector>
uv run python elt/scripts/validate_field_glossary.py
```

## 完成标准

1. Step 1/2/3 的用户成功路径都使用 Rearview 真实接口数据。
2. 断开 Rearview 后不会出现 mock 成功结果。
3. Step 3 使用 `PreviewSnapshot` 管理 applied result、stale 和后续阶段 gate。
4. Step 3 表格展示真实 candidate pool count、rank、score、score breakdown、selected metrics 和 raw values。
5. Step 3 可展示证券名称和交易所代码，不展示行业或板块。
6. Step 3 可查看完整候选池分页。
7. Step 3 可加载选中证券的真实 K 线、行情和指标上下文。
8. Preview-only API 不创建 rule set、rule version、run 或 portfolio run。
9. 前端、Rearview、catalog/dbt 和浏览器验收命令通过。
10. 产生 job report，并在完成后归档本计划。

## 后续维护动作

1. 实施完成后将本计划移入 `docs/plans/archive/`，状态改为 `Completed`。
2. 更新 [docs/plans/README.md](../README.md) 的 active/completed 索引。
3. 新增 [docs/jobs/reports/](../../jobs/reports/) 验收报告。
4. 如新增 API 或 mart，同步 [docs/systems/rearview.md](../../systems/rearview.md)、[docs/systems/racingline.md](../../systems/racingline.md) 和相关 design 文档。
5. 如果后续需要刷新恢复 preview result，另起 RFC 设计短期 preview cache、过期策略和用户隔离。
