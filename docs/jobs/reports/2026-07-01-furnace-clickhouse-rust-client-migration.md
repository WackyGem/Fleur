# Furnace ClickHouse Rust Client Migration

日期：2026-07-01

状态：Passed，`validation=false` 已修复并启用

## 范围

本次迁移覆盖 Furnace 全部股票技术指标：

| CLI 指标 | 输出表 |
|---|---|
| `kdj` | `fleur_calculation.calc_stock_kdj_daily` |
| `ma` | `fleur_calculation.calc_stock_ma_daily` |
| `rsi` | `fleur_calculation.calc_stock_rsi_daily` |
| `boll` | `fleur_calculation.calc_stock_boll_daily` |
| `macd` | `fleur_calculation.calc_stock_macd_daily` |
| `price-pattern` | `fleur_calculation.calc_stock_price_pattern_daily` |

迁移结果：

- `furnace-io` 使用官方 `clickhouse = "0.15.1"` crate，开启 `time` feature。
- Furnace 生产路径通过 ClickHouse HTTP client 访问 `FURNACE_CLICKHOUSE_URL`。
- 生产路径不再依赖宿主机 `clickhouse-client`、Docker exec wrapper、`FURNACE_CLICKHOUSE_CLIENT` 或 `CLICKHOUSE_NATIVE_PORT`。
- 输入读取、scalar 查询、DDL/partition replace 和写入路径改为 typed row I/O。
- `furnace-core` 仍只包含纯指标计算，不引入 ClickHouse、Tokio、serde 或环境变量依赖。

## 配置

`.env` 和 `.env.example` 已登记 Furnace HTTP client 入口：

```dotenv
FURNACE_CLICKHOUSE_URL=http://127.0.0.1:34052
FURNACE_CLICKHOUSE_VALIDATE_SCHEMA=false
```

`CLICKHOUSE_NATIVE_PORT` 仍保留给 Docker Compose 暴露 ClickHouse native 端口和人工运维使用，但不是 Furnace 连接入口。

## 代码验证

迁移过程中修复了 typed validation scalar 的 ClickHouse 类型不一致问题：

- `replace-cascade` duplicate-key validation 使用 `toUInt64(coalesce(sum(duplicates), 0)) AS value`，避免 `sum(duplicates)` 在无重复时返回与 Rust typed row 不匹配的 signed/nullable scalar。
- KDJ runner tests 增加了该 SQL 的断言；BOLL、MA、RSI、MACD 和 Price Pattern 也补齐了 replace-cascade runner 测试。
- 2026-07-01 后续修复：KDJ、MA、RSI 和 MACD previous-state 查询中，SQL 已用 `IS NOT NULL` 过滤但 ClickHouse 输出类型仍为 `Nullable(Float64)` 的状态列，在 SELECT 输出边界统一使用 `assumeNotNull(...) AS ...`。该修复避免 `FURNACE_CLICKHOUSE_VALIDATE_SCHEMA=false` 时纯 `RowBinary` 读取 non-null Rust row 发生字节错位并报 `NotEnoughData`。

Rust 门禁：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：全部通过。

Dagster Furnace resource 和 definitions 门禁：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/unit/resources/test_furnace.py

cd scheduler
uv run dg check defs
```

结果：

- `pytest`：24 passed。
- `dg check defs`：passed。

## ClickHouse Dry-Run Smoke

命令：

```bash
cd engines
set -a
. ../.env
set +a
for indicator in kdj ma rsi boll macd price-pattern; do
  FURNACE_CLICKHOUSE_URL="${FURNACE_CLICKHOUSE_URL}" cargo run -p furnace --quiet -- "$indicator" \
    --from 2026-06-01 \
    --to 2026-06-10 \
    --symbols 601368.SH \
    --mode dry-run \
    --output-format json
done
```

范围：

- 证券：`601368.SH`
- 请求区间：`2026-06-01..2026-06-10`
- 模式：`dry-run`
- 连接：`FURNACE_CLICKHOUSE_URL=http://127.0.0.1:34052`

结果：

| 指标 | `input_from` | `input_to` | `input_rows` | `output_rows` | `writes_applied` |
|---|---:|---:|---:|---:|---|
| `kdj` | `2026-04-21` | `2026-06-10` | 34 | 8 | `false` |
| `ma` | `2021-01-04` | `2026-06-10` | 1,315 | 8 | `false` |
| `rsi` | `2021-01-04` | `2026-06-10` | 1,315 | 8 | `false` |
| `boll` | `2026-03-18` | `2026-06-10` | 57 | 8 | `false` |
| `macd` | `2021-01-04` | `2026-06-10` | 1,315 | 8 | `false` |
| `price-pattern` | `2021-01-04` | `2026-06-10` | 1,315 | 8 | `false` |

性能摘要：

| 指标 | `total_ms` | `read_input_ms` | `compute_ms` | `parallelism` |
|---|---:|---:|---:|---|
| `kdj` | 70 | 19 | 0 | `serial` |
| `ma` | 196 | 150 | 3 | `serial` |
| `rsi` | 119 | 69 | 1 | `serial` |
| `boll` | 59 | 12 | 0 | `serial` |
| `macd` | 104 | 64 | 1 | `serial-streaming` |
| `price-pattern` | 206 | 163 | 4 | `serial` |

## 写入验证边界

本次已对 dev ClickHouse 执行真实 `replace-cascade` 写入 smoke。命令：

```bash
cd engines
set -a
. ../.env
set +a
for indicator in kdj ma rsi boll macd price-pattern; do
  run_id="furnace_http_client_smoke_20260701_${indicator//-/_}"
  FURNACE_CLICKHOUSE_URL="${FURNACE_CLICKHOUSE_URL}" cargo run -p furnace --quiet -- "$indicator" \
    --from 2026-06-01 \
    --to 2026-06-10 \
    --symbols 601368.SH \
    --mode replace-cascade \
    --run-id "$run_id" \
    --insert-batch-size 10000 \
    --output-format json
done
```

结果：

| 指标 | `input_rows` | `output_rows` | `effective_to` | staging validation | replace partitions | `writes_applied` |
|---|---:|---:|---|---|---|---|
| `kdj` | 46 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `ma` | 1,327 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `rsi` | 1,327 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `boll` | 69 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `macd` | 1,327 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `price-pattern` | 1,327 | 20 | `2026-06-29` | passed | `[2026]` | `true` |

目标表核验：

| 表 | rows | unique keys | duplicate keys | min date | max date | smoke rows |
|---|---:|---:|---:|---|---|---:|
| `fleur_calculation.calc_stock_kdj_daily` | 20 | 20 | 0 | `2026-06-01` | `2026-06-29` | 20 |
| `fleur_calculation.calc_stock_ma_daily` | 20 | 20 | 0 | `2026-06-01` | `2026-06-29` | 20 |
| `fleur_calculation.calc_stock_rsi_daily` | 20 | 20 | 0 | `2026-06-01` | `2026-06-29` | 20 |
| `fleur_calculation.calc_stock_boll_daily` | 20 | 20 | 0 | `2026-06-01` | `2026-06-29` | 20 |
| `fleur_calculation.calc_stock_macd_daily` | 20 | 20 | 0 | `2026-06-01` | `2026-06-29` | 20 |
| `fleur_calculation.calc_stock_price_pattern_daily` | 20 | 20 | 0 | `2026-06-01` | `2026-06-29` | 20 |

staging 清理核验：

```sql
SELECT count()
FROM system.tables
WHERE name LIKE '%staging%furnace_http_client_smoke%'
```

结果：`0`。当前环境中仍有两个更早的无关 staging 表，名称包含 `9d96bd4e...`，不属于本次 smoke。

写入路径的覆盖来自迁移后的 Rust runner tests：

- staging 表创建和清理。
- typed rows 插入分批。
- duplicate-key validation。
- append-latest 写入保护。
- replace-cascade 年度 partition replace 语义。
- dry-run 不写入 ClickHouse。

## 性能对比

迁移前旧 CLI executor baseline 从当前 Git HEAD 导出到临时目录后执行，使用同一证券和区间。迁移后 HTTP client dry-run 的非耗时 summary 与 baseline 一致：

| 指标 | baseline `input_rows` | current `input_rows` | baseline `output_rows` | current `output_rows` |
|---|---:|---:|---:|---:|
| `kdj` | 34 | 34 | 8 | 8 |
| `ma` | 1,315 | 1,315 | 8 | 8 |
| `rsi` | 1,315 | 1,315 | 8 | 8 |
| `boll` | 57 | 57 | 8 | 8 |
| `macd` | 1,315 | 1,315 | 8 | 8 |
| `price-pattern` | 1,315 | 1,315 | 8 | 8 |

代表性 full-market BOLL dry-run 性能：

| 构建 | schema validation | `total_ms` | 结论 |
|---|---:|---:|---|
| old CLI baseline, debug | - | 4,981 | 迁移前参考 |
| HTTP client, debug | `true` | 14,617 | debug 下明显慢于旧 CLI |
| HTTP client, debug | `false` | 13,036 | 关闭 validation 有改善但仍慢 |
| old CLI baseline, release | - | 3,153 | 迁移前 release 参考 |
| HTTP client, release | `true` | 3,760 | 约慢 19% |
| HTTP client, release | `false` | 3,072 | 与旧 CLI release 基本持平 |

决策：

- `.env` / `.env.example` 已切换为 `FURNACE_CLICKHOUSE_VALIDATE_SCHEMA=false`。
- `false` 使用官方 client 的纯 `RowBinary` 路径；性能实验中 BOLL full-market release dry-run 从 `validation=true` 的 3,760ms 降到 3,072ms，与旧 CLI release baseline 3,153ms 基本持平。
- 关闭 validation 后，typed SELECT 的 SQL 输出类型必须与 Rust row 严格一致。对已用 `IS NOT NULL` 过滤的 nullable 状态列，查询必须显式使用 `assumeNotNull(...) AS ...`，不能只依赖 WHERE 条件。

## Validation=False Fix Smoke

修复后执行全指标 `validation=false` `replace-cascade` smoke：

```bash
cd engines
set -a
. ../.env
set +a
for indicator in kdj ma rsi boll macd price-pattern; do
  run_id="furnace_validate_false_fix_20260701_${indicator//-/_}"
  FURNACE_CLICKHOUSE_VALIDATE_SCHEMA=false FURNACE_CLICKHOUSE_URL="${FURNACE_CLICKHOUSE_URL}" \
    cargo run -p furnace --quiet -- "$indicator" \
      --from 2026-06-01 \
      --to 2026-06-10 \
      --symbols 601368.SH \
      --mode replace-cascade \
      --run-id "$run_id" \
      --insert-batch-size 10000 \
      --output-format json
done
```

结果：

| 指标 | `input_rows` | `output_rows` | `effective_to` | staging validation | replace partitions | `writes_applied` |
|---|---:|---:|---|---|---|---|
| `kdj` | 46 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `ma` | 1,327 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `rsi` | 1,327 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `boll` | 69 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `macd` | 1,327 | 20 | `2026-06-29` | passed | `[2026]` | `true` |
| `price-pattern` | 1,327 | 20 | `2026-06-29` | passed | `[2026]` | `true` |

staging 清理核验：

```sql
SELECT count()
FROM system.tables
WHERE database = 'fleur_calculation'
  AND name LIKE '%staging%furnace_validate_false_fix_20260701%'
```

结果：`0`。

## 文档收口

- [../../architecture/furnace.md](../../architecture/furnace.md) 已更新为当前事实入口。
- [../../../engines/README.md](../../../engines/README.md) 已更新 engines 工作区和 Furnace CLI 配置口径。
- [../../plans/archive/0068-furnace-clickhouse-rust-client-migration-plan.md](../../plans/archive/0068-furnace-clickhouse-rust-client-migration-plan.md) 已归档为 Completed。

## 结论

Furnace 全部股票技术指标已完成官方 ClickHouse Rust client 迁移。Dagster 调用 Furnace 时只需要传递或继承 HTTP ClickHouse 配置；本机缺少 `clickhouse-client` 不再影响 Furnace CLI 和 Dagster Furnace assets。
