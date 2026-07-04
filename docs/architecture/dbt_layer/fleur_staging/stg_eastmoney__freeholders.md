# stg_eastmoney__freeholders 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/eastmoney__freeholders.md`
- Raw source：`source('raw', 'eastmoney__freeholders')`
- 目标位置：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__freeholders.sql`

## 1. 模型定位

EastMoney 前十大流通股东 source-local staging model。模型保留 raw 中每个证券、报告期、名次、股东标识/名称和股份类别的明细行，完成证券代码标准化和字段命名整理。

本模型不做股东身份跨源归并、不按名次裁剪为严格前十、不解析持股变动文本、不修正 `FREE_HOLDNUM_RATIO` 超过 100 的少量供应商异常，也不把明细聚合为每证券每报告期一行。

## 2. 数据特征

- 行数：2,736,392。
- 覆盖证券：5,496 个 `SECUCODE`。
- 日期范围：`END_DATE` 覆盖 2003-12-31 至 2026-06-03。
- raw 中 `SECUCODE + END_DATE + HOLDER_RANK` 不唯一，最高重复组 123 行。
- `SECUCODE + END_DATE + HOLDER_RANK + HOLDER_NEW + HOLDER_NAME + SHARES_TYPE` 未发现重复，可作为 staging 自然键。
- `HOLDER_RANK` 范围为 1 至 50；rank 大于 10 的少量记录不在 staging 层过滤。
- `CHANGE_RATIO` 大量为空，且空值均可由 `HOLD_NUM_CHANGE = '不变'` 或 `HOLD_NUM_CHANGE = '新进'` 解释，属于 source-local 预期缺失。
- `FREE_HOLDNUM_RATIO` 和 `CHANGE_RATIO` 均保留东方财富百分比数值口径，`5` 表示 `5%`，不是 `0.05`。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `security_code` | `SECUCODE` | `String` | 使用 `normalize_cn_security_code(input_format='eastmoney_suffix')` 标准化为 canonical 证券代码。 |
| `report_date` | `END_DATE` | `Date` | 前十大流通股东名单对应的报告期截止日期，使用 canonical 报告期字段。 |
| `holder_rank` | `HOLDER_RANK` | `Int64` | 东方财富返回的流通股东名次；不限制为 1 至 10。 |
| `holder_identifier` | `HOLDER_NEW` | `String` | 持有人业务标识文本，可能为数字编码或股东姓名，不作为跨源稳定股东 ID。 |
| `holder_name` | `HOLDER_NAME` | `String` | 东方财富披露的流通股东名称。 |
| `holder_type` | `HOLDER_TYPE` | `LowCardinality(String)` | 供应商股东类型或机构类型文本，不做同义归一。 |
| `shares_type` | `SHARES_TYPE` | `LowCardinality(String)` | 供应商股份类别文本，例如 `A股`、`B股`、`H股` 或混合类别。 |
| `free_float_hold_shares` | `HOLD_NUM` | `Int64` | 股东持有流通股数量，单位为股。 |
| `free_float_holdnum_ratio_pct` | `FREE_HOLDNUM_RATIO` | `Float64` | 股东持有流通股数量占流通股比例，保留百分比数值。 |
| `hold_num_change_text` | `HOLD_NUM_CHANGE` | `String` | 较上期持股数量变动的原始文本，可为数值文本、`不变` 或 `新进`。 |
| `change_ratio_pct` | `CHANGE_RATIO` | `Nullable(Float64)` | 较上期持股数量变动比例，保留百分比数值；预期可空。 |

## 4. 标准化与 NULL 处理

- `SECUCODE` 使用 `normalize_cn_security_code(..., input_format='eastmoney_suffix')` 输出为 `security_code`。
- `SECURITY_CODE` 已在 raw profile 中画像，但当前 staging 不输出；如后续需要本地代码，可补 `security_local_code`。
- `END_DATE` 暴露为 `report_date`，用于和股东披露报告期、后续 as-of 股本计算对齐。
- 不对 `HOLD_NUM_CHANGE` 做数值解析或方向拆分。
- 不把 `CHANGE_RATIO` 的预期缺失填 0。
- 不修正 `FREE_HOLDNUM_RATIO > 100` 的少量异常值，后续如需要应在 intermediate/mart 或质量告警中处理。

## 5. 测试建议

- 模型级组合唯一：`security_code`, `report_date`, `holder_rank`, `holder_identifier`, `holder_name`, `shares_type`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `report_date`: `not_null`。
- `holder_rank`: `not_null`。
- `holder_identifier`: `not_null`。
- `holder_name`: `not_null`。
- `shares_type`: `not_null`。
- `free_float_hold_shares`: `not_null`。
- `free_float_holdnum_ratio_pct`: `not_null`。
- `change_ratio_pct`: 不加 `not_null`，因为“不变”和“新进”场景为空属于预期缺失。
- 不对 `holder_rank` 或 `shares_type` 加过窄 accepted-values，避免把真实长尾披露误判为失败。

## 6. 延后事项

- 每证券每报告期每名次一行的业务去重或排序规则。
- `HOLDER_NEW` / `HOLDER_NAME` 的股东身份跨源或跨期归并。
- `HOLD_NUM_CHANGE` 的数值解析、方向拆分或变动比例重算。
- 只保留严格前十大流通股东的业务过滤。
- `A股,H股`、`A股,B股` 等混合股份类别的 A 股部分拆分。
