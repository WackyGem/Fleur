# stg_baostock__query_stock_basic 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/baostock__query_stock_basic.md`
- Raw source：`source('raw', 'baostock__query_stock_basic')`
- 目标位置：`pipeline/elt/models/staging/baostock/stg_baostock__query_stock_basic.sql`

## 1. 模型定位

BaoStock 证券基础信息快照的 source-local staging model。staging 只统一证券代码、暴露上市/退市日期和供应商类型状态，不做证券主数据修正、退市状态裁决或跨源实体合并。

## 2. 数据特征

- 行数：8,769。
- 粒度：一行一个 BaoStock `code`。
- 候选键：`code`，profile 未发现重复。
- `code` 全部为 `sh.000001` 类供应商前缀格式。
- `ipoDate` 无 NULL，范围 1990-12-10 至 2026-06-01。
- `outDate` NULL 7,644 行，表示未退市或 source 未提供退市日期。
- `type` 取值为 `1`, `2`, `4`, `5`；`status` 取值为 `1`, `0`。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `security_code` | `code` | `String` | 使用 `normalize_cn_security_code(input_format='baostock_prefix')` 转为 canonical 格式。 |
| `security_local_code` | `code` | `String` | 使用 `cn_security_local_code(input_format='baostock_prefix')` 提取本地代码。 |
| `exchange_code` | `code` | `LowCardinality(String)` | 使用 `cn_exchange_code(input_format='baostock_prefix')` 提取交易所代码，取值为 `SSE`、`SZSE`。 |
| `security_name` | `code_name` | `String` | source-local 证券简称；不做历史简称归并。 |
| `ipo_date` | `ipoDate` | `Date` | source-local 上市日期；如纳入 canonical glossary，可再命名为项目统一字段。 |
| `out_date` | `outDate` | `Nullable(Date)` | 保留 NULL，NULL 不代表数据质量错误。 |
| `security_type_code` | `type` | `Int8` | BaoStock 证券类型原始枚举编码。 |
| `security_type` | `type` | `Enum8('stock' = 1, 'index' = 2, 'other' = 3, 'convertible_bond' = 4, 'etf' = 5)` | 由 `security_type_code` 确定性映射出的 ClickHouse 枚举值。 |
| `security_board` | `security_code` | `Nullable(Enum8('sse_main_board' = 1, 'szse_main_board' = 2, 'chinext' = 3, 'star_market' = 4))` | 证券所属板块，仅股票类型证券需要。根据 canonical `security_code` 解析交易所和本地代码段，派生沪市主板、深市主板、创业板、科创板；代码段不命中股票板块时保留 NULL。 |
| `listing_status_code` | `status` | `Int8` | BaoStock 上市状态原始枚举编码。 |
| `listing_status` | `status` | `Enum8('delisted' = 0, 'listed' = 1)` | 由 `listing_status_code` 确定性映射出的 ClickHouse 枚举值。 |
| `is_listed` | `listing_status` | `Bool` | 当前快照是否上市；`listing_status = 'listed'` 时为 true，不代表历史任意交易日状态。 |

## 4. 枚举字段设计

### 证券类型枚举

| 编码字段 | 枚举字段 | 来源编码 | 枚举值 | 中文说明 | Profile 观察计数 |
|----------|----------|----------|--------|----------|------------------|
| `security_type_code` | `security_type` | `1` | `stock` | 股票 | 5,532 |
| `security_type_code` | `security_type` | `2` | `index` | 指数 | 596 |
| `security_type_code` | `security_type` | `3` | `other` | 其他 | 0 |
| `security_type_code` | `security_type` | `4` | `convertible_bond` | 可转债 | 1,097 |
| `security_type_code` | `security_type` | `5` | `etf` | ETF | 1,544 |

### 证券所属板块枚举

| 枚举字段 | 条件 | 枚举值 | 中文说明 |
|----------|------|--------|----------|
| `security_board` | `security_code` 解析为 `SH`，且本地代码以 `68` 开头 | `star_market` | 科创板 |
| `security_board` | `security_code` 解析为 `SH`，且本地代码以 `600`、`601`、`603` 或 `605` 开头 | `sse_main_board` | 沪市主板 |
| `security_board` | `security_code` 解析为 `SZ`，且本地代码以 `30` 开头 | `chinext` | 创业板 |
| `security_board` | `security_code` 解析为 `SZ`，且本地代码以 `000`、`001`、`002` 或 `003` 开头 | `szse_main_board` | 深市主板 |
| `security_board` | `security_code` 未命中上述股票板块代码段 | `NULL` | 非股票证券或暂未纳入口径的代码段不需要所属板块 |

板块语义边界：

- `security_board` 是基于 canonical `security_code` 在 staging 层做的确定性代码段派生，属于 source-local 分类。
- 当前只表达沪市主板、深市主板、创业板、科创板；如后续出现未命中上述代码段但需要归入股票板块的记录，应先补充 raw profile 和本设计文档，再决定是否扩展枚举或保留 NULL。
- 不在 staging 层处理跨市场板块历史变更、北交所分类、B 股分类或业务口径修正。

### 上市状态枚举

| 编码字段 | 枚举字段 | 来源编码 | 枚举值 | 中文说明 | Profile 观察计数 |
|----------|----------|----------|--------|----------|------------------|
| `listing_status_code` | `listing_status` | `1` | `listed` | 上市 | 7,644 |
| `listing_status_code` | `listing_status` | `0` | `delisted` | 退市 | 1,125 |

枚举语义边界：

- `security_type` 和 `listing_status` 使用 ClickHouse `Enum8` 表达 BaoStock source-local 枚举值，不是跨源 canonical 枚举。
- `security_type_code = 3` 是 BaoStock 证券类型定义中的合法编码，本次 profile 未观察到；测试仍应允许该值，避免未来 source 出现合法记录时误报。
- 如后续 source 出现本表未列出的编码，应先更新 raw profile 和本设计文档，再调整映射和测试。
- 编码中文业务含义、跨源证券类型归一和上市/退市状态裁决延后到 intermediate/mart 或主数据模型。

## 5. 标准化与 NULL 处理

- `code` 使用现有 BaoStock prefix macro；不要从 6 位本地代码推断交易所。
- `outDate` 的 NULL 是预期缺失，staging 不填充、不推断。
- `code_name` 无空字符串；仅做 `trim`，如未来出现空字符串再转 NULL。
- `type` / `status` 在 staging 只映射为 BaoStock source-local 枚举值，不映射为跨源枚举。
- `security_board` 只根据 `security_code` 可解析出的沪深股票板块代码段派生；非股票证券或未命中代码段的证券必须保留 NULL，不填充为 `other`。

## 6. 测试建议

- `security_code`: `not_null`，`cn_security_code_format`，唯一。
- `exchange_code`: `accepted_values`，取值 `SSE`, `SZSE`。
- `ipo_date`: `not_null`。
- `security_type_code`: `accepted_values`，取值 `1`, `2`, `3`, `4`, `5`。
- `security_type`: `accepted_values`，取值 `stock`, `index`, `other`, `convertible_bond`, `etf`。
- `security_board`: `accepted_values`，取值 `sse_main_board`, `szse_main_board`, `chinext`, `star_market`，允许 NULL。
- `security_board`: 条件测试，`security_code` 可解析为沪市主板、深市主板、创业板、科创板代码段时必须非 NULL；未命中这些代码段时必须为 NULL。
- `listing_status_code`: `accepted_values`，取值 `0`, `1`。
- `listing_status`: `accepted_values`，取值 `listed`, `delisted`。
- `is_listed`: `not_null`。
- 不对 `out_date` 加 `not_null`。

## 7. 延后事项

- 上市/退市状态与证券主数据的最终裁决。
- 证券类型编码的跨源枚举映射。
- 证券所属板块的跨源口径归一、历史板块迁移和非沪深股票板块扩展。
- 历史简称、代码迁移、交易所归属修正。
