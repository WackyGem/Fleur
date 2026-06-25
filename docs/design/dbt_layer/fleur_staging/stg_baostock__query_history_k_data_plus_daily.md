# stg_baostock__query_history_k_data_plus_daily 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/baostock__query_history_k_data_plus_daily_compacted.md`
- Raw source：`source('raw', 'baostock__query_history_k_data_plus_daily_compacted')`
- 目标位置：`pipeline/elt/models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.sql`

## 1. 模型定位

BaoStock 日频 K 线的 source-local staging model。模型只做字段命名、证券代码格式标准化、日期保留、未复权口径过滤和基础类型暴露，不做行情异常修正、跨源复权口径合并、停牌业务裁决或跨源主数据匹配。

本模型只输出 `adjustflag = 3` 的未复权行情。`adjustflag` 作为输入过滤字段，不作为 staging 输出字段。BaoStock 原始 `tradestatus` 在 staging 中派生为下游更易使用的 `is_suspend`，原始状态码不输出。

## 2. 数据特征

- 行数：19,648,244。
- 覆盖日期：`date` 为 1990-12-19 至 2026-06-25，NULL 0 行。
- 粒度：一行代表一个 BaoStock `code` 在一个 `date` 的日频行情记录。
- 候选键：`code`, `date`，profile 未发现重复。
- 证券代码：`code` 全部为 BaoStock `sh.600000` / `sz.000001` 类供应商前缀格式。
- `adjustflag` 全部为 `3`；`tradestatus` 为 `1` 或 `0`；`isST` 为布尔值。
- 价格、成交量、成交金额、换手率存在业务合理的 0 值；profile 未发现负值。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `security_code` | `code` | `LowCardinality(String)` | 使用 `normalize_cn_security_code(input_format='baostock_prefix')` 转为 `000001.SZ` 格式。 |
| `trade_date` | `date` | `Date` | 保留交易日期语义，作为行情事实日期。 |
| `open_price` | `open` | `Float64` | source-local 字段；是否进入第一版取决于 glossary 扩展。 |
| `high_price` | `high` | `Float64` | source-local 字段；不在 staging 判断异常高价。 |
| `low_price` | `low` | `Float64` | source-local 字段；不在 staging 判断异常低价。 |
| `close_price` | `close` | `Float64` | source-local 字段；不在 staging 做复权口径转换。 |
| `prev_close_price` | `preclose` | `Float64` | source-local 字段；保留 BaoStock 原始口径。 |
| `volume` | `volume` | `Int64` | source-local 字段；0 值保留。 |
| `amount` | `amount` | `Float64` | source-local 字段；单位按 raw 保留，落地时在 YAML meta 说明。 |
| `is_suspend` | `tradestatus = 0` | `Bool` | 是否停牌；`tradestatus = 0` 映射为 `true`，`tradestatus = 1` 映射为 `false`。 |
| `is_st` | `isST` | `Bool` | 保留 source-local 布尔值。 |

## 4. 标准化与 NULL 处理

- `code` 必须通过现有证券代码 macro 标准化，不能直接暴露为 canonical join key。
- `date` 已是 `Date` 类型且无 NULL；staging 不需要额外 cast。
- 所有数值 0 值按 profile 观察保留，不转 NULL，不过滤。
- 只保留 `adjustflag = 3` 的未复权行情；`adjustflag` 不输出。
- `tradestatus` 只用于派生 `is_suspend`；原始状态码不输出。
- `turn` 和 `pctChg` 第一版不输出；如后续下游需要，应单独评估 source-local 口径、单位和重算策略。

## 5. 测试建议

- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- 组合键：`security_code`, `trade_date` 唯一。
- `is_suspend`: `not_null`。
- 不对价格、成交量、成交金额加全字段 `not_null` 以外的业务阈值测试；阈值异常检查延后。

## 6. 延后事项

- 输出 `adjustflag`、`tradestatus`、`turn` 或 `pctChg` 对应的 source-local 字段。
- 复权价格选择、复权因子处理和多行情口径切换。
- 停牌、ST、涨跌幅异常、成交量极值等业务解释。
- 与交易日历、证券主数据或其他行情源的对账。
