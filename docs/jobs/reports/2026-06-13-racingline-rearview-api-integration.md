# Racingline Rearview API Integration

日期：2026-06-13

范围：Racingline 第一版前端依赖的 Rearview API、CORS、真实运行闭环。

## 环境

```text
Rearview API = http://127.0.0.1:34057
Frontend URL = http://127.0.0.1:5173
PostgreSQL rearview database = postgresql://mono_fleur:...@127.0.0.1:34054/rearview
ClickHouse marts database = fleur_marts
```

Rearview 本地服务进程：

```text
target/debug/rearview serve
```

## 基础检查

```bash
curl -sS http://127.0.0.1:34057/healthz
```

结果：

```json
{"status":"ok"}
```

CORS 预检：

```bash
curl -sS -X OPTIONS -D - -o /tmp/racingline_cors_options.txt \
  -H 'Origin: http://127.0.0.1:5173' \
  -H 'Access-Control-Request-Method: GET' \
  'http://127.0.0.1:34057/rearview/runs?limit=1'
```

结果摘要：

```text
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: *
Access-Control-Allow-Headers: *
```

## API 验证

| API | 结果 |
|---|---|
| `GET /healthz` | 200 |
| `GET /rearview/runs?limit=5` | 200，返回 paged response |
| `GET /rearview/rule-sets?limit=5` | 200，返回 3 个 rule set |
| `GET /rearview/rule-sets/{rule_set_id}/versions?limit=100&offset=0` | 200 |
| `GET /rearview/metrics` | 200，返回 14 个 metric |
| `POST /rearview/explain` | 200 |
| `POST /rearview/rule-sets/{rule_set_id}/versions` | 201 |
| `POST /rearview/runs` | 202 |
| `GET /rearview/runs/{run_id}` | 200 |
| `GET /rearview/runs/{run_id}/chunks` | 200 |
| `GET /rearview/runs/{run_id}/days` | 200 |
| `GET /rearview/runs/{run_id}/signals?limit=50&offset=0&sort=rank_asc&trade_date=2026-06-01` | 200 |
| invalid `POST /rearview/explain` | 400，返回 `error_type` 和 `message` |

错误响应样例：

```json
{
  "error_type": "validation",
  "message": "validation error: metric is not registered: missing_metric_for_error_check"
}
```

## UI 驱动闭环

通过 Racingline `/rules` 页面完成：

1. 选择 `close_price` 作为 pool filter metric。
2. 设置 `op = gte`、`operand = 0`。
3. 设置运行区间 `2026-06-01` 至 `2026-06-01`，`top_n = 5`。
4. 点击 `Explain`。
5. 点击 `Publish`。
6. 点击 `Run` 并跳转 run detail。

关键结果：

```text
POST /rearview/explain -> 200
POST /rearview/rule-sets/96a803dc-a3ba-4d5b-a600-e90525d15954/versions -> 201
created rule_version_id = 4b30c55c-d9b1-4641-99d9-68ca71e39814
POST /rearview/runs -> 202
created run_id = 81cdee48-5131-4b6b-b555-c1456d793539
final status = succeeded
summary = day_count: 1, pool_count: 4959, signal_count: 5
```

终态 run 查询：

```json
{
  "run_id": "81cdee48-5131-4b6b-b555-c1456d793539",
  "rule_version_id": "4b30c55c-d9b1-4641-99d9-68ca71e39814",
  "status": "succeeded",
  "summary": {
    "day_count": 1,
    "pool_count": 4959,
    "signal_count": 5
  }
}
```

轮询检查：run 达到 terminal `succeeded` 后等待一个轮询周期，CDP request list 没有新增 run/chunks/days 自动请求，保留手动刷新。

## 处理过的问题

1. Lightweight Charts 不支持 shadcn Tailwind v4 的 `oklch(...)` token。已在业务图表组件中改为传入 hex chart palette，并设置 `layout.attributionLogo = false`，重验后无 chart runtime error。
2. `ExplainResponse.required_columns` 后端返回 `Record<string, string[]>`，前端原先按 `string[]` 渲染导致 `values.map is not a function`。已修正类型和 summary 展示，重验后 explain 页面不再崩溃。
