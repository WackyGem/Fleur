# RFC 0004: 韭研产业研究图片 OCR 资产设计

状态：草案

## 摘要

本文定义 `jiuyan__industry_list` 产业研究图片 OCR 的第一版需求草稿。
当前约 950 条产业研究记录，预计解析出约 1200 张图片需要 OCR。

流程分为三步：

1. 从 raw Parquet `imgs` 字段解析图片 URL，下载图片并写入 S3。
2. 使用 PostgreSQL `pipeline` 库记录图片下载和 OCR 状态，保证幂等。
3. 调用本地 OCR 服务做表格信息抽取，写入 Parquet 资产。

目标资产：

```text
jiuyan__industry_images
jiuyan__industry_ocr
```

## 目标

- 从 `raw/jiuyan__industry_list` 的 `imgs` 字段解析图片 URL。
- 下载 OSS 图片，写入 S3 `img/jiuyan__industry_images/<image_filename>`。
- 使用 PostgreSQL 记录下载/OCR 状态，避免重复处理和重试。
- 按单张图片调用 OpenAI-compatible OCR 服务，要求返回扁平 JSON array。
- 校验 OCR 输出 schema，写入 Parquet，保留文章和图片来源关联。

## 非目标

- 不重新采集 `jiuyan__industry_list`。
- 不涉及 dbt 模型、ClickHouse 表或前端查询页面。
- 不做股票代码标准化、实体消歧或外部数据关联增强。
- 不保存 OCR prompt 中不能由图片直接看出的推断信息。
- 不设计多模型投票或人工审核工作流。

## 参考资料

上游接口：
```text
docs/references/remote_endpoint/jiuyan__industry_list.md
docs/references/openapi/jiuyan__industry_list.yaml
docs/RFC/0003-http-resource-market-event-ingestion.md
```

OCR 接口和样例图片：
```text
docs/references/openapi/jiuyan__industry_ocr.yaml
docs/references/jiuyan_images/README.md
```

项目设计约束：
```text
docs/ADR/0001-market-data-raw-assets-on-dagster.md
docs/ADR/0002-s3-parquet-storage-layout.md
```

## 资产矩阵

| Asset | 输入 | 输出 | 分区 | 存储模式 | 行粒度 |
| --- | --- | --- | --- | --- | --- |
| `jiuyan__industry_images` | `raw/jiuyan__industry_list` | S3 图片 + PostgreSQL 状态 | 无 | image objects | 一张去重后的图片 |
| `jiuyan__industry_ocr` | `jiuyan__industry_images` + PostgreSQL | 单图 OCR Parquet + PostgreSQL 状态 | 无 | object-per-image | 一个 OCR 条目 |

图片资产路径：

```text
img/jiuyan__industry_images/<image_filename>
```

文件名取 URL path basename，忽略 query string。例如 `https://cdn.jiuyangongshe.com/import/4E01FC2B-F3E0-4989-A3CE-5764A068D84D.png?x-oss-process=...` 对应 `4E01FC2B-F3E0-4989-A3CE-5764A068D84D.png`。缺失文件名或重名时用 URL hash 兜底。

## OCR 存储方案

OCR 结果按图片写入独立对象，避免多个 run 并发覆盖同一个 bucket 文件：

```text
raw/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet
```

上游 `raw/jiuyan__industry_list` 生产路径采用 `000000_0` 命名。

选择单图对象的原因：

- OCR 是按单张图片调用，结果天然以图片为幂等边界。
- 新增图片只写自己的结果对象，不需要先读取旧 bucket 文件再覆盖写回。
- 并发运行时，不同图片写不同 S3 key，配合 PostgreSQL 行级领取避免重复处理。
- 单图失败只影响该图片，不会破坏同 bucket 内其他图片的既有结果。

读取规则：

- PostgreSQL 记录每张图片的 `ocr_result_s3_key`。
- 已 OCR 成功的图片默认不重复调用，直接保留已有 `ocr_result_s3_key`。
- 下游需要全量读取时，根据 PostgreSQL 中 `ocr_status='success'` 的清单读取所有单图 Parquet。

如果后续全量读取小文件成本变高，可新增一个独立的 compaction/snapshot 资产，串行读取所有成功的单图 OCR 结果并写出少量汇总 Parquet。该汇总资产只做读后合并，不参与 OCR 领取和单图结果写入，因此不会影响 OCR 并发安全。

不采用 hash bucket 快照写法。bucket 快照会让多个 run 竞争同一个 Parquet 文件，且新增结果需要先读出桶内旧数据再合并覆盖，复杂度和并发风险都更高。

## 输入数据

### `imgs` 解析规则

`imgs` 是 JSON 字符串，解析步骤：

1. `imgs` 为空/null/非法 JSON 时跳过。
2. `json.loads` 得到字符串数组。
3. 对每个字符串用正则提取图片 URL（支持 `.png`/`.jpg`/`.jpeg`）。
4. 不按英文逗号切分——OSS query 参数可能包含逗号。
5. 同一 `industry_id` 内去重；全局重复 URL 可去重下载，但输出能关联回所有引用文章。

典型值：

```text
["https://cdn.jiuyangongshe.com/import/9DB8BFE7-47A2-42FB-9649-E42D8913006D.png"]
```

异常情况：数组元素可能包含逗号拼接的多个 URL，需用正则而非 split 提取。

## PostgreSQL 状态库

新增 `pipeline` database，维护 `jiuyan_industry_images` 表，记录每张图片的下载状态和 OCR 状态。主键 `image_filename`，按 `(ocr_status)` 建索引供 OCR 资产领取任务。

状态枚举：

```text
download_status: pending | success | failed
ocr_status: pending | running | success | failed
```

幂等规则：

- 按 `image_filename` upsert；已下载成功的不重复下载。
- OCR 资产领取 `download_status='success'` 且 `ocr_status in ('pending', 'failed')` 的图片。
- 领取时原子更新 `ocr_status='running'` + 行级锁，避免并发重复处理。
- OCR 成功后写入单图 Parquet，并更新 `ocr_status='success'`、`ocr_result_s3_key` 和统计字段。

## 图片下载

- HTTP GET 下载，设置 User-Agent 和超时。
- 5xx/429 退避重试，4xx 不重试。
- 校验 `Content-Type` 为 `image/*`，规范化 MIME 供 data URL 使用。
- 记录字节数和 SHA256。
- 写入 S3 成功后再更新 PostgreSQL `download_status='success'`。
- 失败时写回错误类型和信息，下次运行可重试。

## OCR 服务

输入图片从 S3 读取（不直接请求远端 OSS），构造 `data:image/*;base64,...` data URL。

第一版不设置单张图片大小限制；如 OCR 服务或 HTTP 客户端返回超时、请求过大等错误，按图片级失败记录并进入重试/失败率统计。

接口：`POST <JIUYAN_OCR_BASE_URL>/v1/chat/completions`

OCR 模型约 4B，prompt 和 JSON schema 需要保持简单。`response_format` 使用 `json_schema` strict mode，schema 字段：

```text
stock_name, theme_path, relation, source
```

默认参数见 `docs/references/openapi/jiuyan__industry_ocr.yaml`。

OCR 调用并发数固定为 6。并发限制按单次 asset materialization 生效，用于控制本地 OCR 服务压力；如服务返回 429/5xx，按错误处理策略退避重试。

OCR prompt 原则：

- 只看图片文字，不补充常识，不做股票代码标准化、实体消歧或题材归一。
- 每个“个股 × 题材路径”关联条目输出一个 object。
- `stock_name` 填“个股/公司/标的”列中的名称。
- `theme_path` 填该行左侧主题分类；多级分类用英文逗号 `,` 连接，例如 `AI服务器相关,覆铜板`。
- `relation` 填该行描述个股与题材关系的原文说明，可合并多个相关说明列，但不要求模型总结。
- `source` 填“信源/资料来源/来源”列；没有则填空字符串。
- 合并单元格主题继承到覆盖的每一行。
- 风险提示、免责声明、水印、页脚、二维码、跳转链接、装饰文字不抽取。
- 看不清或不存在的字段填空字符串，不猜测。

输出必须是 JSON array，不输出解释、Markdown 或代码块。示例：

```json
[
  {
    "stock_name": "示例股份",
    "theme_path": "一级题材,二级题材",
    "relation": "与该题材相关的原文说明",
    "source": "资料来源"
  }
]
```

重复个股处理规则：

- 同一个个股出现在多个题材下时，保留多条记录。
- 同一张图片内，只有 `stock_name + theme_path + relation + source` 完全相同时才去重。
- `stock_name` 相同但 `theme_path` 不同，必须保留。
- `stock_name` 和 `theme_path` 相同但 `relation` 不同，第一版也保留，后续再做聚合。

第一版不单独设计 `ocr_version` 编码体系。OCR 变化由代码仓库版本、Dagster run 记录和 metadata 中的 `ocr_model` / `base_url_host` 追踪；需要重跑时，将目标图片的 `ocr_status` 重置为 `pending` 后重新 materialize。重新处理成功后覆盖对应图片的单图结果对象。

## 输出资产

路径：

```text
raw/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet
```

输出字段：

```text
industry_id, stock_name, theme_path, relation, source
```

Parquet schema：

| Column | PyArrow type | Nullable | 说明 |
| --- | --- | --- | --- |
| `industry_id` | `pa.string()` | no | 韭研产业研究文章 ID，用于关联回 `jiuyan__industry_list`。 |
| `stock_name` | `pa.string()` | no | 个股、公司或标的名称。 |
| `theme_path` | `pa.string()` | no | 多级题材路径，使用英文逗号 `,` 连接。 |
| `relation` | `pa.string()` | no | 个股与题材关系的原文说明。 |
| `source` | `pa.string()` | no | 来源、信源或资料来源。 |

`industry_id` 必须非空。其它字符串缺失值写空字符串，不写 null。OCR 成功但返回空数组时，仍用该 schema 写入 0 行 Parquet。

`image_index`、`image_filename`、`image_url`、`image_s3_key`、`ocr_result_s3_key` 等技术溯源字段只保存在 PostgreSQL 状态表、S3 对象路径或 Dagster metadata 中，不进入业务 Parquet。

第一版不输出 OCR 原始响应、OCR 原始 JSON array 字符串、prompt 全文、base64 图片内容或运行信息列（这些进入 Dagster metadata 或日志）。

## 错误处理

图片级错误类型：

- `imgs_parse_error` / `image_download_error` / `image_content_type_error` / `image_s3_write_error` / `image_s3_read_error` / `ocr_http_error` / `ocr_response_parse_error` / `ocr_schema_error`

策略：

- 单张图片失败不直接导致整个 asset 失败；按本次 materialization 的可处理图片集合统计失败率。
- 失败率定义为 `OCR 失败图片数 / OCR 请求图片数`。空 OCR 结果不计为失败，但单独计入 metadata。
- 本次 OCR 图片失败率阈值默认 20%；超过阈值时 asset 失败，未超过时继续产出成功图片的 OCR 结果。
- 所有失败计入 metadata，保留 `industry_id`、`image_filename`、错误类型。
- 可解析图片全部 OCR 失败时 asset 应失败。输入无图片 URL 时 asset 应失败（通常表示上游 schema 错误）。

## Metadata

Dagster materialization metadata 覆盖：

- 处理概览：文章数、含图文章数、解析 URL 数
- 下载统计：成功/失败/跳过数，下载耗时
- OCR 统计：请求数、成功/失败/空结果数，OCR 耗时
- 输出信息：处理图片数、成功图片数、结果行数、空结果图片数
- 环境信息：OCR model、base URL host、S3 bucket

不得记录认证信息、密钥或图片 base64 内容。

## 数据质量与校验

单元测试覆盖：

- `imgs` 空/非法 JSON/合法 JSON 数组的处理
- 单元素多 URL 提取、逗号 query 参数不被错误切分
- 重复 URL 去重、MIME 规范化、S3 key 稳定生成
- PostgreSQL 幂等选择（已成功的不重复下载/OCR）
- OCR 响应 JSON array 解析和 schema 校验
- 输出行保留 `industry_id`，不包含 `image_filename`、`image_s3_key` 等技术溯源字段

真实样例验证：

- 使用 `docs/references/jiuyan_images/` 中至少两张样例图片调用 OCR 服务
- 验证表头变化时 `relation` 和 `source` 输出正确
- 验证合并单元格主题进入每行 `theme_path`
- 验证同一个个股出现在多个 `theme_path` 下时保留多条记录
- 验证风险提示、水印不会成为结果行

## 验收标准

实现完成后应满足：

1. **资产可见**：Dagster UI 中能看到 `jiuyan__industry_images` 和 `jiuyan__industry_ocr` 两个 asset。
2. **图片下载**：能从 `imgs` 解析 URL → 下载 → 写入 S3 → 记录 PostgreSQL 状态，已成功的跳过。
3. **OCR 抽取**：从 PostgreSQL 领取图片 → 从 S3 读取 → 调用 OCR → schema 校验 → 写入单图 Parquet，已成功的跳过。
4. **幂等与重试**：失败项下次运行重试，成功项保留已有 `ocr_result_s3_key`。
5. **数据完整性**：输出行保留 `industry_id`，可直接关联 `jiuyan__industry_list`；图片级溯源通过 PostgreSQL 状态表和 `ocr_result_s3_key` 完成；同一 `stock_name` 在多个 `theme_path` 下的关联记录不被错误去重。
6. **可观测性**：下载失败、OCR 失败、空 OCR 可在 metadata 中观测。敏感信息不写入 Parquet。

## 实施顺序

1. **基础设施**：创建 `pipeline` database、Alembic 迁移工程、环境变量配置。
2. **图片下载资产**：`imgs` 解析 → 下载 → S3 写入 → PostgreSQL 状态 upsert → `jiuyan__industry_images` asset。
3. **OCR 资产**：PostgreSQL 领取 → S3 读取 → OCR 调用 → schema 校验 → 写入单图 `jiuyan__industry_ocr` 对象。
4. **测试与联调**：单元测试覆盖核心函数 + 本地 OCR 服务联调 + 真实数据小批量端到端验证。

## 待确认

无。
