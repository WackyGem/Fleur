# Racingline Step5 Backtest Worker Latency Optimization

日期：2026-06-26

范围：记录 Plan 0058 实施结果。本文覆盖已落地的 Phase 0 事实快照辅助工具、Phase 1 simulation 热路径重构、Phase 2 backtest/daily run signal SQL 窄化、Phase 3 `MarketDataDemand` 实验结论、Phase 4 Step 5 overview 首屏读取收敛、Phase 5 worker bounded concurrency，以及 1y/2y/3y live smoke、query_log 和结果事实 hash。

## 实现摘要

- `simulate_portfolio()` 保持既有输入/输出 contract，内部改为调用 `simulate_portfolio_with_diagnostics()`。
- 新增私有 `PriceStore`：使用原始 `Vec<PriceBar>` 切片和 `(trade_date, &str) -> index` 索引读取价格，移除全量 `PriceBar` clone 和循环内 `security_code.to_string()` 查价。
- 新增私有 `TradeCalendarPlan`：预构造 `trade_date -> next_trade_date`，最后一个交易日返回 `None`，不再把无下一交易日的退出单静默 enqueue 到当前日。
- simulation diagnostics 输出 `price_store_build`、`calendar_build`、`signal_index_build`、`daily_loop`、`sell_handling`、`buy_handling`、`valuation`、`exit_evaluation`、`output_finalize` 和 row counts。
- Worker summary `worker_timing` 升级为 version 2，包含 `stages_ms`、`simulation_ms`、`row_counts`、`query_ids` 和 `total_ms`。
- Planner 新增 `compile_backtest_signals()`，worker SQL 默认只返回 `security_code`、`trade_date`、`score`、`signal_rank`，并在 SQL 层执行 `WHERE signal_rank <= top_n`。
- ClickHouse 新增 `BacktestSignalRow` 与 `query_backtest_signal_rows()`，不收缩 `ScreeningRow`，Step 3 preview/explain 仍使用原完整 row contract。
- `materialize_strategy_backtest_signals()` 与 `materialize_strategy_portfolio_daily_run_signals()` 均切换到窄化 TopN SQL，避免 backtest 与 daily run TopN 语义分叉。
- Signal summary 不再把 TopN row count 冒充全量候选数，改写为 `top_n_row_count`、`diagnostic_generated_candidate_count_unavailable` 和 `signal_date_count_semantics`。
- 新增只读事实快照辅助脚本 `scripts/strategy_backtest_fact_snapshot.py`，通过 Docker Compose ClickHouse 客户端导出 strategy backtest 结果事实表，剔除 volatile 字段后按业务键排序并计算 stable hash、row count 和摘要。
- 新增 `MarketDataDemand` 与 demand join SQL builder，用于离线比较按证券最早 execution date 读取行情的 SQL 形态；默认 worker 仍使用完整区间 price bars SQL。
- 新增 `GET /rearview/strategy-backtests/{id}/overview?view=ui`，一次返回 Step 5 首屏需要的 compact status、nav points、performance 和 rebalance read model；非 `view=ui` 请求显式拒绝。
- Racingline Step 5 succeeded 首屏改为读取独立 overview query key，避免同时发起 nav/performance/rebalance 三个 UI wrapper 请求；发布确认弹窗仍使用原 detail/full hooks。
- `rearview-portfolio-worker` 使用既有 `REARVIEW_MAX_CONCURRENT_RUNS` 构造 bounded `Semaphore`，获取 permit 后再拉取下一条 JetStream 消息，限制 delivered-but-unacked task 数量；ack 仍只在 handler 完成后执行，保留 at-least-once/redelivery 语义。
- `POST /rearview/strategy-backtests` 的 range-resolution 和 risk-free preflight query id 改为带 request scope，避免两个并发 create 请求共用 ClickHouse query id 触发 `QUERY_WITH_SAME_ID_IS_ALREADY_RUNNING`。

## Live 样本

环境：`make racingline-dev`，`REARVIEW_MAX_CONCURRENT_RUNS=1`，benchmark `000300.SH`，代表性 `rearview-server sample-rule`，TopN/Max positions 10，range as of `2026-06-26`，latest available trade date `2026-06-25`。

| Period | Run id | Result attempt | Worker total | Signal | Price bars | Simulation | Overview UI |
|---|---|---|---:|---:|---:|---:|---:|
| 1y | `994c8af2-5353-45e1-b040-aac4ee42ba6e` | `01KW0TB5Q2MAW6TJ6BMYF8XFJ2` | 4,932ms | 2,548ms | 1,174ms | 350ms | 205ms |
| 2y | `9a874a25-2164-4271-8014-b9d19042471b` | `01KW0TBF6TRZWNMFSADMRXD85Y` | 9,376ms | 4,173ms | 3,208ms | 1,059ms | 229ms |
| 3y | `d245cd83-a3fe-41ab-b175-00bb8284db3d` | `01KW0TBY1RAQPAK4R116M22Q93` | 13,982ms | 6,061ms | 5,261ms | 1,806ms | 267ms |

与 Plan 0058 / RFC 0032 基线对比：

| Metric | Before | After | 结论 |
|---|---:|---:|---|
| 1y worker elapsed | 6,477ms | 4,932ms | 下降 23.9% |
| 2y worker elapsed | 13,797ms | 9,376ms | 下降 32.0%；单次样本略高于 6-9s 讨论目标，后续并发 smoke 2y 为 8,931ms |
| 2y signal materialization | 4,385ms | 4,173ms | 小幅下降；query_log 显示主要仍在 mart join/input scan |
| 2y price bars worker stage | 4,302ms | 3,208ms | 下降 25.4%；默认 full-range SQL 保留 |
| 2y simulation | 4,212ms | 1,059ms | 下降 74.9%，达到 1s 级别 |

2y row counts：`signal_count=928`，`signal_security_count=769`，`price_bar_count=372394`，`nav_count=485`，`trade_count=10`，`position_day_count=4222`，`event_count=0`。

2y ClickHouse query_log 摘要：

| Query | duration | read_rows | read_bytes | memory |
|---|---:|---:|---:|---:|
| chunk 0 | 1,512ms | 3,354,214 | 154,372,541 | 693,810,595 |
| chunk 1 | 1,812ms | 6,257,370 | 285,348,079 | 792,266,101 |
| chunk 2 | 748ms | 2,581,279 | 119,075,417 | 239,221,216 |
| price bars | 375ms | 2,507,447 | 80,238,304 | 140,807,385 |
| benchmark | 7ms | 3,588 | 78,936 | 7,574,524 |
| risk-free | 8ms | 727 | 10,905 | 7,570,564 |

## MarketDataDemand 结论

2y 样本从实际 TopN signal rows 派生 demand：`signal_rows=928`，`demand_security_count=769`，`demand_start_min=2024-07-19`。`EXPLAIN indexes = 1` 显示 current full-range price SQL 的 PrimaryKey granules 为 `316/386`，inline demand join 为 `306/386`，只小幅减少 granules，同时引入 769 行 inline `UNION` join 右表。

`FORMAT Null` 对比：

| Shape | duration | read_rows | read_bytes | memory |
|---|---:|---:|---:|---:|
| current full-range | 123ms | 2,507,447 | 80,238,304 | 21,485,552 |
| inline demand join | 433ms | 2,426,296 | 77,617,633 | 44,053,780 |

结论：inline demand join 在 2y 样本中只减少 3.2% read_rows / 3.3% read_bytes，但 duration 增至 3.5x、memory 增至 2.1x，因此不接入默认 worker。`MarketDataDemand` 类型和 SQL builder 保留为 opt-in 实验面。

## Queue Smoke

并发提交 1y + 2y：

| Period | Run id | Created | Started | Completed | Worker total |
|---|---|---|---|---|---:|
| 1y | `f415ec6d-ff00-47cb-a577-ff39c8be2030` | 02:01:59.409846 | 02:01:59.659874 | 02:02:03.864625 | 4,195ms |
| 2y | `c6df28c7-7e50-490a-8b50-98ec296caf3e` | 02:01:59.529666 | 02:02:03.894902 | 02:02:12.832522 | 8,931ms |

结论：`REARVIEW_MAX_CONCURRENT_RUNS=1` 时，第二个 run 保持 queued，直到第一个 run completed 后约 30ms 启动。ClickHouse fact check 显示两个 run 各只有一个 `result_attempt_id`，没有 double-finalize：1y nav rows 243，2y nav rows 485。

## 结果事实 Hash

`scripts/strategy_backtest_fact_snapshot.py` 剔除 volatile id 后的 stable hash：

| Period | nav rows/hash | trade rows/hash | order rows/hash | position rows/hash | event rows/hash |
|---|---|---|---|---|---|
| 1y | 243 / `9f5ca9d43c3c8491a49bac208f94e8c92bacc2f003e11b9a3a7b857871392b85` | 10 / `9dc13f174bd9cb1c673b64f584281055d1b8af1c1e3c5865dea6e00e9b005f17` | 10 / `5ce132d8f427807a8acfb0de7b5c1c548326de438617b393ecfd0a4429f5a52c` | 2366 / `0ea61419ca5fdfd6f8198105f4a0a26d0bc896c6b0f821422f539baebcb30d83` | 0 / `4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945` |
| 2y | 485 / `68022cf2338c1922a6ec3832ca39cd9aba9b2424840f44a0a8c7678974ca3308` | 10 / `dfc3f451608f67d9d08f1c74a12df52344328b776daeaa09f9e35344e423bfdf` | 10 / `79e5adc4e9ccab89a92c32c8e1c28409f1c1fb3d0ff168f4f8430923a747da9a` | 4222 / `5b742f6630e59360b6bf97b39d825d3b7c5c0c0dc280f90087b0b4abdfab0d15` | 0 / `4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945` |
| 3y | 727 / `48e6e0d54e043f88210b485b646ddb780c90a793a98f324f6ff028d8b5a34c39` | 10 / `63960b3868eedb52bd91563a601c599d4f9cf7d2509c396ac0c43911f15fe0c4` | 10 / `7967da4e1f95c9e1ca85b9b4ef15e464c2053f29083282cc09f52ff5b75e92ce` | 7206 / `dc4ec352525f5d0a6cfe819c4e62627056bff8d826c71884c7c2e070a6399a09` | 0 / `4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945` |

## 验证命令

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm run lint
npm run typecheck
npm test
npm run build

cd ../..
python3 -m py_compile scripts/strategy_backtest_fact_snapshot.py
make docs-check
git diff --check
```

结果：

- `cargo fmt --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace`：通过，包含 `rearview-core` 131 个单测、`rearview-portfolio-worker` 4 个单测和 workspace 其他 crate 测试。
- `npm run lint`：通过。
- `npm run typecheck`：通过。
- `npm test`：通过，7 个 test files、51 个 tests。
- `npm run build`：通过；Vite 报告现有单 chunk 大于 500 kB 的 warning。
- `python3 -m py_compile scripts/strategy_backtest_fact_snapshot.py`：通过。
- `make docs-check`：通过。
- `git diff --check`：通过。

## 结果等价护栏

新增/保留的单测覆盖：

- same-day signal 继续被拒绝。
- 缺失 open/close price 的 skipped order/event 语义由既有测试覆盖。
- indicator stop loss 触发和 missing indicator 事件由既有测试覆盖。
- `TradeCalendarPlan` 最后一个交易日不返回非法下一交易日。
- 无下一交易日的 exit signal 不产生同日卖单。
- `PriceStore` 在重复证券/日期、缺失 metric、缺失 security/date 时返回明确结果。
- `compile_backtest_signals()` SQL 包含 TopN filter 且不包含 `score_breakdown`、`selected_metrics`、`raw_values`、`is_buy_signal`。
- `BacktestSignalRow` 只依赖 worker 热路径窄字段即可解析。
- `MarketDataDemand` 对重复证券保留最早 start date，demand join SQL 在 join 前过滤 security/date，且拒绝 unsupported indicator metric。
- overview UI response 序列化保持 compact contract，不混入 full/detail 字段。
- frontend query key 测试覆盖 overview cache 与原 nav/performance/rebalance cache 分离。
- worker bounded concurrency 单测覆盖 task status 枚举和 demand summary；live queue smoke 覆盖 bounded pickup 和 no double-finalize。

## 残余风险

- 本次 queue smoke 未强杀 worker 验证 JetStream redelivery；代码仍保持 ack-after-handler，可靠性边界未放宽。
- 本次首屏 overview 验证使用直接 HTTP 计时，未额外通过 Playwright/CDP 采集浏览器 network waterfall；前端质量门禁已覆盖类型、lint、单测和 build。
- `MarketDataDemand` 不接入默认 worker。若后续样本证券分布明显不同，应重新比较 current、inline demand join 和分 chunk demand 查询。
