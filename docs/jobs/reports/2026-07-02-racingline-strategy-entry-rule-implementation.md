# 2026-07-02 Racingline 最近信号建仓日期与空位补仓规则实施

状态：Completed

范围：Plan 0069；Rearview publish preview/create stale gate；Racingline 发布弹层日期展示；Step 4/Step 5 空位补仓命名解释。

## 结论

已完成 [RFC 0041](../../RFC/archive/0041-racingline-strategy-backtest-entry-rule-baseline.md) / [Plan 0069](../../plans/archive/0069-racingline-strategy-entry-rule-implementation-plan.md) 的代码实施：

1. Rearview publish preview 返回 `server_current_date`，并按服务端 CN A Share 市场日期阻断 stale signal 和 future signal。
2. `source_signal_date < server_current_date` 时，preview 返回 `can_publish=false`、blocker，`planned_live_start_date=null`，不生成 pending 首仓信号。
3. `source_signal_date == server_current_date` 时，保留原 T+1 下一交易日建仓逻辑。
4. create API 继续重新解析 publish preview；当 preview blocked 时拒绝创建组合，因此旧弹层或绕过前端的请求不能发布 stale signal。
5. Racingline 发布弹层展示最后信号日、当前日期和计划建仓日；blocked 时禁用确认。
6. Step 4/Step 5 文案改为“每日候选信号 Top N”和“仅空位调入；旧持仓由风控退出”，execution config 语义不变。

## 代码变更

| 区域 | 变更 |
|---|---|
| `engines/crates/rearview-core/src/service/mod.rs` | `AppState` 增加可注入 `current_market_date()`，默认按 UTC+8 解析 CN A Share 市场日期。 |
| `engines/crates/rearview-core/src/api/mod.rs` | publish preview response 增加 `server_current_date`；新增 stale/future date blocker；补充日期 gate 和序列化单测。 |
| `app/racingline/src/types/rearview.ts` | publish preview 类型增加 `server_current_date`。 |
| `app/racingline/src/routes/strategy-page.tsx` | 发布弹层展示最后信号日、当前日期、计划建仓日，并在建仓摘要中解释候选 Top N 和空位补仓。 |
| `app/racingline/src/features/strategy/components/simulation-position-panel.tsx` | Step 4 标签调整为“每日候选信号 Top N”和“仅空位调入；旧持仓由风控退出”。 |
| `app/racingline/src/features/strategy/utils.ts` | 修复当前前端 build 中可选 catalog 参数的类型阻断，保持默认 catalog 语义不变。 |

## 验证

Rust 后端：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：通过。

前端：

```bash
cd app/racingline
npm test
npm run build
npm run lint
```

结果：通过。`npm run build` 仅保留 Vite chunk size warning，无类型或构建错误。

文档：

```bash
make docs-check
git diff --check
```

结果：通过。

## 样本覆盖

自动化测试覆盖：

| 样本 | 证据 |
|---|---|
| stale signal：`2026-06-29 < 2026-07-02` | `strategy_portfolio_publish_date_blocker_should_block_stale_signal_date` |
| current signal：`2026-07-02 == 2026-07-02` | `strategy_portfolio_publish_date_blocker_should_allow_current_signal_date` |
| future signal：`2026-07-03 > 2026-07-02` | `strategy_portfolio_publish_date_blocker_should_block_future_signal_date` |
| preview response 暴露 `server_current_date` | `strategy_portfolio_publish_preview_should_serialize_server_current_date` |
| 空位补仓 execution config 不变 | `simulationSettingsToBacktestExecutionConfig` 前端测试继续通过 |

## 未执行项

本次没有使用真实行情滞后样本打开浏览器做截图验收，因为当前实现目标已由后端单测、前端类型构建和 UI 编译覆盖；没有伪造后端响应截图。后续如需要生产数据 smoke，可用最后信号日早于当前日期的回测 run 打开 Step 5 发布弹层，预期按钮禁用并展示 blocker。
