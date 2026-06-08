# Plan 0028: Furnace KDJ 全市场日频并行计算性能优化实施方案

日期：2026-06-07

状态：Implemented through full-range benchmark; best measured local full backfill is RowBinary input/output, 8 Rayon threads, single full-range historical batch, single-query staging validation, and multi-query partition replacement. Safer historical batch setting remains 4,000,000 rows with part and memory checks.

关联文档：

- `docs/plans/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- `docs/RFC/0016-rust-furnace-compute-engine.md`
- `docs/ADR/0009-clickhouse-layered-database-migration.md`
- `docs/jobs/reports/2026-06-07-furnace-kdj-smoke-run.md`
- `docs/jobs/reports/2026-06-07-furnace-kdj-performance-baseline.md`
- `docs/jobs/reports/2026-06-07-furnace-kdj-parallel-optimization.md`
- `engines/crates/furnace-core/src/indicators/kdj.rs`
- `engines/crates/furnace-io/src/lib.rs`
- `engines/crates/furnace/src/main.rs`
- `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
- `pipeline/scheduler/src/scheduler/defs/resources/furnace.py`

相关 skills：

- `fleur-harness`：计划、报告、验收和长期维护边界。
- `rust-best-practices`：Rust 性能优化、release 基准、profile、避免不必要 clone 和并行边界。
- `clickhouse-best-practices`：ClickHouse 输入查询、批量写入、分区和 part 健康检查。
- `clickhouse-architecture-advisor`：market data / financial services 场景下的时间序列计算架构取舍。

## 1. 目标

优化 Furnace 对全市场日频 KDJ 的计算性能，使日常全市场增量、月度回填和历史级联重算具备可观测、可验证、可扩展的执行路径。

完成后应满足：

1. `furnace kdj` 能在 JSON summary 中输出分阶段性能指标。
2. Dagster materialization metadata 能展示 Furnace 性能摘要，不需要查自由文本日志。
3. KDJ 计算按证券维度并行；单证券内部仍按交易日串行递推。
4. 并行版与串行版在相同输入下结果一致且输出顺序确定。
5. ClickHouse 读取、计算、写入、staging 和 `REPLACE PARTITION` 的耗时能拆开观察。
6. INSERT 仍按合理批次聚合写入，不因并行计算产生大量小 part。
7. 全市场性能瓶颈可被定位到输入扫描、分组、计算、写入或分区替换中的某一段。

## 2. 非目标

本计划不做以下事情：

1. 不改变 KDJ 公式、默认参数、分母为 0 时 RSV=50、历史 K/D 状态读取和级联重算语义。
2. 不把同一证券的时间序列按日期拆并行；K/D 递推状态不允许这样做。
3. 不把 `furnace-core` 绑定到 Rayon、ClickHouse、Dagster 或 CLI。
4. 不修改 `fleur_calculation.calc_stock_kdj_daily` 的 grain、字段或分区替换协议。
5. 不通过 Dagster daily partitions 拆分 KDJ 计算；历史级联写入仍突破单日切片语义。
6. 不在本阶段重建上游 `int_stock_quotes_daily_adj` 表的 `ORDER BY`。如需要新读取路径，应另立 dbt/ClickHouse 建模计划。
7. 不实现多进程 shard 并发替换同一个 ClickHouse 年分区。

## 3. 当前事实基线

当前实现事实：

1. `furnace-core` 已提供单证券 KDJ 纯计算 API，适合继续作为串行、确定性的指标核心。
2. `furnace-io::calculate_outputs` 当前先把输入行按 `security_code` 分组到 `BTreeMap<&str, Vec<KdjInput>>`，再逐证券串行调用 `calculate_kdj_series`。
3. `read_input_rows` 当前从 ClickHouse 一次性读取 TSV stdout，再解析为 `Vec<PriceInputRow>`。
4. 输入 SQL 已按 `ORDER BY security_code, trade_date` 输出，符合单证券递推计算顺序。
5. `KdjRunSummary` 当前包含请求区间、输入区间、row counts、受影响年份、staging 和 partition replace 信息，但没有分阶段耗时。
6. Dagster `_metadata_from_summary` 当前透传业务运行摘要，但没有性能 metadata。
7. CLI 已提供 `--insert-batch-size`，默认 10,000。Per `insert-batch-size`，生产写入应保持 10,000-100,000 行目标批次，避免小 part。
8. 上游 `fleur_intermediate.int_stock_quotes_daily_adj` 当前 `ORDER BY (security_code, trade_date)`，更利于单证券长序列读取；全市场只按 `trade_date` 过滤时，需要用 `EXPLAIN indexes = 1` 验证主键索引利用情况。Per `schema-pk-filter-on-orderby`，跳过 ORDER BY 前缀会影响索引效果。

## 4. 架构决策

### 4.1 并行粒度

采用证券维度并行：

```text
ClickHouse input rows
  -> grouped or streamed by security_code
  -> per-security KDJ worker
  -> deterministic merge and sort
  -> batched ClickHouse write
```

依据：

- K/D 是递推状态，同一 `security_code` 的 `trade_date` 序列必须串行。
- 不同 `security_code` 之间没有状态依赖，天然适合数据并行。
- `furnace-core` 继续只负责单证券纯计算；并行调度放在 `furnace-io`。

### 4.2 观测优先

Per `rust-best-practices` Chapter 3，性能优化先测量再修改。第一阶段先增加计时和基准报告，再引入并行计算。

`performance_metrics` 建议结构：

```json
{
  "total_ms": 0,
  "read_input_ms": 0,
  "read_state_ms": 0,
  "group_ms": 0,
  "compute_ms": 0,
  "write_ms": 0,
  "staging_ms": 0,
  "partition_replace_ms": 0,
  "input_rows_per_sec": 0.0,
  "output_rows_per_sec": 0.0,
  "symbols_count": 0,
  "parallelism": "serial|rayon",
  "worker_threads": 0
}
```

### 4.3 ClickHouse 读取策略

输入读取分两步推进：

1. 短期保持当前一次性读取 `Vec<PriceInputRow>`，先完成计时和证券维度并行。
2. 中期再评估流式按证券切段读取，减少全市场长区间回填的内存峰值。

Per `schema-pk-prioritize-filters` 和 `schema-pk-filter-on-orderby`，全市场日期过滤可能无法充分利用上游 `(security_code, trade_date)` 排序键。优化时必须记录：

- 输入 SQL。
- `EXPLAIN indexes = 1` 输出摘要。
- 读取行数、扫描耗时和 ClickHouse 返回耗时。
- 是否需要先解析证券全集，再按证券 chunk 查询。

### 4.4 写入和分区替换

计算可以并行，写入和分区替换保持单 coordinator：

- 并行 worker 不直接 INSERT。
- worker 输出合并成统一结果集后，再按 `insert_batch_size` 批量写入。
- staging table、staging validation 和 `REPLACE PARTITION` 仍由一个 Furnace run 顺序协调。
- 不按证券并发替换同一年分区，避免互相覆盖。

## 5. 实施阶段

### 阶段 1：性能指标和基线报告

目标：

1. 新增 `PerformanceMetrics` 或等价结构，记录 Furnace 内部分阶段耗时。
2. 将 `performance_metrics` 加入 `KdjRunSummary::to_json()`。
3. Dagster `_metadata_from_summary` 透传 `performance_metrics`。
4. 在 dry-run、append-latest、replace-cascade 三种路径中都能输出 metrics。
5. 建立串行基线报告。

实现要点：

- 使用 `std::time::Instant` 做粗粒度阶段计时。
- 先不引入新的 metrics crate，保持 CLI summary 稳定简单。
- `total_ms` 应覆盖从 `run_kdj` 开始到 summary 生成前的总耗时。
- 写入关闭的 `dry-run` 中，`write_ms`、`staging_ms`、`partition_replace_ms` 可以为 0。
- Dagster metadata 不记录 ClickHouse 连接串、密码或 token。

基线场景：

| 场景 | 模式 | 目的 |
|------|------|------|
| 单证券一个月 | dry-run | 验证小样本开销 |
| 全市场单日 | dry-run | 验证横截面读取和分组 |
| 全市场一个月 | dry-run | 验证主要计算耗时 |
| 全市场一年 | dry-run | 验证内存和长区间耗时 |
| 小证券历史区间 | replace-cascade | 验证 staging/replace 耗时 |

产出：

- `docs/jobs/reports/<date>-furnace-kdj-performance-baseline.md`

完成标准：

- CLI JSON summary 包含 `performance_metrics`。
- Dagster asset metadata 包含 `performance_metrics`。
- 基线报告能说明当前瓶颈是在读取、分组、计算、写入还是分区替换。

### 阶段 2：证券维度并行计算

目标：

1. 在 `furnace-io` 引入 Rayon 或等价数据并行库。
2. 将当前逐证券串行循环改为 per-security parallel iterator。
3. 保留串行实现作为测试对照或小输入 fast path。
4. 并行结果合并后排序，保证输出确定性。

实现要点：

- 并行边界在 `furnace-io::calculate_outputs` 或其拆分后的 helper 内。
- 每个 worker 接收完整证券输入序列、该证券 previous K/D state、KDJ 参数和输出日期范围。
- worker 不共享可变状态。
- `KdjParams`、日期范围和配置应通过引用或小型值传递，避免热点循环 clone 大对象。
- 小输入 fast path 可按 `symbols_count` 或 `input_rows` 设置阈值，例如证券数低于 CPU 线程数的 2 倍时保持串行；具体阈值由基准调整。
- 输出合并后按 `(security_code, trade_date)` 排序；写入目标表是否按 `(trade_date, security_code)` 排序不影响业务正确性，但排序规则必须稳定。

测试要求：

- 固定 fixture：并行输出与串行输出完全一致。
- 多证券乱序输入：I/O 层排序或错误处理行为不因并行改变。
- 缺价、`high < low`、窗口不足、分母为 0 和历史 state 读取语义不变。
- 同一输入重复运行，输出 row count、唯一键集合和值一致。

完成标准：

- `cargo test --workspace` 通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。
- 全市场一个月 dry-run 的 `compute_ms` 相比串行基线有可解释的变化；若没有提升，报告必须说明瓶颈不在 CPU 计算。

### 阶段 3：输入查询与分组内存优化

目标：

1. 验证全市场读取 SQL 对上游 `ORDER BY (security_code, trade_date)` 的索引利用情况。
2. 降低全市场长区间回填时的分组内存峰值。
3. 评估证券 chunk 查询是否优于单次全市场日期区间查询。

实施路径：

1. 对当前输入 SQL 执行 `EXPLAIN indexes = 1`，记录主键和分区裁剪情况。
2. 如果 `WHERE trade_date BETWEEN ...` 扫描过大，新增证券全集解析步骤，并评估 chunk 查询：

```sql
WHERE security_code IN (...)
  AND trade_date BETWEEN <from> AND <to>
```

3. 初始评估 chunk 大小：200、500、1000 只证券。
4. 如果当前 ClickHouse executor 只能一次性读取 stdout，先保留 Vec 方案，仅优化 chunk 粒度。
5. 如果长区间内存压力明显，再新增流式读取接口：按 `security_code, trade_date` 顺序读取，证券切换时提交上一证券序列给计算队列。

完成标准：

- 性能报告记录 `EXPLAIN indexes = 1` 摘要。
- 能说明当前采用单次查询还是证券 chunk 查询，以及依据。
- 如果实现 chunk 查询，结果必须与单次查询路径一致。
- 如果实现流式读取，必须有测试覆盖证券边界切换、最后一个证券 flush 和空输入。

### 阶段 4：写入路径和 ClickHouse 健康验证

目标：

1. 确认并行计算没有改变批量写入策略。
2. 确认 staging 和 `REPLACE PARTITION` 仍按单 Furnace run 协调。
3. 记录 ClickHouse part 健康。

实施要点：

- `insert_batch_size` 继续默认 10,000。
- 小样本低于 10,000 行可以单批写入，但运行报告要说明原因。
- 全市场生产写入不得按证券单独 INSERT。
- Per `insert-batch-size`，理想批次保持 10,000-100,000 行。
- Per `schema-partition-low-cardinality`，继续使用年分区，避免为了并行写入引入高基数分区。

验收查询：

```sql
SELECT
    partition,
    count() AS parts,
    sum(rows) AS rows
FROM system.parts
WHERE database = 'fleur_calculation'
  AND table = 'calc_stock_kdj_daily'
  AND active
GROUP BY partition
ORDER BY partition;
```

完成标准：

- 并行计算前后，目标表唯一键校验无重复。
- `system.parts` 没有因为并行优化出现大量小 part。
- `replace-cascade` 的 affected years、retained rows 和 staging validation 语义不变。

### 阶段 5：滚动窗口微优化评估

目标：

1. 判断 RSV rolling min/max 是否真是热点。
2. 只有 profile 证明必要时，才把简单窗口扫描替换为单调队列。

实施要点：

- 默认 `rsv_window=9`，简单扫描可能已经足够。
- 如果 `compute_ms` 占比高但 Rayon 后仍不理想，再用 flamegraph 或等价 profiler 看 `calculate_kdj_series` 热点。
- 单调队列实现必须保持 `None`、`high < low`、窗口不足、分母为 0 时 RSV=50 和状态不推进等边界语义。

完成标准：

- 若不实施单调队列，报告说明 profile 结论。
- 若实施单调队列，串行 fixture、并行 fixture、doctest 和回归样本全部通过。

### 阶段 6：性能报告和运行手册更新

目标：

1. 记录优化前后基准对比。
2. 固化全市场日频 KDJ 的推荐运行命令。
3. 明确后续是否需要上游 helper/projection 表。

产出：

- `docs/jobs/reports/<date>-furnace-kdj-performance-baseline.md`
- `docs/jobs/reports/<date>-furnace-kdj-parallel-optimization.md`
- 如新增 benchmark harness，更新 `engines/README.md` 或 rustdoc。
- 如发现上游 ORDER BY 不适合全市场读取，新增后续 ClickHouse/dbt 建模计划，不在本计划内直接改表。

报告至少包含：

- Furnace binary 构建模式和 commit。
- 输入日期范围、输出日期范围、证券数量、输入行数、输出行数。
- `performance_metrics` 原始 JSON。
- `EXPLAIN indexes = 1` 摘要。
- `system.parts` 摘要。
- 串行与并行耗时对比。
- 当前瓶颈和下一步建议。

## 6. 验证命令

Rust：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D clippy::perf
cargo test --workspace
cargo test --doc --workspace
cargo run --release -p furnace -- kdj --from <date> --to <date> --mode dry-run --output-format json
```

如新增 benchmark：

```bash
cd engines
cargo bench -p furnace-core
cargo bench -p furnace-io
```

Dagster：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/furnace scheduler/tests/unit/resources/test_furnace.py
uv run dg list defs --json
cd scheduler
uv run dg check defs
```

ClickHouse：

```sql
EXPLAIN indexes = 1
SELECT
    security_code,
    trade_date,
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj
FROM fleur_intermediate.int_stock_quotes_daily_adj
WHERE trade_date BETWEEN <from> AND <to>
ORDER BY
    security_code,
    trade_date;
```

```sql
SELECT
    security_code,
    trade_date,
    count() AS rows
FROM fleur_calculation.calc_stock_kdj_daily
GROUP BY
    security_code,
    trade_date
HAVING rows > 1
LIMIT 10;
```

```sql
SELECT
    partition,
    count() AS parts,
    sum(rows) AS rows
FROM system.parts
WHERE database = 'fleur_calculation'
  AND table = 'calc_stock_kdj_daily'
  AND active
GROUP BY partition
ORDER BY partition;
```

文档-only 变更：

```bash
git diff --check
```

## 7. 禁止模式

1. 不按日期并行单证券 KDJ。
2. 不让并行 worker 直接写 ClickHouse。
3. 不按证券并发执行 `REPLACE PARTITION`。
4. 不为了全市场计算直接修改上游 dbt 表 `ORDER BY`。
5. 不用 debug 构建结果评估性能。
6. 不只报告总耗时；必须拆分读取、分组、计算、写入和替换耗时。
7. 不把性能日志只写 stderr；结构化指标必须进入 CLI JSON summary。
8. 不牺牲确定性输出顺序换取并行吞吐。
9. 不把 Rayon 或并行调度暴露到 `furnace-core` 公共 API。

## 8. 已决策项

1. 并行粒度固定为证券维度；单证券内部保持串行递推。
2. 第一阶段先做 metrics 和基线，再做 Rayon 并行。
3. ClickHouse 写入保持统一批量写入和单 coordinator 分区替换。
4. 上游读取路径优化先通过 `EXPLAIN indexes = 1` 和基准验证；是否新增 helper/projection 表另立计划决策。
5. Dagster 继续触发一个 Furnace run，由 Furnace 内部并行；不把 KDJ 全市场计算拆成 Dagster daily partitions。
6. 2026-06-07 第一轮实施选择“全市场请求不拼接大 `IN (...)` + Rayon 证券维度并行 + 现有年分区/批写存储协议”。实测详见 `docs/jobs/reports/2026-06-07-furnace-kdj-parallel-optimization.md`。
7. 2026-06-07 样本中，8 个 Rayon 线程在全市场一个月 dry-run 中表现最好；Dagster `FurnaceCliResource` 默认注入 `RAYON_NUM_THREADS=8`，但如果外部环境已设置该变量则尊重外部设置。该运行配置不进入 `furnace-core` 或指标 API。
8. 当前证据不足以证明需要新增 ClickHouse helper/projection 表；后续只有在 full-year 或 multi-year benchmark 显示输入读取持续成为瓶颈时再另立建模计划。
