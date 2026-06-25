# RFC 0023: Racingline 前端原型驱动开发流程

状态：Accepted（2026-06-25；Plan 0053 已采用重命名替换路径）
领域：racingline
关联系统：racingline
代码根：app/racingline/
需求入口：docs/intake/racingline.md

## 摘要

本文档定义 Racingline 前端下一阶段开发流程：前端 UX、样式和交互不完全适合沿用数据任务和后端任务的"先写完整执行计划，再交由 Agent 一次性完成"模式。前端页面的布局密度、视觉层级、响应式表现、空状态、交互反馈和实际可用性，需要在浏览器中多轮调试才能判断。

因此，Racingline 前端允许在 RFC 阶段编写低风险交互原型代码。原型代码用于验证导航、布局、组件组合、响应式和关键交互，不等同于生产实现；只有当原型结论被沉淀回 RFC、计划和验收标准后，才进入正式实施。

## 背景

mono-fleur 的数据平台、dbt、Dagster、Rust 后端和数据契约任务，通常可以用较清晰的输入、输出、schema、状态机和质量门禁描述。适合的开发模式是：

1. 需求进入 intake。
2. 复杂需求写 RFC。
3. 已确定方案写 plan。
4. Agent 按计划完成代码、测试和验证。

前端页面不同。即使用户故事和路由已经清楚，也很难只靠文档判断：

- 首屏信息是否过载。
- 导航是否清晰。
- 表格、卡片、抽屉、tab 和工具栏组合是否顺手。
- 空状态、加载态、错误态是否压迫主流程。
- 桌面和移动端是否都能扫描。
- shadcn preset、字体、间距和颜色是否适合当前产品。

这些问题往往必须通过可运行页面、截图和实际点击才能收敛。2026-06-17 到 2026-06-25 期间，Racingline 使用 `app/racingline_new/` 作为并行重构工程验证新策略工作台；Plan 0053 已删除旧工程并将新工程重命名为正式 `app/racingline/`。

关联文档：

- [Q&A 0003: Racingline 策略实验室两入口导航与首屏承载](../Q&A/0003-racingline-strategy-lab-two-entry-navigation.md)
- [RFC 0019: Racingline Rearview 前端工作台](0019-racingline-rearview-frontend-workbench.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](0021-racingline-virtual-account-portfolio-rebalancing.md)
- [ADR 0011: Racingline 前端技术栈和工程边界](../ADR/0011-racingline-frontend-technology-stack.md)
- [System: Racingline](../systems/racingline.md)

## 目标

1. 允许 Racingline 前端在 RFC 阶段编写可运行的轻量原型代码，用于验证 UX 判断。
2. 明确原型代码和生产实现的边界，避免原型无约束沉淀为正式代码。
3. 把浏览器调试、截图、响应式检查和用户反馈纳入 RFC 收敛过程。
4. 在替换完成前保持原型工程与生产入口并行，直到新体验被确认可替换。
5. 让正式 plan 从已验证的页面骨架、交互结论和验收标准出发，而不是从纯文字假设出发。
6. 原型工程不是静默重新选择前端技术栈的入口；若原型技术栈升级为正式栈，必须通过 ADR 记录。

## 非目标

1. 不取消 RFC、plan、ADR 和质量门禁。
2. 不允许在 RFC 阶段绕过 shadcn/ui 基础组件边界。
3. 不允许原型代码直接替换生产入口。
4. 不要求所有前端任务都必须先做原型；只有布局、交互、视觉和信息架构不确定时才使用。
5. 不在前端原型中实现 Rearview 后端状态机、ClickHouse 查询、PostgreSQL 写入或权威计算。

## 流程决策

### D0: 技术选型必须由 ADR 收敛

原型工程不是第二个产品线，也不是静默重新选型工程。正式实现默认继承 [ADR 0011](../ADR/0011-racingline-frontend-technology-stack.md) 的工程边界；如果原型引入新的 shadcn style、主题、字体或图标策略，必须另起 ADR 或更新既有 ADR 后才能进入正式工程。

2026-06-25 的结论见 [ADR 0013](../ADR/0013-racingline-ui-stack-variant-evaluation.md)：`base-lyra`、`taupe`、IBM Plex Sans、Lucide 与 Hugeicons 并存已被接受为正式 Racingline UI 栈。

| 类别 | 必须保持一致 |
|---|---|
| 构建工具 | Vite |
| 前端框架 | React |
| 开发语言 | TypeScript |
| 样式体系 | Tailwind CSS v4 + CSS Variables |
| UI 组件体系 | shadcn/ui + Base UI |
| 图标 | ADR 0013 记录当前正式策略 |
| 路由 | React Router |
| 服务端状态 | TanStack Query |
| 客户端状态 | Zustand |
| 图表 | TradingView Lightweight Charts |

RFC 阶段可以临时比较字体、主题、密度或组件外观，但这些实验只属于视觉原型材料。任何改变 shadcn style、primitive base、icon library、状态管理、路由、图表库或 package 管理方式的决定，都必须进入 ADR，不能只靠原型脚手架结果自然沉淀。

### D1: 前端 RFC 可以包含原型循环

前端 UX RFC 可以包含一个或多个原型循环：

1. 写出用户故事、首屏任务和导航假设。
2. 在隔离的原型目录或临时 route 中实现最小可运行交互骨架。
3. 启动 dev server，用浏览器检查桌面和移动端。
4. 截图或记录观察结果。
5. 根据反馈调整布局、信息层级和交互。
6. 把最终结论写回 RFC。

原型循环的产出不是"完成代码"，而是：

- 页面职责确认。
- 导航和路由确认。
- 组件边界确认。
- 交互状态确认。
- 响应式和视觉约束确认。
- 正式实施计划的验收标准。

### D2: 原型代码必须低风险隔离

历史上本轮 Racingline 原型代码放在：

```text
app/racingline_new/
```

原型不得直接修改 `app/racingline/` 的生产入口。只有当 RFC、ADR 和实施计划确认替换路径后，才允许把原型作为新生产前端的来源。Plan 0053 已采用并完成 `app/racingline_new/` 重命名为 `app/racingline/` 的路径。

如果后续需要进一步隔离，可以在临时 route 或独立原型目录中构建原型页面；进入正式实现时再移动到 `src/routes/`、`src/components/racingline/` 和 `src/features/`。

### D3: 原型不能改写基础 UI 组件

原型阶段仍遵守 Racingline 组件边界：

- `src/components/ui/` 只放 shadcn CLI 生成的基础组件。
- 业务组件放在 `src/components/racingline/` 或 `src/features/<domain>/components/`。
- 不手工改写 shadcn 基础组件实现。
- 需要新增基础组件时使用 `npx shadcn@latest add ...`。
- 原型业务组件默认使用与 `racingline` 一致的 icon library 和 primitive 约定；如临时实验不同 preset 或图标库，必须在 RFC 中标记为实验，正式实现前恢复或通过 ADR 裁决。

原型可以组合基础组件、写业务组件、写 mock 数据和本地状态，但不应把业务语义塞进 `src/components/ui/`。

### D4: 原型优先使用 mock 数据

当后端 API 尚不完整时，原型优先使用前端本地 mock 数据验证交互。mock 数据必须明显标记，并集中放置，例如：

```text
src/mocks/
```

mock 数据只用于验证页面结构、状态和交互，不代表最终 API contract。若原型发现需要后端补接口，应回写 RFC 的"后端 API 缺口"章节，并同步关联 Rearview 文档。

### D5: 原型完成不等于正式完成

一个原型只有在满足以下条件时，才能进入正式实施计划：

1. RFC 已写明最终页面职责、导航、关键组件和状态。
2. RFC 已写明哪些原型代码保留、重写或删除。
3. RFC 已写明 API 依赖和 mock 到真实数据的迁移方式。
4. 已定义最小验收：lint、typecheck、build、浏览器截图和响应式检查。
5. 用户确认原型方向可继续推进。

正式计划仍应拆分阶段，至少区分：

- 页面骨架和导航。
- mock 数据交互。
- API 接入。
- 状态、错误和空态。
- 浏览器验收。
- 替换旧工程或迁移入口。

### D6: 浏览器验收是前端 RFC 的一等输入

前端 RFC 和计划应明确浏览器验收方式。Racingline 默认使用本地 dev server 和 Playwright / CDP 检查：

```bash
cd app/racingline_new
npm run dev
npm run lint
npm run typecheck
npm run build
```

需要浏览器调试时使用仓库现有入口：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

验收至少覆盖：

- 桌面首屏。
- 窄屏或移动端。
- 空状态。
- 加载状态。
- 错误状态。
- 主操作路径。
- 控制台错误。

### D7: 用户反馈优先进入 RFC，再进入 plan

用户对交互和视觉的反馈不应只停留在聊天上下文。稳定反馈应进入 RFC 或 Q&A：

- 页面方向、导航收敛、首屏职责：进入 RFC 或 Q&A。
- 长期工程边界：进入 ADR 或 systems。
- 具体执行阶段：进入 plan。
- 实际验收结果：进入 jobs report。

## Racingline 两入口原型的适用方式

Q&A 0003 已确认 `看板` 和 `策略` 两个主入口。下一阶段可以先在 `app/racingline_new/` 编写一个两入口原型：

| 页面 | 原型目标 | 不做 |
|---|---|---|
| `看板` | 验证盘后总览、今日行动、策略表现、异常提醒的信息层级 | 不接真实行情和权威收益计算 |
| `策略` | 验证策略列表、草稿入口、规则编辑入口、虚拟账户 tab 的组织方式 | 不实现完整规则引擎和真实提交 |

该原型应优先验证：

1. 两入口导航是否足够。
2. 首屏是否能回答"今天该看什么"。
3. 策略页是否能承接创建、编辑、草稿、虚拟账户和回测入口。
4. 当前 `racingline` 技术栈和组件体系是否足以承载新的信息架构；如果视觉 preset 或图标库实验效果更好，只能作为 ADR 讨论输入，不能默认成为正式技术选型。

## 质量门禁

RFC 阶段原型至少运行：

```bash
cd app/racingline_new
npm run typecheck
npm run build
```

正式实施阶段追加：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm run build
```

如果接入真实 API，还必须追加相关 Rearview 后端测试或 smoke 验证。前端仍不得直接访问 ClickHouse 或 PostgreSQL。

## 迁移和替换策略

本轮 `app/racingline_new/` 已按 Plan 0053 重命名为正式 `app/racingline/`。后续若再次使用原型工程，替换生产入口前必须完成：

1. 新工程主要页面和交互验收通过。
2. 旧工程需要保留的 API 类型、工具函数和业务逻辑已迁移或明确丢弃。
3. `docs/systems/racingline.md` 更新代码根和运行命令。
4. `Makefile` 或相关 dev 命令切换到新工程。
5. 旧工程归档、删除或重命名策略明确。

## 待决问题

1. 已决：`app/racingline_new/` 重命名为 `app/racingline/`。
2. 原型阶段是否需要专门的 screenshot report 模板？
3. 已决：UI 栈差异由 ADR 0013 接受为正式栈。
4. mock 数据是否需要建立统一 schema，还是按页面就近维护？

## 相关文档

- [docs/intake/racingline.md](../intake/racingline.md)
- [docs/systems/racingline.md](../systems/racingline.md)
- [docs/Q&A/0003-racingline-strategy-lab-two-entry-navigation.md](../Q&A/0003-racingline-strategy-lab-two-entry-navigation.md)
- [docs/ADR/0011-racingline-frontend-technology-stack.md](../ADR/0011-racingline-frontend-technology-stack.md)
- [docs/ADR/0013-racingline-ui-stack-variant-evaluation.md](../ADR/0013-racingline-ui-stack-variant-evaluation.md)
- [docs/plans/archive/0053-racingline-legacy-cleanup-and-rename-plan.md](../plans/archive/0053-racingline-legacy-cleanup-and-rename-plan.md)
