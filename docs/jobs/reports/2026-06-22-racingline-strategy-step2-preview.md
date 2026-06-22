# Racingline Strategy Step 2 Preview Implementation Report

日期：2026-06-22

状态：Completed

## 范围

- Rearview `RuleVersionSpec.scoring.clamp` 统一到 `[0, 100]`。
- `app/racingline_new` 新增 Step 2 scoring catalog、`WeightIndicator[] -> ScoringSpec` adapter 和 Step 1 + Step 2 preview rule composer。
- Rearview 新增 `POST /rearview/strategy-preview` preview-only API，复用 planner + ClickHouse 查询，不写 rule set、rule version、run 或 portfolio run。
- `/strategies` Step 2 使用真实 `allow_scoring = true` metric catalog；Step 3 消费 preview response 展示 trade date、pool count、signals 和 score breakdown。

## 验证命令

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
npm run build
```

结果：通过。`vite build` 仅保留已有 chunk size warning。

```bash
cd app/racingline
npm test -- workbench
npm run typecheck
```

结果：通过。

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
```

结果：全部通过。catalog check 返回 81 metrics；catalog coverage 返回 136 dbt fields checked。

## 浏览器烟测

启动 Rearview 当前编译版本：

```bash
cd engines
REARVIEW_HTTP_BIND=127.0.0.1:34058 cargo run -p rearview-server -- serve
```

结果：服务启动成功，Postgres、ClickHouse 和 NATS readiness 通过。

API live smoke：

```bash
curl -sS --max-time 60 -w '\nHTTP_STATUS:%{http_code}\n' \
  -X POST http://127.0.0.1:34058/rearview/strategy-preview \
  -H 'Content-Type: application/json' \
  --data-binary '<single-day close_price >= 0 preview rule>'
```

结果：HTTP 200。返回 `preview_id`、`sql_hash`、`required_metrics = ["close_price"]`、`trade_dates[0].pool_count = 4959`、top 3 signals 和 `score_breakdown = {"weight:smoke:1": 50}`。

启动 Racingline：

```bash
cd app/racingline_new
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34058 npm run dev -- --host 127.0.0.1 --port 5175
```

Vite 使用 `http://127.0.0.1:5175/`。

Playwright CDP：

- 打开 `http://127.0.0.1:5175/strategies`，页面正常渲染 Step 1/2/3 导航和策略选股区域。
- 切换到 Step 2，权重配置面板正常渲染，指标类型和指标名使用 Rearview catalog 中文展示。
- 在无 Step 1 条件时点击「股池预览」，页面停留在 Step 2 并展示 adapter error：`至少需要一个指标组`。
- 创建 Step 1 默认条件、添加 Step 2 默认权重后点击「股池预览」，前端真实调用 `http://127.0.0.1:34058/rearview/strategy-preview`。
- Step 3 展示 `2026-05-26` 至 `2026-06-01` 的 5 个交易日；`2026-06-01` 显示 pool count `4959 只`、Top10 signals 和 score breakdown。
- Playwright console：0 errors，0 warnings。

后端 route 的 request 校验、按交易日聚合和 JSON 字段解析由 Rust 单元测试与 workspace test 覆盖；真实 ClickHouse 执行由 34058 live smoke 覆盖。

## 结论

Plan 0046 的 Step 2 记录评分规则、Step 3 执行选股评分预览的主链路已落地。`POST /rearview/explain` 仍只用于编译校验；真实股票池、score 和 rank 由 preview-only API 执行并返回。
