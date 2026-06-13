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
4. 第一版不在没有后端鉴权设计的情况下自行定义用户隔离协议。

## 技术栈

| 类别 | 选型 |
|---|---|
| 构建工具 | Vite |
| 前端框架 | React |
| 开发语言 | TypeScript |
| CSS 风格与样式体系 | Tailwind CSS v4（`@tailwindcss/vite`）+ CSS Variables |
| UI 组件体系 | shadcn/ui（style: `base-nova`）+ Base UI（`@base-ui/react`） |
| 图标 | Hugeicons（`@hugeicons/react`） |
| 类名与变体工具 | `clsx` + `tailwind-merge` + `class-variance-authority` |
| 代码规范 | ESLint Flat Config + `typescript-eslint` + `react-hooks` + `react-refresh` |
| 路由 | React Router |
| 服务端状态 | TanStack Query |
| 客户端状态 | Zustand |
| 图表 | TradingView Lightweight Charts |

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
| [../RFC/0018-rust-stock-screening-service.md](../RFC/0018-rust-stock-screening-service.md) | Rearview 后端服务 RFC |
| [rearview.md](rearview.md) | Rearview 当前系统地图 |

## 待决问题

1. `app/` 是否需要顶层 workspace 管理器，还是 `app/racingline/` 先作为单独 package 管理。
2. API base URL、CORS 和本地开发环境变量如何约定。
3. 是否在第一版引入登录入口，还是等待 Rearview 鉴权设计统一处理。
