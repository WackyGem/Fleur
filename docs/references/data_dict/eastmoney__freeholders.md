# eastmoney__freeholders 数据字典

本文件由 `pipeline/contracts/datasets/eastmoney__freeholders.yml` 生成。字段事实以 contract 为准。

- 数据集：`eastmoney__freeholders`
- 版本：`1`
- 说明：东方财富前十大流通股东 F10 年度 raw 分区
- 粒度：one row per security code per report end date per free-float shareholder rank
- Source asset：`source/eastmoney__freeholders`
- Raw asset：`clickhouse/raw/eastmoney__freeholders`
- ClickHouse raw：`fleur_raw.eastmoney__freeholders`
- 分区策略：`year`
- ORDER BY：`(SECUCODE, END_DATE, HOLDER_RANK)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `SECUCODE` | `string` | `string` | `SECUCODE` | `LowCardinality(String)` | 证券代码（含市场后缀）。 |
| 2 | `SECURITY_CODE` | `string` | `string` | `SECURITY_CODE` | `LowCardinality(String)` | 证券代码（纯数字）。 |
| 3 | `END_DATE` | `string` | `date32[day]` | `END_DATE` | `Date` | 前十大流通股东名单对应的报告期截止日期。 |
| 4 | `HOLDER_RANK` | `number` | `int64` | `HOLDER_RANK` | `Int64` | 股东在该报告期前十大流通股东名单中的排名。 |
| 5 | `HOLDER_NEW` | `string` | `string` | `HOLDER_NEW` | `String` | 东方财富返回的股东标识编码。 |
| 6 | `HOLDER_NAME` | `string` | `string` | `HOLDER_NAME` | `String` | 股东名称。 |
| 7 | `HOLDER_TYPE` | `string` | `string` | `HOLDER_TYPE` | `LowCardinality(String)` | 股东类型或机构类型。 |
| 8 | `SHARES_TYPE` | `string` | `string` | `SHARES_TYPE` | `LowCardinality(String)` | 股东持有股份类别。 |
| 9 | `HOLD_NUM` | `number` | `int64` | `HOLD_NUM` | `Int64` | 股东持有流通股数量，单位为股。 |
| 10 | `FREE_HOLDNUM_RATIO` | `number` | `double` | `FREE_HOLDNUM_RATIO` | `Float64` | 股东持有流通股数量占流通股比例，单位为百分比。 |
| 11 | `HOLD_NUM_CHANGE` | `string` | `string` | `HOLD_NUM_CHANGE` | `String` | 较上期持股数量变动；接口以字符串返回，可为数值文本或“不变”。 |
| 12 | `CHANGE_RATIO` | `number` | `double` | `CHANGE_RATIO` | `Nullable(Float64)` | 较上期持股数量变动比例，单位为百分比；无变动时可能为空。 |

## 数据集备注

东方财富前十大流通股东 F10 年度 raw 分区

## 校验记录

- Initial contract added from EastMoney RPT_F10_EH_FREEHOLDERS endpoint sample.
