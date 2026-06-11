# 2026-06-11 ClickHouse P1 ORDER BY Evaluation

## 基本信息

- 评估日期：2026-06-11
- ClickHouse 版本：26.5.1.882
- 实例：`127.0.0.1:34052`，用户 `mono_fleur`，host `637384b0a4a5`
- 工作负载：A 股日频 market data / financial services
- 测试窗口：
  - 日期截面：`2024-01-01` 至 `2024-12-31`
  - 单证券长历史：`2010-01-01` 至 `2026-06-01`
  - ASOF 消费路径：行情 `2025-01-01` 至 `2025-03-31`
- 执行人：Codex agent
- 查询限制：所有 benchmark 设置 `max_execution_time <= 180`、`max_rows_to_read = 1000000000`、`max_bytes_to_read = 100000000000`、`timeout_before_checking_execution_speed = 0`，结果使用 `FORMAT Null`。
- query_id 前缀：
  - `orderby_p1_metric_%_20260611`：强制读取 benchmark。
  - `orderby_p1_run_%_20260611`：ASOF / consumer benchmark。
  - `orderby_p1_explain_%_20260611`：`EXPLAIN indexes = 1` 对照。

## 命令

主要命令均从仓库根目录或 `pipeline/` 目录执行：

```bash
cd pipeline
uv run dbt build \
  --project-dir elt \
  --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_trend_indicator mart_stock_momentum_indicator mart_stock_volume_indicator \
  --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'

uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run dbt parse --project-dir elt --profiles-dir elt
uv run pytest scheduler/tests/unit/clickhouse contract_tools/tests -q
```

ClickHouse 评估通过 HTTP interface 执行，使用 `.env` 中的 `CLICKHOUSE_HOST`、`CLICKHOUSE_PORT`、`CLICKHOUSE_USER` 和 `CLICKHOUSE_PASSWORD`。代表性 SQL 类型：

```sql
EXPLAIN indexes = 1
SELECT security_code, effective_date
FROM fleur_intermediate.int_stock_shares_history
WHERE effective_date BETWEEN toDate('2024-01-01') AND toDate('2024-12-31');

CREATE TABLE fleur_scratch.orderby_eval__int_stock_shares_history__date_first
ENGINE = MergeTree()
ORDER BY (effective_date, security_code)
AS SELECT * FROM fleur_intermediate.int_stock_shares_history;

SELECT sum(length(security_code))
FROM fleur_scratch.orderby_eval__int_stock_shares_history__date_first
WHERE effective_date BETWEEN toDate('2024-01-01') AND toDate('2024-12-31')
FORMAT Null;

DROP TABLE IF EXISTS fleur_scratch.orderby_eval__int_stock_shares_history__date_first;
```

## 当前物理事实

| 对象 | 当前排序键 | 分区键 | 行数 | 大小 |
|---|---|---|---:|---:|
| `fleur_intermediate.int_stock_financial_valuation` | `security_code, report_date` | `toYear(report_date)` | 298,556 | 19.00 MiB |
| `fleur_intermediate.int_stock_shares_history` | `security_code, effective_date` | 无 | 367,733 | 6.42 MiB |
| `fleur_intermediate.int_stock_exrights_event` | `security_code, ex_dividend_date` | `toYear(ex_dividend_date)` | 55,881 | 1.50 MiB |
| `fleur_raw.eastmoney__balance` | `SECUCODE, REPORT_DATE` | `year` | 284,265 | 191.59 MiB |
| `fleur_raw.eastmoney__income_ytd` | `SECUCODE, REPORT_DATE` | `year` | 298,396 | 131.91 MiB |
| `fleur_raw.eastmoney__income_sq` | `SECUCODE, REPORT_DATE` | `year` | 279,918 | 183.38 MiB |
| `fleur_raw.eastmoney__cashflow_ytd` | `SECUCODE, REPORT_DATE` | `year` | 283,613 | 175.31 MiB |
| `fleur_raw.eastmoney__cashflow_sq` | `SECUCODE, REPORT_DATE` | `year` | 274,016 | 168.08 MiB |
| `fleur_raw.eastmoney__dividend_main` | `SECUCODE, REPORT_DATE` | `year` | 151,607 | 6.77 MiB |
| `fleur_raw.eastmoney__dividend_allotment` | `SECUCODE, NOTICE_DATE` | `year` | 1,156 | 93.62 KiB |
| `fleur_raw.eastmoney__equity_history` | `SECUCODE, END_DATE` | `year` | 146,365 | 24.99 MiB |
| `fleur_raw.eastmoney__freeholders` | `SECUCODE, END_DATE, HOLDER_RANK` | `year` | 2,736,392 | 90.56 MiB |
| `fleur_raw.baostock__query_stock_basic` | `code` | `tuple()` | 8,769 | 149.85 KiB |
| `fleur_raw.jiuyan__industry_list` | `industry_id` | `tuple()` | 957 | 735.36 KiB |
| `fleur_raw.jiuyan__industry_ocr_snapshot` | `industry_id, image_filename, ocr_row_index` | `tuple()` | 1,322 | 77.37 KiB |

## 查询画像

| 对象 | 真实消费者 | 访问形态 | 排序键倾向 |
|---|---|---|---|
| `int_stock_financial_valuation` | `mart_stock_quotes_daily.sql`、`mart_stock_quotes_daily_financial_valuation_asof_matches.sql` | `security_code` + `trade_date >= report_date` ASOF | security-first |
| `int_stock_shares_history` | `int_stock_quotes_daily_unadj.sql`、`int_stock_financial_valuation.sql` | `security_code` + `trade_date/report_date >= effective_date` ASOF | security-first |
| `int_stock_exrights_event` | `int_stock_quotes_daily_unadj.sql` | 按证券累计现金分红、ASOF 到行情，也有按 `ex_dividend_date` 聚合 | security-first 或双路径 |
| EastMoney raw 财报 / 股本 / 分红表 | staging models 和 source/raw sync | raw 到 staging 的字段标准化，低频 profile / 回填检查 | keep raw，date-first 放下游 |
| 维表 / OCR 表 | 维度 lookup 或 OCR 明细 | 按维度键访问 | keep |

## 候选影子表

在 `fleur_scratch` 创建 date-first 候选，不修改生产表：

| 原对象 | 候选排序键 |
|---|---|
| `int_stock_financial_valuation` | `report_date, security_code` |
| `int_stock_shares_history` | `effective_date, security_code` |
| `int_stock_exrights_event` | `ex_dividend_date, security_code` |
| EastMoney 财报 raw | `REPORT_DATE, SECUCODE` |
| `eastmoney__dividend_allotment` | `NOTICE_DATE, SECUCODE` |
| `eastmoney__equity_history` | `END_DATE, SECUCODE` |
| `eastmoney__freeholders` | `END_DATE, SECUCODE, HOLDER_RANK` |

一致性检查：所有影子表与原表行数、日期范围一致。代表性结果：

| 对象 | 原表行数 | 候选行数 | 原表唯一键 | 候选唯一键 | 日期范围 |
|---|---:|---:|---:|---:|---|
| `int_stock_financial_valuation` | 298,556 | 298,556 | 298,556 | 298,556 | 1988-12-31 至 2026-03-31 |
| `int_stock_shares_history` | 367,733 | 367,733 | 367,733 | 367,733 | 1990-12-19 至 2026-06-10 |
| `int_stock_exrights_event` | 55,881 | 55,881 | 55,881 | 55,881 | 1991-06-01 至 2026-06-12 |
| `eastmoney__balance` | 284,265 | 284,265 | 284,265 | 284,265 | 1989-12-31 至 2026-03-31 |
| `eastmoney__freeholders` | 2,736,392 | 2,736,392 | 272,449 | 272,449 | 2003-12-31 至 2026-06-03 |

`eastmoney__dividend_main.REPORT_DATE` 是 `LowCardinality(String)` 的报告期标签，日期基准只作为 raw 物理排序压力测试，不作为业务可见日期结论。

## EXPLAIN 摘要

| 查询 | 当前表摘要 | 候选表摘要 |
|---|---|---|
| `int_stock_financial_valuation` 2024 报告期 | `PrimaryKey Keys: report_date`，Granules `3/3`，generic exclusion | `PrimaryKey Keys: report_date`，Granules `3/3`，binary search |
| `int_stock_financial_valuation` 单证券长历史 | `PrimaryKey Keys: security_code, report_date`，Granules `17/29` | `PrimaryKey Keys: report_date, security_code`，Granules `27/29` |
| `int_stock_shares_history` 2024 effective_date | 当前 Granules `45/45` | 候选 Granules `5/45` |
| `int_stock_exrights_event` 2024 ex_dividend_date | 当前和候选均 Granules `1/1` | 候选只有 search algorithm 更直接 |
| `eastmoney__balance` 2024 REPORT_DATE | 当前 Granules `71/82` | 候选 Granules `7/80` |

## Benchmark 指标

强制读取 benchmark 使用 `sum(length(key))` 避免 `count()` 被元数据优化。表内单位为 `read_rows / duration_ms`。

### Intermediate

| 对象 | 日期路径 原表 | 日期路径 候选 | 单证券 原表 | 单证券 候选 | ASOF 原表 | ASOF 候选 |
|---|---:|---:|---:|---:|---:|---:|
| `int_stock_financial_valuation` | 20,514 / 8 | 20,514 / 8 | 147,728 / 16 | 229,285 / 15 | 618,044 / 111 | 618,044 / 133 |
| `int_stock_shares_history` | 367,733 / 7 | 40,960 / 4 | 8,192 / 4 | 326,773 / 9 | 687,221 / 112 | 687,221 / 119 |
| `int_stock_exrights_event` | 4,539 / 7 | 4,539 / 7 | 40,444 / 13 | 40,444 / 13 | 375,369 / 73 | 375,369 / 60 |

### EastMoney raw

| 对象 | 日期路径 原表 | 日期路径 候选 | 单证券 原表 | 单证券 候选 |
|---|---:|---:|---:|---:|
| `eastmoney__balance` | 274,065 / 20 | 24,072 / 19 | 81,568 / 22 | 184,234 / 18 |
| `eastmoney__income_ytd` | 287,979 / 20 | 27,416 / 15 | 126,354 / 21 | 230,568 / 23 |
| `eastmoney__income_sq` | 269,689 / 26 | 23,198 / 19 | 82,867 / 26 | 223,409 / 25 |
| `eastmoney__cashflow_ytd` | 273,373 / 24 | 25,231 / 18 | 94,850 / 24 | 217,915 / 26 |
| `eastmoney__cashflow_sq` | 263,761 / 28 | 23,531 / 23 | 60,805 / 29 | 177,183 / 32 |
| `eastmoney__dividend_main` | 124,750 / 15 | not comparable | 117,897 / 19 | 117,897 / 18 |
| `eastmoney__dividend_allotment` | 1,156 / 13 | 0 / 5 | 10 / 6 | 10 / 6 |
| `eastmoney__equity_history` | 41,271 / 15 | 33,079 / 11 | 100,946 / 21 | 100,946 / 17 |
| `eastmoney__freeholders` | 2,467,762 / 17 | 222,349 / 11 | 139,264 / 9 | 578,780 / 14 |

## 决策

| 对象 | 决策 | 依据 | 后续动作 |
|---|---|---|---|
| `int_stock_financial_valuation` | `keep` | 真实消费是 `mart_stock_quotes_daily` 的 ASOF enrichment；date-first 没有减少日期路径 read_rows，单证券和 ASOF 路径不更优。 | 不改本表。若需要交易日可见估值特征，新增日频 feature mart。 |
| `int_stock_shares_history` | `keep` | date-first 对日期扫描显著更好，但单证券路径从 8,192 退到 326,773 rows；现有主路径是按证券 ASOF 到行情和估值。 | 不改本表。若市值/股本特征成为独立高频日截面入口，新增日频展开表或 mart。 |
| `int_stock_exrights_event` | `keep` | 表仅 1.50 MiB，date-first 对日期路径无实质 read_rows 改善；现有消费按证券累计和 ASOF。 | 不改本表。若后续需要日历事件查询，可新增 `mart_stock_exrights_event_daily`。 |
| EastMoney 财报 raw | `keep` | date-first 对 raw 日期扫描有收益，但 raw 不是服务层入口；单证券路径普遍回退，且下游已在 staging/intermediate 形成 canonical 模型。 | 不改 raw contract。高频消费放 intermediate/mart。 |
| `eastmoney__equity_history` / `eastmoney__freeholders` | `keep` | `freeholders` 日期路径收益明显，但单证券路径回退；两者当前用于构建 `int_stock_shares_history`，不直接服务日截面。 | 不改 raw。股本特征需求在 downstream 建模。 |
| `baostock__query_stock_basic`、`jiuyan__industry_list`、`jiuyan__industry_ocr_snapshot` | `keep` | 维表或 OCR 明细，不是交易日日频事实表；当前排序键匹配维度访问。 | 无需迁移。 |

## 结论

P1 本次不产生第二批排序键迁移计划。date-first 对若干 raw 日期扫描有可见收益，但收益不落在当前主消费者路径；对 `int_stock_shares_history` 和多个 raw 单证券路径存在明确回退。若未来新增选股特征或日历事件服务，应优先新增 date-first 派生表、projection 或 mart，而不是重排当前主表。
