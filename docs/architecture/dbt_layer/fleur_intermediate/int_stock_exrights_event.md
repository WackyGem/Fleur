# int_stock_exrights_event 设计

状态：Design

依据：

- Staging model：`ref('stg_eastmoney__dividend_main')`
- Staging model：`ref('stg_eastmoney__dividend_allotment')`
- Staging 设计：`docs/architecture/dbt_layer/fleur_staging/stg_eastmoney__dividend_main.md`
- Staging 设计：`docs/architecture/dbt_layer/fleur_staging/stg_eastmoney__dividend_allotment.md`
- 目标位置：`pipeline/elt/models/intermediate/int_stock_exrights_event.sql`

## 1. 模型定位

股票除权除息事件 intermediate 模型。模型把东方财富分红送转主表和配股事件表合并为证券级除权除息事件表，输出每股现金分红、每股送股、每股转增股、每股配股四类权益组成，并按组成生成 `XR`、`XD`、`DR` 事件标签。

本模型负责把 source-local 的分红送转文本和配股文本解析为可复用的结构化事件，用于后续复权因子、除权除息日历、价格复权和事件分析。它不计算复权因子，不判断税后现金分红，不处理公告版本全生命周期，也不把事件展开到交易日粒度。

## 2. 数据粒度与依赖

- 分红送转依赖：`stg_eastmoney__dividend_main`。
- 配股依赖：`stg_eastmoney__dividend_allotment`。
- 粒度：一行一个 `security_code` + `ex_dividend_date` 的除权除息事件。
- 候选键：`security_code`, `ex_dividend_date`。
- 分红送转主表过滤：`assign_progress = '实施方案'`，`is_unassign = false`，`ex_dividend_date is not null`。
- 配股表没有 `assign_progress` 字段；第一版视为已经落到配股实施事件的 source-local 明细，保留全部 `ex_dividend_date is not null` 行。

抽样检查结论：

- `stg_eastmoney__dividend_main` 中 `assign_progress = '实施方案'` 且非不分配的事件，按 `security_code + ex_dividend_date` 聚合后仍有少量重复键；实现必须先聚合再输出。
- `stg_eastmoney__dividend_allotment` 按 `security_code + ex_dividend_date` 未发现重复。
- 两个来源存在同一 `security_code + ex_dividend_date` 同时命中的情况；实现必须合并为同一事件行，再生成 `DR/XR/XD` 标签。

## 3. 核心口径

权益组成字段全部转换为“每 1 股”口径：

- `cash_dividend_per_share`：每股派现金额，单位元，来自分红送转文本中的 `10派X元`，计算为 `X / 10`。
- `bonus_share_per_share`：每股送股数，来自分红送转文本中的 `10送X股`，计算为 `X / 10`。
- `transfer_share_per_share`：每股转增股数，来自分红送转文本中的 `10转X股`，计算为 `X / 10`。
- `allotment_share_per_share`：每股配股数，来自配股文本中的 `每10股配X股`，计算为 `X / 10`。

分红送转解析文本优先级：

1. `new_profile`：含税方案文本，样例包括 `10派3元(含税)`、`10转4派1.5元(含税)`、`10转4.50股`。
2. `impl_plan_profile`：方案简述，样例包括 `10派3元`、`10转4派1.5元`、`10转4.5`。
3. `impl_plan_newprofile`：方案进度简述，作为兜底解析文本。

解析规则：

- 现金分红：匹配 `10派([0-9]+(?:\.[0-9]+)?)元`。
- 送股：匹配 `10送([0-9]+(?:\.[0-9]+)?)(?:股)?`。
- 转增：匹配 `10转([0-9]+(?:\.[0-9]+)?)(?:股)?`。
- 配股：匹配 `每?10股配([0-9]+(?:\.[0-9]+)?)股`。
- 未匹配的组成输出 `0`，不输出 `NULL`；`0` 表示该事件没有该组成。
- 四个权益组成字段使用非空 `Float64`：本模型只输出已识别的除权除息事件，事件中不存在的组成是确定的 `0`，不是未知值。解析失败或无法覆盖的文本不应通过 `NULL` 混入组成字段，而应保留在 `source_plan_text` / `source_allotment_text` 并由解析覆盖率测试发现。
- 如果同一来源在同一事件键下出现多行，按组成字段取 `max`，日期字段取 `min` 或 `max` 只用于追踪，不用于重复放大权益组成。

事件标签：

| 标签 | 英文 | 中文 | 组成条件 |
|---|---|---|---|
| `XR` | Exclude Right | 除权，只除去领取送股/配股的权利 | 有送股、转股或配股，且无现金分红 |
| `XD` | Exclude Dividend | 除息，只除去领取现金分红的权利 | 有现金分红，且无送股、转股、配股 |
| `DR` | Exclude Dividend & Right | 除权 + 除息，同时除去领股和领钱的权利 | 现金分红与送股/转股/配股同时发生 |

判定表达：

```text
has_cash = cash_dividend_per_share > 0
has_right = bonus_share_per_share > 0
    or transfer_share_per_share > 0
    or allotment_share_per_share > 0

event_tag =
    DR when has_cash and has_right
    XD when has_cash and not has_right
    XR when not has_cash and has_right
```

聚合后 `has_cash = false` 且 `has_right = false` 的行不进入模型输出；这类行代表实施方案存在但第一版规则未解析出可用权益组成，应作为解析覆盖率专项检查。

## 4. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | 两个 staging 来源 | `String` | 股票标准连接代码。 |
| `ex_dividend_date` | 两个 staging 来源 | `Date` | 除权除息日，也是本模型事件日期。 |
| `equity_record_date` | 两个 staging 来源聚合 | `Nullable(Date)` | 股权登记日；同事件多来源时取非空最小值。 |
| `notice_date` | 两个 staging 来源聚合 | `Nullable(Date)` | 事件相关公告日；同事件多来源时取非空最大值。 |
| `report_date` | `dividend_main.report_date` | `Nullable(Date)` | 分红送转报告期截止日；仅分红主表提供。 |
| `report_period_label` | `dividend_main.report_period_label` | `Nullable(String)` | 分红送转报告期标签；保留 source-local 文本。 |
| `cash_dividend_per_share` | `10派X元 / 10` | `Float64` | 每股派现金额，单位元，含税口径。 |
| `bonus_share_per_share` | `10送X股 / 10` | `Float64` | 每股送股数。 |
| `transfer_share_per_share` | `10转X股 / 10` | `Float64` | 每股转增股数。 |
| `allotment_share_per_share` | `每10股配X股 / 10` | `Float64` | 每股配股数。 |
| `allotment_price_yuan` | `dividend_allotment.issue_price` | `Nullable(Float64)` | 配股价格，单位元；后续复权因子计算可能需要。 |
| `event_tag` | 组成判定 | `Enum8` 或 `LowCardinality(String)` | 事件标签：`XR`、`XD`、`DR`。如 dbt/ClickHouse 测试暂不约束 Enum，可先用 `LowCardinality(String)` 并加 accepted-values。 |
| `has_cash_dividend` | 组成判定 | `Bool` | 是否包含现金分红。 |
| `has_share_right` | 组成判定 | `Bool` | 是否包含送股、转增或配股。 |
| `source_has_dividend_main` | 来源追踪 | `Bool` | 是否来自分红送转主表。 |
| `source_has_allotment` | 来源追踪 | `Bool` | 是否来自配股表。 |
| `source_plan_text` | 分红送转解析文本 | `Nullable(String)` | 用于解析现金、送股、转增的原始文本，便于排查解析覆盖率。 |
| `source_allotment_text` | 配股解析文本 | `Nullable(String)` | 用于解析配股比例的原始文本。 |

字段顺序建议：

1. 主键与日期：`security_code`, `ex_dividend_date`, `equity_record_date`, `notice_date`
2. 报告期追踪：`report_date`, `report_period_label`
3. 权益组成：`cash_dividend_per_share`, `bonus_share_per_share`, `transfer_share_per_share`, `allotment_share_per_share`, `allotment_price_yuan`
4. 事件标签：`event_tag`, `has_cash_dividend`, `has_share_right`
5. 来源追踪：`source_has_dividend_main`, `source_has_allotment`, `source_plan_text`, `source_allotment_text`

## 5. SQL 逻辑建议

```sql
with dividend_main_filtered as (
    select
        security_code,
        ex_dividend_date,
        equity_record_date,
        notice_date,
        report_date,
        report_period_label,
        coalesce(
            nullIf(new_profile, ''),
            nullIf(impl_plan_profile, ''),
            nullIf(impl_plan_newprofile, '')
        ) as plan_text
    from {{ ref('stg_eastmoney__dividend_main') }}
    where assign_progress = '实施方案'
      and is_unassign = false
      and ex_dividend_date is not null
),

dividend_main_components as (
    select
        security_code,
        ex_dividend_date,
        min(equity_record_date) as equity_record_date,
        max(notice_date) as notice_date,
        max(report_date) as report_date,
        anyLast(report_period_label) as report_period_label,
        max(
            coalesce(
                toFloat64OrNull(regexpExtract(plan_text, '10派([0-9]+(?:\\.[0-9]+)?)元', 1)) / 10,
                0
            )
        ) as cash_dividend_per_share,
        max(
            coalesce(
                toFloat64OrNull(regexpExtract(plan_text, '10送([0-9]+(?:\\.[0-9]+)?)(?:股)?', 1)) / 10,
                0
            )
        ) as bonus_share_per_share,
        max(
            coalesce(
                toFloat64OrNull(regexpExtract(plan_text, '10转([0-9]+(?:\\.[0-9]+)?)(?:股)?', 1)) / 10,
                0
            )
        ) as transfer_share_per_share,
        true as source_has_dividend_main,
        anyLast(plan_text) as source_plan_text
    from dividend_main_filtered
    group by
        security_code,
        ex_dividend_date
),

allotment_components as (
    select
        security_code,
        ex_dividend_date,
        min(equity_record_date) as equity_record_date,
        max(notice_date) as notice_date,
        max(
            coalesce(
                toFloat64OrNull(regexpExtract(event_explain, '每?10股配([0-9]+(?:\\.[0-9]+)?)股', 1)) / 10,
                0
            )
        ) as allotment_share_per_share,
        max(issue_price) as allotment_price_yuan,
        true as source_has_allotment,
        anyLast(event_explain) as source_allotment_text
    from {{ ref('stg_eastmoney__dividend_allotment') }}
    where ex_dividend_date is not null
    group by
        security_code,
        ex_dividend_date
),

event_keys as (
    select security_code, ex_dividend_date from dividend_main_components
    union distinct
    select security_code, ex_dividend_date from allotment_components
)
```

后续步骤：

1. 从 `event_keys` 分别 `LEFT JOIN` 聚合后的 `dividend_main_components` 和 `allotment_components`。
2. 使用 `coalesce(..., 0)` 合成四类权益组成。
3. 派生 `has_cash_dividend` 和 `has_share_right`。
4. 过滤掉四类权益组成均为 0 的行。
5. 按判定表达生成 `event_tag`。

ClickHouse 实现注意：

- 按 `query-join-filter-before`，先在来源 CTE 中过滤 `assign_progress = '实施方案'`、`is_unassign = false` 和 `ex_dividend_date is not null`，再聚合和 join，避免全表 join 后再过滤。
- 按 `query-join-null-handling`，ClickHouse 默认 `join_use_nulls = 0` 时外连接未命中的右表字段可能是类型默认值；实现中布尔来源标记应用 `event_keys` 命中情况或显式 `coalesce` 处理，避免把未命中误判为真实 `false` 或空字符串。
- 按 `schema-types-enum`，`event_tag` 是固定三值集合，目标表类型可优先考虑 `Enum8('XR' = 1, 'XD' = 2, 'DR' = 3)`；如项目 dbt 类型测试暂不方便覆盖 Enum，先用 `LowCardinality(String)`。
- 按 `schema-types-lowcardinality`，`report_period_label` 等低基数字符串可使用 `LowCardinality(String)` 或 `LowCardinality(Nullable(String))`。

## 6. 测试建议

- 模型级组合唯一：`security_code`, `ex_dividend_date`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `ex_dividend_date`: `not_null`。
- `event_tag`: `not_null`，accepted values 为 `XR`, `XD`, `DR`。
- 四个权益组成字段：`not_null`，且应大于等于 0。
- `cash_dividend_per_share`: 单位为元/股，允许 0。
- `bonus_share_per_share`, `transfer_share_per_share`, `allotment_share_per_share`: 单位为股/股，允许 0。
- `has_cash_dividend`: `not_null`。
- `has_share_right`: `not_null`。
- 定向数据测试：
  - `event_tag = 'XD'` 时 `cash_dividend_per_share > 0`，且三类股权组成均为 0。
  - `event_tag = 'XR'` 时 `cash_dividend_per_share = 0`，且至少一类股权组成大于 0。
  - `event_tag = 'DR'` 时 `cash_dividend_per_share > 0`，且至少一类股权组成大于 0。
  - 输出行不得出现四类权益组成全为 0。
  - `source_has_allotment = true` 时 `source_allotment_text` 非空。
  - `source_has_dividend_main = true` 时 `source_plan_text` 非空。

## 7. 延后事项

- 覆盖非 `10派`、`10送`、`10转`、`每10股配` 的少数异常方案文本。
- 对分红送转主表同一 `security_code + ex_dividend_date` 多行的公告版本做更精细优先级裁决。
- 解析税前/税后、扣税说明和差异化分红。
- 配股认购价格、缴款起止日、停复牌日等更完整配股事件信息。
- 与交易日历对齐，校验 `ex_dividend_date` 是否为交易日。
- 结合前收盘价、现金分红和配股价格计算复权因子。
