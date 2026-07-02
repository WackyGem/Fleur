# 2026-07-02 Racingline 策略详情页删除按钮验收

状态：Completed

范围：Plan 0071；Racingline 策略详情页删除按钮、Rearview Strategy Portfolio archive contract、archived detail `410 Gone`、手动 daily run archived 拒绝。

## 时间

执行日期：2026-07-02

执行环境：本地开发工作区 `/storage/program/fleur`。

## 结论

已完成策略详情页删除按钮第一版闭环：

1. 前端删除按钮调用 `PATCH /rearview/strategy-portfolios/{id}`，body 为 `{ "status": "archived" }`。
2. 删除成功后刷新 dashboard query、移除当前 portfolio 详情 query cache，并跳转 `/dashboard`。
3. 删除弹层具备 pending 禁用、失败提示和重试能力。
4. Rearview 新增 `RearviewError::Gone`，HTTP status 为 `410 Gone`，`error_type = "gone"`。
5. Rearview archived portfolio detail 和详情 live 子资源返回 `410 Gone`。
6. 手动指定 archived portfolio 创建 daily run 返回 `410 Gone`，不创建新 daily run/outbox。
7. 未新增 `DELETE /rearview/strategy-portfolios/{id}` route；归档仍是软删除，不删除 ClickHouse backtest/live facts。

## 代码范围

| 区域 | 文件 |
|---|---|
| Rearview API/error | `engines/crates/rearview-core/src/api/mod.rs`, `engines/crates/rearview-core/src/error.rs` |
| Rearview repository | `engines/crates/rearview-core/src/postgres/mod.rs` |
| Worker error classification | `engines/crates/rearview-core/src/service/runner.rs`, `engines/crates/rearview-portfolio-worker/src/main.rs` |
| Racingline API/hooks | `app/racingline/src/api/rearview.ts`, `app/racingline/src/api/hooks.ts` |
| Racingline detail UI | `app/racingline/src/routes/strategy-detail-page.tsx`, `app/racingline/src/routes/strategy-detail-utils.ts` |
| Tests | `app/racingline/src/api/rearview.test.ts`, `app/racingline/src/routes/strategy-detail-utils.test.ts` |

## Validation

已完成验证：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p rearview-core

cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build

make docs-check
git diff --check
```

结果：全部通过。

`npm run build` 仍输出 Vite chunk size warning，这是当前前端 bundle 的既有提示，不影响构建成功。

## 测试覆盖

| 行为 | 覆盖 |
|---|---|
| `RearviewError::Gone` HTTP 410 / `gone` error type | Rust unit test |
| archived portfolio active guard | Rust unit test |
| PATCH status 只接受 `archived` | Rust unit test |
| archive API 使用 PATCH 和 `{ status: "archived" }` body | Vitest API test |
| 410 archived error 识别 | Vitest utility test |
| 删除失败文案 fallback/后端 message | Vitest utility test |
| 详情页删除确认、成功跳转和 cache 刷新 | Vitest jsdom component test |
| 详情页删除失败弹层保持打开并展示后端错误 | Vitest jsdom component test |
| archived detail `410` 跳转 Dashboard、404 保持 not found 空态 | Vitest jsdom component test |

本次未运行真实 PostgreSQL archive integration smoke，因此 `archive_strategy_portfolio()` 的 `archived_at = coalesce(archived_at, now())` 幂等写入语义由现有 repository SQL 和 Rust 编译/单测覆盖；后续如新增 Rearview HTTP integration harness，应补 `PATCH` 归档、重复归档和 archived daily-run `410` 的数据库级测试。
