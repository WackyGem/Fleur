# Plan 0038: MA30 与复权行情 mart 前置实施计划

日期：2026-06-15

状态：Completed

领域：data-platform

关联文档：

- `docs/RFC/archive/0020-racingline-run-result-security-analysis-page.md`
- `docs/systems/data-platform.md`
- `docs/systems/data-platform.md`
- `docs/design/dbt_layer/fleur_intermediate/int_stock_quotes_daily_adj.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator.md`
- `docs/plans/archive/0029-furnace-moving-average-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0035-stock-technical-indicator-marts-implementation-plan.md`
- `pipeline/elt/models/sources_fleur_calculation.yml`
- `pipeline/elt/models/intermediate/int_stock_ma_daily.sql`
- `pipeline/elt/models/marts/mart_stock_trend_indicator.sql`
- `pipeline/elt/models/marts/mart_stock_quotes_daily.sql`
- `engines/crates/furnace-core/src/indicators/moving_average.rs`
- `engines/crates/furnace-io/src/schema/tables.rs`
- `engines/crates/furnace-io/src/runners/ma/writing.rs`
- `docs/jobs/reports/2026-06-15-ma30-adjusted-quotes-mart-rerun.md`

相关 skills：

- `fleur-harness`：计划文档、active plan 索引和文档-only 校验。
- `using-dbt-for-analytics-engineering`：dbt source、intermediate wrapper、mart SQL/YAML、测试和定向 build。
- `clickhouse-best-practices`：ClickHouse mart 排序键、join 过滤和 schema 演进约束。

## 1. 背景

RFC 0020 要求 Racingline 个股分析页支持：

1. K 线主图默认展示 MA5、MA10、MA30。
2. K 线支持前复权、后复权和不复权三种价格口径。
3. Rearview 只能通过 mart 层读取 ClickHouse 数据，不应绕过 mart 直接读 intermediate。

当前数据平台还缺两项前置能力：

1. `price_ma_30` 没有出现在 `fleur_calculation.calc_stock_ma_daily`、`int_stock_ma_daily` 或 `mart_stock_trend_indicator` 中。
2. `mart_stock_quotes_daily` 只输出未复权 OHLC，前复权和后复权 OHLC 仍停留在 `int_stock_quotes_daily_adj`。

本计划只处理这两个数据前置项。完成后，Rearview analysis API 可以从 mart 层读取 MA30 和三种 K 线口径。

## 2. 目标

完成后应满足：

1. Furnace Moving Average 计算结果新增 `price_ma_30`，口径为 30 个有效 `close_price_forward_adj` 的简单移动平均。
2. `price_ma_30` 通过 `fleur_calculation.calc_stock_ma_daily`、`int_stock_ma_daily` 和 `mart_stock_trend_indicator` 稳定暴露。
3. `mart_stock_quotes_daily` 新增前复权和后复权 OHLC 字段，来源为 `int_stock_quotes_daily_adj`，不在 mart 层重算复权公式。
4. `mart_stock_quotes_daily` 继续保留原有未复权 OHLC 字段名和语义，避免破坏现有消费方。
5. 新增字段均有 dbt YAML 文档和数据测试覆盖。
6. 历史数据通过 Furnace MA 回填和 dbt mart build 重新物化，不能只改 schema 留空值。
7. 计划完成后，RFC 0020 中的 MA30 和复权 K 线数据前置项可标记为已满足。

## 3. 非目标

本计划不做以下事情：

1. 不实现 Racingline 页面、Rearview analysis API 或浏览器图表交互。
2. 不增加任意 MA 窗口编辑能力；本计划只补 `price_ma_30`。
3. 不用 `price_ma_28` 或其他近似窗口替代 MA30。
4. 不在 dbt SQL、Dagster Python 或 ClickHouse SQL 中重写 MA 公式；MA30 仍由 Furnace 计算。
5. 不改变已有 MA5、MA10、MA28、MA60 等字段语义。
6. 不改变 `int_stock_quotes_daily_adj` 的复权公式、字段语义或模型边界。
7. 不把 RSI、MACD、BOLL、KDJ 改成多复权口径；这些指标仍按当前已物化口径暴露。
8. 不新建独立 chart mart，除非实施阶段发现 `mart_stock_quotes_daily` 字段扩展无法满足性能或边界要求。

## 4. 当前事实基线

### 4.1 MA 字段现状

当前 `calc_stock_ma_daily` source、`int_stock_ma_daily` 和 `mart_stock_trend_indicator` 已有价格 MA 字段：

```text
price_ma_3
price_ma_5
price_ma_6
price_ma_10
price_ma_12
price_ma_14
price_ma_20
price_ma_24
price_ma_28
price_ma_57
price_ma_60
price_ma_114
price_ma_250
```

当前没有：

```text
price_ma_30
```

影响范围：

| 层 | 当前文件 | 需要补齐 |
|---|---|---|
| Rust core | `engines/crates/furnace-core/src/indicators/moving_average.rs` | MA 输出结构、窗口映射和测试 |
| Rust IO schema | `engines/crates/furnace-io/src/schema/tables.rs` | calculation 表 DDL 增加 `price_ma_30` |
| Rust IO RowBinary | `engines/crates/furnace-io/src/rows/ma.rs`、`engines/crates/furnace-io/src/runners/ma/writing.rs` | 输出 row 和 insert column list 增加 `price_ma_30` |
| dbt source | `pipeline/elt/models/sources_fleur_calculation.yml` | `calc_stock_ma_daily` column 文档增加 `price_ma_30` |
| dbt intermediate | `pipeline/elt/models/intermediate/int_stock_ma_daily.sql/.yml` | wrapper 透传和字段文档 |
| dbt mart | `pipeline/elt/models/marts/mart_stock_trend_indicator.sql/.yml` | trend mart 透传和字段文档 |
| 设计文档 | `docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator.md` | 字段列表和验证命令同步 |

### 4.2 复权 OHLC 现状

`int_stock_quotes_daily_adj` 当前已有：

```text
open_price_backward_adj
high_price_backward_adj
low_price_backward_adj
close_price_backward_adj
prev_close_price_backward_adj
open_price_forward_adj
high_price_forward_adj
low_price_forward_adj
close_price_forward_adj
prev_close_price_forward_adj
backward_adjustment_factor
backward_adjustment_ratio
forward_adjustment_factor
forward_adjustment_ratio
```

`mart_stock_quotes_daily` 当前只输出未复权：

```text
open_price
high_price
low_price
close_price
prev_close_price
prev_close_price_unadj
```

本计划要求 `mart_stock_quotes_daily` 通过 `(security_code, trade_date)` 左连接 `int_stock_quotes_daily_adj`，至少新增前复权和后复权 OHLC。建议同步暴露 `prev_close_price_*_adj` 和复权因子，便于 API 解释价格口径和除权除息影响。

## 5. ClickHouse 约束

实施时按以下规则执行：

1. Per `schema-pk-plan-before-creation`：不改变现有 `mart_stock_quotes_daily` 的 `ORDER BY (security_code, trade_date)`，也不改变 `mart_stock_trend_indicator` 的 `ORDER BY (trade_date, security_code)`；本计划只加列。
2. Per `schema-pk-filter-on-orderby`：验收和 Rearview 后续查询必须继续按排序键过滤。单证券 K 线查询应使用 `security_code` + 日期窗口；指标 mart 查询应使用日期窗口 + `security_code`。
3. Per `query-join-filter-before`：新增测试或 API 查询如果 join quote/trend/momentum mart，应先在各自 CTE 中按证券和日期窗口过滤，再 join。
4. Per `query-join-use-any`：一行一个 `(security_code, trade_date)` 的表在只需要单匹配时可使用 `ANY JOIN` 语义，避免异常重复行扩大结果集。
5. Per `insert-mutation-avoid-update`：历史 MA30 不通过 `ALTER TABLE UPDATE` 回填。允许 `ALTER TABLE ADD COLUMN IF NOT EXISTS` 增加列，但历史值必须通过 Furnace `replace-cascade` 或等价分区替换重算写入。

## 6. 实施阶段

### Phase 0: 契约确认和影响扫描

任务：

1. 确认 `price_ma_30` 口径：30 个有效 `close_price_forward_adj` 的简单移动平均，NULL 语义沿用其他 `price_ma_*`。
2. 确认 `mart_stock_quotes_daily` 新增字段命名：
   - `open_price_forward_adj`
   - `high_price_forward_adj`
   - `low_price_forward_adj`
   - `close_price_forward_adj`
   - `prev_close_price_forward_adj`
   - `open_price_backward_adj`
   - `high_price_backward_adj`
   - `low_price_backward_adj`
   - `close_price_backward_adj`
   - `prev_close_price_backward_adj`
   - `forward_adjustment_factor`
   - `forward_adjustment_ratio`
   - `backward_adjustment_factor`
   - `backward_adjustment_ratio`
3. 用 `rg price_ma_30` 确认实施前没有旧字段残留；如果已有部分实现，先审查并决定复用或修正。
4. 确认不新增独立 adjusted quote mart；第一阶段直接扩展 `mart_stock_quotes_daily`。

完成标准：

- 字段名、口径和下游消费边界已确认。
- 如果字段集合有变化，先同步更新本计划再进入实现。

### Phase 1: Furnace MA30 计算输出

任务：

1. 在 `furnace-core` Moving Average 输出结构中增加 `price_ma_30`。
2. 把窗口 `30` 纳入价格 MA 窗口集合和 `price_ma(window)` 映射。
3. 更新单证券序列计算测试，覆盖：
   - 第 29 个有效 close 输出 `NULL`。
   - 第 30 个有效 close 输出 MA30。
   - close 为 `NULL` 时 MA30 不推进有效窗口。
4. 在 `furnace-io` calculation 表 DDL 中增加 `price_ma_30 Nullable(Float64)`。
5. 更新 `MaResultRow` 和 RowBinary 写入顺序，insert column list 必须显式包含 `price_ma_30`。
6. 更新 Rust schema / runner 测试，断言 DDL 和 insert SQL 均包含 `price_ma_30`。

完成标准：

- Rust 单元测试能证明 MA30 计算语义与其他 `price_ma_*` 一致。
- RowBinary 字段顺序和 ClickHouse insert column list 一致。
- `price_ma_28` 不被重命名、不被替换、不参与 MA30 输出。

### Phase 2: calculation 表 schema 演进和 MA 历史重算

任务：

1. 处理已有 `fleur_calculation.calc_stock_ma_daily` 表缺少列的问题。`CREATE TABLE IF NOT EXISTS` 不会给既有表自动加列，因此需要以下策略之一：
   - 在 Furnace MA runner 中执行 `ALTER TABLE ... ADD COLUMN IF NOT EXISTS price_ma_30 Nullable(Float64)`；或
   - 在实施 runbook 中显式执行一次 add column DDL。
2. 不使用 `ALTER TABLE UPDATE` 回填历史值。
3. 对 MA calculation 执行覆盖目标历史区间的 `replace-cascade` 或等价分区替换重算。
4. 记录实际运行报告到 `docs/jobs/reports/`，包含命令、日期范围、运行模式、输出行数和 spot check 结果。

完成标准：

- `fleur_calculation.calc_stock_ma_daily` 物理表存在 `price_ma_30`。
- 全部目标历史区间的 `price_ma_30` 随 MA 重算写入，不是全表长期 `NULL`。
- 运行报告能追溯本次 schema 演进和重算范围。

### Phase 3: dbt source、intermediate 和 trend mart 透传 MA30

任务：

1. 在 `pipeline/elt/models/sources_fleur_calculation.yml` 的 `calc_stock_ma_daily` 下新增 `price_ma_30` 字段文档。
2. 在 `int_stock_ma_daily.sql` 和 `.yml` 中透传并描述 `price_ma_30`。
3. 在 `mart_stock_trend_indicator.sql` 和 `.yml` 中透传并描述 `price_ma_30`。
4. 更新 `docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator.md` 的价格 MA 字段列表。
5. 如 Rearview metric catalog 自动从 mart YAML 同步字段，执行 catalog sync；如果 `metric_policy.yml` 需要手工暴露图表字段，再补 `price_ma_30` 对应配置。

完成标准：

- dbt parse 能识别 `price_ma_30`。
- `mart_stock_trend_indicator` 查询可返回 `price_ma_30`。
- 字段文档明确 MA30 基于 `close_price_forward_adj`，不是 MA28 的别名。

### Phase 4: `mart_stock_quotes_daily` 透传前复权和后复权行情

任务：

1. 在 `mart_stock_quotes_daily.sql` 新增 `adjusted_quotes` CTE，从 `{{ ref('int_stock_quotes_daily_adj') }}` 选择复权 OHLC、复权前收和因子字段。
2. 以未复权 `quotes` 为主事实表，按 `(security_code, trade_date)` 左连接 `adjusted_quotes`。不得因为复权数据缺口丢失原始 quote 行。
3. 在最终 select 中保留原未复权字段，并新增前复权和后复权字段。
4. 更新 `mart_stock_quotes_daily.yml`：
   - 模型描述说明同时暴露未复权、前复权和后复权价格。
   - 为新增字段补中文描述和 `data_type`。
   - 因子字段如果来自 upstream 非空，可在 mart 层追加 `not_null` 测试，或用专门数据测试检测缺口。
5. 更新 `docs/design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md`，移除“本模型不做复权价格输出”的过期描述，改为“复权价格只透传 `int_stock_quotes_daily_adj`，不在 mart 重算”。

完成标准：

- `mart_stock_quotes_daily` 粒度仍是一行一个 `security_code` + `trade_date`。
- 原有未复权字段值和 key set 不变。
- 前复权和后复权字段来自 `int_stock_quotes_daily_adj`，不在 mart SQL 中重复公式。

### Phase 5: dbt 数据测试和定向 build

新增或更新测试：

1. `mart_stock_quotes_daily_key_set_matches_quotes.sql` 继续保证 mart key set 和 `int_stock_quotes_daily_unadj` 一致。
2. 新增 adjusted passthrough 测试，比较 `mart_stock_quotes_daily` 的复权字段与 `int_stock_quotes_daily_adj` 同键字段一致。
3. 新增 adjusted key coverage 测试，检测 quote mart 中任一未复权 quote 行是否缺少对应复权因子或复权 OHLC。
4. `mart_stock_trend_indicator` 的 YAML 和设计文档包含 `price_ma_30`；如需要，增加一个 schema/compiled SQL 防回归测试，确认 trend mart select 了 `price_ma_30`。

完成标准：

- 所有新增测试随定向 `dbt build` 通过。
- 数据测试失败时不得通过前端或 Rearview fallback 掩盖。

### Phase 6: 端到端消费验收

任务：

1. 用 dbt 查询抽样验证单证券日期窗口：
   - `mart_stock_quotes_daily` 同时返回未复权、前复权和后复权 OHLC。
   - `mart_stock_trend_indicator` 返回 `price_ma_5`、`price_ma_10`、`price_ma_30`。
2. 确认 Rearview 后续 analysis API 可只读 mart 层满足 RFC 0020 图表需求。
3. 如同步了 Rearview metric catalog，验证 catalog 中可发现 `price_ma_30` 或对应图表字段。
4. 写运行报告到 `docs/jobs/reports/`，记录样本证券、日期窗口、查询结果摘要和任何 NULL 缺口。

完成标准：

- RFC 0020 的两个数据前置项完成：
  - MA30 mart 数据可用。
  - `mart_stock_quotes_daily` 前复权和后复权 OHLC 可用。
- 没有前端或后端绕过 mart 读取 intermediate 的需求。

## 7. 禁止模式

1. 禁止在 `mart_stock_trend_indicator` 中用 `price_ma_28` 填充或重命名为 `price_ma_30`。
2. 禁止在 dbt SQL 中用窗口函数临时计算 MA30。
3. 禁止在 `mart_stock_quotes_daily` 中重复实现复权公式；只能透传 `int_stock_quotes_daily_adj`。
4. 禁止通过 `ALTER TABLE UPDATE` 给历史 MA30 补值。
5. 禁止为了新增复权字段改变 `mart_stock_quotes_daily` 的主 grain、排序键或原未复权字段名。
6. 禁止把当前 mart 查询值写回 Rearview PostgreSQL run snapshot。

## 8. 最小验证命令

文档-only 阶段：

```bash
make docs-check
git diff --check
```

涉及 Rust / Furnace MA30 实现时：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 dbt mart 实现时：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_ma_daily mart_stock_trend_indicator mart_stock_quotes_daily
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator --limit 20
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_quotes_daily --limit 20
```

涉及 Rearview metric catalog 同步时：

```bash
cd engines
cargo run -p rearview -- catalog check
cargo run -p rearview -- catalog sync
```

## 9. 完成标准

计划完成后：

1. `rg price_ma_30 engines pipeline/elt docs/design` 能定位到 Rust 计算、ClickHouse DDL、dbt source、intermediate、mart 和设计文档。
2. `mart_stock_trend_indicator` 提供 `price_ma_30`，且不改变已有 MA 字段。
3. `mart_stock_quotes_daily` 提供前复权和后复权 OHLC，并保留未复权 OHLC。
4. 定向 Rust 和 dbt 验证命令通过。
5. MA30 历史重算和 mart 字段抽样核验有 `docs/jobs/reports/` 运行报告。
6. 完成后将本计划状态改为 `Completed` 并移入 `docs/plans/archive/`，同步更新 `docs/plans/README.md`。
