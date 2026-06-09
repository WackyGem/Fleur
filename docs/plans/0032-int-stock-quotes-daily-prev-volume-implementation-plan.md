# Plan 0032: int_stock_quotes_daily_unadj prev_volume 字段实施方案

日期：2026-06-09

状态：Completed

完成日期：2026-06-09

实际验证：

- `uv run dbt parse --project-dir elt --profiles-dir elt`
- `uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_quotes_daily_unadj mart_stock_quotes_daily`
- `uv run python elt/scripts/validate_field_glossary.py`
- `git diff --check`

关联文档：

- `docs/design/dbt_layer/fleur_intermediate/int_stock_quotes_daily_unadj.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.yml`
- `pipeline/elt/models/marts/mart_stock_quotes_daily.sql`
- `pipeline/elt/models/marts/mart_stock_quotes_daily.yml`
- `pipeline/elt/tests/marts/mart_stock_quotes_daily_quote_passthrough_matches.sql`

相关 skills：

- `using-dbt-for-analytics-engineering`：dbt model、YAML、singular tests、定向 build 和数据校验。
- `fleur-harness`：计划文档结构、当前代码事实检查和最小质量门禁。

## 1. 目标

在现有 A 股日频未复权行情链路中新增字段：

```text
prev_volume
```

字段语义：

```text
当前证券在前一个 A 股交易日的成交量。
```

完成后应满足：

1. `int_stock_quotes_daily_unadj` 输出 `prev_volume`，类型为 `Nullable(Int64)`。
2. `prev_volume` 使用 `int_trade_calendar.prev_trade_date` 定义“前一个交易日”，再按 `security_code + prev_trade_date` 自连接日行情取 `volume`。
3. `prev_volume` 与现有 `prev_close_price_unadj` 使用同一前交易日口径；不使用 `lag(volume)` 替代交易日历语义。
4. `volume = 0` 是有效成交量，前一交易日成交量为 0 时 `prev_volume = 0`，不能转成 `NULL`。
5. `mart_stock_quotes_daily` 从 `int_stock_quotes_daily_unadj` 原样透传 `prev_volume`。
6. 模型 YAML、设计文档和 mart passthrough 测试同步更新，避免字段只存在于 SQL 但缺少文档或回归保护。

## 2. 非目标

本计划不做以下事情：

1. 不在本次计划阶段修改 dbt SQL、YAML、tests 或设计文档；本文件只记录实施计划。
2. 不修改 raw contract、staging model、Dagster asset、Furnace Rust 计算引擎或 ClickHouse raw 表。
3. 不新增复权成交量字段，也不按复权因子调整成交量。
4. 不改变 `prev_close_price` 或 `prev_close_price_unadj` 的既有语义。
5. 不把 `prev_volume` 派生为成交量变化率、缩量布尔值或策略指标；这些应由后续 intermediate、mart 或 Furnace 指标计划单独处理。
6. 不在 mart 层重新计算 `prev_volume`；mart 只透传 intermediate 字段。

## 3. 当前事实基线

### 3.1 `int_stock_quotes_daily_unadj`

当前模型位于：

```text
pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql
```

已存在的相关 CTE：

1. `stock_quotes` 从 `stg_baostock__query_history_k_data_plus_daily` 读取未复权日行情字段，包含 `volume`。
2. `quotes_with_prev_trade_date` 左连接 `int_trade_calendar`，得到 `prev_trade_date`。
3. `quotes_with_prev_close_unadj` 按 `security_code + prev_trade_date` 自连接 `stock_quotes`，取 `previous_quotes.close_price as prev_close_price_unadj`。
4. 后续 CTE 继续透传 `volume` 和 `prev_close_price_unadj`，但当前没有 `prev_volume`。

现有字段顺序：

```text
security_code
trade_date
open_price
high_price
low_price
close_price
prev_close_price
prev_close_price_unadj
volume
amount
...
```

当前 YAML 位于：

```text
pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.yml
```

YAML 已记录 `volume` 和 `prev_close_price_unadj`，但没有 `prev_volume`。

### 3.2 `mart_stock_quotes_daily`

当前 mart 位于：

```text
pipeline/elt/models/marts/mart_stock_quotes_daily.sql
```

`quotes` CTE 以 `int_stock_quotes_daily_unadj` 为左表，透传基础行情、成交、市值、股本、股息率和状态字段。最终输出当前包含：

```text
prev_close_price_unadj
volume
amount
```

当前 mart YAML 位于：

```text
pipeline/elt/models/marts/mart_stock_quotes_daily.yml
```

YAML 明确说明交易指标、市值、股本和股息率字段只从 `int_stock_quotes_daily_unadj` 透传，不在 mart 层重算。

当前 mart passthrough singular test 位于：

```text
pipeline/elt/tests/marts/mart_stock_quotes_daily_quote_passthrough_matches.sql
```

该测试已校验 `mart` 与 `int_stock_quotes_daily_unadj` 的基础行情字段逐列一致，但尚未包含 `prev_volume`。

## 4. 字段定义

### 4.1 字段契约

| 字段 | 类型 | 来源/派生 | 说明 |
|------|------|-----------|------|
| `prev_volume` | `Nullable(Int64)` | 前一交易日行情自连接 `previous_quotes.volume` | 当前证券在 `int_trade_calendar.prev_trade_date` 对应日期的成交量。 |

### 4.2 NULL 语义

`prev_volume` 输出 `NULL` 的情况：

1. 该证券当前行没有可用 `prev_trade_date`。
2. `prev_trade_date` 存在，但该证券在前一交易日没有行情行。
3. 前一交易日行情行存在，但 `previous_quotes.volume` 本身为 `NULL`。

`prev_volume` 不输出 `NULL` 的情况：

1. 前一交易日成交量为 `0`。`0` 表示有效成交量，必须保留。
2. 当前日 `volume` 为 `NULL`，但前一交易日 `volume` 非空。当前日成交量缺失不影响 `prev_volume` 的派生。

### 4.3 与相近字段的边界

| 字段 | 口径 |
|------|------|
| `volume` | 当前交易日成交量。 |
| `prev_volume` | 当前证券前一个 A 股交易日的成交量，使用交易日历前日口径。 |
| `prev_close_price` | BaoStock 原始 `preclose` 口径前收盘价。 |
| `prev_close_price_unadj` | 当前证券前一个 A 股交易日的未复权收盘价。 |

`prev_volume` 应与 `prev_close_price_unadj` 保持同一“前一个 A 股交易日”语义，而不是与 BaoStock `preclose` 供应商字段绑定。

## 5. 实施阶段

### Phase 0: 基线确认

范围：

- `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.yml`
- `pipeline/elt/models/marts/mart_stock_quotes_daily.sql`
- `pipeline/elt/models/marts/mart_stock_quotes_daily.yml`
- `pipeline/elt/tests/marts/mart_stock_quotes_daily_quote_passthrough_matches.sql`
- `docs/design/dbt_layer/fleur_intermediate/int_stock_quotes_daily_unadj.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md`

动作：

1. 确认仓库中没有已有 `prev_volume` 字段，避免重复定义或语义冲突。
2. 确认 `int_stock_quotes_daily_unadj` 仍通过 `int_trade_calendar.prev_trade_date` 派生 `prev_close_price_unadj`。
3. 确认 mart 仍以 `int_stock_quotes_daily_unadj` 为左表，字段透传测试仍在使用逐列 `is not distinct from` 比较。

完成标准：

1. 影响文件清单完整。
2. 字段语义与当前 `prev_close_price_unadj` 前交易日口径一致。

### Phase 1: intermediate SQL 增加 `prev_volume`

范围：

- `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql`

动作：

1. 在 `quotes_with_prev_close_unadj` 中增加：

```sql
previous_quotes.volume as prev_volume
```

2. 后续 CTE 逐层透传 `prev_volume`：

```text
quotes_with_shares
quotes_with_metrics
quotes_with_limit_prices
final select
```

3. 字段顺序放在 `prev_close_price_unadj` 之后、`volume` 之前：

```text
prev_close_price
prev_close_price_unadj
prev_volume
volume
```

完成标准：

1. `int_stock_quotes_daily_unadj` 编译后包含 `prev_volume`。
2. `prev_volume` 不参与换手率、市值、股息率、涨跌幅或涨跌停价计算。
3. 现有 `volume` 计算逻辑不变。

### Phase 2: intermediate YAML 和设计文档同步

范围：

- `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.yml`
- `docs/design/dbt_layer/fleur_intermediate/int_stock_quotes_daily_unadj.md`

动作：

1. 在 YAML columns 中新增 `prev_volume`：

```yaml
- name: prev_volume
  description: 当前证券在前一 A 股交易日的成交量；首个交易日、前一交易日无行情或 source 缺口时为 NULL，0 值保留。
  data_type: Nullable(Int64)
```

2. 在设计文档字段表中新增 `prev_volume`，来源写为“前一交易日行情自连接 `volume`”。
3. 在字段顺序建议中把 `prev_volume` 放入前收盘价和当前成交量之间。
4. 在 `prev_close_price_unadj` 设计判断附近补充说明：成交量前值也使用同一交易日历口径。

完成标准：

1. SQL、YAML、设计文档三处字段顺序和语义一致。
2. 文档明确 `0` 成交量是有效值。

### Phase 3: mart SQL、YAML 和设计文档透传

范围：

- `pipeline/elt/models/marts/mart_stock_quotes_daily.sql`
- `pipeline/elt/models/marts/mart_stock_quotes_daily.yml`
- `docs/design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md`

动作：

1. 在 mart `quotes` CTE 中从 `int_stock_quotes_daily_unadj` 读取 `prev_volume`。
2. 在 `quotes_with_financial_valuation` 中透传：

```sql
quotes.prev_volume as prev_volume
```

3. 在最终 `select` 输出 `prev_volume`。
4. 字段顺序与 intermediate 保持一致，放在 `prev_close_price_unadj` 之后、`volume` 之前。
5. mart YAML 新增 `prev_volume` column description，说明来自 `int_stock_quotes_daily_unadj`。
6. mart 设计文档的行情字段列表、字段设计表和字段顺序建议同步加入 `prev_volume`。

完成标准：

1. mart 输出 `prev_volume`。
2. mart 不重新 join `int_trade_calendar` 或自连接行情表计算 `prev_volume`。
3. mart row grain 和 key set 不变。

### Phase 4: 测试和数据校验

范围：

- `pipeline/elt/tests/marts/mart_stock_quotes_daily_quote_passthrough_matches.sql`
- 可选新增 singular test：`pipeline/elt/tests/int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date.sql`

动作：

1. 在 mart passthrough test 中增加：

```sql
and mart.prev_volume is not distinct from quotes.prev_volume
```

2. 新增 intermediate singular test，校验 `prev_volume` 等于前一交易日行情 `volume`：

```sql
with quotes_with_expected as (
    select
        current_quotes.security_code,
        current_quotes.trade_date,
        current_quotes.prev_volume,
        previous_quotes.volume as expected_prev_volume
    from {{ ref('int_stock_quotes_daily_unadj') }} as current_quotes
    left join {{ ref('int_trade_calendar') }} as trade_calendar
        on current_quotes.trade_date = trade_calendar.trade_date
    left join {{ ref('int_stock_quotes_daily_unadj') }} as previous_quotes
        on current_quotes.security_code = previous_quotes.security_code
        and trade_calendar.prev_trade_date = previous_quotes.trade_date
)

select *
from quotes_with_expected
where not (prev_volume is not distinct from expected_prev_volume)
```

3. 使用 `is not distinct from` 保留 NULL 等值语义。
4. 不对 `prev_volume` 增加 `not_null` 测试；首个交易日、前日无行情和 source 缺口都允许为空。

完成标准：

1. intermediate 字段派生有可执行回归保护。
2. mart 透传测试覆盖 `prev_volume`。
3. 现有唯一键和 mart key set 测试不需要改变。

## 6. 最小验证命令

所有 dbt 命令在 `pipeline/` 目录执行：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_quotes_daily_unadj mart_stock_quotes_daily
uv run python elt/scripts/validate_field_glossary.py
```

如果实现阶段同步修改设计文档，额外运行：

```bash
git diff --check
```

定向数据 spot check 建议：

```bash
cd pipeline
uv run dbt show --project-dir elt --profiles-dir elt --inline "
select
    q.security_code,
    q.trade_date,
    q.prev_volume,
    p.volume as expected_prev_volume
from {{ ref('int_stock_quotes_daily_unadj') }} as q
left join {{ ref('int_trade_calendar') }} as cal
    on q.trade_date = cal.trade_date
left join {{ ref('int_stock_quotes_daily_unadj') }} as p
    on q.security_code = p.security_code
    and cal.prev_trade_date = p.trade_date
where q.prev_volume is not distinct from p.volume
limit 20
"
```

说明：`dbt show --inline` 中的 `ref()` 能否按当前 dbt 版本和项目配置解析，应以实际运行为准；如不支持，改用临时 analysis 或定向 model/test 查询，不把该限制绕到生产 SQL 中。

## 7. 验收标准

实现完成后必须满足：

1. `int_stock_quotes_daily_unadj` manifest columns 包含 `prev_volume`，类型文档为 `Nullable(Int64)`。
2. `mart_stock_quotes_daily` manifest columns 包含 `prev_volume`，描述明确来自 `int_stock_quotes_daily_unadj`。
3. `prev_volume` 字段顺序在 `prev_close_price_unadj` 之后、`volume` 之前。
4. `prev_volume` 与 `int_trade_calendar.prev_trade_date` 对应行情 `volume` 完全一致，包括 `NULL` 和 `0`。
5. `mart_stock_quotes_daily.prev_volume` 与 `int_stock_quotes_daily_unadj.prev_volume` 完全一致。
6. `dbt parse` 成功。
7. 定向 `dbt build --select int_stock_quotes_daily_unadj mart_stock_quotes_daily` 成功。
8. `validate_field_glossary.py` 成功。
9. `git diff --check` 成功。

## 8. 风险和处理

| 风险 | 影响 | 处理 |
|------|------|------|
| 把 `prev_volume` 写成 `lag(volume)` | source 缺行或非交易日缺口时语义偏离 `prev_close_price_unadj` | 必须复用 `int_trade_calendar.prev_trade_date` 自连接口径 |
| 把 `0` 成交量当成缺失 | 停牌或无成交场景被错误置空 | SQL 中不使用 `nullIf(volume, 0)`；测试和文档明确 0 值保留 |
| mart 层重算字段 | 口径分散，后续维护困难 | mart 只透传 `int_stock_quotes_daily_unadj.prev_volume` |
| 只改 SQL 不改 YAML/docs/tests | manifest、文档和回归保护缺失 | Phase 2、Phase 3、Phase 4 必须与 SQL 同步完成 |

## 9. 后续维护

1. 计划执行完成后，将本文状态改为 `Completed`，补充完成日期和实际验证命令结果。
2. 如后续需要 `volume_change_rate`、`volume_ratio`、缩量/放量布尔字段，应另立计划，明确是否放在 dbt intermediate、mart 还是 Furnace 指标层。
3. 如果未来 `int_stock_quotes_daily_unadj` 的前交易日口径从交易日历自连接调整，必须同时复核 `prev_close_price_unadj` 和 `prev_volume`。
