# jiuyan__industry_ocr_snapshot 字段校对

> 生成时间: 2026-06-01 00:00:00 UTC
> OpenAPI 文档: jiuyan__industry_ocr.yaml
> 实现来源: `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr_snapshot.py`

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | industry_id | N/A | ✅ | string | LowCardinality(String) |
| 2 | image_filename | N/A | ✅ | string | String |
| 3 | image_index | N/A | ✅ | int32 | Int32 |
| 4 | ocr_row_index | N/A | ✅ | int32 | Int32 |
| 5 | stock_name | string | ✅ | string | LowCardinality(String) |
| 6 | theme_path | array | ✅ | string | LowCardinality(String) |
| 7 | relation | N/A | ✅ | string | LowCardinality(String) |
| 8 | source | string | ✅ | string | LowCardinality(String) |

## 字段说明

- `jiuyan__industry_ocr_snapshot` 是 source 层 latest snapshot 发布资产，读取已成功 OCR 的单图结果和 PostgreSQL 状态清单后合并输出。
- `image_filename`、`image_index`、`ocr_row_index` 是 snapshot 发布时补充的技术定位字段，不来自 OCR OpenAPI 响应。
- `theme_path` 在 OCR 响应中是 array，进入 source parquet 时按逗号连接为 string。
- `relation` 来自 OCR 归一化字段，对应 OpenAPI 示例中的 `relevance`、`相关性`、`说明` 或 `业务说明` 等别名。

## 统计

- OpenAPI 字段总数: 8
- 资产使用字段数: 8
- 未使用字段数: 3
