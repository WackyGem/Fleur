# eastmoney__freeholders

东方财富 F10 — 前十大流通股东

## Endpoint

```
GET https://datacenter.eastmoney.com/securities/api/data/v1/get
```

无需认证，直接 GET 请求。

## Query Parameters

| 参数 | 必填 | 说明 | 示例值 |
|:-----|:-----|:-----|:-------|
| `reportName` | 是 | 报表标识，固定值 | `RPT_F10_EH_FREEHOLDERS` |
| `columns` | 是 | 返回字段清单 | `SECUCODE,SECURITY_CODE,END_DATE,HOLDER_RANK,HOLDER_NEW,HOLDER_NAME,HOLDER_TYPE,SHARES_TYPE,HOLD_NUM,FREE_HOLDNUM_RATIO,HOLD_NUM_CHANGE,CHANGE_RATIO` |
| `filter` | 是 | DataCenter DSL 过滤条件 | `(SECUCODE="601088.SH")(END_DATE='2025-12-31')` |
| `pageNumber` | 是 | 页码，从 1 开始 | `1` |
| `pageSize` | 是 | 每页条数；scheduler 使用 `500` | `500` |
| `sortColumns` | 是 | 排序字段 | `END_DATE,HOLDER_RANK` |
| `sortTypes` | 是 | 排序方向：`-1` 降序，`1` 升序 | `-1,1` |
| `source` | 是 | 数据来源标识 | `HSF10` |
| `client` | 是 | 客户端标识 | `PC` |
| `quoteColumns` | 否 | 行情字段，一般留空 | - |
| `v` | 否 | 缓存破坏参数（时间戳数字） | `001928638127177329` |

用户提供的样例：

```text
https://datacenter.eastmoney.com/securities/api/data/v1/get?reportName=RPT_F10_EH_FREEHOLDERS&columns=SECUCODE%2CSECURITY_CODE%2CEND_DATE%2CHOLDER_RANK%2CHOLDER_NEW%2CHOLDER_NAME%2CHOLDER_TYPE%2CSHARES_TYPE%2CHOLD_NUM%2CFREE_HOLDNUM_RATIO%2CHOLD_NUM_CHANGE%2CCHANGE_RATIO&quoteColumns=&filter=(SECUCODE%3D%22601088.SH%22)(END_DATE%3D%272025-12-31%27)&pageNumber=1&pageSize=&sortTypes=1&sortColumns=HOLDER_RANK&source=HSF10&client=PC&v=001928638127177329
```

## Response

顶层 JSON：

```json
{
  "success": true,
  "message": "ok",
  "code": 0,
  "result": {
    "pages": 1,
    "count": 10,
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
| `result.data` | array | 前十大流通股东记录数组 |

### `result.data[]`

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `SECUCODE` | string | 证券代码（含市场后缀） | `"601088.SH"` |
| `SECURITY_CODE` | string | 证券代码（纯数字） | `"601088"` |
| `END_DATE` | string | 报告期截止日期 | `"2025-12-31 00:00:00"` |
| `HOLDER_RANK` | int | 股东排名 | `1` |
| `HOLDER_NEW` | string | 东方财富股东标识编码 | `"10066363"` |
| `HOLDER_NAME` | string | 股东名称 | `"国家能源投资集团有限责任公司"` |
| `HOLDER_TYPE` | string | 股东类型 | `"投资公司"` |
| `SHARES_TYPE` | string | 持股股份类别 | `"A股"` |
| `HOLD_NUM` | int | 持有流通股数量，单位为股 | `13812709196` |
| `FREE_HOLDNUM_RATIO` | float | 持有流通股比例，单位为百分比 | `69.520574392477` |
| `HOLD_NUM_CHANGE` | string | 较上期持股数量变动，可为数值文本或“不变” | `"不变"` |
| `CHANGE_RATIO` | float? | 较上期持股数量变动比例，单位为百分比 | `null` |

## Filter DSL

`filter` 是东方财富数据中心自带的查询表达式。多个条件直接拼接，无显式 AND/OR 运算符。

**按证券代码和单个报告期**：

```text
(SECUCODE="601088.SH")(END_DATE='2025-12-31')
```

**按证券代码和报告期区间**：

```text
(SECUCODE="601088.SH")(END_DATE>='2025-01-01')(END_DATE<='2025-12-31')
```

Scheduler 年分区资产使用 `END_DATE` 作为日期过滤字段：

- `partition_key=2025` -> `(END_DATE>='2025-01-01')(END_DATE<='2025-12-31')`
- `refresh_until_date=2025-06-30` -> `(END_DATE>='2025-01-01')(END_DATE<='2025-06-30')`

## Pagination

标准 URL 分页，通过 `pageNumber` / `pageSize` 控制。单只股票每个报告期通常返回 10
条记录，按年过滤会返回多个报告期，仍需处理多页结果。scheduler 使用 `pageSize=500`。

## Profiling Notes

- `columns=ALL` 会返回额外字段；当前数据集按用户提供的 12 个字段固化 contract。
- 样例 `601088.SH`、`600000.SH`、`000001.SZ`、`300750.SZ` 在 2024-2025 年样本中只有
  `CHANGE_RATIO` 出现 null。
- `HOLD_NUM_CHANGE` 虽表示数量变动，但接口以字符串返回，且会出现“不变”。
