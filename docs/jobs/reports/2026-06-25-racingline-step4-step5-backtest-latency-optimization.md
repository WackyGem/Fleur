# Racingline Step4-Step5 Backtest Latency Optimization

日期：2026-06-25

范围：验收 Plan 0056 的 Step 4 到 Step 5 回测链路瘦身实现，覆盖 Racingline handoff、Rearview status/compact API、worker price bars 动态投影、worker timing、outbox 唤醒和 stale active run 诊断。

## 环境

| 项 | 值 |
|---|---|
| Rearview API | `http://127.0.0.1:34057` |
| Racingline | `http://127.0.0.1:5173` |
| PostgreSQL | Docker compose `postgres` service, database `rearview` |
| ClickHouse | Docker compose `clickhouse` service |
| NATS | Docker compose `nats` service |
| Worker | `rearview-portfolio-worker run` dev process |

## 实现摘要

- Step 4 `openBacktest()` 在 `POST /rearview/strategy-backtests` 返回 `202 Accepted` 后立即写入 full run cache 并进入 Step 5，不再等待 worker terminal。
- Step 5 以父页面 `activeBacktestRun` 为 active run owner，移除自动首跑提交和手写 terminal polling；显式重新回测仍创建新 run。
- Rearview 新增 `GET /rearview/strategy-backtests/{id}/status`，并为 `/nav`、`/performance`、`/rebalance-records` 增加 `view=ui` compact response。
- Worker price bars 查询按 indicator stop-loss metrics 动态投影趋势列；无 indicator metrics 时不 JOIN trend 表。
- Worker summary 写入 `worker_timing` 阶段耗时。
- Outbox dispatcher 保留 PG outbox 事务边界，create 成功后通过进程内 notify 唤醒 dispatcher；日志包含 pending scan、publish success/fail、NATS sequence 和 created-to-published elapsed。
- 新增只读诊断 endpoint：`GET /rearview/strategy-backtests/diagnostics/stale-active?limit=...`。

## 样本

使用既有成功 run `0eeb7f71-028a-43fb-af91-e3ec609e4e4b` 的 stored `rule_snapshot` 和 `execution_config` 构造 1y/2y 新 run。

| Period | client_request_id | run id | HTTP create |
|---|---|---|---:|
| 1y | `perf-0056-1y-20260625T225302Z` | `0794a193-135a-40c8-aa12-4338d4410c52` | 202, 0.113224s |
| 2y | `perf-0056-2y-20260625T225302Z` | `ed8594ab-1cde-48ff-ac18-06375a83c14f` | 202, 0.105489s |

## PostgreSQL 阶段时间

| Period | outbox publish | worker pickup | worker elapsed | backend total | status |
|---|---:|---:|---:|---:|---|
| 1y | 0.039s | 0.013s | 6.477s | 6.529s | succeeded |
| 2y | 0.031s | 6.238s | 13.797s | 20.066s | succeeded |

2y 的 pickup 等待包含单 worker 先处理 1y run 的排队时间；outbox publish 本身已降到 31-39ms。

## Payload

| Period | full run | status | nav UI | performance UI | rebalance UI |
|---|---:|---:|---:|---:|---:|
| 1y | 9,468 bytes | 578 bytes | 23,506 bytes | 542 bytes | 23,701 bytes |
| 2y | 9,470 bytes | 578 bytes | 46,725 bytes | 539 bytes | 45,632 bytes |

对比 full result wrapper：

| Period | nav full | performance full |
|---|---:|---:|
| 1y | 32,481 bytes | 3,813 bytes |
| 2y | 64,593 bytes | 3,810 bytes |

## ClickHouse Query Log

| Period | query | duration | read_rows | read_bytes | memory |
|---|---|---:|---:|---:|---:|
| 1y | trade dates | 10ms | 1,263,960 | 2.41 MiB | 6.26 MiB |
| 1y | screening chunk 0 | 81ms | 68,526 | 4.01 MiB | 12.31 MiB |
| 1y | screening chunk 1 | 2,162ms | 6,257,370 | 272.13 MiB | 768.31 MiB |
| 1y | price bars | 346ms | 2,548,129 | 77.59 MiB | 142.55 MiB |
| 2y | trade dates | 17ms | 2,488,048 | 4.75 MiB | 5.27 MiB |
| 2y | screening chunk 0 | 2,069ms | 6,182,870 | 268.89 MiB | 750.56 MiB |
| 2y | screening chunk 1 | 1,901ms | 6,257,370 | 272.13 MiB | 766.45 MiB |
| 2y | price bars | 601ms | 4,976,096 | 151.86 MiB | 251.12 MiB |

Per `schema-pk-filter-on-orderby`、`schema-pk-prioritize-filters` 和 `query-join-filter-before`，动态投影 SQL 保留 quotes/trend 两侧 `trade_date` 与 `security_code` 过滤。`query-join-use-any` 仍未作为本次主改动，因为动态投影已经提供主要收益。

## Worker Timing

| Period | total | signal materialization | price bars | simulation | writes |
|---|---:|---:|---:|---:|---:|
| 1y | 6,471ms | 2,536ms | 2,096ms | 1,031ms | 767ms |
| 2y | 13,790ms | 4,385ms | 4,302ms | 4,212ms | 838ms |

`worker_timing.stages_ms` 已包含 `simulation`、`benchmark_query`、`risk_free_query`、`performance_calculation`、`output_serialization_write_preparation`、`clickhouse_write_portfolio_facts`、`postgres_insert_metric_config`、`clickhouse_write_calculation_outputs` 和 `clickhouse_write_run_snapshot`。

## Stale Active 诊断

`GET /rearview/strategy-backtests/diagnostics/stale-active?limit=10` 返回历史 stale active run：

| run id | status | claim_expires_at | stale_seconds |
|---|---|---|---:|
| `88194a48-948e-4122-89ff-e0739df55dc6` | `running_clickhouse` | 2026-06-24T15:35:20.834031Z | 112710 |

性能聚合应继续把 stale active run 单独列出，不混入成功路径 pickup/worker p50/p95。

## Browser Smoke

命令：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
playwright-cli tab-new http://127.0.0.1:5173/strategies
playwright-cli snapshot
playwright-cli console
playwright-cli requests
```

结果：

- `/strategies` 页面加载成功，title 为 `Racingline`。
- Console errors: 0；warnings: 0。
- 初始 API `GET /rearview/metrics` 和 `GET /rearview/market-fee-templates/default?market=CN_A_SHARE` 均返回 200。

## 验收结论

- Step 4 到 Step 5 的代码阻塞项已从 worker terminal 变为 create accepted；本轮 create HTTP 为 0.105-0.113s。浏览器 smoke 覆盖页面可加载和无 console error，未做完整手点链路计时。
- outbox publish 从 baseline 0.807s/1.454s 降到 0.031s/0.039s，满足 p95 <= 0.5s 的样本目标。
- status polling payload 从约 9.47KB full run 降到 578B，满足 <= 1KB。
- 2y nav UI payload 为 46.7KB，performance UI payload 为 539B，满足计划目标。
- 2y price bars read_bytes 为 151.86 MiB，达到 <= 180 MiB；live memory 从 baseline 844.24 MiB 降到 251.12 MiB，明显下降，但不等同于隔离 FORMAT Null 的 <= 120 MiB 目标。
- worker summary 已能解释 simulation/performance/write preparation 等阶段耗时。
