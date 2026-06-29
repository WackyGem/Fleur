### 状态

已完成首期优化：删除低价值测试、收敛日常作业 selection、新增 raw sync 成功后的下游触发路径，并完成 `mart_stock_quotes_daily` KDJ join 低风险 SQL 优化。见 `docs/jobs/reports/2026-06-29-dbt-baostock-downstream-performance-optimization.md`。


# BaoStock 下游 dbt 全量扫描性能异常记录（2026-06-26）

## 扫描基线

- 报告来源：`docs/jobs/reports/2026-06-26-dbt-baostock-downstream-performance.md`
- 优化方案：`docs/RFC/0036-dbt-baostock-downstream-performance-optimization.md`
- 首期优化报告：`docs/jobs/reports/2026-06-29-dbt-baostock-downstream-performance-optimization.md`
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

> 注：后续 RFC 0036 已将首期优先级收敛为“删除低价值字段匹配测试 + raw 最新 year 刷新后触发相关 int/mart 全量重建 + 保留 key/基础测试 + int/mart SQL 基准优化”，暂不把 int 层分区增量或多套长期 dbt 调度 job 作为首期动作。

1. 删除以下低价值字段逐列匹配测试：
   - `mart_stock_quotes_daily_quote_passthrough_matches`
   - `mart_stock_quotes_daily_adjusted_passthrough_matches`
   - `mart_stock_quotes_daily_financial_valuation_asof_matches`
2. raw ClickHouse 最新 year 分区刷新成功后，触发相关 int/mart 全量重建：
   - `int_stock_quotes_daily_unadj`
   - `int_stock_adjustment_factor`
   - `int_stock_quotes_daily_adj`
   - `mart_stock_quotes_daily`
3. 保留 key 类和基础质量测试：
   - `not_null`
   - `unique`
   - `cn_security_code_format`
   - `mart_stock_quotes_daily_adjusted_key_coverage`
   - `mart_stock_quotes_daily_key_set_matches_quotes`
4. 将 int/mart SQL 基准优化作为后续主要性能方向：
   - 先对 `mart_stock_quotes_daily` 做 CTE 分段基准和 query log 基准。
   - 若仍超标，再依次评估 `int_stock_quotes_daily_adj`、`int_stock_quotes_daily_unadj`、`int_stock_adjustment_factor`。
   - 所有 SQL 改动必须有前后基准，低于 `10%` 收益不实施。
5. 若删除字段匹配测试和 SQL 优化后的日常路径仍超标，再评估剩余测试窗口化或 int 增量；不在首期引入多套长期 dbt 调度 job。

## 首期执行结果（2026-06-29）

- 已删除 3 个低价值 mart 字段逐列匹配测试，dbt manifest 不再包含这些 test nodes。
- 相关 int/mart selector 已通过：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_quotes_daily
```

- 本次结果：`PASS=50 WARN=0 ERROR=0 SKIP=0 NO-OP=0 TOTAL=50`
- 总耗时：`203.03s`
- 对比原 baseline `374.86s`，本次相关路径降低约 `171.83s`。
- `stock__daily_build_job` 已从全 dbt model group 收敛为 quote int 链路、calculation assets、`int_stock_kdj_daily` 和 `mart_stock_quotes_daily`。
- 新增 `baostock_raw_sync_success_triggers_stock_daily_build` sensor：监听 `clickhouse__raw_sync_baostock_job` 成功并触发 `stock__daily_build_job`；不使用 `_sync_at`、raw sync 状态表或 dbt vars 水位线。
- `mart_stock_quotes_daily` 的 KDJ join 已从 `LEFT JOIN` 改为 `LEFT ANY JOIN`；`int_stock_kdj_daily` 的 `(security_code, trade_date)` 唯一性测试通过，`FORMAT Null` full select 从 `75424ms` 降到 `42827ms`。

剩余瓶颈：`mart_stock_quotes_daily` 本次仍耗时 `86.61s`。若日常 SLA 仍不满足，后续重点应评估 ClickHouse table materialization/write path；int incremental 仍需另起 RFC。

## 关联风险

- 字段错接不再由全表逐列匹配测试自动发现，需要依赖 mart SQL review、字段来源文档、key coverage 和发版前抽样验证。
- 若要切到分区级重建，应先验证 `partition_by='toYear(trade_date)'` 与增量策略（报告建议 `insert_overwrite`）在生产语义下的正确性。
