# Racingline Portfolio Publish T+1 Smoke

日期：2026-06-27

范围：Plan 0060 的 Step 5「建立组合」发布预检、T+1 建仓、pending dashboard/detail 语义、live daily run 信号窗口、backtest/live ClickHouse 事实族拆分和线上 Alembic migration。

## 环境

使用 dev Docker 依赖服务：

| 服务 | 端口 |
|---|---|
| PostgreSQL `rearview` | `127.0.0.1:34054` |
| ClickHouse | `127.0.0.1:34052` / `127.0.0.1:34053` |
| NATS | `127.0.0.1:34055` |
| Rearview HTTP | `127.0.0.1:34057` |

本 worktree 没有 `.env`，smoke 命令使用 `.env.example` 中的 dev 连接信息。

## Migration

线上 rearview migration 执行通过：

```bash
cd pipeline/migrate
REARVIEW_DATABASE_URL=postgresql://mono_fleur:change-me-postgres-password@127.0.0.1:34054/rearview \
  uv run alembic -c alembic.ini -x target=rearview upgrade head
```

注意：Alembic `version_num` 为 `VARCHAR(32)`，本轮 revision id 使用 `0009_strategy_portfolio_ctx`，对应文件为 `pipeline/migrate/versions/rearview/0009_strategy_portfolio_publish_context.py`。

PostgreSQL schema readiness 期望 head：

```text
0009_strategy_portfolio_ctx
```

## Publish Preview

最新数据源 run 结束在当前最大交易日 `2026-06-25`，发布预检正确阻止发布，没有回退到 T 日建仓：

| 字段 | 值 |
|---|---|
| Backtest run | `34222644-1c42-4b22-99b1-526c807a8df1` |
| Result attempt | `01KW3JMC9ZNRKMECSF1BPXFNTK` |
| Source signal date | `2026-06-25` |
| Result | `can_publish=false` |
| Blocker | `conflict: could not resolve next trading date after source_signal_date 2026-06-25` |

使用已有 succeeded source run 验证 T+1 发布成功：

| 字段 | 值 |
|---|---|
| Source run | `056c7294-0cf1-4d76-9230-1c2e103cd27c` |
| Source attempt | `01KVXD7RA0EAZPHJK0812Y5215` |
| Source signal date | `2026-06-01` |
| Planned live start date | `2026-06-02` |
| Pending buy signals | `5` |

## Pending Portfolio

创建组合：

| 字段 | 值 |
|---|---|
| Portfolio id | `01KW3JSSXS64DW6KB2CAJ56366` |
| Name | `Plan 0060 T+1 Smoke Portfolio` |
| `initial_signal_date` | `2026-06-01` |
| `live_start_date` | `2026-06-02` |
| `pending_buy_signal_snapshot` | `5` rows |
| Initial live status | `pending_first_run` |

Pending 状态接口行为：

| Endpoint | 结果 |
|---|---|
| `/rearview/strategy-portfolios/dashboard` | `curve_source=none`，`latest_nav=null`，returns/risk/curve empty，`pending_buy_signals=5` |
| `/rearview/strategy-portfolios/{id}/nav` | `409 portfolio_pending_first_run` |
| `/rearview/strategy-portfolios/{id}/performance` | `409 portfolio_pending_first_run` |
| `/rearview/strategy-portfolios/{id}/positions` | `409 portfolio_pending_first_run` |
| `/rearview/strategy-portfolios/{id}/rebalance-records` | `409 portfolio_pending_first_run` |
| `/rearview/strategy-portfolios/{id}/signals` | `source=publish_preview`，`signal_source=publish_preview`，pending snapshot 5 rows |
| `/rearview/strategy-portfolios/{id}/signal-timeline` | `trade_date=2026-06-01`，`signal_count=5` |

## Live Daily Runs

对 smoke portfolio 连续创建 daily run：

| Trade date | Daily run id | Result attempt | `run_start_date` | 状态 |
|---|---|---|---|---|
| `2026-06-02` | `01KW3JV9HDVCCJKZHBASVERHM0` | `01KW3JV9QFRCXXBGP2ZT56GVKD` | `2026-06-01` | `succeeded` |
| `2026-06-03` | `01KW3JXN114JTN5REMD5S2NZXZ` | `01KW3JXPD1W8EKV19Z4B1DX8N6` | `2026-06-01` | `succeeded` |
| `2026-06-10` | `01KW3JZWGH2X9E2S76BWRGRA1Q` | `01KW3JZWMS6WDK4E1QT98Q73HW` | `2026-06-01` | `succeeded` |
| `2026-06-25` | `01KW3K63SGKSFYFPK9AG90ZCMW` | `01KW3K64R76KCES1X09YHK0MYJ` | `2026-06-01` | `succeeded` |

关键 API 结果：

| 检查 | 结果 |
|---|---|
| Dashboard | `live_status=succeeded`，`curve_source=live_daily_run`，`latest_nav=0.967592571895344` |
| NAV first point | `2026-06-02`，strategy nav `1.0`，benchmark nav `1.0` |
| NAV last point | `2026-06-25`，strategy nav `0.967592571895344`，benchmark nav `1.0214759244628882` |
| Signals | `source=live_daily_run`，`signal_date=2026-06-01`，`execution_date=2026-06-02`，5 rows |
| Live performance | `insufficient_observations`，`observation_count=16`；该组合 live window 只有 17 NAV points，首日 return 被排除 |

## Performance Success Branch

为覆盖 `metric_status=succeeded`，使用更早的 succeeded source run 创建第二个组合：

| 字段 | 值 |
|---|---|
| Source run | `cd21b68b-05f9-4772-b840-da89f517447f` |
| Source attempt | `01KW0JT3EQYC1ZD82DC8XFSP3C` |
| Source signal date | `2025-12-31` |
| Planned live start date | `2026-01-05` |
| Portfolio id | `01KW3K9453WXGX3B1KJFZYQRVC` |
| Daily run id | `01KW3K9CRKA235WB2RKCGDHS7M` |
| Result attempt | `01KW3K9ECN3NZQD641Q65RH9FZ` |
| `run_start_date` | `2025-12-31` |
| `trade_date` | `2026-06-25` |

结果：

| 检查 | 结果 |
|---|---|
| NAV first point | `2026-01-05`，strategy nav `1.0`，benchmark nav `1.0` |
| NAV last point | `2026-06-25`，strategy nav `0.9339985558783508`，benchmark nav `1.0640895290307826` |
| Signals | `signal_date=2025-12-31`，`execution_date=2026-01-05`，5 rows |
| Performance | `metric_status=succeeded`，`observation_count=112` |
| Full-period window | `window_start=null`，`window_end=null`；这是 `PerformanceMetricConfig::full_period_with_benchmark` 的设计，不表示窗口缺失 |

## ClickHouse Facts

新事实族已分离：

```text
fleur_backtest.backtest_nav_daily
fleur_backtest.backtest_target
fleur_backtest.backtest_performance_metric
fleur_portfolio.live_nav_daily
fleur_portfolio.live_target
fleur_portfolio.live_performance_metric
```

Representative row checks：

| Table | Key | Date range | Rows | 结果 |
|---|---|---|---|---|
| `fleur_backtest.backtest_nav_daily` | `34222644-1c42-4b22-99b1-526c807a8df1` / `01KW3JMC9ZNRKMECSF1BPXFNTK` | `2025-06-25` - `2026-06-25` | `243` | backtest rows only |
| `fleur_portfolio.live_nav_daily` | `01KW3K63SGKSFYFPK9AG90ZCMW` / `01KW3K64R76KCES1X09YHK0MYJ` | `2026-06-02` - `2026-06-25` | `17` | live rows only |
| `fleur_portfolio.live_nav_daily` | `01KW3K9CRKA235WB2RKCGDHS7M` / `01KW3K9ECN3NZQD641Q65RH9FZ` | `2026-01-05` - `2026-06-25` | `113` | live rows only |
| `fleur_portfolio.live_target` | `01KW3K63SGKSFYFPK9AG90ZCMW` / `01KW3K64R76KCES1X09YHK0MYJ` | signal `2026-06-01`，execution `2026-06-02` | `5` | T -> T+1 target |
| `fleur_portfolio.live_target` | `01KW3K9CRKA235WB2RKCGDHS7M` / `01KW3K9ECN3NZQD641Q65RH9FZ` | signal `2025-12-31`，execution `2026-01-05` | `5` | T -> T+1 target |
| `fleur_portfolio.live_performance_metric` | `01KW3K63SGKSFYFPK9AG90ZCMW` | full period | `16` observations | `insufficient_observations` |
| `fleur_portfolio.live_performance_metric` | `01KW3K9CRKA235WB2RKCGDHS7M` | full period | `112` observations | `succeeded` |

Negative split checks:

```text
backtest_family_live_ids = 0
live_family_backtest_id = 0
```

## 浏览器截图证据

通过 `playwright-cli` 连接现有 CDP 浏览器采集：

| 截图 | 证据 |
|---|---|
| [Step 5 发布弹窗 - 策略配置](assets/2026-06-27-racingline-portfolio-publish-dialog-config.png) | Backtest `b14a7382-747c-4344-ac8f-1cf2a6d3f13f` / attempt `01KW3M65DCZG38KF09JSDRXBTB`；弹窗展示 backend publish preview blocker：`could not resolve next trading date after source_signal_date 2026-06-25`，建仓日期为空，确认按钮禁用。 |
| [Step 5 发布弹窗 - 回测业绩](assets/2026-06-27-racingline-portfolio-publish-dialog-performance.png) | 同一弹窗切换到「回测业绩」tab，展示回测区间 `2025-06-25 - 2026-06-25` 和业绩指标，同时保留发布预检 blocker。 |
| [Portfolio dashboard live card](assets/2026-06-27-racingline-portfolio-publish-dashboard-live.png) | Portfolio `01KW3JSSXS64DW6KB2CAJ56366` daily run 后展示 live segment、live curve source 和 latest NAV，不再用 backtest 伪装 live 数据。 |
| [Portfolio detail pending state](assets/2026-06-27-racingline-portfolio-publish-detail-pending.png) | Pending portfolio `01KW11CM8B1CFZYTR8XJTDDD9E` 展示待首次运行状态和 pending buy signals，不展示 fake live performance/nav。 |
| [Portfolio detail live state](assets/2026-06-27-racingline-portfolio-publish-detail-live.png) | Portfolio `01KW3JSSXS64DW6KB2CAJ56366` daily run 后展示 live detail。截图采集时仅在浏览器会话中临时隐藏 canvas，避免 Chromium 截图超时；页面数据和 UI 代码未因此变更。 |

## 验证命令

已通过：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
```

```bash
cd pipeline/migrate
REARVIEW_DATABASE_URL=postgresql://mono_fleur:change-me-postgres-password@127.0.0.1:34054/rearview \
  uv run alembic -c alembic.ini -x target=rearview upgrade head
```

```bash
make docs-check
make versions-check
git diff --check
```

Vite build 仍输出既有 chunk-size warning；本轮未改变该风险。
