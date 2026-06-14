# RFC 0019: Racingline Rearview 前端工作台

状态：Completed（2026-06-13）

## 摘要

本文档定义 `racingline` 前端工作台的第一版需求。`racingline` 放在 `app/racingline/`，作为 `app/` 目录下的独立前端工作区，对接 RFC 0018 中定义的 Rearview HTTP API。

第一版定位为内部指标选股工作台，用于把规则创建、运行发起、进度追踪和结果解释串成可操作闭环。它不是营销页面、交易系统或完整回测平台；第一屏应直接进入选股工作流。

关联后端设计：

- `docs/RFC/0018-rust-stock-screening-service.md`
- `engines/crates/rearview/src/api/mod.rs`

关联前端技术决策：

- `docs/ADR/0011-racingline-frontend-technology-stack.md`

关联实施计划：

- `docs/plans/archive/0037-racingline-frontend-implementation-plan.md`

验收报告：

- `docs/jobs/reports/2026-06-13-racingline-playwright-cdp-acceptance.md`

## 目标

1. 提供一个可操作的 Rearview 选股工作台，而不是只展示 API 返回值。
2. 支持创建或选择规则集，表单化配置规则版本，并在发布前执行 explain。
3. 支持按日期区间发起异步选股运行，并追踪 run、chunk 和 day 粒度进度。
4. 支持按交易日查看完整股票池和 TopN 买入信号。
5. 支持打开单个买入信号，查看排名、总分、评分拆解和关键指标快照。
6. 明确前端工程栈和 `app/racingline/` 工作区边界，为后续实现和质量门禁留出稳定约束。

## 非目标

1. 第一版不引入登录入口、认证/鉴权、用户隔离或权限系统；后续如需要，应另起 RFC 统一设计前后端鉴权边界。
2. 第一版不实现交易、下单、风控、组合调仓或完整回测。
3. 第一版不让用户提交任意 SQL。
4. 第一版不要求用户直接手写完整规则 JSON；JSON 只作为只读预览和排查辅助。
5. 第一版不把 ClickHouse 当前 mart 查询值伪装成运行时历史快照。

## 项目边界

前端项目名为 `racingline`，路径固定为：

```text
app/racingline/
```

`racingline` 是 `app/` 目录下的独立前端工作区，负责对接 Rearview HTTP API。Rearview 后端仍是规则校验、查询规划、ClickHouse 查询执行和 PostgreSQL 运行状态的权威实现；前端不重写规则编译逻辑，不直接访问 ClickHouse 或 PostgreSQL。

第一版采用单独 package 管理：只在 `app/racingline/` 维护 `package.json`、lockfile、Vite 配置和 npm scripts；暂不在 `app/` 顶层引入 npm/pnpm/yarn workspace 管理器。只有当出现第二个前端应用或共享前端 package 时，再评估 `app/` 顶层 workspace。

第一版 UI 以紧凑工作台为主：表格、筛选器、状态标记、详情抽屉和少量趋势图优先，不做大幅 hero、装饰卡片或与选股无关的展示页。

## 技术选型

Racingline 第一版前端技术栈和工程边界由 [ADR 0011](../ADR/0011-racingline-frontend-technology-stack.md) 固化。本文档只描述第一版页面、接口和交互范围；后续如果技术栈、包管理、环境变量、shadcn/ui 默认组件维护规则或组件体系变化，应先更新 ADR。

## 环境配置

第一版使用仓库根目录 `.env` 和 `.env.example` 作为唯一前端运行时配置入口。`app/racingline/` 不创建 `.env`、`.env.local`、`.env.example` 或其他 `.env*` 文件；Vite 通过 `envDir` 从仓库根目录读取配置，并保持默认暴露规则：只有 `VITE_` 前缀变量可以在客户端代码中通过 `import.meta.env` 读取。

第一版至少定义：

```text
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34057
```

前端代码只读取 `import.meta.env.VITE_REARVIEW_API_BASE_URL` 作为 Rearview API base URL。不要把非公开密钥、数据库连接串或服务端 token 放入 `VITE_` 变量，也不要在项目代码子路径另行创建环境变量入口。

## 第一版页面范围

第一版只实现内部选股工作流所需页面，不做登录页、个人设置页、营销首页、完整回测页、调度配置页或交易执行页。应用根路径直接进入运行看板。

### 路由与信息架构

| Route | 页面 | 第一版职责 | 主要操作 |
|---|---|---|---|
| `/` | 根入口 | 重定向到 `/runs` | 无 |
| `/runs` | 运行看板 | 查看最近选股运行，筛选运行状态，进入运行详情，发起新运行入口 | 刷新、筛选、打开运行、创建运行 |
| `/runs/:runId` | 运行详情与结果页 | 查看 run summary、chunk 进度、day 进度、某日股票池和 TopN 买入信号 | 切换交易日、切换结果 tab、打开信号详情抽屉 |
| `/rules` | 规则工作台 | 查看规则集和版本，创建规则草稿，执行 explain，发布不可变规则版本 | 选择规则集、新建规则、校验草稿、发布版本、发起运行 |
| `/metrics` | 指标目录 | 浏览 Rearview 可用 metric 和规则引擎策略 | 搜索、按 mart/类型/能力过滤 |

### 运行看板 `/runs`

目标：让用户从第一屏看到最近 Rearview 运行状态，并能快速进入正在运行或最近完成的结果。

第一版内容：

1. 顶部状态摘要：运行中、成功、失败、最近完成时间。摘要来自 `GET /rearview/runs` 返回的列表聚合，不单独新增统计接口。
2. 运行筛选器：状态、规则集、日期区间、关键词。关键词第一版只匹配 `run_id`、`rule_version_id` 和规则集名称。
3. 运行表格字段：`run_id`、规则集名称、`rule_version_id`、`rule_hash` 短 hash、`start_date`、`end_date`、`top_n`、`status`、`summary.pool_count`、`summary.signal_count`、错误摘要。
4. 行操作：打开详情、复制 `run_id`、失败运行查看错误。
5. 主操作：进入 `/rules` 创建规则或发起运行。

状态处理：

- run 的 `created`、`validating`、`compiling`、`running_clickhouse`、`writing_pool`、`writing_signals` 显示为进行中，并启用定时刷新。
- chunk 的 `created`、`running` 显示为进行中；day 的 `created` 显示为等待结果。
- `succeeded` 显示为完成，可进入结果页。
- `failed_validation`、`failed_compile`、`failed_clickhouse`、`failed_write`、`cancelled` 显示为终态错误，并展示 `error_type` / `error_message`。

### 运行详情与结果页 `/runs/:runId`

目标：把一次区间运行的执行状态和逐日结果放在同一个工作面板里，避免用户在多个页面之间跳转才能理解结果。

页面结构：

1. 运行头部：`run_id`、`rule_version_id`、`rule_hash`、日期区间、`top_n`、`status`、`compiled_sql_hash`、错误摘要。
2. Chunk 进度条：展示 `chunk_no`、日期范围、状态、`clickhouse_query_id`、耗时和错误。
3. 交易日列表：展示 `trade_date`、状态、`pool_count`、`signal_count`，用于选择当前查看日期。
4. 结果 tabs：`买入信号`、`股票池`、`运行日历`、`Chunks`。
5. 信号详情抽屉：点击买入信号行打开，展示 `rank`、`score`、`score_breakdown`、`selected_metrics` 和运行时快照说明。

买入信号表字段：

| 字段 | 来源 | 展示要求 |
|---|---|---|
| `rank` | `buy_signal.rank` | 固定宽度，按升序 |
| `security_code` | `buy_signal.security_code` | 可复制 |
| `score` | `buy_signal.score` | 保留合理小数位 |
| `selected_metrics` | `buy_signal.selected_metrics` | 动态列，来自规则版本 `output_metrics` |
| `score_breakdown` | `buy_signal.score_breakdown` | 默认折叠，详情抽屉展开 |

股票池表字段：

| 字段 | 来源 | 展示要求 |
|---|---|---|
| `security_code` | `pool_member.security_code` | 可复制 |
| `score` | `pool_member.score` | 无信号时可为空 |
| `signal_rank` | `pool_member.signal_rank` | 入选 TopN 时显示 |
| `selected_metrics` | `pool_member.selected_metrics` | 动态列 |
| `filter_snapshot` | `pool_member.filter_snapshot` | 详情区域展示，不默认展开 |

结果页必须优先展示 PostgreSQL 中保存的运行时快照。任何后续从 ClickHouse 当前 mart 补查的字段，都必须标记为“当前 mart 查询值”，不能混入 `selected_metrics`。

### 规则工作台 `/rules`

目标：让用户通过表单创建可解释、可校验、可版本化的选股规则，并能直接用该规则发起区间运行。

第一版内容：

1. 规则集列表：展示规则集名称、状态、标签、`current_version_id`，支持选择已有规则集。
2. 规则版本列表：展示 `version_no`、`rule_version_id`、`status`、`top_n_default`、`rule_hash`。
3. 规则草稿表单：
   - Universe：`base`、排除 ST、排除停牌、包含/排除证券代码。
   - Pool filters：用 metric、operator、operand 表单表达 `all` / `any` / `not` / `compare`。
   - Scoring：支持 `conditional_points` 和 `weighted_metric`。
   - Score clamp：`min`、`max`。
   - Output metrics：从指标目录选择。
   - TopN default。
4. Explain 面板：显示校验结果、依赖 metrics、required marts、输出列、chunk plan 和编译摘要。
5. 发布动作：创建规则集或选择现有规则集后，发布不可变版本。
6. 发起运行动作：选择规则版本、日期区间和 `top_n` 后创建 run。

规则表单不要求用户手写完整 JSON。第一版可以提供只读 JSON 预览，用于排查和复制问题；所有提交给后端的内容必须仍是结构化 `RuleVersionSpec`。

### 指标目录 `/metrics`

目标：让用户知道哪些指标可以用于过滤、评分和输出，避免在规则编辑器中凭记忆输入 metric 名称。

第一版内容：

1. 指标表格字段：`logical_metric`、`mart_database`、`mart_table`、`column_name`、`value_kind`、`allow_filter`、`allow_scoring`、`allowed_ops`、`null_policy`、`default_output`、`description`。
2. 筛选器：mart table、value kind、可过滤、可评分、默认输出。
3. 搜索：匹配 `logical_metric`、`column_name`、`description`。
4. 行操作：复制 metric 名称，加入规则草稿的 output metrics。

指标目录只展示 Rearview metric catalog 当前 allowlist，不重新定义字段语义。

## 后端接口对接矩阵

第一版前端以 `VITE_REARVIEW_API_BASE_URL` 为 base URL。接口分为“当前后端已存在”和“第一版前端闭环需补齐”两类。

| Method | Path | 当前状态 | 页面/流程 | 第一版前端用途 |
|---|---|---|---|---|
| `GET` | `/healthz` | 已存在 | 应用启动 | 检查 Rearview 服务是否可达，失败时显示全局错误横幅 |
| `POST` | `/rearview/explain` | 已存在 | 规则工作台 | 校验规则草稿，返回编译摘要和可选 chunk plan |
| `POST` | `/rearview/rule-sets` | 已存在 | 规则工作台 | 创建规则集 |
| `POST` | `/rearview/rule-sets/{rule_set_id}/versions` | 已存在 | 规则工作台 | 发布不可变规则版本 |
| `POST` | `/rearview/runs` | 已存在 | 规则工作台、运行看板 | 发起异步区间运行，返回 `run_id` |
| `GET` | `/rearview/runs/{run_id}` | 已存在 | 运行详情 | 查询 run summary、状态、错误和 `compiled_sql_hash` |
| `GET` | `/rearview/runs/{run_id}/chunks` | 已存在 | 运行详情 | 查询 chunk 进度、ClickHouse query id、耗时和错误 |
| `GET` | `/rearview/runs/{run_id}/days` | 已存在 | 运行详情 | 查询每日状态、股票池数量和信号数量 |
| `GET` | `/rearview/runs/{run_id}/pool?trade_date=...` | 已存在，需增强 | 运行详情 | 查询某日股票池；第一版 UI 需要分页、排序和证券代码过滤 |
| `GET` | `/rearview/runs/{run_id}/signals?trade_date=...` | 已存在，需增强 | 运行详情 | 查询某日 TopN 买入信号；第一版 UI 需要分页、排序和证券代码过滤 |
| `GET` | `/rearview/runs` | 需补齐 | 运行看板 | 查询运行列表，支持状态、规则集、日期区间和分页 |
| `GET` | `/rearview/rule-sets` | 需补齐 | 规则工作台 | 查询规则集列表，返回 `current_version_id`、标签和状态 |
| `GET` | `/rearview/rule-sets/{rule_set_id}/versions` | 需补齐 | 规则工作台 | 查询规则版本列表，支持选择历史版本运行 |
| `GET` | `/rearview/metrics` | 需补齐 | 指标目录、规则工作台 | 查询 metric catalog allowlist |

接口增强要求：

1. 列表接口使用稳定分页参数，例如 `limit`、`offset` 或 cursor；响应中必须能判断是否还有下一页。
2. pool/signals 查询至少支持 `trade_date`、分页、`security_code` 过滤和稳定排序。
3. 错误响应统一包含 `error_type`、`message` 和可选 `field_path`，让规则表单能定位字段级错误。
4. CORS 允许本地 Vite dev server 调用 Rearview API；具体 origin 可在开发环境放宽，生产环境另行约束。

### 接口数据约定

第一版前端按以下数据契约建模。已存在接口以当前 Rust record 为准；需补齐接口应复用相同字段名，避免前端维护第二套视图模型。

| 接口 | 请求数据 | 响应数据 |
|---|---|---|
| `GET /healthz` | 无 | `{ "status": "ok" }` |
| `POST /rearview/rule-sets` | `name`、可选 `description`、`owner`、`tags` | `RuleSetRecord`：`rule_set_id`、`name`、`description`、`owner`、`status`、`tags`、`current_version_id` |
| `POST /rearview/rule-sets/{rule_set_id}/versions` | `rule: RuleVersionSpec`、可选 `activate`、`created_by` | `RuleVersionRecord`：`rule_version_id`、`rule_set_id`、`version_no`、`status`、`top_n_default`、`rule_hash` |
| `POST /rearview/explain` | `RuleVersionSpec`，或 `{ rule, start_date, end_date, top_n }` | `sql`、`sql_hash`、`required_metrics`、`required_marts`、`required_columns`，带日期范围时额外返回 `chunk_plan` |
| `POST /rearview/runs` | `rule_set_id` 和 `rule_version_id` 二选一、`start_date`、`end_date`、可选 `top_n`、`universe_snapshot` | `RunRecord`：`run_id`、`rule_version_id`、`rule_hash`、`start_date`、`end_date`、`top_n`、`status`、`compiled_sql_hash`、`summary`、`error_type`、`error_message` |
| `GET /rearview/runs/{run_id}` | path `run_id` | `RunRecord` |
| `GET /rearview/runs/{run_id}/chunks` | path `run_id` | `RunChunkRecord[]`：`run_id`、`chunk_no`、`start_date`、`end_date`、`status`、`clickhouse_query_id`、`elapsed_ms`、`error_type`、`error_message` |
| `GET /rearview/runs/{run_id}/days` | path `run_id` | `RunDayRecord[]`：`run_id`、`trade_date`、`status`、`universe_count`、`pool_count`、`signal_count`、`error_type`、`error_message` |
| `GET /rearview/runs/{run_id}/pool` | path `run_id`、query `trade_date`；需增强 `limit`、`offset`、`security_code`、`sort` | `PoolMemberRecord[]`：`run_id`、`trade_date`、`security_code`、`score`、`signal_rank`、`selected_metrics`、`filter_snapshot` |
| `GET /rearview/runs/{run_id}/signals` | path `run_id`、query `trade_date`；需增强 `limit`、`offset`、`security_code`、`sort` | `BuySignalRecord[]`：`run_id`、`trade_date`、`security_code`、`rank`、`score`、`score_breakdown`、`selected_metrics` |
| `GET /rearview/runs` | query `status`、`rule_set_id`、`start_date`、`end_date`、`limit`、`offset` | 分页 `RunRecord[]`，并附带规则集名称或可由前端用规则集列表补齐 |
| `GET /rearview/rule-sets` | query `status`、`keyword`、`limit`、`offset` | 分页 `RuleSetRecord[]` |
| `GET /rearview/rule-sets/{rule_set_id}/versions` | path `rule_set_id`、query `status`、`limit`、`offset` | 分页 `RuleVersionRecord[]` |
| `GET /rearview/metrics` | query `mart_table`、`value_kind`、`allow_filter`、`allow_scoring`、`keyword` | `MetricDefinition[]`：`logical_metric`、`mart_database`、`mart_table`、`column_name`、`value_kind`、`allow_filter`、`allow_scoring`、`allowed_ops`、`null_policy`、`default_output`、`description` |

`RuleVersionSpec` 的第一版 UI 字段必须覆盖 `universe`、`pool_filters`、`scoring`、`top_n_default` 和 `output_metrics`。`FilterExpr` 至少支持 `all`、`any`、`not`、`compare`；`Operand` 至少支持 metric、number、bool、string、range；`ScoringRule` 至少支持 `conditional_points` 和 `weighted_metric`。

## 端到端交互流程

### 1. 应用启动

1. 前端读取 `import.meta.env.VITE_REARVIEW_API_BASE_URL`。
2. 调用 `GET /healthz`。
3. 健康检查失败时，保留页面壳和本地路由，但所有需要后端的操作显示错误状态。
4. 健康检查成功后，进入 `/runs` 并加载运行列表。

### 2. 创建规则草稿并 explain

1. 用户进入 `/rules`。
2. 前端加载 `GET /rearview/metrics`，用于填充 metric 选择器、operator 选择器和 output metrics。
3. 用户创建或选择规则集。
4. 用户通过表单配置 universe、pool filters、scoring、clamp、output metrics 和 `top_n_default`。
5. 用户点击校验，前端调用 `POST /rearview/explain`。
6. explain 成功时，页面展示依赖 metric、required marts、输出列、chunk plan 和编译摘要。
7. explain 失败时，页面展示 `error_type`、用户可读 message 和字段级错误位置；不允许发布版本。

### 3. 发布规则版本

1. 如果是新规则集，先调用 `POST /rearview/rule-sets`。
2. 调用 `POST /rearview/rule-sets/{rule_set_id}/versions` 发布不可变版本。
3. 发布成功后，页面显示 `rule_version_id`、`version_no`、`rule_hash` 和 `top_n_default`。
4. 发布后的版本不能在 UI 中原地编辑；继续修改必须创建新草稿并发布新版本。

### 4. 发起区间运行

1. 用户选择 `rule_set_id` 或 `rule_version_id`、`start_date`、`end_date` 和可选 `top_n`。
2. 前端调用 `POST /rearview/runs`。
3. 后端返回 `202 Accepted` 和 run record 后，前端跳转到 `/runs/:runId`。
4. 运行详情页立即开始轮询 `GET /rearview/runs/{run_id}`、`/chunks` 和 `/days`。

### 5. 运行进度轮询

1. 当 run status 为 `created`、`validating`、`compiling`、`running_clickhouse`、`writing_pool` 或 `writing_signals` 时，前端每 2 到 5 秒刷新 run、chunks 和 days。
2. 当 run status 进入 `succeeded`、`failed_validation`、`failed_compile`、`failed_clickhouse`、`failed_write` 或 `cancelled` 时停止自动轮询。
3. chunk 或 day 存在错误时，在对应行展示 `error_type` 和 `error_message`，并在运行头部汇总。
4. 用户可以手动刷新终态运行，但 UI 不应重新发起 run。

### 6. 查看每日结果

1. 运行详情页从 `/days` 选择默认交易日：优先选择最新有信号的成功日；没有信号时选择最新成功日。
2. 切换交易日后，同时刷新该日 `signals` 和 `pool`。
3. `买入信号` tab 调用 `GET /rearview/runs/{run_id}/signals?trade_date=...`。
4. `股票池` tab 调用 `GET /rearview/runs/{run_id}/pool?trade_date=...`。
5. 如果该日 `signal_count = 0` 或 `pool_count = 0`，展示空状态，但保留交易日和运行摘要。

### 7. 信号解释

1. 用户点击买入信号行。
2. 前端打开详情抽屉，使用当前行的 `score_breakdown`、`selected_metrics`、`rank`、`score` 和 `security_code`。
3. 详情抽屉按 scoring rule 展示 matched、points、raw values 和最终 score。
4. 详情抽屉必须标注这些字段来自运行时 PostgreSQL 快照。

### 8. 错误与恢复

1. 网络错误：TanStack Query 保留上一次成功数据，页面显示轻量错误状态和重试按钮。
2. 校验错误：规则表单定位到具体条件或 metric；不清空用户草稿。
3. 后端运行错误：运行详情展示终态错误，不自动重试 run。
4. 空结果：区分“接口还在加载”“该交易日无结果”“运行失败无结果”三种状态。

## 交互要求

规则编辑器不应要求用户直接手写完整 JSON。第一版可以提供结构化表单，并保留只读 JSON 预览用于排查；提交前必须调用 `POST /rearview/explain`，展示规则校验结果、依赖 mart、输出列、日期 chunk plan 和编译摘要。规则版本创建后不可变，UI 必须明确区分“编辑草稿”和“发布新版本”。

选股结果页应把分数解释作为一等能力，而不是只展示证券代码和总分。`buy_signal.score_breakdown.raw_values` 和 `selected_metrics` 是运行时快照，UI 展示历史结果时优先使用这些字段，避免回查 ClickHouse 当前 mart 值造成解释漂移。

如果页面展示非运行时快照字段，例如从当前 ClickHouse mart 查询得到的补充指标，必须在 UI 上标记为当前 mart 查询值，不能与 run 结果快照混淆。

## 后端接口补齐

为支持前端闭环，Rearview 后端需要补齐以下 UI 友好接口和协议细节：

1. `GET /rearview/runs`：运行列表，支持按状态、规则集、日期区间分页查询。
2. `GET /rearview/rule-sets`：规则集列表，支持查看 current version 和标签。
3. `GET /rearview/rule-sets/{rule_set_id}/versions`：规则版本列表，支持选择历史版本运行。
4. `GET /rearview/metrics`：指标目录，来自当前 metric catalog，不重新定义字段语义。
5. pool 和 signals 查询支持分页、排序、证券代码过滤，避免大结果集一次性返回。
6. 统一错误响应结构，至少包含错误类型、用户可读消息和可选字段级校验路径。
7. 明确 CORS，并支持通过 `VITE_REARVIEW_API_BASE_URL` 配置 API base URL，保证本地前端可以直接对接开发环境 Rearview 服务。

## 验收标准

1. 用户可以创建或选择规则集，使用表单配置一版规则，并通过 explain 看到校验和编译摘要。
2. 用户可以发起指定日期区间的异步选股运行，并在运行看板看到 run、chunk 和 day 粒度进度。
3. 用户可以按交易日查看完整股票池和 TopN 买入信号。
4. 用户可以打开某个买入信号，理解其排名、总分、评分拆解和关键指标快照。
5. 页面展示的历史信号解释只依赖 PostgreSQL 中保存的运行结果快照；如果展示非快照字段，必须标记为当前 mart 查询值。
6. `app/racingline/` 的前端工程使用本文档定义的技术栈，并提供可重复执行的 lint、typecheck 和 build 命令。

## 已决事项

1. 第一版采用 `app/racingline/` 单独 package 管理，不引入 `app/` 顶层 workspace 管理器。
2. 前端开发环境 API base URL 使用仓库根目录 `.env` 或 `.env.example` 约定，变量名为 `VITE_REARVIEW_API_BASE_URL`。
3. 第一版不引入登录入口、认证/鉴权、用户隔离或权限系统。
