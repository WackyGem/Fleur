# Rearview N Structure Low Reversal Smoke Run

日期：2026-06-12

## 范围

- 服务：Rust `rearview` HTTP service
- 规则：`n_structure_low_reversal_screen`
- 短区间 smoke：`2026-05-20` 到 `2026-06-01`
- `top_n`：3
- 多年 dry explain：`2021-01-01` 到 `2025-12-31`

## 环境

本次运行复用根目录 `.env` 和 `deploy/docker-compose.yml` 的本地设施。

使用的环境变量名：

- PostgreSQL：`POSTGRES_USER`、`POSTGRES_PASSWORD`、`POSTGRES_PORT`、`PIPELINE_DATABASE_URL`、`REARVIEW_DATABASE_URL`
- ClickHouse：`CLICKHOUSE_HOST`、`CLICKHOUSE_PORT`、`CLICKHOUSE_USER`、`CLICKHOUSE_PASSWORD`
- Rearview：`REARVIEW_HTTP_BIND`、`REARVIEW_CLICKHOUSE_MARTS_DATABASE`、`REARVIEW_CHUNK_SMALL_RANGE_TRADING_DAYS`、`REARVIEW_CLICKHOUSE_MAX_EXECUTION_TIME_SECONDS`、`REARVIEW_CLICKHOUSE_MAX_ROWS_TO_READ`、`REARVIEW_CLICKHOUSE_MAX_BYTES_TO_READ`

Compose 状态：

```text
mono-fleur-postgres: Up, healthy, 127.0.0.1:34054
mono-fleur-clickhouse: Up, healthy, 127.0.0.1:34052 HTTP / 34053 native
```

## 命令

迁移：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=pipeline upgrade head
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
```

Catalog：

```bash
cd engines
cargo run -p rearview -- catalog check
cargo run -p rearview -- catalog sync
```

结果：

```text
metric catalog check passed: 14 metrics
metric catalog sync completed: 14 metrics, 14 rows affected
```

服务：

```bash
cd engines
cargo run -p rearview -- serve
curl -fsS http://127.0.0.1:34057/healthz
```

结果：

```json
{"status":"ok"}
```

## 规则版本

创建规则集和规则版本后得到：

```text
rule_set_id: fa457364-0c32-4e27-82dd-edc60601420b
rule_version_id: a952be67-2ff4-491d-a334-1d4e5ac6733d
rule_hash: b821387378c8c381656baa53b55b77637c218fbed96f200d8f68c6f7091e8e57
version_no: 1
status: active
```

Explain 确认代表性用例需要 5 张 mart、14 个指标：

```text
fleur_marts.mart_stock_momentum_indicator
fleur_marts.mart_stock_price_pattern_daily
fleur_marts.mart_stock_quotes_daily
fleur_marts.mart_stock_trend_indicator
fleur_marts.mart_stock_volume_indicator
```

## 多年 Dry Explain

请求 `POST /rearview/explain`，payload 包含 `{rule, start_date, end_date, top_n}`。

日期范围：`2021-01-01` 到 `2025-12-31`

返回 chunk plan：

```text
0: 2021-01-01 -> 2021-12-31
1: 2022-01-01 -> 2022-12-31
2: 2023-01-01 -> 2023-12-31
3: 2024-01-01 -> 2024-12-31
4: 2025-01-01 -> 2025-12-31
```

未执行多年真实 run；第一版验收先记录 explain 和自然年 chunk plan。短区间真实 run 已完成。

## 短区间 Run

请求：

```json
{
  "rule_set_id": "fa457364-0c32-4e27-82dd-edc60601420b",
  "start_date": "2026-05-20",
  "end_date": "2026-06-01",
  "top_n": 3
}
```

结果：

```text
run_id: a4470e63-6fd3-46ce-9dab-8802c84cef26
status: succeeded
compiled_sql_hash: f165d15c2218cbccac7bc6e0165acf2a746f5eedbf38559c82acfc5835b4ea6a
summary.day_count: 9
summary.pool_count: 389
summary.signal_count: 27
```

Chunk：

```text
chunk_no: 0
range: 2026-05-20 -> 2026-06-01
status: succeeded
clickhouse_query_id: rearview-a4470e63-6fd3-46ce-9dab-8802c84cef26-chunk-0
elapsed_ms: 852
```

Day sample：

```text
2026-05-20: pool_count=19, signal_count=3
2026-06-01: pool_count=51, signal_count=3
```

交易日覆盖校验：

```text
ClickHouse anchor mart distinct trade_date count: 9
Rearview run_day rows: 9
zero_pool_days in this sample: 0
```

Signal sample：

```text
trade_date: 2026-05-20
security_code: 688016.SH
rank: 1
score: 50.25
points: below_short_average=15, kdj_j_below_minus_15=35, near_boll_dn=0.25
raw_values include: close_price, boll_dn_20_2, kdj_j_value, rsi_6, volume, volume_ma_5, n_structure_20_is_valid
```

## ClickHouse 观测

短区间 SQL 执行了 `EXPLAIN indexes = 1`。结果显示每个 mart CTE 在 join 前按 `trade_date` 过滤，并使用 partition / primary key 路径；`mart_stock_quotes_daily` 也在该短区间使用了 `trade_date` 条件的索引裁剪。

`system.query_log` 摘要：

```text
query_id: rearview-a4470e63-6fd3-46ce-9dab-8802c84cef26-chunk-0
type: QueryFinish
read_rows: 723263
read_bytes: 29732731
memory_usage: 56402176
query_duration_ms: 171
```

## 修复记录

1. ClickHouse `JSONEachRow` 将 `signal_rank <= top_n` 输出为 `1/0`，Rust `bool` 反序列化失败。已在 `ScreeningRow.is_buy_signal` 上增加 ClickHouse bool 兼容反序列化，并补充单元测试。
2. `run_chunk.elapsed_ms` 初始未写入。已在 `set_chunk_finished` 中从 `started_at` / `completed_at` 派生写入。
3. `POST /rearview/explain` 初始不返回 chunk plan。已支持 `{rule, start_date, end_date, top_n}` 请求，并新增 `GET /rearview/runs/{run_id}/chunks` 查询 chunk 状态、query id 和耗时。
4. 完成审计发现 `run_day` 只会为有入池证券的交易日写行。已新增 anchor mart 交易日解析和 run_day 占位，零入池日也会作为日粒度事实保留并计数为 0。

## 验证

```bash
cd engines
cargo fmt --all --check
cargo test -p rearview --no-fail-fast
cargo clippy -p rearview --all-targets --all-features -- -D warnings
```

以上命令均通过。
