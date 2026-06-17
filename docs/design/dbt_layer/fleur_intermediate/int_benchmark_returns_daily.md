# int_benchmark_returns_daily 设计

状态：Design

依据：

- Upstream model：`ref('int_benchmark_basic_snapshot')`
- Upstream model：`ref('int_index_quotes_daily')`
- 目标位置：`pipeline/elt/models/intermediate/int_benchmark_returns_daily.sql`
- 实施计划：`docs/plans/0042-index-benchmark-intermediate-implementation-plan.md`

## 1. 模型定位

组合绩效 benchmark 日收益 intermediate 模型。模型从 benchmark basic 和指数日行情 join 生成 benchmark 日频收益，供组合绩效指标、portfolio worker 或后续 mart 复用。

本模型只输出日频事实所需字段，不输出本地指数代码和交易所代码；需要这些维度时通过 `benchmark_key`、`security_code` join `int_benchmark_basic_snapshot`。

## 2. 数据粒度与依赖

- 直接依赖：`int_benchmark_basic_snapshot`、`int_index_quotes_daily`。
- 粒度：一行代表一个 `benchmark_key`、`security_code` 在一个 `trade_date` 的价格指数日收益。
- 候选键：`benchmark_key`, `security_code`, `trade_date`。
- 收益口径：价格指数简单收益，不包含分红再投资。

## 3. 字段设计

| 字段 | 来源/派生 | 类型建议 | 设计说明 |
|---|---|---|---|
| `benchmark_key` | `int_benchmark_basic_snapshot` | `String` | 稳定 benchmark 业务 key。 |
| `benchmark_name` | `int_benchmark_basic_snapshot` | `String` | benchmark 中文名称。 |
| `security_code` | `int_index_quotes_daily` | `String` | 选中 BaoStock 指数 canonical 代码。 |
| `trade_date` | `int_index_quotes_daily` | `Date` | 交易日。 |
| `close_price` | `int_index_quotes_daily` | `Nullable(Float64)` | benchmark 收盘点位。 |
| `prev_close_price` | `int_index_quotes_daily` | `Nullable(Float64)` | BaoStock 原始 preclose 口径前收盘点位。 |
| `return_daily` | `int_index_quotes_daily.return_daily` | `Nullable(Float64)` | 价格指数简单日收益。 |

## 4. SQL 逻辑

```sql
select ...
from {{ ref('int_benchmark_basic_snapshot') }} as benchmarks
inner join {{ ref('int_index_quotes_daily') }} as quotes
    on benchmarks.security_code = quotes.security_code
```

实现注意：

- 不重新维护 benchmark 清单。
- 不重新计算指数 universe。
- 不输出 `security_local_code` 和 `exchange_code`。

## 5. 测试建议

- 组合键 `benchmark_key`, `security_code`, `trade_date`: 唯一。
- `benchmark_key`: `not_null`，accepted values。
- `security_code`: `not_null`，`cn_security_code_format`，relationships 到 `int_benchmark_basic_snapshot.security_code`。
- `trade_date`: `not_null`。

## 6. 延后事项

- 是否在 mart 层新增 `mart_benchmark_returns_daily` 作为 worker 长期读取入口。
- benchmark 缺口和异常行情处理策略。
