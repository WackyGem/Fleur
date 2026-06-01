# jiuyan__industry_ocr_snapshot 数据字典

本文件由 `pipeline/contracts/datasets/jiuyan__industry_ocr_snapshot.yml` 生成。字段事实以 contract 为准。

- 数据集：`jiuyan__industry_ocr_snapshot`
- 版本：`1`
- 说明：韭研 OCR 成功结果快照
- 粒度：one row per OCR result row per industry image
- Source asset：`source/jiuyan__industry_ocr_snapshot`
- Raw asset：`clickhouse/raw/jiuyan__industry_ocr_snapshot`
- ClickHouse raw：`raw.jiuyan__industry_ocr_snapshot`
- 分区策略：`snapshot`
- ORDER BY：`(industry_id, image_filename, ocr_row_index)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | stg 字段 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|----------|
| 1 | `industry_id` | `N/A` | `string` | `industry_id` | `LowCardinality(String)` | `industry_id` | 行业研究记录在来源系统中的唯一标识。 |
| 2 | `image_filename` | `N/A` | `string` | `image_filename` | `String` | `image_filename` | 来源图片在本地或对象存储中的文件名。 |
| 3 | `image_index` | `N/A` | `int32` | `image_index` | `Int32` | `image_index` | 同一批图片中的图片顺序编号。 |
| 4 | `ocr_row_index` | `N/A` | `int32` | `ocr_row_index` | `Int32` | `ocr_row_index` | 图片 OCR 结果中的行序号。 |
| 5 | `stock_name` | `string` | `string` | `stock_name` | `LowCardinality(String)` | `stock_name` | 股票或证券的中文简称。 |
| 6 | `theme_path` | `array` | `string` | `theme_path` | `LowCardinality(String)` | `theme_path` | 题材或主题在来源系统中的层级路径。 |
| 7 | `relation` | `N/A` | `string` | `relation` | `LowCardinality(String)` | `relation` | 记录之间或主题之间的关联说明。 |
| 8 | `source` | `string` | `string` | `source` | `LowCardinality(String)` | `source` | 记录来源系统、渠道或原始文件来源。 |

## 数据集备注

韭研 OCR 成功结果快照

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
