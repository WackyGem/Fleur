# Plan 0053: Racingline 旧工程清理与 `racingline_new` 重命名实施计划

日期：2026-06-25

状态：Completed

完成报告：[2026-06-25 Racingline legacy cleanup and rename](../../jobs/reports/2026-06-25-racingline-legacy-cleanup-rename.md)

领域：racingline

关联系统：racingline, rearview, deploy-ops

代码根：

- `app/racingline/`
- `app/racingline_new/`
- `Makefile`

系统地图：[docs/systems/racingline.md](../../systems/racingline.md)

## 背景

`app/racingline_new/` 已完成策略创建、回测、策略组合发布和看板真实接口闭环，业务上可以替代旧 `app/racingline/`。本计划把并行原型/重构工程切换为唯一正式 Racingline 前端：先盘点并清理旧工程依赖，再删除旧 `app/racingline/`，最后将 `app/racingline_new/` 重命名为 `app/racingline/` 并收敛 Makefile、文档和质量门禁。

## 目标

1. 明确旧 `app/racingline/` 的代码结构、接口依赖、数据依赖和文档引用范围。
2. 明确新 `app/racingline_new/` 的代码结构、接口依赖、数据依赖、运行入口和仍需改名的工程元数据。
3. 删除旧 `app/racingline/` 第一版前端工程，不保留双目录并行状态。
4. 将 `app/racingline_new/` 重命名为 `app/racingline/`，并把 package name、lockfile、README、Makefile、系统地图和 intake 文档同步到正式目录名。
5. 将 `make racingline-dev`、`make racingline-frontend-dev` 作为唯一正式 Racingline 本地入口，移除或归并 `racingline-new-*` 临时入口。
6. 完成后仓库当前事实只指向 `app/racingline/`；`racingline_new` 只允许出现在历史 RFC、归档计划、历史 job report 或明确说明历史背景的文档中。

## 非目标

1. 不在本计划内重做 Racingline 页面视觉设计、导航结构或策略工作流。
2. 不恢复旧 `app/racingline/` 的 `/runs`、`/rules`、`/metrics`、`/portfolios` 页面作为正式入口。
3. 不让前端直接访问 PostgreSQL、ClickHouse、NATS、dbt mart 或 Furnace 输出表。
4. 不修改 Rearview API contract、PostgreSQL migration、ClickHouse schema 或 worker 计算逻辑。
5. 不引入 `app/` 顶层 npm workspace；重命名后仍保持单独 package 管理。
6. 不批量改写历史验收报告中的旧路径；历史文档只在当前事实误导时补状态说明。

## 关联文档

- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)
- [ADR 0011: Racingline 前端技术栈和工程边界](../../ADR/0011-racingline-frontend-technology-stack.md)
- [ADR 0013: Racingline UI 栈变体评估](../../ADR/0013-racingline-ui-stack-variant-evaluation.md)
- [RFC 0023: Racingline 前端原型驱动开发流程](../../RFC/0023-racingline-frontend-prototype-led-development.md)
- [RFC 0029: Racingline 回测结果发布为策略组合与 Dagster 日运行资产](../../RFC/0029-racingline-strategy-portfolio-publish-and-daily-run.md)
- [Plan 0052: Racingline 策略组合发布、看板真实数据与 Dagster 日运行实施计划](0052-racingline-strategy-portfolio-publish-dashboard-dagster-plan.md)

## 当前事实基线

### 代码根和 tracked 文件

| 路径 | tracked 文件数 | 当前角色 |
|---|---:|---|
| `app/racingline/` | 75 | 旧 Racingline 第一版前端，包含 run、rule、metric、run result、个股分析和旧 portfolio 页面 |
| `app/racingline_new/` | 78 | 新策略工作台，包含 dashboard、strategy creation、strategy backtest 和 strategy portfolio 页面 |

根 `.gitignore` 已忽略 `node_modules/`、`dist/` 和 `.playwright-cli/`。当前本地目录中存在这些 ignored 产物，实施删除/重命名前应先清理，避免 ignored 文件留在旧路径或阻塞目录移动。

### 旧 `app/racingline/` 结构

| 区域 | 路径 | 当前用途 |
|---|---|---|
| 路由入口 | `src/App.tsx` | `/runs`、`/runs/:runId`、`/runs/:runId/securities/:securityCode`、`/rules`、`/metrics`、`/portfolios`、`/portfolios/:portfolioRunId` |
| API runtime | `src/api/client.ts`、`src/api/rearview.ts`、`src/api/hooks.ts`、`src/api/queryKeys.ts` | `VITE_REARVIEW_API_BASE_URL` + TanStack Query wrappers |
| Rearview 类型 | `src/types/rearview.ts` | 旧 rule set、run、pool、signal、portfolio run 和 security analysis 类型 |
| 本地状态 | `src/store/workbench.ts` | 旧规则编辑草稿和 RuleVersionSpec builder |
| 业务组件 | `src/features/rules/`、`src/features/runs/`、`src/features/analysis/`、`src/features/portfolio/`、`src/features/metrics/` | 旧工作台页面实现 |
| UI 组件 | `src/components/ui/`、`src/components/racingline/` | shadcn/base-nova + Base UI + Hugeicons 组件组合 |
| 测试 | `src/api/client.test.ts`、`src/lib/status.test.ts`、`src/store/workbench.test.ts`、`src/features/analysis/security-analysis.test.ts` | 旧 API helper、状态和分析工具测试 |

旧工程 `package.json` 的 package name 是 `racingline`，包含 `packageManager: "npm@11.13.0"`。依赖包括 Vite、React、TypeScript、Tailwind CSS v4、shadcn、Base UI、Hugeicons、React Router、TanStack Query、Zustand 和 Lightweight Charts。

### 新 `app/racingline_new/` 结构

| 区域 | 路径 | 当前用途 |
|---|---|---|
| 路由入口 | `src/App.tsx` | `/dashboard`、`/dashboard/strategies/:portfolioId`、`/strategies` |
| API runtime | `src/api/client.ts`、`src/api/rearview.ts`、`src/api/hooks.ts`、`src/api/queryKeys.ts` | `VITE_REARVIEW_API_BASE_URL` + TanStack Query wrappers |
| Rearview 类型 | `src/types/rearview.ts` | strategy preview、strategy backtest、strategy portfolio、security analysis 类型 |
| 策略工作流 | `src/features/strategy/` | Step 1/2/3/4/5 adapter、preview snapshot、execution config、pool trend 和表单工具 |
| 看板 | `src/components/racingline/dashboard/`、`src/routes/dashboard-page.tsx`、`src/routes/strategy-detail-page.tsx` | strategy portfolio dashboard 和详情页面 |
| UI 组件 | `src/components/ui/`、`src/components/racingline/` | shadcn/base-lyra + Base UI，代码中同时存在 Lucide 和 Hugeicons imports |
| 测试 | `src/features/strategy/*.test.ts` | adapter、execution、preview、pool-count-trend 和 utils 测试 |

新工程 `package.json` 的 package name 仍是 `racingline_new`，`package-lock.json` 顶层和 packages root 也仍是 `racingline_new`。重命名时必须同步改为 `racingline`，并补齐是否保留 `packageManager` 的工程决策。

### 接口依赖

两个前端都通过 `src/api/client.ts` 读取 `import.meta.env.VITE_REARVIEW_API_BASE_URL`，默认 `http://127.0.0.1:34057`。Vite 配置均使用 `envDir: "../.."`，运行时配置入口仍是仓库根 `.env` / `.env.example`。

旧 `app/racingline/` 当前封装的 Rearview HTTP 端点：

| API 族 | 端点 |
|---|---|
| health | `GET /healthz` |
| metric catalog | `GET /rearview/metrics` |
| rule set/version | `GET/POST /rearview/rule-sets`、`GET/POST /rearview/rule-sets/{id}/versions` |
| explain/run | `POST /rearview/explain`、`GET/POST /rearview/runs`、`GET /rearview/runs/{id}`、`/chunks`、`/days`、`/pool`、`/signals` |
| run security analysis | `GET /rearview/runs/{run_id}/securities/{security_code}/analysis` |
| market/account templates | `GET /rearview/market-fee-templates/default`、rule-set account templates、`PATCH /rearview/account-templates/{id}` |
| old portfolio runs | `GET/POST /rearview/portfolio-runs`、`/nav`、`/targets`、`/orders`、`/trades`、`/positions`、`/events`、`/performance`、`/closed-trades`、`/trade-metrics` |

新 `app/racingline_new/` 当前封装的 Rearview HTTP 端点：

| API 族 | 端点 |
|---|---|
| metric/explain | `GET /rearview/metrics`、`POST /rearview/explain` |
| strategy preview | `POST /rearview/strategy-preview`、`/timeline`、`/pool-page` |
| fee template | `GET /rearview/market-fee-templates/default` |
| strategy backtest | `POST /rearview/strategy-backtests/validate`、`GET /rearview/strategy-backtests/options`、`POST /rearview/strategy-backtests`、`GET /rearview/strategy-backtests/{id}` |
| backtest results | `/nav`、`/rebalance-records`、`/targets`、`/orders`、`/trades`、`/positions`、`/events`、`/performance`、`/closed-trades`、`/trade-metrics` under `/rearview/strategy-backtests/{id}` |
| strategy portfolio | `POST /rearview/strategy-portfolios`、`GET /rearview/strategy-portfolios/dashboard`、`GET /rearview/strategy-portfolios/{id}`、`/nav`、`/performance`、`/signals`、`/signal-timeline`、`/positions`、`/rebalance-records` |
| security analysis | `POST /rearview/security-analysis` |

### 数据依赖

1. Racingline 浏览器端不直接访问 ClickHouse、PostgreSQL、NATS、S3、dbt 或 Furnace。
2. Racingline 的唯一运行时数据入口是 Rearview HTTP API。
3. Rearview 间接读取 PostgreSQL `rearview` database、ClickHouse `fleur_marts`、`fleur_portfolio`、`fleur_calculation`、NATS JetStream worker 结果和 dbt/Furnace 产出的指标 mart。
4. 新 `dashboard` 当前通过 `useStrategyPortfolioDashboardQuery()` 读取 `/rearview/strategy-portfolios/dashboard`，不再使用 `portfolioCards` mock。
5. 新 `strategy-detail-page` 当前通过 strategy portfolio hooks 读取详情、净值、绩效、信号、持仓和调仓记录；`portfolio-data.ts` 只保留 view model 类型和格式化工具。
6. `src/features/strategy/catalog.ts` 仍是前端表单、fallback catalog 和测试相关工具来源；正式指标事实仍必须来自 `GET /rearview/metrics` 和 Rearview 返回的 allowlist/contract。

### Makefile 和运行入口

当前 Makefile 已有以下状态：

| 项 | 当前值或入口 | 处理要求 |
|---|---|---|
| `RACINGLINE_DEV_HOST` | `127.0.0.1` | 保留 |
| `RACINGLINE_DEV_PORT` | `5173` | 保留 |
| `RACINGLINE_APP_DIR` | `app/racingline_new` | 改为 `app/racingline` |
| `RACINGLINE_NEW_APP_DIR` | `app/racingline_new` | 删除或归并 |
| `racingline-frontend-dev` | 启动 `$(RACINGLINE_APP_DIR)` | 保留，指向重命名后的 `app/racingline` |
| `racingline-dev` | 启动 Rearview server、portfolio worker 和前端 | 保留，指向重命名后的 `app/racingline` |
| `racingline-new-rearview-dev` | 并行新工程复验入口 | 删除或改为临时兼容 alias；完成标准中不再作为正式入口 |

### 文档引用

必须更新的当前事实入口：

| 文档 | 当前问题 | 处理 |
|---|---|---|
| `AGENTS.md` | Racingline 路径写作 `app/racingline/`，但运行说明仍受系统地图影响 | 保持路径，必要时更新 `make racingline-dev` 描述 |
| `docs/systems/racingline.md` | 仍以旧 `app/racingline/` 第一版创建口径描述验证要求 | 更新为重命名后的唯一前端入口和新验证命令 |
| `docs/systems/README.md` | Racingline 当前代码根已是 `app/racingline/`，不需结构性变化 | 如角色描述需要从旧选股工作台改成策略工作台则同步 |
| `docs/systems/racingline.md` | 同时列出 `app/racingline/` 和 `app/racingline_new/`，运行入口也包含 `racingline-new-rearview-dev` | 收敛为单代码根 `app/racingline/` |
| `docs/systems/rearview.md` | 仍列出 `make racingline-new-rearview-dev` | 删除或改为已归并说明 |
| `docs/plans/0041-*.md` | active plan 指向旧 `app/racingline/` 的 virtual account/portfolio 路径 | 重新评估是否 Superseded；若仍活跃，改写为新正式 `app/racingline/` 目标 |
| `docs/ADR/0013-*.md` | 当前 Proposed 决策不接受 `racingline_new` UI 栈直接升级为正式栈 | 实施前必须更新 ADR 结论或补新 ADR |
| `docs/RFC/0023-*.md` | 待决问题仍问是否重命名 | 更新状态或追加结论：采用重命名为 `app/racingline/` |

允许保留历史路径的文档：

1. `docs/jobs/reports/**` 中的历史验收报告。
2. `docs/plans/archive/**` 中已完成或归档计划。
3. `docs/debt/archive/**` 中已 resolved 的漂移记录，除非它们被当前系统地图引用为当前事实。
4. 历史 RFC 中作为背景或迁移来源的 `app/racingline_new/` 描述，但需要避免“当前仍并行”的误导性语句。

## 实施阶段

### 阶段 0：冻结和预检查

目标：确认替换前基线干净，避免删除用户未提交工作或把 ignored 产物带入重命名。

步骤：

1. 确认工作树状态：

```bash
git status --short
```

2. 记录 tracked 文件和当前引用：

```bash
git ls-files app/racingline app/racingline_new
rg -n "racingline_new|racingline-new|RACINGLINE_NEW|RACINGLINE_APP_DIR|app/racingline_new|app/racingline/" --glob '!app/**/node_modules/**' --glob '!app/**/dist/**'
```

3. 清理 ignored 前端产物，只清理前端生成目录：

```bash
rm -rf app/racingline/node_modules app/racingline/dist app/racingline/.playwright-cli
rm -rf app/racingline_new/node_modules app/racingline_new/dist app/racingline_new/.playwright-cli
```

4. 在删除旧工程前保存一次新工程质量门禁结果：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

完成标准：

1. `git status --short` 中没有未解释的用户改动。
2. `app/racingline_new` 的 lint、typecheck、test、build 通过。
3. `rg` 输出已归类为当前必须更新、历史允许保留或实现阶段删除。

### 阶段 1：技术栈和 ADR 决策收敛

目标：不要把 `racingline_new` 当前 UI 栈“静默升级”为正式栈。

步骤：

1. 对比并确认新正式 Racingline 要采用的 stack：
   - `components.json`: `base-lyra`、`taupe`、`lucide`、`menuColor: inverted`。
   - `package.json`: 新工程包含 `lucide-react`、`@fontsource-variable/ibm-plex-sans`，同时仍包含 `@hugeicons/*`。
   - 源码：新工程存在 Lucide 和 Hugeicons imports。
2. 若接受新工程现状为正式栈，更新 `docs/ADR/0013-racingline-ui-stack-variant-evaluation.md`，把结论从 Proposed 的“不接受直接升级”改为新的 Accepted/Superseded 结论，并说明本计划是替换决策来源。
3. 若不接受新工程 UI 栈，先在 `app/racingline_new` 内恢复 ADR 0011 要求，再进入删除和重命名。

本计划默认走第 2 条：接受当前新工程作为正式 Racingline 前端，并用 ADR 更新记录这次技术栈变化。

完成标准：

1. ADR 不再与“把 `racingline_new` 重命名为正式 `racingline`”冲突。
2. `docs/systems/racingline.md` 的技术栈摘要能链接到更新后的 ADR 结论。

### 阶段 2：删除旧 `app/racingline/`

目标：清除作废旧工程，不保留双 Racingline 前端。

步骤：

1. 删除旧 tracked 文件：

```bash
git rm -r app/racingline
```

2. 确认旧路径不存在 tracked 文件：

```bash
git ls-files app/racingline
```

3. 不从旧 `app/racingline/` 迁移以下内容，除非阶段 0 发现新工程缺少等价能力：
   - 旧 `/runs`、`/rules`、`/metrics`、`/portfolios` 路由。
   - 旧 `src/store/workbench.ts`。
   - 旧 rule set/version/run/portfolio-run API wrappers。
   - 旧 Hugeicons-only UI 组件实现。

完成标准：

1. `app/racingline/` 旧 tracked 文件已从 index 删除。
2. 删除清单在 review 中可解释，不存在误删 `app/racingline_new/` 或其他系统文件。

### 阶段 3：重命名 `app/racingline_new/` 为 `app/racingline/`

目标：把新工程变成唯一正式 Racingline 前端目录。

步骤：

1. 移动 tracked 文件：

```bash
git mv app/racingline_new app/racingline
```

2. 更新工程元数据：
   - `app/racingline/package.json`: `"name": "racingline"`。
   - `app/racingline/package-lock.json`: 顶层 `"name"` 和 packages root `"name"` 改为 `"racingline"`。
   - `app/racingline/package.json`: 评估是否补回旧工程的 `"packageManager": "npm@11.13.0"`；若保留 npm 版本约束，lockfile 应由同版本 npm 更新。
   - `app/racingline/README.md`: 删除 Vite template 文案，改成 Racingline 本地运行和质量门禁说明，或删除该 README 由系统地图承载。
   - `app/racingline/public/vite.svg`、`src/assets/react.svg`: 若未被引用，删除模板残留资产。
3. 重新生成 lockfile 元数据：

```bash
cd app/racingline
npm install --package-lock-only
```

4. 搜索并修正源代码、配置和文档中的当前路径：

```bash
rg -n "racingline_new|racingline-new|RACINGLINE_NEW|app/racingline_new" --glob '!docs/jobs/reports/**' --glob '!docs/plans/archive/**'
```

完成标准：

1. `app/racingline_new/` 不再存在 tracked 文件。
2. `app/racingline/` package name 和 lockfile name 为 `racingline`。
3. 模板 README 和未引用模板资产已处理。
4. 当前代码、Makefile、系统地图和 active plan 中没有 `racingline_new` 当前事实引用。

### 阶段 4：Makefile 和运行入口收敛

目标：让正式开发命令只指向 `app/racingline/`。

步骤：

1. 修改 Makefile：
   - `RACINGLINE_APP_DIR ?= app/racingline`
   - 删除 `RACINGLINE_NEW_APP_DIR`
   - 删除 `.PHONY` 中的 `racingline-new-rearview-dev`，或保留为短期 alias 并打印 deprecation message。
   - 更新 help 文案，去掉 “New”。
   - `racingline-dev` 和 `racingline-frontend-dev` 均使用 `$(RACINGLINE_APP_DIR)`。
2. 如保留 alias，alias 必须调用 `racingline-dev`，不得再引用 `app/racingline_new`。

完成标准：

1. `rg -n "RACINGLINE_NEW|racingline-new|app/racingline_new" Makefile` 无命中，或仅有明确 deprecation alias。
2. `make racingline-frontend-dev` 和 `make racingline-dev` 日志显示从 `app/racingline` 启动。

### 阶段 5：文档当前事实收敛

目标：让文档入口不再把旧工程或并行工程描述为当前事实。

步骤：

1. 更新 `docs/systems/racingline.md`：
   - 代码根收敛为单一 `app/racingline/`。
   - 状态改为新策略工作台已替代旧工程。
   - 路由职责改为 `/dashboard`、`/dashboard/strategies/:portfolioId`、`/strategies`。
   - 接口依赖改为 strategy preview、strategy backtest、strategy portfolio 和 security analysis API。
   - 运行命令去掉 `make racingline-new-rearview-dev`。
   - 质量门禁只保留 `cd app/racingline && npm run lint && npm run typecheck && npm test && npm run build`。
2. 更新 `docs/systems/racingline.md`：
   - 不再描述 `app/racingline/` “创建前/第一版”。
   - 验证要求只指向重命名后的正式目录。
3. 更新 `docs/systems/rearview.md`：
   - 删除只启动 `app/racingline_new` 的说明。
   - 保留 `make racingline-dev` 作为 Rearview + Racingline 联调入口。
4. 更新 `docs/RFC/0023-racingline-frontend-prototype-led-development.md`：
   - 在迁移和替换策略处追加结论：采用重命名为 `app/racingline/`。
   - 若不继续作为活跃流程，可标明已完成本轮替换。
5. 更新 `docs/plans/0041-*.md`：
   - 若虚拟账户旧 portfolio run 路径被新 strategy portfolio 流程替代，标记 Superseded 并归档或另立迁移说明。
   - 若仍保留，改写为重命名后的 `app/racingline/` 目标，不得引用旧页面事实。
6. 更新 `AGENTS.md` 中 Racingline 运行入口，如当前系统地图和 Makefile 说明发生变化。
7. 更新 `docs/plans/README.md`：
   - 本计划保持 active，实施完成后移入 archive 并在 Recently Completed 记录验收报告。

完成标准：

1. 当前事实入口只指向 `app/racingline/`。
2. `rg -n "app/racingline_new|racingline_new|racingline-new"` 的非历史命中已清零或明确标记历史。
3. 文档状态与 ADR、系统地图和 Makefile 一致。

### 阶段 6：质量门禁和浏览器验收

目标：证明重命名没有破坏正式前端和联调入口。

前端静态门禁：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

文档门禁：

```bash
make docs-check
git diff --check
```

联调 smoke：

```bash
make racingline-dev
```

浏览器验收使用共享 CDP：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

至少检查：

1. `http://127.0.0.1:5173/dashboard` 可加载，并调用 `GET /rearview/strategy-portfolios/dashboard`。
2. `/strategies` Step 1 可加载 metric catalog，调用 `GET /rearview/metrics`。
3. Step 1 explain 调用 `POST /rearview/explain`。
4. Step 3 preview 调用 strategy preview/timeline/pool-page/security analysis 相关接口。
5. Step 4 点击进入回测时调用 `POST /rearview/strategy-backtests/validate`。
6. Step 5 create/poll/result 查询调用 strategy backtest APIs。
7. 发布组合后返回 `/dashboard`，新组合来自 strategy portfolio APIs。
8. 桌面和移动视口没有页面级横向溢出、主要按钮文字溢出或遮挡。

完成标准：

1. 所有前端静态门禁通过。
2. `make docs-check` 和 `git diff --check` 通过。
3. live smoke 覆盖 dashboard、strategies、strategy detail 的真实 Rearview API 请求。
4. 若 live smoke 因本地依赖或外部状态失败，必须在 job report 记录命令、失败点、已验证项和后续阻塞。

### 阶段 7：验收报告和归档

目标：把实际删除、重命名和验证结果沉淀为可追溯事实。

步骤：

1. 新增 job report：

```text
docs/jobs/reports/YYYY-MM-DD-racingline-legacy-cleanup-rename.md
```

2. 报告至少包含：
   - 删除旧工程和重命名命令摘要。
   - `rg` 清理结果。
   - 前端 lint/typecheck/test/build 输出摘要。
   - `make docs-check` 和 `git diff --check` 输出摘要。
   - live smoke 的 Rearview API 请求和页面范围。
   - 截图或 Playwright CDP 证据路径。
3. 将本 plan 移入 `docs/plans/archive/`，状态改为 Completed。
4. 更新 `docs/plans/README.md` 的 Recently Completed。

完成标准：

1. job report 可追溯实际执行事实。
2. active plans 顶层不再保留已完成计划。
3. 系统地图链接到验收报告。

## 禁止模式

1. 禁止同时保留 `app/racingline/` 旧工程和 `app/racingline_new/` 新工程作为当前事实。
2. 禁止在代码中写 `racingline_new || racingline`、路径候选轮询或双目录兼容逻辑。
3. 禁止前端绕过 Rearview 直接访问 PostgreSQL、ClickHouse、NATS、dbt 或本地数据文件作为正式数据来源。
4. 禁止用 mock、fixture 或静态 fallback 掩盖 Rearview API 失败。
5. 禁止把 ADR 0013 的冲突留到重命名后再处理。
6. 禁止改写历史 job report 使其看起来是当前事实；历史事实只做状态说明。

## 最小验证命令

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

```bash
make docs-check
git diff --check
```

涉及 live smoke 时追加：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

如果实现中修改 Rearview API 或 Rust worker，追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## 完成标准

1. `app/racingline_new/` 已不存在。
2. `app/racingline/` 是新策略工作台代码，package name 和 lockfile name 均为 `racingline`。
3. Makefile 默认前端目录为 `app/racingline`，没有正式命令指向 `app/racingline_new`。
4. 当前系统地图、intake、AGENTS 和 Rearview 运行说明只引用正式 `app/racingline/`。
5. `racingline_new` 只存在于历史文档、归档材料或明确的迁移说明中。
6. 前端 lint、typecheck、test、build 通过。
7. docs-check 和 diff whitespace 检查通过。
8. live smoke 或 job report 证明 `/dashboard`、`/strategies` 和 strategy detail 仍能通过 Rearview 真实接口工作。
