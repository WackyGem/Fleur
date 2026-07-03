# Racingline Strategy Backtest Search

日期：2026-07-03

范围：

- 基于 Rearview 当前 metric catalog、Racingline Step 1/2 规则结构和 Step 4 `BacktestExecutionConfig` 风控字段，探索近一年策略回测候选。
- 回测入口：production-like nginx `http://127.0.0.1:35080`。
- 回测周期：`1y`，动态解析区间 `2025-07-03` 到 `2026-07-03`。
- 基准：中证500 `000905.SH`，同期 benchmark nav `1.4765765213603361`。

## 执行入口

事实确认命令：

```bash
curl -sS --noproxy '*' \
  'http://127.0.0.1:35080/rearview/strategy-backtests/options?benchmark_security_code=000905.SH'

curl -sS --noproxy '*' \
  'http://127.0.0.1:35080/rearview/metrics?allow_filter=true'

curl -sS --noproxy '*' \
  'http://127.0.0.1:35080/rearview/metrics?allow_scoring=true'
```

批量探索使用一次性 `python3` API harness，逐个执行：

1. `POST /rearview/strategy-backtests/validate`
2. `POST /rearview/strategy-backtests`
3. `GET /rearview/strategy-backtests/{id}/status`
4. `GET /rearview/strategy-backtests/{id}/overview?view=ui`
5. `GET /rearview/strategy-backtests/{id}/performance?view=ui`
6. `GET /rearview/strategy-backtests/{id}/trade-metrics`

运行产物 JSON 临时记录在 `/tmp/fleur_strategy_search_20260703.json`。

## Catalog 覆盖

本次候选只使用 Rearview `/rearview/metrics` 返回的 allowlisted 指标：

- `quotes`：`close_price`、`close_price_forward_adj`、`pct_amplitude`、`pct_change`、`prev_volume`、`volume`
- `trend`：MA、均线组、BOLL、MACD、`price_ema2_10`
- `momentum`：KDJ、RSI
- `volume`：`volume_ma_5/10/20/60`
- `pattern`：N 字结构布尔和比例字段

探索的风控组合：

- 关闭退出规则，仅保留调仓持仓逻辑。
- MA10 / MA20 指标止损。
- 固定止损 8% + MA20 指标止损。
- 固定止损 10% + 止盈 25% + MA20 指标止损。
- 止盈 30% + MA20 指标止损。
- 时间止损 10/20 个交易日，`max_return_pct = 0`，可单独使用或叠加 MA20 / 固定止损。

## 搜索结果

共创建 31 个 succeeded backtest run：第一阶段 12 个规则族，第二阶段 15 个风控变体，补充阶段 4 个时间止损变体。

| 排名 | 候选 | Run ID | 收益 | 最大回撤 | Sharpe | 日胜率 | 策略 NAV |
|---:|---|---|---:|---:|---:|---:|---:|
| 1 | 宽松低位反转 + 20 日时间止损 | `7166416c-97bc-4cc5-809e-832287135114` | 142.95% | 19.12% | 3.75 | 53.72% | 2.4295 |
| 2 | 宽松低位反转 + 无退出规则 | `7015e789-21fc-452e-9325-95606ed679e9` | 136.29% | 19.67% | 3.56 | 52.48% | 2.3629 |
| 3 | 宽松低位反转 + 固定止损 8% + MA20 指标止损 | `e2ad17a8-ed1b-4548-8074-084d94abfaf8` | 121.44% | 19.88% | 3.21 | 58.68% | 2.2144 |
| 4 | 宽松低位反转 + MA20 指标止损 | `28187167-7d32-43ca-a282-789ab9dd7667` | 116.32% | 20.44% | 3.18 | 59.09% | 2.1632 |
| 5 | 宽松低位反转 + MA10 指标止损 | `9ad78590-ac10-49d7-8530-4f6b550d8f03` | 84.22% | 21.43% | 2.33 | 57.02% | 1.8422 |
| 6 | 无长均线链低位反转 + MA20 指标止损 | `457979e5-abc0-4ab9-89d0-002c57e0bd68` | 64.57% | 13.84% | 2.10 | 57.44% | 1.6457 |
| 7 | 无长均线链低位反转 + 固定止损 10% + 止盈 25% + MA20 | `43703365-06c9-46d1-8dc2-39204e222b08` | 58.75% | 12.84% | 2.12 | 58.26% | 1.5875 |
| 8 | 布林突破 + MA20 指标止损 | `7176297e-254e-4d7d-98e4-511a750cd2e5` | 37.02% | 30.50% | 0.94 | 56.61% | 1.3702 |

纯趋势动量、MACD 转强、均线交叉和牛市回调候选在该样本内不如放宽后的低位反转规则。

## 样本内最佳

候选：宽松低位反转 + 20 日时间止损。

选股过滤：

```text
kdj_j_value < 20
pct_amplitude < 5
pct_change > -3
pct_change < 3
close_down_streak_days < 5
price_ema2_10 > price_avg_ma_14_28_57_114
close_price_forward_adj > price_avg_ma_3_6_12_24
volume < prev_volume * 1.0
price_ma_60 > price_ma_114
price_ma_114 > price_ma_250
```

评分规则：

```text
kdj_j_value < -15                                      +25
-15 <= kdj_j_value < -10                               +15
volume < volume_ma_5 * 0.6                             +20
price_ma_20 < close_price_forward_adj < price_ma_60    +15
n_structure_20_is_valid = true                         +20
close_price_forward_adj < boll_lower_20_2              +15
rsi_6 < 25                                             +5
```

建仓和风控：

- 初始资金：`1_000_000 CNY`
- `buy_signal_top_n = 5`
- `max_positions = 5`
- `single_position_limit_pct = 0.20`
- `cash_reserve_pct = 0`
- 价格口径：`backward_adjusted`
- 交易成本：默认 A 股费率模板，买卖滑点 `10 bps`
- 风控：`time_stop_loss`，持仓满 20 个交易日且收益 `<= 0` 时，收盘确认后次日开盘退出

结果：

- Run ID：`7166416c-97bc-4cc5-809e-832287135114`
- Result attempt：`01KWMAKVVKS8M00VDFY3TF3HC0`
- Rule hash：`115a15f03f9946cebc5de4d5fedc7bd607a7536fd3a6b7b3fd0fd4eac0a8989a`
- Execution config hash：`6cf814ca48e47c76dcde0beff203b003377e7a5ce94bf0430d8343a4490aa0b3`
- 收益：`142.95%`
- 年化收益：`152.03%`
- 年化波动：`40.20%`
- 最大回撤：`19.12%`
- Sharpe：`3.7492`
- Sortino：`6.1511`
- Information ratio：`3.6107`
- Alpha：`0.8839`
- Beta：`1.2786`
- 日胜率：`53.72%`，`130 / 242`
- 信号：`951` 条可执行 TopN 信号，`223` 个信号日
- 行情覆盖：`178605` 根 price bars，`735` 只证券
- 成交：`33` 笔成交，`14` 笔已平仓，`19` 个 target rows
- 总费用：`1934.63`

末日持仓：

| 证券 | 名称 | 持仓天数 | 未实现收益 | 贡献 |
|---|---|---:|---:|---:|
| `688630.SH` | 芯碁微装 | 241 | 506.31% | 40.57% |
| `002645.SZ` | 华宏科技 | 220 | 119.38% | 9.53% |
| `300440.SZ` | 运达科技 | 241 | 101.60% | 8.35% |
| `688585.SH` | 上纬新材 | 47 | 38.63% | 2.85% |
| `603135.SH` | 中重科技 | 9 | 13.62% | 0.84% |

注意：该候选收益主要来自继续持有未触发时间止损的赢家；已平仓交易 `14` 笔中只有 `1` 笔微盈利，closed-trade win rate 为 `7.14%`。因此它是样本内效果最好的候选，但对持有赢家和结束日期敏感。

## 更均衡风控备选

候选：同一规则 + 固定止损 8% + MA20 指标止损。

- Run ID：`e2ad17a8-ed1b-4548-8074-084d94abfaf8`
- Result attempt：`01KWMADZSBS6XJNAXH51YNM6JY`
- 收益：`121.44%`
- 最大回撤：`19.88%`
- Sharpe：`3.2114`
- 日胜率：`58.68%`
- 成交：`282` 笔成交，`139` 笔已平仓
- Closed trade win rate：`34.53%`
- 平均盈利交易收益：`16.68%`
- 平均亏损交易收益：`-3.56%`
- 盈亏比：`4.68`
- 最大单笔盈利：`248.26%`
- 最大单笔亏损：`-10.39%`
- 总费用：`27681.23`

这个版本收益低于时间止损单独版本，但交易样本更多，退出逻辑更主动，适合作为后续 out-of-sample 或 2y/3y 验证的优先候选。

## 结论

近一年样本内最佳策略是“宽松低位反转 + 20 日时间止损”，收益 `142.95%`，最大回撤 `19.12%`，显著跑赢中证500同期 `47.66%`。更适合继续验证的稳健备选是“宽松低位反转 + 固定止损 8% + MA20 指标止损”，收益 `121.44%`，交易统计更充分。

后续不应直接发布该策略；需要至少追加 2y/3y、不同 benchmark、固定起止日期滚动窗口和最新信号日 publish preview 验证，以判断是否只是近一年行情和少数赢家带来的样本内过拟合。
