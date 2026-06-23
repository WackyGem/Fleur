# Racingline Strategy Step 5 Backtest Acceptance

日期：2026-06-23

范围：

- Racingline `/strategies` Step 1 到 Step 5 端到端验收。
- Rearview strategy backtest control plane、typed outbox、NATS worker、ClickHouse result wrapper API。
- Step 5 默认动态近一年、period/benchmark 重新回测和 worker at-least-once 重投递演练。

## 环境

启动命令：

```bash
make racingline-dev
```

服务：

- Rearview: `http://127.0.0.1:34057`
- Racingline: `http://127.0.0.1:5173`
- PostgreSQL、ClickHouse、NATS 使用 `deploy/docker-compose.yml` dev 依赖。

## 前端验收输入

Step 1 使用 canonical 字段录入 10 条 AND 过滤：

```text
kdj_j_value < 13
pct_amplitude < 4
pct_change > -2
pct_change < 2
volume < prev_volume * 0.8
price_ema2_10 > price_avg_ma_14_28_57_114
close_down_streak_days < 4
close_price_forward_adj > price_avg_ma_3_6_12_24
price_ma_60 > price_ma_114
price_ma_114 > price_ma_250
```

Step 2 使用 conditional points：

```text
kdj_j_value < -15                                      +25
-15 <= kdj_j_value < -10                               +15
volume < volume_ma_5 * 0.6                             +20
price_ma_20 < close_price_forward_adj < price_ma_60    +15
n_structure_20_second_low_ratio > 1                    +15
close_price_forward_adj < boll_lower_20_2              +15
rsi_6 < 25                                             +5
```

KDJ 分段已用附加条件表达，`kdj_j_value < -15` 与 `-15 <= kdj_j_value < -10` 不会重复计分。

Step 4 使用默认仓位、费率和全部风控：

- `initialCapital = 1_000_000`
- `buyTopN = 5`
- `maxPositions = 5`
- `singlePositionLimitPercent = 10`
- 默认费率模板：佣金、最低佣金、印花税、过户费和买卖滑点。
- 风控：固定止损、止盈、时间止损、指标止损全部启用；指标止损为 `close_below_metric price_ma_10`。

## 浏览器截图证据

截图目录：[docs/references/screenshots/racingline/2026-06-23/](../../references/screenshots/racingline/2026-06-23/)

| 截图 | 证据 |
|---|---|
| [step5-acceptance-01-step1-filters.png](../../references/screenshots/racingline/2026-06-23/step5-acceptance-01-step1-filters.png) | Step 1 10 条 canonical 过滤条件 |
| [step5-acceptance-02-step2-scoring.png](../../references/screenshots/racingline/2026-06-23/step5-acceptance-02-step2-scoring.png) | Step 2 7 条得分条件，包含互斥 KDJ 分段和字段间比较 |
| [step5-acceptance-03-step3-preview.png](../../references/screenshots/racingline/2026-06-23/step5-acceptance-03-step3-preview.png) | Step 3 preview 成功，进入非 stale snapshot |
| [step5-acceptance-04-step4-defaults-risk.png](../../references/screenshots/racingline/2026-06-23/step5-acceptance-04-step4-defaults-risk.png) | Step 4 默认仓位、默认费率和全部风控 |
| [step5-acceptance-05-step5-running.png](../../references/screenshots/racingline/2026-06-23/step5-acceptance-05-step5-running.png) | Step 5 创建真实 run 后进入 queued/running 状态 |
| [step5-acceptance-06-step5-succeeded-nav.png](../../references/screenshots/racingline/2026-06-23/step5-acceptance-06-step5-succeeded-nav.png) | Step 5 succeeded 后展示真实策略净值和 benchmark 净值 |
| [step5-acceptance-07-step5-rebalance-performance.png](../../references/screenshots/racingline/2026-06-23/step5-acceptance-07-step5-rebalance-performance.png) | Step 5 调仓记录、证券名称、收益贡献和策略业绩侧栏 |
| [step5-acceptance-08-step5-rerun.png](../../references/screenshots/racingline/2026-06-23/step5-acceptance-08-step5-rerun.png) | 修改周期和 benchmark 后展示历史快照/配置已变更，并提供重新回测 |

## Run 结果

默认 run：

- run id: `7b7aec38-1c65-4d87-8d1e-1718db6eadfa`
- status: `succeeded`
- period: `1y`
- benchmark: `000300.SH`
- frozen range: `2025-06-03` 到 `2026-06-01`
- result attempt: `01KVTTX2W8VWPBAQ0CEV8G2HEF`
- compiled SQL hash: `9b72ef2f8f9484a6e2c7cec5774462b6fbaf1614274c7427123e24370f495124`
- worker attempt: `1`
- signal summary: generated `826`，TopN `527`，executable `527`，dropped `0`，signal dates `166`
- data coverage: price bars `112772`，securities `466`，indicator stop metric `price_ma_10`

重新回测 run：

- run id: `f02c823d-263b-431a-8927-a6e1c0fb4082`
- status: `succeeded`
- period: `2y`
- benchmark: `000905.SH`
- frozen range: `2024-06-03` 到 `2026-06-01`
- result attempt: `01KVTV34KVCTC4D63QNDYM5PBY`
- compiled SQL hash: `793f3014f1ef1f67653c11c44e7e32f1daa2fc8f9b6d0c9f46ee69b0db9d1ee6`
- worker attempt: `1`
- signal summary: generated `1327`，TopN `836`，executable `836`，dropped `0`，signal dates `266`
- data coverage: price bars `344232`，securities `714`，indicator stop metric `price_ma_10`

故障重投递演练 run：

- run id: `fc9da304-aaf7-43e4-b6f8-33e6b03417b2`
- 操作：run 进入 `running_clickhouse` 后终止 `rearview-portfolio-worker`，将 dev lease 置过期，重启 `make racingline-dev`
- final status: `succeeded`
- period: `3y`
- benchmark: `000905.SH`
- frozen range: `2023-06-26` 到 `2026-06-01`
- result attempt: `01KVTX0J7NNW1WMDWQ833M9XWC`
- worker attempt: `2`
- ClickHouse nav rows: `710`
- 结论：NATS at-least-once 重投递后 worker 能重新 claim，旧未 finalize attempt 未暴露为 current result。

## API 证据

Options API：

```bash
curl -sS 'http://127.0.0.1:34057/rearview/strategy-backtests/options?benchmark_security_code=000905.SH'
```

返回：

- default period: `1y`
- selected benchmark: `000905.SH`
- latest available trade date: `2026-06-01`
- `1y`: `2025-06-03` 到 `2026-06-01`
- `2y`: `2024-06-03` 到 `2026-06-01`
- `3y`: `2023-06-26` 到 `2026-06-01`

Wrapper API：

- `/nav` 默认 run 返回 `242` 行，首日 `2025-06-03`，末日 `2026-06-01`，benchmark nav 非空 `242` 行。
- `/rebalance-records` 默认 run 返回 `242` 个调仓日，选中 `2026-06-01`，持仓 `1`、调入 `0`、持有 `1`、卖出 `1`。
- selected rows 包含 `深圳能源 000027.SZ` 持有行和 `苏博特 603916.SH` 卖出行，卖出原因为 `indicator_stop_loss`。
- `/performance` 默认 run 返回 `security_code = 000300.SH`、`metric_status = succeeded`、`daily_win_rate = 0.42738589211618255`、`observation_count = 241`。
- `/targets`、`/orders`、`/trades`、`/positions`、`/events`、`/closed-trades`、`/trade-metrics` 均返回当前 attempt 的非空样本。

## 数据库证据

PostgreSQL：

```text
strategy_backtest_run_id                 status     period  start_date  end_date    benchmark  result_attempt               worker_attempt  sql_hash_len
7b7aec38-1c65-4d87-8d1e-1718db6eadfa     succeeded  1y      2025-06-03  2026-06-01  000300.SH  01KVTTX2W8VWPBAQ0CEV8G2HEF   1               64
f02c823d-263b-431a-8927-a6e1c0fb4082     succeeded  2y      2024-06-03  2026-06-01  000905.SH  01KVTV34KVCTC4D63QNDYM5PBY   1               64
fc9da304-aaf7-43e4-b6f8-33e6b03417b2     succeeded  3y      2023-06-26  2026-06-01  000905.SH  01KVTX0J7NNW1WMDWQ833M9XWC   2               64
```

Outbox：

```text
7b7aec38-1c65-4d87-8d1e-1718db6eadfa  published  sequence 12
f02c823d-263b-431a-8927-a6e1c0fb4082  published  sequence 13
fc9da304-aaf7-43e4-b6f8-33e6b03417b2  published  sequence 14
```

ClickHouse：

```text
portfolio_nav_daily:
7b7aec38-1c65-4d87-8d1e-1718db6eadfa  01KVTTX2W8VWPBAQ0CEV8G2HEF  242 rows  2025-06-03  2026-06-01
f02c823d-263b-431a-8927-a6e1c0fb4082  01KVTV34KVCTC4D63QNDYM5PBY  483 rows  2024-06-03  2026-06-01
fc9da304-aaf7-43e4-b6f8-33e6b03417b2  01KVTX0J7NNW1WMDWQ833M9XWC  710 rows  2023-06-26  2026-06-01

calc_portfolio_performance_metric:
7b7aec38-1c65-4d87-8d1e-1718db6eadfa  01KVTTX2W8VWPBAQ0CEV8G2HEF  000300.SH  full_period  succeeded  241 observations
f02c823d-263b-431a-8927-a6e1c0fb4082  01KVTV34KVCTC4D63QNDYM5PBY  000905.SH  full_period  succeeded  482 observations
```

所有 result row 的最大日期均不超过 run frozen `end_date`。

## 实现收敛

本轮发现并修复一个 Step 5 UI 交互缺口：

- Options API 对未选中的 benchmark 返回 `availability_status = not_checked`。
- 前端之前把非 `available` 全部禁用，导致无法从 `沪深300` 切换到 `中证500`。
- 现已只禁用明确 `unavailable` 的 benchmark；选择 `not_checked` benchmark 后 options query 会按该 benchmark 重新解析动态区间。

同时收敛 `range_hint`：

- 前端不再用浏览器当前日期自然年回退生成 Step 5 hint。
- create request 的 `range_hint` 来自 options API 返回的动态 `resolved_start_date/resolved_end_date`。
- live smoke 中 default run 和 rerun 的 `range_hint` 与 Rearview frozen range 一致。

## 测试覆盖

代码测试覆盖了以下语义：

- `BacktestExecutionConfig::canonicalized()` 不再把 `max_positions` 覆盖为 `buy_signal_top_n`。
- `TopN=3/maxPositions=5`、空仓位限制、已持仓不重复买入、卖出后允许重新买入。
- `signal_date = end_date` 不产生 `end_date` 之后成交，nav/trades 不越过 `end_date`。
- 指标止损支持 MA、MA 组合和 EMA 主图指标。
- range resolver 覆盖非交易日、benchmark 最新 return 缺失回退和 `1y/2y/3y` 起始交易日解析。
- result wrapper 先解析 current attempt，再读取 ClickHouse 当前结果。

最终质量门禁：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test -- --run
npm run build

cd ../../engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ..
make docs-check
git diff --check
```

结果：全部通过。`npm run build` 有 Vite chunk size warning，不影响本次验收。

## 后续

“成功 backtest 发布为正式运行策略”仍是非目标，应另起 RFC/plan。
