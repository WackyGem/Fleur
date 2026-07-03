# Architecture: Racingline

状态：单一正式前端工作台已落到 `app/racingline/`；Step 4 到 Step 5 回测 handoff 已改为 create accepted 后进入状态页（2026-06-25）；Step 5 succeeded 首屏已切到 Rearview overview compact endpoint，HTTP live 计时和前端质量门禁已通过（2026-06-26）；Step 5「建立组合」已接入 T+1 publish preview、pending 首次运行展示和 live/backtest 语义分离（2026-06-27）；组合详情页已展示 live 虚拟资金账户和对账单 read model（2026-06-29）；建立组合发布预检已改为 15:00 前/后交易阶段感知规则（2026-07-02）；策略详情页删除按钮已接入 Rearview archive API，归档成功后返回 Dashboard，archived detail 410 后同样返回 Dashboard（2026-07-02）；策略详情页页头已新增左侧配置图标 + `策略配置方案` 的普通字重无边框按钮，并已从 `rule_snapshot` + `execution_config` 派生配置 Dialog，展示模板、period/benchmark/preview 文案由前端统一生成，不再向 `ui_display_snapshot` 写入或读取策略配置展示字段（2026-07-02）

## 代码根

| 路径 | 角色 |
|---|---|
| [app/racingline/](../../app/racingline/) | Racingline 唯一正式前端工作区，承载 dashboard、`/strategies`、strategy backtest 和 strategy portfolio 页面 |

页面历史设计可追溯 [RFC 0023](../RFC/archive/0023-racingline-frontend-prototype-led-development.md)、[RFC 0024](../RFC/archive/0024-racingline-strategy-selection-step1.md)、[RFC 0025](../RFC/archive/0025-racingline-strategy-weight-configuration-step2.md)、[RFC 0026](../RFC/archive/0026-racingline-strategy-pool-preview-step3.md)、[RFC 0027](../RFC/archive/0027-racingline-strategy-simulation-position-step4.md)、[RFC 0028](../RFC/archive/0028-racingline-strategy-backtest-step5.md) 和 [RFC 0029](../RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md)；工程边界以 [ADR 0011](../ADR/0011-racingline-frontend-technology-stack.md) 和 [ADR 0013](../ADR/0013-racingline-ui-stack-variant-evaluation.md) 为准。

## 职责

1. 提供 Rearview 策略研究前端工作台。
2. 支持 `/dashboard` 的 strategy portfolio 看板、`/dashboard/strategies/:portfolioId` 详情和 `/strategies` 策略创建主流程。
3. 在 `/strategies` 中支持 Step 1 筛选条件、Step 2 评分规则、Step 3 preview、Step 4 模拟建仓和 Step 5 异步回测。
4. 通过 Rearview preview-only API 展示 applied preview snapshot、动态近一年交易日股池、分页候选股、rank、score、Step 2 得分项、Step 1 指标列、证券交易板块、K 线复权、MA5/MA10/MA30 和成交量柱。
5. 通过 Rearview strategy backtest API 展示 validate、options、create、status、overview、nav、rebalance records、targets、orders、trades、positions、events、performance、closed trades 和 trade metrics；`/strategies` Step 4 创建 backtest run 成功后立即进入 Step 5，Step 5 内部按 status view 轮询并在 succeeded 后优先读取 `overview?view=ui` 作为首屏 compact result wrapper，detail/publish 场景继续使用原明细接口。
6. Step 5「建立组合」弹层使用「策略配置」和「回测业绩」两段视图，打开时调用 Rearview publish preview；UI 只展示最后信号日和计划建仓日，内部以后端返回的 `required_source_signal_date`、`source_signal_date`、`market_phase` 和 `planned_live_start_date` 作为确认条件；交易日 15:00 前允许上一交易日信号，15:00 后要求当天信号，preview blocked 或 expected date 过期时禁止确认。
7. 通过 Rearview strategy portfolio API 展示看板、详情、净值、信号、signal timeline、虚拟资金账户、对账单、持仓和调仓记录。`pending_first_run` 组合展示待建仓和待调入信号，不展示 live nav、绩效、账户资金、对账单或曲线跳转；详情页在 portfolio record 可用后再查询 live endpoints，并把 `portfolio_pending_first_run` 映射为空 live 状态。详情页页头提供左侧配置图标 + `策略配置方案` 的普通字重无边框按钮，从 `portfolioRecord.rule_snapshot` 和 `portfolioRecord.execution_config` 派生与 Step 5 一致的 `指标过滤`、`权重得分` 和 `建仓摘要`；配置 Dialog 标题下保留分割线，不展示 source backtest 周期、区间、基准和建仓日拼接文案；`ui_display_snapshot` 中的 display-only JSON 不作为权威来源，period/benchmark/preview 文案由前端模板从结构字段生成，不从 live 结果反推配置。策略信号结果区和策略看板买入信号表没有买入信号时保留日期、`0股` 数量和表头，只在表体展示虚线空态 `没有产生买入信号`，文案使用普通弱文本，空态容器不展示滚动条。对账单区块位于虚拟资金账户之后，period 只传 `month`、`three_months`、`six_months`、`ytd`、`all` key，summary、持仓余额和实现盈亏均展示 Rearview 返回值，第一版不提供证券筛选。详情页“删除”按钮调用 `PATCH /rearview/strategy-portfolios/{id}` 归档组合，成功后刷新 dashboard query、移除当前详情 query cache 并跳回 `/dashboard`；旧链接返回 `410 Gone` 时不展示 archived detail，直接返回 `/dashboard`。
8. 继续使用 Rearview default market fee template 初始化 Step 4 草稿，并在 UI 中区分 draft、applied snapshot、backtest result、publish preview、pending portfolio 和 live portfolio result。

## 非职责

1. 不实现 Rearview 规则编译、ClickHouse 查询、PostgreSQL 写入或 worker 状态机。
2. 不直接访问 ClickHouse、PostgreSQL、NATS 或 dbt。
3. 不在浏览器内计算权威成交、持仓、费用、滑点、净值、绩效或 backtest hash。
4. 不引入登录入口、认证/鉴权、用户隔离或权限系统。

## 技术栈

技术栈和工程边界以 [ADR 0011](../ADR/0011-racingline-frontend-technology-stack.md) 为基础，UI 栈变体决策以 [ADR 0013](../ADR/0013-racingline-ui-stack-variant-evaluation.md) 为准。当前正式实现使用 Vite + React + TypeScript、Tailwind CSS v4 + CSS Variables、shadcn/ui + Base UI、Hugeicons 与 Lucide 的组合、React Router、TanStack Query、Zustand 和 TradingView Lightweight Charts。

## 工程管理

`app/racingline/` 采用单独 package 管理，只在该目录维护 `package.json`、lockfile、Vite 配置和 npm scripts。前端运行时配置只使用仓库根目录 `.env` 和 `.env.example`；Vite 通过 `envDir: "../.."` 从仓库根读取配置。

本地开发入口：

```bash
make racingline-dev
```

该命令会按端口清理既有 Frontend / Rearview 进程，准备 Docker 依赖，执行 PostgreSQL migrations，同步 Rearview metric catalog，并启动 Rearview server、Rearview portfolio worker 和前端 `http://127.0.0.1:5173/`。

只启动前端：

```bash
make racingline-frontend-dev
```

只清理前端和 Rearview dev server 端口：

```bash
make racingline-dev-stop
```

## 后端依赖

| 系统 | 依赖 |
|---|---|
| [Rearview](rearview.md) | metric catalog、preview API、strategy backtest control plane、strategy portfolio API、security analysis API |
| Furnace/dbt marts | 通过 Rearview 间接消费指标 mart，不由前端直接访问 |

## 浏览器调试

前端调试优先复用 Docker `vnc-mini-desktop` 暴露的 Chromium CDP 端点：

```text
http://127.0.0.1:9222
```

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

## 质量门禁

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [../ADR/0011-racingline-frontend-technology-stack.md](../ADR/0011-racingline-frontend-technology-stack.md) | 基础前端工程边界 |
| [../ADR/0013-racingline-ui-stack-variant-evaluation.md](../ADR/0013-racingline-ui-stack-variant-evaluation.md) | 当前 Racingline UI 栈决策 |
| [../RFC/archive/0023-racingline-frontend-prototype-led-development.md](../RFC/archive/0023-racingline-frontend-prototype-led-development.md) | 前端原型/正式替换流程 |
| [../RFC/archive/0024-racingline-strategy-selection-step1.md](../RFC/archive/0024-racingline-strategy-selection-step1.md) | Step 1 策略选股 |
| [../RFC/archive/0025-racingline-strategy-weight-configuration-step2.md](../RFC/archive/0025-racingline-strategy-weight-configuration-step2.md) | Step 2 权重配置 |
| [../RFC/archive/0026-racingline-strategy-pool-preview-step3.md](../RFC/archive/0026-racingline-strategy-pool-preview-step3.md) | Step 3 股池预览 |
| [../RFC/archive/0027-racingline-strategy-simulation-position-step4.md](../RFC/archive/0027-racingline-strategy-simulation-position-step4.md) | Step 4 模拟建仓 |
| [../RFC/archive/0028-racingline-strategy-backtest-step5.md](../RFC/archive/0028-racingline-strategy-backtest-step5.md) | Step 5 异步回测 |
| [../RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md](../RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md) | 策略组合发布和日运行 |
| [../jobs/reports/2026-06-24-racingline-strategy-portfolio-publish-dashboard-dagster.md](../jobs/reports/2026-06-24-racingline-strategy-portfolio-publish-dashboard-dagster.md) | Step 5 建立组合、Dashboard 接入、Dagster daily run 注册和 0051 低位反转组合创建验收报告 |
| [../RFC/archive/0044-racingline-0051-low-reversal-regression-case.md](../RFC/archive/0044-racingline-0051-low-reversal-regression-case.md) | 0051 低位反转端到端回归用例固化讨论 |
| [../RFC/archive/0031-racingline-step4-step5-backtest-latency-slimming.md](../RFC/archive/0031-racingline-step4-step5-backtest-latency-slimming.md) | Step 4 到 Step 5 回测延时瘦身依据、字段审计和性能基线 |
| [../plans/archive/0056-racingline-step4-step5-backtest-latency-optimization-plan.md](../plans/archive/0056-racingline-step4-step5-backtest-latency-optimization-plan.md) | Step 4/5 handoff、status/compact API、worker timing、动态 price bars 和 outbox 唤醒实施计划 |
| [../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md](../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md) | Step 4/5 回测延时优化验收报告 |
| [../plans/archive/0058-racingline-step5-backtest-worker-latency-optimization-plan.md](../plans/archive/0058-racingline-step5-backtest-worker-latency-optimization-plan.md) | Step 5 worker 热路径、overview 首屏读取和 pickup wait 治理完成计划 |
| [../jobs/reports/2026-06-26-racingline-step5-backtest-worker-latency-optimization.md](../jobs/reports/2026-06-26-racingline-step5-backtest-worker-latency-optimization.md) | Step 5 worker latency live smoke、frontend overview 门禁和 queue smoke 报告 |
| [../plans/archive/0060-racingline-step5-portfolio-publish-dialog-tplus1-plan.md](../plans/archive/0060-racingline-step5-portfolio-publish-dialog-tplus1-plan.md) | Step 5「建立组合」弹层、T+1 publish preview、pending dashboard/detail 和 live/backtest 语义分离完成计划 |
| [../jobs/reports/2026-06-27-racingline-portfolio-publish-tplus1-smoke.md](../jobs/reports/2026-06-27-racingline-portfolio-publish-tplus1-smoke.md) | T+1 publish、pending endpoint、daily run、ClickHouse split 和 performance success 端到端验收报告 |
| [../plans/archive/0061-racingline-strategy-portfolio-virtual-account-panel-plan.md](../plans/archive/0061-racingline-strategy-portfolio-virtual-account-panel-plan.md) | 组合详情页「虚拟资金账户」区块、Rearview account read model 和前后端闭环完成计划 |
| [../jobs/reports/2026-06-28-racingline-portfolio-virtual-account-panel.md](../jobs/reports/2026-06-28-racingline-portfolio-virtual-account-panel.md) | 虚拟资金账户 API、pending 语义、桌面/移动端 UI 和质量门禁验收报告 |
| [../plans/archive/0062-racingline-strategy-portfolio-statement-plan.md](../plans/archive/0062-racingline-strategy-portfolio-statement-plan.md) | 组合详情页「对账单」区块、Rearview statement read model、Dagster 清算 fact-count 核验和 2025 T+1 建仓验收完成计划 |
| [../jobs/reports/2026-06-29-racingline-strategy-portfolio-statement.md](../jobs/reports/2026-06-29-racingline-strategy-portfolio-statement.md) | 对账单 API、ClickHouse live facts、Dagster materialization metadata 和 mini-desktop CDP 截图验收报告 |
| [../RFC/archive/0043-racingline-strategy-detail-delete-action.md](../RFC/archive/0043-racingline-strategy-detail-delete-action.md) | 策略详情页删除按钮归档语义、410 跳转和前后端 contract 设计 |
| [../plans/archive/0071-racingline-strategy-detail-delete-action-plan.md](../plans/archive/0071-racingline-strategy-detail-delete-action-plan.md) | 策略详情页删除按钮、Rearview archived 410 和 manual daily run 拒绝完成计划 |
| [../jobs/reports/2026-07-02-racingline-strategy-detail-delete-action.md](../jobs/reports/2026-07-02-racingline-strategy-detail-delete-action.md) | 策略详情页删除按钮归档 API、前端跳转和质量门禁验收报告 |
| [../RFC/archive/0046-racingline-strategy-detail-config-display.md](../RFC/archive/0046-racingline-strategy-detail-config-display.md) | 策略详情页 `策略配置方案` 入口、Step 5 配置展示复用和 canonical 配置派生展示设计 |
| [../plans/archive/0074-racingline-strategy-detail-config-display-plan.md](../plans/archive/0074-racingline-strategy-detail-config-display-plan.md) | 策略详情页配置展示实施完成计划 |
| [../jobs/reports/2026-07-02-racingline-strategy-detail-config-display.md](../jobs/reports/2026-07-02-racingline-strategy-detail-config-display.md) | 策略详情页配置展示、质量门禁和 CDP smoke 验收报告 |
| [../plans/archive/0075-racingline-strategy-real-config-snapshot-correction-plan.md](../plans/archive/0075-racingline-strategy-real-config-snapshot-correction-plan.md) | 0074 纠偏完成：从 `rule_snapshot` + `execution_config` 派生配置展示、拒绝 display-only JSON、0051 example 和真实 browser smoke |
| [../plans/archive/0053-racingline-legacy-cleanup-and-rename-plan.md](../plans/archive/0053-racingline-legacy-cleanup-and-rename-plan.md) | 旧工程清理和目录重命名实施计划 |
| [../jobs/reports/2026-06-25-racingline-legacy-cleanup-rename.md](../jobs/reports/2026-06-25-racingline-legacy-cleanup-rename.md) | 清理和重命名验收报告 |
