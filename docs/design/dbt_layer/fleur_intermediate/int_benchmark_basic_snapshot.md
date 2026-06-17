# int_benchmark_basic_snapshot 设计

状态：Design

依据：

- Upstream model：`ref('int_index_basic_snapshot')`
- 目标位置：`pipeline/elt/models/intermediate/int_benchmark_basic_snapshot.sql`
- 实施计划：`docs/plans/0042-index-benchmark-intermediate-implementation-plan.md`

## 1. 模型定位

组合绩效 benchmark 基础信息 intermediate 模型。模型从 `int_index_basic_snapshot` 过滤当前允许的 benchmark 清单，输出一 benchmark 一个选中指数代码的当前基础信息。

第一版 benchmark 清单只保留 raw profile 已验证具备有效指数基础信息和日行情的数据，不输出不可用 benchmark，也不输出备用代码多行。

## 2. 数据粒度与依赖

- 直接依赖：`int_index_basic_snapshot`。
- 粒度：一行代表一个 `benchmark_key` 的选中指数代码。
- 候选键：`benchmark_key`；组合键 `benchmark_key`, `security_code` 也唯一。

第一版清单：

| benchmark_key | benchmark_name | security_code |
|---|---|---|
| `csi_a100` | 中证A100 | `000903.SH` |
| `csi_300` | 沪深300 | `000300.SH` |
| `csi_500` | 中证500 | `000905.SH` |
| `csi_800` | 中证800 | `000906.SH` |
| `csi_1000` | 中证1000 | `000852.SH` |
| `cnindex_1000` | 国证1000 | `399311.SZ` |

## 3. 字段设计

| 字段 | 来源/派生 | 类型建议 | 设计说明 |
|---|---|---|---|
| `benchmark_key` | inline mapping | `String` | 稳定 benchmark 业务 key。 |
| `benchmark_name` | inline mapping | `String` | benchmark 中文名称。 |
| `security_code` | mapping join `int_index_basic_snapshot` | `String` | 选中 BaoStock 指数 canonical 代码。 |
| `security_local_code` | `int_index_basic_snapshot` | `String` | 6 位本地指数代码，仅在 basic snapshot 中保留。 |
| `exchange_code` | `int_index_basic_snapshot` | `String` | 指数代码所属交易所。 |
| `index_name` | `int_index_basic_snapshot` | `String` | BaoStock 当前快照中的指数名称。 |
| `listing_status` | `int_index_basic_snapshot` | `Enum8` | BaoStock 当前快照上市状态标签。 |
| `is_listed` | `int_index_basic_snapshot` | `Bool` | 当前快照是否上市。 |

## 4. SQL 逻辑

```sql
with benchmark_map as (
    select 'csi_300' as benchmark_key, '沪深300' as benchmark_name, '000300.SH' as security_code
    ...
)

select ...
from benchmark_map
inner join {{ ref('int_index_basic_snapshot') }} as index_basic
    on benchmark_map.security_code = index_basic.security_code
```

实现注意：

- 不从 staging 或 raw 表直接读取。
- 不补空值保留不可用 benchmark。
- 不输出备用代码多行。

## 5. 测试建议

- `benchmark_key`: `not_null`，`unique`，accepted values。
- `security_code`: `not_null`，`cn_security_code_format`，relationships 到 `int_index_basic_snapshot.security_code`。
- 组合键 `benchmark_key`, `security_code`: 唯一。
- `benchmark_name`, `security_local_code`, `exchange_code`, `index_name`, `is_listed`: `not_null`。

## 6. 延后事项

- 清单增长后是否迁移为 dbt seed 或独立小维表。
- 后续是否新增 mart 层 benchmark 读取入口。
