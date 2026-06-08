# Plan 0030: Furnace RSI 日线技术指标实施方案

日期：2026-06-08

状态：Draft

关联文档：

- `docs/RFC/0016-rust-furnace-compute-engine.md`
- `docs/plans/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- `docs/plans/0028-furnace-kdj-parallel-performance-implementation-plan.md`
- `docs/plans/0029-furnace-moving-average-technical-indicators-implementation-plan.md`
- `engines/README.md`
- `engines/crates/furnace-core/src/indicators/kdj.rs`
- `engines/crates/furnace-core/src/indicators/moving_average.rs`
- `engines/crates/furnace-core/src/operators/ema.rs`
- `engines/crates/furnace-core/src/operators/sma.rs`
- `engines/crates/furnace-io/src/lib.rs`
- `engines/crates/furnace/src/main.rs`
- `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
- `pipeline/scheduler/src/scheduler/defs/resources/furnace.py`
- `pipeline/elt/models/sources_fleur_calculation.yml`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.yml`
- `pipeline/elt/models/intermediate/int_stock_kdj_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_ma_daily.sql`

相关 skills：

- `rust-best-practices` / `rust-patterns` / `rust-testing`：Rust crate 边界、RSI 递推状态、错误模型、测试和性能。
- `running-dbt-commands` / `using-dbt-for-analytics-engineering`：dbt source、thin wrapper、字段文档和定向 build。
- `dagster-expert`：Dagster asset、job、schedule、资源和 materialization metadata。
- `clickhouse-best-practices` / `clickhouse-architecture-advisor`：ClickHouse 宽表、RowBinary 批量写入、年度分区替换和全量回填验收。

## 1. 目标

在现有 Furnace Rust workspace 中新增日频 RSI 指标计算能力，从 `fleur_intermediate.int_stock_quotes_daily_adj` 读取前复权收盘价，计算以下 canonical RSI 窗口：

```text
RSI(6), RSI(12), RSI(14), RSI(24), RSI(25), RSI(50)
```

Furnace 直接写入：

```text
fleur_calculation.calc_stock_rsi_daily
```

再由 dbt thin wrapper 暴露为：

```text
fleur_intermediate.int_stock_rsi_daily
```

完成后应满足：

1. `furnace rsi` 支持按日期区间、证券集合和运行模式计算 RSI 指标。
2. 所有 RSI 使用 `close_price_forward_adj` 作为唯一 canonical close 输入，不混用未复权、后复权或其他价格口径。
3. 第一版输出业务字段固定为 `rsi_6`、`rsi_12`、`rsi_14`、`rsi_24`、`rsi_25`、`rsi_50`。
4. RSI 公式只放在 `furnace-core`；ClickHouse I/O、RowBinary、Rayon 并行和 staging/partition replace 放在 `furnace-io`；CLI 放在 `furnace`；调度与 metadata 放在 Dagster；消费契约放在 dbt。
5. 生产写入使用 staging + 年度 `REPLACE PARTITION` 协议，保持幂等并避免高频 mutation。
6. 计算按证券维度并行；单证券内部按 `trade_date` 严格串行递推。
7. 支持全量计算和日常增量优化。增量结果必须与同一证券全历史一次性计算一致，不能因为 warm-up 截断产生偏差。
8. 验收必须包含全市场、全历史数据量的并行 dry-run 和写入验证，并记录性能和结果质量报告。

## 2. 非目标

本计划不做以下事情：

1. 不实现 MACD、布林线、WR、CCI 或其他未列入本计划的指标。
2. 不改变 `int_stock_quotes_daily_adj` 的复权逻辑、字段语义或物化策略。
3. 不在 dbt SQL、Dagster Python asset 或 ClickHouse SQL 中重写 RSI 公式。
4. 不让 Furnace 直接写入 `fleur_intermediate.int_stock_rsi_daily`。
5. 不把同一证券时间序列按日期并行；RSI Wilder 平滑状态不允许这样做。
6. 不在第一版支持多价格口径、动态参数集合或长表 `indicator_name/value` 结构。
7. 不强制同步重构 KDJ 或 MA 已有模块；如需抽取通用 Wilder smoothing 算子，只服务 RSI 第一版稳定边界。
8. 不把第一版 `calc_stock_rsi_daily` 建成 Dagster daily partition asset；历史级联写入会突破单日 partition 语义。

## 3. 当前事实基线

### 3.1 Rust workspace

当前 Rust workspace 已有三层 crate：

```text
engines/
├── crates/furnace/       # CLI binary，当前已有 kdj/ma 子命令
├── crates/furnace-core/  # 纯指标计算，当前已有 KDJ 和 MA
└── crates/furnace-io/    # ClickHouse I/O、RowBinary、Rayon、staging/replace
```

可复用事实：

1. `furnace-core` 已有 `indicators::kdj` 和 `indicators::moving_average` 的单证券纯计算模式。
2. `furnace-core::operators` 已有 SMA/EMA 公共算子，可作为 RSI Wilder smoothing 的实现参考。
3. `furnace-io` 已有 ClickHouse executor 抽象、RowBinary 读取/写入、证券维度 Rayon 并行、staging 表、年度分区替换和 `PerformanceMetrics`。
4. `furnace` CLI 当前已按 KDJ/MA 模式输出 JSON summary，可作为 `rsi` 子命令模板。
5. MA 全市场验收已证明全市场、全历史 close-only 指标可以用 release binary + RowBinary + Rayon 在当前数据量上完成并行计算。

### 3.2 输入模型

RSI 输入默认来自：

```text
fleur_intermediate.int_stock_quotes_daily_adj
```

第一版只读取：

| 字段 | 用途 |
|------|------|
| `security_code` | 证券代码 |
| `trade_date` | 交易日 |
| `close_price_forward_adj` | RSI 的 canonical close 输入 |

输入必须按以下顺序提供给单证券核心计算：

```text
security_code ASC, trade_date ASC
```

输入和输出行口径：

1. 输出 grain 与输入行情 grain 对齐，为每证券、交易日一行。
2. 请求输出区间内即使 `close_price_forward_adj IS NULL` 也要输出一行，所有业务 RSI 字段为 `NULL`。
3. `input_rows` 统计实际读取的行情行数，不只统计有效 close 行。
4. `output_rows` 统计实际落入 `effective_output_from..effective_output_to` 的行情行数。
5. `valid_close_rows` 统计 `close_price_forward_adj IS NOT NULL` 的行数，作为运行报告辅助指标。
6. 同一证券内 `trade_date` 必须严格递增；如果输入中出现重复日期或乱序，核心计算返回错误，不在 Rust 中静默去重。
7. `null_indicator_rows` 统计所有业务 RSI 字段均为 `NULL` 的输出行；只要任一 RSI 字段非空，该行不计入 `null_indicator_rows`。

### 3.3 dbt 和 Dagster 现状

可复用的 dbt 模式：

1. `pipeline/elt/models/sources_fleur_calculation.yml` 已声明 KDJ 和 MA calculation source。
2. `int_stock_kdj_daily.sql` 和 `int_stock_ma_daily.sql` 都是 thin wrapper，不重写公式。
3. 对应 YAML 提供 wrapper 文档、唯一性测试和字段说明。

可复用的 Dagster 模式：

1. `pipeline/scheduler/src/scheduler/defs/furnace/assets.py` 已实现 Furnace calculation asset 模式。
2. `pipeline/scheduler/src/scheduler/defs/resources/furnace.py` 已实现 `FurnaceCliResource`、请求 dataclass、命令构造、stdout JSON 解析和 `RAYON_NUM_THREADS` 注入。
3. `pipeline/scheduler/src/scheduler/defs/furnace/definitions.py` 已定义 Furnace jobs 和 schedule 模式。

## 4. 指标定义

### 4.1 默认参数

第一版生产参数固定为：

```text
RSI_WINDOWS = [6, 12, 14, 24, 25, 50]
```

这些窗口直接体现在宽表字段中：

| 参数 | 输出字段 | 说明 |
|------|----------|------|
| `6` | `rsi_6` | 6 个有效涨跌幅的 Wilder RSI |
| `12` | `rsi_12` | 12 个有效涨跌幅的 Wilder RSI |
| `14` | `rsi_14` | 14 个有效涨跌幅的 Wilder RSI |
| `24` | `rsi_24` | 24 个有效涨跌幅的 Wilder RSI |
| `25` | `rsi_25` | 25 个有效涨跌幅的 Wilder RSI |
| `50` | `rsi_50` | 50 个有效涨跌幅的 Wilder RSI |

第一版生产写入只允许上述 canonical 窗口集合。若后续需要新增参数，必须重新评估唯一键、字段契约、dbt tests 和下游消费口径。

### 4.2 公式

对每个 `security_code` 按 `trade_date` 升序计算。仅使用有效 close 行推进状态。

```text
change_t = close_t - previous_valid_close
gain_t = max(change_t, 0)
loss_t = max(-change_t, 0)
```

对窗口 `n`：

启动阶段：

```text
avg_gain_n = SMA(gain, n)
avg_loss_n = SMA(loss, n)
```

递推阶段使用 Wilder smoothing：

```text
avg_gain_n = (previous_avg_gain_n * (n - 1) + gain_t) / n
avg_loss_n = (previous_avg_loss_n * (n - 1) + loss_t) / n
```

RSI：

```text
if avg_gain_n = 0 and avg_loss_n = 0:
    rsi_n = 50
else if avg_loss_n = 0:
    rsi_n = 100
else if avg_gain_n = 0:
    rsi_n = 0
else:
    rs_n = avg_gain_n / avg_loss_n
    rsi_n = 100 - 100 / (1 + rs_n)
```

空值和状态规则：

1. 每个窗口需要 `n` 个有效涨跌幅才能启动，因此 `RSI(n)` 最早出现在同一证券第 `n + 1` 个有效 close 对应的交易日。
2. 当前行 `close_price_forward_adj IS NULL` 时，所有 RSI 输出 `NULL`，状态不推进，上一有效 close 保持不变。
3. 新证券首个有效 close 没有上一有效 close，无法形成涨跌幅；所有 RSI 输出 `NULL`，但记录上一有效 close 供下一条有效 close 使用。
4. 缺价后的下一条有效 close 使用缺价前最近一条有效 close 计算 `change_t`，缺价行不形成涨跌幅。
5. 输入值必须为有限 `f64`；非有限值作为错误处理，不悄悄进入计算。
6. 核心计算不做四舍五入；结果以 `Float64` 递推和输出，测试断言使用 `1e-9` 级别容差。

### 4.3 RSI 状态方案

第一版采用“同表内部状态列 + 输入表 previous state 日期 close 校验”的方案。

业务字段：

```text
rsi_6
rsi_12
rsi_14
rsi_24
rsi_25
rsi_50
```

内部状态字段：

```text
avg_gain_6_state
avg_loss_6_state
avg_gain_12_state
avg_loss_12_state
avg_gain_14_state
avg_loss_14_state
avg_gain_24_state
avg_loss_24_state
avg_gain_25_state
avg_loss_25_state
avg_gain_50_state
avg_loss_50_state
```

说明：

1. `avg_gain_*_state` 和 `avg_loss_*_state` 用于后续增量继续 Wilder 递推。
2. dbt `int_stock_rsi_daily` 默认只暴露业务 RSI 字段，不暴露内部状态。
3. 日常增量还需要 previous state 日期对应的有效 close，用于初始化 `previous_close` 并校验上游输入未被历史修正；该 close 从输入表 `int_stock_quotes_daily_adj` 查询，不在 RSI calculation 表重复存储价格状态。
4. 读取 previous RSI state 时，只能使用目标区间前最近一条所有 canonical 窗口状态均非空的结果行，作为完整状态；状态值允许为 `0.0`，不能用 truthy/falsy 判断状态是否存在。
5. 如果某证券在目标区间前不存在完整状态，或只存在部分窗口状态，则该证券必须回读足够历史输入，从首个有效 close 或可证明等价的起点重新推导。
6. 历史回填和复权修正使用 `replace-cascade`，从请求起点级联到受影响证券最新输入交易日。
7. 当前行 close 为空时，业务 RSI 字段和内部状态列均输出 `NULL`，内存中的上一条完整状态不推进。
8. 当某个窗口已启动后，后续有效 close 行的对应 `avg_gain_*_state` / `avg_loss_*_state` 输出当前行最新状态。
9. previous state 对应的 close 只用于初始化 `previous_close`，不作为一条新的涨跌幅输入；实现不得把 previous state 日期对应的 close 再次传入 Wilder 递推，否则会产生一条错误的 `change = 0`。
10. 对同一证券，如果批量读取的 `input_from` 早于该证券 previous state 日期，则 previous state 日期及其之前的输入行必须跳过；只从 previous state 之后的第一条输入行开始计算。
11. 如果 previous state 日期之后、`request_from` 之前存在有效 close 输入行但目标表没有对应完整 RSI 状态，说明结果表存在缺口。第一版生产 `append-latest` 必须拒绝并提示从缺口起点补算；不得直接从 `request_from` 跳算。

状态来源枚举：

| 值 | 含义 |
|----|------|
| `previous-state` | 所有受影响证券都从目标区间前完整 RSI 状态和该状态日期对应 close 延续 |
| `full-history` | 所有受影响证券都从首个有效 close 或全量历史重新推导 |
| `mixed` | 一部分证券用 previous state，一部分证券回读历史推导 |

summary 可以使用扩展字符串记录混合明细，例如：

```text
mixed:previous-state:5520,full-history:12
```

## 5. 全量计算和计算优化方案

### 5.1 全量计算口径

全量计算用于首次建表、全历史验收和无法安全延续状态的修复场景。

运行区间：

```text
request_from = 输入表实际最早 trade_date
request_to = 输入表实际最新 trade_date
effective_output_from = request_from
effective_output_to = request_to
input_from = request_from
input_to = request_to
```

全量计算要求：

1. 省略 `--symbols` 表示全市场，不能用少量证券替代全市场验收。
2. 每个证券从首个有效 close 开始构造涨跌幅和 RSI 状态。
3. 输出行数应与请求日期范围内输入行情行数一致。
4. 对任一证券，前 `n` 个有效 close 对应的 `rsi_n` 必须为 `NULL`，第 `n + 1` 个有效 close 才可能出现首个非空 RSI。
5. 全量 dry-run 不建表、不写入，只验证读取、分组、并行计算、summary 和性能。
6. 全量 replace-cascade 可以写生产表；开发验收优先写隔离表，例如 `fleur_calculation.calc_stock_rsi_daily_validation`，避免污染生产消费表。

执行前必须记录输入规模：

```sql
SELECT
    min(trade_date),
    max(trade_date),
    count() AS input_rows,
    countIf(close_price_forward_adj IS NOT NULL) AS valid_close_rows,
    uniqExact(security_code) AS symbols
FROM fleur_intermediate.int_stock_quotes_daily_adj
```

### 5.2 日常增量优化

日常增量用于目标表已有完整历史 RSI 结果后的最新区间追加。

运行区间：

```text
request_from = 用户请求开始日期
request_to = 用户请求结束日期
effective_output_from = request_from
effective_output_to = request_to
input_to = request_to
```

每个证券的输入读取起点：

1. 如果目标区间前存在完整 RSI previous state，且 previous state 之后到 `request_from` 之前不存在未物化的有效 close，则从 previous state 日期开始读取或初始化。
2. previous state 日期对应的 close 只用于初始化 `previous_close`；该 anchor 行不进入输出区间，也不推进状态。
3. 如果 previous state 之后到 `request_from` 之前存在未物化的有效 close，第一版 `append-latest` 必须拒绝并提示使用更早的 `request_from` 或 `replace-cascade` 从缺口起点补算；summary 或错误 payload 必须记录建议补算起点 `gap_fill_from`。
4. 如果目标区间内第一条有效 close 之前没有任何历史有效 close，则从该证券首个可用输入行开始，按新证券处理。
5. 如果 previous state 不完整、缺失或与输入历史不一致，则回退 full-history 或至少回读到能完整重建所有 50 窗口状态的安全起点。
6. 多证券运行时可以按证券决定 `input_from`；第一版为了复用批量读取路径，可以取所有受影响证券中最早 `input_from` 作为一次性读取起点，但 summary 必须记录实际全局最早读取日期。
7. 批量读取使用全局最早 `input_from` 时，每个 per-security worker 必须携带自己的 continuation anchor：`previous_state_trade_date`、`previous_close` 和 gap policy。worker 对 anchor 日期及之前的额外输入行只用于校验或跳过，不得重复推进 RSI 状态。
8. `append-latest` 的受影响证券集合应来自目标请求区间内实际存在输入行的证券；省略 `--symbols` 表示在请求区间内有输入行的全市场证券，不要求为没有新区间输入的证券补输出行。
9. 如果请求区间内某证券只有 close 为空的行，仍应输出这些行且 RSI 全为空；如果该证券此前有 previous state，则状态保持不变但不写入状态列。

优化目标：

1. 对成熟且无结果缺口的证券，日常增量只读取 previous state anchor + 目标区间行，不需要回看 50 个窗口。
2. 对新上市或状态缺失证券，自动使用 `mixed` 策略，避免用空状态截断 RSI。
3. 增量输出必须与同一证券全历史一次性计算结果一致。
4. `append-latest` 写入前检查目标表中同证券同日或更晚结果；如果存在则拒绝，提示使用 `replace-cascade`。

### 5.3 历史修正和 replace-cascade

复权因子、历史 close 修正或源数据回补会改变请求区间之后的所有 RSI 递推状态。因此生产历史修正必须使用 `replace-cascade`。

级联规则：

```text
effective_output_from = request_from
effective_output_to = 受影响证券在输入表中的最新 trade_date，且不早于 request_to
input_to = effective_output_to
```

状态启动规则：

1. 如果 `request_from` 前存在完整 RSI previous state，且 previous state 所在日期之前的历史没有被本次修正影响，可以从 previous state 和该日期对应 close 延续。
2. 如果修正影响 previous state 本身，必须从更早的安全起点或全历史重新推导。
3. 第一版为了简单正确，`replace-cascade` 可默认对受影响证券从首个有效 close 重新推导；优化版再引入 previous state 安全延续。
4. staging 保留旧行时，只保留未受影响证券，或受影响证券中不在 `effective_output_from..effective_output_to` 的旧行。
5. staging validation 必须按 `(security_code, trade_date)` 检查重复 key，且覆盖所有受影响年度分区。
6. 如果 `replace-cascade` 从首个有效 close 重新推导，但 `request_from` 晚于首个输入日期，`input_from` 可以早于 `effective_output_from`；只写出 `effective_output_from..effective_output_to`，不得把 warm-up 历史行写入本次替换窗口之外。
7. 如果 `request_from` 前存在 close 为空的输入行，不能把空行当作 continuation anchor；anchor 必须是最近一条有完整 RSI state 的有效 close 结果行，或 full-history 重新推导出的上一有效 close。

### 5.4 性能优化方向

RSI 指标本身计算量小，主要瓶颈预期在 ClickHouse I/O、分组和写入。

第一版必须实现：

1. RowBinary 输入读取，避免 TSV/JSON 文本解析。
2. 按 `security_code` 分组后 Rayon per-security worker 并行。
3. 单证券内部一次遍历同时计算 6 个 RSI 窗口。
4. 输出使用固定字段结构，不使用 per-row `BTreeMap` / `HashMap` 存储窗口值。
5. RowBinary 批量写入，默认 `insert_batch_size = 10_000`，全量验收可调大到 `100_000`。
6. summary 中记录 `read_input_ms`、`read_state_ms`、`group_ms`、`compute_ms`、`write_ms`、`staging_ms`、`partition_replace_ms`、`total_ms`。

后续可选优化：

1. 将 RowBinary 读取后的分组阶段改为流式分段计算，减少全量 `Vec<RsiInput>` 和二次遍历。
2. 对 previous state 查询做按证券集合预聚合，降低日常增量 `read_state_ms`。
3. 评估专用状态表，减少从宽结果表读取 12 个内部状态列的成本。
4. 评估 Native protocol 长连接或 ClickHouse local ingestion，降低超大全量写入耗时。

## 6. 目标输出模型

### 6.1 表名

Furnace 直接写入表：

```text
fleur_calculation.calc_stock_rsi_daily
```

dbt intermediate wrapper：

```text
fleur_intermediate.int_stock_rsi_daily
```

### 6.2 Grain

每证券、交易日一行。第一版固定使用前复权口径和 canonical RSI 参数集合：

```text
security_code
trade_date
```

RSI 窗口不作为业务 grain 的一部分，而是固化为宽表字段。若后续需要同表保存多组参数，必须重新设计唯一键、dbt tests 和下游消费口径。

### 6.3 ClickHouse 字段草案

```text
security_code String
trade_date Date
rsi_6 Nullable(Float64)
rsi_12 Nullable(Float64)
rsi_14 Nullable(Float64)
rsi_24 Nullable(Float64)
rsi_25 Nullable(Float64)
rsi_50 Nullable(Float64)
avg_gain_6_state Nullable(Float64)
avg_loss_6_state Nullable(Float64)
avg_gain_12_state Nullable(Float64)
avg_loss_12_state Nullable(Float64)
avg_gain_14_state Nullable(Float64)
avg_loss_14_state Nullable(Float64)
avg_gain_24_state Nullable(Float64)
avg_loss_24_state Nullable(Float64)
avg_gain_25_state Nullable(Float64)
avg_loss_25_state Nullable(Float64)
avg_gain_50_state Nullable(Float64)
avg_loss_50_state Nullable(Float64)
```

Engine 和排序：

```text
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)
```

写入规则：

1. `dry-run` 只读输入、计算和输出 summary，不建表、不写入。
2. `append-latest` 建表后检查目标表中同证券同日或更晚结果；如果存在则拒绝。
3. `replace-cascade` 写 staging，保留未受影响行，校验 staging 无重复 key，再年度分区替换。
4. INSERT 使用 RowBinary 和批量写入。
5. `calc_stock_rsi_daily` 不写 `run_id` 或 `computed_at`；运行审计只进入 Dagster materialization metadata 和报告。
6. DDL 建表必须由 `furnace-io` 负责，dbt 不负责创建 calculation 物理表。
7. RowBinary 输出字段顺序必须与 DDL 完全一致，并有单元测试覆盖 nullable marker 和 Float64 little-endian 编码。
8. 所有 RSI SQL helper 必须接受 `output_table` 参数；不得把 `fleur_calculation.calc_stock_rsi_daily` 硬编码到 staging、insert、retain、validate 或 partition replace 路径中，否则隔离库全量验收不可执行。
9. per-security worker 不直接写 ClickHouse；合并后的输出顺序必须确定，建议按 `(trade_date, security_code)` 排序后批量写入，以匹配表排序键并便于验收。
10. 状态列可以为空，但非空时必须为有限 `Float64` 且 `avg_gain_*_state >= 0`、`avg_loss_*_state >= 0`。

### 6.4 dbt wrapper 字段

`int_stock_rsi_daily` 只 select 业务字段：

```text
security_code
trade_date
rsi_6
rsi_12
rsi_14
rsi_24
rsi_25
rsi_50
```

不暴露内部状态字段：

```text
avg_gain_*_state
avg_loss_*_state
```

## 7. Rust 设计

### 7.1 `furnace-core` RSI 模块

新增模块建议：

```text
engines/crates/furnace-core/src/indicators/rsi.rs
```

核心类型建议：

```text
RsiInput {
  trade_date,
  close_price
}

RsiParams {
  windows
}

RsiWindowState {
  avg_gain,
  avg_loss
}

RsiState {
  previous_close,
  previous_close_trade_date,
  window_states
}

RsiOutput {
  trade_date,
  rsi_6,
  rsi_12,
  rsi_14,
  rsi_24,
  rsi_25,
  rsi_50,
  avg_gain_6_state,
  avg_loss_6_state,
  ...
}
```

实现要求：

1. `RsiParams::default()` 返回 `[6, 12, 14, 24, 25, 50]`。
2. `RsiParams::is_canonical()` 校验窗口集合严格等于默认集合。
3. 输入必须按 `trade_date` 严格升序。
4. 单证券内部一次遍历同时维护所有窗口的启动累积和递推状态。
5. `None` close 不形成涨跌幅、不推进状态、不改变 previous valid close。
6. 所有状态和输入值必须是有限数。
7. 核心 API 不依赖 ClickHouse、Dagster、dbt、Rayon、CLI 参数或环境变量。
8. previous state continuation 必须区分“初始化 previous close”和“消费输入行”。previous state anchor 行不产生输出、不进入 gain/loss 队列、不推进 Wilder 状态。
9. 如果调用方传入 `previous_state_trade_date`，核心计算必须拒绝或跳过该日期及之前的输入行；推荐由 `furnace-io` 在 worker 入口先裁剪输入，再调用核心 API。
10. 对 full-history 计算，`previous_close` 和 `window_states` 均为空；对 previous-state 计算，二者必须同时存在，否则返回错误并让 I/O 层回退 full-history。

### 7.2 可选公共算子

如实现中发现 RSI Wilder smoothing 可作为稳定公共算子，新增：

```text
engines/crates/furnace-core/src/operators/wilder.rs
```

建议 API：

```text
WilderAverage
WilderState
calculate_wilder_average_series(values, window, previous_state)
```

算子语义：

1. 前 `n` 个有效值用 SMA 启动。
2. 启动后使用 `(previous * (n - 1) + current) / n` 递推。
3. `None` 输入不推进状态。
4. window 必须大于 0。

如果该抽象会让第一版复杂化，可以只在 `indicators::rsi` 内部实现私有小结构，后续再抽取公共算子。

## 8. `furnace-io` 运行路径

目标：

1. 新增 RSI run request、summary、result row、DDL、staging 和 run 函数。
2. 复用 KDJ/MA 的 ClickHouse executor、RowBinary、timing、Rayon 并行和 partition replace 模式。
3. 支持 `dry-run`、`append-latest`、`replace-cascade`。

建议新增或复用结构：

```text
RsiRunRequest
RsiRunSummary
RsiInputRow
RsiResultRow
RsiPreviousStateRow
run_rsi
create_rsi_output_table_sql
rsi_staging_table_name
replace_rsi_partition_sql
```

输入 SQL：

```sql
SELECT
    security_code,
    trade_date,
    close_price_forward_adj
FROM fleur_intermediate.int_stock_quotes_daily_adj
WHERE trade_date >= {input_from}
  AND trade_date <= {input_to}
  AND {optional symbols filter}
ORDER BY security_code, trade_date
FORMAT RowBinary
```

previous state SQL 需要同时读取：

1. 目标区间前最近完整 RSI 状态行。
2. 输入表中 previous state 日期对应的 close。
3. 输入表中目标区间前、且晚于 previous state 日期的有效 close 数量，用于发现结果缺口。

previous state 查询口径：

1. 完整 RSI 状态行必须满足所有 `avg_gain_*_state IS NOT NULL` 且所有 `avg_loss_*_state IS NOT NULL`。
2. previous state close anchor 应取 previous state 日期对应的输入 close，而不是 `request_from` 前最近有效 close。这样才能避免跳过 previous state 之后的有效涨跌幅。
3. 如果 previous state 日期对应的输入 close 已缺失或变为空，说明上游历史被修正，必须回退 full-history 或 `replace-cascade`。
4. 如果 `previous_state_trade_date < trade_date < request_from` 范围内存在 `close_price_forward_adj IS NOT NULL` 的输入行，且这些行没有完整 RSI 结果，则存在结果缺口。第一版 `append-latest` 必须拒绝，并提示使用更早的 `request_from` 或 `replace-cascade` 从缺口起点补算。
5. previous state 之后、`request_from` 之前只有 close 为空的输入行是合法情况；空行不推进状态。
6. state 查询和 gap 查询都必须按证券返回，不能用全局一条记录套用到多证券。
7. dry-run 也应执行同样的 previous state 判定，确保 summary 中的 `rsi_state_source` 能反映真实增量路径。

summary 字段：

```text
indicator = "rsi"
request_from / request_to
effective_output_from / effective_output_to
input_from / input_to
mode
symbols_count
input_rows
output_rows
valid_close_rows
null_indicator_rows
affected_years
retained_rows
staging_table
staging_validation
partition_replace
rsi_windows
rsi_state_source
gap_symbols_count
gap_fill_from
run_id
writes_applied
performance_metrics
```

## 9. CLI 子命令

目标：

1. 在 `engines/crates/furnace/src/main.rs` 新增 `rsi` 子命令。
2. 复用 KDJ/MA 的参数解析、错误输出和 JSON summary 模式。

CLI 形态：

```bash
cargo run --release -p furnace -- rsi \
  --from 2026-01-01 \
  --to 2026-01-31 \
  --mode dry-run \
  --input-table fleur_intermediate.int_stock_quotes_daily_adj \
  --output-table fleur_calculation.calc_stock_rsi_daily \
  --price-column close_price_forward_adj \
  --insert-batch-size 10000 \
  --output-format json
```

CLI 参数口径：

1. `--input-table` 默认 `fleur_intermediate.int_stock_quotes_daily_adj`。
2. `--output-table` 默认 `fleur_calculation.calc_stock_rsi_daily`；全量验收如果使用隔离 database，必须允许通过该参数指向隔离表。
3. `--price-column` 默认且生产只允许 `close_price_forward_adj`。
4. 写入模式下，如果 `--input-table` 或 `--price-column` 偏离 canonical 口径，必须拒绝；隔离验收只允许改变 `--output-table`。
5. `--symbols` 省略或传空集合表示全市场。
6. 多证券显式传参使用逗号分隔代码，例如 `--symbols 000001.SZ,600000.SH`。
7. 不要求支持字面值 `--symbols all`；如果实现选择支持，必须把 `all` 明确解析为全市场，而不是证券代码。
8. 生产写入模式下，如果解析后的证券集合为空且请求区间内输入表也没有任何证券，必须拒绝写入，避免创建空结果分区。

CLI 测试：

1. `rsi --mode dry-run --output-format json` 返回 JSON object。
2. 未知 mode 返回非 0。
3. 非 canonical price column 在写入模式下拒绝。
4. `--symbols` 解析与 KDJ/MA 保持一致。
5. 省略 `--symbols` 等价全市场。
6. 写入模式允许自定义 `--output-table` 到隔离表，但不允许改变 canonical input table 和 price column。

## 10. dbt 接入

目标：

1. 在 `sources_fleur_calculation.yml` 新增 `calc_stock_rsi_daily` source。
2. 新增 `pipeline/elt/models/intermediate/int_stock_rsi_daily.sql` thin wrapper。
3. 新增 `pipeline/elt/models/intermediate/int_stock_rsi_daily.yml` 文档和 tests。

dbt source 应包含 calculation 表所有字段，包括内部状态字段；wrapper 只暴露业务字段。

dbt tests：

1. `security_code` not null + A 股代码格式。
2. `trade_date` not null。
3. `security_code + trade_date` 唯一。
4. `rsi_*` 字段允许为 `NULL`，但非空时应在 `[0, 100]` 范围内。
5. wrapper 不暴露 `avg_gain_*_state` 或 `avg_loss_*_state`。
6. source YAML 应记录内部状态字段仅供 Furnace 延续计算使用，下游模型不得直接依赖这些字段。

实施完成后运行：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_rsi_daily
uv run python elt/scripts/validate_field_glossary.py
```

## 11. Dagster 接入

目标：

1. 扩展 `FurnaceCliResource` 支持 `run_rsi`。
2. 新增 `FurnaceRsiCliRequest` / result dataclass。
3. 新增 `calc_stock_rsi_daily` asset。
4. 新增 `furnace__rsi_daily_job`、`furnace__rsi_backfill_job`、`furnace__rsi_dry_run_job`。
5. 可选新增 RSI daily schedule；如果暂不启用 schedule，必须在文档中说明由手动 job 或后续计划开启。

Dagster asset：

```text
AssetKey(["fleur_calculation", "calc_stock_rsi_daily"])
```

上游：

```text
AssetKey(["int_stock_quotes_daily_adj"])
```

metadata 至少包含：

```text
request_range
effective_output_range
input_range
mode
symbols_count
input_rows
output_rows
valid_close_rows
null_indicator_rows
affected_years
retained_rows
rsi_windows
rsi_state_source
gap_symbols_count
gap_fill_from
staging_validation
partition_replace
performance_metrics
writes_applied
```

实施完成后运行：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

## 12. 全量验收方案

全量验收必须覆盖：

| 场景 | 命令模式 | 是否写表 | 目的 |
|------|----------|----------|------|
| 全市场全历史 dry-run | `dry-run` | 否 | 验证并行计算能跑完整数据量，观察性能和内存 |
| 全市场全历史 replace-cascade | `replace-cascade` | 是 | 验证 staging、RowBinary 写入和年度分区替换可承载完整数据量 |
| dbt wrapper build | `dbt build --select int_stock_rsi_daily` | 读 RSI 表 | 验证 source、wrapper 和 tests |
| Dagster dry-run asset | `dg launch` 或等价 job | 否 | 验证 Dagster resource、config 和 metadata |

全量运行日期范围必须从输入表实际最早交易日到最新交易日，不允许只挑样本区间代替。

推荐 dry-run 命令：

```bash
cd engines
RAYON_NUM_THREADS=8 cargo run --release -p furnace -- rsi \
  --from <min_trade_date> \
  --to <max_trade_date> \
  --mode dry-run \
  --insert-batch-size 10000 \
  --output-format json
```

推荐写入验收命令：

```bash
cd engines
RAYON_NUM_THREADS=8 cargo run --release -p furnace -- rsi \
  --from <min_trade_date> \
  --to <max_trade_date> \
  --mode replace-cascade \
  --output-table fleur_calculation.calc_stock_rsi_daily_validation \
  --run-id furnace_rsi_full_market_validation_<yyyymmdd> \
  --insert-batch-size 100000 \
  --output-format json
```

验收报告路径：

```text
docs/jobs/reports/<date>-furnace-rsi-full-market-parallel-validation.md
```

验收报告必须包含：

1. 命令、环境、git commit 或 worktree 标识。
2. 输入日期范围、输入行数、有效 close 行数、证券数。
3. summary JSON 的关键字段。
4. `performance_metrics` 完整内容。
5. `calc_stock_rsi_daily` 或隔离输出表的行数和唯一键检查。
6. 年度分区替换结果和 ClickHouse part 数量检查。
7. 至少 3 只证券的 spot check：`rsi_6`、`rsi_14`、`rsi_50` 与固定样本或独立脚本结果一致。
8. 至少 1 只上市早期证券检查，证明 `RSI(50)` 在第 51 个有效 close 前为 `NULL`。
9. 至少 1 段缺价样本检查，证明缺价行不推进状态，下一有效 close 使用上一有效 close 计算涨跌幅。
10. 非空 RSI 值范围检查，所有 `rsi_*` 应在 `[0, 100]`。
11. 增量一致性检查：选择至少 3 只证券，将小窗口 dry-run 的 previous-state 结果与同区间 full-history 结果逐字段比较。
12. 状态列健康检查：所有非空 `avg_gain_*_state` / `avg_loss_*_state` 非负，且不存在 `NaN` / infinite 写入。
13. `append-latest` 缺口保护检查：人为选择存在历史结果缺口的证券或构造 fixture，确认命令拒绝写入并报告 `gap_symbols_count`。

建议 SQL 检查：

```sql
SELECT
    count() AS rows,
    uniqExact(security_code) AS symbols,
    count() - uniqExact(security_code, trade_date) AS duplicate_keys
FROM fleur_calculation.calc_stock_rsi_daily_validation
```

```sql
SELECT count() AS out_of_range_rows
FROM fleur_calculation.calc_stock_rsi_daily_validation
WHERE (rsi_6 IS NOT NULL AND (rsi_6 < 0 OR rsi_6 > 100))
   OR (rsi_12 IS NOT NULL AND (rsi_12 < 0 OR rsi_12 > 100))
   OR (rsi_14 IS NOT NULL AND (rsi_14 < 0 OR rsi_14 > 100))
   OR (rsi_24 IS NOT NULL AND (rsi_24 < 0 OR rsi_24 > 100))
   OR (rsi_25 IS NOT NULL AND (rsi_25 < 0 OR rsi_25 > 100))
   OR (rsi_50 IS NOT NULL AND (rsi_50 < 0 OR rsi_50 > 100))
```

```sql
SELECT count() AS negative_state_rows
FROM fleur_calculation.calc_stock_rsi_daily_validation
WHERE (avg_gain_6_state IS NOT NULL AND avg_gain_6_state < 0)
   OR (avg_loss_6_state IS NOT NULL AND avg_loss_6_state < 0)
   OR (avg_gain_12_state IS NOT NULL AND avg_gain_12_state < 0)
   OR (avg_loss_12_state IS NOT NULL AND avg_loss_12_state < 0)
   OR (avg_gain_14_state IS NOT NULL AND avg_gain_14_state < 0)
   OR (avg_loss_14_state IS NOT NULL AND avg_loss_14_state < 0)
   OR (avg_gain_24_state IS NOT NULL AND avg_gain_24_state < 0)
   OR (avg_loss_24_state IS NOT NULL AND avg_loss_24_state < 0)
   OR (avg_gain_25_state IS NOT NULL AND avg_gain_25_state < 0)
   OR (avg_loss_25_state IS NOT NULL AND avg_loss_25_state < 0)
   OR (avg_gain_50_state IS NOT NULL AND avg_gain_50_state < 0)
   OR (avg_loss_50_state IS NOT NULL AND avg_loss_50_state < 0)
```

## 13. 测试和质量门禁

### 13.1 Rust

实施完成后运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Rust 测试必须覆盖：

1. `RsiParams` canonical 窗口校验。
2. 空输入返回空输出。
3. 非递增 `trade_date` 返回错误。
4. 首个有效 close 不输出 RSI。
5. `RSI(n)` 首个非空值出现在第 `n + 1` 个有效 close。
6. 缺价行不推进状态，下一有效 close 使用上一有效 close。
7. 连续上涨导致 `avg_loss = 0` 时 RSI 为 100。
8. 连续下跌导致 `avg_gain = 0` 时 RSI 为 0。
9. 无涨跌时 RSI 为 50。
10. previous state 延续与全历史一次性计算结果一致。
11. 多窗口固定样本 golden test。
12. previous state anchor 行不会产生 `change = 0`，不会重复推进状态。
13. 批量读取早于 per-security previous state 日期的额外输入行会被跳过。
14. `avg_gain = 0` / `avg_loss = 0` 的状态值仍被当作完整有效状态。
15. RowBinary input/output 编码测试。
16. staging SQL 和 partition replace SQL 测试。
17. 并行输出与串行输出一致性测试。
18. KDJ 和 MA 既有测试全部保持通过。

### 13.2 dbt

实施完成后运行：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_rsi_daily
uv run python elt/scripts/validate_field_glossary.py
```

### 13.3 Dagster / Python

实施完成后运行：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

Dagster 测试必须覆盖：

1. `FurnaceCliResource.command_for_rsi_request` 生成正确命令。
2. RSI summary metadata 映射正确。
3. RSI asset key、upstream dependency、group、tags 和 owners 符合 KDJ/MA 模式。

## 14. 验收清单

实施完成必须同时满足以下条件：

1. `furnace-core::indicators::rsi` 存在，并提供单证券纯计算 API。
2. `furnace-core` 不依赖 ClickHouse、Dagster、dbt、Rayon、CLI 参数或环境变量。
3. RSI 使用 `close_price_forward_adj`，并固定输出 `rsi_6`、`rsi_12`、`rsi_14`、`rsi_24`、`rsi_25`、`rsi_50`。
4. RSI 采用 Wilder smoothing，启动阶段使用前 `n` 个有效涨跌幅的 SMA。
5. 空 close 行输出空 RSI，且不推进状态。
6. `avg_gain_*_state` 和 `avg_loss_*_state` 可支持日常增量无截断延续。
7. previous-state 增量不会重复消费 previous state anchor 行，结果与 full-history 对齐。
8. 批量全局 `input_from` 早于某证券 continuation anchor 时，该证券 anchor 日期及之前的额外行不会推进状态。
9. `append-latest` 能检测 previous state 之后、请求区间之前的有效输入缺口，并拒绝跳算。
10. `furnace rsi` 支持 `dry-run`、`append-latest`、`replace-cascade`。
11. `furnace rsi` summary 包含 `indicator="rsi"`、`rsi_windows`、`rsi_state_source`、`gap_symbols_count` 和 `performance_metrics`。
12. `calc_stock_rsi_daily` 可以被自动创建，并使用 RowBinary 批量写入。
13. `replace-cascade` 使用 staging + 年度分区替换，且 staging validation 无重复 `(security_code, trade_date)`。
14. `int_stock_rsi_daily` dbt wrapper 只 select Furnace 输出，不重写公式。
15. Dagster 能物化 `fleur_calculation/calc_stock_rsi_daily` asset，并记录 summary metadata。
16. Rust、dbt、Dagster/Python 质量门禁全部通过。
17. 全市场、全历史 dry-run 并行计算成功完成，`performance_metrics.parallelism = "rayon"`，`worker_threads >= 2`。
18. 全市场、全历史 replace-cascade 写入验收成功完成；如果目标环境不允许写生产表，必须在同等数据量的隔离 ClickHouse database 中完成，并在报告中说明 database、表名和隔离方式。
19. 全量验收的 `symbols_count` 必须与输入表证券数对齐；不允许用显式少量证券列表代替全市场。
20. 全量验收后，`calc_stock_rsi_daily` 或隔离输出表满足每证券、交易日唯一，输出行数与请求日期范围内输入行情行数的预期一致。
21. 所有非空 `rsi_*` 值在 `[0, 100]` 范围内，所有非空状态列非负。
22. spot check 证明 `RSI(6)`、`RSI(14)`、`RSI(50)` 与独立计算结果一致。
23. 生成 `docs/jobs/reports/<date>-furnace-rsi-full-market-parallel-validation.md`，报告包含命令、summary、性能、行数、唯一性、分区替换和 spot check 结果。

## 15. 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| RSI 启动口径不一致 | 与常见 Wilder RSI 或下游预期不一致 | 文档固定为前 `n` 个有效涨跌幅 SMA 启动；golden test 覆盖首个非空日期 |
| 缺价后 previous close 处理错误 | 涨跌幅错位，结果偏差 | 明确缺价不推进状态，下一有效 close 使用上一有效 close；增加缺价样本测试 |
| 增量缺少 previous state 日期 close | 目标区间第一条有效 close 无法正确计算 change | 增量 previous state 查询必须校验输入表中 state 日期对应 close；缺失时回退 full-history |
| 增量重复消费 previous state anchor | 多出一条 `change = 0`，所有后续 RSI 偏移 | previous state anchor 只初始化状态，不产生输出、不推进 Wilder；测试覆盖 |
| 增量跳过 previous state 后的缺口输入 | 漏算一段有效涨跌幅，后续 RSI 全部偏差 | `append-latest` 检测 previous state 到 request_from 的有效输入缺口并拒绝跳算 |
| 全局 `input_from` 早于证券状态 anchor | 旧输入行重复推进某些证券状态 | 每证券携带 continuation anchor，worker 跳过 anchor 日期及之前的额外输入 |
| previous state 不完整 | RSI(50) 等长窗口被截断 | 只允许读取所有 canonical 窗口状态均非空的完整状态；否则回读历史推导 |
| 全量写入耗时过长 | 验收阻塞或 ClickHouse part 压力过大 | 先 dry-run 量级评估；写入使用 release binary、RowBinary、合理 batch 和 part 健康检查 |
| 内部状态字段被下游误用 | 消费契约混乱 | dbt wrapper 不暴露状态字段；YAML 明确状态字段仅 calculation 内部使用 |
| 复制 MA/KDJ I/O 代码过多 | 后续维护三套逻辑 | 抽取 staging、timing、RowBinary、summary helper 的稳定重复片段 |

## 16. 推荐实施顺序

1. 完成 `furnace-core::indicators::rsi` 和核心 golden tests。
2. 如有必要，抽取私有或公共 Wilder smoothing helper。
3. 在 `furnace-io` 复用 MA close-only 输入路径实现 `run_rsi` dry-run。
4. 扩展 `furnace` CLI，先打通 `rsi` dry-run JSON summary。
5. 实现 RSI output DDL、RowBinary 写入、append-latest 和 replace-cascade。
6. 增加 previous state 查询和日常增量优化。
7. 增加并行一致性测试和性能 metrics。
8. 增加 dbt source/wrapper/tests。
9. 增加 Dagster resource、asset、jobs 和 metadata。
10. 跑 Rust/dbt/Dagster 质量门禁。
11. 跑全市场全历史 dry-run 并行验收。
12. 跑全市场全历史 replace-cascade 写入验收。
13. 编写全量验收报告，并根据报告修复遗留问题。
