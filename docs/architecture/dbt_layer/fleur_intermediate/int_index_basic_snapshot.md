# int_index_basic_snapshot 设计

状态：Design

依据：

- Staging model：`ref('stg_baostock__query_stock_basic')`
- Staging 设计：`docs/architecture/dbt_layer/fleur_staging/stg_baostock__query_stock_basic.md`
- 目标位置：`pipeline/elt/models/intermediate/int_index_basic_snapshot.sql`
- 实施计划：`docs/plans/0042-index-benchmark-intermediate-implementation-plan.md`

## 1. 模型定位

BaoStock 指数基础信息当前快照 intermediate 模型。模型从 BaoStock 证券基础信息 staging 中筛选指数类证券，输出一指数一行的当前快照，供指数日行情、benchmark 基础信息和后续组合绩效分析复用。

本模型只表达 BaoStock source-local 指数 universe，不做跨源指数主数据归并、指数简称历史追踪、沪深双代码合并或指数体系裁决。

## 2. 数据粒度与依赖

- 直接依赖：`stg_baostock__query_stock_basic`。
- 粒度：一行一个 `security_code` 的 BaoStock 当前指数基础信息快照。
- 候选键：`security_code`。
- 指数 universe：`security_type = 'index'`。

快照语义：

- `stg_baostock__query_stock_basic` 是当前快照，不是按日期分区的历史主数据表。
- `listing_status` 表达 BaoStock 当前快照中的上市状态，不代表历史任意交易日状态。
- `index_name` 表达当前或 source 当前返回的指数名称，不代表历史名称。

## 3. 字段设计

| 字段 | 来源/派生 | 类型建议 | 设计说明 |
|---|---|---|---|
| `security_code` | `stg_baostock__query_stock_basic.security_code` | `String` | canonical 指数代码。 |
| `security_local_code` | `stg_baostock__query_stock_basic.security_local_code` | `String` | 6 位本地指数代码，仅在 basic snapshot 中保留。 |
| `exchange_code` | `stg_baostock__query_stock_basic.exchange_code` | `String` | 交易所代码。 |
| `index_name` | `stg_baostock__query_stock_basic.security_name` | `String` | BaoStock 当前快照中的指数名称。 |
| `ipo_date` | `stg_baostock__query_stock_basic.ipo_date` | `Date` | BaoStock 指数上市日期。 |
| `out_date` | `stg_baostock__query_stock_basic.out_date` | `Nullable(Date)` | BaoStock 指数退市日期；NULL 保留。 |
| `listing_status_code` | `stg_baostock__query_stock_basic.listing_status_code` | `Int8` | BaoStock 上市状态编码。 |
| `listing_status` | `stg_baostock__query_stock_basic.listing_status` | `Enum8` | BaoStock 上市状态标签。 |
| `is_listed` | `stg_baostock__query_stock_basic.is_listed` | `Bool` | 当前快照是否上市。 |
| `security_type_code` | `stg_baostock__query_stock_basic.security_type_code` | `Int8` | BaoStock 证券类型编码，指数为 `2`。 |
| `security_type` | `stg_baostock__query_stock_basic.security_type` | `Enum8` | 固定为 `index`。 |

## 4. SQL 逻辑

```sql
select ...
from {{ ref('stg_baostock__query_stock_basic') }}
where security_type = 'index'
```

实现注意：

- 不使用 `listing_status = 'listed'` 过滤；历史行情是否存在由行情事实模型决定。
- 不输出股票专用字段 `security_board`。
- 不在本模型内维护 benchmark 清单。

## 5. 测试建议

- `security_code`: `not_null`，`unique`，`cn_security_code_format`。
- `security_local_code`: `not_null`。
- `exchange_code`: `not_null`，accepted values。
- `security_type`: accepted values，仅允许 `index`。
- `security_type_code`: accepted values，仅允许 `2`。

## 6. 延后事项

- 跨源指数主数据合并和主数据优先级裁决。
- 指数名称历史变化和代码映射治理。
- 若需要统一全证券 universe，再另行设计 `int_security_basic_snapshot`。
