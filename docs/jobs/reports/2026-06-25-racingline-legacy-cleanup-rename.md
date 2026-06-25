# Racingline Legacy Cleanup And Rename

日期：2026-06-25

范围：Plan 0053，删除旧 `app/racingline/`，将 `app/racingline_new/` 重命名为正式 `app/racingline/`，并收敛 Makefile、ADR、RFC、系统地图和质量门禁。

## 结果

已完成：

1. 删除旧 Racingline 第一版前端 tracked 文件。
2. 将 `app/racingline_new/` 移动为 `app/racingline/`。
3. 更新 `package.json` / `package-lock.json` package name 为 `racingline`，保留 npm package manager 约束。
4. 删除 Vite/React 模板资产 `public/vite.svg` 和 `src/assets/react.svg`，并清理 `index.html` 中的模板 favicon/title 元数据。
5. 更新 `Makefile` 默认前端目录为 `app/racingline`，移除 `racingline-new-rearview-dev`。
6. 更新 Racingline / Rearview 系统地图、Racingline intake、ADR 0011、ADR 0013、RFC 0023 和相关历史 RFC/Q&A 路径说明。
7. 将旧 active Plan 0041 归档为 Superseded。

## 命令摘要

前置新工程基线：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

结果：通过，5 个测试文件、41 个测试通过。

删除和重命名：

```bash
git rm -r app/racingline
rm -rf app/racingline app/racingline_new/dist app/racingline_new/.playwright-cli
git mv app/racingline_new app/racingline
git rm -f app/racingline/public/vite.svg app/racingline/src/assets/react.svg
cd app/racingline
npm install --package-lock-only
```

重命名后前端门禁：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

结果：通过，5 个测试文件、41 个测试通过。Build 通过；Vite 输出 chunk size warning，未阻塞构建。

## Live Smoke

启动：

```bash
make racingline-dev
```

结果：

- PostgreSQL、ClickHouse、NATS 和 RustFS dev containers 已启动或保持运行。
- Alembic migrations 执行到 head。
- Rearview metric catalog sync 完成：81 metrics。
- Rearview server 启动在 `127.0.0.1:34057`。
- Rearview portfolio worker 启动。
- Racingline Vite dev server 启动在 `http://127.0.0.1:5173/`。

HTTP checks：

```bash
curl -fsS -o /tmp/racingline-dashboard.html -w '%{http_code} %{content_type}\n' http://127.0.0.1:5173/dashboard
curl -fsS -o /tmp/racingline-strategies.html -w '%{http_code} %{content_type}\n' http://127.0.0.1:5173/strategies
curl -fsS -o /tmp/rearview-dashboard.json -w '%{http_code} %{content_type}\n' http://127.0.0.1:34057/rearview/strategy-portfolios/dashboard
curl -fsS -o /tmp/rearview-metrics.json -w '%{http_code} %{content_type}\n' http://127.0.0.1:34057/rearview/metrics
```

结果：

| Check | Result |
|---|---|
| `/dashboard` | `200 text/html` |
| `/strategies` | `200 text/html` |
| `/rearview/strategy-portfolios/dashboard` | `200 application/json`, `portfolios = 1` |
| `/rearview/metrics` | `200 application/json`, `metric_count = 81` |

CDP check：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

结果：连接 Docker `vnc-mini-desktop` Chromium 成功，browser `Chrome/148.0.7778.178`。

Playwright network evidence：

| Page | Evidence |
|---|---|
| `/dashboard` | `GET http://127.0.0.1:34057/rearview/strategy-portfolios/dashboard => 200 OK` |
| `/strategies` | `GET http://127.0.0.1:34057/rearview/metrics => 200 OK`、`GET http://127.0.0.1:34057/rearview/market-fee-templates/default?market=CN_A_SHARE => 200 OK` |

Console evidence：

```text
Total messages: 3 (Errors: 0, Warnings: 0)
```

Only React DevTools informational message was present.

## Screenshots

| Path | Page |
|---|---|
| [assets/2026-06-25-racingline-dashboard.png](assets/2026-06-25-racingline-dashboard.png) | `/dashboard` |
| [assets/2026-06-25-racingline-strategies.png](assets/2026-06-25-racingline-strategies.png) | `/strategies` |

## Cleanup

The long-running dev stack was stopped with Ctrl-C after smoke. The `make racingline-dev` process exited with code 130 because it was intentionally interrupted after validation.
