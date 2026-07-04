# int_stock_shares_history 设计

状态：Design

依据：

- Staging model：`ref('stg_eastmoney__equity_history')`
- Staging model：`ref('stg_eastmoney__freeholders')`
- Staging 设计：`docs/architecture/dbt_layer/fleur_staging/stg_eastmoney__equity_history.md`
- Staging 设计：`docs/architecture/dbt_layer/fleur_staging/stg_eastmoney__freeholders.md`
- Raw profile：`docs/references/raw_profile/eastmoney__equity_history.md`
- Raw profile：`docs/references/raw_profile/eastmoney__freeholders.md`
- 目标位置：`pipeline/elt/models/intermediate/int_stock_shares_history.sql`

## 1. 模型定位

股票股本区间 intermediate 模型。模型把东方财富股本变动历史和前十大流通股东报告期明细合并为证券级股本有效区间，输出总股本、流通股本、A 股股本、A 股流通股本和 A 股自由流通股本。

本模型负责把 source-local 的股本事件和流通股东披露期转换为可做 as-of join 的区间表，供日频估值、市值、换手率和自由流通市值模型复用。它不展开到交易日粒度，不处理跨源证券主数据裁决，不做股东身份跨源归并，也不解释 B 股、H 股或混合股份类别持股的 A 股拆分。

## 2. 数据粒度与依赖

- 股本依赖：`stg_eastmoney__equity_history`。
- 流通股东依赖：`stg_eastmoney__freeholders`。
- 粒度：一行一个 `security_code` + `effective_date` 的股本有效区间。
- 候选键：`security_code`, `effective_date`。
- 区间语义：`effective_date` 当日生效，`expiry_date` 为下一生效日前一天；最后一个已知区间 `expiry_date` 为 `NULL`。
- 物化：ClickHouse `MergeTree()` table。
- 排序键：`security_code`, `effective_date`。
- 分区：`toYear(effective_date)`。

`effective_date` 的来源：

- `stg_eastmoney__equity_history.end_date`：总股本、流通股本、A 股股本或 A 股流通股本可能变化。
- `stg_eastmoney__freeholders.report_date`：超过 5% 大股东扣减额可能变化；模型实现中可在 CTE 内别名为 `end_date` 参与股本区间逻辑。即使该报告期没有超过 5% 的 A 股流通股东，也必须作为 change point，用于把上一报告期的扣减额归零。

as-of 取值规则：

- 对每个 change point，取同一证券 `end_date <= effective_date` 的最近一条股本记录。
- 对每个 change point，取同一证券 `end_date <= effective_date` 的最近一个 A 股流通股东报告期聚合结果。
- 如果某个股本 change point 之前没有可用流通股东报告期，`major_holder_a_float_shares` 取 `0`，`source_freeholders_end_date` 为 `NULL`。

## 3. 核心口径

股本字段口径：

- `total_shares`：总股本，来自 `stg_eastmoney__equity_history.total_shares`。
- `float_shares`：流通股本，来自 `stg_eastmoney__equity_history.unlimited_a_shares`。
- `float_shares_a`：A 股流通股本，来自 `stg_eastmoney__equity_history.listed_a_shares`。
- `shares`：A 股股本，等于 `listed_a_shares + limited_a_shares`。当两个输入字段均为空时输出 `NULL`；单边为空时按 0 参与相加。

A 股自由流通股本口径：

```text
free_float_shares =
    float_shares_a
    - sum(free_float_hold_shares where shares_type = 'A股' and free_float_holdnum_ratio_pct > 5)
```

说明：

- `free_float_holdnum_ratio_pct` 保留东方财富百分数口径，`5` 表示 5%，不是 0.05。
- 只纳入 `shares_type = 'A股'` 的流通股东记录。`A股,H股`、`A股,B股` 等混合类别无法从当前字段拆出 A 股部分，第一版不纳入扣减。
- 扣减后结果下限钳制为 `0`，避免供应商异常比例或重复披露导致负自由流通股本。
- 这里的自由流通股本是项目口径估算值，不直接采用 `stg_eastmoney__equity_history.free_shares`。现有 remote endpoint 文档已记录 `FREE_SHARES` 在部分样例中通常等于 `TOTAL_SHARES`，不能无条件视为实际自由流通 A 股。

流通股东去重口径：

- `stg_eastmoney__freeholders` 明确保留每证券、报告期、名次、股东标识/名称和股份类别的明细行，且 `security_code + end_date + holder_rank` 不唯一。
- 本模型需要的是“同一报告期中大股东持有的流通 A 股数量合计”，不需要每名次一行。
- 第一版按 `security_code`, `end_date`, `holder_identifier`, `holder_name`, `shares_type` 折叠同一股东同一股份类别；重复行中取 `max(free_float_hold_shares)` 和 `max(free_float_holdnum_ratio_pct)`，避免同一股东重复披露放大扣减额。
- 不做 `holder_identifier` 与 `holder_name` 的跨源或跨期身份归并。

## 4. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | change point | `String` | 股票标准连接代码。 |
| `effective_date` | 股本记录或 A 股流通股东报告期 | `Date` | 区间生效开始日期。 |
| `expiry_date` | 下一 `effective_date - 1` | `Nullable(Date)` | 区间失效日期；`NULL` 表示当前已知最新区间。 |
| `source_equity_end_date` | 股本 as-of 记录 | `Date` | 本区间采用的最近一条股本变动记录截止日。 |
| `source_freeholders_end_date` | 流通股东 as-of 聚合 | `Nullable(Date)` | 本区间采用的最近一个 A 股流通股东报告期；无报告期时为 `NULL`。 |
| `total_shares` | `equity_history.total_shares` | `Nullable(Float64)` | 总股本，单位为股。 |
| `float_shares` | `equity_history.unlimited_a_shares` | `Nullable(Float64)` | 流通股本，单位为股。 |
| `shares` | `listed_a_shares + limited_a_shares` | `Nullable(Float64)` | A 股股本，单位为股。 |
| `float_shares_a` | `equity_history.listed_a_shares` | `Nullable(Float64)` | A 股流通股本，单位为股。 |
| `major_holder_a_float_shares` | A 股流通股东报告期聚合 | `Float64` | 持有流通 A 股比例超过 5% 的股东持股数量合计，单位为股。 |
| `free_float_shares` | `float_shares_a - major_holder_a_float_shares` | `Nullable(Float64)` | A 股自由流通股本估算值，单位为股。 |
| `major_holder_count` | A 股流通股东报告期聚合 | `UInt64` | 纳入扣减的超过 5% A 股流通股东数量。 |

字段顺序建议：

1. 主键与区间字段：`security_code`, `effective_date`, `expiry_date`
2. 来源追踪字段：`source_equity_end_date`, `source_freeholders_end_date`
3. 股本字段：`total_shares`, `float_shares`, `shares`, `float_shares_a`, `free_float_shares`
4. 扣减解释字段：`major_holder_a_float_shares`, `major_holder_count`

## 5. SQL 逻辑建议

```sql
with equity_history as (
    select
        security_code,
        report_date as end_date,
        total_shares,
        unlimited_a_shares,
        listed_a_shares,
        limited_a_shares
    from {{ ref('stg_eastmoney__equity_history') }}
),

freeholders_deduplicated as (
    select
        security_code,
        end_date,
        holder_identifier,
        holder_name,
        shares_type,
        max(free_float_hold_shares) as free_float_hold_shares,
        max(free_float_holdnum_ratio_pct) as free_float_holdnum_ratio_pct
    from {{ ref('stg_eastmoney__freeholders') }}
    where shares_type = 'A股'
    group by
        security_code,
        end_date,
        holder_identifier,
        holder_name,
        shares_type
),

freeholders_report_aggregates as (
    select
        security_code,
        end_date,
        sumIf(free_float_hold_shares, free_float_holdnum_ratio_pct > 5) as major_holder_a_float_shares,
        countIf(free_float_holdnum_ratio_pct > 5) as major_holder_count
    from freeholders_deduplicated
    group by
        security_code,
        end_date
),

change_points as (
    select security_code, end_date as effective_date
    from equity_history

    union distinct

    select security_code, end_date as effective_date
    from freeholders_report_aggregates
)
```

后续步骤：

1. 对 `change_points` as-of join 最近一条股本记录。
2. 对 `change_points` as-of join 最近一个 A 股流通股东报告期聚合。
3. 使用窗口函数计算下一 `effective_date`，派生 `expiry_date`。
4. 输出目标字段。

实现注意：

- ClickHouse 默认 `join_use_nulls = 0` 时，`LEFT JOIN` 未命中的右表字段会给类型默认值；实现中如需区分“无报告期”和“报告期为默认日期”，应使用右表匹配键或显式 nullable 处理。
- `change_points` 必须包含所有 A 股流通股东报告期，而不是只包含有超过 5% 股东的报告期。
- `major_holder_a_float_shares` 缺失时按 `0` 参与计算；`source_freeholders_end_date` 仍应保留为 `NULL` 表达尚无报告期。
- 不输出 `free_shares` 字段，避免和本模型计算的 `free_float_shares` 混淆。
- 不把区间展开到交易日；日频模型使用 `trade_date >= effective_date and (expiry_date is null or trade_date <= expiry_date)` 关联。

## 6. 测试建议

- 模型级组合唯一：`security_code`, `effective_date`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `effective_date`: `not_null`。
- `source_equity_end_date`: `not_null`，且应小于等于 `effective_date`。
- `source_freeholders_end_date`: 可空；非空时应小于等于 `effective_date`。
- `total_shares`: `not_null`。
- `major_holder_a_float_shares`: `not_null`，且应大于等于 0。
- `major_holder_count`: `not_null`，且应大于等于 0。
- `free_float_shares`: 可空；非空时应满足 `0 <= free_float_shares <= float_shares_a`。
- 增加定向数据测试：
  - 同一证券区间不得重叠。
  - 每只证券最多一条 `expiry_date is null` 的当前区间。
  - 当下一条 `effective_date` 存在时，当前 `expiry_date = next_effective_date - 1`。
  - 当 `source_freeholders_end_date` 为 `NULL` 时，`major_holder_a_float_shares = 0` 且 `major_holder_count = 0`。

## 7. 延后事项

- 对 `A股,H股`、`A股,B股` 等混合股份类别进行 A 股部分拆分。
- 股东身份跨期归并、同一集团或一致行动人穿透。
- 用公告日、上市流通日或交易日历修正 `effective_date`。
- 把股本区间展开到交易日粒度。
- 接入其他股本或自由流通股本供应商做优先级合并和对账。
- 对超过 5% 阈值附近的披露误差设置数据质量告警。
