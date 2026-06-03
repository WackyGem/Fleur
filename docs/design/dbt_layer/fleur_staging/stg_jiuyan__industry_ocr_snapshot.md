# stg_jiuyan__industry_ocr_snapshot 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/jiuyan__industry_ocr_snapshot.md`
- Raw source：`source('raw', 'jiuyan__industry_ocr_snapshot')`
- 目标位置：`pipeline/elt/models/staging/jiuyan/stg_jiuyan__industry_ocr_snapshot.sql`

## 1. 模型定位

韭研行业研究图片 OCR 结果快照的 source-local staging model。模型保留 OCR 行级结果，完成字段命名和轻量文本空值处理，不做股票名称到证券代码匹配、主题路径拆解或 OCR 纠错。

## 2. 数据特征

- 行数：1,069。
- 粒度：一行代表一个 `industry_id`, `image_filename`, `image_index`, `ocr_row_index` OCR 结果行。
- 候选键：`industry_id`, `image_filename`, `image_index`, `ocr_row_index`，profile 未发现重复。
- 无 Date/DateTime 字段。
- 无证券代码字段；只有 `stock_name`。
- `source` 有 535 行空字符串，其他值包括 `互动`、`公告`、`调研`、`年报` 等。
- `image_index` 范围 0 至 1；`ocr_row_index` 范围 0 至 171。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `industry_id` | `industry_id` | `String` | 行业研究记录 ID。 |
| `image_filename` | `image_filename` | `String` | OCR 来源图片文件名。 |
| `image_index` | `image_index` | `Int32` | 图片序号，0 起始值有效。 |
| `ocr_row_index` | `ocr_row_index` | `Int32` | OCR 行序号，0 起始值有效。 |
| `stock_name` | `stock_name` | `String` | OCR 识别出的股票名称；不匹配证券主数据。 |
| `theme_path` | `theme_path` | `String` | OCR 识别出的主题路径原文。 |
| `relation` | `relation` | `String` | 股票与主题关系说明原文。 |
| `ocr_source` | `source` | `Nullable(String)` | `trim(nullif(source, ''))`，空字符串转 NULL。 |

## 4. 标准化与 NULL 处理

- 不输出 `security_code`；仅凭 OCR 股票名无法稳定构造 canonical join key。
- `source` 空字符串转 NULL。
- `theme_path` 不拆层级；主题层级解析和清洗延后。
- `image_index = 0` 与 `ocr_row_index = 0` 是有效序号，不转 NULL。

## 5. 测试建议

- `industry_id`, `image_filename`, `image_index`, `ocr_row_index`: `not_null`。
- 组合键：`industry_id`, `image_filename`, `image_index`, `ocr_row_index` 唯一。
- `stock_name`: 可加 `not_null`，但不加证券代码格式测试。
- 不对 `ocr_source` 加 `not_null`。

## 6. 延后事项

- OCR 股票名称到证券主数据的实体匹配。
- 主题路径拆解、主题归并和关系类型归一。
- OCR 纠错、置信度建模和图片维表建设。

