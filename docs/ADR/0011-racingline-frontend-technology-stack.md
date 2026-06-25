# ADR 0011: Racingline 前端技术栈和工程边界

状态：Accepted

日期：2026-06-13

后续决策：2026-06-25 [ADR 0013](0013-racingline-ui-stack-variant-evaluation.md) 接受 `base-lyra`、`taupe`、IBM Plex Sans、Lucide 与 Hugeicons 并存作为 Racingline 正式 UI 栈；本 ADR 中 Vite、React、TypeScript、Tailwind CSS v4、shadcn/ui、Base UI、React Router、TanStack Query、Zustand、Lightweight Charts、单独 package 管理和根目录 env 边界继续有效。

## 背景

RFC 0019 定义了 `racingline` 作为 Rearview 指标选股前端工作台的第一版需求。该前端尚未创建代码，但已经需要固定工程栈、组件体系、状态管理、图表方案和本地配置约定，避免实现阶段在多个等价方案之间反复切换。

`racingline` 的第一版目标是内部工作台，而不是营销页面或交易系统。界面需要高密度展示规则、运行状态、结果表格、详情抽屉和少量趋势图，因此技术选择优先支持 React 组件组合、表单化规则编辑、服务端状态缓存、可维护样式令牌和可重复构建。

## 决策

`racingline` 前端代码根固定为：

```text
app/racingline/
```

第一版采用独立 package 管理：只在 `app/racingline/` 维护 `package.json`、lockfile、Vite 配置和 package scripts。暂不在 `app/` 顶层引入 npm/pnpm/yarn workspace 管理器；只有出现第二个前端应用或共享前端 package 时，再另起 ADR 或更新本 ADR 评估顶层 workspace。

前端技术栈固定如下：

| 类别 | 选型 |
|---|---|
| 构建工具 | Vite |
| 前端框架 | React |
| 开发语言 | TypeScript |
| CSS 风格与样式体系 | Tailwind CSS v4（`@tailwindcss/vite`）+ CSS Variables（设计令牌） |
| UI 组件体系 | shadcn/ui + Base UI（`@base-ui/react`）；当前 style 见 ADR 0013 |
| 图标 | 当前图标策略见 ADR 0013 |
| 类名与变体工具 | `clsx` + `tailwind-merge` + `class-variance-authority` |
| 代码规范 | ESLint Flat Config + `typescript-eslint` + `react-hooks` + `react-refresh` |
| 路由 | React Router（`react-router-dom`） |
| 服务端状态管理 | TanStack Query（`@tanstack/react-query`） |
| 客户端状态管理 | Zustand（`zustand`） |
| 图表方案 | TradingView Lightweight Charts（`lightweight-charts`） |

## 工程约束

1. Vite 是唯一第一版构建入口；不引入 Next.js、Remix 或其他 SSR/full-stack framework。
2. React + TypeScript 是默认 UI 开发组合；业务 API 类型应从前端本地类型或后续生成协议中收敛，不在组件中散落 `any`。
3. Tailwind CSS v4 通过 `@tailwindcss/vite` 接入；主题颜色、间距、圆角、语义色等长期令牌通过 CSS Variables 表达。
4. shadcn/ui 是组件组合的主入口。当前 style、base color 和 menu 配置以 ADR 0013 为准。需要无障碍 primitive 或更底层交互能力时使用 Base UI，不另行引入 Radix 作为默认 primitive 层。
5. 新增 shadcn/ui 组件时使用官方 CLI；agent 参与组件开发时应使用官方 shadcn skill 辅助。优先组合已有组件，不直接复制不受控的第三方组件实现。
6. 不得改写 shadcn/ui 官方 CLI 生成的默认 UI 组件文件（例如 `src/components/ui/*`）。业务 UI、领域组合、布局和交互封装必须放在独立业务组件目录中，并通过依赖引用这些默认组件完成组合；如默认组件需要更新，使用 shadcn/ui 官方 CLI 重新生成或升级，不手工改写其实现。
7. 图标策略以 ADR 0013 为准。
8. 类名组合统一通过 `clsx`、`tailwind-merge` 和 `class-variance-authority`，避免在组件内堆叠不可复用的字符串拼接。
9. 服务端请求、缓存、轮询、错误和重试策略默认由 TanStack Query 承担；Zustand 只保存跨页面 UI 状态、草稿状态和不属于服务端事实的客户端状态。
10. 股票、指标或运行趋势图使用 TradingView Lightweight Charts；表格和状态面板不为图表库让渡基础布局职责。
11. `app/racingline/` 创建后必须提供可重复执行的 `lint`、`typecheck` 和 `build` scripts。

## 环境配置

第一版使用仓库根目录 `.env` 和 `.env.example` 作为唯一环境变量控制入口。`app/racingline/` 不创建 `.env`、`.env.local`、`.env.example` 或其他 `.env*` 文件；Vite 项目通过 `vite.config.ts` 的 `envDir` 指向仓库根目录读取这些文件。

Rearview API base URL 使用 Vite 客户端变量：

```text
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34057
```

前端代码只能通过 `import.meta.env.VITE_REARVIEW_API_BASE_URL` 读取 Rearview API base URL。不得把非公开密钥、数据库连接串或服务端 token 放入 `VITE_` 变量，也不得在项目代码子路径另行创建环境变量入口。

## 后果

- RFC 0019 继续描述 Racingline 第一版页面、接口和交互流程；本 ADR 是前端技术选型和工程边界的权威来源。
- `docs/systems/racingline.md` 只保留当前事实和指向本 ADR 的摘要，不重复维护完整技术栈决策。
- 实现 `app/racingline/` 时，脚手架、依赖安装、lint/typecheck/build 命令必须与本 ADR 对齐。
- 如果未来需要登录鉴权、顶层 workspace、SSR、共享组件包、替换图标库或替换状态管理方案，应通过新 ADR 或更新本 ADR 处理。

## 关联文档

- `docs/RFC/0019-racingline-rearview-frontend-workbench.md`
- `docs/systems/racingline.md`
- `docs/systems/racingline.md`
