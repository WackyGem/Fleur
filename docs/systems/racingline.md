# System: Racingline

状态：组合净值第一版实施中（2026-06-16）

## 代码根

`racingline` 位于 `app/racingline/`，作为 `app/` 目录下的独立前端工作区。页面需求以 [RFC 0019](../RFC/0019-racingline-rearview-frontend-workbench.md) 为准；工程边界以 [ADR 0011](../ADR/0011-racingline-frontend-technology-stack.md) 为准。

## 职责

1. 提供 Rearview 指标选股前端工作台。
2. 支持规则集选择、规则版本表单化编辑、explain 校验和运行发起。
3. 展示 run、chunk 和 day 粒度进度。
4. 按交易日展示股票池、TopN 买入信号、score breakdown 和 selected metrics。
5. 用 UI 明确区分运行时结果快照和当前 mart 查询值。
6. 从 run result 的 `Open` 进入 `/runs/:runId/securities/:securityCode` 个股分析页，提供结果列表、日 K 线、MA5/MA10/MA30、KDJ/RSI/MACD/BOLL 和右侧 mart 指标面板。
7. 提供虚拟账户模板表单，使用 Rearview 默认市场费率模板预填初始资金、费率、滑点和卖出规则。
8. 提供 `/portfolios` 和 `/portfolios/:portfolioRunId`，展示组合运行状态、净值曲线、summary、参数、持仓、成交、订单、调仓目标和事件。

## 非职责

1. 不实现 Rearview 规则编译、ClickHouse 查询、PostgreSQL 写入或业务状态机。
2. 不直接访问 ClickHouse 或 PostgreSQL。
3. 不在浏览器内计算权威成交、持仓、费用、滑点或净值；组合账本以 Rearview PostgreSQL API 为准。
4. 第一版不引入登录入口、认证/鉴权、用户隔离或权限系统。

## 技术栈

技术栈和工程边界以 [ADR 0011](../ADR/0011-racingline-frontend-technology-stack.md) 为权威来源。当前摘要：Vite + React + TypeScript，Tailwind CSS v4 + CSS Variables，shadcn/ui（`base-nova`）+ Base UI，Hugeicons，React Router，TanStack Query，Zustand 和 TradingView Lightweight Charts。

## 工程管理

第一版采用单独 package 管理：只在 `app/racingline/` 维护 `package.json`、lockfile、Vite 配置和 npm scripts；暂不在 `app/` 顶层引入 npm/pnpm/yarn workspace 管理器。

前端运行时配置只使用仓库根目录 `.env` 和 `.env.example`。`app/racingline/` 不创建 `.env`、`.env.local`、`.env.example` 或其他 `.env*` 文件；Vite 通过 `envDir` 从仓库根目录读取配置。Vite 客户端变量必须使用 `VITE_` 前缀，第一版 API base URL 变量为：

```text
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34057
```

本地开发入口：

```bash
make racingline-dev
```

该命令会先按端口清理已启动的 Rearview 和 Racingline dev 进程，再启动 Docker dev 依赖服务、等待 PostgreSQL/ClickHouse、执行 PostgreSQL migrations、同步 Rearview metric catalog，最后同时启动 Rearview server `http://127.0.0.1:34057`、Rearview portfolio worker 与前端 `http://127.0.0.1:5173/`。

单独启动或清理：

```bash
make rearview-dev
make racingline-frontend-dev
make racingline-dev-stop
```

`make racingline-dev-stop` 只清理前后端 dev server 端口，不停止 Docker 依赖服务；停止依赖服务仍使用 `make dev-down`。

## 后端依赖

| 系统 | 依赖 |
|---|---|
| [Rearview](rearview.md) | 规则集、规则版本、运行、股票池、买入信号、explain、个股 analysis、虚拟账户模板和组合运行 API |
| Furnace/dbt marts | 通过 Rearview 间接消费 mart 指标，不由前端直接访问 |

## 浏览器调试

Racingline 前端调试优先复用 Docker `vnc-mini-desktop` 中的 Chromium 浏览器。该浏览器通过 CDP 暴露到默认端点：

```text
http://127.0.0.1:9222
```

全局安装 `@playwright/cli` 后，用以下命令检查和连接：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

Racingline 调试默认连接 `vnc-mini-desktop` 已运行的 Chromium。不要使用 `playwright-cli open` 启动本机 Chrome；本机环境可能没有安装系统 Chrome，且会绕过共享 VNC/CDP 调试环境。

具体 agent 调试流程见 [../skills/playwright-cdp-frontend-debug/SKILL.md](../skills/playwright-cdp-frontend-debug/SKILL.md)。官方 Playwright CLI skill 可通过 `playwright-cli install --skills agents` 安装到本地 `.agents/skills/playwright-cli`。

## 质量门禁

`app/racingline/` 提供可重复执行的 lint、typecheck、test 和 build 命令：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run test
npm run build
```

涉及 Rearview 后端 API 变更时追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [../RFC/0019-racingline-rearview-frontend-workbench.md](../RFC/0019-racingline-rearview-frontend-workbench.md) | Racingline 前端 RFC |
| [../RFC/0020-racingline-run-result-security-analysis-page.md](../RFC/0020-racingline-run-result-security-analysis-page.md) | Run result 个股分析页已实现 RFC |
| [../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md](../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md) | 虚拟账户、交易费率、止盈止损和组合调仓净值 Proposed RFC |
| [../RFC/0023-racingline-frontend-prototype-led-development.md](../RFC/0023-racingline-frontend-prototype-led-development.md) | Racingline 前端 RFC 阶段允许原型驱动多轮 UX 验证的流程 RFC |
| [../RFC/0024-racingline-strategy-selection-step1.md](../RFC/0024-racingline-strategy-selection-step1.md) | 从 `/strategies` Step 1 策略选股切入，接通 metric catalog、RuleVersionSpec 和 explain 的 Proposed RFC |
| [../RFC/0025-racingline-strategy-weight-configuration-step2.md](../RFC/0025-racingline-strategy-weight-configuration-step2.md) | 从 `/strategies` Step 2 权重配置切入，把权重草稿落到 `RuleVersionSpec.scoring.rules`，并定义点击股池预览时才执行选股、评分和排名的 Implemented RFC |
| [../plans/archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md](../plans/archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md) | 策略选股 Step 1 缺口填补实施计划归档 |
| [../plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md](../plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md) | 策略权重配置 Step 2、preview-only API 和真实股池预览实施计划归档 |
| [../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md](../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md) | 策略权重配置 Step 2 到股池预览闭环验收报告 |
| [../Q&A/user-logic.md](../Q&A/user-logic.md) | Racingline 当前用户画像和策略研究工作台主路径 |
| [../Q&A/0003-racingline-strategy-lab-two-entry-navigation.md](../Q&A/0003-racingline-strategy-lab-two-entry-navigation.md) | Racingline 策略研究工作台两入口导航和首屏承载 Proposed Q&A |
| [../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md](../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md) | `app/racingline_new/` 看板到选股、回测和运行策略闭环用户故事 |
| [../ADR/0013-racingline-ui-stack-variant-evaluation.md](../ADR/0013-racingline-ui-stack-variant-evaluation.md) | Racingline UI 栈变体评估 Proposed ADR |
| [../plans/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md](../plans/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md) | 虚拟账户、组合运行、NATS worker 和组合页面当前实施计划 |
| [../ADR/0011-racingline-frontend-technology-stack.md](../ADR/0011-racingline-frontend-technology-stack.md) | Racingline 前端技术栈和工程边界 |
| [../plans/archive/0037-racingline-frontend-implementation-plan.md](../plans/archive/0037-racingline-frontend-implementation-plan.md) | Racingline 前端第一版实施计划归档 |
| [../plans/archive/0039-racingline-run-result-security-analysis-page-implementation-plan.md](../plans/archive/0039-racingline-run-result-security-analysis-page-implementation-plan.md) | Run result 个股分析页实施计划归档 |
| [../plans/archive/0040-racingline-security-analysis-optimization-plan.md](../plans/archive/0040-racingline-security-analysis-optimization-plan.md) | 个股分析页交互、趋势叠加线和新选股规则适配计划归档 |
| [../jobs/reports/2026-06-13-racingline-frontend-skeleton.md](../jobs/reports/2026-06-13-racingline-frontend-skeleton.md) | 前端骨架和工程门禁报告 |
| [../jobs/reports/2026-06-13-racingline-rearview-api-integration.md](../jobs/reports/2026-06-13-racingline-rearview-api-integration.md) | Rearview API 联调报告 |
| [../jobs/reports/2026-06-13-racingline-playwright-cdp-acceptance.md](../jobs/reports/2026-06-13-racingline-playwright-cdp-acceptance.md) | Playwright CDP 验收报告 |
| [../jobs/reports/2026-06-15-racingline-security-analysis-page.md](../jobs/reports/2026-06-15-racingline-security-analysis-page.md) | 个股分析页 API、桌面/移动和交互验收报告 |
| [../jobs/reports/2026-06-15-racingline-security-analysis-optimization.md](../jobs/reports/2026-06-15-racingline-security-analysis-optimization.md) | 个股分析页优化、规则适配和评分 clamp 验收报告 |
| [../jobs/reports/2026-06-16-racingline-portfolio-nav.md](../jobs/reports/2026-06-16-racingline-portfolio-nav.md) | 组合净值、明细 API、列表页和详情页 smoke 验收报告 |
| [../jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md](../jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md) | 策略选股 Step 1 metric catalog、RuleVersionSpec、crossing explain 和浏览器验收报告 |
| [../RFC/0018-rust-stock-screening-service.md](../RFC/0018-rust-stock-screening-service.md) | Rearview 后端服务 RFC |
| [rearview.md](rearview.md) | Rearview 当前系统地图 |

## 已决事项

1. `app/racingline/` 第一版按单独 package 管理。
2. API base URL 使用仓库根目录 `.env` 或 `.env.example` 中的 `VITE_REARVIEW_API_BASE_URL`。
3. 不得改写 shadcn/ui 官方 CLI 生成的默认 UI 组件文件；业务组件必须在独立业务目录中组合引用这些默认组件。
4. 第一版不引入登录入口、认证/鉴权、用户隔离或权限系统。
5. `app/racingline_new/` 如作为并行原型或重构工程，正式实现阶段必须继承 ADR 0011 的 Racingline 技术栈；任何 shadcn style、primitive base、icon library、状态管理或图表库变化都需要另起 ADR 或更新 ADR 0011。
