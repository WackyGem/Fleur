# Intake: Racingline

状态：当前需求投递入口（2026-06-13）

当前事实地图：[../systems/racingline.md](../systems/racingline.md)

## 适用需求

- Racingline 前端页面、路由、布局、交互和工作流。
- React/TypeScript/Tailwind/shadcn/Base UI/Hugeicons 组件体系。
- TanStack Query、Zustand、TradingView Lightweight Charts 状态和图表需求。
- `app/racingline/` 单独 package 管理、根目录 `.env` / `.env.example` 和 `VITE_REARVIEW_API_BASE_URL` 配置。
- Playwright CDP 调试、截图、DOM、console、network 和响应式验收。
- 由前端驱动的 Rearview API 补齐需求。

## 不适用

- Rearview 后端状态机、API 实现和 database schema：走 [rearview.md](rearview.md)。
- mart 模型和指标字段事实：走 [data-platform.md](data-platform.md) 或 [data-governance.md](data-governance.md)。
- Rust 指标计算：走 [furnace.md](furnace.md)。

## 投递材料

1. 目标用户工作流和首屏/页面入口。
2. 目标页面、路由、关键组件、表格/图表/详情抽屉需求。
3. 依赖的 Rearview API、缺失接口和请求/响应草案。
4. API base URL、根目录 `.env` / `.env.example` 变量和本地开发端口约定。
5. 视觉约束、响应式断点和可访问性/可用性要求。
6. Playwright CDP 验收步骤和截图要求。

## 文档落点

| 情况 | 落点 |
|---|---|
| 新前端产品能力或跨页面工作流 | `docs/RFC/` |
| 已确定的前端实施阶段 | `docs/plans/` |
| 前端工程栈、工作区边界或长期 UI 协议变化 | `docs/ADR/` 或 [../systems/racingline.md](../systems/racingline.md) |
| Playwright CDP agent 调试流程变化 | `docs/skills/playwright-cdp-frontend-debug/SKILL.md` |
| 需要后端补接口 | 同步关联 [rearview.md](rearview.md) |

## 验证要求

`app/racingline/` 创建前，需求 RFC 必须写明预期验证方式。创建后至少提供：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run build
```

浏览器调试使用：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```
