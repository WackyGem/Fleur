# eastmoney_dividend_allotment

东方财富 F10 — 上市公司配股明细

## Endpoint

```
GET https://datacenter.eastmoney.com/securities/api/data/v1/get
```

无需认证，直接 GET 请求。

## Query Parameters

| 参数 | 必填 | 说明 | 示例值 |
|:-----|:-----|:-----|:-------|
| `reportName` | 是 | 报表标识，固定值 | `RPT_F10_DIVIDEND_ALLOTMENT` |
| `columns` | 是 | 返回字段，`ALL` = 全部 | `ALL` |
| `filter` | 是 | DataCenter DSL 过滤条件 | `(SECUCODE="000001.SZ")` |
| `pageNumber` | 是 | 页码，从 1 开始 | `1` |
| `pageSize` | 是 | 每页条数 | `10` |
| `sortColumns` | 是 | 排序字段 | `NOTICE_DATE` |
| `sortTypes` | 是 | 排序方向：`-1` 降序，`1` 升序 | `-1` |
| `source` | 是 | 数据来源标识 | `HSF10` |
| `client` | 是 | 客户端标识 | `PC` |
| `quoteColumns` | 否 | 行情字段，一般留空 | — |
| `v` | 否 | 缓存破坏参数（时间戳数字） | `06322991966673641` |

## Response

顶层 JSON：

```json
{
  "success": true,
  "message": "ok",
  "code": 0,
  "result": {
    "pages": 1,
    "count": 3,
    "data": [ ... ]
  }
}
```

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `success` | bool | 请求是否成功 |
| `code` | int | 0 = 正常；`9201` + `result=null` 表示空响应（无配股历史） |
| `result.pages` | int | 总页数 |
| `result.count` | int | 总记录数 |
| `result.data` | array | 配股记录数组 |

空响应示例（无配股历史）：

```json
{
  "success": false,
  "message": "返回数据为空",
  "code": 9201,
  "result": null
}
```

`code=9201` 表示 DataCenter 空响应，SDK 返回空 records，非请求错误。

### 配股记录字段

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `SECUCODE` | string | 证券代码（含市场后缀） | `"000001.SZ"` |
| `SECURITY_CODE` | string | 证券代码（纯数字） | `"000001"` |
| `SECURITY_NAME_ABBR` | string | 证券简称 | `"平安银行"` |
| `NOTICE_DATE` | string | 公告日期 | `"2000-10-21 00:00:00"` |
| `ISSUE_NUM` | int | 配股数量（股） | `393975057` |
| `TOTAL_RAISE_FUNDS` | float | 募集资金总额（元） | `3151800456` |
| `ISSUE_PRICE` | float | 配股价格（元/股） | `8` |
| `EQUITY_RECORD_DATE` | string | 股权登记日 | `"2000-11-03 00:00:00"` |
| `EX_DIVIDEND_DATEE` | string | 除权日（注意字段名末尾双 E） | `"2000-11-06 00:00:00"` |
| `EVENT_EXPLAIN` | string | 配股方案说明 | `"每10股配3股"` |

注意：`EX_DIVIDEND_DATEE` 字段名末尾有两个 `E`，为接口原始命名，非拼写错误。

## Filter DSL

`filter` 是东方财富数据中心自带的查询表达式。多个条件直接拼接，无显式 AND/OR 运算符。

**按证券代码**：

```
(SECUCODE="000001.SZ")
```

证券代码格式：`{6位代码}.{市场}`，沪市 `SH`，深市 `SZ`。

**按公告日期区间**（日期值用单引号）：

```
(SECUCODE="000001.SZ")(NOTICE_DATE>='2026-01-01')(NOTICE_DATE<='2026-01-31')
```

运算符支持 `>=`、`<=`、`>`、`<`。适合作为业务日期过滤锚点的字段：

| 字段 | 说明 | 适用场景 |
|:-----|:-----|:---------|
| `NOTICE_DATE` | 公告披露日 | 日常增量抓取默认优先使用 |
| `EQUITY_RECORD_DATE` | 股权登记日 | 按权益登记窗口补采 |
| `EX_DIVIDEND_DATEE` | 除权日 | 按实施窗口补采 |

即使配股接口常见结果集较小，也不应假定永远只有单页结果。

## Pagination

标准 URL 分页，通过 `pageNumber` / `pageSize` 控制。配股记录通常较少（多数股票 0~3 条），但在更宽过滤窗口下仍可能命中多页结果，如需取全量结果应结合 `result.pages` 继续请求后续页面。

SDK `pagination_mode="all"`（默认）从 `page_number` 指定页开始继续翻页并聚合；`pagination_mode="single"` 只请求指定的一页。

## SDK 调用

```python
await client.eastmoney.dividend_allotment(
    code="sz.000001",
    start_date="2026-01-01",
    end_date="2026-01-31",
    page_number=1,
    page_size=50,
    pagination_mode="single",
)
```

常规 facade 使用 `code` / `start_date` / `end_date` 生成 `filter`，不再接受 `secucode` 或 `filter` 参数。低层 `client.call("eastmoney__dividend_allotment", params={"filter": "..."})` 仍可用于手写 DataCenter DSL。

## cURL 示例

```bash
curl -s 'https://datacenter.eastmoney.com/securities/api/data/v1/get?\
reportName=RPT_F10_DIVIDEND_ALLOTMENT&\
columns=ALL&\
filter=(SECUCODE="000001.SZ")&\
pageNumber=1&\
pageSize=10&\
sortTypes=-1&\
sortColumns=NOTICE_DATE&\
source=HSF10&\
client=PC'
```

## Sample Response

以 `SECUCODE="000001.SZ"`（平安银行）为例，全部 3 条记录：

```json
[
  {
    "SECUCODE": "000001.SZ",
    "SECURITY_CODE": "000001",
    "SECURITY_NAME_ABBR": "平安银行",
    "NOTICE_DATE": "2000-10-21 00:00:00",
    "ISSUE_NUM": 393975057,
    "TOTAL_RAISE_FUNDS": 3151800456,
    "ISSUE_PRICE": 8,
    "EQUITY_RECORD_DATE": "2000-11-03 00:00:00",
    "EX_DIVIDEND_DATEE": "2000-11-06 00:00:00",
    "EVENT_EXPLAIN": "每10股配3股"
  },
  {
    "SECUCODE": "000001.SZ",
    "SECURITY_CODE": "000001",
    "SECURITY_NAME_ABBR": "平安银行",
    "NOTICE_DATE": "1994-01-09 00:00:00",
    "ISSUE_NUM": 26941789,
    "TOTAL_RAISE_FUNDS": 134708945,
    "ISSUE_PRICE": 5,
    "EQUITY_RECORD_DATE": "1994-07-08 00:00:00",
    "EX_DIVIDEND_DATEE": "1994-07-09 00:00:00",
    "EVENT_EXPLAIN": "每10股配1股"
  },
  {
    "SECUCODE": "000001.SZ",
    "SECURITY_CODE": "000001",
    "SECURITY_NAME_ABBR": "平安银行",
    "NOTICE_DATE": "1993-05-07 00:00:00",
    "ISSUE_NUM": 20205000,
    "TOTAL_RAISE_FUNDS": 323280000,
    "ISSUE_PRICE": 16,
    "EQUITY_RECORD_DATE": "1993-05-21 00:00:00",
    "EX_DIVIDEND_DATEE": "1993-05-24 00:00:00",
    "EVENT_EXPLAIN": "每10股配1股"
  }
]
```

以 `SECUCODE="600030.SH"`（中信证券）为例，1 条记录：

```json
{
  "SECUCODE": "600030.SH",
  "SECURITY_CODE": "600030",
  "SECURITY_NAME_ABBR": "中信证券",
  "NOTICE_DATE": "2022-01-14 00:00:00",
  "ISSUE_NUM": 1552021645,
  "TOTAL_RAISE_FUNDS": 22395672337.35,
  "ISSUE_PRICE": 14.43,
  "EQUITY_RECORD_DATE": "2022-01-18 00:00:00",
  "EX_DIVIDEND_DATEE": "2022-01-27 00:00:00",
  "EVENT_EXPLAIN": "每10股配1.5股"
}
```

无配股记录示例（`SECUCODE="601088.SH"`）：

```json
{
  "success": false,
  "message": "返回数据为空",
  "code": 9201,
  "result": null
}
```
