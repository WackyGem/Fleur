# Plan 0071: Racingline 策略详情页删除按钮实施计划

日期：2026-07-02

状态：Completed

完成日期：2026-07-02

领域：racingline, rearview

关联系统：racingline, rearview, PostgreSQL rearview

代码根：

- `app/racingline/`
- `engines/crates/rearview-core/`

关联文档：

- [RFC 0043: Racingline 策略详情页删除按钮功能完善](../../RFC/archive/0043-racingline-strategy-detail-delete-action.md)
- [Racingline 系统地图](../../architecture/racingline.md)
- [Rearview 系统地图](../../architecture/rearview.md)
- [RFC 0029: Racingline 策略组合发布和日运行](../../RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md)
- [RFC 0036: Racingline 策略组合详情页对账单](../../RFC/0036-racingline-strategy-portfolio-statement.md)
- [验收报告：2026-07-02 Racingline 策略详情页删除按钮](../../jobs/reports/2026-07-02-racingline-strategy-detail-delete-action.md)

## 背景

RFC 0043 已确定策略详情页“删除”按钮第一版语义：

1. 不新增 `DELETE /rearview/strategy-portfolios/{id}` alias。
2. 删除按钮调用现有 `PATCH /rearview/strategy-portfolios/{id}`，body 为 `{ "status": "archived" }`。
3. 删除是软删除/归档，只修改 PostgreSQL `strategy_portfolio.status` 和 `archived_at`，不删除 ClickHouse backtest/live facts。
4. 归档后直接访问详情接口返回 `410 Gone`。
5. 前端遇到 archived detail 的 `410 Gone` 后跳转回 `/dashboard`。
6. 手动指定 archived portfolio 创建 daily run 时返回 `410 Gone`，不允许继续生成新的 live facts。
7. 暂不提供已归档策略入口和恢复能力。

当前前端已经有删除按钮和确认弹层，但确认按钮只关闭弹层，没有调用 Rearview API，也没有 pending/error 状态、缓存失效或跳转。Rearview 已有 repository 级 `archive_strategy_portfolio()`，但当前 API detail 仍会返回 archived record；`RearviewError` 也还没有 `410 Gone` 对应错误类型。

## 目标

1. 接通详情页删除按钮到 Rearview archive API。
2. 删除成功后跳转 `/dashboard`，并刷新 dashboard 列表。
3. 删除弹层具备 pending、失败提示和重复点击保护。
4. Rearview 对 archived portfolio detail 返回 `410 Gone`。
5. Rearview 拒绝手动为 archived portfolio 创建 daily run，返回 `410 Gone`。
6. 保持删除动作为软删除，不清理历史事实数据。
7. 补齐前端和后端测试，覆盖 archive contract、410 跳转和失败状态。

## 非目标

1. 不新增 `DELETE /rearview/strategy-portfolios/{id}`。
2. 不实现 archived portfolio 列表、恢复按钮或审计详情页。
3. 不删除 `strategy_portfolio_daily_run`、daily outbox、source backtest run 或 ClickHouse facts。
4. 不引入权限、用户隔离或删除审批。
5. 不修改 strategy backtest、portfolio worker、dbt 或 Dagster 清算链路。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| 前端详情页 | `app/racingline/src/routes/strategy-detail-page.tsx` 已显示删除按钮和确认弹层；确认按钮是 `DialogClose`，没有 mutation。 |
| 前端 API | `app/racingline/src/api/rearview.ts` 没有 `archiveStrategyPortfolio()`；`hooks.ts` 没有 archive mutation。 |
| 前端 cache keys | `app/racingline/src/api/queryKeys.ts` 已有 dashboard、detail、nav、performance、virtual-account、statement、signals、signal-timeline、positions、rebalance-records keys。 |
| 前端错误类型 | `ApiError` 暴露 `status`、`errorType` 和 message，可识别 HTTP 410。 |
| 后端 route | `PATCH /rearview/strategy-portfolios/{strategy_portfolio_id}` 已绑定 `patch_strategy_portfolio()`。 |
| 后端 archive repository | `archive_strategy_portfolio()` 会把 `status` 设为 `archived`，`archived_at = coalesce(archived_at, now())`。 |
| Dashboard 查询 | `list_active_strategy_portfolios()` 只返回 `status = 'active'`。 |
| Detail 查询 | `get_strategy_portfolio()` 当前按 id 返回 record，没有过滤 archived。 |
| Daily run 创建 | `create_strategy_portfolio_daily_runs_for_trade_date(Some(id))` 当前先 `get_strategy_portfolio(id)`；如果 detail 改为拒绝 archived，需要确认该路径也返回 410。 |
| 错误模型 | `RearviewError` 当前有 `NotFound`、`Conflict`、`PortfolioPendingFirstRun`、`Validation` 等，没有 `Gone`。 |

## 实施阶段

### Phase 0: API 和错误语义审计

目标：确认 410 能用单一后端错误类型表达，并确认 archived 检查只来自 `strategy_portfolio.status`。

实施项：

1. 审计 `engines/crates/rearview-core/src/error.rs`，新增 `RearviewError::Gone(String)`：
   - `status_code()` 返回 `StatusCode::GONE`。
   - `error_type()` 返回 `"gone"`。
   - message 明确包含 archived portfolio id。
2. 审计 `get_strategy_portfolio()`、`patch_strategy_portfolio()`、`create_strategy_portfolio_daily_runs_for_trade_date()` 的调用链。
3. 决定 helper 落点：优先新增小 helper，例如 `ensure_strategy_portfolio_active(record)`，避免多个调用点手写 `status == "archived"`。

测试策略：

1. Rust 单测覆盖 `RearviewError::Gone` 的 status code 和 error type。

完成标准：

1. 410 错误类型在后端有唯一表达。
2. archived 判断只依据 PostgreSQL record 的 `status` 字段。

### Phase 1: Rearview archive contract 补强

目标：后端保留 PATCH archive 写入口，并补齐 archived 读/运行拒绝语义。

实施项：

1. 保持 route 不变：`PATCH /rearview/strategy-portfolios/{id}`。
2. `patch_strategy_portfolio()` 继续只接受 `{ "status": "archived" }`，其他 status 返回 validation error。
3. `get_strategy_portfolio()` 获取 record 后，如果 `status == "archived"`，返回 `RearviewError::Gone(...)`。
4. `create_strategy_portfolio_daily_runs_for_trade_date(Some(id))` 如果指定 portfolio 是 archived，返回 `RearviewError::Gone(...)`，不创建 daily run、不写 outbox。
5. 不改变 `list_active_strategy_portfolios()`；dashboard 继续只读 active。

测试策略：

1. Rust 测试：归档 active portfolio 成功，返回 record 中 `status = "archived"`，`archived_at` 非空。
2. Rust 测试：重复归档同一个 portfolio 不改变首次 `archived_at` 语义。
3. Rust 测试：PATCH 非 `archived` status 返回 validation error。
4. Rust 测试：归档后 detail 返回 `410 Gone` / `error_type = "gone"`。
5. Rust 测试：手动指定 archived portfolio 创建 daily run 返回 `410 Gone`，且不创建 outbox。

完成标准：

1. 后端 archive 写入口可用且幂等。
2. archived portfolio 不再可通过 detail API 当作 active detail 使用。
3. archived portfolio 不再能被手动生成新的 daily run。

### Phase 2: Racingline API 和 hook 接入

目标：前端 API 层提供 archive mutation，并让详情页能够使用 query cache。

实施项：

1. 在 `app/racingline/src/api/rearview.ts` 新增 `archiveStrategyPortfolio(strategyPortfolioId)`：
   - URL：`/rearview/strategy-portfolios/${strategyPortfolioId}`。
   - method：`PATCH`。
   - body：`{ status: "archived" }`。
2. 在 `app/racingline/src/api/hooks.ts` 新增 `useStrategyPortfolioArchiveMutation()`。
3. 必要时在 `queryKeys.ts` 新增 helper，或直接复用已有 strategy portfolio keys。

测试策略：

1. 前端 API 测试覆盖 `archiveStrategyPortfolio()` 使用 PATCH 和正确 body。
2. Hook 测试如现有测试体系支持，则覆盖 mutation 调用函数。

完成标准：

1. 前端删除按钮不直接拼 fetch；只通过 API/helper hook 调用 Rearview。
2. archive API contract 在前端测试中固定。

### Phase 3: 详情页删除交互

目标：把当前只关闭弹层的按钮改为真实归档操作。

实施项：

1. 在 `StrategyDetailPage` 使用 `useNavigate()` 和 `useQueryClient()`。
2. 用 controlled dialog state 管理删除弹层，不再让确认按钮仅靠 `DialogClose` 关闭。
3. 点击确认时调用 `archiveMutation.mutateAsync(strategyPortfolioId)`。
4. pending 时：
   - 禁用删除触发按钮、取消按钮和确认按钮。
   - 确认按钮文案显示“删除中”。
5. 成功时：
   - invalidate `queryKeys.strategyPortfolioDashboard()`。
   - remove 或 invalidate 当前 detail 和 live 子资源 cache。
   - 导航到 `/dashboard`。
6. 失败时：
   - 保持弹层打开。
   - 展示 `ApiError.message` 或 fallback `删除失败，请稍后重试。`。
7. 确认弹层文案改为：`删除后该策略将从看板移除，历史回测和运行记录会保留。`

测试策略：

1. 前端组件测试：点击删除 -> 确认 -> 调用 archive mutation。
2. 前端组件测试：pending 时按钮禁用。
3. 前端组件测试：成功后跳转 `/dashboard` 并刷新 dashboard query。
4. 前端组件测试：失败时弹层保持打开并展示错误。

完成标准：

1. 用户不会再遇到“确认删除但没有任何后端效果”的假成功。
2. 成功和失败状态都可见且可测试。

### Phase 4: Archived detail 410 前端处理

目标：用户打开旧详情链接时回到 dashboard，不展示 archived detail。

实施项：

1. 在 `StrategyDetailPage` 识别 `portfolioQuery.error`：
   - `ApiError.status === 410` 时导航 `/dashboard`。
   - 可复用现有 toast 机制或最小页面状态提示“该策略已删除”。
2. 不为 archived portfolio 渲染 detail 内容。
3. 404 继续使用当前“未找到策略”空态或按文案更新为“策略不存在或链接无效”。

测试策略：

1. 前端组件测试：detail query 返回 410 时触发 dashboard 导航。
2. 前端组件测试：404 不触发同一套 archived 跳转逻辑，仍展示 not found 空态。

完成标准：

1. archived 直链不会展示旧详情。
2. 410 和 404 在前端行为上可区分。

### Phase 5: 文档和架构事实收敛

目标：实现完成后把长期事实写回当前架构入口，并记录验收。

实施项：

1. 更新 `docs/architecture/racingline.md`：
   - 策略详情页删除按钮接入 archive API。
   - archived detail 410 后回 dashboard。
2. 更新 `docs/architecture/rearview.md`：
   - Strategy Portfolio archive 语义。
   - archived detail 和 manual daily run 返回 410。
3. 新增 job report：
   - `docs/jobs/reports/2026-07-02-racingline-strategy-detail-delete-action.md`。
   - 记录命令、范围、测试结果和未执行的检查。
4. 完成后归档本计划到 `docs/plans/archive/0071-...md`，并更新 `docs/plans/README.md`。

测试策略：

1. `make docs-check`
2. `git diff --check`

完成标准：

1. 当前架构文档能反映删除按钮实际语义。
2. 验收报告能追溯测试和行为结果。
3. active plan 不长期滞留顶层。

## 禁止模式

1. 禁止物理删除 ClickHouse facts 或 PostgreSQL daily run/outbox。
2. 禁止新增 `DELETE` route。
3. 禁止前端本地把策略从 UI 里过滤掉但不调用后端 archive。
4. 禁止前端在 410 后继续渲染 archived detail。
5. 禁止用 `404` 混淆 archived 状态；本计划要求 archived detail 返回 `410 Gone`。
6. 禁止为 archived portfolio 手动创建新的 daily run。

## 允许保留的例外

1. 归档后的历史 backtest/live facts 保留在 ClickHouse，可供后续审计或维护工具使用。
2. `PATCH status=archived` 继续作为第一版唯一写入口。
3. `archive_strategy_portfolio()` 可保持重复调用幂等，`archived_at` 保留首次归档时间。
4. 第一版不提供恢复能力，也不展示 archived portfolio 列表。

## 最小验证命令

后端：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p rearview-core
```

前端：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

文档：

```bash
make docs-check
git diff --check
```

## 完成标准

1. 删除按钮真实归档策略组合。
2. 删除成功后 dashboard 不再展示该策略。
3. archived detail 返回 410，前端跳回 dashboard。
4. archived portfolio 手动 daily run 创建返回 410。
5. 历史 backtest/live facts 没有被删除。
6. 前后端测试和文档检查通过，或在 job report 中记录未运行原因。
7. 架构文档和 job report 已更新，本计划已归档。
