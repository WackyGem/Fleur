# Data Dictionary Type Mapping

`docs/references/data_dict/*.md` 同时记录远端 OpenAPI 字段、source asset 实际使用字段、S3 Parquet 的 PyArrow 类型，以及 ClickHouse raw 层建议字段类型。

## PyArrow 到 ClickHouse 映射

| PyArrow 类型 | ClickHouse 类型 | 说明 |
|--------------|----------------|------|
| `string` | `LowCardinality(String)` | 默认用于标识、分类、状态、名称等重复值较多的字符串。 |
| `string` | `String` | 用于标题、正文、图片文件名、JSON/URL 列表、长文本或高基数字符串。 |
| `bool` | `Bool` | 布尔字段。 |
| `int8` | `Int8` | 保留 PyArrow 位宽，用于源接口已限定的小整数枚举。 |
| `int32` | `Int32` | 保留 PyArrow 位宽。 |
| `int64` | `Int64` | 保留 PyArrow 位宽；后续有明确非负范围时可再收窄为 `UInt*`。 |
| `double` | `Float64` | 保留 PyArrow 浮点语义。 |
| `date32[day]` | `Date` | 日期字段。 |
| `timestamp[ns]` | `DateTime64(3)` | 无时区时间戳；ClickHouse raw 层先保留毫秒精度。 |
| `timestamp[ns, tz=UTC]` | `DateTime64(3, 'UTC')` | UTC 时间戳；ClickHouse raw 层先保留毫秒精度。 |
| `time32[ms]` | `String` | ClickHouse 没有直接等价的 time-of-day 类型，raw 层先保留字符串表达。 |
| `-` | `-` | 未进入资产输出的字段，不进入 ClickHouse raw 表。 |

## 设计规则

- ClickHouse raw 字段类型以 PyArrow 类型为第一依据，不回退到全 `String`。
- `LowCardinality(String)` 是初始设计建议；如果实际 cardinality 超过 10,000 或写入性能受到影响，应在 raw spec 中调整为 `String` 并同步更新 data_dict。
- `DateTime64(3)` 是 raw 层默认精度；如果下游确实需要纳秒精度，应先新增设计说明，再改为更高精度。
- Nullable 不作为默认设计。source 层应先明确缺失值语义；只有业务上必须区分 `NULL` 和默认值时，ClickHouse raw 才使用 `Nullable(...)`。
