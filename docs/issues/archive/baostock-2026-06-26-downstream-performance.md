# BaoStock 下游 dbt 全量扫描性能异常记录（2026-06-26）

## 扫描基线

- 报告来源：`docs/jobs/reports/2026-06-26-dbt-baostock-downstream-performance.md`
- 执行时间：`2026-06-26T00:05:00Z`
- 命令：`uv run dbt build --project-dir elt --profiles-dir elt --select stg_baostock__query_history_k_data_plus_daily+`
- dbt 版本：`1.11.11`，ClickHouse 适配器：`1.10.0`
- 本次运行结果：`PASS=72 WARN=0 ERROR=0 SKIP=0 NO-OP=0 TOTAL=72`
- 总耗时：`374.86s`

## 关键耗时现象（明显超标）

1. 表模型总耗时 205.89s，占比 54.9%；其中：
   - `mart_stock_quotes_daily`：`119.31s`
   - `int_stock_quotes_daily_adj`：`35.83s`
   - `int_stock_quotes_daily_unadj`：`28.76s`
   - `int_stock_adjustment_factor`：`12.68s`
2. 数据测试总耗时 168.06s，占比 44.8%，其中 5 个 Mart 回归类测试最重：
   - `mart_stock_quotes_daily_quote_passthrough_matches`：`63.64s`
   - `mart_stock_quotes_daily_adjusted_passthrough_matches`：`37.10s`
   - `mart_stock_quotes_daily_financial_valuation_asof_matches`：`30.53s`
   - `mart_stock_quotes_daily_adjusted_key_coverage`：`9.27s`
   - `mart_stock_quotes_daily_key_set_matches_quotes`：`8.80s`
3. 其他超过 1s 的慢测：
   - `int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date`：`4.21s`
   - `unique_combination_of_columns_mart_stock_quotes_daily_security_code__trade_date`：`1.94s`
   - `unique_combination_of_columns_int_stock_adjustment_factor_security_code__trade_date`：`1.87s`
   - `unique_combination_of_columns_int_stock_quotes_daily_adj_security_code__trade_date`：`1.71s`
   - `unique_combination_of_columns_int_stock_quotes_daily_unadj_security_code__trade_date`：`1.37s`

## 根因定位

- 该扫描是 `stg_baostock__query_history_k_data_plus_daily+` 的下游全量路径，覆盖 18M 行行情日线数据，且模型与测试多为全量重建/全量比较。
- 报告明确指出运行被以下两类动作主导：
  - 全市场、18M 行重建（`mart_stock_quotes_daily` 为单点最大开销）。
  - Mart 层大宽表的全表一致性对比测试（`passthrough/asof` 系列测试）。
- 该路径在日常增量场景下并不理想，因为实际变化通常仅是最新交易日或当前年度。

## 对应优化动作建议（可直接转化为后续改造项）

1. 为以下表先引入分区级/增量构建（按 `trade_date` 或年份）：
   - `int_stock_quotes_daily_unadj`
   - `int_stock_adjustment_factor`
   - `int_stock_quotes_daily_adj`
   - `mart_stock_quotes_daily`
2. 将重型 Mart 回归测试按运行频率拆分：
   - 日常运行：只跑日新增窗口/当年窗口相关的 `not_null`、`unique`、key 覆盖/通道筛选测试。
   - 全量/夜间/发版运行：保留完整 `passthrough`、`ASOF` 全表回归。
3. 为高成本测试引入 `validation_start_date` / `validation_year` 等可变变量：
   - 日常默认只覆盖最近 N 天；
   - 修复或回放任务可切到按年窗口；
   - 仅在缺省变量时才跑全历史。
4. 降低日常执行命令范围：避免常规日更继续使用 `stg_baostock__query_history_k_data_plus_daily+` 全链路重跑，待增量与窗口测试机制就绪后改为更窄选择器。

## 关联风险

- 全量 Mart 回归测试对正确性收益高，不能直接删除，需要保留“完整验证”路径。
- 若要切到分区级重建，应先验证 `partition_by='toYear(trade_date)'` 与增量策略（报告建议 `insert_overwrite`）在生产语义下的正确性。
