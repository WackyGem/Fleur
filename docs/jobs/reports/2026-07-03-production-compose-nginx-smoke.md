# 2026-07-03 Production Compose nginx Smoke

日期：2026-07-03

范围：RFC 0047 production-like Docker Compose、nginx 统一入口、Racingline 静态构建、Rearview server/worker、Dagster webserver/daemon、Alembic migration 和 Rearview catalog sync。

## 结论

Production-like Compose 已完成端到端 smoke：

- `make prod-up` 成功构建并启动 production-like 栈。
- `db-migrate` 自动执行并 `Exited (0)`。
- `rearview-catalog-sync` 自动执行并 `Exited (0)`，同步 `84 metrics`。
- nginx 统一入口 `http://127.0.0.1:35080/` 默认进入 Racingline。
- `/rearview/health`、`/rearview/strategy-portfolios/dashboard` 和 `/dagster/` 均通过 nginx 返回 `200`。
- Playwright CDP 浏览器验证 Racingline 请求走 `http://127.0.0.1:35080/rearview/...`，Dagster 静态资源和 GraphQL 请求走 `http://127.0.0.1:35080/dagster/...`。

本轮发现并修复了一个生产构建问题：Racingline API client 的 path 已包含 `/rearview/...`，因此 production build 不能设置 `VITE_REARVIEW_API_BASE_URL=/rearview`。修复后生产 build 使用显式空 base URL，并让 API client 区分“未配置”和“显式空字符串”，从而在生产同源入口下请求 `/rearview/...`，在 dev 未配置时仍回退到 `http://127.0.0.1:34057`。

## 环境

| 项 | 值 |
|---|---|
| Compose file | `deploy/docker-compose.yml` |
| Compose project | `fleur-prod` |
| nginx unified HTTP | `35080:80` |
| RustFS API | `35050:9000` |
| RustFS console | `35051:9001` |
| ClickHouse HTTP | `35052:8123` |
| ClickHouse native | `35053:9000` |
| PostgreSQL | `35054:5432` |
| NATS client | `35055:4222` |
| NATS monitor | `35056:8222` |

## Commands

Production startup:

```bash
make prod-up
```

Explicit init idempotency check:

```bash
make prod-init
```

HTTP smoke:

```bash
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/rearview/health
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/rearview/strategy-portfolios/dashboard
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/dagster/
```

Browser smoke:

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
playwright-cli --s=default tab-new http://127.0.0.1:35080/
playwright-cli --s=default console
playwright-cli --s=default requests --static
playwright-cli --s=default tab-new http://127.0.0.1:35080/dagster/
playwright-cli --s=default console
playwright-cli --s=default requests --static
```

## Service Status

After `make prod-up` and nginx rebuild:

| Service | Status |
|---|---|
| `rustfs` | Up, healthy |
| `rustfs-init` | Exited 0 |
| `clickhouse` | Up, healthy |
| `postgres` | Up, healthy |
| `nats` | Up, healthy |
| `db-migrate` | Exited 0 |
| `rearview-catalog-sync` | Exited 0 |
| `rearview-server` | Up, healthy |
| `rearview-portfolio-worker` | Up |
| `dagster-webserver` | Up, healthy |
| `dagster-daemon` | Up |
| `nginx` | Up, healthy |

## Evidence

Compose port rendering:

```text
clickhouse 35052:8123
clickhouse 35053:9000
nats 35055:4222
nats 35056:8222
nginx 35080:80
postgres 35054:5432
rustfs 35050:9000
rustfs 35051:9001
```

HTTP smoke result:

```text
root_status=200
dashboard_status=200
rearview_health=200
rearview_dashboard_status=200
dagster_status=200
```

Rearview init evidence:

```text
metric catalog sync completed: 84 metrics, 84 rows affected
```

Racingline browser evidence:

```text
Page URL: http://127.0.0.1:35080/dashboard
Page Title: Racingline
Console: 0 errors, 0 warnings
[GET] http://127.0.0.1:35080/rearview/strategy-portfolios/dashboard => [200] OK
```

Dagster browser evidence:

```text
Page URL: http://127.0.0.1:35080/dagster/overview/activity/timeline
Page Title: Overview | Timeline
Console: 0 errors, 14 warnings
[POST] http://127.0.0.1:35080/dagster/graphql?op=LocationWorkspaceQuery => [200] OK
[POST] http://127.0.0.1:35080/dagster/graphql?op=CompletedRunTimelineQuery => [200] OK
```

The Dagster warnings are Apollo client `notifyOnNetworkStatusChange` warnings emitted by Dagster's bundled UI, not nginx path-prefix failures.

## Fixes During Smoke

Initial browser smoke exposed two bad Racingline API request forms:

```text
http://127.0.0.1:35080/rearview/rearview/strategy-portfolios/dashboard
http://127.0.0.1:34057/rearview/strategy-portfolios/dashboard
```

Root cause:

- `app/racingline/src/api/rearview.ts` already passes paths beginning with `/rearview/...`.
- `VITE_REARVIEW_API_BASE_URL=/rearview` duplicated that path.
- `VITE_REARVIEW_API_BASE_URL=""` then hit the existing falsy fallback in `apiBaseUrl()` and reverted to `http://127.0.0.1:34057`.

Fix:

- `deploy/docker-compose.yml` now passes `VITE_REARVIEW_API_BASE_URL: ""` for nginx/Racingline production build.
- `deploy/docker/nginx/Dockerfile` defaults the arg to `""`.
- `app/racingline/src/api/client.ts` treats `undefined` as “use dev fallback” and an explicit empty string as “same-origin base”.
- `app/racingline/src/api/rearview.test.ts` covers the explicit empty base behavior.

## Follow-Up

Production hardening remains outside RFC 0047's first implementation scope:

- TLS/auth is not implemented.
- Infrastructure ports remain exposed on `35xxx` for smoke and ops access.
- Dagster telemetry is still enabled by default in the container `DAGSTER_HOME`.
