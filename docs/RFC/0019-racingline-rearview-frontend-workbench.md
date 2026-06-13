# RFC 0019: Racingline Rearview 前端工作台

状态：草案 / 构想（2026-06-13）

## 摘要

本文档定义 `racingline` 前端工作台的第一版需求。`racingline` 放在 `app/racingline/`，作为 `app/` 目录下的独立前端工作区，对接 RFC 0018 中定义的 Rearview HTTP API。

第一版定位为内部指标选股工作台，用于把规则创建、运行发起、进度追踪和结果解释串成可操作闭环。它不是营销页面、交易系统或完整回测平台；第一屏应直接进入选股工作流。

关联后端设计：

- `docs/RFC/0018-rust-stock-screening-service.md`
- `engines/crates/rearview/src/api/mod.rs`

## 目标

1. 提供一个可操作的 Rearview 选股工作台，而不是只展示 API 返回值。
2. 支持创建或选择规则集，表单化配置规则版本，并在发布前执行 explain。
3. 支持按日期区间发起异步选股运行，并追踪 run、chunk 和 day 粒度进度。
4. 支持按交易日查看完整股票池和 TopN 买入信号。
5. 支持打开单个买入信号，查看排名、总分、评分拆解和关键指标快照。
6. 明确前端工程栈和 `app/racingline/` 工作区边界，为后续实现和质量门禁留出稳定约束。

## 非目标

1. 第一版不实现登录、用户隔离或权限系统；这些能力后续应结合 Rearview 后端鉴权设计统一处理。
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

第一版 UI 以紧凑工作台为主：表格、筛选器、状态标记、详情抽屉和少量趋势图优先，不做大幅 hero、装饰卡片或与选股无关的展示页。

## 技术选型

| 类别 | 选型 |
|---|---|
| 构建工具 | Vite |
| 前端框架 | React |
| 开发语言 | TypeScript |
| CSS 风格与样式体系 | Tailwind CSS v4（`@tailwindcss/vite`）+ CSS Variables（设计令牌） |
| UI 组件体系 | shadcn/ui（style: `base-nova`）+ Base UI（`@base-ui/react`） |
| 图标 | Hugeicons（`@hugeicons/react`） |
| 类名与变体工具 | `clsx` + `tailwind-merge` + `class-variance-authority` |
| 代码规范 | ESLint（Flat Config）+ `typescript-eslint` + `react-hooks` + `react-refresh` |
| 路由 | React Router（`react-router-dom`） |
| 服务端状态管理 | TanStack Query（`@tanstack/react-query`） |
| 客户端状态管理 | Zustand（`zustand`） |
| 图表方案 | TradingView Lightweight Charts（`lightweight-charts`） |

## 核心视图

| 视图 | 目标 | 主要后端接口 |
|---|---|---|
| 运行看板 | 展示最近选股运行、状态、规则版本、日期区间、TopN、耗时、错误摘要和 chunk 进度 | `GET /rearview/runs`、`GET /rearview/runs/{run_id}`、`GET /rearview/runs/{run_id}/chunks`、`GET /rearview/runs/{run_id}/days` |
| 选股结果页 | 按交易日查看股票池和 TopN 买入信号，支持日期切换、分页、排序和证券代码过滤 | `GET /rearview/runs/{run_id}/pool?trade_date=...`、`GET /rearview/runs/{run_id}/signals?trade_date=...` |
| 信号详情抽屉 | 展示单只证券当日排名、score、`score_breakdown`、`selected_metrics` 和评分条件原始值 | `GET /rearview/runs/{run_id}/signals?trade_date=...` |
| 规则编辑器 | 用表单化方式配置 universe、过滤条件、评分规则、输出指标和 TopN；保存前先 explain | `POST /rearview/explain`、`POST /rearview/rule-sets`、`POST /rearview/rule-sets/{rule_set_id}/versions`、`POST /rearview/runs` |
| 指标目录页 | 展示可用 metric、来源 mart、值类型、是否可过滤/评分、允许操作符和 NULL 语义 | `GET /rearview/metrics` |

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
7. 明确 CORS、API base URL 和部署环境变量，保证本地前端可以直接对接开发环境 Rearview 服务。

## 验收标准

1. 用户可以创建或选择规则集，使用表单配置一版规则，并通过 explain 看到校验和编译摘要。
2. 用户可以发起指定日期区间的异步选股运行，并在运行看板看到 run、chunk 和 day 粒度进度。
3. 用户可以按交易日查看完整股票池和 TopN 买入信号。
4. 用户可以打开某个买入信号，理解其排名、总分、评分拆解和关键指标快照。
5. 页面展示的历史信号解释只依赖 PostgreSQL 中保存的运行结果快照；如果展示非快照字段，必须标记为当前 mart 查询值。
6. `app/racingline/` 的前端工程使用本文档定义的技术栈，并提供可重复执行的 lint、typecheck 和 build 命令。

## 待决问题

1. `app/` 是否需要顶层 workspace 管理器，还是 `app/racingline/` 先作为单独 package 管理。
2. 前端开发环境 API base URL 使用 `.env` 约定还是 repo 级配置约定。
3. 是否在第一版就引入登录入口，还是等待 Rearview 后端鉴权设计后统一处理。
