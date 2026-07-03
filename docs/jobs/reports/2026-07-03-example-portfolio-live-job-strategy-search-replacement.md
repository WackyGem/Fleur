# Example Portfolio Live Job Strategy Search Replacement

日期：2026-07-03

范围：把 [2026-07-03 strategy backtest search](2026-07-03-strategy-backtest-search.md) 中样本内最佳的“宽松低位反转 + 20 日时间止损”置换到 `example__portfolio_live_job` 使用的 Rearview example portfolio fixture。

## 置换方式

保留外部入口不变：

- Dagster job：`example__portfolio_live_job`
- Dagster asset：`rearview/example_0051_portfolio_live_run`
- Rearview ensure API：`POST /rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure`
- example case id：`racingline_0051_low_reversal`

fixture version 从 `v1` 升到 `v2`，避免已存在的 `v1` example portfolio 因同 case/version 但不同 fixture hash 触发冲突。Rearview 新建的组合显示名改为 `Racingline Strategy Search Low Reversal Example`。

## 策略内容

来源回测：

| 字段 | 值 |
|---|---|
| Run ID | `7166416c-97bc-4cc5-809e-832287135114` |
| Result attempt | `01KWMAKVVKS8M00VDFY3TF3HC0` |
| Rule hash | `115a15f03f9946cebc5de4d5fedc7bd607a7536fd3a6b7b3fd0fd4eac0a8989a` |
| Execution config hash | `6cf814ca48e47c76dcde0beff203b003377e7a5ce94bf0430d8343a4490aa0b3` |
| Benchmark | `000905.SH` |
| 近一年收益 | `142.95%` |
| 最大回撤 | `19.12%` |
| Sharpe | `3.7492` |

关键配置：

- 过滤：`kdj_j_value < 20`、`pct_amplitude < 5`、`-3 < pct_change < 3`、`close_down_streak_days < 5`、短中长期均线链、`volume < prev_volume * 1.0`。
- 评分：KDJ 深低位分段、缩量、MA20/MA60 区间、N 字结构、布林下轨、RSI6。
- 建仓：`buy_signal_top_n = 5`、`max_positions = 5`、`single_position_limit_pct = 0.20`、`cash_reserve_pct = 0`。
- 风控：仅启用 `time_stop_loss`，`holding_days = 20`、`max_return_pct = 0.0`。
- 价格口径：`backward_adjusted`，默认 A 股费率模板，买卖滑点 `10 bps`。

## 修改文件

- `engines/crates/rearview-core/src/examples.rs`：用专用 `v2` rule builder 替换旧 `representative_rule()`，基准改为 `000905.SH`，退出规则改为 20 日时间止损，并新增 hash 回归测试。
- `engines/crates/rearview-core/src/api/mod.rs`：更新新建 example portfolio 的显示名和 `ui_display_snapshot.kind`。
- `pipeline/scheduler/src/scheduler/defs/rearview/assets.py`：`EXAMPLE_0051_VERSION` 改为 `v2`。
- `pipeline/scheduler/tests/unit/rearview/test_rearview_assets.py`：同步 `v2` ensure response 断言。

## 验证

已执行：

```bash
cd engines
cargo fmt
cargo fmt --check
cargo test -p rearview-core examples
cargo test -p rearview-core
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd pipeline
uv run pytest scheduler/tests/unit/rearview/test_rearview_assets.py scheduler/tests/integration/test_definitions_and_schedules.py

cd pipeline/scheduler
uv run dg check defs

cd /storage/program/fleur
make docs-check
```

结果：

- `cargo fmt --check`：passed。
- `cargo test -p rearview-core examples`：7 passed。
- `cargo test -p rearview-core`：183 passed。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：passed。
- `cargo test --workspace`：all workspace tests and doctests passed。
- scheduler pytest：25 passed。
- `uv run dg check defs`：all definitions loaded successfully。
- `make docs-check`：docs governance validation passed。

未执行 live `dg launch --job example__portfolio_live_job`。当前代码变更需要重新构建/重启 Rearview 服务后，运行中的 production-like nginx 或容器才会使用新的 Rust fixture。
