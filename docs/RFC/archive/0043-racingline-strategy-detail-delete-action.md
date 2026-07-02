# RFC 0043: Racingline 策略详情页删除按钮功能完善

状态：Implemented（2026-07-02）
日期：2026-07-02
领域：Racingline, Rearview, Strategy Portfolio, UX
关联系统：app/racingline, engines/crates/rearview-core, PostgreSQL rearview
相关文档：
- docs/architecture/racingline.md
- docs/architecture/rearview.md
- docs/RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md
- docs/RFC/0034-racingline-step5-portfolio-publish-dialog-tabs.md
- docs/RFC/0036-racingline-strategy-portfolio-statement.md

## 摘要

策略详情页当前已经有“删除”按钮和确认弹层，但确认按钮只关闭弹层，没有调用 Rearview API，也没有更新 dashboard 缓存、详情页状态或跳转。因此用户会以为删除成功，但策略仍然留在看板和后端控制面。

当前 Rearview 已具备软删除基础能力：`PATCH /rearview/strategy-portfolios/{strategy_portfolio_id}` 支持 `{ "status": "archived" }`，后端会把 `strategy_portfolio.status` 更新为 `archived` 并写入 `archived_at`。Dashboard 当前读取 `list_active_strategy_portfolios()`，只返回 `status = 'active'` 的组合。因此第一版应把前端删除按钮接到现有 archive 能力，并把产品文案定义为“从看板移除/归档”，不做物理删除。

## 当前事实

### 前端详情页已有确认 UI

`app/racingline/src/routes/strategy-detail-page.tsx` 当前在页面标题栏右侧展示：

```tsx
<Dialog>
  <DialogTrigger>
    <Trash2 />
    删除
  </DialogTrigger>
  <DialogContent>
    <DialogTitle>删除策略</DialogTitle>
    <DialogDescription>删除后该策略将从看板移除。</DialogDescription>
    ...
    <DialogClose>
      确认删除
    </DialogClose>
  </DialogContent>
</Dialog>
```

确认按钮没有绑定 mutation，也没有 pending/error 状态。点击确认只关闭弹层。

### 前端 API 层缺少 archive 方法和 mutation hook

`app/racingline/src/api/rearview.ts` 已有：

- `createStrategyPortfolio()`
- `getStrategyPortfolioDashboard()`
- `getStrategyPortfolio()`
- `listStrategyPortfolioNav()`
- `getStrategyPortfolioPerformance()`
- `getStrategyPortfolioVirtualAccount()`
- `getStrategyPortfolioStatement()`
- `listStrategyPortfolioSignals()`
- `listStrategyPortfolioSignalTimeline()`
- `listStrategyPortfolioPositions()`
- `listStrategyPortfolioRebalanceRecords()`

但没有 `archiveStrategyPortfolio()` 或 `deleteStrategyPortfolio()`。

`app/racingline/src/api/hooks.ts` 也没有对应 mutation hook。`queryKeys.ts` 已有 dashboard、detail 和各详情子资源 query keys，可以用于成功后精准失效或移除缓存。

### Rearview 已有软删除 API

`engines/crates/rearview-core/src/api/mod.rs` 当前 route：

```rust
.route(
    "/rearview/strategy-portfolios/{strategy_portfolio_id}",
    get(get_strategy_portfolio).patch(patch_strategy_portfolio),
)
```

`patch_strategy_portfolio()` 只接受：

```json
{ "status": "archived" }
```

其他 status 会返回 validation error。

### PostgreSQL repository 已实现归档

`archive_strategy_portfolio()` 当前执行：

```sql
update strategy_portfolio
set status = 'archived',
    archived_at = coalesce(archived_at, now()),
    updated_at = now()
where strategy_portfolio_id = $1
returning ...
```

`list_active_strategy_portfolios()` 使用：

```sql
where status = 'active'
```

因此归档后策略会从 dashboard 消失。

### 详情读取仍可读取 archived record

`get_strategy_portfolio()` 当前只按 `strategy_portfolio_id` 查询，没有过滤 `status = 'active'`。归档后的直接链接仍可能返回 record，并由 response wrapper 继续计算 `live_status`。这是当前实现事实，不应在前端删除按钮中用假设兜底。

## 目标

1. 让详情页“删除”按钮真正调用 Rearview archive API。
2. 删除成功后从看板移除策略，并把用户导航回 `/dashboard`。
3. 删除交互具备明确的确认、pending、失败和成功状态。
4. 保留历史控制面和 ClickHouse live/backtest facts，不做物理删除。
5. 不让前端自行修改权威状态；以 Rearview 返回的 archived record 为准。
6. 归档后的详情接口返回 `410 Gone`，前端遇到 410 后跳转回 `/dashboard`。

## 非目标

1. 不删除 `fleur_portfolio.live_*` 或 `fleur_backtest.backtest_*` facts。
2. 不删除 PostgreSQL `strategy_portfolio_daily_run`、daily outbox 或 source backtest run。
3. 不实现恢复 archived portfolio 的 UI。
4. 不新增认证、权限或多用户隔离语义。
5. 不把 archived portfolio 混回 dashboard 默认列表。
6. 第一版不新增 `DELETE /rearview/strategy-portfolios/{id}` alias。

## 语义选择

### 第一版采用软删除/归档

用户可见文案使用“删除”，但系统语义定义为“归档策略组合并从看板移除”。

原因：

1. 当前 Rearview 和 PostgreSQL 已有 `status = 'archived'`、`archived_at` 和 archive repository。
2. ClickHouse backtest/live facts 是历史运行事实，不适合作为 UI 删除动作的一部分被物理清理。
3. Dashboard 已按 active 过滤，归档天然满足“从看板移除”。
4. 归档操作可重复执行，`archived_at = coalesce(archived_at, now())` 保留首次归档时间。

### 第一版不新增 DELETE alias

第一版复用现有 PATCH contract，不新增 `DELETE /rearview/strategy-portfolios/{id}` alias：

```http
PATCH /rearview/strategy-portfolios/{id}
Content-Type: application/json

{ "status": "archived" }
```

理由是后端已经明确只支持 `status=archived` 的状态转换，且 repository 行为已存在。删除按钮不需要第二个 HTTP 入口才能完成闭环；减少 alias 可以避免后续出现 PATCH 和 DELETE 行为漂移。

## 前端设计

### API 函数

新增：

```ts
export function archiveStrategyPortfolio(strategyPortfolioId: string) {
  return requestJson<StrategyPortfolioRecord>(
    `/rearview/strategy-portfolios/${strategyPortfolioId}`,
    {
      body: JSON.stringify({ status: "archived" }),
      method: "PATCH",
    }
  )
}
```

如果希望更贴近用户动作，函数也可以命名为 `deleteStrategyPortfolio()`，但内部仍调用 PATCH archive。建议使用 `archiveStrategyPortfolio()`，避免把实现误读为物理删除。

### Mutation hook

新增：

```ts
export function useStrategyPortfolioArchiveMutation() {
  return useMutation({
    mutationFn: (strategyPortfolioId: string) =>
      archiveStrategyPortfolio(strategyPortfolioId),
  })
}
```

成功后由页面处理 query cache：

- invalidate `queryKeys.strategyPortfolioDashboard()`。
- remove 或 invalidate 当前 `queryKeys.strategyPortfolio(id)`。
- remove 或 invalidate 当前详情页 live 子资源 keys：nav、performance、virtual-account、statement、signals、signal-timeline、positions、rebalance-records。
- 导航回 `/dashboard`。

是否批量 remove 子资源可按实现便利度决定，但不能只依赖浏览器刷新。

### 详情页交互

确认弹层需要补齐状态：

| 状态 | UI 行为 |
| --- | --- |
| 初始 | 显示“删除”按钮；点击打开确认弹层 |
| pending | 禁用取消、确认和触发按钮；确认按钮显示“删除中” |
| succeeded | 关闭弹层，跳转 `/dashboard`，dashboard 不再出现该策略 |
| failed | 保持弹层打开，显示错误提示；允许重试和取消 |

建议确认文案：

```text
删除后该策略将从看板移除，历史回测和运行记录会保留。
```

这比当前“删除后该策略将从看板移除”更明确，避免用户误以为系统会清理所有历史数据。

### 跳转策略

删除成功后直接跳回 `/dashboard`。不要留在详情页等待 `getStrategyPortfolio()` 返回 archived record，因为当前后端仍能按 id 读取 archived record；留在详情页会让用户感知为删除失败。

如果用户之后通过旧链接再次打开已归档策略，`GET /rearview/strategy-portfolios/{id}` 应返回 `410 Gone`。前端识别 `ApiError.status === 410` 后跳转 `/dashboard`，并可短暂展示“该策略已删除”的提示。这里不展示 archived detail 页面，也不提供恢复入口。

### 错误处理

错误消息优先使用 `ApiError.message`。建议覆盖三类：

| 错误 | 用户展示 |
| --- | --- |
| 404 | `策略不存在或链接无效。` |
| 410 | 跳转 `/dashboard`，提示 `该策略已删除。` |
| validation error | 后端 message |
| network/unknown | `删除失败，请稍后重试。` |

归档接口具备幂等倾向：重复归档同一个 portfolio 会返回 archived record。但用户从详情页正常只能触发一次，前端仍应通过 pending 禁用避免重复点击。

## 后端设计

第一版保留现有 `PATCH status=archived` 作为唯一写入口，不新增 `DELETE` route。

需要补齐的后端行为：

1. `patch_strategy_portfolio()` 继续只接受 `{ "status": "archived" }`，其他 status 返回 validation error。
2. `get_strategy_portfolio()` 在 record 存在且 `status = 'archived'` 时返回 `410 Gone`，不要继续返回 archived detail payload。
3. 保持 dashboard 只读取 active portfolios。
4. 手动指定 archived portfolio 创建 daily run 时返回 `410 Gone`，不允许继续生成新的 live facts。
5. 为 archive API contract 增加测试：归档成功、重复归档、非 archived status 被拒绝、archived detail 返回 410、archived portfolio daily run 创建被拒绝。

## 数据边界

删除按钮只修改 PostgreSQL `strategy_portfolio` 控制面：

| 数据 | 行为 |
| --- | --- |
| `strategy_portfolio.status` | 设为 `archived` |
| `strategy_portfolio.archived_at` | 首次归档时写入 |
| `strategy_portfolio.updated_at` | 每次归档请求更新 |
| `strategy_portfolio_daily_run` | 不删除 |
| `strategy_portfolio_daily_task_outbox` | 不删除 |
| `fleur_portfolio.live_*` | 不删除 |
| `fleur_backtest.backtest_*` | 不删除 |
| source strategy backtest run | 不删除 |

## 验收标准

1. 从策略详情页点击“删除”，确认后会发起 `PATCH /rearview/strategy-portfolios/{id}`，body 为 `{ "status": "archived" }`。
2. 请求 pending 时按钮不可重复点击。
3. 请求失败时弹层不关闭，并展示可理解错误。
4. 请求成功后跳转到 `/dashboard`。
5. Dashboard query 刷新后不再显示该策略。
6. 后端返回的 record 中 `status = "archived"`，`archived_at` 非空。
7. 归档后直接访问 `GET /rearview/strategy-portfolios/{id}` 返回 `410 Gone`。
8. 前端遇到详情接口 410 时跳转 `/dashboard`，不渲染 archived detail。
9. 手动指定 archived portfolio 创建 daily run 时返回 `410 Gone`。
10. ClickHouse backtest/live facts 不被删除。
11. 前端测试覆盖 mutation 调用、成功跳转、dashboard cache invalidation、410 跳转和失败提示；后端测试覆盖 archive repository/API contract。

## 已决问题

1. 第一版不新增 `DELETE /rearview/strategy-portfolios/{id}` alias。
2. 归档后的直接详情链接返回 `410 Gone`。
3. 第一版不提供“已删除/已归档策略”入口和恢复能力。
4. 手动指定 archived portfolio 创建 daily run 返回 `410 Gone`，不允许继续运行。
