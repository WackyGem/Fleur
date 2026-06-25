# Plan 0027: Furnace RSV/KDJ 日线技术指标实施方案

日期：2026-06-07

状态：Archived

归档日期：2026-06-10

归档原因：Completed

关联文档：

- `docs/RFC/archive/0016-rust-furnace-compute-engine.md`
- `docs/RFC/archive/0015-dagster-dbt-asset-graph-integration.md`
- `docs/RFC/archive/0014-clickhouse-layered-database-migration.md`
- `AGENTS.md`
- Dagster 官方文档：software-defined assets、Dagster Pipes external pipelines、external assets、custom integrations/components、asset jobs/schedules、`MaterializeResult` metadata、`dg check`
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
fleur_calculation.calc_stock_kdj_daily
```

完成后应满足：

1. `furnace` 能按日期区间、证券代码集合和第一版固定参数计算 RSV/KDJ。
2. 输入默认来自 `fleur_intermediate.int_stock_quotes_daily_adj`。
3. Furnace 输出表 grain 为每证券、交易日一行。
4. Dagster 将输出表建模为 asset，并表达对 dbt intermediate 日线行情模型的真实依赖。
5. 重跑同一日期区间不会产生不可控重复数据；结果通过受控年度分区重建/替换实现幂等。
6. dbt 将 Furnace 输出包装为 `fleur_intermediate.int_stock_kdj_daily`，再供 marts 层消费。
7. Rust、Dagster、dbt 和 ClickHouse 各自职责边界清晰，不把指标公式散落到 Python asset 或 dbt SQL 中。
8. 历史回填必须保持 K/D 递推状态连续；任何影响历史 K/D 的重算都要级联到后续受影响交易日，不能只替换请求区间。

## 2. 非目标

本计划不做以下事情：

1. 不一次性实现 MACD、RSI、布林线等其他指标。
2. 不实现实时流式指标计算或交易策略执行。
3. 不在本计划中创建或修改生产代码；本文档只定义实施方案。
4. 不改变 `int_stock_quotes_daily_adj` 的现有字段和复权逻辑。
5. 不把 `furnace-core` 绑定到 ClickHouse、Dagster 或 dbt。
6. 不让 dbt 执行 Rust 计算过程；dbt 只负责上下游建模、文档、tests 和 marts 消费。
7. 不把本计划范围内的 Furnace 结果建模为 Dagster 只观测的 external asset；第一版由 Dagster 触发 Rust CLI，因此应是可物化 asset。
8. 不在第一版强制引入 Rust 原生 Dagster SDK 或让 Rust 进程直接调用 Dagster API 上报物化事件；物化事件由 Python scheduler asset 返回。若后续采用 Dagster Pipes，应由 scheduler 侧 client 注入上下文，Rust CLI 仅消费环境变量/参数并输出结构化事件。

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
fleur_calculation.calc_stock_kdj_daily
  -> dbt source('fleur_calculation', 'calc_stock_kdj_daily')
  -> dbt ref('int_stock_kdj_daily')
  -> dbt marts
```

`calc_stock_kdj_daily` 由 Furnace/Dagster 物化，`int_stock_kdj_daily` 由 dbt 维护。这样保留 `int_*` 的 dbt intermediate 语义，同时让外部计算产物拥有独立 database、owner、权限和重算边界。

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
k = ((k_smoothing - 1) / k_smoothing) * previous_k + (1 / k_smoothing) * rsv
d = ((d_smoothing - 1) / d_smoothing) * previous_d + (1 / d_smoothing) * k
j = 3 * k - 2 * d
```

其中 `n = rsv_window`。当窗口完整且价格不为空，但 `highest_high_n = lowest_low_n` 时，按行业习惯将 RSV 填充为 50。当窗口不足或输入价格为空时，RSV/K/D/J 输出 `NULL`。第一版不使用行级 `is_valid` 字段；下游通过具体指标字段是否为 `NULL` 判断该指标是否可用。

价格输入和 K/D 递推状态规则：

1. 滚动窗口只使用同一 `security_code`、按 `trade_date` 升序排列且 `high_price_forward_adj`、`low_price_forward_adj`、`close_price_forward_adj` 都非空的有效价格记录。
2. 输入行价格为空、`high < low` 或无法形成完整有效窗口时，该交易日 RSV/K/D/J 输出 `NULL`，且不推进 K/D 历史状态。
3. 日常增量计算必须读取同一 `security_code` 在目标区间之前最近一条有效历史 K、D，作为 `previous_k` 和 `previous_d`。
4. `initial_k = 50`、`initial_d = 50` 只用于没有任何历史 K、D 状态的空状态启动，例如全量首跑或新证券首个有效 RSV。
5. K/D 是无限递推状态。历史区间中任意一日的 RSV/K/D 变化都会影响该证券后续所有有效 K/D/J；因此历史回填不能只重算请求区间，必须从有效重算起点级联计算到该证券当前最新输入交易日，或明确标记为不写入生产表的实验运行。
6. 回填重算一段历史区间时，如果有效重算起点之前存在有效历史状态，应优先从历史状态启动；如果为了可重复性选择从更早 lookback 区间重新推导状态，必须保证结果与读取上一条历史 K、D 的口径一致，并在运行 metadata 中记录状态来源。

### 4.3 Lookback

计算请求区间 `[from, to]` 时，输入读取区间必须向前扩展：

```text
input_from = from - max(rsv_window, warmup_window) 个交易日
input_to = effective_output_to
```

其中 `effective_output_to` 不是简单等于请求 `to`：

- 日常追加增量：如果请求区间之后没有已物化结果，`effective_output_to = to`。
- 历史回填或修正：`effective_output_to` 必须扩展到受影响证券当前可用输入的最新交易日，确保递推状态被级联修正。
- 如果只想重算历史片段但不级联后续结果，只允许作为 dry-run / 实验输出，不写入 `fleur_calculation.calc_stock_kdj_daily`。

第一版建议 `warmup_window >= 3 * max(rsv_window, k_smoothing, d_smoothing)`，并允许 CLI 参数覆盖。Dagster 传入请求日期区间时，`furnace` 应基于交易日历或输入表实际交易日序列计算 lookback，而不是简单按自然日倒推。日常增量运行还必须读取目标区间之前最近一条有效 K、D 状态；lookback 用于构造滚动窗口和状态校验，不替代历史 K、D 状态读取。

## 5. 目标输出模型

### 5.1 表名

Furnace 直接写入表：

```text
fleur_calculation.calc_stock_kdj_daily
```

dbt intermediate wrapper：

```text
fleur_intermediate.int_stock_kdj_daily
```

### 5.2 Grain

每证券、交易日一行。第一版固定使用前复权口径和默认 KDJ 参数，生产写入只允许 `rsv_window=9`、`k_smoothing=3`、`d_smoothing=3` 这一组 canonical 参数。

```text
security_code
trade_date
```

字段中仍保留 `rsv_window`、`k_smoothing`、`d_smoothing`，用于让结果自描述并辅助下游核验；但它们不是第一版业务 grain 的一部分。生产 CLI / Dagster asset 必须校验参数等于默认值。若后续要在同一表中保存多组参数，必须重新设计唯一键、dbt tests 和下游消费口径，例如改为 `security_code + trade_date + rsv_window + k_smoothing + d_smoothing` 或引入新的参数化结果表。

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

```sql
CREATE TABLE IF NOT EXISTS fleur_calculation.calc_stock_kdj_daily
(
    security_code String,
    trade_date Date,
    rsv_window UInt16,
    k_smoothing UInt16,
    d_smoothing UInt16,
    rsv Nullable(Float64),
    k_value Nullable(Float64),
    d_value Nullable(Float64),
    j_value Nullable(Float64)
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code);
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

1. 先把请求区间 `[from, to]` 扩展为实际写入区间 `[effective_output_from, effective_output_to]`。`effective_output_from = from`，但 Furnace 读取会额外包含 lookback 和上一条历史 K/D 状态；历史回填时 `effective_output_to` 必须级联到受影响证券当前最新输入交易日。
2. 识别实际写入区间覆盖的年份集合，而不是只看原始请求区间。
3. 对每个受影响年份，构建与目标表同 schema、同 engine、同 partition key 的 staging table。
4. staging table 必须包含该年份完整分区内容：
   - 受影响证券在实际写入区间内使用 Furnace 新计算结果。
   - 非受影响证券保留目标表旧数据。
   - 受影响证券在该年份但实际写入区间外的旧数据按需保留；如果递推级联覆盖到年底，则该证券年底前不应保留旧值。
5. 校验 staging table 的唯一键、row count、日期范围、参数常量、受影响证券集合和抽样结果。
6. 使用受控 `REPLACE PARTITION` 或等价封装逐年替换整年分区；不得使用高频 `ALTER TABLE UPDATE` / `ALTER TABLE DELETE` 修补单行结果。
7. 替换完成后清理 staging table，并在 Dagster metadata 和 `docs/jobs/reports/` 记录受影响年份、请求区间、实际写入区间、输入区间、输出行数、保留旧行数、替换结果和 staging 校验摘要。

该协议的关键约束是：ClickHouse `REPLACE PARTITION` 是分区级替换，不是行级 upsert。只重算部分证券或部分日期时，staging table 必须补齐同年未重算的旧行，否则会在替换分区时误删数据。

## 6. 方案边界

### 6.1 主方案：Furnace 写 calculation，dbt 包装 intermediate

主方案：

```text
dbt int_stock_quotes_daily_adj
  -> Dagster asset dependency
  -> furnace CLI
  -> fleur_calculation.calc_stock_kdj_daily
  -> dbt int_stock_kdj_daily
  -> dbt marts
```

优点：

1. `fleur_calculation` 明确表达 Furnace/Dagster 是直接写入 owner。
2. `int_stock_kdj_daily` 仍由 dbt 维护，符合 `int_*` 命名直觉。
3. dbt lineage、docs 和 tests 更自然，下游 marts 可以继续 `ref()` dbt intermediate wrapper。
4. 计算产物的运行审计和重算策略由 Dagster metadata、job report 和受控替换协议承载，不污染 dbt-owned intermediate 命名边界。

约束：

1. 需要在 ClickHouse 中新增 `fleur_calculation` database；该决策已纳入 ADR 0009。
2. dbt 必须先以 source 声明 `fleur_calculation.calc_stock_kdj_daily`，再用 thin wrapper model 产出 `int_stock_kdj_daily`。
3. source YAML 应通过 `meta.dagster.asset_key` 或等价配置映射到 Furnace/Dagster 产出的计算资产，保证 Dagster dbt 集成能展示跨工具 lineage。
4. dbt tests 覆盖 `int_stock_kdj_daily` wrapper；物理表创建、写入、分区替换和运行审计由 Rust/Dagster/ClickHouse 路径负责。

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
      - name: calc_stock_kdj_daily
        description: Furnace/Dagster 物化的日线技术指标计算产物。
        meta:
          dagster:
            asset_key: ["fleur_calculation", "calc_stock_kdj_daily"]
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
from {{ source('fleur_calculation', 'calc_stock_kdj_daily') }}
```

第一版 wrapper 建议 materialized 为 view。只有当 marts 查询 SLA 或重复扫描成本证明 view 不够时，才通过后续计划改为 table 或 incremental。

### 6.3 不采用方案：Furnace 直接写 `int_*`

不采用：

```text
furnace -> fleur_intermediate.int_stock_kdj_daily
```

问题：

1. `int_*` 命名会暗示 dbt intermediate ownership，但实际由外部计算引擎写入。
2. dbt lineage、docs 和 tests 需要额外例外规则。
3. 后续每个 agent 和开发者都需要记住 `int_*` 中存在非 dbt 维护表，维护成本高。

本计划不采用该方案。

### 6.4 Dagster 官方模式映射

按 Dagster 官方资产建模建议，本计划采用“Dagster 物化 asset + Rust CLI 外部计算”的主路径：

1. `fleur_calculation.calc_stock_kdj_daily` 是一个 Dagster 可物化 asset，而不是普通 task。Dagster 负责触发 Rust CLI、捕获结果摘要并记录物化事件。
2. 上游 dbt intermediate 模型只作为数据依赖和 lineage，不通过 IOManager 把数据加载进 Python；因此依赖使用 `deps=` / `AssetKey`，而不是 asset 函数参数。
3. Rust/Furnace 当前没有现成 `dagster-*` 集成库。按 Dagster integration/component 取舍，长期接入形态应封装为自定义 Furnace component，由 component 生成 asset、job 和 schedule definitions，避免把 CLI 调用协议散落在多个 Python asset 中。
4. 第一版 component 只生成一个 materializable asset 和对应定向 job/schedule；不做 state-backed component，因为目标资产定义不需要从外部系统动态发现。
5. `AssetSpec` / external asset 只适用于 Furnace 完全脱离 Dagster 自行运行、Dagster 只记录外部物化事件的模式。本计划不采用该模式；若未来 Furnace 由独立服务、Kubernetes CronJob 或其他编排系统触发，再评估 external asset sensor 或 Dagster REST materialization reporting。
6. 每次运行返回 `MaterializeResult`，用 metadata 记录运行上下文；业务结果表不增加 `run_id`、`computed_at` 或输入区间字段。
7. Dagster 官方外部进程接入优先推荐 Dagster Pipes，用于把外部进程的日志、metadata 和 materialization 事件带回 Dagster。Furnace 第一版可以先使用稳定 JSON summary 协议，但协议边界必须按 Pipes 可迁移方式设计：结构化 metadata 与自由文本日志分离、退出码表达失败、敏感环境变量不回显。

建议的 Dagster asset key：

```text
AssetKey(["fleur_calculation", "calc_stock_kdj_daily"])
```

建议的上游 dbt asset key：

```text
AssetKey(["int_stock_quotes_daily_adj"])
```

该上游 key 与当前 `FleurDbtProjectComponent` 的 dbt model key 规则一致：dbt model asset key 使用模型名本身，不包含 database/schema 前缀。dbt source YAML 中的 `meta.dagster.asset_key` 必须与 Furnace asset key 保持一致，才能让 dbt wrapper 的 source lineage 映射回 Furnace 计算资产。

### 6.5 Dagster 与 Rust CLI 边界

Dagster 侧只负责 orchestration、配置、观测和失败语义，不实现指标公式，也不直接写指标结果表。

建议新增的 scheduler 边界：

```text
pipeline/scheduler/src/scheduler/
├── components/
│   └── furnace.py                 # 自定义 Furnace component 类型
└── defs/
    └── furnace/
        └── defs.yaml              # Furnace component instance
```

如果第一版暂不落 component，也必须把 CLI 调用封装为 `FurnaceCliResource` 或等价 service，再由 asset 薄调用；不得在 asset 函数内拼接复杂命令、读取环境变量或解析 ClickHouse 连接细节。后续稳定后再升级为 component 时，资源和服务协议应能复用。

Rust CLI 与 Dagster 的最小协议：

1. Dagster 传入 `--from`、`--to`、`--mode`、canonical KDJ 参数、`--run-id` 和可选证券列表。
2. Rust CLI 从环境变量或参数读取 ClickHouse 连接配置；连接配置由 scheduler resource 注入进子进程环境，不由 asset 直接读 `.env`。
3. Rust CLI 成功时 stdout 输出单个 JSON summary；stderr 用于日志。Dagster 只解析 stdout JSON，不从自由文本日志中提取事实。
4. JSON summary 字段至少包含：`request_from`、`request_to`、`effective_output_from`、`effective_output_to`、`input_from`、`input_to`、`mode`、`symbols_count`、`input_rows`、`output_rows`、`null_indicator_rows`、`affected_years`、`retained_rows`、`staging_table`、`staging_validation`、`partition_replace`、`kdj_params`、`state_source`。
5. Rust CLI 失败时返回非零退出码，并在 stderr 输出错误摘要；Dagster asset 应让 run fail，不记录成功物化。
6. Dagster metadata 中可记录命令摘要，但不得记录密码、token 或完整连接串。

### 6.6 Dagster Pipes 兼容接入策略

Dagster 官方对外部 pipeline 的推荐方向是使用 Dagster Pipes：由 Dagster 进程打开 Pipes session、启动外部进程，并把外部进程产生的日志、metadata 或 materialization 信息回传到 Dagster run。对 Furnace/Rust 的落地建议如下：

1. 接入形态分三层：
   - `FurnaceKdjComponent`：只负责根据 YAML/config 生成 asset、job、schedule 和 resource 绑定。
   - `FurnaceCliResource` 或 Pipes client resource：只负责构造命令、注入环境变量、启动 Rust 进程、处理 timeout、捕获 stdout/stderr、解析结构化结果。
   - Rust `furnace kdj`：只负责参数校验、ClickHouse I/O、指标计算、staging/partition replace 和输出结构化 summary。
2. 如果当前 Dagster 版本和依赖已经提供可用的 Pipes subprocess client，阶段 4 实施时优先用 Pipes client 启动 Rust CLI，并让 resource 将 CLI summary 转换为 Dagster metadata。
3. 如果 Rust 侧暂不引入 `dagster_pipes` 协议实现，保留 JSON summary 作为兼容层。JSON summary 字段命名、metadata 粒度和失败语义必须与 Dagster `MaterializeResult` metadata 一一对应，避免后续迁移 Pipes 时重新定义运行协议。
4. Rust 进程的普通日志只写 stderr；stdout 保留给 JSON summary 或 Pipes 事件通道。Dagster 不从自由文本日志中推导业务事实。
5. 外部进程可以知道 `DAGSTER_RUN_ID` 或 `--run-id` 用于日志关联，但不得直接调用 Dagster API 写 materialization。生产物化事件仍由 scheduler asset 统一返回，除非未来明确切换为 Furnace 独立编排模式。
6. Pipes/JSON 二选一不是业务语义差异，只是传输协议差异。无论采用哪种传输协议，asset key、上游依赖、run config、ClickHouse 写入协议和验收标准保持一致。

迁移顺序建议：

1. 第一阶段使用 `FurnaceCliResource + JSON summary` 完成最小可运行闭环。
2. 稳定后在不改变 Rust 计算 API 和 ClickHouse 写入协议的前提下，将 resource 内部启动方式替换为 Dagster Pipes client。
3. 只有当 Rust 侧需要主动推送多段 metadata、日志或中间进度时，再评估实现 Rust Pipes 事件写入协议；否则保持“Rust 输出 summary，Python asset 返回 materialization”的简单边界。

### 6.7 Dagster 分区、作业和自动化策略

第一版不把 `calc_stock_kdj_daily` 建成 Dagster 分区资产，原因是 K/D 递推状态会让历史回填写出请求日期之外的后续交易日；如果用 daily partition 表达结果切片，容易产生“只物化了某日 partition，但实际替换了多年分区”的语义错位。

第一版采用非分区 asset + typed run config：

```text
request_from
request_to
mode = append-latest | replace-cascade | dry-run
symbols = optional list
rsv_window = 9
k_smoothing = 3
d_smoothing = 3
```

作业建议：

| Job | 用途 | 选择范围 |
|-----|------|----------|
| `furnace__kdj_daily_job` | 日常追加最新交易日或小范围增量 | 只选择 `fleur_calculation/calc_stock_kdj_daily` |
| `furnace__kdj_backfill_job` | 显式历史回填和级联替换 | 同一 asset，必须传 `replace-cascade` config |
| `furnace__kdj_dry_run_job` | 验证参数、证券集合和输出摘要，不写生产表 | 同一 asset，必须传 `dry-run` config |

自动化建议：

1. 初始使用固定 schedule，在 dbt daily build 完成后留出时间窗口运行 `furnace__kdj_daily_job`。
2. schedule 的 run config 只表达请求日期或“最新可用交易日”策略；实际交易日解析和 lookback 仍由 Furnace/ClickHouse 输入事实决定。
3. 如果后续要在上游 dbt materialization 后立刻触发，再评估 asset sensor 或 Dagster declarative automation。
4. 历史回填不挂自动 schedule；只通过手动 launch 或回填 runbook 执行，并必须生成 `docs/jobs/reports/` 运行报告。

运行前置校验：

1. `int_stock_quotes_daily_adj` 在请求区间、lookback 区间和历史状态读取所需区间内有输入数据。
2. 生产模式参数必须为 `9/3/3`。
3. `append-latest` 模式要求请求区间之后没有同证券已物化结果；否则必须切换 `replace-cascade`。
4. `replace-cascade` 模式必须能计算受影响证券的 `effective_output_to`，并确认 staging 分区会补齐同年应保留旧行。

## 7. 实施阶段

### 阶段 1：接口和数据契约冻结

目标：

1. 确认 `int_stock_quotes_daily_adj` 的输入字段和默认前复权口径。
2. 冻结 RSV/KDJ 参数和无效值处理规则。
3. 冻结 `fleur_calculation.calc_stock_kdj_daily` 字段、grain、engine、partition、order 和替换协议。
4. 明确 `calc_stock_kdj_daily` 的 owner 为 Furnace/Dagster，`int_stock_kdj_daily` 的 owner 为 dbt。
5. 冻结 dbt 消费方式：`source('fleur_calculation', 'calc_stock_kdj_daily')` + Dagster asset metadata + `int_stock_kdj_daily` thin wrapper。
6. 冻结生产参数策略：第一版只写入 canonical `9/3/3` 参数；非默认参数只能 dry-run 或写入另行设计的表。
7. 冻结递推级联策略：历史回填必须扩展实际写入区间，直到受影响证券的最新可用输入交易日。

产出：

- 更新 RFC 或 ADR（如需要）记录 `fleur_calculation` 计算产物层。
- 目标表 DDL 草案和 staging partition replace 协议。
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
5. 将 RSV/KDJ 等指标实现为可复用函数库 API，而不是只服务 CLI 的内部函数。
6. 为公共 API 编写 rustdoc 文档和可执行 doctest，说明公式、参数、边界行为、`NULL` / `None` 语义和增量状态规则。

建议模块结构：

```text
engines/crates/furnace-core/src/
├── lib.rs
└── indicators/
    ├── mod.rs
    ├── rsv.rs
    └── kdj.rs
```

建议 API 分层：

```rust
pub mod indicators;

pub use indicators::kdj::{
    KdjInput,
    KdjOutput,
    KdjParams,
    KdjState,
    calculate_kdj_next,
    calculate_kdj_series,
};
```

其中：

- `calculate_kdj_series` 面向全量或回填场景，输入单证券按交易日升序排列的序列，输出同粒度指标序列。
- `calculate_kdj_next` 面向日常增量场景，显式接收上一条历史 K/D 状态；只有没有历史状态时才使用 `initial_k=50` 和 `initial_d=50`。
- RSV 分母为 0 的行为应在 `rsv` 或 KDJ API 文档中明确写为返回 50，不视为错误或无值。
- 窗口不足、输入价格为空等不可计算情况应通过 `Option<f64>` 或等价结构表达，不使用行级 `is_valid`。
- 公共 API 必须明确输入排序约束：同一证券、`trade_date` 严格升序且不重复。若 API 选择校验输入，应返回 typed error；若 API 选择信任调用方，应在 rustdoc 中写明前置条件，并由 CLI / I/O 层负责排序和去重校验。
- fixture 必须覆盖跨年状态延续、历史回填级联、非默认参数被生产写入拒绝、输入乱序/重复日期处理和浮点比较容差。

公共 API 文档要求：

- `furnace-core/src/lib.rs` 使用 `//!` 说明 crate 职责：金融指标纯计算库，不依赖 ClickHouse、Dagster、dbt。
- 每个 `pub` struct / enum / function 使用 `///` 说明用途、参数含义、返回值、边界行为和示例。
- 公共函数示例尽量写成可执行 doctest，避免文档与实现漂移。
- 早期可先人工保证文档覆盖；公共 API 稳定后再评估启用 `#![deny(missing_docs)]`。
- 如果函数返回 `Result`，文档必须包含 `# Errors`；如果可能 panic，必须包含 `# Panics`。

产出：

- `furnace-core` 纯计算 API。
- `furnace-core` rustdoc API 文档。
- RSV/KDJ 公共函数 doctest。
- golden fixture 或 snapshot tests。

验证：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test --doc --workspace
cargo doc --workspace --no-deps
```

后续如需要面向开发者沉淀更完整指标手册，可新增 mdBook：

```text
docs/furnace/
├── book.toml
└── src/
    ├── SUMMARY.md
    ├── indicators.md
    └── kdj.md
```

mdBook 负责公式说明、示例、设计背景和指标扩展指南；rustdoc 负责函数级 API 文档。第一版不强制创建 mdBook，但不应把函数 API 文档写成只存在于 Markdown 的孤立说明。

### 阶段 3：Furnace CLI 和 ClickHouse I/O

目标：

1. 在 `furnace` 中提供 `kdj` 子命令。
2. 支持 `--from`、`--to`、`--symbols`、`--rsv-window`、`--k-smoothing`、`--d-smoothing`、`--run-id`、`--mode`。
3. 在 `furnace-io` 中实现从 ClickHouse 读取输入区间和批量写入结果。
4. 写入批次遵守 `insert-batch-size`，目标 10,000-100,000 行。
5. 生产写入模式必须拒绝非默认 `9/3/3` 参数；非默认参数只允许 `--mode dry-run` 或显式写入非生产目标表。
6. 写入模式必须区分：
   - `dry-run`：只计算和输出摘要，不写 ClickHouse。
   - `append-latest`：日常追加增量，要求请求区间之后没有已物化结果。
   - `replace-cascade`：历史回填，自动扩展 `effective_output_to` 并走 staging + partition replace。
7. 提供 `--output-format json` 或等价固定输出协议；成功时 stdout 输出单个 JSON summary，stderr 只放日志，便于 Dagster 稳定解析。
8. 输出运行摘要：输入行数、输出行数、证券数量、请求区间、实际写入区间、实际 lookback 区间、受影响年份、批次数、历史 K/D 状态来源、staging 校验结果和分区替换结果。

产出：

- `furnace kdj ...` CLI。
- ClickHouse read/write adapter。
- 本地 dry-run 或 fixture-run 能力。
- staging table 创建、校验、partition replace 和清理封装。

验证：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### 阶段 4：Dagster asset 集成

目标：

1. 按 Dagster 官方 custom integration/component 建议，为 Furnace 预留自定义 component 接入点；第一版 component 生成一个 materializable asset、定向 jobs 和 schedule。
2. 在 scheduler 中新增 Furnace KDJ 计算资产，asset key 固定为 `["fleur_calculation", "calc_stock_kdj_daily"]`。
3. 用 Dagster asset dependency 表达对 dbt `int_stock_quotes_daily_adj` 的依赖。由于 Furnace 从 ClickHouse 自行读取输入，不通过 IOManager 传递 DataFrame，依赖应使用 `deps=[dg.AssetKey(["int_stock_quotes_daily_adj"])]`，而不是函数参数注入。
4. 通过 typed run config 传入请求日期区间、运行模式、证券集合和参数；第一版不使用 Dagster daily partitions 表达输出切片。
5. 封装 `FurnaceCliResource` 或等价 service：负责定位 Rust binary、设置工作目录、注入安全环境变量、执行子进程、处理 timeout、解析 JSON summary。
6. 捕获 CLI JSON summary 并返回 `MaterializeResult` metadata。
7. 提供定向 job，避免运行整个 dbt 或全部指标。
8. 计算资产的 asset key 应与 dbt source 的 `meta.dagster.asset_key` 保持一致，便于 Dagster dbt 集成将 dbt wrapper 的上游 source 映射回 Furnace 资产。
9. 资产执行前应确认上游 dbt `int_stock_quotes_daily_adj` 已物化到覆盖请求区间、lookback 区间和历史状态读取区间；该检查作为运行时前置校验，不在 definitions 加载阶段连接 ClickHouse。
10. 评估当前 Dagster 版本是否可直接使用 Pipes subprocess client；如可用，resource 优先采用 Pipes；如不可用，使用 JSON summary 协议并保留 Pipes 迁移验收项。

建议 definitions 落点：

```text
pipeline/scheduler/src/scheduler/components/furnace.py
pipeline/scheduler/src/scheduler/defs/furnace/defs.yaml
pipeline/scheduler/src/scheduler/defs/resources/furnace.py
pipeline/scheduler/tests/defs/furnace/
```

如果采用 component，`defs()` 需要把 Furnace component definitions 与现有 base/dbt definitions 合并。Furnace 不是 HTTP/TCP source bundle，不应塞进 `SOURCE_BUNDLES`；它属于外部计算引擎接入。

Dagster asset definition 契约：

```text
key = AssetKey(["fleur_calculation", "calc_stock_kdj_daily"])
deps = [AssetKey(["int_stock_quotes_daily_adj"])]
group_name = "calculation"
kinds/tags = {"owner": "furnace", "layer": "calculation", "storage": "clickhouse", "modality": "batch"}
return = MaterializeResult(metadata=...)
```

`MaterializeResult` metadata 至少包含：

| Metadata | 类型建议 | 说明 |
|----------|----------|------|
| `request_range` | json/text | 用户或 schedule 请求的 `[from, to]` |
| `effective_output_range` | json/text | 实际写入区间，历史回填时可能扩展到最新输入交易日 |
| `input_range` | json/text | Furnace 实际读取输入区间 |
| `mode` | text | `dry-run` / `append-latest` / `replace-cascade` |
| `symbols_count` | int | 证券数量 |
| `input_rows` | int | 输入行数 |
| `output_rows` | int | 新计算输出行数 |
| `null_indicator_rows` | int | RSV/K/D/J 为空的输出行数 |
| `affected_years` | json | 受影响 ClickHouse 年分区 |
| `retained_rows` | int | staging 中保留的旧行数 |
| `kdj_params` | json | `9/3/3` 参数 |
| `state_source` | text/json | 历史 K/D 状态来源 |
| `staging_validation` | json/md | staging 唯一键、row count、日期范围和抽样校验摘要 |
| `partition_replace` | json/md | `REPLACE PARTITION` 执行结果 |
| `furnace_binary` | path/text | Rust binary 或容器镜像版本摘要 |
| `furnace_exit_code` | int | 子进程退出码 |

失败语义：

1. CLI 非零退出码、JSON summary 缺失或 schema 不合法时，Dagster run 必须失败。
2. 上游输入覆盖不足、canonical 参数不符、`append-latest` 命中后续已物化结果、staging 校验失败时，Dagster run 必须失败。
3. `dry-run` 可以返回成功 materialization metadata，但 metadata 必须明确 `mode=dry-run` 和 `writes_applied=false`；生产 schedule 不允许使用 `dry-run`。
4. 失败 run 不写 `docs/jobs/reports/` 成功报告；需要人工诊断时另写 failure report。

Dagster 接入验收计划：

1. Definitions 验收：
   - `uv run dg list defs --json` 能看到 `fleur_calculation/calc_stock_kdj_daily` asset。
   - asset group、tags、owner、kind/storage/layer metadata 符合上表。
   - asset dependency 指向 `int_stock_quotes_daily_adj`，没有函数参数式 IOManager 输入。
2. CLI resource 单元测试：
   - 成功 JSON summary 能转换为 `MaterializeResult` metadata。
   - 非零退出码、非法 JSON、缺少必要字段、timeout 会让 asset 失败。
   - 命令展示和 metadata 不泄露 ClickHouse 密码或完整连接串。
   - 若采用 Pipes client，验证 Pipes metadata 与 JSON summary fallback 生成的 Dagster metadata 等价。
   - 若暂不采用 Pipes，验证 stdout 只包含结构化 JSON，stderr 日志不会影响 metadata 解析。
3. Asset 单元测试：
   - 使用 fake `FurnaceCliResource` 验证 run config 到 CLI 参数映射。
   - 验证 canonical 参数检查、`mode` 检查、`append-latest` / `replace-cascade` 前置校验路径。
   - 验证 metadata 包含请求区间、实际写入区间、受影响年份、row counts 和状态来源。
4. Job/schedule 验收：
   - `furnace__kdj_daily_job` 只选择 Furnace KDJ asset。
   - `furnace__kdj_backfill_job` 和 `furnace__kdj_dry_run_job` 需要显式 run config，不隐式全量重算。
   - schedule 不直接依赖自然日减法作为交易日事实；交易日判断来自输入表或交易日历。
5. 集成 smoke 验收：
   - 使用 `dry-run` 对 1-3 只证券、短日期区间运行，确认 Dagster metadata 与 CLI summary 一致。
   - 使用 `append-latest` 对最新交易日小范围运行，确认目标表新增或替换行为符合模式约束。
   - 使用 `replace-cascade` 对历史样本运行，确认 metadata 中 `effective_output_to` 覆盖受影响证券最新输入交易日，且 affected years 与 ClickHouse 分区替换记录一致。
   - 如果启用 Pipes，确认外部进程日志进入 Dagster run log，结构化 metadata 出现在 asset materialization 详情中；如果未启用 Pipes，确认 JSON summary fallback 仍能产生同等 metadata。

验证：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests
uv run dg list defs --json
cd scheduler
uv run dg check defs
```

### 阶段 5：dbt marts 消费和文档测试

目标：

1. 在 dbt sources YAML 中声明 `fleur_calculation.calc_stock_kdj_daily`，并配置 Dagster asset key metadata。
2. 在 dbt 中将 source 包装为 `int_stock_kdj_daily` thin wrapper，第一版默认 materialized 为 view。
3. 为 marts 层新增消费 `int_stock_kdj_daily` 的模型；marts 不直接读取 `fleur_calculation.*`。
4. 在 dbt YAML 中记录 grain、固定前复权口径、默认 KDJ 参数和 `NULL` 值语义。
5. 添加高价值 tests：
   - `security_code + trade_date` 唯一。
   - `security_code` 非空。
   - `trade_date` 非空。
   - `rsv_window = 9`、`k_smoothing = 3`、`d_smoothing = 3`。
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
3. 对至少一个历史区间执行 `replace-cascade` 验证，确认实际写入区间扩展到受影响证券最新输入交易日。
4. 记录运行命令、请求区间、实际写入区间、输入区间、输出行数、保留旧行数、ClickHouse part 数、抽样校验和问题。

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
FROM fleur_calculation.calc_stock_kdj_daily;
```

唯一键和 canonical 参数校验：

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
    rsv_window,
    k_smoothing,
    d_smoothing,
    count() AS rows
FROM fleur_calculation.calc_stock_kdj_daily
GROUP BY
    rsv_window,
    k_smoothing,
    d_smoothing;
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

dbt wrapper 另行验证：

```sql
SELECT
    count() AS rows,
    uniqExact(security_code) AS securities,
    min(trade_date) AS min_trade_date,
    max(trade_date) AS max_trade_date
FROM fleur_intermediate.int_stock_kdj_daily;
```

递推级联验收：

```sql
SELECT
    security_code,
    min(trade_date) AS min_written_date,
    max(trade_date) AS max_written_date,
    count() AS rows
FROM fleur_calculation.calc_stock_kdj_daily
WHERE security_code IN (<sample_security_codes>)
GROUP BY security_code
ORDER BY security_code;
```

历史回填报告必须说明 `max_written_date` 是否覆盖受影响证券当前最新输入交易日；如果没有覆盖，该运行不得视为生产回填成功。

## 8. 禁止模式

1. 不在 Dagster Python asset 中实现 RSV/KDJ 公式。
2. 不在 dbt SQL 中重复实现 RSV/KDJ 作为第一版主路径。
3. 不对 ClickHouse 目标表执行高频 `ALTER TABLE UPDATE`。
4. 不逐行 insert 或使用过小 batch 写入指标结果。
5. 不在 `furnace-core` 中读取环境变量、连接 ClickHouse 或调用 Dagster。
6. 不在 scheduler definitions 加载阶段连接 ClickHouse 或执行 Furnace CLI。
7. 不让下游 mart 直接依赖未包装的 `fleur_calculation.*` 计算产物。
8. 不让 Furnace 直接写入 `fleur_intermediate.int_stock_kdj_daily`。
9. 不让 marts 直接 `source()` 或硬编码查询 `fleur_calculation.*`；marts 必须通过 `ref('int_stock_kdj_daily')` 消费。
10. 不把 RSV/KDJ 函数 API 文档只写在独立 Markdown 中；公共函数必须使用 rustdoc 文档，Markdown 手册只能作为补充。
11. 不允许历史回填只替换请求日期区间后就结束；K/D 递推必须级联修正后续已物化交易日。
12. 不允许用 `REPLACE PARTITION` 替换只包含部分证券或部分日期的新数据分区；staging 分区必须补齐同年应保留的旧行。
13. 不把第一版 `calc_stock_kdj_daily` 建成 Dagster daily partition asset；历史级联写入会突破单日 partition 语义。
14. 不在 Dagster asset 函数中散落 subprocess 命令拼接、环境变量读取、JSON 解析和 timeout 处理；这些必须封装进 Furnace component/resource/service。
15. 不把由 Dagster 触发的 Furnace 计算建模为只观测 external asset；external asset 只用于未来 Furnace 完全外部运行的模式。
16. 不把 Dagster Pipes 当成业务依赖写进 `furnace-core`；Pipes 只属于 orchestration/transport 层，纯计算库必须保持无 Dagster 依赖。

## 9. 最小验证命令汇总

Rust：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test --doc --workspace
cargo doc --workspace --no-deps
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
uv run dg list defs --json
cd scheduler
uv run dg check defs
```

文档-only 变更：

```bash
git diff --check
```

## 10. 已决策项

1. dbt 消费 `fleur_calculation.calc_stock_kdj_daily` 采用 `source + Dagster asset metadata + thin wrapper model`。source 是 dbt 外部输入边界，Dagster metadata 是运行观测和跨工具 lineage，thin wrapper 是 `fleur_intermediate` 稳定消费契约。
2. 第一版生产结果表只承载 canonical `KDJ(9,3,3)`，参数列用于自描述和核验，不扩展业务 grain。多参数集需要后续重新设计唯一键或新表。
3. K/D 是无限递推状态，历史回填必须从重算起点级联到受影响证券最新输入交易日；只替换请求区间的历史回填不能写入生产表。
4. 年分区替换使用 staging table + `REPLACE PARTITION`，staging 分区必须补齐同年未重算但应保留的旧行，避免分区替换误删数据。
5. 第一版 Dagster 接入采用可物化 asset 调用 Rust CLI，不采用 external asset 上报模式；Furnace 完全外部运行时再重新评估 `AssetSpec`、sensor 或 REST materialization reporting。
6. 第一版 `calc_stock_kdj_daily` 不使用 Dagster daily partitions；请求区间通过 run config 表达，实际写入区间通过 `MaterializeResult` metadata 记录。
7. Dagster 外部进程接入按 Pipes 可迁移协议设计；第一版可使用 JSON summary fallback，但 resource 边界必须允许后续替换为 Pipes client，而不改变 Rust 计算 API、asset key、dbt wrapper 或 ClickHouse 写入协议。
