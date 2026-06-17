# chinabond__government_bond 数据字典

本文件由 `pipeline/contracts/datasets/chinabond__government_bond.yml` 生成。字段事实以 contract 为准。

- 数据集：`chinabond__government_bond`
- 版本：`1`
- 说明：中债国债收益率曲线年度 raw 分区
- 粒度：one row per ChinaBond government bond yield curve work date
- Source asset：`source/chinabond__government_bond`
- Raw asset：`clickhouse/raw/chinabond__government_bond`
- ClickHouse raw：`fleur_raw.chinabond__government_bond`
- 分区策略：`year`
- ORDER BY：`(work_date)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `work_date` | `string` | `date32[day]` | `work_date` | `Date` | 中债曲线日期，来自原始 JSON 字段 workTime，格式为 YYYY-MM-DD。 |
| 2 | `curve_name` | `string` | `string` | `curve_name` | `LowCardinality(String)` | 曲线名称，来自原始 JSON 字段 qxmc。 |
| 3 | `three_month_yield_pct` | `string` | `double` | `three_month_yield_pct` | `Nullable(Float64)` | 3 个月收益率，来自原始 JSON 字段 threeMonth，单位为百分比点。 |
| 4 | `six_month_yield_pct` | `string` | `double` | `six_month_yield_pct` | `Nullable(Float64)` | 6 个月收益率，来自原始 JSON 字段 sixMonth，单位为百分比点。 |
| 5 | `one_year_yield_pct` | `string` | `double` | `one_year_yield_pct` | `Nullable(Float64)` | 1 年收益率，来自原始 JSON 字段 oneYear，单位为百分比点。 |
| 6 | `two_year_yield_pct` | `string` | `double` | `two_year_yield_pct` | `Nullable(Float64)` | 2 年收益率，来自原始 JSON 字段 twoYear，单位为百分比点。 |
| 7 | `three_year_yield_pct` | `string` | `double` | `three_year_yield_pct` | `Nullable(Float64)` | 3 年收益率，来自原始 JSON 字段 threeYear，单位为百分比点。 |
| 8 | `five_year_yield_pct` | `string` | `double` | `five_year_yield_pct` | `Nullable(Float64)` | 5 年收益率，来自原始 JSON 字段 fiveYear，单位为百分比点。 |
| 9 | `seven_year_yield_pct` | `string` | `double` | `seven_year_yield_pct` | `Nullable(Float64)` | 7 年收益率，来自原始 JSON 字段 sevenYear，单位为百分比点。 |
| 10 | `ten_year_yield_pct` | `string` | `double` | `ten_year_yield_pct` | `Nullable(Float64)` | 10 年收益率，来自原始 JSON 字段 tenYear，单位为百分比点。 |
| 11 | `fifteen_year_yield_pct` | `string` | `double` | `fifteen_year_yield_pct` | `Nullable(Float64)` | 15 年收益率，来自原始 JSON 字段 fifteenYear，单位为百分比点。 |
| 12 | `twenty_year_yield_pct` | `string` | `double` | `twenty_year_yield_pct` | `Nullable(Float64)` | 20 年收益率，来自原始 JSON 字段 twentyYear，单位为百分比点。 |
| 13 | `thirty_year_yield_pct` | `string` | `double` | `thirty_year_yield_pct` | `Nullable(Float64)` | 30 年收益率，来自原始 JSON 字段 thirtyYear，单位为百分比点。 |

## 数据集备注

中债国债收益率曲线年度分区 raw 数据，收益率字段单位为百分比点。

## 校验记录

- Initial contract derived from docs/references/remote_endpoint/chinabond__government_bond_yield_curve.md.
