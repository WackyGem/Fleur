# Racingline Playwright CDP Acceptance

日期：2026-06-13

## 基本信息

```text
Frontend URL = http://127.0.0.1:5173
Rearview API base URL = http://127.0.0.1:34057
CDP endpoint = http://127.0.0.1:9222
Browser = Chrome/148.0.7778.178
Desktop viewport = 1440x1000
Mobile viewport = 390x844
```

CDP 连通性：

```bash
node scripts/check_playwright_cdp.mjs
```

结果摘要：

```text
browser = Chrome/148.0.7778.178
protocolVersion = 1.3
webSocketDebuggerUrl = ws://127.0.0.1:9222/devtools/browser/...
```

## 前置检查

```text
GET http://127.0.0.1:34057/healthz -> 200
GET http://127.0.0.1:5173/ -> 200 text/html
```

工程门禁：

```text
npm run lint       -> passed
npm run typecheck  -> passed
npm run test       -> 3 files passed, 10 tests passed
npm run build      -> passed with Vite chunk size warning
```

## 截图清单

| 文件 | 视口 | URL / 状态 | 结论 |
|---|---|---|---|
| `docs/references/screenshots/racingline/2026-06-13/01-runs-desktop.png` | desktop | `/runs` | 运行看板直接进入选股工作流 |
| `docs/references/screenshots/racingline/2026-06-13/02-run-detail-desktop.png` | desktop | `/runs/e5f59cae-69b4-4a4a-a102-86c77f25848e` | run detail、信号表和图表可见 |
| `docs/references/screenshots/racingline/2026-06-13/03-signal-detail-desktop.png` | desktop | signal sheet | score breakdown、selected metrics、snapshot 说明可见 |
| `docs/references/screenshots/racingline/2026-06-13/04-rules-desktop.png` | desktop | `/rules` | explain 成功后展示 required metrics/marts/columns/chunk plan |
| `docs/references/screenshots/racingline/2026-06-13/05-ui-created-run-desktop.png` | desktop | UI 创建的 run detail | publish/run 后跳转并完成 run |
| `docs/references/screenshots/racingline/2026-06-13/06-metrics-desktop.png` | desktop | `/metrics` | metric catalog 表格可见 |
| `docs/references/screenshots/racingline/2026-06-13/07-runs-mobile.png` | mobile | `/runs` | 移动导航和运行看板可用 |
| `docs/references/screenshots/racingline/2026-06-13/08-run-detail-mobile.png` | mobile | UI 创建的 run detail | 移动 run detail 无页面级横向溢出 |
| `docs/references/screenshots/racingline/2026-06-13/09-signal-detail-mobile.png` | mobile | signal sheet | 移动抽屉可见且可关闭 |
| `docs/references/screenshots/racingline/2026-06-13/10-rules-mobile.png` | mobile | `/rules` | 表单控件无页面级横向溢出 |
| `docs/references/screenshots/racingline/2026-06-13/11-metrics-mobile.png` | mobile | `/metrics` | 指标目录移动页无页面级横向溢出 |
| `docs/references/screenshots/racingline/2026-06-13/12-health-failure-mobile.png` | mobile | healthz route 503 | 健康失败横幅可见 |
| `docs/references/screenshots/racingline/2026-06-13/13-health-recovered-mobile.png` | mobile | healthz restored 200 | 健康状态恢复 |

## 工作流证据

| 步骤 | 用户动作 | Network | Console | 状态 |
|---|---|---|---|---|
| 1 | 打开 `/runs` | `GET /rearview/runs?limit=50&offset=0 -> 200`，`GET /rearview/rule-sets?limit=100&offset=0 -> 200`，`GET /healthz -> 200` | 正常路径无 errors/warnings，仅 React DevTools info | 通过 |
| 2 | 打开 run detail | `GET /runs/{run_id}`、`/chunks`、`/days`、`/signals` 均 200 | 无 errors/warnings | 通过 |
| 3 | 检查图表 | canvas count = 7；首个 canvas `nonTransparent = 6400` | 无 Lightweight Charts error | 通过 |
| 4 | 打开信号详情抽屉 | 使用第一条 buy signal 的 `Open` 按钮 | signal 数据来自 run snapshot | 无 errors/warnings | 通过 |
| 5 | 打开 `/rules` 并 explain | `POST /rearview/explain -> 200` | 无 errors/warnings | 通过 |
| 6 | 发布规则版本 | `POST /rearview/rule-sets/{id}/versions -> 201` | 无 errors/warnings | 通过 |
| 7 | 发起 run | `POST /rearview/runs -> 202`，跳转 `/runs/81cdee48-5131-4b6b-b555-c1456d793539` | 无 errors/warnings | 通过 |
| 8 | terminal run 停止轮询 | run 最终 `succeeded` 后等待 6 秒，request list 无新增自动轮询请求 | 无 errors/warnings | 通过 |
| 9 | 打开 `/metrics` | `GET /rearview/metrics -> 200` | 无 errors/warnings | 通过 |
| 10 | 指标目录 keyword 过滤、复制 metric、加入 output metrics | `GET /rearview/metrics?keyword=rsi -> 200` | 无 errors/warnings | 通过 |
| 11 | 健康失败演练 | Playwright route 仅拦截 `GET /healthz -> 503` | 预期资源 503 error，两条；页面显示失败状态 | 通过 |
| 12 | 健康恢复 | 取消 route 后重载，`GET /healthz -> 200` | 新 console 仅 React DevTools info | 通过 |

## 移动响应式检查

DOM 尺寸检查结果：

```text
/runs mobile:       innerWidth=390, docScrollWidth=375, overflow=false
/runs/:runId mobile innerWidth=390, docScrollWidth=375, overflow=false
signal sheet mobile innerWidth=390, docScrollWidth=375, overflow=false, openDialogs=1
signal sheet close  openDialogs=0
/rules mobile       innerWidth=390, docScrollWidth=375, overflow=false
/metrics mobile     innerWidth=390, docScrollWidth=375, overflow=false
```

## Network 清单

关键请求结论：

```text
GET  /healthz -> 200
GET  /rearview/runs?limit=50&offset=0 -> 200
GET  /rearview/rule-sets?limit=100&offset=0 -> 200
GET  /rearview/rule-sets/{rule_set_id}/versions?limit=100&offset=0 -> 200
GET  /rearview/metrics -> 200
POST /rearview/explain -> 200
POST /rearview/rule-sets/{rule_set_id}/versions -> 201
POST /rearview/runs -> 202
GET  /rearview/runs/{run_id} -> 200
GET  /rearview/runs/{run_id}/chunks -> 200
GET  /rearview/runs/{run_id}/days -> 200
GET  /rearview/runs/{run_id}/signals?limit=50&offset=0&sort=rank_asc&trade_date=2026-06-01 -> 200
GET  /rearview/metrics?keyword=rsi -> 200
```

健康失败演练期间：

```text
GET /healthz -> 503
GET /healthz -> 503
```

恢复后：

```text
GET /healthz -> 200
```

## Console 清单

正常路径 final clean session：

```text
Errors: 0
Warnings: 0
Info: React DevTools development hint only
```

健康失败演练 session：

```text
Expected resource load errors: 2 x GET /healthz 503
Unhandled promise rejection: none observed
React runtime error: none observed
Duplicate key warning: none observed
```

## 遗留问题

1. `npm run build` 有 Vite chunk size warning。第一版不阻塞；后续前端体积继续增长时，优先引入 route-level dynamic import。
2. 健康失败状态通过 Playwright route 拦截 `GET /healthz` 演练，未实际停止 Rearview 进程。

## 验收结论

通过。Racingline 第一版页面、真实 Rearview API 联调、规则 explain/publish/run 闭环、桌面和移动响应式、console/network、截图证据链均已完成。
