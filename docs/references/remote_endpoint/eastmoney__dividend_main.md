# eastmoney_dividend_main

东方财富 F10 — 上市公司分红方案明细

## Endpoint

```
GET https://datacenter.eastmoney.com/securities/api/data/v1/get
```

无需认证，直接 GET 请求。

## Query Parameters

| 参数 | 必填 | 说明 | 示例值 |
|:-----|:-----|:-----|:-------|
| `reportName` | 是 | 报表标识，固定值 | `RPT_F10_DIVIDEND_MAIN` |
| `columns` | 是 | 返回字段，`ALL` = 全部 | `ALL` |
| `filter` | 是 | DataCenter DSL 过滤条件 | `(SECUCODE="601088.SH")` |
| `pageNumber` | 是 | 页码，从 1 开始 | `1` |
| `pageSize` | 是 | 每页条数 | `500` |
| `sortColumns` | 是 | 排序字段 | `NOTICE_DATE` |
| `sortTypes` | 是 | 排序方向：`-1` 降序，`1` 升序 | `-1` |
| `source` | 是 | 数据来源标识 | `HSF10` |
| `client` | 是 | 客户端标识 | `PC` |
| `quoteColumns` | 否 | 行情字段，一般留空 | — |
| `v` | 否 | 缓存破坏参数（时间戳数字） | `0876545049357239` |

## Response

顶层 JSON：

```json
{
  "success": true,
  "message": "ok",
  "code": 0,
  "version": "ebdb9677c8e...",
  "result": {
    "pages": 4,
    "count": 38,
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
| `result.data` | array | 分红记录数组 |

### 分红记录字段

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `SECUCODE` | string | 证券代码（含市场后缀） | `"601088.SH"` |
| `SECURITY_CODE` | string | 证券代码（纯数字） | `"601088"` |
| `SECURITY_NAME_ABBR` | string | 证券简称 | `"中国神华"` |
| `NOTICE_DATE` | string | 公告日期 | `"2026-03-31 00:00:00"` |
| `REPORT_DATE` | string | 报告期 | `"2025年报"` |
| `REPORT_TIME` | string? | 报告期截止日 | `"2025-12-31 00:00:00"` |
| `IMPL_PLAN_PROFILE` | string | 分红方案简述 | `"10派10.3元"` |
| `NEW_PROFILE` | string | 分红方案（含税） | `"10派10.3元(含税)"` |
| `IMPL_PLAN_NEWPROFILE` | string | 方案简介 + 进度后缀 | `"10派10.3元(实施方案)"` |
| `ASSIGN_PROGRESS` | string | 分配进度 | `"董事会预案"` / `"实施方案"` |
| `ASSIGN_OBJECT` | string? | 分配对象 | `"全体股东"` |
| `IS_UNASSIGN` | string | 是否不分配：`"0"` 否，`"1"` 是 | `"0"` |
| `EQUITY_RECORD_DATE` | string? | 股权登记日 | `"2025-11-07 00:00:00"` |
| `EX_DIVIDEND_DATE` | string? | 除权除息日 | `"2025-11-10 00:00:00"` |
| `PAY_CASH_DATE` | string? | 派息日 | `"2025-11-10 00:00:00"` |
| `GMDECISION_NOTICE_DATE` | string? | 股东大会决议公告日 | `"2025-10-25 00:00:00"` |
| `INFO_CODE` | string? | 公告编号 | `"AN202603301820880241"` |
| `DAT_YAGGR` | string? | 年度股东大会日期 | `"2026-03-31 00:00:00"` |
| `TOTAL_DIVIDEND` | float | 分红总额（元） | `19471149555.9` |
| `TOTAL_DIVIDEND_A` | float | A股分红总额（元） | `16161217195.9` |

标记位字段（可忽略）：`DAT_YAGGR_TODAY`、`NOTICE_TODAY`、`GMDECISION_TODAY`、`DIRECTORSUPERVISOR_TODAY`、`EQUITY_TODAY`、`EX_DIVIDEND_TODAY`、`PAYCASH_TODAY`、`IS_PAYCASH`、`IS_EQUITY_RECENT`、`LAST_TRADE_DATE`。

## Filter DSL

`filter` 是东方财富数据中心自带的查询表达式。多个条件直接拼接，无显式 AND/OR 运算符。

**按证券代码**：

```
(SECUCODE="601088.SH")
```

证券代码格式：`{6位代码}.{市场}`，沪市 `SH`，深市 `SZ`。

**按公告日期区间**（日期值用单引号）：

```
(SECUCODE="601088.SH")(NOTICE_DATE>='2025-10-01')(NOTICE_DATE<='2025-12-31')
```

运算符支持 `>=`、`<=`、`>`、`<`。支持的日期字段：`NOTICE_DATE`、`EQUITY_RECORD_DATE`、`EX_DIVIDEND_DATE`、`PAY_CASH_DATE`、`GMDECISION_NOTICE_DATE`。

## Pagination

标准 URL 分页，通过 `pageNumber` / `pageSize` 控制。`sortColumns` 与 `filter` 绑定字段不必相同。

## cURL 示例

```bash
curl -s 'https://datacenter.eastmoney.com/securities/api/data/v1/get?\
reportName=RPT_F10_DIVIDEND_MAIN&\
columns=ALL&\
filter=(SECUCODE="601088.SH")&\
pageNumber=1&\
pageSize=2&\
sortTypes=-1&\
sortColumns=NOTICE_DATE&\
source=HSF10&\
client=PC'
```

## Sample Response

`601088.SH` 第 1 页第 1 条（`pageSize=10` 时共 38 条 / 4 页）：

```json
{
  "SECUCODE": "601088.SH",
  "SECURITY_CODE": "601088",
  "SECURITY_NAME_ABBR": "中国神华",
  "NOTICE_DATE": "2026-03-31 00:00:00",
  "IMPL_PLAN_PROFILE": "10派10.3元",
  "ASSIGN_PROGRESS": "董事会预案",
  "EQUITY_RECORD_DATE": null,
  "EX_DIVIDEND_DATE": null,
  "PAY_CASH_DATE": null,
  "IS_UNASSIGN": "0",
  "REPORT_DATE": "2025年报",
  "ASSIGN_OBJECT": "全体股东",
  "IMPL_PLAN_NEWPROFILE": "10派10.3元",
  "NEW_PROFILE": "10派10.3元(含税)",
  "GMDECISION_NOTICE_DATE": null,
  "INFO_CODE": "AN202603301820880241",
  "DAT_YAGGR": "2026-03-31 00:00:00",
  "TOTAL_DIVIDEND": 19471149555.9,
  "TOTAL_DIVIDEND_A": 16161217195.9,
  "REPORT_TIME": "2025-12-31 00:00:00"
}
```
