# jiuyan — 产业研究接口 (`industry_list`)

> 韭研公社 APP 端产业研究列表，按关键词或全量分页获取行业研究报告。

## 基本信息

| 项目 | 值 |
|:-----|:---|
| API | `jiuyangongshe_industry_list` |
| URL | `POST https://app.jiuyangongshe.com/jystock-app/api/v1/industry/list` |
| Content-Type | `application/json` |
| 限速 | 无 |
| 配额 | 无限制 |

## 请求头

同 [action/field](jiuyan__action_field.md)，需 `token` + `cookie` + `platform` + `timestamp` 头部。`timestamp` 必须在每次请求发送时动态计算为当前 Unix 毫秒时间戳，例如 Python 中使用 `str(int(time.time() * 1000))`；`token` 使用同一个 `timestamp` 按 `md5("Uu0KfOB8iUP69d3c:{timestamp_ms}")` 动态计算；不要写死复测用的固定值。

## 请求体

```json
{
  "keyword": "光通信",
  "start": "0",
  "limit": "500"
}
```

| 参数 | 必填 | 类型 | 说明 |
|:-----|:-----|:-----|:-----|
| `keyword` | 否 | string | 关键词搜索（多字段模糊匹配：标题、正文、关键词），空字符串或不传返回全部 |
| `start` | 是 | string | 页码参数。`"0"` 与 `"1"` 都返回第 1 页；第 2 页需使用响应 `nextPage` 返回的 `"2"` |
| `limit` | 是 | string | 每页条数，建议 `"500"` |

## 响应结构

### 顶层

```json
{
  "errCode": "0",
  "msg": "",
  "serverTime": 1778363882,
  "data": {
    "pageNo": 1,
    "pageSize": 500,
    "totalCount": 501,
    "totalPages": 2,
    "hasNext": true,
    "nextPage": 2,
    "hasPre": false,
    "prePage": 1,
    "first": 1,
    "orderBy": null,
    "order": null,
    "autoCount": true,
    "map": null,
    "params": "",
    "result": [ ... ]
  }
}
```

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `errCode` | string | `"0"` = 成功 |
| `data.pageNo` | int | 当前页码（服务端页码；`start=0` 和 `start=1` 都返回 `pageNo=1`） |
| `data.pageSize` | int | 每页条数（与请求 `limit` 一致） |
| `data.totalCount` | int | 见下方「totalCount / totalPages 行为」 |
| `data.totalPages` | int | 不可靠，随 `start`/`limit` 变化；翻页不要依赖该字段 |
| `data.hasNext` | bool | 是否有下一页（**翻页唯一可靠依据**） |
| `data.nextPage` | int | 下一页页码 |
| `data.hasPre` | bool | 是否有上一页 |
| `data.prePage` | int | 上一页页码 |
| `data.first` | int | 首页页码（始终为 `1`） |
| `data.orderBy` | null | 未使用（始终为 `null`） |
| `data.order` | null | 未使用（始终为 `null`） |
| `data.autoCount` | bool | 服务端是否自动计算总数（始终为 `true`） |
| `data.map` | null | 未使用（始终为 `null`） |
| `data.params` | string | 未使用（始终为空字符串） |
| `data.result` | array | 产业研究条目数组 |

### 产业研究对象 (`data.result[]`)

```json
{
  "industry_id": "9851d9c36c4f4933bd2d01ff3f8cc1fa",
  "title": "光芯片(260422)",
  "title_red": 1,
  "title_bold": 0,
  "author": null,
  "imgs": "[\"https://jiucaigongshe.oss-cn-beijing.aliyuncs.com/import/xxx.png\"]",
  "keyword": "源杰科技  长光华芯  仕佳光子  永鼎股份  ...",
  "content": "事件：\n1、2026年4月22日盘后券商观点...",
  "is_top": 1,
  "status": 0,
  "sort_no": 6,
  "forward_count": 1,
  "browsers_count": 2028,
  "is_delete": "0",
  "delete_time": null,
  "create_time": "2026-04-22 23:09:06",
  "update_time": "2026-04-23 09:14:18"
}
```

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `industry_id` | string | 产业研究唯一 ID |
| `title` | string | 标题，格式 `题材名(YYMMDD)` 或 `题材名(YYMMDD)更新` |
| `keyword` | string | 关联股票/关键词，制表符 `\t` 分隔 |
| `content` | string | 正文（多行纯文本） |
| `imgs` | string | 图片 URL 数组的 JSON 字符串，需二次解析 |
| `is_top` | int | 是否置顶（`1` = 置顶，排在最前） |
| `sort_no` | int | 排序序号 |
| `forward_count` | int | 转发数 |
| `browsers_count` | int | 浏览数 |
| `create_time` | string | 首次发布时间 `YYYY-MM-DD HH:mm:ss` |
| `update_time` | string \| null | 最后更新时间 |

## 分页

- **全量**: ~949 条产业研究（截至 2026-05-09），日期范围 2024-03-16 ~ 2026-05-09
- **`start` 参数**: 不是严格 0-indexed；`start="0"` 和 `start="1"` 都返回第 1 页，`start="2"` 返回第 2 页
- **实际状态机**: 第一次请求固定使用 `start="0"`；若当前响应 `hasNext=true`，下一次请求的 `start` 使用当前响应 `nextPage` 的字符串值；若当前响应 `hasNext=false`，停止翻页
- **禁止**: 不要用本地页码、自增计数器或 `start + 1` 推导下一页
- **建议**: `limit=500`，循环至 `hasNext=false`

SDK `pagination_mode="all"`（默认）从 `start` 开始按 `nextPage` 继续翻页并聚合；`pagination_mode="single"` 只请求指定的一页。

### 2026-05-19 curl 复测记录

- 使用本地 `.env` 中 `JIUYAN_COOKIE`，并按有效请求要求携带测试用固定 `timestamp=1779217286540` 和同 timestamp 动态计算的 `token`，对 `start=0/1/2/3, limit=500` 发起请求；生产实现必须动态计算当前 Unix 毫秒时间戳和 token。
- `start=0` 返回 `pageNo=1`、`result_len=500`、`totalCount=501`、`totalPages=2`、`hasNext=true`、`nextPage=2`。
- `start=1` 返回结果与 `start=0` 相同，确认 `start=0` 和 `start=1` 都映射到第 1 页。
- `start=2` 返回 `pageNo=2`、`result_len=453`、`totalCount=953`、`totalPages=2`、`hasNext=false`、`nextPage=2`。
- `start=3` 返回 `pageNo=3`、`result_len=0`、`totalCount=1000`、`totalPages=2`、`hasNext=false`、`nextPage=3`，说明越界页可能返回空结果且计数字段不可靠。
- 实测确认：全量抓取应请求 `start=0`，之后仅在 `hasNext=true` 时使用当前响应 `nextPage` 继续；不得预生成页码或依赖 `totalCount` / `totalPages`。

### totalCount / totalPages 行为

`totalCount` 和 `totalPages` **不可靠**，具体表现：

| 请求 | 实际返回条数 | totalCount | 说明 |
|:-----|:-----------|:-----------|:-----|
| `start=0, limit=5` | 5 | 6 | 偏大 1 |
| `start=0, limit=200` | 200 | 201 | 偏大 1 |
| `start=0, limit=500` | 500 | 501 | 偏大 1 |
| `start=0, limit=1000` | 949 | 949 | 准确 |
| `start=10, limit=5` | 5 | 51 | 偏大很多 |
| `start=100, limit=5` | 5 | 501 | 偏大很多 |
| `start=200, limit=200` | 0 | 39800 | 越界时严重膨胀 |

**规律**: `totalCount ≈ pageNo × limit + 1`，与实际数据量无关；`totalPages` 也会随请求页变化。翻页时**必须以 `hasNext=false` 为准**，不要依赖 `totalCount` 或 `totalPages`。

### keyword 搜索行为

`keyword` 参数执行**多字段模糊匹配**，同时搜索：
- `title`（标题）
- `content`（正文）
- `keyword`（关联股票/关键词字段）

示例：搜索 `"光通信"` 命中标题含"光通信"、正文中提及"光通信"、或 keyword 字段含"光通信"的所有条目。实测 `"光通信"` 当前返回 5 条，`totalCount=5`，`hasNext=false`；若其他关键词命中超过 `limit`，仍应按 `hasNext` / `nextPage` 翻页。

## SDK 调用

```python
# 全量获取（推荐）
await client.jiuyan.industry_list(
    keyword="",
    start=0,
    limit=500,
    pagination_mode="all",
)

# 按关键词搜索（不分页，直接返回全部命中）
await client.jiuyan.industry_list(
    keyword="光通信",
    start=0,
    limit=500,
    pagination_mode="single",
)
```

## 注意事项

- 标题日期格式 `YYMMDD`（如 `260422` = 2026-04-22），非 YYYY-MM-DD
- `keyword`（请求参数）搜索标题+正文+关联股票字段；`keyword`（响应字段）为制表符分隔的股票名称/概念关键词
- `imgs` 是 JSON 字符串而非数组，需二次解析
- 当前 raw parquet 实测显示 `imgs_json` 虽然整体可 `json.loads`，但不能假设每个数组元素只含一个 URL；样本 `eff11de7fb5b41518d53dfc36aea39e6` 的数组内单个字符串用英文逗号拼接了 6 个图片 URL。
- 图片 URL 可能带 `x-oss-process` query 参数，且 query 内本身包含英文逗号；解析图片 URL 时不要对整个字段简单按逗号切分，应先 JSON parse，再对每个候选字符串用 URL 正则提取。
- 产业研究为长期内容（跨日期累积），不按天清除；`create_time` 反映首次发布时间
- `is_top=1` 的条目排在最前
- 置顶条目 `sort_no` 从 6 开始递减（非从 1 开始），非置顶条目按 `create_time` 倒序
- 响应中 `orderBy`、`order`、`map`、`params` 字段始终为 `null`/空值，可忽略
- `totalCount` / `totalPages` 不可靠，翻页务必以 `hasNext=false` 为准（详见上方「totalCount / totalPages 行为」）
