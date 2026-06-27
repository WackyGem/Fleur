# 0001 ClickHouse Date-First ORDER BY Optimization

日期：2026-06-11

状态：Proposed

## 结论

基本同意将证券日频事实源、指标表和选股特征表的主扫描路径调整为：

```sql
ORDER BY (trade_date, security_code)
```

但这不是所有 raw / stg / calc / int / mart 对象的无条件规则。适用边界是：表的主访问路径是按交易日或交易日范围批量扫描全市场，再按 `security_code` 做明细连接、排名、筛选或回测。基础行情宽表 `mart_stock_quotes_daily` 本轮明确保持当前 `ORDER BY (security_code, trade_date)`，不纳入 P0 重建。证券维表、交易日历、财报源表、股本历史和其他 ASOF 状态表仍应按各自访问路径单独评估。

staging 层当前主要是 dbt view，没有 ClickHouse `MergeTree ORDER BY`，本优化主要落在 raw ClickHouse 表、Furnace calculation 表和 dbt materialized table。

## 依据

Workload：

- 类型：market data / financial services。
- 数据形态：A 股日频行情、技术指标和选股宽表，主粒度为 `(trade_date, security_code)`。
- 主要查询：按交易日构建模型、日截面选股、回测调仓日读取、按日期范围验证指标。
- 主要风险：ClickHouse `ORDER BY` 建表后不能直接修改，错误排序键需要重建表。

规则检查：

- Per `schema-pk-plan-before-creation`：`ORDER BY` 是物理排序和 sparse primary index，需在建表前按查询模式确认。
- Per `schema-pk-prioritize-filters`：高频过滤列应进入 `ORDER BY` 前缀；当前建模和选股最常见过滤是 `trade_date` / 日期范围。
- Per `schema-pk-cardinality-order`：低基数字段应靠前；在年度分区内，`trade_date` 约 250 个取值，低于全市场证券数。
- Per `schema-pk-filter-on-orderby`：跳过排序键前缀会降低索引利用率；`ORDER BY (security_code, trade_date)` 对只按日期扫全市场的查询不友好。
- Per `query-join-filter-before`：大表 join 前应先按日期范围过滤；date-first 排序键更符合这一构建路径。
- Per `query-join-choose-algorithm`：大表 join 应关注右表大小、排序键和 join algorithm；P0 日频表统一 `(trade_date, security_code)` 可降低指标表等值 join 的排序错位。`mart_stock_quotes_daily` 作为显式例外保持 security-first。
- Per `schema-partition-lifecycle`：`PARTITION BY toYear(trade_date)` 继续服务生命周期和年度回填，查询性能主要仍靠 `ORDER BY`。

官方文档参考：

- ClickHouse Choosing a Primary Key: https://clickhouse.com/docs/best-practices/choosing-a-primary-key
- ClickHouse sparse primary indexes: https://clickhouse.com/docs/guides/best-practices/sparse-primary-indexes
- ClickHouse query optimization / `EXPLAIN indexes = 1`: https://clickhouse.com/docs/issues/query-optimization
- ClickHouse minimize and optimize JOINs: https://clickhouse.com/docs/best-practices/minimize-optimize-joins

## 调整清单

### P0：应调整

| 层级 | 对象 | 当前排序键 | 目标排序键 | 说明 |
|---|---|---:|---:|---|
| raw | `fleur_raw.baostock__query_history_k_data_plus_daily` | `(code, date)` | `(date, code)` | 日频行情 raw 是下游所有行情、复权、指标和 marts 的事实源；当前 `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml` 需改。 |
| int | `fleur_intermediate.int_stock_quotes_daily_unadj` | `(security_code, trade_date)` | `(trade_date, security_code)` | 行情未复权日表，模型构建和选股优先按交易日读取全市场。 |
| int | `fleur_intermediate.int_stock_adjustment_factor` | `(security_code, trade_date)` | `(trade_date, security_code)` | 与日行情按 `(trade_date, security_code)` 等值 join；目标排序键应对齐。 |
| int | `fleur_intermediate.int_stock_quotes_daily_adj` | `(security_code, trade_date)` | `(trade_date, security_code)` | Furnace 默认输入表和后续选股特征源；应服务日期范围批量读取。 |
| mart | `fleur_marts.mart_stock_trend_indicator` | `(security_code, trade_date)` | `(trade_date, security_code)` | 技术指标 mart，用于每日全市场筛选。 |
| mart | `fleur_marts.mart_stock_momentum_indicator` | `(security_code, trade_date)` | `(trade_date, security_code)` | 技术指标 mart，用于每日全市场筛选。 |
| mart | `fleur_marts.mart_stock_volume_indicator` | `(security_code, trade_date)` | `(trade_date, security_code)` | 技术指标 mart，用于每日全市场筛选。 |

### 本轮明确保持不改

| 层级 | 对象 | 当前排序键 | 说明 |
|---|---|---:|---|
| mart | `fleur_marts.mart_stock_quotes_daily` | `(security_code, trade_date)` | 用户最新约束：基础行情宽表保持当前排序键，不纳入本轮 date-first 调整。若后续日截面选股需要更强日期前缀，可另建选股特征 mart、projection 或专项评估。 |

### P0：已符合，无需调整

| 层级 | 对象 | 当前排序键 | 说明 |
|---|---|---:|---|
| calc | `fleur_calculation.calc_stock_kdj_daily` | `(trade_date, security_code)` | Furnace DDL 已符合。 |
| calc | `fleur_calculation.calc_stock_ma_daily` | `(trade_date, security_code)` | Furnace DDL 已符合。 |
| calc | `fleur_calculation.calc_stock_rsi_daily` | `(trade_date, security_code)` | Furnace DDL 已符合。 |
| calc | `fleur_calculation.calc_stock_boll_daily` | `(trade_date, security_code)` | Furnace DDL 已符合。 |
| calc | `fleur_calculation.calc_stock_macd_daily` | `(trade_date, security_code)` | Furnace DDL 已符合。 |
| calc | `fleur_calculation.calc_stock_price_pattern_daily` | `(trade_date, security_code)` | Furnace DDL 已符合。 |
| raw | `fleur_raw.ths__limit_up_pool_compacted` | `(date, code)` | 日频事件源已 date-first。 |
| raw | `fleur_raw.jiuyan__action_field_compacted` | `(date, code)` | 日频事件源已 date-first。 |
| raw | `fleur_raw.sina__trade_calendar` | `(trade_date)` | 单列日历表已符合访问路径。 |

### P1：专项评估后再决定

| 层级 | 对象 | 当前排序键 | 候选排序键 | 暂缓原因 |
|---|---|---:|---:|---|
| int | `fleur_intermediate.int_stock_financial_valuation` | `(security_code, report_date)` | `(report_date, security_code)` | 该表不是交易日日频表；当前主要承担财报期估值事实和 ASOF enrichment，需基准测试后再改。 |
| int | `fleur_intermediate.int_stock_shares_history` | `(security_code, effective_date)` | `(effective_date, security_code)` | 股本历史用于按证券 ASOF 到行情，当前 security-first 可能仍合理。 |
| int | `fleur_intermediate.int_stock_exrights_event` | `(security_code, ex_dividend_date)` | `(ex_dividend_date, security_code)` | 事件表如用于日历扫描可改；如用于单证券复权链路则需保留或另建派生表。 |
| raw | EastMoney 财报 / 分红 / 股本 raw 表 | 多数为 `(SECUCODE, *_DATE)` | `(*_DATE, SECUCODE)` | 源表是报告期、公告日或除权日事实，不是统一 `trade_date` 粒度；需按下游查询画像评估。 |
| raw | `fleur_raw.baostock__query_stock_basic` | `(code)` | 不变 | 证券基础快照维表，按证券代码访问。 |
| raw | `fleur_raw.jiuyan__industry_list` | `(industry_id)` | 不变 | 行业维表。 |
| raw | `fleur_raw.jiuyan__industry_ocr_snapshot` | `(industry_id, image_filename, ocr_row_index)` | 不变 | OCR 明细表，不属于证券日频事实表。 |

## P1 专项评估实施方案

### 目标

P1 评估不是证明所有表都应 date-first，而是为每个候选对象给出可复核结论：

- `change`：物理表排序键改为 date-first。
- `keep`：保留当前 security-first 或维表排序键。
- `dual-path`：保留当前主表，同时新增 date-first 派生表、projection 或特定 mart。
- `defer`：缺少真实查询或数据规模证据，继续观察。

每个结论必须附带查询画像、`EXPLAIN indexes = 1` 证据、性能指标、数据一致性检查和迁移影响。

### 评估产物

每次专项评估必须新增一份运行报告：

```text
docs/jobs/reports/YYYY-MM-DD-clickhouse-p1-order-by-evaluation.md
```

报告至少包含：

- 评估日期、ClickHouse 版本、数据库实例、测试窗口、执行人或 agent。
- 评估对象和候选排序键。
- 当前 `system.tables` 中的 `engine`、`sorting_key`、`primary_key`、`partition_key`、`total_rows`、`total_bytes`。
- 查询画像表：查询名称、业务场景、频率、过滤条件、join key、是否 ASOF、期望 SLA。
- 基准 SQL、候选 SQL、`EXPLAIN indexes = 1` 输出摘要。
- `read_rows`、`read_bytes`、`query_duration_ms`、`memory_usage`、`parts` / `granules` 剪枝情况。
- 数据一致性检查结果。
- 决策：`change` / `keep` / `dual-path` / `defer`。
- 后续动作：是否需要新增 `docs/plans/NNNN-*.md` 执行迁移。

### 阶段 0：准备和边界

执行前先确认：

- 不在生产高峰期跑未限制的大表扫描。
- 所有探索 SQL 都带时间窗口、`LIMIT` 或 `FORMAT Null`，并设置查询限制。
- 不直接在原表上尝试修改 `ORDER BY`；候选排序键只能通过影子表、临时库或 dbt 分支模型验证。
- P1 对象只评估排序键，不顺带改变字段语义、复权口径、财报可见性规则或 partition 策略。

建议使用独立 query id 前缀，方便从 `system.query_log` 回收指标；通过 `clickhouse-client --query_id ...`、HTTP 参数或客户端配置传入，不依赖 SQL 文本里的业务注释。

```sql
SET max_execution_time = 60;
SET max_rows_to_read = 1000000000;
SET max_bytes_to_read = 100000000000;
SET timeout_before_checking_execution_speed = 0;
```

### 阶段 1：发现当前物理事实

先从 ClickHouse system tables 读取当前事实。不要用 dbt SQL 或历史文档替代真实物理表状态。

```sql
SELECT
    database,
    name,
    engine,
    sorting_key,
    primary_key,
    partition_key,
    total_rows,
    formatReadableSize(total_bytes) AS total_size
FROM system.tables
WHERE database IN ('fleur_raw', 'fleur_intermediate')
  AND name IN (
      'int_stock_financial_valuation',
      'int_stock_shares_history',
      'int_stock_exrights_event',
      'eastmoney__balance',
      'eastmoney__income_ytd',
      'eastmoney__income_sq',
      'eastmoney__cashflow_ytd',
      'eastmoney__cashflow_sq',
      'eastmoney__dividend_main',
      'eastmoney__dividend_allotment',
      'eastmoney__equity_history',
      'eastmoney__freeholders',
      'baostock__query_stock_basic',
      'jiuyan__industry_list',
      'jiuyan__industry_ocr_snapshot'
  )
ORDER BY database, name;
```

补充字段、分区和数据规模：

```sql
SELECT
    database,
    table,
    name,
    type,
    position
FROM system.columns
WHERE database IN ('fleur_raw', 'fleur_intermediate')
  AND table IN ('int_stock_financial_valuation', 'int_stock_shares_history', 'int_stock_exrights_event')
ORDER BY database, table, position;
```

```sql
SELECT
    database,
    table,
    count() AS active_parts,
    sum(rows) AS rows,
    sum(bytes_on_disk) AS bytes_on_disk,
    formatReadableSize(sum(bytes_on_disk)) AS bytes_on_disk_readable,
    min(partition) AS min_partition,
    max(partition) AS max_partition
FROM system.parts
WHERE active
  AND database IN ('fleur_raw', 'fleur_intermediate')
GROUP BY database, table
ORDER BY bytes_on_disk DESC;
```

### 阶段 2：建立查询画像

对每张 P1 表，先用代码搜索列出真实消费者：

```bash
rg -n "int_stock_financial_valuation|int_stock_shares_history|int_stock_exrights_event|eastmoney__balance|eastmoney__income_ytd|eastmoney__income_sq|eastmoney__cashflow_ytd|eastmoney__cashflow_sq|eastmoney__dividend_main|eastmoney__dividend_allotment|eastmoney__equity_history|eastmoney__freeholders" pipeline docs engines -g '!**/target/**'
```

每张表至少归纳 3 类查询：

- 构建查询：dbt model、Furnace input、未来策略特征构建。
- 服务查询：选股、回测、数据检查、人工分析。
- 维护查询：回填、重建、行数核验、分区替换。

查询画像表模板：

| 对象 | 查询名称 | 场景 | 过滤条件 | join key | 访问形态 | 频率 | SLA | 排序键倾向 |
|---|---|---|---|---|---|---|---|---|
| `int_stock_financial_valuation` | valuation ASOF 到日行情 | mart 构建 | `trade_date/report_date` 范围 | `security_code`, date inequality | ASOF join | 每次重建 | 分钟级 | 待测 |
| `int_stock_shares_history` | shares ASOF 到报告期或交易日 | int/mart 构建 | `effective_date` 范围或单证券 | `security_code`, date inequality | ASOF join | 每次重建 | 分钟级 | 待测 |

### 阶段 3：定义候选排序键

每张 P1 表至少比较当前排序键和一个 date-first 候选。不要只比较单条查询。

| 对象 | 当前排序键 | 候选 A | 候选 B | 可能结论 |
|---|---|---|---|---|
| `int_stock_financial_valuation` | `(security_code, report_date)` | `(report_date, security_code)` | 保留主表并新增交易日特征 mart | `keep` / `change` / `dual-path` |
| `int_stock_shares_history` | `(security_code, effective_date)` | `(effective_date, security_code)` | 保留主表并新增日频展开表 | `keep` / `dual-path` |
| `int_stock_exrights_event` | `(security_code, ex_dividend_date)` | `(ex_dividend_date, security_code)` | 保留事件表并新增日历事件 mart | `change` / `dual-path` |
| EastMoney 财报 raw | `(SECUCODE, REPORT_DATE)` 等 | `(REPORT_DATE, SECUCODE)` 等 | 不改 raw，改 downstream int/mart | `keep` / `dual-path` |

候选影子表命名建议：

```text
fleur_scratch.orderby_eval__<source_table>__date_first
fleur_scratch.orderby_eval__<source_table>__security_first
```

影子表原则：

- 使用与原表相同字段和 engine。
- 分区策略保持一致，避免把 partition 变化混入排序键评估。
- 只复制评估窗口内数据，除非全量规模是必须评估对象。
- 评估完成后删除 scratch 表。

示例：

```sql
CREATE DATABASE IF NOT EXISTS fleur_scratch;

CREATE TABLE fleur_scratch.orderby_eval__int_stock_exrights_event__date_first
ENGINE = MergeTree()
PARTITION BY toYear(ex_dividend_date)
ORDER BY (ex_dividend_date, security_code)
AS
SELECT
    security_code,
    ex_dividend_date,
    equity_record_date,
    notice_date,
    report_date,
    report_period_label,
    cash_dividend_per_share,
    bonus_share_per_share,
    transfer_share_per_share,
    allotment_share_per_share,
    allotment_price_yuan,
    event_tag,
    has_cash_dividend,
    has_share_right,
    source_has_dividend_main,
    source_has_allotment,
    source_plan_text,
    source_allotment_text
FROM fleur_intermediate.int_stock_exrights_event
WHERE 0;

INSERT INTO fleur_scratch.orderby_eval__int_stock_exrights_event__date_first
SELECT
    security_code,
    ex_dividend_date,
    equity_record_date,
    notice_date,
    report_date,
    report_period_label,
    cash_dividend_per_share,
    bonus_share_per_share,
    transfer_share_per_share,
    allotment_share_per_share,
    allotment_price_yuan,
    event_tag,
    has_cash_dividend,
    has_share_right,
    source_has_dividend_main,
    source_has_allotment,
    source_plan_text,
    source_allotment_text
FROM fleur_intermediate.int_stock_exrights_event
WHERE ex_dividend_date >= toDate('2018-01-01')
SETTINGS max_execution_time = 60,
         max_rows_to_read = 1000000000,
         max_bytes_to_read = 100000000000,
         timeout_before_checking_execution_speed = 0;
```

### 阶段 4：执行 Explain 对照

每个候选排序键至少覆盖三类 `EXPLAIN indexes = 1`：

1. 日期截面或日期范围查询。
2. 单证券长历史查询。
3. 下游真实 join / ASOF join 的简化版。

示例：事件表日期路径。

```sql
EXPLAIN indexes = 1
SELECT security_code, ex_dividend_date
FROM fleur_scratch.orderby_eval__int_stock_exrights_event__date_first
WHERE ex_dividend_date BETWEEN toDate('2024-01-01') AND toDate('2024-12-31')
SETTINGS max_execution_time = 30,
         max_rows_to_read = 1000000000,
         timeout_before_checking_execution_speed = 0;
```

示例：事件表单证券路径。

```sql
EXPLAIN indexes = 1
SELECT security_code, ex_dividend_date
FROM fleur_scratch.orderby_eval__int_stock_exrights_event__date_first
WHERE security_code = '000001.SZ'
  AND ex_dividend_date BETWEEN toDate('2018-01-01') AND toDate('2026-06-01')
SETTINGS max_execution_time = 30,
         max_rows_to_read = 1000000000,
         timeout_before_checking_execution_speed = 0;
```

报告中只需摘录：

- `PrimaryKey` 的 `Keys`。
- `Condition` 是否命中第一排序列。
- `Parts: selected/total`。
- `Granules: selected/total`。
- 是否出现跳过排序键前缀导致的弱剪枝。

### 阶段 5：执行性能基准

用同一组查询分别跑当前表和候选影子表。推荐每条查询运行 3 次，记录中位数和最差值。返回大量数据的查询使用 `FORMAT Null`，避免客户端传输影响结果。

```sql
SELECT count()
FROM fleur_scratch.orderby_eval__int_stock_exrights_event__date_first
WHERE ex_dividend_date BETWEEN toDate('2024-01-01') AND toDate('2024-12-31')
SETTINGS max_execution_time = 60,
         max_rows_to_read = 1000000000,
         max_bytes_to_read = 100000000000,
         timeout_before_checking_execution_speed = 0;
```

```sql
SELECT
    query_id,
    query_duration_ms,
    read_rows,
    read_bytes,
    memory_usage
FROM system.query_log
WHERE type = 'QueryFinish'
  AND query_id LIKE 'orderby_eval_%'
  AND event_time >= now() - INTERVAL 1 HOUR
ORDER BY event_time DESC;
```

性能判断不只看耗时。优先级：

1. `read_rows` / `read_bytes` 是否明显减少。
2. `EXPLAIN` 中 parts / granules 是否更可控。
3. `query_duration_ms` 是否稳定下降。
4. `memory_usage` 是否下降或至少不显著上升。
5. 下游 dbt/Furnace 构建时间是否不回退。

### 阶段 6：数据正确性检查

排序键评估不能改变业务结果。每张候选表必须和原表比较：

```sql
SELECT count() FROM <original_table>;
SELECT count() FROM <candidate_table>;
```

```sql
SELECT
    count() AS rows,
    uniqExact(security_code, <date_column>) AS unique_keys,
    min(<date_column>) AS min_date,
    max(<date_column>) AS max_date
FROM <candidate_table>;
```

对 ASOF / join 消费路径，额外验证 join 后粒度：

```sql
SELECT
    count() AS rows,
    uniqExact(security_code, trade_date) AS unique_daily_keys
FROM (
    -- 放入真实下游 join 的最小可复现 SQL
)
SETTINGS max_execution_time = 60,
         max_rows_to_read = 1000000000,
         timeout_before_checking_execution_speed = 0;
```

验收要求：

- 行数、日期范围、唯一键数量与原表一致。
- 下游 join 不产生放大。
- NULL 分布、金额/股数/估值字段聚合结果无不可解释差异。
- 对财报和估值相关表，不能引入未来函数；如涉及回测，应单独核查 `available_date` / `publish_date` 语义。

### 阶段 7：决策阈值

建议按下面规则出结论：

| 结论 | 条件 |
|---|---|
| `change` | 主要查询 60% 以上按日期或日期范围访问；date-first 的 `read_rows` 或 `read_bytes` 明显下降；单证券路径无不可接受回退；迁移成本可控。 |
| `keep` | 主要路径是单证券 ASOF、点查、维表 lookup 或低频维护；date-first 无明显收益或导致关键路径回退。 |
| `dual-path` | 日期截面和单证券历史都是高频关键路径，单一排序键无法同时满足；新增派生表、projection 或 mart 比重排主表更稳妥。 |
| `defer` | 查询画像不足、数据规模太小、候选表无法代表全量、或当前没有明确消费者。 |

`change` 结论后不直接改生产表，必须新增执行计划，说明重建方式、回滚路径、依赖暂停窗口和验收命令。

### P1 对象评估重点

`int_stock_financial_valuation`：

- 重点确认它是更多服务 `report_date` 批量构建，还是按 `security_code` 做 ASOF enrichment。
- 如选股回测消费的是交易日可见估值，优先考虑新增日频 valuation feature mart，而不是直接重排报告期表。
- 必须检查财务数据可见日期，避免把报告期 `report_date` 当成回测可用日期。

`int_stock_shares_history`：

- 重点确认股本历史是否主要作为按证券递推的状态表。
- 如果下游高频需要每日市值、流通市值、自由流通市值，优先考虑新增日频展开表或特征 mart。
- 若原表行数远小于日频行情表，排序键收益可能低于 join 语义清晰度。

`int_stock_exrights_event`：

- 如果主要按日期找当天除权事件，date-first 更可能成立。
- 如果主要用于单证券复权链路或事件回放，security-first 可能仍合理。
- 可以选择 dual-path：事件主表保留 security-first，新增 `mart_stock_exrights_event_daily` 服务日截面。

EastMoney 财报 / 分红 / 股本 raw：

- raw 层优先表达源事实和回填稳定性，不建议为了某个 mart 查询直接大规模重排。
- 对频繁消费的下游形态，优先在 intermediate 或 mart 层建立 date-first 派生表。
- 若 raw 表本身成为高频分析入口，再评估 contract 中 `clickhouse_raw.order_by`。

维表和 OCR 表：

- `baostock__query_stock_basic`、`jiuyan__industry_list`、`jiuyan__industry_ocr_snapshot` 默认 `keep`。
- 只有当出现大规模日期过滤或新事实粒度时，才重新进入 P1 评估。

## 改动范围

代码和配置：

- `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml`
  - `clickhouse_raw.order_by` 从 `[code, date]` 调整为 `[date, code]`。
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql`
  - dbt `config(order_by=...)` 改为 `(trade_date, security_code)`。
- `pipeline/elt/models/intermediate/int_stock_adjustment_factor.sql`
  - dbt `config(order_by=...)` 改为 `(trade_date, security_code)`。
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
  - dbt `config(order_by=...)` 改为 `(trade_date, security_code)`。
- `pipeline/elt/models/marts/mart_stock_trend_indicator.sql`
  - dbt `config(order_by=...)` 改为 `(trade_date, security_code)`。
- `pipeline/elt/models/marts/mart_stock_momentum_indicator.sql`
  - dbt `config(order_by=...)` 改为 `(trade_date, security_code)`。
- `pipeline/elt/models/marts/mart_stock_volume_indicator.sql`
  - dbt `config(order_by=...)` 改为 `(trade_date, security_code)`。
- `pipeline/elt/models/marts/mart_stock_quotes_daily.sql`
  - 本轮不修改，继续保持 `(security_code, trade_date)`。

测试和生成物：

- `pipeline/scheduler/tests/unit/clickhouse/test_clickhouse_specs.py`
  - raw Baostock spec 断言需从 `("code", "date")` 改为 `("date", "code")`。
- `pipeline/scheduler/tests/unit/clickhouse/test_clickhouse_sql.py`
  - DDL 断言需从 `ORDER BY (code, date)` 改为 `ORDER BY (date, code)`；实际 SQL 渲染仍保留 ClickHouse 标识符引用。
- `docs/references/data_dict/baostock__query_history_k_data_plus_daily.md`
  - 由 `fleur-contracts generate` 更新。
- `pipeline/elt/target/`、dbt logs 等构建产物不提交。

运行和迁移：

- ClickHouse `ORDER BY` 不能用普通 `ALTER` 原地修改；现有表需要重建。
- dbt table models 可以通过目标模型重建落新排序键。
- raw 表应走影子表重建、校验、rename/swap 或环境允许时 drop/recreate；不要只改 contract 而不重建物理表。
- calculation 表当前已符合，不纳入重建范围。
- `mart_stock_quotes_daily` 是本轮显式例外，不修改 SQL 配置，也不纳入定向重建；除非后续有内容变更，不应为了排序键优化重建该表。

## 推荐执行顺序

1. 先调整 dbt intermediate 和指标 marts 的 `order_by` 配置；不要修改 `mart_stock_quotes_daily`。
2. 定向重建：

```bash
cd pipeline
uv run dbt build \
  --project-dir elt \
  --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_trend_indicator mart_stock_momentum_indicator mart_stock_volume_indicator
```

3. 修改 Baostock 日行情 raw contract 和相关单元测试。
4. 运行 contract 校验和生成物检查。
5. 对 raw 物理表制定单独迁移步骤，完成重建后再放开依赖任务。
6. 对 P1 表执行 `EXPLAIN indexes = 1` 和查询基准，确认是否需要第二批调整。

## 验收 Checklist

### 静态检查

- [ ] `rg -n "order_by='\\(security_code, trade_date\\)'" pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql pipeline/elt/models/intermediate/int_stock_adjustment_factor.sql pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql pipeline/elt/models/marts/mart_stock_trend_indicator.sql pipeline/elt/models/marts/mart_stock_momentum_indicator.sql pipeline/elt/models/marts/mart_stock_volume_indicator.sql` 不再命中。
- [ ] `rg -n "order_by='\\(security_code, trade_date\\)'" pipeline/elt/models/marts/mart_stock_quotes_daily.sql` 仍能命中，确认该显式例外未被误改。
- [ ] `rg -n "order_by:|order_by=|ORDER BY"` 人工确认 P1 例外均有记录。
- [ ] `uv run fleur-contracts validate` 通过。
- [ ] `uv run fleur-contracts generate --check` 通过。
- [ ] `uv run dbt parse --project-dir elt --profiles-dir elt` 通过。
- [ ] `uv run pytest scheduler/tests/unit/clickhouse contract_tools/tests` 通过。

### 物理表检查

```sql
SELECT
    database,
    name,
    sorting_key,
    partition_key
FROM system.tables
WHERE database IN ('fleur_raw', 'fleur_intermediate', 'fleur_marts', 'fleur_calculation')
  AND name IN (
      'baostock__query_history_k_data_plus_daily',
      'int_stock_quotes_daily_unadj',
      'int_stock_adjustment_factor',
      'int_stock_quotes_daily_adj',
      'mart_stock_quotes_daily',
      'mart_stock_trend_indicator',
      'mart_stock_momentum_indicator',
      'mart_stock_volume_indicator',
      'calc_stock_kdj_daily',
      'calc_stock_ma_daily',
      'calc_stock_rsi_daily',
      'calc_stock_boll_daily',
      'calc_stock_macd_daily',
      'calc_stock_price_pattern_daily'
  )
ORDER BY database, name;
```

验收标准：

- [ ] P0 表 `sorting_key` 均为 `trade_date, security_code` 或源字段名对应的 `date, code`。
- [ ] `mart_stock_quotes_daily` `sorting_key` 仍为 `security_code, trade_date`。
- [ ] calc 表仍为 `trade_date, security_code`。
- [ ] 分区键仍为年度粒度：`toYear(trade_date)` / raw `year`。

### 数据一致性

- [ ] 每张 P0 表重建前后 `count()` 一致。
- [ ] 每张 P0 表重建前后 `uniqExact(security_code, trade_date)` 或源字段 `(code, date)` 唯一性一致。
- [ ] 每张 P0 表 `min(date)` / `max(date)` 或 `min(trade_date)` / `max(trade_date)` 一致。
- [ ] 关键指标 NULL 分布与重建前一致，允许仅因上游数据刷新产生可解释差异。
- [ ] dbt tests 中 unique、not_null、字段格式和指标边界测试均通过。

### 性能验证

使用同一 ClickHouse 环境、同一日期区间、冷热缓存各测一轮，至少验证：

```sql
EXPLAIN indexes = 1
SELECT count()
FROM fleur_marts.mart_stock_trend_indicator
WHERE trade_date BETWEEN toDate('2026-01-01') AND toDate('2026-06-01');
```

```sql
EXPLAIN indexes = 1
SELECT security_code, price_ma_20, macd_histogram
FROM fleur_marts.mart_stock_trend_indicator
WHERE trade_date = toDate('2026-06-01')
ORDER BY macd_histogram DESC
LIMIT 100;
```

```sql
EXPLAIN indexes = 1
SELECT
    security_code,
    trade_date,
    close_price_forward_adj,
    forward_adjustment_factor
FROM fleur_intermediate.int_stock_quotes_daily_adj
WHERE trade_date BETWEEN toDate('2025-01-01') AND toDate('2026-06-01')
  AND security_code IN ('000001.SZ', '600000.SH');
```

验收标准：

- [ ] `EXPLAIN indexes = 1` 显示 `PrimaryKey` 条件命中 `trade_date`。
- [ ] 日截面查询的 `read_rows` / `read_bytes` 明显下降。
- [ ] 全市场日期区间模型构建耗时不回退。
- [ ] `mart_stock_quotes_daily` 作为显式例外不要求命中 `trade_date` 前缀；如该表日截面查询成为瓶颈，另开 feature mart / projection 方案。
- [ ] 单证券长历史查询可接受；如显著回退，记录是否需要补充 date-first 主表 + security-first 派生表或 projection。

### 回归场景

- [ ] Furnace full-market dry-run 读取 `int_stock_quotes_daily_adj` / `int_stock_quotes_daily_unadj` 正常。
- [ ] `mart_stock_quotes_daily` 的 ASOF join 结果无 join 放大。
- [ ] `mart_stock_trend_indicator`、`mart_stock_momentum_indicator`、`mart_stock_volume_indicator` 行数仍等于各自输入主表预期。
- [ ] 选股日截面原型 SQL 不跳过 `ORDER BY` 前缀。

## 风险和例外

- 单证券长历史查询可能不如 `(security_code, trade_date)`；如该路径成为高频交互查询，应另建派生表或 projection，而不是牺牲日截面主路径。
- `mart_stock_quotes_daily` 本轮保留 `(security_code, trade_date)`；如选股日截面直接依赖该表出现扫描瓶颈，优先新增 date-first 选股特征 mart，而不是顺手重排基础行情宽表。
- 财报、股本、除权事件等非交易日日频表不能机械套用 `trade_date` 规则。
- raw 表排序键变更涉及物理表重建，必须有迁移窗口、行数校验和回滚路径。
- 如果后续策略引擎需要同时高频支持单证券回放和全市场截面，应把两类访问路径拆成不同物化表，而不是期望单一排序键同时最优。
