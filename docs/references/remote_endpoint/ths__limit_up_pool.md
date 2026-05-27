# 同花顺 涨停板数据

## Endpoint

```
GET https://data.10jqka.com.cn/dataapi/limit_up/limit_up_pool
```

## 完整请求示例

```
https://data.10jqka.com.cn/dataapi/limit_up/limit_up_pool?page=1&limit=200&field=199112,10,9001,330323,330324,330325,9002,330329,133971,133970,1968584,3475914,9003,9004&filter=HS,GEM2STAR&order_field=330324&order_type=0&date=&_=1778299947223
```

## 请求参数

| 参数 | 必填 | 示例值 | 说明 |
|------|------|--------|------|
| page | 否 | `1` | 页码，默认从第 1 页开始 |
| limit | 否 | `200` | 每页记录数；服务端当前要求小于等于 `200` |
| field | 否 | `199112,10,...,9004` | 返回字段选择串（不透明数字标识列表） |
| filter | 否 | `HS,GEM2STAR` | 市场过滤条件 |
| order_field | 否 | `330324` | 排序字段代码 |
| order_type | 否 | `0` | 排序方向代码 |
| date | 是 | `20260429` | 日期，格式 `YYYYMMDD`。省略或为空则返回当天数据 |
| _ | 否 | `1778299947223` | 浏览器缓存破坏时间戳 |

## 响应结构

```json
{
    "status_code": 0,
    "status_msg": "success",
    "data": {
        "page": {
            "limit": 15,
            "total": 55,
            "count": 4,
            "page": 1
        },
        "date": "20260414",
        "msg": null,
        "trade_status": { ... },
        "info": [ ... ],
        "limit_up_count": { ... },
        "limit_down_count": { ... }
    }
}
```

### `data.page` — 分页信息

| 字段 | 说明 |
|------|------|
| `page` | 当前页码 |
| `limit` | 每页记录数 |
| `total` | 总记录数 |
| `count` | 当前页返回记录数 |

### `data.info[]` — 涨停个股明细

| 字段 | 类型 | 说明 |
|------|------|------|
| `code` | string | 股票代码 |
| `name` | string | 股票名称 |
| `latest` | float | 最新价 |
| `change_rate` | float | 涨跌幅 (%) |
| `turnover_rate` | float | 换手率 (%) |
| `order_volume` | float | 封单量 (手) |
| `order_amount` | float | 封单金额 (元) |
| `currency_value` | int | 流通市值 (元) |
| `limit_up_type` | string | 涨停类型（如 "换手板"） |
| `high_days` | string | 连板天数描述（如 "首板"） |
| `high_days_value` | int | 连板天数数值 |
| `open_num` | int | 开板次数 |
| `first_limit_up_time` | string | 首次涨停时间 (Unix timestamp) |
| `last_limit_up_time` | string | 最后涨停时间 (Unix timestamp) |
| `limit_up_suc_rate` | float | 涨停成功率 |
| `is_again_limit` | int | 是否回封 (1=是) |
| `is_new` | int | 是否新股 (1=是) |
| `reason_type` | string | 涨停原因标签 |
| `change_tag` | string | 变动标签（如 "LIMIT_BACK"） |
| `market_id` | int | 市场 ID |
| `market_type` | string | 市场类型（"HS"=沪深） |

### `data.limit_up_count` — 涨停统计

| 字段 | 说明 |
|------|------|
| `today.num` | 今日涨停数 |
| `today.history_num` | 今日历史涨停数 |
| `today.rate` | 今日涨停比率 |
| `today.open_num` | 今日开板数 |
| `yesterday.*` | 昨日对应统计 |

### `data.limit_down_count` — 跌停统计

结构同 `limit_up_count`，字段含义对应跌停数据。

### `data.trade_status` — 交易状态

| 字段 | 说明 |
|------|------|
| `id` | 状态标识（"no_open"=未开盘） |
| `name` | 状态名称 |
| `start_time` / `end_time` | 状态时段 |

## 测试发现

### 数据保留期限

服务端最大保留 **380 天**数据。超出后返回：

```json
{
    "status_code": -1,
    "status_msg": "date参数不合法"
}
```

例如今天 2026-05-18，最早可查询 2025-05-03（380天前）。

### 响应结构细节

- `page.count` = **总页数**（不是当前页记录数）。当前页实际记录数 = `len(data.info[])`
- 非交易日（周末/节假日）返回 `status_code: 0` 但 `info: []`, `total: 0`，同时返回上一交易日的 `yesterday` 统计
- 未来日期返回 `status_code: 0` 但 `total: 0`

### 有效 filter 值

| filter | 说明 | 示例 total |
|--------|------|-----------|
| `HS` | 沪深主板 | 46 |
| `GEM2STAR` | 创业板+科创板 | 8 |
| `HS,GEM2STAR` | 全部 | 54 |

注意：`SH`、`SZ`、`BJ` **不是**有效 filter 值。

### 请求要求

必须携带浏览器 headers（`User-Agent`、`Referer`），否则返回 `403 Forbidden`。
