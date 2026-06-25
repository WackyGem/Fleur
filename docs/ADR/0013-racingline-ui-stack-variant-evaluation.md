# ADR 0013: Racingline UI 栈变体升级为正式栈

状态：Accepted

日期：2026-06-25

## 背景

ADR 0011 固化了 Racingline 第一版前端技术栈：Vite、React、TypeScript、Tailwind CSS v4、shadcn/ui + Base UI、Hugeicons、React Router、TanStack Query、Zustand 和 Lightweight Charts。第一版 `app/racingline/` 已完成 run、rule、metric、security analysis 和旧 portfolio 页面。

随后 `app/racingline_new/` 作为并行策略工作台完成了 `/dashboard`、`/strategies`、strategy backtest 和 strategy portfolio 的真实 Rearview API 闭环。Plan 0053 决定删除旧 `app/racingline/`，并将 `app/racingline_new/` 重命名为正式 `app/racingline/`。

这次替换也意味着 UI 栈从第一版 `base-nova` / `neutral` / Hugeicons-only 形态迁移到新策略工作台实际使用的栈。

## 决策

接受新策略工作台当前 UI 栈作为 Racingline 正式栈：

| 类别 | 正式选择 |
|---|---|
| shadcn style | `base-lyra` |
| Tailwind base color | `taupe` |
| menuColor | `inverted` |
| UI primitive | Base UI (`@base-ui/react`) |
| 字体 | IBM Plex Sans |
| 图标 | Lucide 与 Hugeicons 并存；新通用控件优先使用 Lucide，已有业务语义图标可继续使用 Hugeicons |

ADR 0011 仍保留 Vite、React、TypeScript、Tailwind CSS v4、shadcn/ui、Base UI、React Router、TanStack Query、Zustand、Lightweight Charts、单独 package 管理和根目录 env 的基础工程边界。本 ADR 覆盖 ADR 0011 中关于 shadcn style、主题、字体和图标的第一版选择。

## 理由

1. `app/racingline_new/` 已经通过 Step 1/2/3/4/5、strategy portfolio dashboard 和详情页的真实接口闭环验证，迁移成本低于把新业务回写到旧 UI 栈。
2. `base-lyra`、`taupe` 和 IBM Plex Sans 更贴合当前策略研究工作台的信息密度和视觉语气。
3. 新工程已经在真实页面中同时使用 Lucide 和 Hugeicons；强制单一图标库会带来无业务收益的大规模替换。
4. Plan 0053 删除旧工程后，继续坚持旧 UI 栈会让文档和当前代码事实冲突。

## 后果

1. `app/racingline/` 的 `components.json` 以 `base-lyra`、`taupe`、`lucide` 和 `menuColor: inverted` 为准。
2. 后续新增 shadcn/ui 组件应遵循当前 `app/racingline/components.json`，不得再按旧 `base-nova` 假设生成。
3. 业务组件仍不得直接改写 shadcn/ui 生成的默认组件；默认组件更新应通过 shadcn CLI 或明确的组件升级计划。
4. 图标新增默认优先 Lucide；Hugeicons 只在已有业务语义或 Lucide 缺少合适图标时使用。
5. `app/racingline_new/` 不再作为并行原型工程存在；历史文档中的该路径只表示迁移前事实。

## 关联文档

- [ADR 0011: Racingline 前端技术栈和工程边界](0011-racingline-frontend-technology-stack.md)
- [RFC 0023: Racingline 前端原型驱动开发流程](../RFC/archive/0023-racingline-frontend-prototype-led-development.md)
- [Plan 0053: Racingline 旧工程清理与 `racingline_new` 重命名实施计划](../plans/archive/0053-racingline-legacy-cleanup-and-rename-plan.md)
- [System: Racingline](../systems/racingline.md)
