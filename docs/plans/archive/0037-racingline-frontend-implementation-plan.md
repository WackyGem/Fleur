# Plan 0037: Racingline 前端第一版实施计划

日期：2026-06-13

状态：Completed

领域：racingline

关联系统：racingline, rearview

代码根：`app/racingline/`

系统地图：`docs/systems/racingline.md`

完成报告：

- [../../jobs/reports/2026-06-13-racingline-frontend-skeleton.md](../../jobs/reports/2026-06-13-racingline-frontend-skeleton.md)
- [../../jobs/reports/2026-06-13-racingline-rearview-api-integration.md](../../jobs/reports/2026-06-13-racingline-rearview-api-integration.md)
- [../../jobs/reports/2026-06-13-racingline-playwright-cdp-acceptance.md](../../jobs/reports/2026-06-13-racingline-playwright-cdp-acceptance.md)

## 目标

1. 按 [ADR 0011](../../ADR/0011-racingline-frontend-technology-stack.md) 搭建 `app/racingline/` 独立前端 package，不引入 `app/` 顶层 workspace 管理器。
2. 实现 [RFC 0019](../../RFC/0019-racingline-rearview-frontend-workbench.md) 定义的第一版页面：运行看板、运行详情与结果页、规则工作台和指标目录。
3. 建立可维护的前端工程分层：API client、类型模型、query hooks、路由、页面、feature components、设计令牌、状态管理和错误处理。
4. 完成 Rearview API 联调，覆盖已存在接口、需增强接口和需补齐接口的验收边界。
5. 以 Playwright CDP 浏览器环境作为第一版验收主证据链，记录交互过程、截图、console、network 和响应式结果。
6. 代码层面只保留必要质量门禁和少量高价值纯逻辑测试，不用大规模组件测试或端到端测试替代 CDP 交互验收。
7. 形成验收报告清单，确保后续实现完成后有可追溯的命令、截图、接口状态和遗留问题记录。

## 非目标

1. 本计划不实现登录入口、认证/鉴权、用户隔离或权限系统。
2. 本计划不实现交易、下单、风控、组合调仓或完整回测。
3. 本计划不要求 Racingline 直接访问 ClickHouse 或 PostgreSQL。
4. 本计划不把 Rearview 后端缺失接口伪装成前端已完成能力；前端可以有开发期 fixture，但验收必须以真实 Rearview API 为准。
5. 本计划不创建第二个前端应用或共享前端 package；如需要，应先更新 ADR 0011。

## 当前事实基线

1. `app/racingline/` 尚未创建；当前只有文档规划。
2. Racingline 技术栈、独立 package 和 `VITE_REARVIEW_API_BASE_URL` 变量名已由 ADR 0011 接受。
3. 本计划将前端环境变量入口收敛为仓库根目录 `.env` 和 `.env.example`；`app/racingline/` 下不另行创建 `.env`、`.env.local`、`.env.example` 或其他 `.env*` 文件。
4. ADR 0011、RFC 0019 或系统地图中仍指向 `app/racingline/.env*` 的旧表述，应在实现 PR 或同一文档调整中同步改为根目录 env 入口。
5. Racingline 第一版页面、接口矩阵、数据约定和交互流程已由 RFC 0019 定义。
6. Rearview 当前已存在接口：
   - `GET /healthz`
   - `POST /rearview/explain`
   - `POST /rearview/rule-sets`
   - `POST /rearview/rule-sets/{rule_set_id}/versions`
   - `POST /rearview/runs`
   - `GET /rearview/runs/{run_id}`
   - `GET /rearview/runs/{run_id}/chunks`
   - `GET /rearview/runs/{run_id}/days`
   - `GET /rearview/runs/{run_id}/pool?trade_date=...`
   - `GET /rearview/runs/{run_id}/signals?trade_date=...`
7. 第一版前端闭环仍依赖 Rearview 补齐：
   - `GET /rearview/runs`
   - `GET /rearview/rule-sets`
   - `GET /rearview/rule-sets/{rule_set_id}/versions`
   - `GET /rearview/metrics`
   - pool/signals 分页、排序、证券代码过滤
   - 统一错误响应和 CORS
8. 浏览器调试环境使用 Docker `vnc-mini-desktop` 暴露的 Chromium CDP 端点，默认 `http://127.0.0.1:9222`。
9. shadcn/ui skill 已放在 `.agents/skills/shadcn`，Racingline 组件开发时应按 AGENTS.md 路由使用。

## 实施和验收口径

第一版验收以真实浏览器中的用户工作流为主。代码检查用于证明工程可构建、类型边界可维护和少量关键纯逻辑可靠；它不是第一版用户体验验收的替代品。

### 代码测试口径

必须执行：

1. `npm run lint`
2. `npm run typecheck`
3. `npm run build`

仅在实现了下列纯逻辑时添加定向单元测试：

1. run/chunk/day 状态分类，例如 active、terminal、failed。
2. API query 参数序列化和错误响应解析。
3. `score_breakdown`、`selected_metrics` 或 metric 动态列的格式化逻辑。
4. 规则草稿到 `RuleVersionSpec` 的转换逻辑。

第一版不要求为了覆盖率新增大规模组件单测、mock-heavy hook 测试或 Playwright test suite。需要验证用户行为时，优先通过 CDP 交互过程和截图证据链完成。

### CDP 验收口径

每个关键工作流的验收记录必须包含：

1. 环境信息：前端 URL、Rearview API base URL、CDP endpoint、浏览器视口、提交版本或分支。
2. 交互步骤：从入口页面开始，逐步记录用户动作和页面状态变化。
3. 截图证据：每个关键状态至少一张截图，路径写入 job report。
4. Network 证据：关键 Rearview 请求的 method、path、status、主要 query/body 和响应结论。
5. Console 证据：是否存在 runtime error、unhandled rejection、React warning 或资源加载错误。
6. 响应式证据：桌面和移动视口均需覆盖，重点检查表格、抽屉、表单和导航。
7. 结论：通过、失败或阻塞；失败和阻塞必须记录原因、owner 和后续文档落点。

## 实施阶段

### Phase 0: 实施准备和接口冻结

目标：在写前端代码前确认工程边界、后端接口状态和验收口径。

任务：

1. 确认 `app/racingline/` 使用独立 package，不在 `app/` 顶层创建 workspace 配置。
2. 确认 package manager 和 lockfile 类型，并在 `app/racingline/package.json` 的 `packageManager` 字段中记录。
3. 确认 Rearview 本地服务端口、根目录 `.env.example` 中的 `VITE_REARVIEW_API_BASE_URL` 默认值，以及根目录 `.env` 的本地覆盖方式。
4. 将 RFC 0019 的接口矩阵转为 implementation checklist，标记每个接口为 `ready`、`needs-backend` 或 `needs-enhancement`。
5. 确认后端优先补齐顺序：先列表类接口，再 pool/signals 查询增强，再错误响应和 CORS。
6. 确认验收报告落点为 `docs/jobs/reports/`。

完成标准：

1. 代码实现前没有未决的 package/workspace/API base URL/env 读取目录决策。
2. 后端缺口以 checklist 形式记录在实现 PR 或实施报告中。
3. 若后端接口未准备好，前端实现明确标注哪些页面只能用 fixture 进入开发态，不能计入最终验收。
4. 验收报告模板先明确 CDP 证据链字段，避免实现后只补命令输出。

### Phase 1: 项目骨架搭建

目标：使用 shadcn/ui 官方脚手架创建可运行、可 lint、可 typecheck、可 build 的 Vite + React + TypeScript 前端骨架，避免手写模板导致工程实现漂移。

任务：

1. 从 `app/` 目录运行 shadcn/ui 官方脚手架创建工程；npm 场景使用：

```bash
cd app
npx shadcn@latest init --name racingline --template vite --preset base-nova
```

   如 Phase 0 已决定使用 pnpm 或 bun，应改用对应的官方 package runner（例如 `pnpm dlx shadcn@latest ...` 或 `bunx --bun shadcn@latest ...`），但仍必须使用 `shadcn@latest init --name racingline --template vite --preset base-nova` 这一官方创建路径。
2. 不使用 `create-vite` 后再手工拼接 shadcn/ui，也不复制自定义模板重建骨架；脚手架生成的 `components.json`、Tailwind 入口、CSS Variables 和 shadcn 配置作为项目初始基线。
3. 核对并补齐 ADR 0011 规定的依赖；脚手架已生成的依赖不重复替换：
   - Vite、React、TypeScript
   - Tailwind CSS v4 和 `@tailwindcss/vite`
   - shadcn/ui（style: `base-nova`）和 `@base-ui/react`
   - `@hugeicons/react`
   - `clsx`、`tailwind-merge`、`class-variance-authority`
   - `react-router-dom`
   - `@tanstack/react-query`
   - `zustand`
   - `lightweight-charts`
   - ESLint Flat Config、`typescript-eslint`、`react-hooks`、`react-refresh`
4. 维护仓库根目录 `.env.example`，至少包含：

```text
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34057
```

   本地覆盖只写入仓库根目录 `.env`。不要在 `app/racingline/` 下创建 `.env`、`.env.local`、`.env.example` 或其他 `.env*` 文件。
5. 在 `app/racingline/vite.config.ts` 配置 Vite 从仓库根目录读取 env 文件，例如 `envDir: '../..'`。前端代码仍只通过 `import.meta.env.VITE_REARVIEW_API_BASE_URL` 读取 API base URL，且不得扩大客户端 env 暴露前缀。
6. 创建基础 scripts：
   - `dev`
   - `lint`
   - `typecheck`
   - `build`
   - 如引入预览命令，命名为 `preview`
7. 建立基础目录结构：

```text
app/racingline/
├── components.json
├── index.html
├── package.json
├── src/
│   ├── app/
│   ├── api/
│   ├── features/
│   │   ├── metrics/
│   │   ├── rules/
│   │   └── runs/
│   ├── lib/
│   ├── routes/
│   ├── store/
│   ├── styles/
│   └── types/
└── vite.config.ts
```

8. 仅在脚手架基线上按 ADR 0011 调整 alias、Tailwind 入口、CSS Variables 设计令牌和 shadcn/ui 工具函数；不得绕过官方脚手架重写同等配置。
9. 移除模板示例页面，根路由直接进入 `/runs`。

完成标准：

1. `cd app/racingline && npm run lint` 通过。
2. `cd app/racingline && npm run typecheck` 通过。
3. `cd app/racingline && npm run build` 通过。
4. 仓库根目录 `.env.example` 包含 `VITE_REARVIEW_API_BASE_URL` 且不包含任何非公开密钥；`app/racingline/` 下不存在 `.env*` 文件。
5. `app/racingline/vite.config.ts` 明确从仓库根目录读取 env 文件，`import.meta.env.VITE_REARVIEW_API_BASE_URL` 在 dev/build 中可用。
6. 如果 Phase 1 未新增可测试纯逻辑，不为骨架阶段补低价值单元测试。

### Phase 2: 工程详细设计

目标：在页面实现前建立清晰的模块边界，避免组件直接散落 API 请求、状态和格式化逻辑。

#### API 和类型设计

任务：

1. 在 `src/types/rearview.ts` 定义 RFC 0019 的前端类型：
   - `RuleSetRecord`
   - `RuleVersionRecord`
   - `RunRecord`
   - `RunChunkRecord`
   - `RunDayRecord`
   - `PoolMemberRecord`
   - `BuySignalRecord`
   - `MetricDefinition`
   - `RuleVersionSpec`
   - `FilterExpr`
   - `Operand`
   - `ScoringSpec`
2. 在 `src/api/client.ts` 建立统一 HTTP client：
   - 读取 `import.meta.env.VITE_REARVIEW_API_BASE_URL`
   - 统一拼接 path 和 query
   - 统一解析 JSON
   - 统一处理 non-2xx 响应
   - 映射 `error_type`、`message`、`field_path`
3. 在 `src/api/rearview.ts` 按资源封装接口：
   - health
   - metrics
   - rule sets
   - rule versions
   - runs
   - run chunks
   - run days
   - pool
   - signals
   - explain
4. 在 `src/api/queryKeys.ts` 固定 TanStack Query key 结构。
5. 为轮询状态定义 `isRunTerminalStatus`、`isRunActiveStatus` 和 `isChunkActiveStatus`。

完成标准：

1. 页面组件不直接写 `fetch`。
2. 所有 query/mutation 使用同一 API client。
3. run 状态枚举与 RFC 0019 和 Rearview schema 对齐。
4. 如果实现状态分类、query 序列化或错误解析纯函数，应添加定向单元测试；否则不新增测试框架。

#### 路由和页面设计

任务：

1. 使用 React Router 建立以下路由：
   - `/` 重定向到 `/runs`
   - `/runs`
   - `/runs/:runId`
   - `/rules`
   - `/metrics`
2. 建立应用壳：
   - 侧边或顶部导航
   - 后端健康状态横幅
   - 页面级 loading/error/empty 状态
   - 全局 query error 处理
3. 路由级组件只负责页面编排，复杂 UI 下沉到 `features/*`。

完成标准：

1. 路由刷新和直接打开深链可用。
2. 后端不可达时仍保留页面壳和本地导航。
3. 没有登录页、营销页或与选股无关的首页。

#### 状态和表单设计

任务：

1. TanStack Query 管理所有 Rearview 服务端状态。
2. Zustand 只保存客户端 UI 状态：
   - 当前规则草稿
   - 当前交易日选择
   - 表格列显示偏好
   - 抽屉/面板展开状态
3. 规则工作台表单覆盖：
   - universe
   - pool filters
   - scoring
   - score clamp
   - output metrics
   - top_n_default
4. 规则表单提交前必须调用 `POST /rearview/explain`。
5. JSON 预览只读，不作为第一版主要编辑方式。

完成标准：

1. 用户草稿不会因 explain 失败或网络错误被清空。
2. 发布后的规则版本不可原地编辑。
3. 服务端事实不复制进 Zustand 做长期缓存。

#### UI 组件和样式设计

任务：

1. 使用 shadcn/ui 组件构建按钮、表格、表单、tabs、badge、sheet/dialog、empty、skeleton 和 alert。
2. 需要底层无障碍 primitive 时使用 Base UI。
3. 使用 Hugeicons 表达操作按钮图标。
4. 用 CSS Variables 定义设计令牌，Tailwind class 只消费语义 token。
5. 工作台布局优先紧凑、可扫描、稳定尺寸；避免营销 hero 和装饰性页面区块。
6. 结果表格动态列来自 `output_metrics` 或 `selected_metrics`，必须保证长字段不撑破布局。

完成标准：

1. 组件实现遵守 shadcn skill 的组合、表单、图标和样式规则。
2. 页面在桌面和移动断点没有明显文字重叠或布局跳动。
3. 运行时快照和当前 mart 查询值在视觉上明确区分。

### Phase 3: 页面实现

目标：按 RFC 0019 完成第一版用户可见工作流。

#### `/runs` 运行看板

任务：

1. 调用 `GET /healthz` 显示服务可达状态。
2. 调用 `GET /rearview/runs` 加载运行列表。
3. 提供状态、规则集、日期区间和关键词筛选。
4. 展示运行表格字段：`run_id`、规则集名称、`rule_version_id`、`rule_hash`、`start_date`、`end_date`、`top_n`、`status`、`summary` 和错误摘要。
5. 支持打开详情、复制 `run_id` 和查看失败错误。

完成标准：

1. 后端列表接口可用时，真实运行数据能进入看板。
2. 列表接口不可用时，页面必须明确标注缺失后端能力，不能伪装为真实空数据。

#### `/runs/:runId` 运行详情与结果页

任务：

1. 查询 run、chunks、days。
2. active run 每 2 到 5 秒轮询 run、chunks 和 days。
3. terminal run 停止自动轮询，保留手动刷新。
4. 默认选择最新有信号的成功交易日；没有信号时选择最新成功交易日。
5. 实现 `买入信号`、`股票池`、`运行日历`、`Chunks` tabs。
6. signals 和 pool 表格支持分页、排序和证券代码过滤。
7. 买入信号行点击打开详情抽屉，展示 score breakdown、selected metrics 和快照来源说明。

完成标准：

1. 运行进度、chunk 错误和 day 错误能在对应区域展示。
2. `score_breakdown` 和 `selected_metrics` 使用 PostgreSQL 运行时快照。
3. 非快照补充字段必须标记为当前 mart 查询值。

#### `/rules` 规则工作台

任务：

1. 调用 `GET /rearview/rule-sets` 加载规则集。
2. 调用 `GET /rearview/rule-sets/{rule_set_id}/versions` 加载版本。
3. 调用 `GET /rearview/metrics` 填充 metric、operator 和 output metrics 选择器。
4. 支持创建规则集。
5. 支持表单化创建规则草稿。
6. 支持 explain 校验，并展示 required metrics、required marts、required columns、SQL hash 和 chunk plan。
7. explain 成功后支持发布不可变规则版本。
8. 支持选择规则版本、日期区间和 top_n 发起运行。

完成标准：

1. 用户无需手写完整 JSON 即可配置一版规则。
2. explain 失败能定位到规则或字段级错误。
3. 发布版本后能直接发起 run 并跳转到详情页。

#### `/metrics` 指标目录

任务：

1. 调用 `GET /rearview/metrics` 加载 metric catalog。
2. 展示 `logical_metric`、mart、column、value kind、filter/scoring 能力、allowed ops、null policy、default output 和 description。
3. 支持 mart table、value kind、allow_filter、allow_scoring 和 keyword 筛选。
4. 支持复制 metric 名称，并从规则工作台加入 output metrics。

完成标准：

1. 指标目录只展示 Rearview metric catalog allowlist。
2. 规则工作台不能选择 catalog 不允许过滤或评分的 metric。

### Phase 4: 前后端联调

目标：把前端页面从开发态连接到真实 Rearview API，并明确后端缺口。

任务：

1. 启动 Rearview 本地服务，并确认 `GET /healthz`。
2. 用仓库根目录 `.env` 配置 `VITE_REARVIEW_API_BASE_URL`，并确认 Vite 通过 `envDir` 从根目录读取；`app/racingline/` 下不得存在额外 `.env*` 文件。
3. 逐项验证已存在接口：
   - explain 成功和失败
   - 创建 rule set
   - 发布 rule version
   - 创建 run
   - 查询 run/chunks/days
   - 查询某日 pool/signals
4. 逐项验证需补齐接口：
   - 运行列表
   - 规则集列表
   - 规则版本列表
   - 指标目录
5. 验证 pool/signals 的分页、排序和证券代码过滤。
6. 验证错误响应是否包含 `error_type`、`message` 和可选 `field_path`。
7. 验证 Vite dev server 到 Rearview 的 CORS。

完成标准：

1. 所有 RFC 0019 第一版 API 都有真实请求记录。
2. 每个后端缺口都有明确 issue、plan 或 RFC 更新入口。
3. 前端不依赖 fixture 通过最终验收。
4. API 联调结论进入 CDP 验收报告的 network 证据链，而不是只保留终端命令输出。

### Phase 5: 交互调试和浏览器验收

目标：用真实浏览器确认工作台交互、布局、控制台、网络和响应式行为，并形成最终验收主证据链。

任务：

1. 启动前端 dev server。
2. 检查 CDP 连通性：

```bash
node scripts/check_playwright_cdp.mjs
```

3. 使用 `playwright-cli` attach 到默认 CDP 端点：

```bash
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

4. 桌面视口验收并截图：
   - `/runs`
   - `/runs/:runId`
   - `/rules`
   - `/metrics`
5. 移动视口验收并截图：
   - 导航可用
   - 表格不溢出屏幕
   - 抽屉/弹层可关闭
   - 表单控件文字不重叠
6. 关键交互路径验收并截图：
   - 服务健康检查失败和恢复
   - 运行列表筛选和打开详情
   - active run 轮询到 terminal run 停止
   - 切换交易日并查看 signals/pool
   - 打开买入信号详情抽屉
   - 创建规则草稿、explain、发布版本、发起 run
   - 指标目录筛选、复制 metric、加入 output metrics
7. console 检查：
   - 无 React runtime error
   - 无未处理 promise rejection
   - 无重复 key warning
8. network 检查：
   - API base URL 正确
   - 请求失败有可见错误状态
   - active run 轮询停止条件正确
   - 关键 Rearview 请求有 method、path、status 和响应结论记录
9. 图表检查：
   - Lightweight Charts 容器非空
   - 数据为空时显示明确空状态
   - 图表不遮挡表格或详情区域

完成标准：

1. 关键页面和关键交互路径在桌面和移动视口都有截图。
2. 控制台和网络问题有记录、结论和处理状态。
3. 截图能证明第一屏直接进入选股工作流，而不是营销页。
4. 每张截图都有对应步骤编号、视口、URL 和预期结论。
5. CDP 验收报告可以独立复盘用户从创建规则到查看信号解释的完整流程。

### Phase 6: 验收报告和归档准备

目标：实现完成后形成可追溯验收材料，并决定计划是否归档。

任务：

1. 在 `docs/jobs/reports/` 记录骨架和必要工程门禁报告。
2. 在 `docs/jobs/reports/` 记录前后端联调报告，作为 CDP network 证据的补充。
3. 在 `docs/jobs/reports/` 记录交互调试和截图验收报告，作为第一版验收主报告。
4. 更新 `docs/systems/racingline.md` 的实际运行命令和质量门禁。
5. 如第一版完成，将本 plan 状态改为 `Completed` 并移入 `docs/plans/archive/`，同步更新 `docs/plans/README.md`。

完成标准：

1. 验收报告包含命令、时间、环境、URL、交互步骤、截图路径、接口结果、console/network 结论和遗留问题。
2. Racingline 系统地图不再停留在“代码尚未创建”的状态。
3. 所有第一版非目标仍未被前端绕过实现。

## 交付物清单

### 代码交付物

1. `app/racingline/package.json`
2. `app/racingline/package-lock.json` 或等价 lockfile
3. 仓库根目录 `.env.example` 的 `VITE_REARVIEW_API_BASE_URL` 条目
4. `app/racingline/vite.config.ts`
5. `app/racingline/components.json`
6. `app/racingline/src/` 前端源码
7. lint、typecheck、build scripts

### 文档交付物

1. `docs/jobs/reports/YYYY-MM-DD-racingline-frontend-skeleton.md`
2. `docs/jobs/reports/YYYY-MM-DD-racingline-rearview-api-integration.md`
3. `docs/jobs/reports/YYYY-MM-DD-racingline-playwright-cdp-acceptance.md`
4. CDP 验收截图目录，建议使用 `docs/references/screenshots/racingline/YYYY-MM-DD/`
5. CDP 验收截图清单，随 acceptance report 记录每张截图对应步骤、视口、URL 和结论
6. `docs/systems/racingline.md` 实现后命令更新
7. 如接口协议变化，更新 `docs/RFC/0019-racingline-rearview-frontend-workbench.md`
8. 如技术栈变化，更新 `docs/ADR/0011-racingline-frontend-technology-stack.md`

## CDP 验收报告模板

`docs/jobs/reports/YYYY-MM-DD-racingline-playwright-cdp-acceptance.md` 至少包含以下栏目：

1. 基本信息：日期、执行人、git commit、前端 URL、Rearview API base URL、CDP endpoint、浏览器版本。
2. 前置检查：`node scripts/check_playwright_cdp.mjs` 结果、前端 dev server 状态、Rearview `GET /healthz` 结果。
3. 工程门禁：`npm run lint`、`npm run typecheck`、`npm run build` 结果；如有定向单元测试，记录命令和覆盖的纯逻辑。
4. 工作流证据表：步骤编号、用户动作、页面 URL、视口、预期结果、截图路径、network 请求、console 结论、状态。
5. 截图清单：每张截图的文件名、对应步骤、桌面/移动视口、关键观察点。
6. Network 清单：关键 Rearview 请求的 method、path、status、用途和结论。
7. Console 清单：runtime error、unhandled rejection、React warning、资源加载错误的检查结论。
8. 阻塞和遗留问题：问题、影响、owner、后续文档落点。
9. 验收结论：通过、失败或阻塞；不得只写“通过”而缺少证据链。

## 验证命令

文档-only 规划变更：

```bash
make docs-check
git diff --check
```

Racingline 代码创建后，前端必要质量门禁：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run build
```

如实现了状态分类、query 序列化、错误解析、动态列格式化或规则草稿转换等纯逻辑，追加对应定向单元测试命令。不要为覆盖率目标引入大规模组件测试或 mock-heavy hook 测试。

浏览器调试入口：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

本计划本身不要求前端实现阶段运行 Rust 全量检查。只有同一变更实际修改 Rearview 后端代码时，才在后端 PR 或联调报告中按实际改动追加 Rust 检查：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## 禁止模式

1. 在 `app/` 顶层引入 workspace 管理器但不更新 ADR 0011。
2. 绕过 shadcn/ui 官方脚手架，用 `create-vite` 或手写模板创建/重建第一版工程骨架。
3. 在 `app/racingline/` 或其他项目代码子路径创建 `.env`、`.env.local`、`.env.example` 或其他 `.env*` 文件；仓库根目录 `.env` 和 `.env.example` 是唯一环境变量控制入口。
4. 前端直接访问 ClickHouse 或 PostgreSQL。
5. 页面用 fixture 数据通过最终验收。
6. 规则编辑器要求用户直接手写完整 JSON。
7. 在前端重写 Rearview 规则编译或指标校验逻辑。
8. 把当前 mart 查询值混入运行时快照字段。
9. 复制一套与 Rearview record 不同名的前端 API 模型。
10. 组件开发绕过 shadcn/ui、Base UI 和项目设计令牌，直接堆叠临时样式。
11. 用单元测试、组件测试或 mock 后端的 Playwright test suite 替代真实 CDP 交互验收。
12. 为追求覆盖率添加大量低价值测试，导致计划偏离第一版联调和体验验收。

## 风险和处理

| 风险 | 影响 | 处理 |
|---|---|---|
| Rearview 列表接口未补齐 | `/runs`、`/rules` 和 `/metrics` 无法真实验收 | 先实现 typed adapter 和空/缺口状态，最终验收前必须补齐后端 |
| pool/signals 大结果集无分页 | 结果页卡顿或请求过大 | 后端补分页、排序和证券代码过滤后再验收结果表 |
| metric catalog 字段变化 | 规则工作台选择器和校验漂移 | 前端类型以后端返回为准，变化时同步 RFC 0019 |
| 错误响应不统一 | 表单无法字段级定位 | 后端统一 `error_type`、`message`、`field_path` 后再关闭联调项 |
| 手写脚手架导致工程漂移 | Tailwind、shadcn/base 配置和目录结构与官方基线不一致 | 使用 `shadcn@latest init --name racingline --template vite --preset base-nova` 创建骨架，并只在生成基线上做必要调整 |
| 子路径 env 文件再次出现 | 前端配置入口分裂，联调和部署难以复盘 | 根目录 `.env` 和 `.env.example` 作为唯一入口，Vite 用 `envDir` 指向仓库根目录 |
| shadcn/base 组件组合不一致 | UI 维护成本上升 | 使用 shadcn skill 和 ADR 0011 约束审查组件实现 |
| CDP 浏览器不可达 | 无法完成截图和交互验收 | 先运行 `node scripts/check_playwright_cdp.mjs`，记录阻塞原因 |
| 测试口径过重 | 实现周期被低价值组件测试拖慢 | 保留 lint、typecheck、build 和少量纯逻辑测试，验收主轴回到 CDP 证据链 |

## 完成标准

1. `app/racingline/` 工程存在，由 shadcn/ui 官方脚手架创建，且符合 ADR 0011。
2. RFC 0019 的四个页面可通过真实 Rearview API 完成主要工作流。
3. 第一版后端缺口全部补齐；若确需延期，必须先更新 RFC 0019 缩小第一版范围，不能用前端降级说明替代。
4. lint、typecheck、build 全部通过；只有实际新增纯逻辑时才要求定向单元测试通过。
5. Playwright CDP 验收覆盖桌面和移动视口，关键交互步骤、截图、console/network 结论记录在 job report。
6. 仓库根目录 `.env` 和 `.env.example` 是唯一环境变量控制入口；`app/racingline/vite.config.ts` 通过 `envDir` 读取根目录 env，项目代码子路径没有 `.env*` 文件。
7. `docs/systems/racingline.md` 更新为实现后的当前事实。
8. 本 plan 移入 `docs/plans/archive/`，`docs/plans/README.md` 同步移除 active entry。
