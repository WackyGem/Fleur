# System: Racingline

状态：规划中（2026-06-13）

## 代码根

`racingline` 规划放在 `app/racingline/`，作为 `app/` 目录下的独立前端工作区。当前代码尚未创建；需求和边界以 [RFC 0019](../RFC/0019-racingline-rearview-frontend-workbench.md) 为准。

## 职责

1. 提供 Rearview 指标选股前端工作台。
2. 支持规则集选择、规则版本表单化编辑、explain 校验和运行发起。
3. 展示 run、chunk 和 day 粒度进度。
4. 按交易日展示股票池、TopN 买入信号、score breakdown 和 selected metrics。
5. 用 UI 明确区分运行时结果快照和当前 mart 查询值。

## 非职责

1. 不实现 Rearview 规则编译、ClickHouse 查询、PostgreSQL 写入或业务状态机。
2. 不直接访问 ClickHouse 或 PostgreSQL。
3. 第一版不实现交易、下单、风控、组合调仓或完整回测。
4. 第一版不引入登录入口、认证/鉴权、用户隔离或权限系统。

## 技术栈

技术栈和工程边界以 [ADR 0011](../ADR/0011-racingline-frontend-technology-stack.md) 为权威来源。当前摘要：Vite + React + TypeScript，Tailwind CSS v4 + CSS Variables，shadcn/ui（`base-nova`）+ Base UI，Hugeicons，React Router，TanStack Query，Zustand 和 TradingView Lightweight Charts。

## 工程管理

第一版采用单独 package 管理：只在 `app/racingline/` 维护 `package.json`、lockfile、Vite 配置和 npm scripts；暂不在 `app/` 顶层引入 npm/pnpm/yarn workspace 管理器。

前端运行时配置使用 `app/racingline/.env` 约定。Vite 客户端变量必须使用 `VITE_` 前缀，第一版 API base URL 变量为：

```text
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34057
```

## 后端依赖

| 系统 | 依赖 |
|---|---|
| [Rearview](rearview.md) | 规则集、规则版本、运行、股票池、买入信号和 explain API |
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

具体 agent 调试流程见 [../skills/playwright-cdp-frontend-debug/SKILL.md](../skills/playwright-cdp-frontend-debug/SKILL.md)。官方 Playwright CLI skill 可通过 `playwright-cli install --skills agents` 安装到本地 `.agents/skills/playwright-cli`。

## 实现后质量门禁

`app/racingline/` 创建后必须提供可重复执行的 lint、typecheck 和 build 命令，并在本文档中记录实际命令。建议第一版至少具备：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run build
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [../RFC/0019-racingline-rearview-frontend-workbench.md](../RFC/0019-racingline-rearview-frontend-workbench.md) | Racingline 前端 RFC |
| [../ADR/0011-racingline-frontend-technology-stack.md](../ADR/0011-racingline-frontend-technology-stack.md) | Racingline 前端技术栈和工程边界 |
| [../plans/0037-racingline-frontend-implementation-plan.md](../plans/0037-racingline-frontend-implementation-plan.md) | Racingline 前端第一版实施计划 |
| [../RFC/0018-rust-stock-screening-service.md](../RFC/0018-rust-stock-screening-service.md) | Rearview 后端服务 RFC |
| [rearview.md](rearview.md) | Rearview 当前系统地图 |

## 已决事项

1. `app/racingline/` 第一版按单独 package 管理。
2. API base URL 使用 `app/racingline/.env` 中的 `VITE_REARVIEW_API_BASE_URL`。
3. 第一版不引入登录入口、认证/鉴权、用户隔离或权限系统。
