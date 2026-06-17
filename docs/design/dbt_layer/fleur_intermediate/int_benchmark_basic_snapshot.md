# int_benchmark_basic_snapshot 设计

状态：Design

依据：

- Upstream model：`ref('int_index_basic_snapshot')`
- 目标位置：`pipeline/elt/models/intermediate/int_benchmark_basic_snapshot.sql`
- 实施计划：`docs/plans/0042-index-benchmark-intermediate-implementation-plan.md`

## 1. 模型定位

组合绩效 benchmark 基础信息 intermediate 模型。模型从 `int_index_basic_snapshot` 过滤当前允许作为 benchmark 的指数清单，输出一 benchmark 一行的基础信息。

benchmark 不发明独立业务 key，直接沿用 canonical `security_code` 标识，与持仓、行情、指数同一套体系，避免平行命名空间和额外映射维护。显示名使用 `index_name`，不另设 `benchmark_name`。

第一版 benchmark 清单只保留 raw profile 已验证具备有效指数基础信息和日行情的数据，不输出不可用 benchmark，也不输出备用代码多行。

## 2. 数据粒度与依赖

- 直接依赖：`int_index_basic_snapshot`。
- 粒度：一行代表一个允许作为 benchmark 的指数 `security_code`。
- 候选键：`security_code`。

第一版清单：

| security_code | 指数名称 |
|---|---|
| `000903.SH` | 中证A100 |
| `000300.SH` | 沪深300 |
| `000905.SH` | 中证500 |
| `000906.SH` | 中证800 |
| `000852.SH` | 中证1000 |
| `399311.SZ` | 国证1000 |

## 3. 字段设计

| 字段 | 来源/派生 | 类型建议 | 设计说明 |
|---|---|---|---|
| `security_code` | `int_index_basic_snapshot` | `String` | canonical 指数代码，benchmark 主键；不另设 benchmark key。 |
| `security_local_code` | `int_index_basic_snapshot` | `String` | 6 位本地指数代码，仅在 basic snapshot 中保留。 |
| `exchange_code` | `int_index_basic_snapshot` | `String` | 指数代码所属交易所。 |
| `index_name` | `int_index_basic_snapshot` | `String` | BaoStock 当前快照中的指数名称，作为 benchmark 显示名。 |
| `listing_status` | `int_index_basic_snapshot` | `Enum8` | BaoStock 当前快照上市状态标签。 |
| `is_listed` | `int_index_basic_snapshot` | `Bool` | 当前快照是否上市。 |

## 4. SQL 逻辑

```sql
with benchmark_universe as (
    select '000300.SH' as security_code
    union all
    ...
)

select ...
from {{ ref('int_index_basic_snapshot') }} as index_basic
inner join benchmark_universe
    on index_basic.security_code = benchmark_universe.security_code
```

实现注意：

- 不从 staging 或 raw 表直接读取。
- 不补空值保留不可用 benchmark。
- 不输出备用代码多行。
- 不引入平行 benchmark key，benchmark 即指数，用 `security_code` 标识。

## 5. 测试建议

- `security_code`: `not_null`，`unique`，`cn_security_code_format`，accepted values（第一版清单），relationships 到 `int_index_basic_snapshot.security_code`。
- `security_local_code`, `exchange_code`, `index_name`, `is_listed`: `not_null`。

## 6. 延后事项

- 清单增长后是否迁移为 dbt seed 或独立小维表。
- 后续是否新增 mart 层 benchmark 读取入口。
