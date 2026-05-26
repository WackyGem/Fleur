# eastmoney__equity_history

东方财富 F10 — 上市公司股本变动历史

## Endpoint

```
GET https://datacenter.eastmoney.com/securities/api/data/v1/get
```

无需认证，直接 GET 请求。

## Query Parameters

| 参数 | 必填 | 说明 | 示例值 |
|:-----|:-----|:-----|:-------|
| `reportName` | 是 | 报表标识，固定值 | `RPT_F10_EH_EQUITY` |
| `columns` | 是 | 返回字段，`ALL` = 全部（含比例、变动量等额外字段） | `ALL` |
| `filter` | 是 | DataCenter DSL 过滤条件 | `(SECUCODE="601088.SH")` |
| `pageNumber` | 是 | 页码，从 1 开始 | `1` |
| `pageSize` | 是 | 每页条数 | `500` |
| `sortColumns` | 是 | 排序字段 | `NOTICE_DATE` |
| `sortTypes` | 是 | 排序方向：`-1` 降序，`1` 升序 | `-1` |
| `source` | 是 | 数据来源标识 | `HSF10` |
| `client` | 是 | 客户端标识 | `PC` |
| `quoteColumns` | 否 | 行情字段，一般留空 | — |
| `v` | 否 | 缓存破坏参数（时间戳数字） | `04325353904715509` |

## Response

顶层 JSON：

```json
{
  "success": true,
  "message": "ok",
  "code": 0,
  "result": {
    "pages": 13,
    "count": 62,
    "data": [ ... ]
  }
}
```

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `success` | bool | 请求是否成功 |
| `code` | int | 0 = 正常；`9201` + `result=null` 表示空响应 |
| `result.pages` | int | 总页数 |
| `result.count` | int | 总记录数 |
| `result.data` | array | 股本变动记录数组 |

### 标识字段

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `SECUCODE` | string | 证券代码（含市场后缀） | `"601088.SH"` |
| `SECURITY_CODE` | string | 证券代码（纯数字） | `"601088"` |
| `SECURITY_NAME_ABBR` | string | 证券简称 | `"中国神华"` |
| `ORG_CODE` | string | 机构代码 | `"10032705"` |
| `END_DATE` | string | 变动截止日期 | `"2026-04-07 00:00:00"` |
| `NOTICE_DATE` | string | 公告日期 | `"2026-04-09 00:00:00"` |
| `LISTING_DATE` | string? | 上市流通日期 | `"2026-04-08 00:00:00"` |
| `CHANGE_REASON` | string | 变动原因 | `"增发A股上市"` |
| `CHANGE_REASON_EXPLAIN` | string? | 变动原因详细说明 | `"非公开增发A股上市"` |

### 股本总量（股）

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `TOTAL_SHARES` | int | 总股本 | `21689434304` |
| `LIMITED_SHARES` | int? | 有限售条件股份 | `1820914349` |
| `UNLIMITED_SHARES` | int | 无限售条件股份（已流通） | `19868519955` |
| `FREE_SHARES` | int | 流通股（通常 = TOTAL_SHARES） | `21689434304` |

### 已上市流通股明细（股）

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `LISTED_A_SHARES` | int | 已上市流通 A 股 | `16491037955` |
| `B_FREE_SHARE` | int? | 已上市流通 B 股 | `null` |
| `H_FREE_SHARE` | int? | 已上市流通 H 股 | `3377482000` |
| `OTHER_FREE_SHARES` | int? | 其他已上市流通股 | `null` |

### 有限售条件股份明细（股）

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `LIMITED_A_SHARES` | int? | 限售 A 股 | `1820914349` |
| `LIMITED_B_SHARES` | int? | 限售 B 股 | `null` |
| `LIMITED_H_SHARES` | int? | 限售 H 股 | `null` |
| `LIMITED_STATE_SHARES` | int? | 国家持股（限售） | `1363248446` |
| `LIMITED_STATE_LEGAL` | int? | 国有法人持股（限售） | `null` |
| `LIMITED_DOMESTIC_NOSTATE` | int? | 境内非国有法人持股（限售） | `423569794` |
| `LIMITED_DOMESTIC_NATURAL` | int? | 境内自然人持股（限售） | `13272311` |
| `LIMITED_OVERSEAS_NOSTATE` | int? | 境外非国有法人持股（限售） | `20823798` |
| `LIMITED_OVERSEAS_NATURAL` | int? | 境外自然人持股（限售） | `null` |
| `LIMITED_OTHARS` | int? | 其他限售股份 | `436842105` |
| `LIMITED_FOREIGN_SHARES` | int? | 外资持股（限售） | `20823798` |
| `LOCK_SHARES` | int? | 锁定股份 | `null` |
| `NON_FREE_SHARES` | int? | 非自由流通股 | `null` |

### 发起人/募集股份（股）

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `SPONSOR_SHARES` | int? | 发起人股份 | `null` |
| `STATE_SPONSOR_SHARES` | int? | 国家发起人股份 | `null` |
| `SPONSOR_SOCIAL_SHARES` | int? | 社会发起人股份 | `null` |
| `RAISE_SHARES` | int? | 募集法人股份 | `null` |
| `RAISE_STATE_SHARES` | int? | 国家募集法人股份 | `null` |
| `RAISE_DOMESTIC_SHARES` | int? | 境内募集法人股份 | `null` |
| `RAISE_OVERSEAS_SHARES` | int? | 境外募集法人股份 | `null` |

### `columns=ALL` 额外字段

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `LIMITED_SHARES_RATIO` | float? | 限售股比例（%） | `8.395398070222` |
| `LISTED_SHARES_RATIO` | float | 已流通股比例（%） | `91.604601929778` |
| `LISTED_A_SHARES_RATIO` | float | A 股流通比例（%） | `76.032586760267` |
| `H_FREE_SHARE_RATIO` | float? | H 股流通比例（%） | `15.572015169511` |
| `LISTED_A_RATIOPC` | float | A 股占已流通比例（%） | `83.000837467262` |
| `LISTED_H_RATIOPC` | float? | H 股占已流通比例（%） | `16.999162532738` |
| `TOTAL_SHARES_CHANGE` | int? | 总股本变动量 | `457665903` |
| `LIMITED_SHARES_CHANGE` | int? | 限售股变动量 | `457665903` |
| `UNLIMITED_SHARES_CHANGE` | int | 流通股变动量 | `0` |
| `IS_FREE_WINDOW` | string | 是否为自由流通窗口 | `"1"` |

## Filter DSL

`filter` 是东方财富数据中心自带的查询表达式。多个条件直接拼接，无显式 AND/OR 运算符。

**按证券代码**：

```
(SECUCODE="601088.SH")
```

证券代码格式：`{6位代码}.{市场}`，沪市 `SH`，深市 `SZ`。

**按公告日期区间**（日期值用单引号）：

```
(SECUCODE="601088.SH")(NOTICE_DATE>='2026-01-01')(NOTICE_DATE<='2026-03-31')
```

运算符支持 `>=`、`<=`、`>`、`<`。适合作为业务日期过滤锚点的字段：

| 字段 | 说明 | 适用场景 |
|:-----|:-----|:---------|
| `NOTICE_DATE` | 公告披露日 | 日常增量抓取默认优先使用 |
| `END_DATE` | 股本变动截止日 | 按事件生效窗口补采 |
| `LISTING_DATE` | 上市流通日期 | 跟踪解禁/上市流通窗口 |

## Pagination

标准 URL 分页，通过 `pageNumber` / `pageSize` 控制。单只股票记录数差异大（6~62 条），全市场按年过滤时记录数可达数千条，必须处理多页结果。`sortColumns` 与 `filter` 绑定字段不必相同。

## cURL 示例

```bash
curl -s 'https://datacenter.eastmoney.com/securities/api/data/v1/get?\
reportName=RPT_F10_EH_EQUITY&\
columns=ALL&\
filter=(SECUCODE="601088.SH")&\
pageNumber=1&\
pageSize=500&\
sortTypes=-1&\
sortColumns=NOTICE_DATE&\
source=HSF10&\
client=PC'
```

## Sample Response

以 `SECUCODE="601088.SH"` 为例，第 1 条记录：

```json
{
  "SECUCODE": "601088.SH",
  "SECURITY_CODE": "601088",
  "SECURITY_NAME_ABBR": "中国神华",
  "ORG_CODE": "10032705",
  "END_DATE": "2026-04-07 00:00:00",
  "NOTICE_DATE": "2026-04-09 00:00:00",
  "LISTING_DATE": "2026-04-08 00:00:00",
  "CHANGE_REASON": "增发A股上市",
  "CHANGE_REASON_EXPLAIN": "非公开增发A股上市",
  "TOTAL_SHARES": 21689434304,
  "LIMITED_SHARES": 1820914349,
  "UNLIMITED_SHARES": 19868519955,
  "FREE_SHARES": 21689434304,
  "LISTED_A_SHARES": 16491037955,
  "B_FREE_SHARE": null,
  "H_FREE_SHARE": 3377482000,
  "OTHER_FREE_SHARES": null,
  "LIMITED_A_SHARES": 1820914349,
  "LIMITED_B_SHARES": null,
  "LIMITED_H_SHARES": null,
  "LIMITED_STATE_SHARES": 1363248446,
  "LIMITED_STATE_LEGAL": null,
  "LIMITED_DOMESTIC_NOSTATE": 423569794,
  "LIMITED_DOMESTIC_NATURAL": 13272311,
  "LIMITED_OVERSEAS_NOSTATE": 20823798,
  "LIMITED_OVERSEAS_NATURAL": null,
  "LIMITED_OTHARS": 436842105,
  "LIMITED_FOREIGN_SHARES": 20823798,
  "LOCK_SHARES": null,
  "NON_FREE_SHARES": null,
  "SPONSOR_SHARES": null,
  "STATE_SPONSOR_SHARES": null,
  "SPONSOR_SOCIAL_SHARES": null,
  "RAISE_SHARES": null,
  "RAISE_STATE_SHARES": null,
  "RAISE_DOMESTIC_SHARES": null,
  "RAISE_OVERSEAS_SHARES": null,
  "LIMITED_SHARES_RATIO": 8.395398070222,
  "LISTED_SHARES_RATIO": 91.604601929778,
  "LISTED_A_SHARES_RATIO": 76.032586760267,
  "H_FREE_SHARE_RATIO": 15.572015169511,
  "LISTED_A_RATIOPC": 83.000837467262,
  "LISTED_H_RATIOPC": 16.999162532738,
  "TOTAL_SHARES_CHANGE": 457665903,
  "LIMITED_SHARES_CHANGE": 457665903,
  "UNLIMITED_SHARES_CHANGE": 0,
  "IS_FREE_WINDOW": "1"
}
```

全量统计：`601088.SH` 共 8 条记录（1 页），`000001.SZ` 共 62 条记录（13 页）。

## 提取注意事项

- 无需认证，直接 GET 请求
- 日期字段格式统一为 `"YYYY-MM-DD HH:MM:SS"` 字符串
- 股数单位为"股"（非万股），数值通常很大
- 大量字段在不同股票、不同变动类型下为 `null`
- `TOTAL_SHARES = LIMITED_SHARES + UNLIMITED_SHARES`（当 `LIMITED_SHARES` 非 null 时）
- `FREE_SHARES` 通常等于 `TOTAL_SHARES`，并非实际自由流通股数
- 实际流通 A 股数应取 `LISTED_A_SHARES`
- `CHANGE_REASON` 常见值：`首发A股上市`、`首发H股上市`、`首发限售股份上市`、`网下配售股份上市`、`增发A股上市`、`回购`、`高管股份变动`、`定向增发`、`股权分置改革`、`配股上市`
