# Racingline Step4-Step5 Backtest Latency Baseline

日期：2026-06-25

范围：评估 RFC 0031 中 Step 4 到 Step 5 策略回测链路的待测试、待观察和待评估点。只采集指标和更新文档，不调整生产代码。

## 环境

| 项 | 值 |
|---|---|
| Rearview API | `http://127.0.0.1:34057` |
| Racingline | `http://127.0.0.1:5173` |
| PostgreSQL | Docker compose `postgres` service, database `rearview` |
| ClickHouse | Docker compose `clickhouse` service, version `26.5.1.882` |
| NATS | Docker compose `nats` service |
| Worker | `rearview-portfolio-worker run` dev process |

## 命令

连接检查：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml exec -T clickhouse clickhouse-client --query "SELECT version()"
docker compose --env-file .env -f deploy/docker-compose.yml exec -T postgres sh -lc 'psql -U "$POSTGRES_USER" -d rearview -Atc "SELECT count(*) FROM strategy_backtest_run;"'
```

PostgreSQL 阶段时间：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml exec -T postgres sh -lc 'psql -U "$POSTGRES_USER" -d rearview -P pager=off -c "SELECT r.strategy_backtest_run_id, r.period_key, r.status, r.created_at, o.published_at, r.started_at, r.completed_at FROM strategy_backtest_run r JOIN strategy_backtest_task_outbox o ON o.strategy_backtest_run_id = r.strategy_backtest_run_id WHERE r.strategy_backtest_run_id IN (...);"'
```

ClickHouse query log、parts 和 EXPLAIN：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml exec -T clickhouse clickhouse-client --format PrettyCompact --query "SELECT event_time_microseconds, query_id, query_duration_ms, read_rows, read_bytes, memory_usage FROM system.query_log WHERE type = 'QueryFinish' AND query_id LIKE 'strategy-backtest-%' ORDER BY event_time_microseconds"
docker compose --env-file .env -f deploy/docker-compose.yml exec -T clickhouse clickhouse-client --format PrettyCompact --query "SELECT database, table, count() AS active_parts, sum(rows) AS rows FROM system.parts WHERE active GROUP BY database, table ORDER BY active_parts DESC"
```

Step 5 wrapper HTTP：

```bash
curl -sS -o /dev/null -w "nav http_code=%{http_code} time_total=%{time_total}\n" "http://127.0.0.1:34057/rearview/strategy-backtests/a1d49988-c1d3-48a0-b1b1-f2fd9963052d/nav"
curl -sS -o /dev/null -w "rebalance-records http_code=%{http_code} time_total=%{time_total}\n" "http://127.0.0.1:34057/rearview/strategy-backtests/a1d49988-c1d3-48a0-b1b1-f2fd9963052d/rebalance-records"
curl -sS -o /dev/null -w "performance http_code=%{http_code} time_total=%{time_total}\n" "http://127.0.0.1:34057/rearview/strategy-backtests/a1d49988-c1d3-48a0-b1b1-f2fd9963052d/performance"
```

## 样本

使用已有成功 run `0eeb7f71-028a-43fb-af91-e3ec609e4e4b` 的 payload 作为请求来源，分别创建 1y 和 2y 受控样本。

| Period | client_request_id | run id | HTTP create |
|---|---|---|---:|
| 1y | `perf-rfc0031-1y-20260625T213613Z` | `145ceb26-b7e3-4581-a7f4-1aa5769b0789` | 202, 0.138225s |
| 2y | `perf-rfc0031-2y-20260625T213713Z` | `a1d49988-c1d3-48a0-b1b1-f2fd9963052d` | 202, 0.130941s |

## PostgreSQL 阶段时间

| Period | outbox publish | worker pickup | worker elapsed | backend total | status |
|---|---:|---:|---:|---:|---|
| 1y | 0.807s | 0.096s | 9.791s | 10.694s | succeeded |
| 2y | 1.454s | 0.014s | 22.170s | 23.638s | succeeded |

信号和覆盖规模：

| Period | generated candidates | top-n candidates | executable signals | signal dates | dropped signals | price bars |
|---|---:|---:|---:|---:|---:|---:|
| 1y | 2,729 | 995 | 990 | 228 | 5 | 154,696 |
| 2y | 3,384 | 1,431 | 1,426 | 390 | 5 | 374,548 |

结论：当前 Step 4 的页面跳转等待被 worker terminal status 绑定。仅按受控样本的后端完成时间估算，进入 Step 5 的阻塞下限约为 10.7s（1y）和 23.6s（2y），还未计入前端 1s 轮询粒度和 600ms cosmetic delay。create API 本身约 0.13s，若 create accepted 后立即进入 Step 5，点击到页面 shell 的等待可以降到百毫秒量级。

## ClickHouse Query Log

关键查询：

| Period | Query | duration | read_rows | read_bytes | memory |
|---|---|---:|---:|---:|---:|
| 1y | options trade dates | 18ms | 3,154,022 | 6.02 MiB | 5.35 MiB |
| 1y | create risk-free preflight | 8ms | 485 | 7.10 KiB | 6.21 MiB |
| 1y | worker trade dates | 12ms | 1,263,960 | 2.41 MiB | 6.26 MiB |
| 1y | screening chunk 0 | 136ms | 68,526 | 4.01 MiB | 11.08 MiB |
| 1y | screening chunk 1 | 2,180ms | 6,257,370 | 272.13 MiB | 774.59 MiB |
| 1y | price bars | 673ms | 2,548,129 | 264.96 MiB | 411.90 MiB |
| 2y | options trade dates | 17ms | 3,154,022 | 6.02 MiB | 5.31 MiB |
| 2y | create risk-free preflight | 7ms | 485 | 7.10 KiB | 6.21 MiB |
| 2y | worker trade dates | 17ms | 2,488,048 | 4.75 MiB | 5.27 MiB |
| 2y | screening chunk 0 | 2,316ms | 6,182,870 | 268.89 MiB | 753.33 MiB |
| 2y | screening chunk 1 | 1,724ms | 6,257,370 | 272.13 MiB | 768.93 MiB |
| 2y | price bars | 1,375ms | 4,976,096 | 514.89 MiB | 844.24 MiB |

ClickHouse heavy reads 合计约 3.0s（1y screening + price bars）和 5.4s（2y screening + price bars），低于 worker elapsed。price bars 完成后到下一条 benchmark 查询开始之间的无 ClickHouse 日志间隙为：

| Period | price bars finish | benchmark query start | gap |
|---|---|---|---:|
| 1y | 21:36:18.452286 | 21:36:24.239742 | 5.787s |
| 2y | 21:37:21.064320 | 21:37:36.456776 | 15.392s |

结论：仅优化 ClickHouse 查询不能解决全部后端耗时。2y 样本的最大未观测区间在 price bars 之后，下一步需要在 Rust worker 内部给 simulation、performance preparation、JSON serialization 和 write preparation 增加阶段计时。

## Price Bars 查询评估

`EXPLAIN indexes = 1` 使用 2y 样本实际 SQL 验证：

- quotes 表命中 `Min-Max`、`Partition` 和 `PrimaryKey`，主键条件包含 `trade_date` 和 774 个证券集合。
- trend 表也命中 `Min-Max`、`Partition` 和 `PrimaryKey`，过滤条件被推到 JOIN 右侧。
- `mart_stock_trend_indicator_daily` 在 2024-01-02 到 2025-12-31 区间没有重复 `(security_code, trade_date)` 键。

按 `clickhouse-best-practices`：

- Per `schema-pk-filter-on-orderby` 和 `schema-pk-prioritize-filters`，当前 price bars 查询已经使用排序键前缀过滤。
- Per `query-join-filter-before`，ClickHouse 已把过滤条件作用到 JOIN 两侧，暂不优先改写为双子查询。
- Per `query-join-use-any`，trend 表在样本区间键唯一，可以评估 `LEFT ANY JOIN`。

用 2y 样本 SQL 做 `FORMAT Null` 隔离对照：

| Query | duration | read_rows | read_bytes | memory |
|---|---:|---:|---:|---:|
| current `LEFT JOIN`, all trend columns | 551ms | 4,976,096 | 514.89 MiB | 277.88 MiB |
| `LEFT ANY JOIN`, all trend columns | 501ms | 4,976,096 | 514.89 MiB | 265.57 MiB |
| current `LEFT JOIN`, OHLC + `price_ma_10` | 269ms | 4,976,096 | 151.86 MiB | 98.15 MiB |
| `LEFT ANY JOIN`, OHLC + `price_ma_10` | 266ms | 4,976,096 | 151.86 MiB | 90.68 MiB |

结论：`LEFT ANY JOIN` 是小幅优化；动态投影趋势列收益更明确。当前 worker 固定读取多条 MA/EMA/BOLL 趋势列，而样本只需要 `price_ma_10` 作为 indicator stop metric 时，读字节减少约 70.5%，内存减少约 64.7%。

## Result Wrapper

2y 样本在 succeeded 后重新请求 Step 5 wrapper：

| Endpoint | HTTP time |
|---|---:|
| `/nav` | 0.112977s |
| `/rebalance-records` | 0.166866s |
| `/performance` | 0.096573s |

ClickHouse query_log 显示 wrapper 会重复读取 nav：

| Endpoint path | Representative query | duration | read_rows |
|---|---|---:|---:|
| `/nav` | `rearview-portfolio-read-nav-*` | 41ms | 10,081 |
| `/rebalance-records` | `rearview-portfolio-read-nav-*` | 14ms | 485 |
| `/performance` | `rearview-portfolio-read-nav-*` | 28ms | 485 |

结论：wrapper 重复 nav 读取是可删减点，但当前 HTTP 耗时在 0.10-0.17s，不是 Step 4 到 Step 5 等待 10-24s 的主瓶颈。

## ClickHouse Parts

结果表 active parts：

| Table | active_parts | rows | bytes |
|---|---:|---:|---:|
| `fleur_portfolio.portfolio_nav_daily` | 99 | 11,035 | 533.27 KiB |
| `fleur_calculation.calc_portfolio_closed_trade` | 92 | 7,610 | 717.44 KiB |
| `fleur_portfolio.portfolio_position_day` | 91 | 44,423 | 824.33 KiB |
| `fleur_portfolio.portfolio_trade` | 88 | 15,391 | 2.05 MiB |
| `fleur_portfolio.portfolio_order` | 87 | 15,437 | 1.06 MiB |
| `fleur_portfolio.portfolio_target` | 82 | 7,781 | 265.60 KiB |

按 `insert-batch-size`，当前远低于 per partition 3000 parts 的风险线，但每次回测对多张表做小批量 insert，parts/rows 比例偏高。按 `insert-async-small-batches`，如果 Step 5 使用频率上升，可评估 async insert 或低优先级明细合批。按 `insert-mutation-avoid-update`，当前 append-only `result_attempt_id` 写入模式是正确方向。

## 近两天聚合与异常样本

成功 run 聚合：

| Period | succeeded runs | avg outbox | p50 outbox | p95 outbox | avg pickup | p50 worker | p95 worker |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1y | 32 | 1.009s | 1.117s | 1.975s | 112.174s | 43.314s | 44.482s |
| 2y | 1 | 1.454s | 1.454s | 1.454s | 0.014s | 22.170s | 22.170s |

存在一个历史 active run：

| run id | status | created_at | heartbeat_at | progress |
|---|---|---|---|---|
| `88194a48-948e-4122-89ff-e0739df55dc6` | `running_clickhouse` | 2026-06-24 15:20:18.693937+00 | 2026-06-24 15:20:20.945405+00 | chunk 0, generated_signal_count 0 |

该 run 的 heartbeat 已明显过期，会污染 pickup/queue 聚合指标。后续性能面板应将 stale active run 单独标记，不应混入成功路径 p50/p95。

## 下一步设计建议

1. P0：修改 Step 4 handoff，create API 返回 202 后立即进入 Step 5。受控样本显示可把页面跳转等待从 10.7s/23.6s 级别降到约 0.13s 的 create response 级别。
2. P0：删除 Step 4 中等待 terminal status 的手写轮询和 600ms cosmetic delay，状态轮询交给 Step 5 的现有 `useStrategyBacktestQuery()`。
3. P1：把 outbox idle publish 延迟作为后端体验优化项。受控样本为 0.807s/1.454s，近两天 1y p95 为 1.975s，符合当前 2s idle sleep 预期。
4. P1：给 Rust worker 增加阶段计时。当前最大缺口是 price bars 后 5.8s/15.4s 的无 ClickHouse 日志区间，需要分解 simulation、performance preparation、serialization 和 write preparation。
5. P2：优先实现 price bars 动态投影趋势列，再评估 `LEFT ANY JOIN`。动态投影在 2y 隔离测试中把 read_bytes 从 514.89 MiB 降到 151.86 MiB，把 memory 从 277.88 MiB 降到 98.15 MiB。
6. P2：result wrapper 可合并或缓存 nav 读取，但它不是当前跳转阻塞主因，适合作为 Step 5 首屏细节优化。
7. P3：监控 ClickHouse parts 增长。当前未到风险线，但小批量多表写入会随使用量累积，需要在使用频率上升前准备 async insert 或合批策略。
