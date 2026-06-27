# Stock Technical Indicator Marts Build

日期：2026-06-10

范围：

- `mart_stock_trend_indicator`
- `mart_stock_momentum_indicator`
- `mart_stock_volume_indicator`

目标 schema：`fleur_marts`

## 环境

dbt 命令均在 `pipeline/` 目录通过 `uv run` 执行。

ClickHouse：

```text
container = fleur-clickhouse
user = mono_fleur
```

## dbt 构建

命令：

```bash
cd pipeline
uv run dbt build \
  --project-dir elt \
  --profiles-dir elt \
  --select mart_stock_trend_indicator mart_stock_momentum_indicator mart_stock_volume_indicator
```

结果：

```text
Finished running 3 project hooks, 3 table models, 17 data tests in 0 hours 2 minutes and 2.12 seconds.
Completed successfully
PASS=23 WARN=0 ERROR=0 SKIP=0 NO-OP=0 TOTAL=23
```

模型构建耗时：

```text
mart_stock_momentum_indicator = 38.16s
mart_stock_trend_indicator = 64.39s
mart_stock_volume_indicator = 9.79s
```

## 表结构

三张 mart 均为 ClickHouse `MergeTree` 表：

```text
database = fleur_marts
engine = MergeTree
sorting_key = security_code, trade_date
partition_key = toYear(trade_date)
```

## 行数和范围

```text
mart      rows      securities  min_date    max_date
trend     17990764  5532        1995-01-03  2026-06-01
momentum  17990764  5532        1995-01-03  2026-06-01
volume    17990764  5532        1995-01-03  2026-06-01
```

## 主键质量

重复 `(security_code, trade_date)` 检查：

```text
trend duplicate groups = 0
momentum duplicate groups = 0
volume duplicate groups = 0
```

dbt schema tests 覆盖：

```text
unique_combination_of_columns: PASS for all three marts
not_null(security_code): PASS for all three marts
not_null(trade_date): PASS for all three marts
cn_security_code_format(security_code): PASS for all three marts
```

## 指标质量

趋势 mart NULL 分布：

```text
null_price_ma_250 = 1366126
null_boll_mid_20_2 = 105048
null_macd_dif = 138192
null_macd_dea = 182354
null_macd_histogram = 182354
```

动量 mart NULL 和参数分布：

```text
null_rsi_6 = 33190
null_rsi_50 = 276076
null_kdj_rsv = 44250
null_kdj_k_value = 44250
noncanonical_kdj_rsv_window = 0
noncanonical_kdj_k_smoothing = 0
noncanonical_kdj_d_smoothing = 0
```

成交量形 mart NULL 分布：

```text
null_volume_ma_5 = 22128
null_volume_ma_10 = 49779
null_volume_ma_20 = 105048
null_volume_ma_60 = 325665
```

补充 singular tests：

```text
mart_stock_trend_indicator_boll_bands_order: PASS
mart_stock_momentum_indicator_rsi_bounds: PASS
```

## dbt Show

已执行：

```bash
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator --limit 20
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_momentum_indicator --limit 20
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_volume_indicator --limit 20
```

观察：

- trend 和 momentum 样本从 `1995-01-03` 起始交易日开始，warm-up 期指标字段为空，符合上游 wrapper NULL 语义。
- volume 样本返回 `volume_ma_5`、`volume_ma_10`、`volume_ma_20`、`volume_ma_60` 四个均量字段。

## Lineage 证据

`mart_stock_trend_indicator` compiled SQL 片段：

```sql
from `fleur_intermediate`.`int_stock_ma_daily`
from `fleur_intermediate`.`int_stock_boll_daily`
from `fleur_intermediate`.`int_stock_macd_daily`
```

MACD 字段只选择：

```sql
macd_dif,
macd_dea,
macd_histogram
```

`mart_stock_momentum_indicator` compiled SQL 片段：

```sql
from `fleur_intermediate`.`int_stock_rsi_daily`
from `fleur_intermediate`.`int_stock_kdj_daily`
```

KDJ 字段在 mart 层统一重命名为：

```text
kdj_rsv_window
kdj_k_smoothing
kdj_d_smoothing
kdj_rsv
kdj_k_value
kdj_d_value
kdj_j_value
```

`mart_stock_volume_indicator` compiled SQL 只引用：

```sql
from `fleur_intermediate`.`int_stock_ma_daily`
```

并只选择：

```text
volume_ma_5
volume_ma_10
volume_ma_20
volume_ma_60
```

## 结论

- 三个技术指标 mart 已按全量数据物化到 `fleur_marts`。
- 三个 mart 均保持每证券、交易日一行，未发生 join 放大。
- 趋势 mart 消费 MA/BOLL/MACD wrapper，不包含均量或 MACD 内部状态列。
- 动量 mart 消费 RSI/KDJ wrapper，KDJ 字段统一使用 `kdj_` 前缀。
- 成交量形 mart 只包含四个均量字段。
