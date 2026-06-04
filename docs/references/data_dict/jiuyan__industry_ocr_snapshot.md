# jiuyan__industry_ocr_snapshot 数据字典

本文件由 `pipeline/contracts/datasets/jiuyan__industry_ocr_snapshot.yml` 生成。字段事实以 contract 为准。

- 数据集：`jiuyan__industry_ocr_snapshot`
- 版本：`1`
- 说明：韭研 OCR 成功结果快照
- 粒度：one row per OCR result row per industry image
- Source asset：`source/jiuyan__industry_ocr_snapshot`
- Raw asset：`clickhouse/raw/jiuyan__industry_ocr_snapshot`
- ClickHouse raw：`fleur_raw.jiuyan__industry_ocr_snapshot`
- 分区策略：`snapshot`
- ORDER BY：`(industry_id, image_filename, ocr_row_index)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `industry_id` | `N/A` | `string` | `industry_id` | `String` | 行业研究记录标识 |
| 2 | `image_filename` | `N/A` | `string` | `image_filename` | `String` | OCR 来源图片文件名。 |
| 3 | `image_index` | `N/A` | `int32` | `image_index` | `Int32` | OCR 来源图片在批次中的序号。 |
| 4 | `ocr_row_index` | `N/A` | `int32` | `ocr_row_index` | `Int32` | OCR 结果在图片中的行序号。 |
| 5 | `stock_name` | `string` | `string` | `stock_name` | `LowCardinality(String)` | OCR 识别出的股票名称。 |
| 6 | `theme_path` | `string` | `string` | `theme_path` | `String` | OCR 识别出的题材或主题路径。 |
| 7 | `relation` | `string` | `string` | `relation` | `Nullable(String)` | OCR 识别出的股票与题材关系说明。 |
| 8 | `source` | `string` | `string` | `source` | `LowCardinality(Nullable(String))` | OCR 结果对应的来源文件或来源渠道。 |

## 数据集备注

韭研 OCR 成功结果快照

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
- Downstream snapshot contract consumes source-only asset source/jiuyan__industry_ocr.
- String type decision on S3 parquet source/jiuyan__industry_ocr_snapshot/000000_0.parquet: rows=1228; industry_id nonnull=1228 uniq=63 unique_rate=0.051303, theme_path nonnull=1228 uniq=267 unique_rate=0.217427, relation nonnull=1228 uniq=866 unique_rate=0.705212. industry_id and theme_path use ClickHouse String by explicit schema decision for this raw table; relation is high-uniqueness relationship text and also uses ClickHouse String. Parquet schema remains string.
