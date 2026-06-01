# jiuyan__industry_ocr 数据字典

本文件由 `pipeline/contracts/datasets/jiuyan__industry_ocr.yml` 生成。字段事实以 contract 为准。

- 数据集：`jiuyan__industry_ocr`
- 版本：`1`
- 说明：韭研行业图片 OCR 结果 source 分区
- 粒度：one row per OCR result row per industry image
- Source asset：`source/jiuyan__industry_ocr`
- Raw asset：不适用
- ClickHouse raw：不适用

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | 中文描述 |
|---|----------|----------|--------------|----------|
| 1 | `industry_id` | `string` | `string` | OCR 来源行业图片所属的行业研究记录标识。 |
| 2 | `stock_name` | `string` | `string` | OCR 识别出的股票或证券名称。 |
| 3 | `theme_path` | `array` | `string` | OCR 识别出的题材或主题层级路径。 |
| 4 | `relation` | `string` | `string` | OCR 识别出的股票与题材关系说明。 |
| 5 | `source` | `string` | `string` | OCR 结果对应的来源文件或来源渠道。 |

## 数据集备注

韭研行业图片 OCR 结果 source 分区；该 source-only asset 不直接同步 ClickHouse raw。

## 校验记录

- Source-only contract added by Plan 0020 Phase 3 from Dagster JIUYAN_INDUSTRY_OCR_SCHEMA.
