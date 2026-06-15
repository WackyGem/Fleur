# Racingline Security Analysis Optimization Acceptance

日期：2026-06-15

关联计划：[../plans/archive/0040-racingline-security-analysis-optimization-plan.md](../../plans/archive/0040-racingline-security-analysis-optimization-plan.md)

## 基本信息

```text
Frontend URL = http://127.0.0.1:5173
Rearview API base URL = http://127.0.0.1:34057
CDP endpoint = http://127.0.0.1:9222
Desktop viewport = 1440x1000
Mobile viewport = 375x900
```

样本：

```text
run_id = a4470e63-6fd3-46ce-9dab-8802c84cef26
security_code = 002298.SZ
trade_date = 2026-05-26
source = signals
adjustment = forward_adjusted
```

目标 URL：

```text
http://127.0.0.1:5173/runs/a4470e63-6fd3-46ce-9dab-8802c84cef26/securities/002298.SZ?adjustment=forward_adjusted&source=signals&trade_date=2026-05-26
```

## 范围

本次验收覆盖 Racingline 个股分析页 UI、Rearview analysis API 响应扩展、代表规则和 Rules preset 的新低位反转筛选/得分条件，以及评分 clamp 保留方案。验收不覆盖真实交易、调仓、回测、分钟线或 mart 指标重算。

## 改造验收

| 项 | 证据 | 结论 |
|---|---|---|
| 红涨绿跌 | chart tokens 已切换为 up red/down green；桌面截图中上涨 K 线和成交量柱为红色、下跌为绿色 | 通过 |
| 去除噪声 badge | DOM 文本检查 `current mart query` 和 `indicators forward_adjusted` 均为 false | 通过 |
| 工具栏交互 | `Forward` / `Backward` / `Unadjusted` 与 MA、EMA2、AVG overlay 控制位于同一横向 toolbar；两个 `ToggleGroup` 均无内部溢出 | 通过 |
| 新 overlay | `MA5`、`MA10`、`MA30`、`EMA2-10`、`AVG 3/6/12/24`、`AVG 14/28/57/114` 可作为价格叠加线展示；`EMA2-10` 点击后 `aria-pressed=true` | 通过 |
| 桌面图表 | 1440x1000 下页面无横向溢出，canvas 数量 23，非空 canvas 数量 12 | 通过 |
| 移动端图表 | 375x900 下页面无横向溢出，canvas 数量 23，非空 canvas 数量 12 | 通过 |
| 评分 clamp | 计划和实现继续使用 `scoring.clamp = { min: 0, max: 99 }`；SQL 使用 rule clamp 编译 `score`，并保留 `raw_score` | 通过 |
| score breakdown | PostgreSQL snapshot 的 `score_breakdown` 保留 points、raw_score、score 和 raw_values，用于解释裁剪前后分数 | 通过 |

## 截图

| 文件 | 视口 | 结论 |
|---|---|---|
| [../references/screenshots/racingline/2026-06-15/security-analysis-desktop-1440x1000.png](../../references/screenshots/racingline/2026-06-15/security-analysis-desktop-1440x1000.png) | 1440x1000 | 三栏布局可扫描，toolbar 和 overlay 控制可用，图表非空 |
| [../references/screenshots/racingline/2026-06-15/security-analysis-mobile-375x900.png](../../references/screenshots/racingline/2026-06-15/security-analysis-mobile-375x900.png) | 375x900 | Chart tab 可用，页面无横向溢出，图表非空 |

## Network 和 Console

关键请求：

```text
GET /rearview/runs/a4470e63-6fd3-46ce-9dab-8802c84cef26/signals?limit=50&offset=0&sort=rank_asc&trade_date=2026-05-26 -> 200
GET /rearview/runs/a4470e63-6fd3-46ce-9dab-8802c84cef26 -> 200
GET /rearview/runs/a4470e63-6fd3-46ce-9dab-8802c84cef26/days -> 200
GET /rearview/runs/a4470e63-6fd3-46ce-9dab-8802c84cef26/securities/002298.SZ/analysis?adjustment=forward_adjusted&source=signals&trade_date=2026-05-26 -> 200
GET /healthz -> 200
```

Console：

```text
Errors: 0
Warnings: 0
Info: React DevTools development hint only
```

## 工程门禁

```bash
cd app/racingline
npm run typecheck
npm run lint
npm run test
npm run build

cd ../../engines
cargo test -p rearview
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check
cargo test --workspace

cd ..
make docs-check
git diff --check
```

结果：

```text
Racingline typecheck/lint/test -> passed
Racingline build -> passed, with existing Vite chunk-size warning
Rearview package tests -> passed
Rust workspace fmt/clippy/test -> passed
docs-check -> passed
git diff --check -> passed
```

## 验收结论

通过。Plan 0040 的个股分析页视觉、工具栏、价格叠加线、Rearview 新规则适配和评分 clamp 保留方案均已完成本地验收。
