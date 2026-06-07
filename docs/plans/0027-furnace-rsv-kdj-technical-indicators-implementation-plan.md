# Plan 0027: Furnace RSV/KDJ 日线技术指标实施方案

日期：2026-06-07

状态：Draft

关联文档：

- `docs/RFC/0016-rust-furnace-compute-engine.md`
- `docs/RFC/0015-dagster-dbt-asset-graph-integration.md`
- `docs/RFC/0014-clickhouse-layered-database-migration.md`
- `AGENTS.md`
- `engines/README.md`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.yml`
- `pipeline/elt/dbt_project.yml`

相关 skills：

- `fleur-harness`：计划、长期边界和质量门禁治理。
- `rust-best-practices` / `rust-patterns` / `rust-testing`：Rust crate 边界、错误处理、测试和质量检查。
- `using-dbt-for-analytics-engineering`：从目标输出 grain 反推输入模型、dbt 文档和 tests。
- `dagster-expert`：Dagster asset、job、schedule、metadata 和回填建模。
- `clickhouse-architecture-advisor` / `clickhouse-best-practices`：ClickHouse 表设计、批量写入、分区、重算和 mutation 策略。

## 1. 目标

使用 `engines/` Rust workspace 中的 `furnace` 计算引擎，从 dbt intermediate 层日线行情模型读取复权日线数据，计算 RSV 和 KDJ 指标，并写回 ClickHouse 外部计算产物层表：

```text
fleur_calculation.calc_stock_technical_indicators_daily
```

完成后应满足：

1. `furnace` 能按日期区间、证券代码集合和第一版固定参数计算 RSV/KDJ。
2. 输入默认来自 `fleur_intermediate.int_stock_quotes_daily_adj`。
3. Furnace 输出表 grain 为每证券、交易日一行。
4. Dagster 将输出表建模为 asset，并表达对 dbt intermediate 日线行情模型的真实依赖。
5. 重跑同一日期区间不会产生不可控重复数据；结果通过受控年度分区重建/替换实现幂等。
6. dbt 将 Furnace 输出包装为 `fleur_intermediate.int_stock_technical_indicators_daily`，再供 marts 层消费。
7. Rust、Dagster、dbt 和 ClickHouse 各自职责边界清晰，不把指标公式散落到 Python asset 或 dbt SQL 中。

## 2. 非目标

本计划不做以下事情：

1. 不一次性实现 MACD、RSI、布林线等其他指标。
2. 不实现实时流式指标计算或交易策略执行。
3. 不在本计划中创建或修改生产代码；本文档只定义实施方案。
4. 不改变 `int_stock_quotes_daily_adj` 的现有字段和复权逻辑。
5. 不把 `furnace-core` 绑定到 ClickHouse、Dagster 或 dbt。
6. 不让 dbt 执行 Rust 计算过程；dbt 只负责上下游建模、文档、tests 和 marts 消费。

## 3. 当前事实基线

### 3.1 Rust workspace

当前已存在最小 Rust workspace：

```text
engines/
├── Cargo.toml
├── Cargo.lock
├── README.md
└── crates/
    ├── furnace/
    ├── furnace-core/
    └── furnace-io/
```

当前状态是工程骨架，尚未包含业务逻辑、CLI 参数、ClickHouse I/O 或指标计算。

### 3.2 输入 dbt 模型

当前输入候选模型：

```text
pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql
pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.yml
```

模型事实：

- materialized 为 ClickHouse table。
- 物理 schema/database 为 `fleur_intermediate`。
- `order_by='(security_code, trade_date)'`。
- `partition_by='toYear(trade_date)'`。
- grain：每证券、交易日一行。
- 唯一键：`security_code + trade_date`。

可用字段：

| 字段 | 用途 |
|------|------|
| `security_code` | 证券代码 |
| `trade_date` | 交易日 |
| `high_price_forward_adj` | 默认 RSV 高价输入 |
| `low_price_forward_adj` | 默认 RSV 低价输入 |
| `close_price_forward_adj` | 默认 RSV 收盘价输入 |

第一版固定使用前复权字段，因为前复权价格更贴近技术分析常用的历史可比价格序列。第一版不支持后复权作为运行参数；如后续需要多价格口径，应通过新的方案重新设计表结构和唯一键。

### 3.3 dbt 层级路由

`pipeline/elt/dbt_project.yml` 当前配置：

```text
models/intermediate -> fleur_intermediate
models/marts        -> fleur_marts
```

本计划采用 `fleur_calculation -> dbt int wrapper` 两段式：

```text
fleur_calculation.calc_stock_technical_indicators_daily
  -> dbt source('fleur_calculation', 'calc_stock_technical_indicators_daily')
  -> dbt ref('int_stock_technical_indicators_daily')
  -> dbt marts
```

`calc_stock_technical_indicators_daily` 由 Furnace/Dagster 物化，`int_stock_technical_indicators_daily` 由 dbt 维护。这样保留 `int_*` 的 dbt intermediate 语义，同时让外部计算产物拥有独立 database、owner、权限和重算边界。

## 4. 指标定义

用户原始描述为“rsv kjd”。本文档按常见技术指标命名统一为 RSV/KDJ。

### 4.1 默认参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `rsv_window` | 9 | RSV 最高价/最低价滚动窗口 |
| `k_smoothing` | 3 | K 值平滑参数 |
| `d_smoothing` | 3 | D 值平滑参数 |
| `initial_k` | 50 | 仅在没有历史 K 状态时使用的启动值 |
| `initial_d` | 50 | 仅在没有历史 D 状态时使用的启动值 |

### 4.2 公式

对每个 `security_code` 按 `trade_date` 升序计算：

```text
lowest_low_n  = rolling_min(low_price, n)
highest_high_n = rolling_max(high_price, n)
if highest_high_n = lowest_low_n:
    rsv = 50
else:
    rsv = (close_price - lowest_low_n) / (highest_high_n - lowest_low_n) * 100
k = (2 / 3) * previous_k + (1 / 3) * rsv
d = (2 / 3) * previous_d + (1 / 3) * k
j = 3 * k - 2 * d
```

其中 `n = rsv_window`。当窗口完整且价格不为空，但 `highest_high_n = lowest_low_n` 时，按行业习惯将 RSV 填充为 50。当窗口不足或输入价格为空时，RSV/K/D/J 输出 `NULL`。第一版不使用行级 `is_valid` 字段；下游通过具体指标字段是否为 `NULL` 判断该指标是否可用。

K/D 的递推状态规则：

1. 日常增量计算必须读取同一 `security_code` 在目标区间之前最近一条有效历史 K、D，作为 `previous_k` 和 `previous_d`。
2. `initial_k = 50`、`initial_d = 50` 只用于没有任何历史 K、D 状态的空状态启动，例如全量首跑或新证券首个有效 RSV。
3. 回填重算一段历史区间时，如果目标区间之前存在有效历史状态，应优先从历史状态启动；如果为了可重复性选择从更早 lookback 区间重新推导状态，必须保证结果与读取上一条历史 K、D 的口径一致，并在运行 metadata 中记录状态来源。

### 4.3 Lookback

计算目标区间 `[from, to]` 时，输入读取区间必须向前扩展：

```text
input_from = from - max(rsv_window, warmup_window) 个交易日
input_to = to
```

第一版建议 `warmup_window >= 3 * max(rsv_window, k_smoothing, d_smoothing)`，并允许 CLI 参数覆盖。Dagster 传入 calendar partition 时，`furnace` 应基于交易日历或输入表实际交易日序列计算 lookback，而不是简单按自然日倒推。日常增量运行还必须读取目标区间之前最近一条有效 K、D 状态；lookback 用于构造滚动窗口和状态校验，不替代历史 K、D 状态读取。

## 5. 目标输出模型

### 5.1 表名

Furnace 直接写入表：

```text
fleur_calculation.calc_stock_technical_indicators_daily
```

dbt intermediate wrapper：

```text
fleur_intermediate.int_stock_technical_indicators_daily
```

### 5.2 Grain

每证券、交易日一行。第一版固定使用前复权口径和默认 KDJ 参数，不在结果表中写入口径或参数集字段。

```text
security_code
trade_date
```

### 5.3 字段草案

| 字段 | 类型建议 | 说明 |
|------|----------|------|
| `security_code` | `String` | 证券代码 |
| `trade_date` | `Date` | 交易日 |
| `rsv_window` | `UInt16` | RSV 窗口 |
| `k_smoothing` | `UInt16` | K 平滑参数 |
| `d_smoothing` | `UInt16` | D 平滑参数 |
| `rsv` | `Nullable(Float64)` | RSV 值 |
| `k_value` | `Nullable(Float64)` | K 值 |
| `d_value` | `Nullable(Float64)` | D 值 |
| `j_value` | `Nullable(Float64)` | J 值 |

说明：

- 指标值使用 `Nullable(Float64)` 是因为窗口不足、停牌/缺价时“无值”有实际语义。`highest_high_n = lowest_low_n` 不视为无值，RSV 按行业习惯填 50。Per `schema-types-avoid-nullable`，其他非指标字段应尽量使用非 Nullable 和默认值。
- 不设置行级 `is_valid` / `invalid_reason`。未来同一表扩展 MACD、RSI 等其他指标后，不同指标可能有不同可用性；行级有效标记会变得含混。指标不可用原因由 Dagster metadata、运行报告或后续专用质量统计表记录。
- 实际读取输入起止日期属于运行上下文，不进入业务结果表；由 Dagster materialization metadata、运行摘要和 `docs/jobs/reports/` 记录。
- 日期、数值、布尔字段应使用原生类型。Per `schema-types-native-types`，不得全部落成 String。
- `run_id`、`computed_at`、写入版本等运行审计信息不进入业务结果表，由 Dagster materialization metadata 和 `docs/jobs/reports/` 运行报告记录。

### 5.4 ClickHouse engine / partition / order 草案

工作负载归类：

- workload：market data / financial services。
- latency target：批处理日线指标，分钟级到小时级。
- data shape：按证券和交易日的宽表技术指标。
- primary query patterns：
  - 单证券时间序列：`WHERE security_code = ? AND trade_date BETWEEN ? AND ?`
  - 某交易日全市场横截面：`WHERE trade_date = ?`
- operational constraints：需要回填、重算、晚到行情/复权因子变更处理。

第一版建议：

```text
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)
```

依据：

- Per `schema-pk-plan-before-creation`：`ORDER BY` 建表后修改成本高，必须先按查询模式规划。
- Per `schema-pk-prioritize-filters`：`trade_date` 和 `security_code` 是主要过滤字段。
- Per `schema-pk-cardinality-order`：第一版更偏每日全市场横截面，`trade_date` 放在 `ORDER BY` 前部，`security_code` 支撑同日证券明细和单证券回查。
- Per `decision-partitioning-timeseries` 与 `schema-partition-low-cardinality`：分区应保持低基数并服务生命周期管理。第一版采用年分区，与上游 `int_stock_quotes_daily_adj` 的 `toYear(trade_date)` 对齐；A 股日频技术指标单年数据量可控，年分区比月分区减少 part 和分区管理开销。
- Per `decision-late-arriving-upserts` 与 `insert-mutation-avoid-update`：重算和晚到修正不使用高频 `ALTER TABLE UPDATE`；第一版不保留版本字段，采用 staging table + 受影响年份分区重建/替换协议保证幂等。
- Per `insert-batch-size`：写入批次应至少 1,000 行，理想 10,000-100,000 行。

如果后续主要查询转为单证券长时间序列，应重新评估 `ORDER BY (security_code, trade_date)`。由于 ClickHouse `ORDER BY` 修改成本高，该变化必须通过新迁移计划处理。

年分区重算协议：

1. 识别目标日期区间覆盖的年份集合。
2. 对每个受影响年份，构建 staging table：保留目标表中该年份但不在重算区间内的旧数据，合并 Furnace 新计算出的目标区间数据。
3. 校验 staging table 的唯一键、row count、日期范围和抽样结果。
4. 使用受控 `REPLACE PARTITION` 或等价封装替换整年分区。
5. 在 Dagster metadata 和 `docs/jobs/reports/` 记录受影响年份、输入区间、输出行数和分区替换结果。

## 6. 方案边界

### 6.1 主方案：Furnace 写 calculation，dbt 包装 intermediate

主方案：

```text
dbt int_stock_quotes_daily_adj
  -> Dagster asset dependency
  -> furnace CLI
  -> fleur_calculation.calc_stock_technical_indicators_daily
  -> dbt int_stock_technical_indicators_daily
  -> dbt marts
```

优点：

1. `fleur_calculation` 明确表达 Furnace/Dagster 是直接写入 owner。
2. `int_stock_technical_indicators_daily` 仍由 dbt 维护，符合 `int_*` 命名直觉。
3. dbt lineage、docs 和 tests 更自然，下游 marts 可以继续 `ref()` dbt intermediate wrapper。
4. 计算产物的运行审计和重算策略由 Dagster metadata、job report 和受控替换协议承载，不污染 dbt-owned intermediate 命名边界。

约束：

1. 需要在 ClickHouse 中新增 `fleur_calculation` database；该决策已纳入 ADR 0009。
2. dbt 必须先以 source 声明 `fleur_calculation.calc_stock_technical_indicators_daily`，再用 thin wrapper model 产出 `int_stock_technical_indicators_daily`。
3. source YAML 应通过 `meta.dagster.asset_key` 或等价配置映射到 Furnace/Dagster 产出的计算资产，保证 Dagster dbt 集成能展示跨工具 lineage。
4. dbt tests 覆盖 `int_stock_technical_indicators_daily` wrapper；物理表创建、写入、分区替换和运行审计由 Rust/Dagster/ClickHouse 路径负责。

### 6.2 dbt / Dagster 接入契约

本计划明确采用 `source + Dagster asset metadata + thin wrapper`，三者职责如下：

| 机制 | 职责 | 不承担 |
|------|------|--------|
| dbt source | 声明 `fleur_calculation` 物理计算表、建立 dbt 外部输入边界、承载 source docs 和基础 source tests | 不作为 marts 的直接消费接口 |
| Dagster asset metadata | 记录 Furnace 运行上下文、输入/输出行数、目标日期范围、lookback 范围、受影响年度分区、参数和替换结果 | 不替代 dbt 的 `source()` / `ref()` DAG |
| thin wrapper model | 在 `fleur_intermediate` 中提供稳定 `int_*` 语义、字段文档、面向消费的 tests 和后续兼容层 | 不重写 RSV/KDJ 公式，不直接管理 Furnace 写入 |

建议的 dbt source 形态：

```yaml
sources:
  - name: fleur_calculation
    schema: fleur_calculation
    tables:
      - name: calc_stock_technical_indicators_daily
        description: Furnace/Dagster 物化的日线技术指标计算产物。
        meta:
          dagster:
            asset_key: ["fleur_calculation", "calc_stock_technical_indicators_daily"]
```

建议的 dbt thin wrapper 形态：

```sql
select
    security_code,
    trade_date,
    rsv_window,
    k_smoothing,
    d_smoothing,
    rsv,
    k_value,
    d_value,
    j_value
from {{ source('fleur_calculation', 'calc_stock_technical_indicators_daily') }}
```

第一版 wrapper 建议 materialized 为 view。只有当 marts 查询 SLA 或重复扫描成本证明 view 不够时，才通过后续计划改为 table 或 incremental。

### 6.3 不采用方案：Furnace 直接写 `int_*`

不采用：

```text
furnace -> fleur_intermediate.int_stock_technical_indicators_daily
```

问题：

1. `int_*` 命名会暗示 dbt intermediate ownership，但实际由外部计算引擎写入。
2. dbt lineage、docs 和 tests 需要额外例外规则。
3. 后续每个 agent 和开发者都需要记住 `int_*` 中存在非 dbt 维护表，维护成本高。

本计划不采用该方案。

## 7. 实施阶段

### 阶段 1：接口和数据契约冻结

目标：

1. 确认 `int_stock_quotes_daily_adj` 的输入字段和默认前复权口径。
2. 冻结 RSV/KDJ 参数和无效值处理规则。
3. 冻结 `fleur_calculation.calc_stock_technical_indicators_daily` 字段、grain、engine、partition、order 和替换协议。
4. 明确 `calc_stock_technical_indicators_daily` 的 owner 为 Furnace/Dagster，`int_stock_technical_indicators_daily` 的 owner 为 dbt。
5. 冻结 dbt 消费方式：`source('fleur_calculation', 'calc_stock_technical_indicators_daily')` + Dagster asset metadata + `int_stock_technical_indicators_daily` thin wrapper。

产出：

- 更新 RFC 或 ADR（如需要）记录 `fleur_calculation` 计算产物层。
- 目标表 DDL 草案。
- dbt source YAML 草案、thin wrapper SQL 草案和 wrapper YAML tests/docs 草案。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
```

### 阶段 2：Rust 纯计算核心

目标：

1. 在 `furnace-core` 中定义日线输入 record、KDJ 参数和输出 record。
2. 实现按单证券有序序列计算 RSV/KDJ 的纯函数。
3. 不引入 ClickHouse、Dagster、dbt 依赖。
4. 使用固定 fixture 覆盖窗口不足、分母为 0 时 RSV=50、空价格、正常递推、读取上一条历史 K/D 和重算一致性。

产出：

- `furnace-core` 纯计算 API。
- golden fixture 或 snapshot tests。

验证：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### 阶段 3：Furnace CLI 和 ClickHouse I/O

目标：

1. 在 `furnace` 中提供 `kdj` 子命令。
2. 支持 `--from`、`--to`、`--symbols`、`--rsv-window`、`--k-smoothing`、`--d-smoothing`、`--run-id`。
3. 在 `furnace-io` 中实现从 ClickHouse 读取输入区间和批量写入结果。
4. 写入批次遵守 `insert-batch-size`，目标 10,000-100,000 行。
5. 输出运行摘要：输入行数、输出行数、证券数量、目标区间、实际 lookback 区间、批次数。

产出：

- `furnace kdj ...` CLI。
- ClickHouse read/write adapter。
- 本地 dry-run 或 fixture-run 能力。

验证：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### 阶段 4：Dagster asset 集成

目标：

1. 在 scheduler 中新增 Furnace 技术指标 asset。
2. 用 asset dependency 表达对 dbt `int_stock_quotes_daily_adj` 的依赖。
3. 将日期分区、run id 和参数传入 CLI。
4. 捕获 CLI 输出并返回 `MaterializeResult` metadata。
5. 提供定向 job，避免运行整个 dbt 或全部指标。
6. 计算资产的 asset key 应与 dbt source 的 `meta.dagster.asset_key` 保持一致，便于 Dagster dbt 集成将 dbt wrapper 的上游 source 映射回 Furnace 资产。

Dagster 建模建议：

- 使用 asset 表达目标表，而不是普通 task。
- 输出 asset key 应能明显区分非 dbt intermediate 外部资产。
- `MaterializeResult` metadata 至少包含目标日期范围、lookback 范围、证券数量、输入行数、输出行数、受影响年度分区、KDJ 参数、历史 K/D 状态来源和 ClickHouse 分区替换结果。
- 初始自动化优先使用固定 schedule；如果后续需要依赖上游 materialization，再评估 asset sensor 或 declarative automation。该选择符合 Dagster automation reference：固定时间用 schedule，依赖 asset 状态用 declarative automation 或 asset sensor。

验证：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests
cd scheduler
uv run dg check defs
```

### 阶段 5：dbt marts 消费和文档测试

目标：

1. 在 dbt sources YAML 中声明 `fleur_calculation.calc_stock_technical_indicators_daily`，并配置 Dagster asset key metadata。
2. 在 dbt 中将 source 包装为 `int_stock_technical_indicators_daily` thin wrapper，第一版默认 materialized 为 view。
3. 为 marts 层新增消费 `int_stock_technical_indicators_daily` 的模型；marts 不直接读取 `fleur_calculation.*`。
4. 在 dbt YAML 中记录 grain、固定前复权口径、默认 KDJ 参数和 `NULL` 值语义。
5. 添加高价值 tests：
   - `security_code + trade_date` 唯一。
   - `security_code` 非空。
   - `trade_date` 非空。
6. 避免把 Furnace 物理表创建和写入逻辑搬进 dbt。

source 层测试应偏基础存在性和 schema 文档；wrapper 层测试面向下游消费契约。指标字段 `rsv`、`k_value`、`d_value`、`j_value` 允许为 `NULL`，不做全表 not null。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_layer_routing.py
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --project-dir elt --profiles-dir elt --select <target_mart_or_wrapper>
```

### 阶段 6：回填和运行报告

目标：

1. 对小范围证券和日期做 smoke run。
2. 对一个完整月做回填验证。
3. 记录运行命令、输入区间、输出行数、ClickHouse part 数、抽样校验和问题。

产出：

- `docs/jobs/reports/<date>-furnace-kdj-smoke-run.md`
- `docs/jobs/reports/<date>-furnace-kdj-backfill.md`

验收查询示例：

```sql
SELECT
    count() AS rows,
    uniqExact(security_code) AS securities,
    min(trade_date) AS min_trade_date,
    max(trade_date) AS max_trade_date
FROM fleur_calculation.calc_stock_technical_indicators_daily;
```

```sql
SELECT
    partition,
    count() AS parts,
    sum(rows) AS rows
FROM system.parts
WHERE database = 'fleur_calculation'
  AND table = 'calc_stock_technical_indicators_daily'
  AND active
GROUP BY partition
ORDER BY partition;
```

dbt wrapper 另行验证：

```sql
SELECT
    count() AS rows,
    uniqExact(security_code) AS securities,
    min(trade_date) AS min_trade_date,
    max(trade_date) AS max_trade_date
FROM fleur_intermediate.int_stock_technical_indicators_daily;
```

## 8. 禁止模式

1. 不在 Dagster Python asset 中实现 RSV/KDJ 公式。
2. 不在 dbt SQL 中重复实现 RSV/KDJ 作为第一版主路径。
3. 不对 ClickHouse 目标表执行高频 `ALTER TABLE UPDATE`。
4. 不逐行 insert 或使用过小 batch 写入指标结果。
5. 不在 `furnace-core` 中读取环境变量、连接 ClickHouse 或调用 Dagster。
6. 不在 scheduler definitions 加载阶段连接 ClickHouse 或执行 Furnace CLI。
7. 不让下游 mart 直接依赖未过滤版本的重复结果。
8. 不让 Furnace 直接写入 `fleur_intermediate.int_stock_technical_indicators_daily`。
9. 不让 marts 直接 `source()` 或硬编码查询 `fleur_calculation.*`；marts 必须通过 `ref('int_stock_technical_indicators_daily')` 消费。

## 9. 最小验证命令汇总

Rust：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

dbt：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_layer_routing.py
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --project-dir elt --profiles-dir elt --select <target_mart_or_wrapper>
```

Dagster：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests
cd scheduler
uv run dg check defs
```

文档-only 变更：

```bash
git diff --check
```

## 10. 已决策项

1. dbt 消费 `fleur_calculation.calc_stock_technical_indicators_daily` 采用 `source + Dagster asset metadata + thin wrapper model`。source 是 dbt 外部输入边界，Dagster metadata 是运行观测和跨工具 lineage，thin wrapper 是 `fleur_intermediate` 稳定消费契约。
