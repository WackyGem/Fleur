# Plan 0068: Furnace official ClickHouse Rust client 一刀切迁移计划

日期：2026-07-01

状态：Completed

## 关联文档

- [../../architecture/furnace.md](../../architecture/furnace.md)
- [../../../engines/README.md](../../../engines/README.md)
- [../../RFC/archive/0016-rust-furnace-compute-engine.md](../../RFC/archive/0016-rust-furnace-compute-engine.md)
- [../../jobs/reports/2026-07-01-furnace-clickhouse-rust-client-migration.md](../../jobs/reports/2026-07-01-furnace-clickhouse-rust-client-migration.md)
- [../../jobs/reports/2026-06-07-furnace-kdj-smoke-run.md](../../jobs/reports/2026-06-07-furnace-kdj-smoke-run.md)
- [../../jobs/reports/2026-06-08-furnace-ma-full-market-parallel-validation.md](../../jobs/reports/2026-06-08-furnace-ma-full-market-parallel-validation.md)
- [../../jobs/reports/2026-06-08-furnace-boll-full-market-parallel-validation.md](../../jobs/reports/2026-06-08-furnace-boll-full-market-parallel-validation.md)
- [../../jobs/reports/2026-06-09-furnace-price-pattern-full-market-validation.md](../../jobs/reports/2026-06-09-furnace-price-pattern-full-market-validation.md)
- [../../jobs/reports/2026-06-10-furnace-macd-performance-baseline.md](../../jobs/reports/2026-06-10-furnace-macd-performance-baseline.md)

## 完成记录

本计划已在 2026-07-01 完成并归档。当前事实入口是 [../../architecture/furnace.md](../../architecture/furnace.md) 和 [../../../engines/README.md](../../../engines/README.md)；验证细节见 [../../jobs/reports/2026-07-01-furnace-clickhouse-rust-client-migration.md](../../jobs/reports/2026-07-01-furnace-clickhouse-rust-client-migration.md)。

## 背景

Furnace 当前通过 `clickhouse-client` 子进程访问 ClickHouse。`engines/crates/furnace-io/src/clickhouse.rs` 的 `ClickHouseCliExecutor` 从 `FURNACE_CLICKHOUSE_CLIENT`、`CLICKHOUSE_CLIENT` 或默认 `clickhouse-client` 解析执行命令；本机未安装 client 时，Dagster Furnace 资产会在运行到 `fleur_calculation__calc_stock_boll_daily` 等指标时失败：

```text
failed to run clickhouse-client: No such file or directory
```

历史运行报告曾通过 `docker exec -i fleur-clickhouse clickhouse-client` 绕过本机依赖，但这会把 Furnace 运行环境绑定到 Docker 容器名、native 端口和宿主机/容器网络差异。下一步目标是移除外部 `clickhouse-client` 运行时依赖，改为 Furnace binary 内部使用官方 Rust client。

本计划选择官方 crate：

```toml
clickhouse = "0.15.1"
```

当前 crates.io 元数据：

- 描述：Official Rust client for ClickHouse DB。
- `rust-version = 1.89.0`；当前 engines toolchain 为 Rust 1.95.0，满足要求。
- transport：HTTP。
- 数据格式：默认 `RowBinaryWithNamesAndTypes`，可通过 `Client::with_validation(false)` 切到 `RowBinary`。
- 支持 typed `SELECT`、typed `INSERT`、DDL `execute()`、LZ4/ZSTD、TLS 和 `test-util` mocks。

> 注：本次会话没有暴露 Context7 MCP resolve/query 工具；crate 事实来自 `cargo info clickhouse` 和本机 crates.io 缓存中的 `clickhouse-0.15.1/README.md`。实施前应再次用官方文档或 `cargo info clickhouse` 确认版本。

## 工作负载和规则依据

### Workload Summary

| 维度 | 结论 |
|---|---|
| workload | market data / financial services；A 股日频行情技术指标批量计算 |
| latency target | Dagster 历史回填和日常下游可接受秒级到分钟级批处理延迟 |
| data shape | `security_code, trade_date` 粒度；按证券分组计算，按年度分区写入 calculation 表 |
| primary query patterns | 从 `fleur_intermediate` 读取日频输入；写入 `fleur_calculation`；dbt thin wrapper 消费 |
| operational constraints | 不依赖本机或容器 `clickhouse-client`；保持 staging + `REPLACE PARTITION` 幂等写入 |

### Rules Checked

- Per `decision-ingestion-strategy`：Furnace 能自然按计算结果批量写入，应使用 direct batched inserts，不使用 Kafka 或 async insert。
- Per `insert-batch-size`：生产写入保持 10K-100K 行健康批次，保留现有默认 `insert_batch_size = 10000`。
- Per `insert-format-native`：Native 格式最快，RowBinary 是高效替代；官方 `clickhouse` crate 当前使用 RowBinary/RowBinaryWithNamesAndTypes over HTTP，迁移后要用基准验证性能。
- Per `insert-async-small-batches`：Furnace 不是高频小批次 producer，不启用 async insert 作为首选。
- Per `insert-mutation-avoid-update` / `insert-mutation-avoid-delete`：继续使用 staging + 年度 `REPLACE PARTITION`，不引入 `ALTER UPDATE` 或 `ALTER DELETE`。
- Per `schema-partition-lifecycle`：年度分区继续服务历史修正、分区替换和生命周期管理，不把 partition 当作普通查询优化开关。

## 目标

1. 一刀切迁移所有 Furnace 指标：KDJ、MA、RSI、BOLL、MACD、Price Pattern 全部改用官方 `clickhouse` crate。
2. 移除 Furnace 生产路径对外部 `clickhouse-client`、Docker exec、`FURNACE_CLICKHOUSE_CLIENT` 和 `CLICKHOUSE_NATIVE_PORT` 的依赖。
3. 通过 HTTP 端口访问 ClickHouse，默认从 `FURNACE_CLICKHOUSE_URL` 读取；未配置时由 `CLICKHOUSE_HOST` + `CLICKHOUSE_PORT` 推导。
4. 保持 `furnace-core` 不依赖 ClickHouse、Tokio、serde 或环境变量。
5. 保持 `furnace-io` 拥有 ClickHouse SQL、DDL、输入读取、typed insert、staging、partition replace 和 summary 统计。
6. 保持现有 CLI 对外子命令和 Dagster asset 配置语义不变，除非该配置只服务旧 `clickhouse-client`。
7. 保持 direct batched inserts，默认批量大小仍为 `10000`。
8. 完成后所有指标的 dry-run、append-latest 和 replace-cascade 行为与迁移前一致。
9. 在 `.env` 和 `.env.example` 中登记 Furnace HTTP client 所需配置，避免继续依赖 native port 或 Docker exec 变量。
10. 输出迁移运行报告，记录 all-indicator smoke、full-market 或代表性历史窗口基准。

## 非目标

1. 不分指标长期灰度，不保留 KDJ 走 HTTP、BOLL 走 CLI 这类混合生产路径。
2. 不引入新的 native TCP Rust client；本计划只选官方 `clickhouse` crate。
3. 不重写 KDJ、MA、RSI、BOLL、MACD 或 Price Pattern 公式。
4. 不改 calculation 表字段契约、排序键、分区策略或 dbt wrapper 口径。
5. 不把 Furnace 改为常驻服务；仍保持 CLI binary 被 Dagster resource 调用。
6. 不引入 async insert、Kafka、物化视图或消息队列。
7. 不为了兼容未知字段写多路径 fallback；typed rows 必须由当前 DDL、SQL 和测试确认。
8. 不在生产代码中保留旧 `clickhouse-client` fallback。开发阶段可临时保留对照代码，但完成前必须移除或降为测试 fixture。

## 当前事实基线

### Rust 边界

| Crate | 当前职责 | 迁移动作 |
|---|---|---|
| `furnace-core` | 纯指标计算，无 ClickHouse 依赖 | 不增加外部 I/O 依赖 |
| `furnace-io` | ClickHouse trait、CLI executor、RowBinary、SQL、staging/replace、runner | 引入官方 `clickhouse` crate、typed rows 和 HTTP executor |
| `furnace` | CLI 子命令、请求解析、JSON summary | 改用 HTTP executor；移除 CLI executor 默认构造 |

### 当前环境变量事实

| 变量 | 当前用途 | 0068 口径 |
|---|---|---|
| `CLICKHOUSE_HTTP_PORT` | Docker Compose 暴露 ClickHouse HTTP `8123` 到宿主机的端口，当前为 `34052` | 可用于人工判断 URL，但 Furnace 不直接读取它作为连接端口 |
| `CLICKHOUSE_PORT` | dbt、Rearview 和 scheduler ClickHouse HTTP client 使用的端口，当前为 `34052` | Furnace 在未设置 `FURNACE_CLICKHOUSE_URL` 时可用它推导 HTTP URL |
| `CLICKHOUSE_NATIVE_PORT` | Docker Compose 暴露 ClickHouse native `9000` 到宿主机的端口，当前为 `34053` | 迁移后 Furnace 不读取、不要求配置 |
| `CLICKHOUSE_DATABASE` | scheduler raw sync / contract 工具使用的默认 raw database，当前为 `raw` | Furnace 禁止读取，避免把 calculation SQL 编译到 raw/default database |
| `CLICKHOUSE_DB` | ClickHouse server bootstrap database，当前为 `fleur` | Furnace 可作为 optional client default database，但所有生产 SQL 仍必须使用 fully-qualified table |

迁移后 `.env` 和 `.env.example` 必须包含：

```dotenv
FURNACE_CLICKHOUSE_URL=http://127.0.0.1:34052
FURNACE_CLICKHOUSE_VALIDATE_SCHEMA=true
```

`FURNACE_CLICKHOUSE_URL` 是 Furnace 的权威连接入口；它指向宿主机 HTTP 端口，不指向 native 端口。`FURNACE_CLICKHOUSE_VALIDATE_SCHEMA=true` 保持官方 client 的 `RowBinaryWithNamesAndTypes` 校验默认开启；只有性能基准证明必要时才允许改为 `false`。

### 当前统一切点

现有所有指标最终共享：

- `ClickHouseExecutor::query_bytes()` 读取 `FORMAT RowBinary` 输入。
- `ClickHouseExecutor::query()` 读取 TSV scalar。
- `ClickHouseExecutor::execute_many()` 批量执行 DDL 和 `REPLACE PARTITION`。
- `insert_rowbinary_rows()` 通过 `insert_bytes_stream()` 向同一个 `clickhouse-client` stdin 写 RowBinary。

一刀切迁移应优先改这些公共切点，而不是在 6 个指标里分别实现不同访问方式。

### 涉及指标

| 指标 | Runner | 输出表 |
|---|---|---|
| KDJ | `run_kdj` | `fleur_calculation.calc_stock_kdj_daily` |
| MA | `run_ma` | `fleur_calculation.calc_stock_ma_daily` |
| RSI | `run_rsi` | `fleur_calculation.calc_stock_rsi_daily` |
| BOLL | `run_boll` | `fleur_calculation.calc_stock_boll_daily` |
| MACD | `run_macd` | `fleur_calculation.calc_stock_macd_daily` |
| Price Pattern | `run_price_pattern` | `fleur_calculation.calc_stock_price_pattern_daily` |

## 目标架构

### ClickHouse executor

在 `furnace-io` 中以官方 crate 建立新的默认 executor：

```text
ClickHouseHttpExecutor
  owns clickhouse::Client
  owns or borrows a Tokio runtime boundary
  provides typed query / scalar query / execute / insert rows
```

首选实现策略：

1. 保持 runner 层同步 API，避免把 async 传播到 CPU-bound 指标计算和 Rayon 分组计算。
2. 在 executor 内部封装 Tokio runtime，并在 I/O 边界 `block_on` 官方 client futures。
3. 如实现中发现 runtime 嵌套或测试复杂度过高，再把 `furnace/src/cli.rs` 改为 async entrypoint，但不改变 `furnace-core`。

### Typed rows

把当前手写 RowBinary struct 转成 official client typed rows：

```rust
#[derive(clickhouse::Row, serde::Serialize)]
struct KdjOutputRow { ... }

#[derive(clickhouse::Row, serde::Deserialize)]
struct KdjInputRow { ... }
```

要求：

1. `Date` 字段使用 crate 支持的 `time::Date` 或 `chrono::NaiveDate`，不得继续假设 `String` 能稳定映射 ClickHouse `Date`。
2. 输入 typed rows 在 `furnace-io` 边界转换为 `furnace-core` 当前输入模型。
3. 输出 typed rows 从现有 calculation result row 派生；字段顺序和字段名必须与 ClickHouse DDL 对齐。
4. nullable 指标继续映射为 `Option<f64>` / `Option<i8>` / `Option<u16>` / `Option<time::Date>`。

### Insert path

替换：

```text
insert_rowbinary_rows(...)
  -> executor.insert_bytes_stream(...)
```

为：

```text
insert_typed_rows<T: clickhouse::Row + serde::Serialize>(...)
  -> client.insert::<T>(table).await?
  -> insert.write(row).await? per row
  -> insert.end().await?
```

保留每 `insert_batch_size` 行一个 INSERT 的批量边界。对全市场 replace-cascade，仍按 staging 表写入，再执行年度 `REPLACE PARTITION`。

### Query path

替换所有 SQL 尾部的 `FORMAT RowBinary` / `FORMAT TSV`：

- 输入大表查询：`client.query(sql).fetch::<InputRow>()` 或 `fetch_all::<InputRow>()`。
- scalar 查询：使用专用 typed row，例如 `{ value: u64 }`，禁止继续依赖 TSV 字符串解析。
- DDL / `REPLACE PARTITION`：`client.query(sql).execute().await?`。

### 配置

新增 Furnace HTTP 配置：

| 环境变量 | 用途 | 默认 |
|---|---|---|
| `FURNACE_CLICKHOUSE_URL` | Furnace 专用 HTTP URL | 首选；`.env` / `.env.example` 显式设置为 `http://127.0.0.1:34052` |
| `CLICKHOUSE_HOST` | ClickHouse host | `127.0.0.1` |
| `CLICKHOUSE_PORT` | ClickHouse HTTP client port | 仅在 `FURNACE_CLICKHOUSE_URL` 未设置时参与推导；默认 `8123`，本项目 `.env` 为 `34052` |
| `CLICKHOUSE_USER` | 用户 | optional |
| `CLICKHOUSE_PASSWORD` | 密码 | optional |
| `CLICKHOUSE_DB` | ClickHouse server bootstrap/default database | optional；Furnace SQL 必须继续使用 fully-qualified table |
| `FURNACE_CLICKHOUSE_VALIDATE_SCHEMA` | 是否启用 official client schema validation | 默认 `true`；性能基准证明必要后才允许改为 `false` |
| `CLICKHOUSE_QUERY_TIMEOUT_SECONDS` | query timeout | 映射为 ClickHouse setting 或 request timeout |

`CLICKHOUSE_DATABASE` 不属于 Furnace 配置。该变量当前服务 scheduler raw sync / contract 工具，值可能是 `raw`，不能被 Furnace 用作 calculation 默认 database。

删除或废弃 Furnace 生产路径中的：

- `FURNACE_CLICKHOUSE_CLIENT`
- `FURNACE_CLICKHOUSE_CLIENT_ARGS`
- `CLICKHOUSE_CLIENT`
- `CLICKHOUSE_NATIVE_PORT`

## 实施阶段

### Phase 0：配置契约和 env 模板收敛

1. 在 `.env` 与 `.env.example` 增加 `FURNACE_CLICKHOUSE_URL=http://127.0.0.1:34052`。
2. 在 `.env` 与 `.env.example` 增加 `FURNACE_CLICKHOUSE_VALIDATE_SCHEMA=true`。
3. 保留 `CLICKHOUSE_NATIVE_PORT`，因为 Docker Compose 和当前旧 Furnace CLI 仍可能在迁移完成前使用；但在计划中明确它不是 0068 后 Furnace 连接入口。
4. 不新增 `FURNACE_CLICKHOUSE_CLIENT`、`FURNACE_CLICKHOUSE_CLIENT_ARGS` 或 `CLICKHOUSE_CLIENT` 到 `.env.example`。

完成标准：

- `rg "FURNACE_CLICKHOUSE_URL|FURNACE_CLICKHOUSE_VALIDATE_SCHEMA" .env .env.example` 能看到两份 env 均已登记。
- `.env.example` 不新增 `FURNACE_CLICKHOUSE_CLIENT`、`FURNACE_CLICKHOUSE_CLIENT_ARGS` 或 `CLICKHOUSE_CLIENT`；`CLICKHOUSE_NATIVE_PORT` 只保留给 Docker Compose 和旧路径过渡使用。

### Phase 1：依赖和 executor 框架

1. 在 `engines/crates/furnace-io/Cargo.toml` 增加：

```toml
clickhouse = { version = "0.15.1", features = ["time"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread"] }
time = { version = "0.3", features = ["serde"] }
thiserror = "2"
```

如果实施中选择把 `furnace/src/cli.rs` 改为 async entrypoint，再给 `tokio` 增加 `macros` feature；不要在没有 async main 时提前增加无用 feature。

2. 把 `FurnaceIoError` 扩展为可表达 official client error、date conversion error、configuration error。
3. 新增 `ClickHouseHttpExecutor::from_env()`，只读取 HTTP 配置。
4. 新增 executor 单元测试，覆盖：
   - URL 推导。
   - user/password/database 注入。
   - validation flag。
   - query timeout 配置。

完成标准：

- `cargo check -p furnace-io` 通过。
- 新 executor 不被任何指标使用时，现有测试仍通过。

### Phase 2：typed row 和通用 I/O trait 重塑

1. 替换当前以字节为中心的 trait 方法：
   - 删除或停止使用 `query_bytes()`。
   - 删除或停止使用 `insert_bytes_stream()`。
   - 新增 typed query、typed scalar 和 typed insert 公共 helper。
2. 为 6 个指标建立输入 row、previous-state row、输出 row。
3. 建立 date conversion helper：
   - `time::Date -> YYYY-MM-DD String`
   - `YYYY-MM-DD String -> time::Date`
4. 保留现有 runner tests 的行为断言，但 fixture 从 RowBinary bytes 改为 typed query result。

完成标准：

- 所有 `engines/crates/furnace-io/src/runners/tests/*` 编译并通过。
- `rg "FORMAT RowBinary|FORMAT TSV|write_row_binary|push_rowbinary" engines/crates/furnace-io/src` 不再命中生产路径。

### Phase 3：一次性迁移 6 个指标 runner

按统一公共 helper 改造所有指标，不允许完成后存在部分指标仍走 CLI executor。

| 指标 | 输入读取 | previous state | 输出写入 | replace-cascade |
|---|---|---|---|---|
| KDJ | typed input rows | 无或现有状态查询 | typed rows | 必须通过 |
| MA | typed price/volume rows | MA previous state | typed rows | 必须通过 |
| RSI | typed close rows | RSI previous state | typed rows | 必须通过 |
| BOLL | typed close rows | 无 | typed rows | 必须通过 |
| MACD | typed close rows | MACD previous state | typed rows | 必须通过 |
| Price Pattern | typed structure/streak rows | 无 | typed rows | 必须通过 |

完成标准：

- 所有 runner 单元测试继续覆盖 dry-run、append-latest 和 replace-cascade。
- `insert_batch_size` 仍控制 INSERT chunk。
- staging validation、duplicate-key validation、retain-existing-rows 和 partition replace 保持原语义。

### Phase 4：CLI 和 Dagster resource 切换

1. `engines/crates/furnace/src/cli.rs` 默认构造 `ClickHouseHttpExecutor::from_env()`。
2. 删除 `ClickHouseCliExecutor` 生产构造和相关 `clickhouse-client` 环境变量说明。
3. 清理 `pipeline/scheduler/src/scheduler/defs/resources/furnace.py` 中只服务 Docker exec 的默认注入。
4. 更新 `pipeline/scheduler/tests/unit/resources/test_furnace.py`，断言 Furnace resource 不再注入 `FURNACE_CLICKHOUSE_CLIENT`。
5. 更新 `docs/architecture/furnace.md` 和 `engines/README.md` 的运行说明。

完成标准：

- Dagster Furnace asset 运行环境只需要 HTTP ClickHouse 配置。
- `rg "FURNACE_CLICKHOUSE_CLIENT|CLICKHOUSE_NATIVE_PORT|clickhouse-client" pipeline/scheduler/src engines/crates docs/architecture engines/README.md` 不再命中当前生产说明；历史 job report 可保留。

### Phase 5：端到端验证和性能基准

1. 先用迁移前当前 main 或备份 binary 生成 baseline JSON summary。
2. 用新 binary 对 6 个指标执行同一范围 dry-run。
3. 在 dev ClickHouse 上对每个指标执行最小 append-latest 或 replace-cascade smoke。
4. 对至少一个全市场大指标执行性能对比，优先 BOLL 或 MA，因为历史报告显示写入阶段受 `docker exec clickhouse-client` 成本影响明显。
5. 生成 `docs/jobs/reports/YYYY-MM-DD-furnace-clickhouse-rust-client-migration.md`。

完成标准：

- 6 个指标 dry-run output summary 与迁移前一致，允许 elapsed 时间不同。
- 写入 smoke 后，目标表 row count、date range、duplicate validation 通过。
- 全市场或代表性窗口没有出现无法解释的明显性能回退；如回退超过 15%，必须记录原因并决定是否启用 `Client::with_validation(false)`。

## 禁止模式

1. 禁止保留生产双路径开关，例如 `FURNACE_USE_HTTP_CLIENT=false`。
2. 禁止指标级分叉，例如 KDJ 用 official client、MA 用 CLI。
3. 禁止继续要求 Dagster 注入 Docker exec 参数。
4. 禁止将 ClickHouse client 依赖放入 `furnace-core`。
5. 禁止用 `a || b`、多字段 fallback 或候选字段名兼容来绕过 typed row 映射不确定性。
6. 禁止用字符串解析 TSV 作为 scalar query 的长期实现。
7. 禁止通过 `ALTER UPDATE` / `ALTER DELETE` 完成历史修正。
8. 禁止跳过 all-indicator smoke 后声明完成。

## 验证命令

Rust：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Dagster resource：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/unit/resources/test_furnace.py
cd scheduler
uv run dg check defs
```

Furnace CLI smoke 示例：

```bash
cd engines
FURNACE_CLICKHOUSE_URL=http://127.0.0.1:34052 \
CLICKHOUSE_USER="${CLICKHOUSE_USER}" \
CLICKHOUSE_PASSWORD="${CLICKHOUSE_PASSWORD}" \
cargo run -p furnace -- boll \
  --from 2026-06-01 \
  --to 2026-06-10 \
  --mode dry-run \
  --output-format json
```

每个指标都要执行同类 smoke：

```text
kdj
ma
rsi
boll
macd
price-pattern
```

ClickHouse 结果核验：

```sql
SELECT
    table,
    count() AS active_parts,
    sum(rows) AS rows
FROM system.parts
WHERE active
  AND database = 'fleur_calculation'
  AND table IN (
      'calc_stock_kdj_daily',
      'calc_stock_ma_daily',
      'calc_stock_rsi_daily',
      'calc_stock_boll_daily',
      'calc_stock_macd_daily',
      'calc_stock_price_pattern_daily'
  )
GROUP BY table
ORDER BY table;
```

## 完成标准

1. `engines` Rust 门禁全部通过。
2. scheduler Furnace resource 和 definitions 门禁通过。
3. 6 个 Furnace 指标全部使用 official `clickhouse` crate HTTP executor。
4. 生产路径没有外部 `clickhouse-client` 依赖。
5. 6 个指标 dry-run、append-latest、replace-cascade 的行为测试通过。
6. 至少一份 dev ClickHouse 端到端 smoke report 归档到 `docs/jobs/reports/`。
7. 当前文档入口更新完成：`docs/plans/README.md`、`docs/architecture/furnace.md`、`engines/README.md`。

## 后续维护动作

完成后将本计划移动到 `docs/plans/archive/`，在 `docs/plans/README.md` 的 Recently Completed 中记录验收报告。若官方 crate 后续提供 Native TCP 支持，应另开 RFC 或计划评估，不在本计划中预留隐式切换。
