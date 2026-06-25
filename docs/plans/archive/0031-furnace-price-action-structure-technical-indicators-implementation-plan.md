# Plan 0031: Furnace 价格行为与前低-次低结构指标实施方案

日期：2026-06-09

状态：Archived

归档日期：2026-06-10

归档原因：Completed

关联文档：

- `engines/README.md`
- `docs/RFC/archive/0016-rust-furnace-compute-engine.md`
- `docs/plans/archive/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0028-furnace-kdj-parallel-performance-implementation-plan.md`
- `docs/plans/archive/0029-furnace-moving-average-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0030-furnace-rsi-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0030-furnace-bollinger-bands-technical-indicators-implementation-plan.md`
- `engines/crates/furnace-core/src/indicators/kdj.rs`
- `engines/crates/furnace-core/src/indicators/moving_average.rs`
- `engines/crates/furnace-core/src/indicators/rsi.rs`
- `engines/crates/furnace-core/src/indicators/bollinger_bands.rs`
- `engines/crates/furnace-io/src/runners/shared/grouping.rs`
- `engines/crates/furnace-io/src/runners/rsi/mod.rs`
- `engines/crates/furnace-io/src/runners/ma/mod.rs`
- `engines/crates/furnace-io/src/schema/tables.rs`
- `engines/crates/furnace/src/cli.rs`
- `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
- `pipeline/scheduler/src/scheduler/defs/resources/furnace.py`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.yml`
- `pipeline/elt/models/sources_fleur_calculation.yml`

相关 skills：

- `rust-best-practices` / `rust-patterns` / `rust-testing`：Rust crate 边界、错误模型、状态延续、RowBinary 编码、单元测试和并行一致性。
- `fleur-harness`：当前代码事实检查、计划文档结构、质量门禁和后续归档。

## 1. 目标

在现有 Furnace Rust workspace 中新增一组日频价格行为和结构检测指标，用于后续指标选股场景：

1. 基于收盘价计算连阳天数和连阴天数。
2. 基于最近窗口内波峰分割，检测前低点和次低点关系。
3. 将结果写入新的 calculation 层表：

```text
fleur_calculation.calc_stock_price_pattern_daily
```

4. 由 dbt thin wrapper 暴露为：

```text
fleur_intermediate.int_stock_price_pattern_daily
```

5. 第一版价格口径拆分如下：

```text
前低-次低结构检测：
  fleur_intermediate.int_stock_quotes_daily_adj.high_price_forward_adj
  fleur_intermediate.int_stock_quotes_daily_adj.low_price_forward_adj

连阳/连阴：
  fleur_intermediate.int_stock_quotes_daily_unadj.close_price
  fleur_intermediate.int_stock_quotes_daily_unadj.prev_close_price
```

6. 指标公式只放在 `furnace-core`；ClickHouse I/O、RowBinary、Rayon 并行、staging 和分区替换放在 `furnace-io`；CLI 放在 `furnace`；Dagster 只调度 CLI；dbt 只做 source/wrapper 和字段文档。

## 2. 非目标

本计划不做以下事情：

1. 不在本次文档阶段实现 Rust、Dagster 或 dbt 代码。
2. 不把连阳、连阴或 N 形结构公式写进 Python asset、dbt SQL 或 ClickHouse SQL。
3. 不修改已有 KDJ、MA、RSI、BOLL 的输出表结构或对外行为。
4. 不把新指标塞进 `calc_stock_ma_daily`、`calc_stock_boll_daily` 或 `mart_stock_quotes_daily`。
5. 不设计任意策略 DSL 或选股查询生成器；本计划只覆盖 Furnace 指标生产。
6. 不支持多频率、多市场、多价格口径或任意用户参数持久化。
7. 不在第一版使用长表 `indicator_name/value` 结构；继续沿用 calculation 层宽表模式。
8. 不将同一证券时间序列按日期并行；所有状态和窗口逻辑必须在单证券内按 `trade_date` 严格串行推进。

## 3. 当前事实基线

### 3.1 Rust engines 现状

当前 `engines/` workspace 已有三个 crate：

```text
engines/crates/furnace       # CLI binary
engines/crates/furnace-core  # 纯指标计算
engines/crates/furnace-io    # ClickHouse I/O、RowBinary、Rayon、summary
```

已实现的 Furnace 指标包括：

| 指标 | core 模块 | CLI 子命令 | calculation 表 |
|------|-----------|------------|----------------|
| KDJ | `indicators/kdj.rs` | `furnace kdj` | `fleur_calculation.calc_stock_kdj_daily` |
| MA | `indicators/moving_average.rs` | `furnace ma` | `fleur_calculation.calc_stock_ma_daily` |
| RSI | `indicators/rsi.rs` | `furnace rsi` | `fleur_calculation.calc_stock_rsi_daily` |
| BOLL | `indicators/bollinger_bands.rs` | `furnace boll` | `fleur_calculation.calc_stock_boll_daily` |

当前缺口：

1. `furnace-core/src/indicators/mod.rs` 中没有价格行为或结构检测模块。
2. `furnace-io/src/request.rs` 中没有 `PricePatternRunRequest` 或对应 write mode。
3. `furnace-io/src/rows/` 中没有价格行为结果行、输入分组和 RowBinary 输出编码。
4. `furnace-io/src/runners/` 中没有价格行为 runner。
5. `furnace-io/src/schema/tables.rs` 中没有 `calc_stock_price_pattern_daily` DDL。
6. `furnace/src/cli.rs` 没有 `price-pattern` 子命令。
7. `pipeline/scheduler/src/scheduler/defs/furnace/` 没有 price-pattern asset、job、schedule 或 resource request。
8. `pipeline/elt/models/sources_fleur_calculation.yml` 没有 `calc_stock_price_pattern_daily` source。
9. `pipeline/elt/models/intermediate/` 没有 `int_stock_price_pattern_daily` wrapper。

### 3.2 可复用实现

本需求应复用以下现有能力：

| 现有能力 | 复用方式 |
|----------|----------|
| `ClickHouseExecutor` | 继续通过同一 executor 抽象执行 SQL、DDL 和 RowBinary 写入 |
| RowBinary scan | 参考 MA/BOLL/RSI 的 `read_*_input_row_binary` 模式，按 `security_code, trade_date` 排序读取 |
| 证券维度并行 | 参考 MA/RSI/BOLL 的 `calculate_*_outputs`，只在证券组之间 Rayon 并行 |
| staging + `REPLACE PARTITION` | 复用 shared writing 协议，保持历史修正幂等 |
| `PerformanceMetrics` | 新 summary 继续输出 read/group/compute/write/staging/replace 耗时和吞吐 |
| CLI common flags | 复用 `--from`、`--to`、`--symbols`、`--run-id`、`--mode`、`--insert-batch-size`、`--output-format json` |
| Dagster Furnace resource | 扩展现有 `FurnaceCliResource`，不新增第二套 CLI resource |
| dbt thin wrapper | 只从 `source('fleur_calculation', 'calc_stock_price_pattern_daily')` 透传和改名，不重写公式 |

### 3.3 文档编号事实

当前 `docs/plans/` 中已有两个 `0030` 文档：

```text
0030-furnace-rsi-technical-indicators-implementation-plan.md
0030-furnace-bollinger-bands-technical-indicators-implementation-plan.md
```

本计划使用 `0031` 作为新的唯一编号。本计划不修复历史编号重复，只记录当前事实。

## 4. 指标定义

### 4.1 输入行模型

新增 core 输入类型建议命名为：

```rust
PricePatternInput
```

字段语义：

| 字段 | 类型 | 来源 | 用途 |
|------|------|------|------|
| `trade_date` | `String` | `trade_date` | 单证券内排序和输出日期 |
| `high_price` | `Option<f64>` | `high_price_forward_adj` | 前低-次低结构窗口的波峰和有效价格柱校验 |
| `low_price` | `Option<f64>` | `low_price_forward_adj` | 前低点、次低点和比例计算 |
| `close_price` | `Option<f64>` | `int_stock_quotes_daily_unadj.close_price` | 连阳、连阴当日未复权收盘价 |
| `prev_close_price` | `Option<f64>` | `int_stock_quotes_daily_unadj.prev_close_price` | 连阳、连阴比较用 BaoStock 原始 preclose |

输入必须满足：

1. 同一证券内 `trade_date` 严格递增。
2. `high_price`、`low_price`、`close_price`、`prev_close_price` 中出现非有限 `f64` 时视为无效输入错误，不进入计算。
3. 结构检测的有效价格柱要求 `high_price`、`low_price` 均存在、有限、`high_price >= low_price` 且 `low_price > 0`。
4. 连阳/连阴要求 `close_price` 和 `prev_close_price` 均存在且有限。

### 4.2 连阳和连阴

基于 `int_stock_quotes_daily_unadj` 的未复权收盘价和 BaoStock 原始 preclose 逐行计算。这里不使用 `int_stock_quotes_daily_adj.close_price_forward_adj`，也不使用内存中的上一有效 close 反推前收盘价。

定义：

```text
close_direction_t =
    1   if close_price_t > prev_close_price_t
    -1  if close_price_t < prev_close_price_t
    0   if close_price_t = prev_close_price_t
```

输出字段建议：

| 字段 | 类型 | 说明 |
|------|------|------|
| `close_direction` | `Nullable(Int8)` | 当日未复权 close 相对 BaoStock preclose 的方向，`1` 为上涨，`-1` 为下跌，`0` 为持平 |
| `close_up_streak_days` | `Nullable(UInt16)` | 截至当前行连续上涨的有效交易日数 |
| `close_down_streak_days` | `Nullable(UInt16)` | 截至当前行连续下跌的有效交易日数 |

状态规则：

1. 当前行 `close_price` 或 `prev_close_price` 为空时，`close_direction`、`close_up_streak_days`、`close_down_streak_days` 均为 `NULL`。
2. 方向不可判定的行会打断连续性；下一条可判定行重新开始计数。
3. 当前行上涨时，`close_up_streak_days = previous_up_streak + 1`，`close_down_streak_days = 0`。
4. 当前行下跌时，`close_down_streak_days = previous_down_streak + 1`，`close_up_streak_days = 0`。
5. 当前行持平时，两个 streak 均重置为 `0`。
6. 如果当前行是缺口后的第一条可判定上涨行，`close_up_streak_days = 1`；下跌同理。
7. `UInt16` 足以覆盖 A 股日频连续天数；如果未来支持分钟线或超长历史，应重新评估类型。

### 4.3 `n_structure_20` 结构窗口

参考用户提供的 Python 逻辑中“最近 20 根、按窗口最高价切分左右两侧”的核心思想。第一版 Furnace 不输出固定阈值判断，也不输出 pass/fail 布尔字段，只记录结构窗口中的最高价、最低价、次低价和二者比例。

生产字段语义建议如下：

1. 最近窗口最多使用 `20` 根有效 high/low 价格柱。
2. 窗口内最高价使用最高 `high_price` 的首次出现位置，与 Python `index(max(...))` 保持一致。
3. 左侧窗口包含最高价所在价格柱：`window[..=high_idx]`。
4. 右侧窗口不包含最高价所在价格柱：`window[high_idx+1..]`。
5. `n_structure_20_low_price` 为左侧窗口最低 `low_price`。
6. `n_structure_20_second_low_price` 为右侧窗口最低 `low_price`。
7. `n_structure_20_second_low_ratio = second_low_price / low_price`；当 `low_price <= 0` 或不存在次低价时为 `NULL`。
8. 多个最高价或最低价并列时，日期字段使用对应窗口中的首次出现日期，保证结果稳定。
9. 如果窗口内还没有右侧价格柱，例如最高价就是当前窗口最后一根，则最高价和最低价仍可记录，次低价和 ratio 为 `NULL`。

输出字段建议：

| 字段 | 类型 | 说明 |
|------|------|------|
| `n_structure_20_valid_bars` | `UInt16` | 当前行可用于结构窗口的最近有效 high/low 价格柱数量，最大 20 |
| `n_structure_20_high_date` | `Nullable(Date)` | 窗口内最高价交易日 |
| `n_structure_20_high_price` | `Nullable(Float64)` | 窗口内最高价 |
| `n_structure_20_low_date` | `Nullable(Date)` | 最高价左侧含最高价价格柱的最低价交易日 |
| `n_structure_20_low_price` | `Nullable(Float64)` | 最高价左侧含最高价价格柱的最低价 |
| `n_structure_20_second_low_date` | `Nullable(Date)` | 最高价右侧最低价交易日 |
| `n_structure_20_second_low_price` | `Nullable(Float64)` | 最高价右侧最低价 |
| `n_structure_20_second_low_ratio` | `Nullable(Float64)` | 次低价 / 最低价 |
| `n_structure_20_is_valid` | `Bool` | 次低价是否严格大于最低价；等价于 `n_structure_20_second_low_ratio > 1.0`，ratio 为空时为 false |

说明：

1. `n_structure_20_is_valid` 只表达最基础的结构有效性：次低价严格大于最低价。
2. “证据不足”不是输出状态；只有字段本身无法计算时为 `NULL`，例如窗口没有有效 high/low，或最高价右侧尚无价格柱导致次低价不存在。
3. 当 ratio 为空时，`n_structure_20_is_valid=false`；旧 Python 函数中 `len < 10`、左右侧长度不足时返回 `True` 是策略保护逻辑，不进入 Furnace 生产字段。

## 5. 输出表设计

新增 ClickHouse 生产表：

```sql
CREATE TABLE IF NOT EXISTS fleur_calculation.calc_stock_price_pattern_daily
(
    security_code String,
    trade_date Date,
    close_direction Nullable(Int8),
    close_up_streak_days Nullable(UInt16),
    close_down_streak_days Nullable(UInt16),
    n_structure_20_valid_bars UInt16,
    n_structure_20_high_date Nullable(Date),
    n_structure_20_high_price Nullable(Float64),
    n_structure_20_low_date Nullable(Date),
    n_structure_20_low_price Nullable(Float64),
    n_structure_20_second_low_date Nullable(Date),
    n_structure_20_second_low_price Nullable(Float64),
    n_structure_20_second_low_ratio Nullable(Float64),
    n_structure_20_is_valid Bool
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)
```

设计说明：

1. grain 为每证券、交易日一行，与现有 KDJ/MA/RSI/BOLL calculation 表保持一致。
2. 年度分区继续服务 replace-cascade 和历史修正，不按证券、形态或布尔字段分区。
3. `ORDER BY (trade_date, security_code)` 对齐当前 calculation 表和选股日截面查询。
4. 不在表中存储输入 close/high/low，避免 calculation 表成为行情事实副本。
5. 不存储阈值参数列，也不输出固定阈值判断；策略层基于 ratio 字段动态判断。

## 6. Rust 设计

### 6.1 `furnace-core`

新增模块：

```text
engines/crates/furnace-core/src/indicators/price_pattern.rs
```

建议公开类型和函数：

```rust
pub const DEFAULT_N_STRUCTURE_WINDOW: usize = 20;

pub struct PricePatternInput { ... }
pub struct PricePatternParams { ... }
pub struct PricePatternPreviousState { ... }
pub struct PricePatternState { ... }
pub struct PricePatternOutput { ... }
pub enum PricePatternError { ... }

pub fn calculate_price_pattern_series(
    inputs: &[PricePatternInput],
    params: &PricePatternParams,
    previous_state: Option<PricePatternPreviousState>,
) -> Result<Vec<PricePatternOutput>, PricePatternError>
```

状态设计：

```text
PricePatternState
  up_streak_days: u16
  down_streak_days: u16
  last_direction: i8
  structure_window: VecDeque<StructurePriceBar>
```

`PricePatternPreviousState` 必须带状态日期：

```text
trade_date
state
```

用途：

1. append-latest 时可以从目标表上一条可判定结果继续连阳/连阴计数状态。
2. pattern 检测只需要最近 20 根有效 high/low 价格柱；即使不从目标表恢复窗口，也可以通过 lookback 输入重建。
3. replace-cascade 可从请求起点前的稳定状态继续，或在状态缺失时回读 full history。

错误模型：

| 错误 | 场景 |
|------|------|
| `InvalidParams` | structure window 无效 |
| `NonIncreasingTradeDate` | 单证券输入日期未严格递增 |
| `InvalidPrice` | 有效输入中出现非有限值、结构 low <= 0 或 high < low |
| `StreakOverflow` | 连续天数超过 `UInt16` 容量 |

实现要求：

1. 参数校验不使用 `unwrap()` 或 `expect()`。
2. 函数参数优先使用 slice 和 reference，避免为绕过 borrow checker 复制大数组。
3. 输出顺序必须与输入顺序一一对应。
4. `None` close 或 `None` prev_close 会输出空 streak 字段并打断 streak 状态。
5. `None` high/low 不推进结构窗口。
6. 单证券内部不得使用 Rayon 并行。

### 6.2 `furnace-io`

新增或扩展模块：

```text
engines/crates/furnace-io/src/request/price_pattern.rs
engines/crates/furnace-io/src/rows/price_pattern.rs
engines/crates/furnace-io/src/summary/price_pattern.rs
engines/crates/furnace-io/src/runners/price_pattern/mod.rs
engines/crates/furnace-io/src/runners/price_pattern/planning.rs
engines/crates/furnace-io/src/runners/price_pattern/materialize.rs
engines/crates/furnace-io/src/runners/price_pattern/writing.rs
```

新增默认常量：

```rust
DEFAULT_PRICE_PATTERN_OUTPUT_TABLE = "fleur_calculation.calc_stock_price_pattern_daily"
DEFAULT_PRICE_PATTERN_STRUCTURE_INPUT_TABLE = "fleur_intermediate.int_stock_quotes_daily_adj"
DEFAULT_PRICE_PATTERN_STREAK_INPUT_TABLE = "fleur_intermediate.int_stock_quotes_daily_unadj"
DEFAULT_PRICE_PATTERN_HIGH_COLUMN = "high_price_forward_adj"
DEFAULT_PRICE_PATTERN_LOW_COLUMN = "low_price_forward_adj"
DEFAULT_PRICE_PATTERN_CLOSE_COLUMN = "close_price"
DEFAULT_PRICE_PATTERN_PREV_CLOSE_COLUMN = "prev_close_price"
```

请求类型：

```text
PricePatternRunRequest
  request_from
  request_to
  symbols
  run_id
  mode
  structure_input_table
  streak_input_table
  output_table
  high_column
  low_column
  close_column
  prev_close_column
  insert_batch_size
  params
```

生产写入校验：

1. 生产写入只允许 canonical structure window：`window=20`。
2. 生产写入只允许默认 structure/streak input table 和默认 high/low/close/prev_close 字段。
3. 生产写入只允许 `insert_batch_size >= MIN_INSERT_BATCH_SIZE`。
4. 所有表名和列名使用现有 validation helper 校验。

RowBinary 读取 SQL：

```sql
SELECT
    adj.security_code,
    toString(adj.trade_date),
    adj.high_price_forward_adj,
    adj.low_price_forward_adj,
    unadj.close_price,
    unadj.prev_close_price
FROM fleur_intermediate.int_stock_quotes_daily_adj AS adj
LEFT JOIN fleur_intermediate.int_stock_quotes_daily_unadj AS unadj
  ON adj.security_code = unadj.security_code
 AND adj.trade_date = unadj.trade_date
WHERE adj.trade_date >= toDate('{input_from}')
  AND adj.trade_date <= toDate('{input_to}')
  AND {symbol_filter}
ORDER BY adj.security_code, adj.trade_date
FORMAT RowBinary
```

需要补充 RowBinary helper：

1. `push_rowbinary_nullable_i8` 或专用 nullable `Int8` 写入。
2. `push_rowbinary_nullable_u16`。
3. `push_rowbinary_bool`，用于写入非空 `n_structure_20_is_valid`。
4. `push_rowbinary_nullable_date`。
5. 测试必须覆盖 nullable marker、Bool、Date days encoding 和字段顺序。

### 6.3 状态与输入范围

本指标同时存在两种状态需求：

1. 连阳/连阴的当日方向来自 `close_price` 与 `prev_close_price` 的逐行比较，但连续天数仍需要上一条可判定结果的 streak 计数。
2. 前低-次低检测需要当前输出日前最多 20 根有效 high/low 价格柱。

推荐 runner 策略：

1. 先解析请求 symbols。
2. resolve `effective_output_to`：`replace-cascade` 时级联到受影响证券最新输入交易日。
3. resolve full-history input start：structure 输入表和 streak 输入表 join 后，请求 symbols 可计算行的最早交易日。
4. 如果目标表存在且不是 full-history 运行，尝试读取 request_from 之前最近一条可延续 streak state；该 state 只包含上一方向和 streak 计数，不包含 close。
5. 对 structure window，始终从 structure 输入表回读 request_from 之前最多 20 根有效 high/low 价格柱；这避免把窗口状态写入 calculation 表。
6. 如果 append-latest 发现目标表存在结果缺口，应像 RSI 一样拒绝写入，并提示使用更早起点或 `replace-cascade`。
7. 如果无法证明 previous state 安全，则从 full-history input start 读取并计算，保证结果正确优先于速度。

`input_from` 选择规则：

```text
input_from = min(
    streak_state_required_from,
    structure_lookback_input_from
)
```

其中：

1. 有安全 previous streak state 时，`streak_state_required_from = request_from`。
2. 无安全 previous streak state 时，`streak_state_required_from = full_history_input_from`。
3. `structure_lookback_input_from` 为 request_from 前最多 20 根有效 high/low 的最早日期；如果无 lookback，则为 request_from。

summary 必须记录：

| 字段 | 说明 |
|------|------|
| `indicator` | 固定为 `price_pattern` |
| `request_from/request_to` | 用户请求输出区间 |
| `effective_output_from/effective_output_to` | 实际输出区间 |
| `input_from/input_to` | 实际读取输入区间 |
| `symbols_count` | 证券数 |
| `input_rows/output_rows` | 输入和输出行数 |
| `valid_streak_rows` | 输出区间内 `close_price` 和 `prev_close_price` 均有效、可判定涨跌方向的行数 |
| `valid_structure_bar_rows` | 输出区间内有效 high/low 结构价格柱行数 |
| `null_streak_rows` | 输出区间内 streak 字段为空的行数 |
| `null_second_low_rows` | 输出区间内次低价或次低/最低价比例为空的行数 |
| `state_source` | `full-history`、`previous-state` 或 `mixed` |
| `n_structure_window` | canonical 值 20 |
| `performance_metrics` | 沿用现有结构 |

### 6.4 CLI

新增子命令建议：

```bash
furnace price-pattern \
  --from 2026-05-06 \
  --to 2026-06-01 \
  --symbols 000069.SZ \
  --mode dry-run \
  --structure-input-table fleur_intermediate.int_stock_quotes_daily_adj \
  --streak-input-table fleur_intermediate.int_stock_quotes_daily_unadj \
  --output-table fleur_calculation.calc_stock_price_pattern_daily \
  --high-column high_price_forward_adj \
  --low-column low_price_forward_adj \
  --close-column close_price \
  --prev-close-column prev_close_price \
  --insert-batch-size 10000 \
  --output-format json
```

第一版不建议暴露 `--n-structure-window` 生产参数。原因：

1. 字段前缀已经固定为 `n_structure_20`，生产写入允许其他窗口会破坏字段语义。
2. 输出表字段名没有参数后缀，生产写入如果允许多参数会破坏字段语义。
3. 如需 dry-run 参数实验，可以后续另加 CLI-only 参数，但生产写入仍必须 canonical。

## 7. Dagster 设计

需要扩展：

```text
pipeline/scheduler/src/scheduler/defs/resources/furnace.py
pipeline/scheduler/src/scheduler/defs/furnace/assets.py
pipeline/scheduler/src/scheduler/defs/furnace/definitions.py
pipeline/scheduler/tests/unit/resources/test_furnace.py
pipeline/scheduler/tests/unit/furnace/test_furnace_definitions.py
pipeline/scheduler/tests/integration/test_definitions_and_schedules.py
```

新增 Python dataclass：

```text
FurnacePricePatternCliRequest
FurnacePricePatternCliResult
```

新增 resource 方法：

```text
run_price_pattern()
command_for_price_pattern_request()
```

新增 Dagster asset：

```text
fleur_calculation/calc_stock_price_pattern_daily
```

默认 config：

| 字段 | 默认值 |
|------|--------|
| `request_from` | schedule 运行日前一交易日或当前已有 Furnace 模式中的默认策略 |
| `request_to` | schedule 目标日期 |
| `mode` | daily schedule 使用 `append-latest`，backfill 使用 `replace-cascade`，dry run job 使用 `dry-run` |
| `structure_input_table` | `fleur_intermediate.int_stock_quotes_daily_adj` |
| `streak_input_table` | `fleur_intermediate.int_stock_quotes_daily_unadj` |
| `output_table` | `fleur_calculation.calc_stock_price_pattern_daily` |
| `high_column` | `high_price_forward_adj` |
| `low_column` | `low_price_forward_adj` |
| `close_column` | `close_price` |
| `prev_close_column` | `prev_close_price` |
| `insert_batch_size` | `10000` |

materialization metadata 至少包含：

```text
indicator
mode
request_from
request_to
effective_output_to
input_rows
output_rows
valid_streak_rows
valid_structure_bar_rows
null_streak_rows
null_second_low_rows
state_source
writes_applied
performance_metrics
```

## 8. dbt 设计

### 8.1 source

在 `pipeline/elt/models/sources_fleur_calculation.yml` 增加：

```text
source('fleur_calculation', 'calc_stock_price_pattern_daily')
```

字段描述应说明：

1. 前低-次低结构检测使用前复权 high/low 口径。
2. 连阳/连阴使用 `int_stock_quotes_daily_unadj.close_price` 与 `prev_close_price` 逐行比较，不使用前复权 close。
3. `close_price` 或 `prev_close_price` 为空时方向不可判定，且会打断 streak。
4. `n_structure_20_*` 字段只记录 20 根窗口内最高价、最低价、次低价和次低/最低价比例，不输出 pass/fail。
5. `n_structure_20_is_valid` 使用 `n_structure_20_second_low_ratio > 1.0` 判断；ratio 为空时为 false。

### 8.2 wrapper

新增：

```text
pipeline/elt/models/intermediate/int_stock_price_pattern_daily.sql
pipeline/elt/models/intermediate/int_stock_price_pattern_daily.yml
```

wrapper 只透传业务字段，不重写公式。

建议暴露全部字段：

```text
security_code
trade_date
close_direction
close_up_streak_days
close_down_streak_days
n_structure_20_valid_bars
n_structure_20_high_date
n_structure_20_high_price
n_structure_20_low_date
n_structure_20_low_price
n_structure_20_second_low_date
n_structure_20_second_low_price
n_structure_20_second_low_ratio
n_structure_20_is_valid
```

测试：

1. `unique_combination_of_columns: [security_code, trade_date]`
2. `security_code` not null + `cn_security_code_format`
3. `trade_date` not null
4. `close_direction` accepted values `[-1, 0, 1]`，允许 NULL

## 9. 测试策略

### 9.1 `furnace-core` 单元测试

必须覆盖：

1. 当日 `close_price > prev_close_price` 输出 `close_direction=1` 并推进连阳。
2. 连续上涨、连续下跌和持平重置。
3. `close_price` 或 `prev_close_price` 缺失时输出 NULL 且打断 streak。
4. 缺口后第一条可判定上涨或下跌行从 1 开始计数。
5. 输入日期非递增返回错误。
6. 非有限 close/prev_close/high/low 返回错误。
7. 窗口没有有效 high/low 时 `n_structure_20_high_price`、`low_price`、`second_low_price` 和 ratio 均为空。
8. 窗口超过 20 根时只使用最近 20 根有效 high/low。
9. 最高价使用最高 high 的首次出现。
10. 最高价右侧没有价格柱时，次低价和 ratio 为空。
11. 次低/最低价 ratio 按 `second_low_price / low_price` 计算。
12. `n_structure_20_is_valid` 在 ratio > 1.0 时为 true，ratio <= 1.0 或 ratio 为空时为 false。
13. 最高价、最低价、次低价日期 tie-break 稳定。
14. previous state continuation 与 full-history 计算结果一致。

### 9.2 `furnace-io` 单元测试

必须覆盖：

1. DDL 包含 canonical 字段、年度分区和 `ORDER BY (trade_date, security_code)`。
2. staging 表名 normalizes run_id。
3. RowBinary result row 写入字段顺序与 DDL 一致。
4. dry-run 读取 high/low/close/prev_close 输入并输出 JSON summary。
5. append-latest 写入目标表且包含所有字段。
6. append-latest 检测 result gap 并拒绝写入。
7. replace-cascade 使用 staging、保留旧行、校验 staging、替换 affected years。
8. 串行和 Rayon 并行输出完全一致。
9. 生产写入拒绝非 canonical input table、字段名和过小 insert batch。

### 9.3 CLI 测试

必须覆盖：

1. `furnace price-pattern --from ... --to ... --output-format json` 能解析。
2. unknown flag 返回 usage error。
3. `--to < --from` 返回 usage error。
4. 非 json output format 返回 usage error。
5. help 文本包含 `price-pattern` 子命令。

### 9.4 Dagster 测试

必须覆盖：

1. `FurnaceCliResource.command_for_price_pattern_request()` 命令参数顺序和默认值。
2. `run_price_pattern()` 能解析 stdout JSON summary。
3. resource timeout、非 0 exit、非 JSON stdout 错误路径沿用现有行为。
4. definitions 注册 price-pattern asset、daily/backfill/dry-run jobs 和 daily schedule。
5. schedule run config 包含默认 structure/streak input/output/high/low/close/prev_close 字段。

### 9.5 dbt 测试

必须覆盖：

1. `dbt parse` 能识别新 source 和 wrapper。
2. `int_stock_price_pattern_daily` 定向 build 通过。
3. YAML 字段文档和 tests 完整。
4. 不在 wrapper SQL 中出现前低/次低公式实现，只读取 source。

## 10. 实施阶段

### 阶段 1：core 指标模块

交付：

1. 新增 `price_pattern.rs`。
2. 在 `indicators/mod.rs` 和 `lib.rs` 导出公共类型。
3. 完成 core 单元测试。

完成标准：

```bash
cd engines
cargo test -p furnace-core price_pattern
```

### 阶段 2：furnace-io schema、rows、request 和 runner

交付：

1. 新增 output table DDL、staging helpers 和 replace partition helpers。
2. 新增 RowBinary nullable bool/date/int helpers。
3. 新增 request、rows、summary 和 runner。
4. 完成 io 单元测试。

完成标准：

```bash
cd engines
cargo test -p furnace-io price_pattern
```

### 阶段 3：CLI

交付：

1. 新增 `furnace price-pattern` 子命令。
2. 更新 help 文本。
3. 完成 CLI 解析测试。

完成标准：

```bash
cd engines
cargo test -p furnace
```

### 阶段 4：Dagster 调度

交付：

1. 扩展 Furnace CLI resource。
2. 新增 price-pattern calculation asset。
3. 新增 daily/backfill/dry-run jobs 和 schedule。
4. 更新 scheduler 单元和集成测试。

完成标准：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/resources/test_furnace.py scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/integration/test_definitions_and_schedules.py -q
cd scheduler
uv run dg check defs
```

### 阶段 5：dbt source 和 wrapper

交付：

1. 更新 `sources_fleur_calculation.yml`。
2. 新增 `int_stock_price_pattern_daily.sql/yml`。
3. 如需消费到 mart，再另立计划决定是否并入选股 serving 宽表。

完成标准：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_price_pattern_daily
```

### 阶段 6：端到端冒烟和性能记录

交付：

1. 单证券 dry-run。
2. 单证券 append-latest。
3. 单证券 replace-cascade。
4. 全市场 dry-run 性能记录。
5. 写入后 dbt wrapper build。
6. 新增 job report。

建议报告路径：

```text
docs/jobs/reports/YYYY-MM-DD-furnace-price-pattern-smoke-run.md
docs/jobs/reports/YYYY-MM-DD-furnace-price-pattern-performance-baseline.md
```

## 11. 最小质量门禁

Rust 变更：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Dagster/dbt 变更：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/resources/test_furnace.py scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/integration/test_definitions_and_schedules.py -q
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_price_pattern_daily
cd scheduler
uv run dg check defs
```

文档-only 变更：

```bash
git diff --check
```

## 12. 风险与决策

### 12.1 为什么旧逻辑会有“证据不足”

参考函数在以下情况返回 `True`：

1. `len(klines) < 10`
2. 左侧窗口少于 2 根
3. 右侧窗口少于 2 根

这是因为旧函数要直接返回一个 `true/false` 判断。要判断结构是否通过，至少要有可用窗口、最高价左侧低点和最高价右侧次低点；当这些条件不存在时，旧函数用 `True` 做宽松保护。

本计划决策：

```text
不输出 pass/fail
只记录 n_structure_20_high/low/second_low/ratio
当最高价右侧没有价格柱时，second_low 和 ratio 为 NULL
```

因此第一版生产表不再需要“证据不足”作为状态。它只存在于旧的布尔策略函数语境中。

### 12.2 价格口径拆分

第一版使用两套价格口径：

1. 连阳/连阴使用 `int_stock_quotes_daily_unadj.close_price` 和 `prev_close_price`，符合日线涨跌定义和 BaoStock 原始 preclose 口径。
2. 前低-次低结构检测继续使用 `int_stock_quotes_daily_adj.high_price_forward_adj` 和 `low_price_forward_adj`，避免除权除息在窗口内制造人工低点。
3. 如果后续策略要求结构检测也使用未复权 high/low，应另立计划并明确字段语义，不在同一字段中混用。

### 12.3 连阳/连阴状态不是有限 lookback

前低-次低结构最多需要 20 根有效 high/low，但连阳/连阴连续天数理论上是无限状态。虽然每日方向来自 `close_price` 与 `prev_close_price` 的逐行比较，streak 计数仍不能只读取 20 根 lookback，否则长连阳/长连阴会被截断。

本计划要求：

1. append-latest 可读取上一条 production result state 继续。
2. 状态缺失或有 gap 时必须 full-history 或从最早缺口重算。
3. replace-cascade 必须级联到受影响证券最新输入交易日。

## 13. 后续扩展

可在本计划完成后另立计划处理：

1. 将 `int_stock_price_pattern_daily` 接入选股 feature registry。
2. 建立 `mart_stock_screen_features_daily` 或类别化 mart 表。
3. 支持更多结构指标，例如平台突破、回踩确认、箱体低点、趋势段拐点。
4. 支持形态评分字段，而不是只输出布尔检测。
5. 对历史 Python 策略样本做兼容性对账，量化去掉 pass/fail 后由策略层判断带来的筛选结果变化。
