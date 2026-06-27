# Plan 0034: Furnace MACD 日线技术指标实施计划

日期：2026-06-10

状态：Archived

归档日期：2026-06-10

归档原因：Completed

0035 前置任务：本计划已完成，`docs/plans/archive/0035-stock-technical-indicator-marts-implementation-plan.md` 已通过 `int_stock_macd_daily` 把 MACD 纳入 `mart_stock_trend_indicator`。

关联文档：

- `docs/RFC/archive/0016-rust-furnace-compute-engine.md`
- `docs/plans/archive/0035-stock-technical-indicator-marts-implementation-plan.md`
- `docs/plans/archive/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0028-furnace-kdj-parallel-performance-implementation-plan.md`
- `docs/plans/archive/0029-furnace-moving-average-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0030-furnace-bollinger-bands-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0030-furnace-rsi-technical-indicators-implementation-plan.md`
- `engines/README.md`
- `engines/crates/furnace-core/src/operators/ema.rs`
- `engines/crates/furnace-core/src/indicators/moving_average.rs`
- `engines/crates/furnace-core/src/indicators/rsi.rs`
- `engines/crates/furnace-io/src/schema/tables.rs`
- `engines/crates/furnace-io/src/rows/ma.rs`
- `engines/crates/furnace-io/src/request/rsi.rs`
- `engines/crates/furnace-io/src/summary/rsi.rs`
- `engines/crates/furnace/src/cli.rs`
- `engines/crates/furnace/src/commands/rsi.rs`
- `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
- `pipeline/scheduler/src/scheduler/defs/resources/furnace.py`
- `pipeline/elt/models/sources_fleur_calculation.yml`

相关 skills：

- `rust-best-practices` / `rust-patterns` / `rust-testing`：Rust 指标模块、状态类型、错误模型、测试和性能。
- `dagster-expert`：Dagster calculation asset、依赖、metadata、resource CLI 调用。
- `using-dbt-for-analytics-engineering` / `running-dbt-commands`：dbt source、intermediate wrapper、字段文档和定向 build。
- `clickhouse-best-practices` / `clickhouse-architecture-advisor`：ClickHouse MergeTree schema、年度分区、批量写入和避免 mutation。

## 1. 目标

在现有 Furnace Rust workspace 中新增日频 MACD 指标计算能力，从：

```text
fleur_intermediate.int_stock_quotes_daily_adj
```

读取前复权收盘价 `close_price_forward_adj`，按 canonical `MACD(12,26,9)` 计算并写入：

```text
fleur_calculation.calc_stock_macd_daily
```

再由 dbt thin wrapper 暴露为：

```text
fleur_intermediate.int_stock_macd_daily
```

完成后应满足：

1. `furnace macd` 支持按日期区间、证券集合和运行模式计算 MACD 指标。
2. MACD 使用 `close_price_forward_adj` 作为唯一 canonical close 输入，不混用未复权、后复权或其他价格口径。
3. 第一版生产参数固定为 `MACD(12,26,9)`。
4. EMA(12)、EMA(26) 和 DEA(9) 均采用 SMA 启动，不使用 zero start。
5. MACD 公式只放在 `furnace-core`；ClickHouse I/O、RowBinary、Rayon 并行和 staging/partition replace 放在 `furnace-io`；CLI 放在 `furnace`；调度与 metadata 放在 Dagster；消费契约放在 dbt。
6. 生产写入使用 staging + 年度 `REPLACE PARTITION` 协议，保持幂等并避免高频 mutation。
7. 计算按证券维度并行；单证券内部按 `trade_date` 严格串行递推。
8. 支持全量计算和日常增量优化。增量结果必须与同一证券全历史一次性计算一致，不能因为 warm-up 截断产生偏差。
9. 完成后 `0035` 可直接通过 `{{ ref('int_stock_macd_daily') }}` 消费 MACD 字段。
10. MACD 增量实现必须明确区分“完整可延续状态”和“需要 lookback 重建状态”的证券，不能用不完整状态直接递推。

## 2. 非目标

本计划不做以下事情：

1. 不实现 WR、CCI、ATR、OBV、DMA、TRIX 或其他技术指标。
2. 不改变 `int_stock_quotes_daily_adj` 的复权逻辑、字段语义或物化策略。
3. 不在 dbt SQL、Dagster Python asset 或 ClickHouse SQL 中重写 MACD 公式。
4. 不让 Furnace 直接写入 `fleur_intermediate.int_stock_macd_daily`。
5. 不支持多参数集合、多价格口径、多频率或长表 `indicator_name/value` 结构。
6. 不把同一证券时间序列按日期并行；EMA 递推状态必须在单证券内串行推进。
7. 不把第一版 `calc_stock_macd_daily` 建成 Dagster daily partition asset；历史级联写入会突破单日 partition 语义。
8. 不修改已上线 MA、BOLL、RSI、KDJ 的对外字段和计算口径。

## 3. 当前事实基线

### 3.1 Rust workspace

当前 Rust workspace 已有三层 crate：

```text
engines/
├── crates/furnace/       # CLI binary，已有 kdj/ma/rsi/boll/price-pattern 子命令
├── crates/furnace-core/  # 纯指标计算，已有 KDJ、MA、RSI、BOLL、Price Pattern
└── crates/furnace-io/    # ClickHouse I/O、RowBinary、Rayon、staging/replace
```

可复用事实：

1. `furnace-core::operators::SmaSeededEma` 已实现 SMA 启动 EMA，支持 `previous_state` 延续递推。
2. `int_stock_ma_daily` 的 `price_ema2_10` 已证明双重 EMA 计算可复用 `SmaSeededEma`。
3. RSI 实现已经包含多窗口状态、previous state 日期、gap 检测和增量延续设计，可作为 MACD 增量状态模式参考。
4. `furnace-io` 已有 calculation 表 DDL、staging 表、RowBinary 读写、按证券 Rayon 并行、年度 `REPLACE PARTITION` 和 JSON summary 模板。
5. `furnace` CLI 已有子命令注册模式；新增 `macd` 应沿用 `rsi` 或 `boll` 的命令参数结构。

### 3.2 输入和输出边界

MACD 输入默认来自：

```text
fleur_intermediate.int_stock_quotes_daily_adj
```

第一版只读取：

| 字段 | 用途 |
|---|---|
| `security_code` | 证券代码 |
| `trade_date` | 交易日 |
| `close_price_forward_adj` | MACD 的 canonical close 输入 |

输出表：

```text
fleur_calculation.calc_stock_macd_daily
```

dbt wrapper：

```text
pipeline/elt/models/intermediate/int_stock_macd_daily.sql
pipeline/elt/models/intermediate/int_stock_macd_daily.yml
```

### 3.3 ClickHouse schema 约束

工作负载：A 股日频市场数据，计算层技术指标宽表，主要查询按交易日范围、证券代码和年度分区消费。

适用规则：

1. Per `schema-pk-plan-before-creation`：`ORDER BY` 创建后不可改，MACD 建表前必须确认排序键。
2. Per `schema-pk-cardinality-order` 和 `schema-pk-prioritize-filters`：沿用现有 calculation 表 `ORDER BY (trade_date, security_code)`，优先服务交易日范围和按日批量消费。
3. Per `schema-types-native-types` 和 `schema-types-minimize-bitwidth`：`trade_date` 使用 `Date`，参数使用 `UInt16`，指标和状态使用 `Nullable(Float64)`。
4. Per `schema-partition-low-cardinality` 和 `decision-partitioning-timeseries`：沿用 `PARTITION BY toYear(trade_date)`，避免日级高分区数。
5. Per `insert-batch-size`：生产写入批次必须至少 `MIN_INSERT_BATCH_SIZE = 1000`，默认 `10000`。
6. Per `insert-mutation-avoid-update` 和 `insert-mutation-avoid-delete`：历史修正使用 staging + `REPLACE PARTITION`，不使用频繁 `ALTER UPDATE` / `ALTER DELETE`。
7. 现有 calculation 表均由 Furnace runner 创建 `fleur_calculation` database、目标表和 run-scoped staging 表；MACD 必须沿用同一建表和 staging 生命周期，不引入 dbt DDL 或手工 SQL 前置步骤。

## 4. 公式和语义

### 4.1 canonical 参数

第一版生产只允许：

```text
MACD(12,26,9)
```

字段命名：

| 字段 | 语义 |
|---|---|
| `ema_fast_state_12` | EMA(12) 的递推状态列，用于增量延续，不由 mart 直接消费 |
| `ema_slow_state_26` | EMA(26) 的递推状态列，用于增量延续，不由 mart 直接消费 |
| `macd_dif` | DIF 快线，`EMA(12) - EMA(26)` |
| `macd_dea` | DEA 慢线，对 DIF 做 EMA(9) 平滑 |
| `macd_dea_state` | DEA(9) 的递推状态列，用于增量延续 |
| `macd_histogram` | 标准版柱状图，`DIF - DEA` |

第一版不输出 `2 * (DIF - DEA)` 的增强版柱状图。若后续下游需要，可新增独立字段如 `macd_bar_2x`，不得改变 `macd_histogram` 口径。

### 4.2 EMA 启动和递推

EMA(12) 启动：

```text
ema_fast_state_12 = avg(close_price_forward_adj over first 12 valid closes)
```

EMA(26) 启动：

```text
ema_slow_state_26 = avg(close_price_forward_adj over first 26 valid closes)
```

在第 N 个有效 close 之后，递推公式：

```text
EMA_N(today) = EMA_N(yesterday) * (N - 1) / (N + 1)
             + close(today) * 2 / (N + 1)
```

具体到 canonical 参数：

```text
EMA(12): alpha = 2 / 13, previous weight = 11 / 13
EMA(26): alpha = 2 / 27, previous weight = 25 / 27
```

现有 `SmaSeededEma` 已按：

```text
next = alpha * value + (1 - alpha) * previous
alpha = 2 / (window + 1)
```

实现，可直接复用，不再新增重复 EMA 算子。

### 4.3 DIF、DEA 和 histogram

DIF：

```text
DIF = EMA(12) - EMA(26)
```

只有当同一行的 EMA(12) 和 EMA(26) 都可用时，`macd_dif` 才可用；在没有空 close 的理想序列中，第一条 DIF 出现在第 26 个有效 close。

DEA：

```text
DEA = EMA(DIF, 9)
```

DEA 也采用 SMA 启动：先收集 9 个有效 DIF，取简单平均作为初始 DEA；之后使用：

```text
DEA(today) = DEA(yesterday) * 8 / 10 + DIF(today) * 2 / 10
```

在没有空 close 的理想序列中，第一条 DEA 出现在第 34 个有效 close。

Histogram：

```text
macd_histogram = macd_dif - macd_dea
```

只有 DIF 和 DEA 都可用时，`macd_histogram` 才可用。

### 4.4 NULL 和有效值推进

1. `close_price_forward_adj IS NULL` 的交易日保留输出行，但不进入 EMA(12)、EMA(26) 启动窗口，也不推进递推状态。
2. close 为空时，业务字段和状态字段均输出 NULL，已有状态保持不变。
3. DIF 为空时不进入 DEA(9) 启动窗口，也不推进 DEA 状态。
4. 所有输入值必须是有限数；非有限数在 `furnace-core` 返回错误，不静默输出。
5. 输出行数应等于请求输出范围内输入行数，不因指标不可用丢行。

## 5. 目标数据模型

### 5.1 `furnace-core`

新增：

```text
engines/crates/furnace-core/src/indicators/macd.rs
```

建议 public API：

```text
MacdInput
MacdParams
MacdState
MacdPreviousState
MacdOutput
MacdError
calculate_macd
```

参数：

```text
fast_window = 12
slow_window = 26
signal_window = 9
```

状态：

```text
ema_fast_state_12
ema_slow_state_26
macd_dea_state
```

MACD 不需要 `previous_close` 才能延续。不要引入无用途状态字段；完整延续状态只需要 EMA fast、EMA slow 和 DEA 三个状态值。

状态完整性规则：

1. 当历史输出行同时具备 `ema_fast_state_12`、`ema_slow_state_26` 和 `macd_dea_state` 时，可从下一条有效 close 直接递推。
2. 当历史输出行只有 EMA fast/slow，尚未形成 DEA 状态时，不能直接进入完整增量；必须读取足够 lookback 行重建 DIF 序列和 DEA 启动窗口。
3. 当证券历史不足 34 个有效 close 或存在长时间 NULL close 时，summary 必须把这类证券归入 warm-up 或 gap 统计，而不是把 NULL 指标误判为失败。

### 5.2 `furnace-io`

新增或扩展：

```text
engines/crates/furnace-io/src/request/macd.rs
engines/crates/furnace-io/src/rows/macd.rs
engines/crates/furnace-io/src/summary/macd.rs
engines/crates/furnace-io/src/runners/macd.rs
```

扩展：

```text
engines/crates/furnace-io/src/schema/tables.rs
engines/crates/furnace-io/src/schema/staging.rs
engines/crates/furnace-io/src/schema/partition.rs
engines/crates/furnace-io/src/request/mod.rs
engines/crates/furnace-io/src/rows/mod.rs
engines/crates/furnace-io/src/summary/mod.rs
engines/crates/furnace-io/src/runners/mod.rs
engines/crates/furnace-io/src/lib.rs
```

新增常量：

```text
DEFAULT_MACD_OUTPUT_TABLE = "fleur_calculation.calc_stock_macd_daily"
DEFAULT_MACD_PRICE_COLUMN = "close_price_forward_adj"
```

DDL 草案：

```sql
CREATE TABLE IF NOT EXISTS fleur_calculation.calc_stock_macd_daily
(
    security_code String,
    trade_date Date,
    ema_fast_state_12 Nullable(Float64),
    ema_slow_state_26 Nullable(Float64),
    macd_dif Nullable(Float64),
    macd_dea Nullable(Float64),
    macd_dea_state Nullable(Float64),
    macd_histogram Nullable(Float64)
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)
```

说明：

1. `ema_*_state` 和 `macd_dea_state` 是内部延续状态列。
2. dbt intermediate wrapper 默认不暴露内部状态列，除非后续明确需要调试或质量审计。
3. production writes 只允许 canonical 参数、默认 input table、默认 price column 和最小插入批次。
4. 生产 output table 默认使用 `fleur_calculation.calc_stock_macd_daily`。若实施时为了测试保留 `--output-table`，production writes 必须至少校验表名位于 `fleur_calculation` schema；推荐与 MA/RSI/BOLL 行为收敛为只允许默认 output table，避免误写非生产表。

### 5.3 `furnace` CLI

新增：

```text
engines/crates/furnace/src/commands/macd.rs
```

扩展：

```text
engines/crates/furnace/src/commands/mod.rs
engines/crates/furnace/src/cli.rs
engines/crates/furnace/src/cli_tests.rs
engines/crates/furnace/src/output.rs
```

命令草案：

```bash
furnace macd \
  --from 2024-01-01 \
  --to 2024-12-31 \
  --mode dry-run \
  --input-table fleur_intermediate.int_stock_quotes_daily_adj \
  --output-table fleur_calculation.calc_stock_macd_daily \
  --price-column close_price_forward_adj \
  --insert-batch-size 10000 \
  --output-format json
```

第一版可不开放 `--fast-window`、`--slow-window`、`--signal-window`。如为了测试开放参数，production writes 仍必须拒绝非 canonical 参数。

CLI 测试还需覆盖 `--output-format json` 不产生额外 stdout 文本；Dagster resource 依赖 stdout 可被整体解析为单个 JSON object。

### 5.4 Dagster

新增：

```text
FURNACE_MACD_ASSET_KEY = dg.AssetKey(["fleur_calculation", "calc_stock_macd_daily"])
FURNACE_MACD_UPSTREAM_ASSET_KEY = dg.AssetKey(["int_stock_quotes_daily_adj"])
FURNACE_MACD_GROUP = "calculation"
FurnaceMacdRunConfig
build_furnace_macd_asset()
FURNACE_MACD_ASSETS
```

扩展：

```text
FURNACE_ASSETS = (... + FURNACE_MACD_ASSETS + ...)
FurnaceMacdCliRequest
FurnaceMacdCliResult
FurnaceCliResource.run_macd()
FurnaceCliResource.command_for_macd_request()
_metadata_from_summary()
```

Dagster asset metadata 至少包含：

```text
database = fleur_calculation
table = calc_stock_macd_daily
indicator = macd
price_adjustment = forward
price_column = close_price_forward_adj
macd_params = {fast_window: 12, slow_window: 26, signal_window: 9}
histogram = DIF - DEA
```

依赖使用 `deps=[FURNACE_MACD_UPSTREAM_ASSET_KEY]`，不通过 IOManager 加载上游表。

### 5.5 dbt

扩展 source：

```text
pipeline/elt/models/sources_fleur_calculation.yml
```

新增 intermediate wrapper：

```text
pipeline/elt/models/intermediate/int_stock_macd_daily.sql
pipeline/elt/models/intermediate/int_stock_macd_daily.yml
```

wrapper 第一版暴露业务字段：

```text
security_code
trade_date
macd_dif
macd_dea
macd_histogram
```

不暴露：

```text
ema_fast_state_12
ema_slow_state_26
macd_dea_state
```

除非实施时决定调试收益大于消费契约噪声；若暴露，必须在 YAML 中明确“内部状态列，下游 mart 不消费”。

source YAML 必须记录 calculation 表所有物理列，包括内部状态列；intermediate wrapper YAML 只记录 wrapper 暴露字段。这样既保留 raw calculation 可观测性，又避免 mart 消费内部状态。

## 6. 实施阶段

### 阶段 0：冻结口径和测试样本

1. 固定 canonical 参数为 `MACD(12,26,9)`。
2. 固定 `macd_histogram = DIF - DEA`，不使用 2 倍增强版。
3. 准备一个小样本证券序列，覆盖：
   - 34 个连续有效 close，验证第一条 DEA/histogram 出现点。
   - 中间 close 为 NULL，验证状态不推进。
   - 增量 previous state 接续，验证与全量计算一致。

完成标准：

- 测试样本写入 `furnace-core` 单元测试或 test helper。
- 浮点断言使用项目已有 `assert_close` 风格，容忍度不大于 `1e-9`。

### 阶段 1：实现 `furnace-core` MACD 纯计算

实施：

1. 新增 `indicators/macd.rs` 并在 `indicators/mod.rs` 导出。
2. 复用 `SmaSeededEma` 创建 EMA(12)、EMA(26) 和 DEA(9)。
3. 定义 `MacdParams::default()` 和 `is_canonical()`。
4. 定义 `MacdOutput::all_business_indicators_null()`。
5. 对非法窗口、非升序窗口、非有限 close 或非法 previous state 返回 typed error。
6. 状态类型应是小型 `Copy` struct，函数参数优先使用 slice/reference，避免在单证券循环中克隆整段输入。

测试：

1. `macd_should_start_ema_from_sma_seed()`。
2. `macd_should_emit_first_dif_when_fast_and_slow_ema_available()`。
3. `macd_should_emit_first_dea_after_nine_valid_dif_values()`。
4. `macd_should_not_advance_state_when_close_is_null()`。
5. `macd_should_continue_from_previous_state_consistently()`。
6. `macd_should_reject_non_finite_close()`。

完成标准：

- `cargo test -p furnace-core macd` 通过。
- MACD 公式没有出现在其他 crate。

### 阶段 2：实现 `furnace-io` ClickHouse 边界

实施：

1. 新增 request/rows/summary/runner 模块。
2. 新增 output/staging DDL、drop staging、replace partition SQL。
3. 新增 RowBinary 写入顺序，并与 DDL 字段顺序保持一致。
4. 读取输入时按 `security_code, trade_date` 分组，单证券内升序递推。
5. 沿用现有 dry-run、append-latest、replace-cascade 模式。
6. 增量模式需读取历史 MACD 状态或足够 lookback，使结果与全历史一次性计算一致。
7. 对 state 不完整的证券，runner 必须回退到 warm-up lookback 或全历史重算；不得把缺失 `macd_dea_state` 当作 0 或跳过 DEA 启动。

测试：

1. DDL 包含目标表、年度分区和排序键。
2. staging 表名包含 run id 并可安全 drop。
3. request validation 拒绝生产写入的非默认 input/output/price column 或过小 batch。
4. RowBinary writer 输出字段顺序稳定。
5. summary JSON 包含 `indicator=macd`、参数、state source、affected years、validation 和 performance metrics。
6. summary JSON 包含 `incomplete_state_symbols_count` 或等价字段，用于观察增量状态不足导致的 lookback 回退。

完成标准：

- `cargo test -p furnace-io macd` 通过。
- production write path 不使用 ClickHouse UPDATE/DELETE mutation。

### 阶段 3：接入 `furnace` CLI

实施：

1. 新增 `furnace macd` 子命令。
2. 参数沿用 `rsi`/`boll`：`--from`、`--to`、`--mode`、`--symbols`、`--run-id`、`--input-table`、`--output-table`、`--price-column`、`--insert-batch-size`、`--output-format json`。
3. `--help` 输出包含 MACD。
4. CLI 输出 JSON summary，不输出额外非 JSON 文本。

测试：

1. CLI 能解析 dry-run MACD 请求。
2. 未知参数报 usage error。
3. JSON summary 中 `indicator` 为 `macd`。

完成标准：

- `cargo test -p furnace macd` 通过。
- `cargo test -p furnace cli` 不回归。

### 阶段 4：接入 Dagster calculation asset

实施：

1. 在 `resources/furnace.py` 新增 MACD request/result、`run_macd()` 和 command builder。
2. 在 `furnace/assets.py` 新增 `FurnaceMacdRunConfig` 和 `build_furnace_macd_asset()`。
3. 将 MACD asset 加入 `FURNACE_ASSETS`。
4. `_metadata_from_summary()` 增加 `macd_params`、`macd_state_source` 或通用 `state_source`、`histogram_mode` 等字段。
5. 如有 scheduler 单元测试覆盖 Furnace resource command builder，同步新增 MACD case。

完成标准：

- `uv run ruff check scheduler/src scheduler/tests` 通过。
- `uv run pyright scheduler/src/scheduler scheduler/tests` 通过。
- `uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing` 通过。
- `cd pipeline/scheduler && uv run dg check defs` 通过。

### 阶段 5：接入 dbt source 和 intermediate wrapper

实施：

1. 在 `sources_fleur_calculation.yml` 新增 `calc_stock_macd_daily`。
2. 新增 `int_stock_macd_daily.sql`，只从 `{{ source('fleur_calculation', 'calc_stock_macd_daily') }}` 选择业务字段。
3. 新增 `int_stock_macd_daily.yml`，包含 grain、business logic、主键唯一性测试、字段文档和状态列不暴露说明。
4. 如有必要，在 `docs/architecture/dbt_layer/fleur_intermediate/` 新增 `int_stock_macd_daily.md`。
5. 在 `docs/Q&A/int-layer-indicators-2026-06-10.md` 或后续正式指标盘点文档中，把 MACD 从“未覆盖”移入“技术指标”清单；若该 Q&A 仍为临时未跟踪文档，可在运行报告中记录未同步原因。

完成标准：

- `uv run dbt parse --project-dir elt --profiles-dir elt` 通过。
- `uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_macd_daily` 通过。
- `dbt show` 抽样确认字段、行数和 NULL warm-up 语义合理。

### 阶段 6：全链路验收和运行报告

验收顺序：

1. Rust 单元测试和 clippy。
2. Furnace CLI dry-run 小范围。
3. Dagster defs check。
4. dbt parse/build wrapper。
5. 小范围写入验证。
6. 全市场全历史 dry-run 或 production 准入 run。

运行报告：

```text
docs/jobs/reports/YYYY-MM-DD-furnace-macd-smoke-run.md
docs/jobs/reports/YYYY-MM-DD-furnace-macd-performance-baseline.md
```

报告至少记录：

1. 命令、时间、环境和 binary path。
2. request/effective/input 日期范围。
3. 输入行数、输出行数、有效 close 行数、null indicator 行数。
4. affected years、retained rows、staging validation、partition replace summary。
5. 性能指标、worker threads 和吞吐。
6. 抽样证券的第一条 DIF、DEA、histogram 日期，验证 SMA 启动语义。

## 7. 禁止模式

1. 禁止在 dbt SQL、Dagster Python 或 ClickHouse SQL 中计算 MACD。
2. 禁止 zero start；第一版必须使用 SMA 启动。
3. 禁止把 `macd_histogram` 定义为 `2 * (DIF - DEA)`。
4. 禁止生产写入非 canonical 参数。
5. 禁止在 production path 使用过小 insert batch。
6. 禁止使用 `ALTER TABLE UPDATE` / `ALTER TABLE DELETE` 做常规历史修正。
7. 禁止丢弃 close 为空的交易日输出行；指标不可用应保留 NULL。
8. 禁止让 mart 直接读取 `fleur_calculation.calc_stock_macd_daily`，必须通过 `int_stock_macd_daily` wrapper。
9. 禁止用 0、上一行 DIF 或上一行 DEA 伪造尚未启动的 DEA 状态；启动窗口不足时必须输出 NULL。

## 8. 最小验证命令

文档-only 阶段至少运行：

```bash
make docs-check
git diff --check
```

Rust 阶段：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Dagster 阶段：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

dbt 阶段：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_macd_daily
```

## 9. 完成标准

本计划完成时应满足：

1. `furnace macd` 可计算 canonical `MACD(12,26,9)`。
2. EMA(12)、EMA(26)、DEA(9) 均使用 SMA 启动，并有单元测试覆盖启动点、NULL 不推进和增量延续。
3. `fleur_calculation.calc_stock_macd_daily` DDL、staging、RowBinary、partition replace 和 summary 已实现。
4. Dagster 暴露 `fleur_calculation/calc_stock_macd_daily` asset，并保留 owner、kind、layer/storage/modality tags 和可核验 metadata。
5. dbt 暴露 `fleur_intermediate.int_stock_macd_daily` thin wrapper。
6. 所有 Rust、Dagster 和 dbt 定向验证通过。
7. 运行报告记录 smoke run 和性能基线。
8. 完成后更新 `0035`，使 `mart_stock_trend_indicator` 明确消费 `int_stock_macd_daily`。
9. 完成后将本计划移入 `docs/plans/archive/`，状态改为 `Archived`，并更新 `docs/plans/README.md`。
10. 0035 开始实施前，`int_stock_macd_daily` 已通过 dbt parse/build，且 source YAML、Dagster asset metadata 和 Furnace JSON summary 的字段名一致。
