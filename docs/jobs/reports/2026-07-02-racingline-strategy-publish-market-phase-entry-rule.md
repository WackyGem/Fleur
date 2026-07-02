# Racingline 盘中/收盘后建仓信号日期规则实施报告

日期：2026-07-02

范围：Racingline Step 5「建立组合」发布预检、Rearview strategy portfolio create 校验、发布弹层展示和相关文档。

关联计划：[Plan 0070](../../plans/archive/0070-racingline-strategy-publish-market-phase-entry-rule-plan.md)

## 变更摘要

1. Rearview publish preview 从 `source_signal_date == server_current_date` 改为交易阶段感知规则：
   - 交易日 15:00 前：`required_source_signal_date = previous_trade_date(server_current_date)`。
   - 交易日 15:00 后：`required_source_signal_date = server_current_date`。
   - 非交易日：`required_source_signal_date = latest_trade_date_before_or_equal(server_current_date)`。
2. Preview response 新增 `required_source_signal_date`、`server_current_time`、`market_phase` 和 `publish_cutoff_time`。
3. Create request 新增 `expected_required_source_signal_date`，创建时重新预检并同时校验 required/source/live dates。
4. Racingline 发布弹层只展示“最后信号日”和“计划建仓日”；`required_source_signal_date`、`market_phase`、`server_current_time` 和 `publish_cutoff_time` 只用于校验、禁用确认和 create expected payload。
5. 数据落后超过允许信号日时继续阻断，不向更早信号日顺延；空位补仓和 TopN 执行规则未改。

## 关键用例

| 场景 | 结果 |
|---|---|
| 交易日 14:30，最后信号日为上一交易日 | 允许发布，计划建仓日为下一交易日，通常为当天 |
| 交易日 14:30，最后信号日早于上一交易日 | 阻断，提示最后信号日与最新行情日存在缺口，请先回填行情数据到最新。 |
| 交易日 15:30，最后信号日为上一交易日 | 阻断，提示最后信号日与最新行情日存在缺口，请先回填行情数据到最新。 |
| 交易日 15:30，最后信号日为当天 | 允许发布，计划建仓日为下一交易日 |
| 非交易日，最后信号日为最近完成交易日 | 允许发布 |
| 弹层打开后跨过 15:00 | create 重新预检，required date 不匹配时返回 conflict |

## 验证命令

```bash
cd engines
cargo test -p rearview-core
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm test
npm run build
npm run lint
npm run typecheck

cd ../..
make docs-check
git diff --check
```

## 结果

全部命令通过。`npm run build` 仅保留既有 Vite chunk size warning，无构建错误。
