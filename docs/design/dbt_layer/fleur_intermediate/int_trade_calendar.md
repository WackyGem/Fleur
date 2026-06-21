# int_trade_calendar 设计

状态：Design

依据：

- Staging model：`ref('stg_sina__trade_calendar')`
- Staging 设计：`docs/design/dbt_layer/fleur_staging/stg_sina__trade_calendar.md`
- 目标位置：`pipeline/elt/models/intermediate/int_trade_calendar.sql`

## 1. 模型定位

A 股交易日历的 intermediate table。模型在新浪 source-local 交易日历之上派生可复用的相邻交易日字段，为后续行情、事件和 mart 模型提供统一交易日序列。

本模型只负责交易日序列内的相邻日期关系，不补全自然日历，不派生交易周、交易月、月末交易日或下一个交易日等字段。

## 2. 数据粒度与依赖

- 依赖：`stg_sina__trade_calendar`。
- 粒度：一行一个 `trade_date`。
- 排序口径：按 `trade_date` 升序计算前交易日。
- 第一条交易日记录没有前交易日，`prev_trade_date` 为 `NULL`。
- 物化：ClickHouse `MergeTree()` table。
- 排序键：`trade_date`。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `trade_date` | `stg_sina__trade_calendar.trade_date` | `Date` | 第一列；交易日主键，沿用 staging canonical 字段。 |
| `prev_trade_date` | 按 `trade_date` 升序窗口计算 | `Nullable(Date)` | 第二列；当前交易日在交易日序列中的前一个交易日。最早交易日无前值，语义上保留 `NULL`。 |

## 4. SQL 逻辑建议

ClickHouse 窗口函数需要显式避免缺省日期哨兵值。建议对窗口输入使用 `toNullable(trade_date)`，并把窗口默认值设为 `NULL`。

```sql
with trade_calendar as (
    select
        trade_date
    from {{ ref('stg_sina__trade_calendar') }}
),

with_previous_trade_date as (
    select
        trade_date,
        lagInFrame(toNullable(trade_date), 1, null) over (
            order by trade_date
            rows between unbounded preceding and unbounded following
        ) as prev_trade_date
    from trade_calendar
)

select
    trade_date,
    prev_trade_date
from with_previous_trade_date
```

## 5. 测试建议

- `trade_date`: `not_null`，唯一。
- `prev_trade_date`: 不做 `not_null`，因为最早交易日应为 `NULL`。
- 增加表达式测试或定向数据测试：
  - 除最早 `trade_date` 外，`prev_trade_date` 不为 `NULL`。
  - `prev_trade_date < trade_date`。
  - `prev_trade_date` 必须存在于本模型的 `trade_date` 集合中。

## 6. 延后事项

- `next_trade_date`。
- 自然日期到最近交易日的映射。
- 交易周、交易月、交易季、交易年序号。
- 月末、季末、年末最后交易日标记。
