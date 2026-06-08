# Plan 0029: Furnace Moving Average 日线技术指标实施方案

日期：2026-06-08

状态：Draft

关联文档：

- `docs/RFC/0017-furnace-moving-average-technical-indicators.md`
- `docs/RFC/0016-rust-furnace-compute-engine.md`
- `docs/plans/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- `docs/plans/0028-furnace-kdj-parallel-performance-implementation-plan.md`
- `engines/README.md`
- `engines/crates/furnace-core/src/indicators/kdj.rs`
- `engines/crates/furnace-io/src/lib.rs`
- `engines/crates/furnace/src/main.rs`
- `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
- `pipeline/scheduler/src/scheduler/defs/resources/furnace.py`
- `pipeline/elt/models/sources_fleur_calculation.yml`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.yml`
- `pipeline/elt/models/intermediate/int_stock_kdj_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_kdj_daily.yml`

相关 skills：

- `rust-best-practices` / `rust-patterns` / `rust-testing`：Rust crate 边界、公共算子抽象、错误模型、测试和性能。
- `running-dbt-commands` / `using-dbt-for-analytics-engineering`：dbt source、thin wrapper、字段文档和定向 build。
- `dagster-expert`：Dagster asset、job、schedule、资源和 materialization metadata。
- `clickhouse-best-practices` / `clickhouse-architecture-advisor`：ClickHouse 宽表、RowBinary 批量写入、年度分区替换和全量回填验收。

## 1. 目标

基于 RFC 0017，在现有 Furnace Rust workspace 中新增日频 Moving Average 指标计算能力，从 `fleur_intermediate.int_stock_quotes_daily_adj` 读取前复权收盘价，计算 MA、组合均线和双重 EMA，并写入：

```text
fleur_calculation.calc_stock_ma_daily
```

再由 dbt thin wrapper 暴露为：

```text
fleur_intermediate.int_stock_ma_daily
```

完成后应满足：

1. `furnace ma` 支持按日期区间、证券集合和运行模式计算 Moving Average 指标。
2. 所有指标使用 `close_price_forward_adj` 作为 `close` 输入，不混用其他价格口径。
3. 第一版输出 RFC 0017 中列出的 canonical 字段：`ma_3`、`ma_5`、`ma_6`、`ma_10`、`ma_12`、`ma_14`、`ma_20`、`ma_24`、`ma_28`、`ma_57`、`ma_60`、`ma_114`、`ma_250`、`avg_ma_3_6_12_24`、`avg_ma_14_28_57_114`、`ema2_10`。
4. MA/EMA 基础算子抽取到 `furnace-core` 公共算子层，Moving Average 指标模块只负责组合算子并映射业务字段。
5. Rust、ClickHouse、Dagster 和 dbt 的 ownership 边界沿用 KDJ：公式在 `furnace-core`，I/O 和并行调度在 `furnace-io`，CLI 在 `furnace`，调度与 metadata 在 Dagster，消费契约在 dbt。
6. 生产写入使用 staging + 年度 `REPLACE PARTITION` 协议，保持幂等并避免高频 mutation。
7. 计算按证券维度并行；单证券内部按 `trade_date` 串行递推。
8. 验收必须包含全市场、全历史数据量的并行计算运行，并记录性能和结果质量报告。

## 2. 非目标

本计划不做以下事情：

1. 不实现 MACD、RSI、布林线或其他未列入 RFC 0017 的指标。
2. 不改变 `int_stock_quotes_daily_adj` 的复权逻辑、字段语义或物化策略。
3. 不在 dbt SQL 或 Dagster Python asset 中重写 MA/EMA 公式。
4. 不让 Furnace 直接写入 `fleur_intermediate.int_stock_ma_daily`。
5. 不把同一证券时间序列按日期并行；EMA 递推状态不允许这样做。
6. 不在第一版支持多价格口径、多参数集合或长表 `indicator_name/value` 结构。
7. 不强制同步重构已实现的 KDJ 模块；KDJ 当前只抽到 `furnace-core::indicators::kdj`，本计划只要求 MA/EMA 新实现使用公共算子层。后续可另立计划将 KDJ 的 K/D 平滑迁移到公共算子。

## 3. 当前事实基线

### 3.1 Rust workspace

当前 Rust workspace 已有三层 crate：

```text
engines/
├── crates/furnace/       # CLI binary，当前已有 furnace kdj
├── crates/furnace-core/  # 纯指标计算，当前已有 KDJ
└── crates/furnace-io/    # ClickHouse I/O、RowBinary、Rayon、staging/replace
```

现有 KDJ 实现事实：

1. `furnace-core/src/indicators/kdj.rs` 提供 `calculate_kdj_series`、`calculate_kdj_next`、`KdjInput`、`KdjOutput`、`KdjState`。
2. KDJ 已经从 CLI/IO 层抽到 `furnace-core`，但 RSV 滚动窗口和 K/D 递推平滑仍是 KDJ 模块私有实现。
3. 当前没有 `furnace-core/src/operators/` 公共算子目录。
4. `furnace-io` 已有 ClickHouse executor 抽象、RowBinary 读取/写入、证券维度 Rayon 并行、staging 表、年度分区替换和 `PerformanceMetrics`。
5. `furnace` CLI 当前只支持 `kdj` 子命令，参数解析和 JSON summary 输出可作为 `ma` 子命令模板。

### 3.2 输入模型

Moving Average 输入默认来自：

```text
fleur_intermediate.int_stock_quotes_daily_adj
```

相关 dbt 文件：

```text
pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql
pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.yml
```

第一版只读取：

| 字段 | 用途 |
|------|------|
| `security_code` | 证券代码 |
| `trade_date` | 交易日 |
| `close_price_forward_adj` | MA/EMA 的 canonical `close` 输入 |

输入必须按以下顺序提供给单证券核心计算：

```text
security_code ASC, trade_date ASC
```

输入和输出行口径：

1. 输出 grain 与输入行情 grain 对齐，为每证券、交易日一行；请求输出区间内即使 `close_price_forward_adj IS NULL` 也要输出一行，所有指标字段为 `NULL`。
2. `input_rows` 统计实际读取的行情行数，不只统计有效 close 行。
3. `output_rows` 统计实际落入 `effective_output_from..effective_output_to` 的行情行数。
4. `valid_close_rows` 可作为运行报告辅助指标，但不是输出行数口径。
5. 同一证券内 `trade_date` 必须严格递增；如果输入中出现重复日期或乱序，核心计算返回错误，不在 Rust 中静默去重。
6. `null_indicator_rows` 统计所有业务指标字段都为 `NULL` 的输出行；只要任一 `ma_*`、组合均线或 `ema2_10` 非空，该行不计入 `null_indicator_rows`。

### 3.3 dbt 和 Dagster 现状

可复用的 dbt 模式：

1. `pipeline/elt/models/sources_fleur_calculation.yml` 已声明 `calc_stock_kdj_daily` source。
2. `pipeline/elt/models/intermediate/int_stock_kdj_daily.sql` 是 thin wrapper，不重写公式。
3. `pipeline/elt/models/intermediate/int_stock_kdj_daily.yml` 提供 wrapper 文档、唯一性测试和字段测试。

可复用的 Dagster 模式：

1. `pipeline/scheduler/src/scheduler/defs/furnace/assets.py` 已实现 `calc_stock_kdj_daily` asset。
2. `pipeline/scheduler/src/scheduler/defs/resources/furnace.py` 已实现 `FurnaceCliResource`、请求 dataclass、命令构造、stdout JSON 解析和 `RAYON_NUM_THREADS` 注入。
3. `pipeline/scheduler/src/scheduler/defs/furnace/definitions.py` 已定义 KDJ jobs 和 schedule。

## 4. 复用原则

本实施必须优先复用已有代码资源，避免为 MA 复制出第二套不兼容基础设施。

### 4.1 必须复用

| 现有资源 | 复用方式 |
|----------|----------|
| `ClickHouseExecutor` | MA 运行路径继续通过同一 executor 抽象执行 SQL 和 RowBinary 写入 |
| RowBinary 输入解析 | 复用现有 RowBinary 读取框架，针对 MA 增加只含 close 的输入行解析 |
| RowBinary 输出写入 | 复用 `insert_result_rows` 思路，新增 MA result row 编码 |
| staging + `REPLACE PARTITION` | 复用 KDJ 的 staging 建表、旧行保留、校验、年度分区替换和清理流程 |
| `PerformanceMetrics` | MA summary 使用同类阶段计时字段，便于和 KDJ 性能横向比较 |
| Rayon 证券维度并行 | 复用 KDJ 的 per-security worker 模式 |
| CLI 参数解析 | 在 `furnace/src/main.rs` 中按 KDJ 模式新增 `ma` 子命令 |
| Dagster `FurnaceCliResource` | 扩展为支持 `run_ma`，不新增重复资源类 |
| dbt source/wrapper 结构 | 在现有 source YAML 中新增 MA 表，新增 thin wrapper model |

### 4.2 可以抽取共用的代码

实施中如果发现 KDJ 和 MA 的 I/O 代码高度重复，应优先抽取小范围 helper，而不是复制粘贴整段逻辑：

1. staging 表名规范化。
2. 年度分区替换 SQL 构造。
3. `affected_years`、`retained_rows`、staging validation。
4. `json_optional_string`、JSON array/number helpers。
5. `time_result` 和 `RunTimings`。
6. symbol filter SQL 构造和输入区间解析。

抽取原则：

- 只抽真实重复的稳定边界。
- 不为了追求泛型化引入复杂 trait 层。
- 不改变 KDJ 对外行为；抽取后 KDJ 现有测试必须全部通过。

## 5. 架构决策

### 5.1 公共算子层

新增模块建议：

```text
engines/crates/furnace-core/src/operators/mod.rs
engines/crates/furnace-core/src/operators/sma.rs
engines/crates/furnace-core/src/operators/ema.rs
```

第一版公共 API 建议包含：

```text
RollingSma
SmaSeededEma
EmaState
calculate_sma_series(values, window)
calculate_sma_seeded_ema_series(values, window, previous_state)
```

算子语义：

1. `None` 输入不进入有效窗口，不推进 EMA 状态。
2. SMA 窗口按有效值计数，不按自然交易日行数计数。
3. rolling SMA 在有效值数量少于窗口时输出 `None`。
4. EMA 使用前 `n` 个有效值的 SMA 启动；启动前输出 `None`。
5. EMA 一旦启动，后续有效值按 `alpha * current + (1 - alpha) * previous` 递推。
6. EMA 状态对象只保存足以延续递推的最小状态，例如上一条有效 EMA 值、有效输入计数和启动窗口累积信息。

公共算子错误模型：

1. window 必须大于 0。
2. 输入值必须是有限 `f64`；非有限值作为错误处理，不悄悄进入计算。
3. 空值语义使用 `Option<f64>`，不要用 `NaN` 表示缺失。
4. 算子不做四舍五入；所有结果以 `Float64` 递推和输出，测试断言使用 `1e-9` 级别容差。

### 5.2 Moving Average 指标模块

新增模块建议：

```text
engines/crates/furnace-core/src/indicators/moving_average.rs
```

核心类型建议：

```text
MaInput {
  trade_date,
  close_price
}

MaOutput {
  trade_date,
  ma_3,
  ...
  avg_ma_3_6_12_24,
  avg_ma_14_28_57_114,
  ema1_10_state,
  ema2_10
}

MaParams {
  ma_windows,
  ema_window
}

MaState {
  ema1_10,
  ema2_10
}
```

实现要求：

1. 单证券 API 输入必须按 `trade_date` 严格升序。
2. MA、EMA 都只消费有效 `close_price_forward_adj`。
3. 当前行 close 为空时，所有指标输出 `None`，且 EMA 状态不推进。
4. `ema2_10` 使用 `ema1_10` 的非空序列作为输入。
5. 组合均线只在组成 MA 全部非空时输出。
6. 输出字段必须使用 `ma_57` 和 `avg_ma_14_28_57_114`，不得出现 `47` 字段。
7. `avg_ma_3_6_12_24` 和 `avg_ma_14_28_57_114` 不做额外 rounding；任一组成 MA 为 `None` 时组合均线为 `None`。
8. `ema2_10` 首个非空值应出现在同一证券第 19 个有效 close 对应的交易日；第 19 个有效 close 之前的 `ema2_10` 必须为 `None`。

### 5.3 EMA 状态方案

第一版采用“同表内部状态列”方案，降低日常增量成本并避免 warm-up 截断误差。

物理表可包含 dbt wrapper 不暴露的内部字段：

```text
ema1_10_state Nullable(Float64)
ema2_10_state Nullable(Float64)
```

说明：

1. `ema1_10_state` 用于后续增量继续计算 `EMA(close, 10)`。
2. `ema2_10_state` 用于后续增量继续计算 `EMA(EMA(close, 10), 10)`。
3. `ema2_10_state` 在数值上等于该行业务字段 `ema2_10`，但语义上是 calculation 层内部状态；保留独立字段是为了让后续若状态扩展时不改变业务字段契约。
4. dbt `int_stock_ma_daily` 默认只暴露业务字段 `ema2_10`，不暴露内部状态。
5. 日常增量读取目标区间前最近一条两项状态都可用的记录作为 previous state。
6. 历史回填和复权修正使用 `replace-cascade`，从请求起点级联到受影响证券最新输入交易日。
7. 对于新证券或历史早期区间，如果目标区间前不存在完整 `ema1_10_state` / `ema2_10_state`，不能只用空状态从请求日期启动；必须回读该证券足够早的历史输入，从首个有效 close 或可证明等价的起点重新推导 SMA 启动窗口。
8. 如果实施选择只保存 `ema1_10_state` / `ema2_10_state` 两个值，则 partial-start 阶段的增量必须走历史推导路径；不得用不完整状态近似。

状态列输出语义：

1. 当前行 `close_price_forward_adj IS NULL` 时，`ema1_10_state`、`ema2_10_state` 和 `ema2_10` 均输出 `NULL`，且内存中的上一条 EMA 状态不推进。
2. `EMA(close, 10)` 未启动前，`ema1_10_state` 为 `NULL`。
3. `EMA(close, 10)` 启动后，`ema1_10_state` 输出当前行最新 `ema1_10`。
4. `EMA(EMA(close, 10), 10)` 未启动前，`ema2_10_state` 和业务字段 `ema2_10` 均为 `NULL`。
5. `EMA(EMA(close, 10), 10)` 启动后，`ema2_10_state` 和业务字段 `ema2_10` 输出同一数值。
6. 读取 previous state 时，只能使用目标区间前最近一条 `ema1_10_state IS NOT NULL AND ema2_10_state IS NOT NULL` 的记录；不能从 close 为空行或 partial-start 行读取完整状态。

如果实施中发现同表状态列会明显增加 wrapper 或替换复杂度，可以改为单独状态表；但必须在代码实施前更新 RFC/plan，并保留无截断误差证明。

### 5.4 运行区间和 lookback 口径

MA 和 EMA 的输入读取起点必须按证券实际有效 close 推导，不按自然日简单回退。

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

1. MA 需要请求区间前至少 249 个有效 close，以支持 `MA(close, 250)`。
2. 如果存在完整 `ema1_10_state` 和 `ema2_10_state`，且该状态交易日在 `request_from` 之前，则 EMA 可以从 previous state 延续；`input_from` 仍必须满足 MA 的 249 个有效 close lookback。
3. 如果不存在完整 EMA previous state，或 previous state 在目标证券上不可用，则该证券必须从首个有效 close 或可证明等价的历史起点重新推导 EMA。
4. 多证券运行时，可以先解析证券集合，再按证券确定各自需要的 `input_from`；第一版也可以取所有受影响证券中最早的 `input_from` 作为一次性读取起点，以复用现有 KDJ 批量读取路径。
5. summary 中的 `input_from` 记录本次实际读取的最早日期；如果各证券 lookback 不同，记录全局最早读取日期，并在 `ema_state_source` 或报告中说明状态来源。

状态来源枚举：

| 值 | 含义 |
|----|------|
| `previous-state` | 所有受影响证券都从目标区间前完整 EMA 状态延续 |
| `full-history` | 所有受影响证券都从首个有效 close 或全量历史重新推导 |
| `mixed` | 一部分证券用 previous state，一部分证券回读历史推导 |

`replace-cascade` 级联规则：

1. 历史 close 修正会影响后续所有 MA 窗口内结果和所有后续 EMA 结果。
2. 生产写入的 `replace-cascade` 必须将 `effective_output_to` 扩展到受影响证券的最新输入交易日。
3. staging 保留旧行时，只保留未受影响证券，或受影响证券中不在 `effective_output_from..effective_output_to` 的旧行。
4. staging validation 必须按 `(security_code, trade_date)` 检查重复 key，且覆盖所有受影响年度分区。

### 5.5 ClickHouse 表和写入

沿用 KDJ calculation 层宽表模式：

```text
fleur_calculation.calc_stock_ma_daily
```

建议字段：

```text
security_code String
trade_date Date
ma_3 Nullable(Float64)
ma_5 Nullable(Float64)
ma_6 Nullable(Float64)
ma_10 Nullable(Float64)
ma_12 Nullable(Float64)
ma_14 Nullable(Float64)
ma_20 Nullable(Float64)
ma_24 Nullable(Float64)
ma_28 Nullable(Float64)
ma_57 Nullable(Float64)
ma_60 Nullable(Float64)
ma_114 Nullable(Float64)
ma_250 Nullable(Float64)
avg_ma_3_6_12_24 Nullable(Float64)
avg_ma_14_28_57_114 Nullable(Float64)
ema1_10_state Nullable(Float64)
ema2_10 Nullable(Float64)
ema2_10_state Nullable(Float64)
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
4. INSERT 使用 RowBinary 和批量写入，默认 `insert_batch_size = 10_000`，可沿用 KDJ 参数限制。
5. `calc_stock_ma_daily` 不写 `run_id` 或 `computed_at`；运行审计只进入 Dagster materialization metadata 和报告。
6. DDL 建表必须由 `furnace-io` 负责，dbt 不负责创建 calculation 物理表。
7. RowBinary 输出字段顺序必须与 DDL 完全一致，并有单元测试覆盖 nullable marker 和 Float64 little-endian 编码。
8. 所有 MA SQL helper 必须接受 `output_table` 参数；不得把 `fleur_calculation.calc_stock_ma_daily` 硬编码到 staging、insert、retain、validate 或 partition replace 路径中，否则隔离库全量验收不可执行。

### 5.6 CLI 和 symbols 口径

沿用 KDJ 的符号语义：

1. `--symbols` 省略或传空集合表示全市场。
2. 不要求支持字面值 `--symbols all`；如果实现选择支持，必须把 `all` 明确解析为全市场，而不是证券代码。
3. 多证券显式传参使用逗号分隔代码，例如 `--symbols 000001.SZ,600000.SH`。
4. 生产写入模式下，如果解析后的证券集合为空且输入表也没有任何证券，必须拒绝写入。

### 5.7 并行计算

并行粒度沿用 KDJ：

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

## 6. 实施阶段

### 阶段 1：公共算子库

目标：

1. 新增 `furnace-core::operators` 模块。
2. 实现 rolling SMA 和 SMA-seeded EMA。
3. 为算子补独立单元测试。

实现要点：

- 使用 `VecDeque` 和 rolling sum 实现 SMA，避免每行重新扫描窗口。
- EMA 启动阶段维护前 `n` 个有效值的 sum/count。
- 输入 `None` 时返回 `None`，且不改变内部状态。
- 参数校验错误使用清晰的 error enum，不 panic。

测试覆盖：

1. window 为 0 返回错误。
2. 有效值不足窗口时 SMA/EMA 输出 `None`。
3. rolling SMA 正常滚动。
4. `None` 输入不推进窗口和状态。
5. 10 日 EMA 示例：前 10 个有效 close 总和 559，初始 EMA 55.9；第 11 个有效 close 为 60 时 EMA 约 56.645454。
6. previous state 延续计算与全历史一次性计算结果一致。

### 阶段 2：Moving Average 核心指标

目标：

1. 新增 `furnace-core::indicators::moving_average`。
2. 提供单证券纯计算 API。
3. 使用公共算子组合出所有 RFC 字段。

实现要点：

- 固定 canonical MA 窗口集合：`[3, 5, 6, 10, 12, 14, 20, 24, 28, 57, 60, 114, 250]`。
- `avg_ma_3_6_12_24` 和 `avg_ma_14_28_57_114` 用输出 MA 字段组合，不重复计算窗口。
- `ema1_10` 作为内部状态参与计算，业务输出只保留 `ema2_10`。
- 输入日期必须严格递增，沿用 KDJ 的校验风格。

测试覆盖：

1. 空输入返回空输出。
2. 非递增 trade_date 返回错误。
3. close 为空行所有指标为 `None`。
4. MA 窗口不足输出 `None`。
5. 多窗口 MA 固定样本 golden test。
6. 组合均线任一组成 MA 为空时输出 `None`。
7. `ema2_10` 首个非空值出现在第 19 个有效 close。
8. 历史状态启动与全历史推导一致。

### 阶段 3：`furnace-io` MA 运行路径

目标：

1. 新增 MA run request、summary、result row、DDL、staging 和 run 函数。
2. 复用 KDJ 的 ClickHouse executor、RowBinary、timing、Rayon 并行和 partition replace 模式。
3. 支持 `dry-run`、`append-latest`、`replace-cascade`。

建议新增或复用结构：

```text
MaRunRequest
MaRunSummary
MaWriteMode 或复用通用 WriteMode
MaInputRow
MaResultRow
MaGroupedInput
run_ma
create_ma_output_table_sql
ma_staging_table_name
replace_ma_partition_sql
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

状态读取：

1. `append-latest` 读取目标区间前最近有效 `ema1_10_state`、`ema2_10_state`。
2. `replace-cascade` 根据请求区间和受影响证券级联到最新输入日期；状态来源记录为 `previous-state`、`full-history` 或 `mixed`。
3. 如果没有可用完整历史状态，或证券仍处于 EMA SMA 启动窗口阶段，从该证券首个有效 close 或足够早的历史输入重新推导启动。

summary 字段：

```text
indicator = "ma"
request_from / request_to
effective_output_from / effective_output_to
input_from / input_to
mode
symbols_count
input_rows
output_rows
null_indicator_rows
affected_years
retained_rows
staging_table
staging_validation
partition_replace
ma_windows
ema_state_source
run_id
writes_applied
performance_metrics
```

### 阶段 4：CLI 子命令

目标：

1. 在 `engines/crates/furnace/src/main.rs` 新增 `ma` 子命令。
2. 复用 KDJ 的参数解析、错误输出和 JSON summary 模式。

CLI 形态：

```bash
cargo run --release -p furnace -- ma \
  --from 2026-01-01 \
  --to 2026-01-31 \
  --mode dry-run \
  --input-table fleur_intermediate.int_stock_quotes_daily_adj \
  --output-table fleur_calculation.calc_stock_ma_daily \
  --price-column close_price_forward_adj \
  --insert-batch-size 10000 \
  --output-format json
```

第一版可以只实现 RFC 中固定 `input-table`、`output-table` 和 `price-column` 默认值；如果提供参数，必须校验不偏离 canonical 口径，或在 summary 中明确记录。

CLI 参数口径：

1. `--input-table` 默认 `fleur_intermediate.int_stock_quotes_daily_adj`。
2. `--output-table` 默认 `fleur_calculation.calc_stock_ma_daily`；全量验收如果使用隔离 database，必须允许通过该参数指向隔离表。
3. `--price-column` 默认且生产只允许 `close_price_forward_adj`。
4. 写入模式下，如果 `--input-table` 或 `--price-column` 偏离 canonical 口径，必须拒绝；隔离验收只允许改变 `--output-table`。

CLI 测试：

1. `ma --mode dry-run --output-format json` 返回 JSON object。
2. 未知 mode 返回非 0。
3. 非 canonical price column 在写入模式下拒绝。
4. `--symbols` 解析与 KDJ 保持一致。
5. 省略 `--symbols` 等价全市场；如果支持 `--symbols all`，必须测试其等价全市场。
6. 写入模式允许自定义 `--output-table` 到隔离表，但不允许改变 canonical input table 和 price column。

### 阶段 5：dbt 接入

目标：

1. 在 `sources_fleur_calculation.yml` 新增 `calc_stock_ma_daily` source。
2. 新增 `pipeline/elt/models/intermediate/int_stock_ma_daily.sql` thin wrapper。
3. 新增 `pipeline/elt/models/intermediate/int_stock_ma_daily.yml` 文档和 tests。

dbt wrapper 只 select 业务字段：

```text
security_code
trade_date
ma_3
...
avg_ma_14_28_57_114
ema2_10
```

不暴露：

```text
ema1_10_state
ema2_10_state
```

dbt tests：

1. `security_code` not null + A 股代码格式。
2. `trade_date` not null。
3. `security_code + trade_date` 唯一。
4. 字段名中不出现 `ma_47` 或 `avg_ma_14_28_47_114`。

### 阶段 6：Dagster 接入

目标：

1. 扩展 `FurnaceCliResource` 支持 `run_ma`。
2. 新增 `FurnaceMaCliRequest` / result dataclass。
3. 新增 `calc_stock_ma_daily` asset。
4. 新增 `furnace__ma_daily_job`、`furnace__ma_backfill_job`、`furnace__ma_dry_run_job`。
5. 可选新增 MA daily schedule；如果暂不启用 schedule，必须在文档中说明由手动 job 或后续计划开启。

Dagster asset：

```text
AssetKey(["fleur_calculation", "calc_stock_ma_daily"])
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
null_indicator_rows
affected_years
retained_rows
ma_windows
ema_state_source
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
docs/jobs/reports/<date>-furnace-ma-full-market-parallel-validation.md
```

全量验收运行必须覆盖：

| 场景 | 命令模式 | 是否写表 | 目的 |
|------|----------|----------|------|
| 全市场全历史 dry-run | `dry-run` | 否 | 验证并行计算能跑完整数据量，观察性能和内存 |
| 全市场全历史 replace-cascade | `replace-cascade` | 是 | 验证 staging、RowBinary 写入和年度分区替换可承载完整数据量 |
| dbt wrapper build | `dbt build --select int_stock_ma_daily` | 读 MA 表 | 验证 source、wrapper 和 tests |
| Dagster dry-run asset | `dg launch` 或等价 job | 否 | 验证 Dagster resource、config 和 metadata |

全量运行日期范围必须从输入表实际最早交易日到最新交易日，不允许只挑样本区间代替。执行前用 ClickHouse 查询记录：

```sql
SELECT
    min(trade_date),
    max(trade_date),
    count() AS input_rows,
    countIf(close_price_forward_adj IS NOT NULL) AS valid_close_rows,
    uniqExact(security_code) AS symbols
FROM fleur_intermediate.int_stock_quotes_daily_adj
```

全市场口径：

1. 全量 dry-run 和 replace-cascade 命令默认省略 `--symbols`，表示全市场。
2. 验收报告必须记录 summary 中的 `symbols_count`，并与输入表 `uniqExact(security_code)` 对齐；如果有停牌或无有效 close 的证券差异，必须说明。
3. 不允许用少量证券列表替代全市场验收。

推荐命令模板：

```bash
cd engines
RAYON_NUM_THREADS=8 cargo run --release -p furnace -- ma \
  --from <min_trade_date> \
  --to <max_trade_date> \
  --mode dry-run \
  --insert-batch-size 10000 \
  --output-format json
```

写入验收命令模板：

```bash
cd engines
RAYON_NUM_THREADS=8 cargo run --release -p furnace -- ma \
  --from <min_trade_date> \
  --to <max_trade_date> \
  --mode replace-cascade \
  --run-id furnace_ma_full_market_<yyyymmdd> \
  --insert-batch-size 10000 \
  --output-format json
```

验收报告必须包含：

1. 命令、环境、git commit 或 worktree 标识。
2. 输入日期范围、输入行数、证券数。
3. summary JSON 的关键字段。
4. `performance_metrics` 完整内容。
5. `calc_stock_ma_daily` 行数和唯一键检查。
6. 年度分区替换结果和 part 数量检查。
7. 至少 3 只证券的 spot check：MA、组合均线、`ema2_10` 与固定样本或独立脚本结果一致。
8. 至少 1 只上市早期或有效 close 少于 19 条的证券/区间检查，证明 partial-start 增量不会错误推进 EMA 状态。

## 7. 测试和质量门禁

### 7.1 Rust

实施完成后运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Rust 测试必须覆盖：

1. 公共算子单元测试。
2. Moving Average 核心指标 golden tests。
3. MA CLI 参数解析测试。
4. MA dry-run summary 测试。
5. RowBinary input/output 编码测试。
6. staging SQL 和 partition replace SQL 测试。
7. 并行输出与串行输出一致性测试。
8. KDJ 既有测试全部保持通过。

### 7.2 dbt

实施完成后运行：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_ma_daily
uv run python elt/scripts/validate_field_glossary.py
```

如新增字段文档触及 staging readiness 或 glossary 规则，按现有 dbt governance 脚本补齐描述后再验收。

### 7.3 Dagster / Python

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

1. `FurnaceCliResource.command_for_ma_request` 生成正确命令。
2. MA summary metadata 映射正确。
3. MA asset key、upstream dependency、group、tags 和 owners 符合 KDJ 模式。

## 8. 验收条件

实施完成必须同时满足以下条件：

1. `docs/RFC/0017-furnace-moving-average-technical-indicators.md` 的字段、公式、价格口径和公共算子边界均被实现。
2. `furnace-core` 存在公共 `operators` 模块，MA/EMA 指标计算复用该模块。
3. `furnace-core` 不依赖 ClickHouse、Dagster、dbt、Rayon、CLI 参数或环境变量。
4. `furnace ma` 支持 `dry-run`、`append-latest`、`replace-cascade`。
5. `furnace ma` summary 包含 `indicator="ma"`、`ma_windows`、`ema_state_source` 和 `performance_metrics`。
6. `calc_stock_ma_daily` 可以被自动创建，并使用 RowBinary 批量写入。
7. `replace-cascade` 使用 staging + 年度分区替换，且 staging validation 无重复 `(security_code, trade_date)`。
8. `int_stock_ma_daily` dbt wrapper 只 select Furnace 输出，不重写公式。
9. Dagster 能物化 `fleur_calculation/calc_stock_ma_daily` asset，并记录 summary metadata。
10. Rust、dbt、Dagster/Python 质量门禁全部通过。
11. 全市场、全历史 dry-run 并行计算成功完成，`performance_metrics.parallelism = "rayon"`，`worker_threads >= 2`。
12. 全市场、全历史 replace-cascade 写入验收成功完成；如果目标环境不允许写生产表，必须在同等数据量的隔离 ClickHouse database 中完成，并在报告中说明 database、表名和隔离方式。
13. 全量验收的 `symbols_count` 必须与输入表证券数对齐；不允许用显式少量证券列表代替全市场。
14. 全量验收后，`calc_stock_ma_daily` 或隔离输出表满足每证券、交易日唯一，输出行数与请求日期范围内输入行情行数的预期一致；缺少有效 close 的行允许指标为空，但不能破坏唯一键。
15. 固定样本、spot check 和全量运行未发现 `ma_47` 或 `avg_ma_14_28_47_114` 字段。
16. 生成 `docs/jobs/reports/<date>-furnace-ma-full-market-parallel-validation.md`，报告包含命令、summary、性能、行数、唯一性、分区替换和 spot check 结果。

## 9. 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| EMA 状态列设计不足 | 增量结果与全历史不一致 | 核心测试必须覆盖 previous state 与全历史一致；全量验收做 spot check |
| 复制 KDJ I/O 代码过多 | 后续维护两套逻辑 | 优先抽取 staging、timing、JSON helper、RowBinary helper 的稳定重复片段 |
| 全量 replace-cascade 写入耗时过长 | 验收阻塞或 ClickHouse part 压力过大 | 先 dry-run 量级评估；写入使用 release binary、RowBinary、合理 batch 和 part 健康检查 |
| Nullable 指标字段过多 | 下游误解为空含义 | dbt YAML 明确说明窗口不足、缺价和状态未启动时为空 |
| 上游日期过滤无法充分利用排序键 | 全量读取慢 | 验收报告记录输入 SQL、读取耗时和吞吐；必要时另立输入读取优化计划 |
| 公共算子抽象过度 | 实施复杂、影响 KDJ | 第一版只抽 SMA/EMA 必需 API，不同步重构 KDJ |

## 10. 推荐实施顺序

1. 完成 `furnace-core::operators` 和测试。
2. 完成 `furnace-core::indicators::moving_average` 和 golden tests。
3. 在 `furnace-io` 复用 KDJ 路径实现 `run_ma` dry-run。
4. 扩展 `furnace` CLI，先打通 dry-run JSON summary。
5. 实现 MA output DDL、RowBinary 写入、append-latest 和 replace-cascade。
6. 增加并行一致性测试和性能 metrics。
7. 增加 dbt source/wrapper/tests。
8. 增加 Dagster resource、asset、jobs 和 metadata。
9. 跑 Rust/dbt/Dagster 质量门禁。
10. 跑全市场全历史 dry-run 并行验收。
11. 跑全市场全历史 replace-cascade 写入验收。
12. 编写全量验收报告，并根据报告修复遗留问题。
