---
name: fleur-playwright-cdp-debug
description: mono-fleur 前端 CDP 调试流程。用于调试 Racingline 或 app/ 下其他前端时，通过全局 playwright-cli 连接 vnc-mini-desktop 暴露的 Chromium CDP 端点 9222，检查页面截图、DOM snapshot、console、network、响应式布局和交互问题；也用于验证 PLAYWRIGHT_CDP_ENDPOINT、安装/使用官方 Playwright CLI agent skill、或排查 CDP 浏览器连接。
---

# Playwright CDP Frontend Debug

## Overview

Use this skill for exploratory frontend debugging in mono-fleur through an existing Chromium browser exposed by `vnc-mini-desktop` over Chrome DevTools Protocol.

This repo skill is a thin mono-fleur wrapper. Prefer the official `playwright-cli` skill for detailed command semantics when it is installed; use this skill for project defaults, endpoint checks, and safe workflow boundaries.

## Defaults

- Global CLI package: `@playwright/cli`
- CLI command: `playwright-cli`
- Official skill install command: `playwright-cli install --skills agents`
- CDP env var: `PLAYWRIGHT_CDP_ENDPOINT`
- Default CDP endpoint: `http://127.0.0.1:9222`
- CDP health check: `node scripts/check_playwright_cdp.mjs`
- Browser owner: Docker `vnc-mini-desktop`; do not close this browser unless explicitly asked.
- Default Playwright CLI entrypoint: `playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"`; do not use `playwright-cli open` for this repo workflow.

## Workflow

1. Start the frontend dev server if the page requires one.
2. Verify the CDP endpoint:

```bash
node scripts/check_playwright_cdp.mjs
```

3. Attach to the existing browser:

```bash
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

4. Navigate and inspect:

```bash
playwright-cli goto http://127.0.0.1:<port>
playwright-cli snapshot
playwright-cli console
playwright-cli requests
playwright-cli screenshot
```

5. Check responsive layouts with stable viewport sizes:

```bash
playwright-cli resize 1440 900
playwright-cli screenshot
playwright-cli resize 390 844
playwright-cli screenshot
```

6. Detach when finished:

```bash
playwright-cli detach
```

## Guidance

- Use CDP debugging for exploratory UI inspection, screenshots, console/network diagnostics, and reproducing user-visible issues.
- Use deterministic Playwright tests for CI-grade assertions; CDP attachment is lower fidelity than Playwright's native protocol and is Chromium-only.
- Do not rely on local browser downloads or system Chrome for this workflow. The target browser is the existing CDP browser in `vnc-mini-desktop`.
- Do not start a new browser with `playwright-cli open` unless the user explicitly asks for an isolated local browser session.
- Do not use `playwright-cli close` against the external VNC browser unless the user explicitly asks to shut it down.
- If `playwright-cli install --skills agents` retries browser downloads and the official skill directory already exists, it is acceptable to stop the download; CDP debugging can still work.

## Common Failures

- `ECONNREFUSED` from `scripts/check_playwright_cdp.mjs`: `vnc-mini-desktop` is not running, port `9222` is not published, or `PLAYWRIGHT_CDP_ENDPOINT` points to the wrong host.
- `playwright-cli: command not found`: install the global package with `npm install -g @playwright/cli`.
- Attached session has no useful page: run `playwright-cli tab-list`, `playwright-cli tab-new <url>`, or `playwright-cli goto <url>`.
- UI looks stale: reload the page and confirm the frontend dev server is serving the expected build.
