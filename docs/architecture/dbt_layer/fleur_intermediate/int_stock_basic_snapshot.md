# int_stock_basic_snapshot 设计

状态：Design

依据：

- Staging model：`ref('stg_baostock__query_stock_basic')`
- Staging 设计：`docs/architecture/dbt_layer/fleur_staging/stg_baostock__query_stock_basic.md`
- 目标位置：`pipeline/elt/models/intermediate/int_stock_basic_snapshot.sql`

## 1. 模型定位

股票基础信息快照 intermediate 模型。模型从 BaoStock 证券基础信息 staging 中筛选股票类证券，并输出一证券一行的当前快照，供日线行情、涨跌停规则、股本 as-of、估值指标和下游 mart 复用。

本模型负责把 BaoStock source-local 的证券类型、板块、上市状态和基础日期字段收敛为统一股票 universe。它不做历史证券主数据裁决、简称历史追踪、代码迁移处理、跨源证券实体合并或上市状态历史版本管理。

## 2. 数据粒度与依赖

- 直接依赖：`stg_baostock__query_stock_basic`。
- 粒度：一行一个 `security_code` 的 BaoStock 当前证券基础信息快照。
- 候选键：`security_code`。
- 股票 universe：`security_type = 'stock'`。
- 物化：ClickHouse `MergeTree()` table。
- 排序键：`security_code`。

快照语义：

- `stg_baostock__query_stock_basic` 是当前快照，不是按日期分区的历史主数据表。
- `listing_status` 表达 BaoStock 当前快照中的上市状态，不代表历史任意交易日状态。
- `security_name` 表达当前或 source 当前返回的证券简称，不代表历史简称。
- 已退市股票如仍存在于快照且 `security_type = 'stock'`，本模型保留；历史行情是否保留由行情事实模型决定。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | `stg_baostock__query_stock_basic.security_code` | `String` | canonical 股票代码。 |
| `security_local_code` | `stg_baostock__query_stock_basic.security_local_code` | `String` | 6 位本地代码。 |
| `exchange_code` | `stg_baostock__query_stock_basic.exchange_code` | `String` | 交易所代码，当前来自 BaoStock 代码前缀标准化。 |
| `security_name` | `stg_baostock__query_stock_basic.security_name` | `String` | BaoStock 当前快照证券简称，不做历史简称归并。 |
| `ipo_date` | `stg_baostock__query_stock_basic.ipo_date` | `Date` | BaoStock 上市日期。 |
| `out_date` | `stg_baostock__query_stock_basic.out_date` | `Nullable(Date)` | BaoStock 退市日期；NULL 保留。 |
| `security_type_code` | `stg_baostock__query_stock_basic.security_type_code` | `Int8` | BaoStock 证券类型编码，股票通常为 `1`。 |
| `security_type` | `stg_baostock__query_stock_basic.security_type` | `Enum8('stock' = 1, ...)` | BaoStock source-local 证券类型标签；本模型筛选为 `stock`。 |
| `security_board` | `stg_baostock__query_stock_basic.security_board` | `Nullable(Enum8(...))` | A 股板块分类。未命中当前 staging 代码段规则时保留 NULL。 |
| `listing_status_code` | `stg_baostock__query_stock_basic.listing_status_code` | `Int8` | BaoStock 上市状态编码。 |
| `listing_status` | `stg_baostock__query_stock_basic.listing_status` | `Enum8('delisted' = 0, 'listed' = 1)` | BaoStock source-local 当前上市状态。 |
| `is_listed` | `stg_baostock__query_stock_basic.is_listed` | `Bool` | 当前快照是否上市，透传自 staging；不代表历史交易日上市状态。 |

字段顺序建议：

1. 证券标识：`security_code`, `security_local_code`, `exchange_code`, `security_name`
2. 上市日期与状态：`ipo_date`, `out_date`, `listing_status_code`, `listing_status`, `is_listed`
3. 类型与板块：`security_type_code`, `security_type`, `security_board`

## 4. SQL 逻辑建议

```sql
with source as (
    select
        security_code,
        security_local_code,
        exchange_code,
        security_name,
        ipo_date,
        out_date,
        security_type_code,
        security_type,
        security_board,
        listing_status_code,
        listing_status,
        is_listed
    from {{ ref('stg_baostock__query_stock_basic') }}
    where security_type = 'stock'
)

select
    security_code,
    security_local_code,
    exchange_code,
    security_name,
    ipo_date,
    out_date,
    listing_status_code,
    listing_status,
    is_listed,
    security_type_code,
    security_type,
    security_board
from source
```

实现注意：

- 不使用 `listing_status = 'listed'` 过滤；否则会丢失已退市股票的历史行情关联能力。
- 不使用 `ipo_date` / `out_date` 裁剪日线行情；这些规则放到行情或主数据 mart 中按业务口径处理。
- 不强制 `security_board is not null`。当前 staging 只覆盖沪市主板、深市主板、创业板、科创板；未覆盖板块需要在后续 profile 和设计中扩展。
- 本模型是股票 universe 快照，不是全证券快照。如后续需要指数、ETF、可转债等统一基础信息，应另设 `int_security_basic_snapshot` 或调整命名和口径。

## 5. 与其他模型的关系

- `int_stock_quotes_daily_unadj`：用本模型过滤 `security_type = 'stock'` 的行情 universe；行情事实表不应完整复制本模型所有字段。
- `int_stock_price_limit_daily`：可 join 本模型获取 `security_board`、`exchange_code` 和 `is_listed`，再结合交易日和 ST 状态计算涨跌停规则。
- `int_stock_share_capital_asof`：可 join 本模型校验股票 universe，但股本字段仍来自 EastMoney 股本历史。
- `int_stock_valuation_daily`：可 join 本模型补充证券名称、板块和上市状态，用于展示或过滤。

## 6. 测试建议

- `security_code`: `not_null`，`unique`，`cn_security_code_format`。
- `security_local_code`: `not_null`。
- `exchange_code`: `not_null`，`accepted_values`，取值 `SH`, `SZ`, `BJ`。
- `security_type`: `accepted_values`，仅允许 `stock`。
- `security_type_code`: `accepted_values`，仅允许 `1`。
- `listing_status`: `accepted_values`，取值 `listed`, `delisted`。
- `listing_status_code`: `accepted_values`，取值 `0`, `1`。
- `is_listed`: `not_null`。
- `security_board`: 保留 staging 既有 accepted-values 与一致性测试；本模型不新增 `not_null`。

## 7. 延后事项

- 跨源证券主数据合并和主数据优先级裁决。
- 历史简称、证券类型历史变化和代码迁移处理。
- 北交所、B 股或其他未覆盖板块的补充分类。
- 基于交易日的上市 / 退市区间裁剪。
- 将全证券 universe 扩展到指数、ETF、可转债等资产类型。
