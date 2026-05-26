# jiuyan — 异动板块接口 (`action_field`)

> 韭研公社 APP 端异动板块列表，按题材分类展示当日异动股票及解析。

## 基本信息

| 项目 | 值 |
|:-----|:---|
| API | `jiuyangongshe_action_field` |
| URL | `POST https://app.jiuyangongshe.com/jystock-app/api/v1/action/field` |
| Content-Type | `application/json` |
| 限速 | 无 |
| 配额 | 每 SESSION 每日 80 个交易日请求 |

## 请求头

所有 4 个必填头缺少任一则返回 `errCode: "9"`（版本过低提示）。

| Header | 必填 | 示例值 | 说明 |
|:-------|:-----|:-------|:-----|
| `token` | Y | `<your_token>` | APP 级别密钥，从环境变量 `JIUYAN_TOKEN` 读取 |
| `cookie` | Y | `SESSION=<your_session_id>` | 服务端 SESSION ID，从环境变量 `JIUYAN_COOKIE` 读取 |
| `platform` | Y | `3` | 平台标识（3 = Android） |
| `timestamp` | Y | `1778309697000` | 当前 Unix 毫秒时间戳，请求发送时动态计算，例如 `str(int(time.time() * 1000))`；不要写死固定值 |
| `User-Agent` | N | `Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 ...` | 标准 Chrome UA |

## 请求体

```json
{
  "pc": "1",
  "date": "2026-05-08"
}
```

| 参数 | 必填 | 类型 | 说明 |
|:-----|:-----|:-----|:-----|
| `pc` | Y | `string` | `"1"` — 返回完整数据（含 `list` 股票明细）；`"2"` — 仅返回板块摘要（名称 + count，额外含 `全部` / `一字板` 聚合行） |
| `date` | Y | `string` | 查询日期，格式 `YYYY-MM-DD` |

## 响应结构

### 顶层

```json
{
  "errCode": "0",
  "msg": "",
  "serverTime": 1778309697,
  "data": [ ... ]
}
```

### 板块对象 (`data[]`)

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `action_field_id` | `string` | 板块唯一 ID；`"简图"` 行为空字符串；聚合行为 `"all,{date}"` / `"recommend,{date}"` |
| `name` | `string` | 板块名称（如 `"机器人"`、`"光通信"`、`"ST板块"`） |
| `date` | `string` | 日期 `YYYY-MM-DD` |
| `reason` | `string` | 板块题材摘要（板块级，非个股级）；普通板块可能为空 |
| `status` | `int` | 状态标志（通常为 `0`） |
| `sort_no` | `int` | 排序序号 |
| `is_delete` | `string` | `"0"` = 未删除 |
| `delete_time` | `string\|null` | 删除时间 |
| `create_time` | `string` | 创建时间 `YYYY-MM-DD HH:mm:ss` |
| `update_time` | `string\|null` | 更新时间 |
| `count` | `int` | 该板块下异动股票数量 |

> **注意**：`"简图"` 行结构精简，无 `status`/`sort_no`/`list` 等字段。

### 股票对象 (`data[].list[]`，仅 `pc="1"` 时存在)

```json
{
  "code": "sh603045",
  "name": "福达合金",
  "article": { ... }
}
```

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `code` | `string` | 带市场前缀的股票代码（`sh` = 上海, `sz` = 深圳） |
| `name` | `string` | 股票简称 |
| `article` | `object` | 异动解析文章（见下表） |

### 文章对象 (`article`)

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `article_id` | `string` | 文章唯一 ID |
| `title` | `string` | 异动解析标题（如 `"05月08日福达合金股票异动解析"`） |
| `create_time` | `string` | 文章创建时间 |
| `comment_count` | `int` | 评论数 |
| `like_count` | `int` | 点赞数 |
| `forward_count` | `int` | 转发数 |
| `step_count` | `int` | 收藏数 |
| `is_like` | `int` | 当前用户是否已点赞（`0`/`1`） |
| `is_step` | `int` | 当前用户是否已收藏（`0`/`1`） |
| `user_id` | `string` | 作者用户 ID |
| `action_info` | `object` | 异动详情（见下表） |
| `user` | `object` | 作者信息（`user_id`, `avatar`, `nickname`） |

### 异动详情 (`action_info`)

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `article_id` | `string` | 关联文章 ID |
| `action_info_id` | `string` | 异动记录唯一 ID |
| `stock_id` | `string` | 股票唯一 ID |
| `action_field_id` | `string` | 所属板块 ID |
| `time` | `string` | 异动发生时间 `HH:mm:ss` |
| `num` | `string\|null` | 连板描述（如 `"4天4板"`、`"3天2板"`）；非连板股为 `null` |
| `price` | `int` | 价格，**单位：分**（如 `6214` = 62.14 元） |
| `day` | `int\|null` | 连板天数；非连板股为 `null` |
| `edition` | `int\|null` | 当日板数；非连板股为 `null` |
| `shares_range` | `float` | 涨跌幅，**单位：‱（万分之一）**（如 `1000.0` = 10.00%） |
| `reason` | `string\|null` | 个股异动原因（通常为 `null`，详细内容在 `expound`） |
| `expound` | `string` | 异动原因详细说明（多行文本，含题材标签 + 公告/新闻摘要） |
| `is_crawl` | `int` | 是否为爬取内容（`1` = 是） |
| `is_recommend` | `int` | 是否为推荐/一字板（`0`/`1`） |
| `is_delete` | `string` | `"0"` = 未删除 |
| `delete_time` | `string\|null` | 删除时间 |
| `create_time` | `string` | 创建时间 |
| `update_time` | `string\|null` | 更新时间 |
| `sort_no` | `int` | 排序序号 |

## 响应示例

### `pc="1"` — 板块内股票明细

```json
{
  "errCode": "0",
  "msg": "",
  "serverTime": 1778309697,
  "data": [
    {
      "action_field_id": "",
      "name": "简图",
      "date": "2026-05-08",
      "reason": "",
      "count": 0
    },
    {
      "action_field_id": "215967a625da4e05a8e4ef36537facf3",
      "name": "公告",
      "date": "2026-05-08",
      "reason": "",
      "status": 0,
      "sort_no": 0,
      "is_delete": "0",
      "delete_time": null,
      "create_time": "2026-05-08 12:10:12",
      "update_time": null,
      "count": 3,
      "list": [
        {
          "code": "sh603045",
          "name": "福达合金",
          "article": {
            "article_id": "3fjp47rlbvm",
            "title": "05月08日福达合金股票异动解析",
            "create_time": "2026-05-08 12:10:12",
            "comment_count": 1,
            "like_count": 18,
            "forward_count": 4,
            "step_count": 4,
            "is_like": 0,
            "is_step": 0,
            "user_id": "1",
            "action_info": {
              "article_id": "3fjp47rlbvm",
              "action_info_id": "b93f97ddd1864824af07cda0fbdfa8d5",
              "stock_id": "7834dd65b3f04f9f97b584373b91487e",
              "action_field_id": "215967a625da4e05a8e4ef36537facf3",
              "time": "09:25:00",
              "num": "4天4板",
              "price": 6214,
              "day": 4,
              "edition": 4,
              "shares_range": 1000.0,
              "reason": null,
              "expound": "扭亏为盈+数据中心电接触材料+机器人+金属回收\n1、2026年4月29日晚发布一季报...",
              "is_crawl": 1,
              "is_recommend": 1,
              "is_delete": "0",
              "delete_time": null,
              "create_time": "2026-05-08 15:43:38",
              "update_time": null,
              "sort_no": 0
            },
            "user": {
              "user_id": "1",
              "avatar": "https://cdn.jiuyangongshe.com/merchant/16619278755407b23a6a66801033e84a591293766f4bc.png",
              "nickname": "韭菜团子"
            }
          }
        }
      ]
    }
  ]
}
```

### `pc="2"` — 板块摘要（无 `list`）

```json
{
  "errCode": "0",
  "msg": "",
  "serverTime": 1778309697,
  "data": [
    {
      "action_field_id": "all,2026-05-08",
      "name": "全部",
      "date": "2026-05-08",
      "reason": "内容全由人工编写...",
      "count": 138
    },
    {
      "action_field_id": "recommend,2026-05-08",
      "name": "一字板",
      "date": "2026-05-08",
      "reason": "该栏仅为复盘详细解析...",
      "count": 4
    },
    {
      "action_field_id": "215967a625da4e05a8e4ef36537facf3",
      "name": "公告",
      "date": "2026-05-08",
      "reason": "",
      "status": 0,
      "sort_no": 0,
      "is_delete": "0",
      "delete_time": null,
      "create_time": "2026-05-08 12:10:12",
      "update_time": null,
      "count": 3
    }
  ]
}
```

## `pc=2` 聚合板块

`pc="2"` 在常规板块之前额外插入两行聚合数据：

| name | action_field_id 格式 | 说明 |
|:-----|:---------------------|:-----|
| `全部` | `all,{date}` | 当日所有异动股票总数 |
| `一字板` | `recommend,{date}` | 仅推荐/一字板股票数 |

## 单位换算

| 字段 | 原始值 | 换算 | 结果 |
|:-----|:-------|:-----|:-----|
| `price` | `6214` | ÷ 100 | 62.14 元 |
| `shares_range` | `1000.0` | ÷ 100 | 10.00% |

## 错误处理

| errCode | msg | 原因 |
|:--------|:----|:-----|
| `"0"` | `""` | 成功 |
| `"9"` | `"因您的版本过低..."` | 缺少 `token`/`cookie`/`platform`/`timestamp` 任一必填头部 |
| `"0"` + data 仅含 `简图`（count=0） | — | 非交易日或该日无异动数据 |

## 注意事项

- 板块列表**动态变化**，不同日期的板块名称、数量可能不同
- `reason` 为板块级题材摘要；`expound` 为个股级详细异动解析（多行文本）
- `num`/`day`/`edition` 在非连板股中为 `null`
- token 和 SESSION cookie 可能过期，需定期更新
- `简图` 行为固定占位行，`action_field_id` 为空，无股票列表
