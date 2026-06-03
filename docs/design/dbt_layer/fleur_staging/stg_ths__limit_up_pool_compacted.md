# stg_ths__limit_up_pool_compacted 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/ths__limit_up_pool_compacted.md`
- Raw source：`source('raw', 'ths__limit_up_pool_compacted')`
- 目标位置：`pipeline/elt/models/staging/ths/stg_ths__limit_up_pool_compacted.sql`

## 1. 模型定位

同花顺涨停池日频明细的 source-local staging model。模型保留一股票一交易日的涨停池记录，完成字段命名、日期时间保留和本地证券代码暴露，不做交易所推断、涨停原因归并或连板语义重算。

## 2. 数据特征

- 行数：15,664。
- 粒度：一行代表一个 `date`, `code` 涨停池记录。
- 候选键：`date`, `code`，profile 未发现重复。
- `code` 全部为 6 位本地代码，不包含交易所。
- 日期范围：`date` 为 2025-05-19 至 2026-06-01，NULL 0 行。
- `first_limit_up_time`、`last_limit_up_time` 无 NULL，类型为 `DateTime64(3, 'UTC')`。
- `limit_up_type` 观察值为 `换手板`、`一字板`、`T字板`。
- `is_new` 全部为 false；`is_again_limit` 约各半。
- 多个数值字段存在 0 值；profile 未发现负值。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `trade_date` | `date` | `Date` | 涨停池交易日期。 |
| `security_local_code` | `code` | `String` | 只输出 6 位本地代码；不生成 canonical `security_code`。 |
| `security_name` | `name` | `String` | 同花顺股票名称，source-local。 |
| `first_limit_up_time` | `first_limit_up_time` | `DateTime64(3, 'UTC')` | 首次涨停时间，保留 raw timezone 类型。 |
| `last_limit_up_time` | `last_limit_up_time` | `DateTime64(3, 'UTC')` | 最后涨停时间。 |
| `open_num` | `open_num` | `Int64` | 开板次数，0 值有效。 |
| `limit_up_type` | `limit_up_type` | `String` | 涨停类型文本，不跨源映射。 |
| `order_volume` | `order_volume` | `Float64` | 封单量，单位按 raw 保留。 |
| `order_amount` | `order_amount` | `Float64` | 封单金额，单位按 raw 保留。 |
| `is_new` | `is_new` | `Bool` | 当前全 false，保留。 |
| `is_again_limit` | `is_again_limit` | `Bool` | 是否再次涨停。 |
| `limit_up_success_rate` | `limit_up_suc_rate` | `Float64` | 涨停成功率，profile 范围 0 至 1。 |
| `currency_value` | `currency_value` | `Float64` | 流通市值，单位按 raw 保留。 |
| `market_id` | `market_id` | `Int64` | 同花顺市场标识。 |
| `market_type` | `market_type` | `String` | `HS` / `GEM` / `STAR` 等供应商市场类型。 |
| `change_rate` | `change_rate` | `Float64` | 当日涨跌幅，单位按 raw 保留。 |
| `turnover_rate` | `turnover_rate` | `Float64` | 换手率，单位按 raw 保留。 |
| `reason_type` | `reason_type` | `String` | 涨停原因类型原文。 |
| `high_days` | `high_days` | `String` | 连板文本，例如 `首板`、`2天2板`。 |
| `high_days_value_raw` | `high_days_value` | `Int64` | profile 显示数值较大，含义待确认，使用 raw 后缀。 |
| `change_tag` | `change_tag` | `String` | `LIMIT_BACK` / `FIRST_LIMIT` 等标签。 |
| `latest_price` | `latest` | `Float64` | 最新价。 |

## 4. 标准化与 NULL 处理

- 不从 6 位 `code` 推断交易所；如果后续使用 `market_type` 或代码段推断，必须先形成单独 macro 和测试。
- `first_limit_up_time` / `last_limit_up_time` 保留 raw timezone；是否转换为本地交易时区延后。
- `is_new` 全 false 不代表字段无效，staging 保留。
- `high_days_value` 当前数值范围异常大，第一版保留 raw 字段名后缀，不解释业务语义。

## 5. 测试建议

- `trade_date`: `not_null`。
- `security_local_code`: `not_null`，可加 6 位数字格式测试。
- 组合键：`trade_date`, `security_local_code` 唯一。
- `limit_up_type`: `accepted_values`，取值 `换手板`, `一字板`, `T字板`。
- `market_type`: `accepted_values`，取值 `HS`, `GEM`, `STAR`。
- `change_tag`: `accepted_values`，取值 `LIMIT_BACK`, `FIRST_LIMIT`。
- 不对 `security_code` 加测试，因为第一版不输出该字段。

## 6. 延后事项

- 交易所代码推断和 canonical 证券主键生成。
- 涨停原因归并、连板高度解释和 `high_days_value` 解码。
- 时间字段时区转换与交易时段合理性校验。
- 与行情、题材或证券主数据合并。

