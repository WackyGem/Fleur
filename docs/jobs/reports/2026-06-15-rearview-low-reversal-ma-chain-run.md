# Rearview Low Reversal MA Chain Run

日期：2026-06-15

## 基本信息

```text
Service = Rust rearview HTTP service
API = http://127.0.0.1:34057
Rule set = low_reversal_ma_filter_20260615
rule_set_id = df0755ac-3f4c-4863-ab05-a7a8d423b8fc
rule_version_id = ffce0e91-8096-479f-92e3-259028404a1a
rule_hash = 8b4f79531bb2d1e07cfc54c0a714b49eecb5e7e706751cc807605f43202cf1b9
run_id = 6fa431fe-d7b8-464f-be46-ecd68b1d32fb
date range = 2026-04-01 -> 2026-06-01
top_n = 10
```

## 范围

本次运行覆盖 Plan 0040 低位反转策略追加后的两个过滤口径：`close_price_forward_adj > price_avg_ma_3_6_12_24`，以及 `price_ma_60 > price_ma_114 > price_ma_250`。不覆盖 scoring points 重分配、回测收益验证、交易执行或 mart 指标重算。

## 规则变更

新增 metric policy：

```text
price_ma_114 -> fleur_marts.mart_stock_trend_indicator.price_ma_114
price_ma_250 -> fleur_marts.mart_stock_trend_indicator.price_ma_250
```

新增 pool filters：

```text
close_price_forward_adj > price_avg_ma_3_6_12_24
price_ma_60 > price_ma_114
price_ma_114 > price_ma_250
```

注意：现有得分项 `close_price_forward_adj < price_avg_ma_3_6_12_24 (+15)` 与新增过滤 `close_price_forward_adj > price_avg_ma_3_6_12_24` 互斥，因此本次规则下该得分项不会命中。

## 运行命令

```bash
make rearview-dev

cd engines
cargo run -q -p rearview -- sample-rule

curl -fsS -X POST http://127.0.0.1:34057/rearview/rule-sets ...
curl -fsS -X POST http://127.0.0.1:34057/rearview/rule-sets/{rule_set_id}/versions ...
curl -fsS -X POST http://127.0.0.1:34057/rearview/runs ...
curl -fsS http://127.0.0.1:34057/rearview/runs/{run_id}
curl -fsS http://127.0.0.1:34057/rearview/runs/{run_id}/chunks
curl -fsS http://127.0.0.1:34057/rearview/runs/{run_id}/days
```

Run request：

```json
{
  "rule_set_id": "df0755ac-3f4c-4863-ab05-a7a8d423b8fc",
  "start_date": "2026-04-01",
  "end_date": "2026-06-01",
  "top_n": 10
}
```

## 运行结果

```text
status = succeeded
compiled_sql_hash = 5287913eee26fe4bca1f40549eb90035a962c534c30d9ef5b636757079d21180
day_count = 40
pool_count = 35
signal_count = 35
```

Chunk：

| chunk_no | start_date | end_date | status | elapsed_ms | clickhouse_query_id |
|---:|---|---|---|---:|---|
| 0 | 2026-04-01 | 2026-06-01 | succeeded | 830 | `rearview-6fa431fe-d7b8-464f-be46-ecd68b1d32fb-chunk-0` |

非零候选交易日：

| trade_date | pool_count | signal_count |
|---|---:|---:|
| 2026-04-15 | 1 | 1 |
| 2026-04-16 | 1 | 1 |
| 2026-04-22 | 3 | 3 |
| 2026-04-23 | 1 | 1 |
| 2026-04-24 | 2 | 2 |
| 2026-04-27 | 3 | 3 |
| 2026-04-29 | 1 | 1 |
| 2026-04-30 | 2 | 2 |
| 2026-05-07 | 4 | 4 |
| 2026-05-08 | 1 | 1 |
| 2026-05-14 | 1 | 1 |
| 2026-05-18 | 5 | 5 |
| 2026-05-19 | 4 | 4 |
| 2026-05-22 | 4 | 4 |
| 2026-05-25 | 1 | 1 |
| 2026-05-28 | 1 | 1 |

`2026-06-01` 当日 `pool_count = 0`、`signal_count = 0`。

最近有信号交易日样本：

| trade_date | security_code | rank | score | close_price_forward_adj | price_avg_ma_3_6_12_24 | price_ma_60 | price_ma_114 | price_ma_250 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 2026-05-28 | 603916.SH | 1 | 15.0 | 14.73 | 14.66166666666665 | 12.045333333333321 | 11.622192982456113 | 10.850499331185201 |

## 验证

```bash
cd engines
cargo fmt --check
cargo test -p rearview
cargo clippy -p rearview --all-targets --all-features -- -D warnings
cargo run -p rearview -- catalog check

cd ../app/racingline
npm run typecheck
npm run lint
npm run test
```

结果：

```text
rearview catalog check -> passed, 21 metrics
rearview fmt/test/clippy -> passed
racingline typecheck/lint/test -> passed
```

## 结论

运行通过。追加 MA 链过滤后，2026-04-01 到 2026-06-01 的 40 个交易日内共产生 35 条 pool/signal 记录；最后一个有信号的交易日为 2026-05-28，2026-06-01 当天无信号。
