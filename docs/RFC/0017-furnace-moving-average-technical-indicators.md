# RFC 0017: Furnace Moving Average 日线技术指标需求

状态：草案 / 原始需求（2026-06-08）

## 摘要

本文档记录 mono-fleur 在 Rust 计算引擎 `furnace` 中新增 Moving Average 日线技术指标的需求基线。第一版目标是基于 `fleur_intermediate.int_stock_quotes_daily_adj` 的前复权收盘价，计算多组简单移动平均线、两个组合均线和一个双重 EMA 指标，并通过 dbt thin wrapper 暴露为：

```text
fleur_intermediate.int_stock_ma_daily
```

本文档只冻结需求和边界，不创建实现代码。后续可拆分为实施 plan、Rust crate 设计、Dagster asset 变更和 dbt 模型变更。

实施修订：`docs/plans/archive/0029-furnace-moving-average-technical-indicators-implementation-plan.md` 与 `docs/ADR/0010-technical-indicator-field-naming.md` 已覆盖本文档中的早期裸字段命名。当前实现使用 `price_ma_*`、`price_avg_ma_*`、`price_ema2_10` 和 `volume_ma_*` canonical 字段；`ma_*`、`avg_ma_*`、`ema2_10` 仅作为本文档的历史需求表述，不作为实现或下游消费契约。

## 背景

当前 Furnace 已按以下模式承载日频 KDJ 计算：

```text
dbt intermediate input
  fleur_intermediate.int_stock_quotes_daily_adj
      ↓
Furnace / Dagster materialized calculation table
  fleur_calculation.calc_stock_kdj_daily
      ↓
dbt thin wrapper
  fleur_intermediate.int_stock_kdj_daily
```

Moving Average 指标应沿用同一 ownership 边界：

```text
dbt intermediate input
  fleur_intermediate.int_stock_quotes_daily_adj
      ↓
Furnace / Dagster materialized calculation table
  fleur_calculation.calc_stock_ma_daily
      ↓
dbt thin wrapper
  fleur_intermediate.int_stock_ma_daily
```

这样保留 `int_*` 的 dbt intermediate 语义，同时让外部计算产物继续由 Furnace/Dagster 管理写入、重算、运行观测和 ClickHouse 分区替换。

## 原始需求

### 功能需求

1. Furnace 新增 Moving Average 日频指标计算能力，建议 CLI 子命令命名为 `furnace ma`。
2. 输入默认来自 `fleur_intermediate.int_stock_quotes_daily_adj`。
3. 第一版固定使用前复权收盘价字段 `close_price_forward_adj` 作为 `close` 输入。
4. 输出物理计算表为 `fleur_calculation.calc_stock_ma_daily`。
5. dbt 通过 thin wrapper 暴露消费表 `fleur_intermediate.int_stock_ma_daily`。
6. 第一版必须计算以下简单移动平均线：

```text
MA(close, 3)
MA(close, 5)
MA(close, 6)
MA(close, 10)
MA(close, 12)
MA(close, 14)
MA(close, 20)
MA(close, 24)
MA(close, 28)
MA(close, 57)
MA(close, 60)
MA(close, 114)
MA(close, 250)
```

7. 第一版必须计算组合均线：

```text
avg_ma_3_6_12_24 = (MA(close, 3) + MA(close, 6) + MA(close, 12) + MA(close, 24)) / 4
avg_ma_14_28_57_114 = (MA(close, 14) + MA(close, 28) + MA(close, 57) + MA(close, 114)) / 4
```

8. 第一版必须计算双重 EMA：

```text
ema2_10 = EMA(EMA(close, 10), 10)
```

9. Furnace 必须支持按日期区间、证券代码集合和运行模式执行。
10. 日常增量、历史回填和晚到复权修正必须保持可重复、幂等和可观测。
11. 指标公式只允许在 `furnace-core` 中实现，不允许在 Dagster Python asset 或 dbt SQL wrapper 中重复实现。
12. MA、EMA 这类基础时间序列算子必须抽取为 `furnace-core` 内的公共算子库，由 Moving Average 指标模块组合调用；不得只写成 `furnace ma` 或单个指标模块的私有实现。

### 非功能需求

1. `furnace-core` 中的 MA/EMA 计算逻辑必须与 ClickHouse、Dagster、dbt、Rayon 和环境变量解耦。
2. 指标结果必须有固定样本测试覆盖，至少覆盖窗口不足、空收盘价、正常滚动窗口、组合均线和双重 EMA 递推。
3. 公共算子库必须有独立单元测试覆盖，至少覆盖 rolling SMA、SMA-seeded EMA、递推状态延续、空值不推进状态和非法参数。
4. ClickHouse 写入必须走批量写入和受控 staging/partition replace 协议，避免单行 insert 或高频 mutation。
5. CLI stdout 应输出稳定 JSON summary；stderr 用于日志，便于 Dagster 捕获。
6. 业务结果表不写 `run_id`、`computed_at` 或输入读取区间；这些运行审计信息由 Dagster materialization metadata 和运行报告记录。

## 非目标

1. 不在本 RFC 中实现代码。
2. 不新增实时流式计算或交易策略执行。
3. 不改变 `int_stock_quotes_daily_adj` 的复权逻辑和字段语义。
4. 不让 dbt 计算 MA/EMA 指标；dbt 只负责 source、thin wrapper、docs 和 tests。
5. 不让 Furnace 直接写入 `fleur_intermediate.int_stock_ma_daily`。
6. 不在第一版同时实现 MACD、RSI、布林线或其他未列出的指标。
7. 不保留 `MA(close, 47)`；原始需求中的 `47` 按笔误处理，canonical 窗口以 `57` 为准。

## 指标定义

### 输入序列

对每个 `security_code`，按 `trade_date` 升序处理：

```text
close = close_price_forward_adj
```

本文档中所有 `MA(close, n)`、组合均线和 `EMA(close, 10)` 的 `close` 均指 `close_price_forward_adj`，即当前前复权收盘价。第一版不允许混用未复权收盘价、后复权收盘价或其他价格字段。后续如果需要未复权、后复权或多价格口径并存，必须重新设计结果表 grain、字段命名和 dbt tests。

### 简单移动平均线

对窗口 `n`：

```text
MA(close, n) = 最近 n 个有效 close 的算术平均值
```

规则：

1. 有效 close 指 `close_price_forward_adj IS NOT NULL` 的输入记录。
2. 当前行 close 为空时，该行所有 MA、组合均线和 `ema2_10` 输出 `NULL`，且 EMA 状态不推进。
3. 当同一证券累计有效 close 数少于 `n` 时，`MA(close, n)` 输出 `NULL`。
4. 当窗口完整时，`MA(close, n)` 输出最近 `n` 个有效 close 的平均值。
5. MA 只依赖有限窗口；窗口最大值为 250。

### 公共算子边界

现有 KDJ 已经抽取到 `furnace-core` 的指标层，当前代码边界为：

```text
engines/crates/furnace-core/src/indicators/kdj.rs
```

其中 RSV 滚动窗口、K/D 递推平滑和 KDJ 序列计算都在 KDJ 模块内部实现；当前尚未抽取出可跨指标复用的公共算子库。

Moving Average 实施时必须新增公共算子层，例如：

```text
engines/crates/furnace-core/src/operators/
```

第一版公共算子至少包含：

1. rolling SMA：维护固定长度有效值窗口，窗口不足时输出 `None`。
2. SMA-seeded EMA：先用前 `n` 个有效值的 SMA 启动，之后按递推公式更新。
3. 递推状态对象：支持从上一条有效 EMA 状态继续计算，用于日常增量。

指标模块只负责把公共算子组合成业务字段：

```text
operators::sma(close, n) -> ma_n
operators::ema(close, 10) -> ema1_10
operators::ema(ema1_10, 10) -> ema2_10
```

公共算子不得依赖 ClickHouse、Dagster、dbt、Rayon、CLI 参数或环境变量。后续如果重构 KDJ，可以考虑将 K/D 递推平滑迁移到同一公共算子层；本 RFC 不要求为了 MA 首版同步重构既有 KDJ。

### 组合均线

组合均线只在所有组成 MA 都非空时输出值；任一组成 MA 为 `NULL` 时，组合均线输出 `NULL`。

```text
avg_ma_3_6_12_24 = (ma_3 + ma_6 + ma_12 + ma_24) / 4
avg_ma_14_28_57_114 = (ma_14 + ma_28 + ma_57 + ma_114) / 4
```

字段名必须使用 `57`：

```text
ma_57
avg_ma_14_28_57_114
```

不得生成 `ma_47` 或 `avg_ma_14_28_47_114`。

### 双重 EMA

第一版定义：

```text
设 t 为同一证券内有效 close 的序号。

alpha_10 = 2 / (10 + 1)

ema1_10[t] =
  NULL,                                      有效 close 计数 < 10
  SMA(close[1..10]),                         有效 close 计数 = 10
  alpha_10 * close[t]
    + (1 - alpha_10) * ema1_10[t-1],         有效 close 计数 > 10

ema2_10[t] =
  NULL,                                      有效 ema1_10 计数 < 10
  SMA(前 10 个有效 ema1_10),                 有效 ema1_10 计数 = 10
  alpha_10 * ema1_10[t]
    + (1 - alpha_10) * ema2_10[t-1],         有效 ema1_10 计数 > 10
```

启动规则：

1. `EMA(close, 10)` 使用同一证券前 10 个有效 `close_price_forward_adj` 的 SMA 作为初始值。
2. 在累计有效 close 少于 10 个时，`ema1_10` 输出 `NULL`，且不能作为 `ema2_10` 的输入。
3. 第 10 个有效 close 对应的 `ema1_10` 等于前 10 个有效 close 的算术平均值；从第 11 个有效 close 开始按递推公式更新。
4. `EMA(EMA(close, 10), 10)` 对 `ema1_10` 的非空序列使用同样规则启动：前 10 个有效 `ema1_10` 的 SMA 作为初始 `ema2_10`。
5. 因此，`ema2_10` 的首个非空值出现在第 19 个有效 close 对应的交易日。
6. 当前行 close 为空时，`ema2_10` 输出 `NULL`，且 `ema1_10` / `ema2_10` 历史状态不推进。
7. 历史 close 发生修正时，该证券后续所有 `ema1_10` 和 `ema2_10` 都可能变化；生产回填必须级联到该证券当前最新输入交易日。

10 日 EMA 示例：

```text
第 1-10 个有效 close 总和 = 559
第 10 个有效 close 的初始 EMA(close, 10) = SMA(close, 10) = 559 / 10 = 55.9

alpha_10 = 2 / (10 + 1) = 0.181818...

假设第 11 个有效 close = 60：
第 11 个有效 close 的 EMA(close, 10)
  = (60 - 55.9) * 0.181818... + 55.9
  = 56.645454...
```

该示例中的 `close` 仍然是 `close_price_forward_adj`。第 12 个有效 close 及之后的 EMA 都以上一条有效 EMA 状态继续递推。

`ema2_10` 的精确增量计算需要上一条有效 `ema1_10` 和上一条有效 `ema2_10` 作为状态。实施方案必须在以下两种方式中选择一种，并在测试中证明结果一致：

1. 在 calculation 层保留内部状态列或状态表，用于精确增量启动；dbt wrapper 只暴露业务消费字段。
2. 每次从足够早的历史输入重新推导 EMA 状态；如果不是从证券首个有效 close 开始，必须证明 warm-up 截断误差在项目可接受范围内。

第一版建议采用精确状态方案，避免 EMA 截断误差进入生产结果。

## Lookback 和重算语义

计算请求区间 `[from, to]` 时：

1. MA 需要读取请求区间之前至少 249 个有效 close，以支持 `MA(close, 250)`。
2. 组合均线不需要额外 lookback，直接依赖组成 MA。
3. `ema2_10` 是递推指标，不能仅用固定 250 日 lookback 保证历史修正后的精确结果。
4. 日常追加模式应读取目标区间之前最近一条有效 EMA 状态。
5. 历史回填或复权修正模式必须将实际写入区间级联到受影响证券的最新输入交易日。

运行区间定义建议：

```text
request_from = 用户请求开始日期
request_to = 用户请求结束日期
input_from = request_from 之前至少 249 个有效 close 对应的交易日，或精确 EMA 状态推导所需更早日期
input_to = effective_output_to
effective_output_from = request_from
effective_output_to = append-latest 模式下等于 request_to；replace-cascade 模式下扩展到受影响证券最新输入交易日
```

如果只想验证历史片段但不级联后续 `ema2_10`，只能使用 dry-run 或实验输出，不允许写入生产 `fleur_calculation.calc_stock_ma_daily`。

## 目标输出模型

### 表名

Furnace 直接写入表：

```text
fleur_calculation.calc_stock_ma_daily
```

dbt intermediate wrapper：

```text
fleur_intermediate.int_stock_ma_daily
```

### Grain

每证券、交易日一行：

```text
security_code
trade_date
```

第一版固定使用前复权收盘价和 RFC 中列出的 canonical 指标集合。参数不作为业务 grain 的一部分；如后续要支持多价格口径或多参数集合，必须重新设计唯一键和消费模型。

### 字段草案

`int_stock_ma_daily` 面向下游暴露以下字段：

| 字段 | 类型建议 | 说明 |
|------|----------|------|
| `security_code` | `String` | 证券代码 |
| `trade_date` | `Date` | 交易日 |
| `ma_3` | `Nullable(Float64)` | 3 日简单移动平均 |
| `ma_5` | `Nullable(Float64)` | 5 日简单移动平均 |
| `ma_6` | `Nullable(Float64)` | 6 日简单移动平均 |
| `ma_10` | `Nullable(Float64)` | 10 日简单移动平均 |
| `ma_12` | `Nullable(Float64)` | 12 日简单移动平均 |
| `ma_14` | `Nullable(Float64)` | 14 日简单移动平均 |
| `ma_20` | `Nullable(Float64)` | 20 日简单移动平均 |
| `ma_24` | `Nullable(Float64)` | 24 日简单移动平均 |
| `ma_28` | `Nullable(Float64)` | 28 日简单移动平均 |
| `ma_57` | `Nullable(Float64)` | 57 日简单移动平均 |
| `ma_60` | `Nullable(Float64)` | 60 日简单移动平均 |
| `ma_114` | `Nullable(Float64)` | 114 日简单移动平均 |
| `ma_250` | `Nullable(Float64)` | 250 日简单移动平均 |
| `avg_ma_3_6_12_24` | `Nullable(Float64)` | `ma_3`、`ma_6`、`ma_12`、`ma_24` 的算术平均 |
| `avg_ma_14_28_57_114` | `Nullable(Float64)` | `ma_14`、`ma_28`、`ma_57`、`ma_114` 的算术平均 |
| `ema2_10` | `Nullable(Float64)` | `EMA(EMA(close, 10), 10)` |

`calc_stock_ma_daily` 可以包含额外内部状态字段，例如 `ema1_10_state`，用于精确增量计算；但 `int_stock_ma_daily` 默认不暴露内部状态字段。若实施阶段选择不持久化内部状态，则必须从足够早的输入重新推导状态，并在运行 metadata 中记录状态来源。

### ClickHouse engine / partition / order 草案

第一版建议沿用 KDJ calculation 表的宽表和年度分区模式：

```sql
CREATE TABLE IF NOT EXISTS fleur_calculation.calc_stock_ma_daily
(
    security_code String,
    trade_date Date,
    ma_3 Nullable(Float64),
    ma_5 Nullable(Float64),
    ma_6 Nullable(Float64),
    ma_10 Nullable(Float64),
    ma_12 Nullable(Float64),
    ma_14 Nullable(Float64),
    ma_20 Nullable(Float64),
    ma_24 Nullable(Float64),
    ma_28 Nullable(Float64),
    ma_57 Nullable(Float64),
    ma_60 Nullable(Float64),
    ma_114 Nullable(Float64),
    ma_250 Nullable(Float64),
    avg_ma_3_6_12_24 Nullable(Float64),
    avg_ma_14_28_57_114 Nullable(Float64),
    ema2_10 Nullable(Float64)
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code);
```

如果选择在同表持久化 EMA 内部状态，DDL 可增加：

```sql
ema1_10_state Nullable(Float64)
```

该状态列属于 calculation 层实现细节；dbt wrapper 应保持下游消费契约稳定。

## CLI 需求草案

建议 CLI 形态：

```bash
furnace ma \
  --from 2026-01-01 \
  --to 2026-01-31 \
  --symbols all \
  --mode append-latest \
  --input-table fleur_intermediate.int_stock_quotes_daily_adj \
  --output-table fleur_calculation.calc_stock_ma_daily \
  --price-column close_price_forward_adj \
  --run-id <dagster-run-id> \
  --output-format json
```

运行模式建议与 KDJ 保持一致：

| Mode | 用途 |
|------|------|
| `dry-run` | 计算并输出摘要，不写生产表 |
| `append-latest` | 日常追加最新区间，若目标表已有同证券更晚结果则拒绝 |
| `replace-cascade` | 历史回填和修正，写入 staging 后替换受影响年度分区 |

JSON summary 至少包含：

| 字段 | 说明 |
|------|------|
| `indicator` | 固定为 `ma` |
| `request_from` / `request_to` | 用户请求区间 |
| `effective_output_from` / `effective_output_to` | 实际写入区间 |
| `input_from` / `input_to` | 实际读取输入区间 |
| `mode` | 运行模式 |
| `symbols_count` | 证券数量 |
| `input_rows` | 输入行数 |
| `output_rows` | 输出行数 |
| `null_indicator_rows` | 指标全空或关键指标为空的行数 |
| `affected_years` | 受影响年度分区 |
| `retained_rows` | staging 中保留的旧行数 |
| `partition_replace` | 分区替换结果 |
| `ma_windows` | 固定窗口集合 |
| `ema_state_source` | `previous-state`、`full-history` 或其他状态来源 |

## dbt 接入需求

dbt source：

```yaml
sources:
  - name: fleur_calculation
    schema: fleur_calculation
    tables:
      - name: calc_stock_ma_daily
        description: Furnace/Dagster materialized daily Moving Average technical indicator table.
        meta:
          dagster:
            asset_key:
              - fleur_calculation
              - calc_stock_ma_daily
```

dbt thin wrapper：

```sql
{{ config(materialized='view') }}

select
    security_code,
    trade_date,
    ma_3,
    ma_5,
    ma_6,
    ma_10,
    ma_12,
    ma_14,
    ma_20,
    ma_24,
    ma_28,
    ma_57,
    ma_60,
    ma_114,
    ma_250,
    avg_ma_3_6_12_24,
    avg_ma_14_28_57_114,
    ema2_10
from {{ source('fleur_calculation', 'calc_stock_ma_daily') }}
```

dbt tests 至少覆盖：

1. `security_code` 非空并符合 A 股代码格式。
2. `trade_date` 非空。
3. `security_code + trade_date` 唯一。
4. 字段名中只存在 `ma_57` 和 `avg_ma_14_28_57_114`，不得引入 `47` 口径字段。

## Dagster 接入需求

建议新增 Dagster asset：

```text
AssetKey(["fleur_calculation", "calc_stock_ma_daily"])
```

上游依赖：

```text
AssetKey(["int_stock_quotes_daily_adj"])
```

Dagster 负责：

1. 调用 `furnace ma` CLI。
2. 注入 ClickHouse 连接环境变量和运行参数。
3. 捕获 stdout JSON summary。
4. 将 summary 转换为 `MaterializeResult` metadata。
5. 提供定向 job，例如 `furnace__ma_daily_job`、`furnace__ma_backfill_job` 和 `furnace__ma_dry_run_job`。

Dagster 不负责：

1. 在 Python 中计算 MA/EMA。
2. 解析自由文本日志获取业务事实。
3. 绕过 Furnace 直接写 `calc_stock_ma_daily`。

## 验收标准

RFC 后续实施完成时应满足：

1. `furnace-core` 提供公共 `operators` 模块，包含 rolling SMA 和 SMA-seeded EMA 算子，并通过独立单元测试。
2. `furnace-core` 提供单证券 MA/EMA 指标组合纯计算 API，该 API 复用公共算子并通过固定样本测试。
3. `furnace ma` 支持 date range、symbols、mode、input table、output table 和 JSON summary。
4. `furnace-io` 能创建并写入 `fleur_calculation.calc_stock_ma_daily`。
5. 生产表写入使用 staging + 年度分区替换协议，历史修正能级联更新 `ema2_10`。
6. Dagster 可以物化 `fleur_calculation/calc_stock_ma_daily` asset，并记录输入区间、输出区间、行数、受影响年份和 EMA 状态来源。
7. dbt 声明 `calc_stock_ma_daily` source，并暴露 `fleur_intermediate.int_stock_ma_daily` view。
8. dbt wrapper 不重写 MA/EMA 公式，只 select Furnace 产出的字段。
9. 输出字段包含 `ma_57` 和 `avg_ma_14_28_57_114`，不包含 `ma_47` 或 `avg_ma_14_28_47_114`。

## 已决策项

1. `47` 为笔误，Moving Average canonical 窗口使用 `57`。
2. 第一版价格输入固定为 `close_price_forward_adj`。
3. 下游消费表命名为 `fleur_intermediate.int_stock_ma_daily`。
4. Furnace 物理写入表命名为 `fleur_calculation.calc_stock_ma_daily`，dbt 只做 thin wrapper。
5. 第一版采用宽表字段，不采用长表 `indicator_name/value` 结构。
6. `EMA(close, 10)` 使用前 10 个有效前复权收盘价的 SMA 启动，`EMA(EMA(close, 10), 10)` 使用前 10 个有效 `ema1_10` 的 SMA 启动。

## 待决问题

1. `ema2_10` 的精确增量状态是保存在 `calc_stock_ma_daily` 的内部状态列，还是放入单独状态表？
2. `int_stock_ma_daily` 是否需要额外暴露 `close_price_forward_adj` 便于下游调试？第一版建议不暴露，避免和行情事实表重复。
3. MA 窗口是否按有效 close 计数，而不是按自然交易日行数计数？本文档当前采用有效 close 计数。

## 相关文档

- `docs/RFC/0016-rust-furnace-compute-engine.md`
- `docs/plans/archive/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- `docs/plans/archive/0028-furnace-kdj-parallel-performance-implementation-plan.md`
- `engines/README.md`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.yml`
- `pipeline/elt/models/intermediate/int_stock_kdj_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_kdj_daily.yml`
- `pipeline/elt/models/sources_fleur_calculation.yml`
