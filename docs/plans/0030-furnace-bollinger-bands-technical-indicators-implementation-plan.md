# Plan 0030: Furnace Bollinger Bands 日线技术指标实施方案

日期：2026-06-08

状态：Draft

关联文档：

- `docs/plans/0029-furnace-moving-average-technical-indicators-implementation-plan.md`
- `docs/RFC/0016-rust-furnace-compute-engine.md`
- `docs/plans/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- `docs/plans/0028-furnace-kdj-parallel-performance-implementation-plan.md`
- `engines/README.md`
- `engines/crates/furnace-core/src/operators/sma.rs`
- `engines/crates/furnace-core/src/operators/ema.rs`
- `engines/crates/furnace-core/src/indicators/moving_average.rs`
- `engines/crates/furnace-io/src/lib.rs`
- `engines/crates/furnace/src/main.rs`
- `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
- `pipeline/scheduler/src/scheduler/defs/resources/furnace.py`
- `pipeline/elt/models/sources_fleur_calculation.yml`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.yml`
- `pipeline/elt/models/intermediate/int_stock_ma_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_ma_daily.yml`

相关 skills：

- `rust-best-practices` / `rust-patterns` / `rust-testing`：Rust crate 边界、公共 rolling 算子、错误模型、测试和性能。
- `running-dbt-commands` / `using-dbt-for-analytics-engineering`：dbt source、thin wrapper、字段文档和定向 build。
- `dagster-expert`：Dagster asset、job、schedule、资源和 materialization metadata。
- `clickhouse-best-practices` / `clickhouse-architecture-advisor`：ClickHouse 宽表、RowBinary 批量写入、年度分区替换和全量回填验收。

## 1. 目标

在现有 Furnace Rust workspace 中新增日频 Bollinger Bands 指标计算能力，从：

```text
fleur_intermediate.int_stock_quotes_daily_adj
```

读取前复权收盘价 `close_price_forward_adj`，按三组固定配置计算布林带，并写入：

```text
fleur_calculation.calc_stock_boll_daily
```

再由 dbt thin wrapper 暴露为：

```text
fleur_intermediate.int_stock_boll_daily
```

完成后应满足：

1. `furnace boll` 支持按日期区间、证券集合和运行模式计算 Bollinger Bands 指标。
2. 所有指标使用 `close_price_forward_adj` 作为 `close` 输入，不混用未复权、后复权或其他价格口径。
3. 第一版只支持三组 canonical 配置：

| 配置 | 含义 |
|------|------|
| `BOLL(10, 1.5)` | 10 个有效 close 的 SMA 中轨，带宽倍数 1.5 |
| `BOLL(20, 2)` | 标准布林带默认配置，20 个有效 close 的 SMA 中轨，带宽倍数 2 |
| `BOLL(50, 2.5)` | 50 个有效 close 的 SMA 中轨，带宽倍数 2.5 |

4. 输出字段采用 calculation 层宽表结构，每证券、交易日一行，包含三组配置的 `MID`、`UP`、`DN`。
5. SMA 和 rolling standard deviation 基础算子放在 `furnace-core` 公共算子层；Bollinger Bands 指标模块只负责组合算子并映射业务字段。
6. Rust、ClickHouse、Dagster 和 dbt 的 ownership 边界沿用 KDJ/MA：公式在 `furnace-core`，I/O 和并行调度在 `furnace-io`，CLI 在 `furnace`，调度与 metadata 在 Dagster，消费契约在 dbt。
7. 生产写入使用 staging + 年度 `REPLACE PARTITION` 协议，保持幂等并避免高频 mutation。
8. 计算按证券维度并行；单证券内部按 `trade_date` 串行滚动计算。
9. 验收必须包含全市场、全历史数据量的并行计算运行，并记录性能和结果质量报告。

## 2. 非目标

本计划不做以下事情：

1. 不实现 MACD、RSI、DMA、TRIX 或其他未列入本计划的指标。
2. 不改变 `int_stock_quotes_daily_adj` 的复权逻辑、字段语义或物化策略。
3. 不在 dbt SQL 或 Dagster Python asset 中重写布林带公式。
4. 不让 Furnace 直接写入 `fleur_intermediate.int_stock_boll_daily`。
5. 不支持任意用户自定义 `(N, K)` 参数；第一版生产只允许三组 canonical 配置。
6. 不支持多价格口径、多频率、长表 `indicator_name/value` 结构或带宽衍生指标如 `WIDTH`、`%B`。
7. 不将同一证券时间序列按日期并行；rolling window 状态必须在单证券内串行推进。
8. 不同步重构已实现的 KDJ/MA 对外行为；如需抽取公共 helper，必须保持现有测试通过。

## 3. 公式和语义

### 3.1 canonical 公式

对每只证券按 `trade_date ASC` 排序后的有效 `close_price_forward_adj` 序列计算：

```text
MID(N) = MA(CLOSE, N)
STD(N) = STD(CLOSE, N)
UP(N, K) = MID(N) + K * STD(N)
DN(N, K) = MID(N) - K * STD(N)
```

标准默认配置：

```text
BOLL(20, 2)
```

本计划还要求支持：

```text
BOLL(10, 1.5)
BOLL(50, 2.5)
```

### 3.2 标准差口径

第一版固定使用总体标准差，分母为 `N`：

```text
STD(CLOSE, N) = sqrt(sum((close_i - MID)^2) / N)
```

实现要求：

1. `STD` 与 `MID` 使用同一个 rolling window 中的最近 `N` 个有效 close。
2. 有效 close 数量少于 `N` 时，`MID`、`UP`、`DN` 均输出 `NULL`。
3. `close_price_forward_adj IS NULL` 的行必须输出一行，但不进入 rolling window，也不推进任何 rolling 状态。
4. 当一行 close 为空时，该行所有 Bollinger Bands 业务字段均为 `NULL`。
5. 输入值必须为有限 `f64`；非有限值作为错误处理，不悄悄进入计算。
6. 不做四舍五入；以 `Float64` 输出，测试断言使用 `1e-9` 级别容差。
7. 若未来要改为样本标准差或兼容第三方公式，必须另立计划并迁移字段语义文档。

### 3.3 字段命名口径

字段名固定使用：

```text
boll_<mid|up|dn>_<N>_<K>
```

其中：

1. `<mid|up|dn>` 分别对应中轨、上轨、下轨。
2. `<N>` 使用窗口整数，不加 `n` 前缀。
3. `<K>` 使用倍数值；整数直接写数字，例如 `2`；小数使用 `p` 代替小数点，例如 `1p5`、`2p5`。
4. 字段名不使用 `k` 前缀，不使用 `1_5` 表示小数，不省略 K 参数。

三组 canonical 字段固定为：

| 配置 | MID | UP | DN |
|------|-----|----|----|
| `BOLL(10, 1.5)` | `boll_mid_10_1p5` | `boll_up_10_1p5` | `boll_dn_10_1p5` |
| `BOLL(20, 2)` | `boll_mid_20_2` | `boll_up_20_2` | `boll_dn_20_2` |
| `BOLL(50, 2.5)` | `boll_mid_50_2p5` | `boll_up_50_2p5` | `boll_dn_50_2p5` |

### 3.4 输出区间口径

`input_from..input_to` 可能早于用户请求区间，用于 rolling window warm-up。输出和写入必须只包含：

```text
effective_output_from..effective_output_to
```

要求：

1. lookback 行只用于填充 rolling window，不进入 `output_rows`，不写入目标表。
2. `null_indicator_rows` 只统计输出区间内的行，不统计 lookback 行。
3. summary 必须同时保留 `input_valid_close_rows` 和 `output_valid_close_rows`，避免混淆读取行和写出行的统计范围。
4. 用户请求必须满足 `request_from <= request_to`，否则 CLI 返回非 0。
5. `effective_output_to` 必须不早于 `effective_output_from`；如果上游没有任何落入输出区间的行情行，写入模式必须拒绝或输出 `writes_applied=false` 的明确 summary，不得创建空分区替换。

## 4. 当前事实基线

### 4.1 Rust workspace

当前 Rust workspace 已有三层 crate：

```text
engines/
├── crates/furnace/       # CLI binary，已有 furnace kdj / furnace ma
├── crates/furnace-core/  # 纯指标计算，已有 KDJ、MA 和 SMA/EMA 公共算子
└── crates/furnace-io/    # ClickHouse I/O、RowBinary、Rayon、staging/replace
```

可复用事实：

1. `furnace-core::operators::RollingSma` 已定义 `None` 不进入窗口、不推进状态的 SMA 语义。
2. MA 计划 0029 已建立公共算子层和 Moving Average 指标模块边界。
3. `furnace-io` 已有 ClickHouse executor、RowBinary 读取/写入、证券维度 Rayon 并行、staging 表、年度分区替换和 `PerformanceMetrics`。
4. `furnace` CLI 已有 KDJ/MA 子命令模式，可作为 `boll` 子命令模板。
5. Dagster `furnace` definitions 已有 KDJ/MA asset、job、schedule 和 metadata 映射模式。
6. dbt 已有 calculation source + intermediate thin wrapper 模式。

### 4.2 输入模型

Bollinger Bands 输入默认来自：

```text
fleur_intermediate.int_stock_quotes_daily_adj
```

第一版只读取：

| 字段 | 用途 |
|------|------|
| `security_code` | 证券代码 |
| `trade_date` | 交易日 |
| `close_price_forward_adj` | Bollinger Bands 的 canonical `close` 输入 |

输入必须按以下顺序提供给单证券核心计算：

```text
security_code ASC, trade_date ASC
```

输入和输出行口径：

1. 输出 grain 与输入行情 grain 对齐，为每证券、交易日一行。
2. 请求输出区间内即使 `close_price_forward_adj IS NULL` 也要输出一行，所有 Bollinger Bands 字段为 `NULL`。
3. `input_rows` 统计实际读取的行情行数，不只统计有效 close 行。
4. `output_rows` 统计实际落入 `effective_output_from..effective_output_to` 的行情行数。
5. `input_valid_close_rows` 和 `output_valid_close_rows` 可作为运行报告辅助指标，但不是输出行数口径。
6. 同一证券内 `trade_date` 必须严格递增；如果输入中出现重复日期或乱序，核心计算返回错误，不在 Rust 中静默去重。
7. `null_indicator_rows` 统计所有 Bollinger Bands 业务字段都为 `NULL` 的输出行；只要任一配置的 `MID`、`UP` 或 `DN` 非空，该行不计入 `null_indicator_rows`。

## 5. 复用原则

本实施必须优先复用 KDJ/MA 已有基础设施，避免为 Bollinger Bands 复制出第三套不兼容路径。

### 5.1 必须复用

| 现有资源 | 复用方式 |
|----------|----------|
| `ClickHouseExecutor` | Bollinger Bands 运行路径继续通过同一 executor 抽象执行 SQL 和 RowBinary 写入 |
| `RollingSma` 语义 | `MID` 与 rolling standard deviation 使用同样的有效值窗口和 `None` 处理规则 |
| RowBinary 输入解析 | 复用 MA 的只含 close 的输入行读取模式 |
| RowBinary 输出写入 | 复用现有 result row 编码模式，新增 Bollinger Bands result row |
| staging + `REPLACE PARTITION` | 复用 KDJ/MA 的 staging 建表、旧行保留、校验、年度分区替换和清理流程 |
| `PerformanceMetrics` | Bollinger Bands summary 使用同类阶段计时字段 |
| Rayon 证券维度并行 | 复用 per-security worker 模式 |
| CLI 参数解析 | 在 `furnace/src/main.rs` 中按 KDJ/MA 模式新增 `boll` 子命令 |
| Dagster `FurnaceCliResource` | 扩展为支持 `run_boll`，不新增重复资源类 |
| dbt source/wrapper 结构 | 在现有 source YAML 中新增 Bollinger Bands 表，新增 thin wrapper model |

### 5.2 可以抽取共用的代码

实施中如果发现 KDJ、MA 和 Bollinger Bands 的 I/O 代码高度重复，应优先抽取小范围 helper：

1. staging 表名规范化。
2. 年度分区替换 SQL 构造。
3. `affected_years`、`retained_rows`、staging validation。
4. `json_optional_string`、JSON array/number helpers。
5. `time_result` 和 `RunTimings`。
6. symbol filter SQL 构造和输入区间解析。
7. close-only input row 读取和 per-security grouping。

抽取原则：

- 只抽真实重复的稳定边界。
- 不为了追求泛型化引入复杂 trait 层。
- 不改变 KDJ/MA 对外行为；抽取后现有测试必须全部通过。

## 6. 架构决策

### 6.1 公共 rolling standard deviation 算子

新增模块建议：

```text
engines/crates/furnace-core/src/operators/stddev.rs
```

并在：

```text
engines/crates/furnace-core/src/operators/mod.rs
```

导出：

```text
RollingStdDev
calculate_stddev_series
```

算子语义：

1. `window` 必须大于 0。
2. `None` 输入不进入有效窗口，也不改变状态。
3. 有效值数量少于 `window` 时输出 `None`。
4. 当前窗口有效值数量等于 `window` 时输出总体标准差。
5. 输入值必须是有限 `f64`。
6. 标准差输出不得为负；浮点误差导致方差出现极小负数时，只允许在 `variance >= -1e-12` 时 clamp 到 `0.0`，更小的负方差必须返回错误并暴露输入样本问题。

实现建议：

1. 使用 `VecDeque<f64>` 保存最近 `window` 个有效值。
2. 同时维护 `sum` 和 `sum_sq`，避免每行重新扫描窗口。
3. 方差使用：

```text
variance = (sum_sq / window) - mean * mean
```

4. 若全窗口数值相等，标准差必须输出 `0.0`，`UP == MID == DN`。
5. 如果 `RollingSma` 和 `RollingStdDev` 分开实现，Bollinger Bands 指标层必须保证两者接收完全相同的有效 close 序列；禁止对 `MID` 和 `STD` 使用不同的 null/filter 规则。

### 6.2 Bollinger Bands 指标模块

新增模块建议：

```text
engines/crates/furnace-core/src/indicators/bollinger_bands.rs
```

核心类型建议：

```text
BollInput {
  trade_date,
  close_price
}

BollConfig {
  window,
  multiplier
}

BollParams {
  configs
}

BollBand {
  mid,
  up,
  dn
}

BollOutput {
  trade_date,
  boll_mid_10_1p5,
  boll_up_10_1p5,
  boll_dn_10_1p5,
  boll_mid_20_2,
  boll_up_20_2,
  boll_dn_20_2,
  boll_mid_50_2p5,
  boll_up_50_2p5,
  boll_dn_50_2p5
}
```

固定 canonical 参数：

```text
[
  BollConfig { window: 10, multiplier: 1.5 },
  BollConfig { window: 20, multiplier: 2.0 },
  BollConfig { window: 50, multiplier: 2.5 },
]
```

实现要求：

1. 单证券 API 输入必须按 `trade_date` 严格升序。
2. 所有配置都只消费有效 `close_price_forward_adj`。
3. 当前行 close 为空时，所有 Bollinger Bands 字段输出 `None`，且 rolling 状态不推进。
4. 每组配置的 `MID` 和 `STD` 必须来自同一个窗口。
5. 每组配置在有效 close 数量不足 `N` 时，`MID`、`UP`、`DN` 同时为 `None`。
6. `UP >= MID >= DN` 应在标准差非负时自然成立；核心测试必须覆盖。
7. 输出字段必须使用第 3.3 节固定名称。
8. `BollConfig.multiplier` 必须为有限正数；canonical 判断使用固定配置表，不用直接浮点相等比较用户输入。

### 6.3 运行区间和 lookback 口径

Bollinger Bands 没有 EMA 式无限递推状态，lookback 只需要满足最大窗口 `N=50`。

运行区间定义：

```text
request_from = 用户请求开始日期
request_to = 用户请求结束日期
effective_output_from = request_from
effective_output_to =
  append-latest: request_to
  replace-cascade: 受影响证券在输入表中的最新 trade_date，且不早于 request_to
input_to = effective_output_to
```

`input_from` 选择规则：

1. 每个证券需要请求区间前至少 49 个有效 close，以支持 `BOLL(50, 2.5)` 在 `request_from` 当日输出。
2. 如果历史有效 close 不足 49 个，则从该证券可用最早输入日期开始读取。
3. 多证券运行时，可以先解析证券集合，再按证券确定各自需要的 `input_from`；第一版也可以取所有受影响证券中最早的 `input_from` 作为一次性读取起点，以复用 KDJ/MA 批量读取路径。
4. summary 中的 `input_from` 记录本次实际读取的最早日期；如果各证券 lookback 不同，记录全局最早读取日期。
5. `state_source` 固定为 `rolling-lookback`，表示结果只依赖有限 rolling window，不依赖跨运行持久状态。
6. lookback 选择必须按有效 close 计数，不按自然日或交易日简单回退 49 天。
7. 如果使用全局最早 `input_from` 一次性读取，核心计算仍必须按证券独立 warm-up；不得让其他证券的历史行影响当前证券窗口。

`replace-cascade` 级联规则：

1. 历史 close 修正最多影响受影响日期之后 49 个有效 close 的 `BOLL(50, 2.5)` 结果。
2. 为降低实现复杂度并保持与 KDJ/MA 一致，第一版生产 `replace-cascade` 将 `effective_output_to` 扩展到受影响证券的最新输入交易日。
3. 后续若要把 cascade 缩短到最大窗口影响范围，可另立优化计划，并必须证明年度分区替换和局部保留逻辑正确。
4. staging 保留旧行时，只保留未受影响证券，或受影响证券中不在 `effective_output_from..effective_output_to` 的旧行。
5. staging validation 必须按 `(security_code, trade_date)` 检查重复 key，且覆盖所有受影响年度分区。

### 6.4 ClickHouse 表和写入

沿用 calculation 层宽表模式：

```text
fleur_calculation.calc_stock_boll_daily
```

建议字段：

```text
security_code String
trade_date Date
boll_mid_10_1p5 Nullable(Float64)
boll_up_10_1p5 Nullable(Float64)
boll_dn_10_1p5 Nullable(Float64)
boll_mid_20_2 Nullable(Float64)
boll_up_20_2 Nullable(Float64)
boll_dn_20_2 Nullable(Float64)
boll_mid_50_2p5 Nullable(Float64)
boll_up_50_2p5 Nullable(Float64)
boll_dn_50_2p5 Nullable(Float64)
```

Engine 和排序：

```text
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)
```

写入规则：

1. `dry-run` 只读输入、计算和输出 summary，不建表、不写入。
2. `append-latest` 建表后检查目标表中同证券同日或更晚结果；如果存在则拒绝，提示使用 `replace-cascade`。
3. `replace-cascade` 写 staging，保留未受影响行，校验 staging 无重复 key，再年度分区替换。
4. INSERT 使用 RowBinary 和批量写入，默认 `insert_batch_size = 10_000`，可沿用 KDJ/MA 参数限制。
5. `calc_stock_boll_daily` 不写 `run_id` 或 `computed_at`；运行审计只进入 Dagster materialization metadata 和报告。
6. DDL 建表必须由 `furnace-io` 负责，dbt 不负责创建 calculation 物理表。
7. RowBinary 输出字段顺序必须与 DDL 完全一致，并有单元测试覆盖 nullable marker 和 Float64 little-endian 编码。
8. 所有 Bollinger Bands SQL helper 必须接受 `output_table` 参数；不得把 `fleur_calculation.calc_stock_boll_daily` 硬编码到 staging、insert、retain、validate 或 partition replace 路径中。

### 6.5 CLI 和 symbols 口径

沿用 KDJ/MA 的符号语义：

1. `--symbols` 省略或传空集合表示全市场。
2. 不要求支持字面值 `--symbols all`；如果实现选择支持，必须把 `all` 明确解析为全市场，而不是证券代码。
3. 多证券显式传参使用逗号分隔代码，例如 `--symbols 000001.SZ,600000.SH`。
4. 生产写入模式下，如果解析后的证券集合为空且输入表也没有任何证券，必须拒绝写入。

CLI 形态：

```bash
cargo run --release -p furnace -- boll \
  --from 2026-01-01 \
  --to 2026-01-31 \
  --mode dry-run \
  --input-table fleur_intermediate.int_stock_quotes_daily_adj \
  --output-table fleur_calculation.calc_stock_boll_daily \
  --price-column close_price_forward_adj \
  --insert-batch-size 10000 \
  --output-format json
```

参数口径：

1. `--input-table` 默认 `fleur_intermediate.int_stock_quotes_daily_adj`。
2. `--output-table` 默认 `fleur_calculation.calc_stock_boll_daily`；全量验收如果使用隔离 database，必须允许通过该参数指向隔离表。
3. `--price-column` 默认且生产只允许 `close_price_forward_adj`。
4. 第一版不暴露 `--window` 或 `--multiplier` 参数；三组 canonical 配置由代码常量固定。
5. 写入模式下，如果 `--input-table` 或 `--price-column` 偏离 canonical 口径，必须拒绝；隔离验收只允许改变 `--output-table`。

### 6.6 并行计算

并行粒度沿用 KDJ/MA：

```text
ClickHouse RowBinary input
  -> group by security_code
  -> Rayon per-security worker
  -> deterministic merge
  -> RowBinary batched write
```

要求：

1. 单证券内部严格按 `trade_date` 串行计算。
2. worker 不直接写 ClickHouse。
3. 合并后的输出顺序确定，建议按 `(trade_date, security_code)` 或最终插入要求排序。
4. JSON summary 必须包含 `performance_metrics.parallelism` 和 `performance_metrics.worker_threads`。

## 7. 实施阶段

### 阶段 1：公共 rolling standard deviation 算子

目标：

1. 新增 `furnace-core::operators::stddev` 模块。
2. 实现总体标准差 rolling 算子。
3. 为算子补独立单元测试。

测试覆盖：

1. window 为 0 返回错误。
2. 有效值不足窗口时输出 `None`。
3. `None` 输入不推进窗口和状态。
4. 全窗口值相等时输出 `0.0`。
5. rolling 窗口正常移除旧值。
6. 输入包含 `NaN` 或 infinite 值时报错。
7. 固定样本 `[1, 2, 3, 4, 5]`、window 5 的总体标准差为 `sqrt(2)`。

### 阶段 2：Bollinger Bands 核心指标

目标：

1. 新增 `furnace-core::indicators::bollinger_bands`。
2. 提供单证券纯计算 API。
3. 使用 `RollingSma` 和 `RollingStdDev` 组合出三组 canonical 字段。

测试覆盖：

1. 空输入返回空输出。
2. 非递增 `trade_date` 返回错误。
3. close 为空行所有指标为 `None`。
4. 窗口不足时对应配置三列都为 `None`。
5. `BOLL(20, 2)` 固定样本 golden test。
6. `BOLL(10, 1.5)` 和 `BOLL(50, 2.5)` 字段映射正确。
7. 当标准差为 0 时，`UP == MID == DN`。
8. 所有非空输出满足 `UP >= MID >= DN`。

### 阶段 3：`furnace-io` Bollinger Bands 运行路径

目标：

1. 新增 Bollinger Bands run request、summary、result row、DDL、staging 和 run 函数。
2. 复用 MA 的 close-only 输入读取、KDJ/MA 的 ClickHouse executor、RowBinary、timing、Rayon 并行和 partition replace 模式。
3. 支持 `dry-run`、`append-latest`、`replace-cascade`。

建议新增或复用结构：

```text
BollRunRequest
BollRunSummary
BollInputRow
BollResultRow
BollGroupedInput
run_boll
create_boll_output_table_sql
boll_staging_table_name
replace_boll_partition_sql
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

summary 字段：

```text
indicator = "boll"
request_from / request_to
effective_output_from / effective_output_to
input_from / input_to
mode
symbols_count
input_rows
output_rows
input_valid_close_rows
output_valid_close_rows
null_indicator_rows
affected_years
retained_rows
staging_table
staging_validation
partition_replace
boll_configs
max_window = 50
stddev_ddof = 0
state_source = "rolling-lookback"
run_id
writes_applied
performance_metrics
```

`boll_configs` JSON 结构固定为：

```json
[
  {"window": 10, "multiplier": 1.5, "field_suffix": "10_1p5"},
  {"window": 20, "multiplier": 2.0, "field_suffix": "20_2"},
  {"window": 50, "multiplier": 2.5, "field_suffix": "50_2p5"}
]
```

要求：

1. `boll_configs` 顺序必须与字段输出顺序一致。
2. `field_suffix` 必须与第 3.3 节字段命名口径一致。
3. `max_window` 从 canonical configs 推导，第一版固定为 `50`。
4. `stddev_ddof` 第一版固定为 `0`。

### 阶段 4：CLI 子命令

目标：

1. 在 `engines/crates/furnace/src/main.rs` 新增 `boll` 子命令。
2. 复用 KDJ/MA 的参数解析、错误输出和 JSON summary 模式。

CLI 测试：

1. `boll --mode dry-run --output-format json` 返回 JSON object。
2. 未知 mode 返回非 0。
3. 非 canonical price column 在写入模式下拒绝。
4. `--symbols` 解析与 KDJ/MA 保持一致。
5. 省略 `--symbols` 等价全市场；如果支持 `--symbols all`，必须测试其等价全市场。
6. 写入模式允许自定义 `--output-table` 到隔离表，但不允许改变 canonical input table 和 price column。
7. 不支持自定义 `(N, K)` 参数；误传未知参数时 CLI 返回非 0。

### 阶段 5：dbt 接入

目标：

1. 在 `sources_fleur_calculation.yml` 新增 `calc_stock_boll_daily` source。
2. 新增 `pipeline/elt/models/intermediate/int_stock_boll_daily.sql` thin wrapper。
3. 新增 `pipeline/elt/models/intermediate/int_stock_boll_daily.yml` 文档和 tests。

dbt wrapper 只 select 业务字段：

```text
security_code
trade_date
boll_mid_10_1p5
boll_up_10_1p5
boll_dn_10_1p5
boll_mid_20_2
boll_up_20_2
boll_dn_20_2
boll_mid_50_2p5
boll_up_50_2p5
boll_dn_50_2p5
```

dbt tests：

1. `security_code` not null + A 股代码格式。
2. `trade_date` not null。
3. `security_code + trade_date` 唯一。
4. 字段名遵循 `boll_mid_10_1p5` 风格，不出现 `boll_mid_n20_k2`、`boll_up_n20_k2`、`boll_dn_n20_k2` 旧风格。
5. 字段文档明确说明窗口不足、缺价和历史有效 close 不足时允许为 `NULL`。
6. 字段文档必须说明 `STD` 使用总体标准差，即 `ddof=0`。
7. dbt wrapper 不新增 `WIDTH`、`PERCENT_B`、`stddev_*` 等未由 Furnace 输出的衍生字段。

### 阶段 6：Dagster 接入

目标：

1. 扩展 `FurnaceCliResource` 支持 `run_boll`。
2. 新增 `FurnaceBollCliRequest` / result dataclass。
3. 新增 `calc_stock_boll_daily` asset。
4. 新增 `furnace__boll_daily_job`、`furnace__boll_backfill_job`、`furnace__boll_dry_run_job`。
5. 可选新增 Bollinger Bands daily schedule；如果暂不启用 schedule，必须在文档中说明由手动 job 或后续计划开启。

Dagster asset：

```text
AssetKey(["fleur_calculation", "calc_stock_boll_daily"])
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
input_valid_close_rows
output_valid_close_rows
null_indicator_rows
affected_years
retained_rows
boll_configs
max_window
stddev_ddof
state_source
staging_validation
partition_replace
performance_metrics
writes_applied
```

### 阶段 7：性能和全量验收报告

目标：

1. 使用 release binary 和 Rayon 执行全市场、全历史数据量并行计算。
2. 记录输入行数、证券数量、输出行数、空指标行数、耗时、吞吐、worker 数、内存/ClickHouse part 健康情况。
3. 产出运行报告：

```text
docs/jobs/reports/<date>-furnace-boll-full-market-parallel-validation.md
```

全量验收运行必须覆盖：

| 场景 | 命令模式 | 是否写表 | 目的 |
|------|----------|----------|------|
| 全市场全历史 dry-run | `dry-run` | 否 | 验证并行计算能跑完整数据量，观察性能和内存 |
| 全市场全历史 replace-cascade | `replace-cascade` | 是 | 验证 staging、RowBinary 写入和年度分区替换可承载完整数据量 |
| dbt wrapper build | `dbt build --select int_stock_boll_daily` | 读 Bollinger Bands 表 | 验证 source、wrapper 和 tests |
| Dagster dry-run asset | `dg launch` 或等价 job | 否 | 验证 Dagster resource、config 和 metadata |

全量运行日期范围必须从输入表实际最早交易日到最新交易日，不允许只挑样本区间代替。执行前用 ClickHouse 查询记录：

```sql
SELECT
    min(trade_date),
    max(trade_date),
    count() AS input_rows,
    countIf(close_price_forward_adj IS NOT NULL) AS input_valid_close_rows,
    uniqExact(security_code) AS symbols
FROM fleur_intermediate.int_stock_quotes_daily_adj
```

推荐 dry-run 命令模板：

```bash
cd engines
RAYON_NUM_THREADS=8 cargo run --release -p furnace -- boll \
  --from <min_trade_date> \
  --to <max_trade_date> \
  --mode dry-run \
  --insert-batch-size 10000 \
  --output-format json
```

写入验收命令模板：

```bash
cd engines
RAYON_NUM_THREADS=8 cargo run --release -p furnace -- boll \
  --from <min_trade_date> \
  --to <max_trade_date> \
  --mode replace-cascade \
  --run-id furnace_boll_full_market_<yyyymmdd> \
  --insert-batch-size 10000 \
  --output-format json
```

验收报告必须包含：

1. 命令、环境、git commit 或 worktree 标识。
2. 输入日期范围、输入行数、有效 close 行数、证券数。
3. summary JSON 的关键字段。
4. `performance_metrics` 完整内容。
5. `calc_stock_boll_daily` 行数和唯一键检查。
6. 年度分区替换结果和 part 数量检查。
7. 至少 3 只证券的 spot check：三组 Bollinger Bands 与固定样本或独立脚本结果一致；独立脚本必须使用总体标准差 `ddof=0`，不得使用 pandas 默认样本标准差 `ddof=1`。
8. 至少 1 只上市早期或有效 close 少于 50 条的证券/区间检查，证明窗口不足时输出为空且不会错误推进状态。
9. 至少 1 个含 `close_price_forward_adj IS NULL` 的区间检查，证明该行输出全空，且后一条有效 close 的窗口计数没有被空值推进。

## 8. 测试和质量门禁

### 8.1 Rust

实施完成后运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Rust 测试必须覆盖：

1. rolling standard deviation 公共算子单元测试。
2. Bollinger Bands 核心指标 golden tests。
3. `boll` CLI 参数解析测试。
4. `boll` dry-run summary 测试。
5. RowBinary input/output 编码测试。
6. staging SQL 和 partition replace SQL 测试。
7. 并行输出与串行输出一致性测试。
8. KDJ/MA 既有测试全部保持通过。

### 8.2 dbt

实施完成后运行：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_boll_daily
uv run python elt/scripts/validate_field_glossary.py
```

如新增字段文档触及 staging readiness 或 glossary 规则，按现有 dbt governance 脚本补齐描述后再验收。

### 8.3 Dagster / Python

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

1. `FurnaceCliResource.command_for_boll_request` 生成正确命令。
2. Bollinger Bands summary metadata 映射正确。
3. Bollinger Bands asset key、upstream dependency、group、tags 和 owners 符合 KDJ/MA 模式。

## 9. 验收条件

实施完成必须同时满足以下条件：

1. `furnace-core` 存在公共 rolling standard deviation 算子，Bollinger Bands 指标计算复用 `RollingSma` 和 `RollingStdDev`。
2. `furnace-core` 不依赖 ClickHouse、Dagster、dbt、Rayon、CLI 参数或环境变量。
3. `furnace boll` 支持 `dry-run`、`append-latest`、`replace-cascade`。
4. `furnace boll` summary 包含 `indicator="boll"`、`boll_configs`、`max_window=50`、`stddev_ddof=0`、`state_source="rolling-lookback"` 和 `performance_metrics`。
5. `calc_stock_boll_daily` 可以被自动创建，并使用 RowBinary 批量写入。
6. `replace-cascade` 使用 staging + 年度分区替换，且 staging validation 无重复 `(security_code, trade_date)`。
7. `int_stock_boll_daily` dbt wrapper 只 select Furnace 输出，不重写公式。
8. Dagster 能物化 `fleur_calculation/calc_stock_boll_daily` asset，并记录 summary metadata。
9. Rust、dbt、Dagster/Python 质量门禁全部通过。
10. 全市场、全历史 dry-run 并行计算成功完成，`performance_metrics.parallelism = "rayon"`，`worker_threads >= 2`。
11. 全市场、全历史 replace-cascade 写入验收成功完成；如果目标环境不允许写生产表，必须在同等数据量的隔离 ClickHouse database 中完成，并在报告中说明 database、表名和隔离方式。
12. 全量验收的 `symbols_count` 必须与输入表证券数对齐；不允许用显式少量证券列表代替全市场。
13. 全量验收后，`calc_stock_boll_daily` 或隔离输出表满足每证券、交易日唯一，输出行数与请求日期范围内输入行情行数的预期一致。
14. 固定样本、spot check 和全量运行未发现 `boll_mid_n20_k2` 等旧风格字段名。
15. 所有非空布林带输出满足 `UP >= MID >= DN`。
16. summary、dbt 文档和验收报告均明确记录 `stddev_ddof = 0`。
17. lookback 行未写入目标表，`output_rows` 与输出区间内行情行数口径一致。
18. 生成 `docs/jobs/reports/<date>-furnace-boll-full-market-parallel-validation.md`，报告包含命令、summary、性能、行数、唯一性、分区替换和 spot check 结果。

## 10. 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 标准差口径与外部工具不一致 | spot check 出现差异 | 本计划固定总体标准差并在 dbt 文档说明；如需样本标准差另立迁移计划 |
| 浮点方差出现极小负数 | `sqrt` 产生 NaN | 对接近 0 的负方差 clamp 到 0，并用等值序列测试覆盖 |
| spot check 使用 pandas 默认 `std()` | 验收误报 | 验收脚本必须显式使用 `ddof=0` |
| lookback 行被误写入 | 输出行数膨胀并污染历史分区 | summary 区分 input/output 口径，测试覆盖 request 前 lookback 不写出 |
| 复制 KDJ/MA I/O 代码过多 | 后续维护三套逻辑 | 优先抽取 staging、timing、JSON helper、RowBinary helper 的稳定重复片段 |
| 全量 replace-cascade 写入耗时过长 | 验收阻塞或 ClickHouse part 压力过大 | 先 dry-run 量级评估；写入使用 release binary、RowBinary、合理 batch 和 part 健康检查 |
| Nullable 指标字段多 | 下游误解为空含义 | dbt YAML 明确说明窗口不足、缺价和历史有效 close 不足时为空 |
| 历史修正实际影响范围有限但被全量 cascade | 写入成本偏高 | 第一版保持正确性优先；后续可按最大窗口另立局部 cascade 优化 |

## 11. 推荐实施顺序

1. 完成 `furnace-core::operators::stddev` 和测试。
2. 完成 `furnace-core::indicators::bollinger_bands` 和 golden tests。
3. 在 `furnace-io` 复用 MA close-only 输入路径实现 `run_boll` dry-run。
4. 扩展 `furnace` CLI，先打通 dry-run JSON summary。
5. 实现 Bollinger Bands output DDL、RowBinary 写入、append-latest 和 replace-cascade。
6. 增加并行一致性测试和 performance metrics。
7. 增加 dbt source/wrapper/tests。
8. 增加 Dagster resource、asset、jobs 和 metadata。
9. 跑 Rust/dbt/Dagster 质量门禁。
10. 跑全市场全历史 dry-run 并行验收。
11. 跑全市场全历史 replace-cascade 写入验收。
12. 编写全量验收报告，并根据报告修复遗留问题。
