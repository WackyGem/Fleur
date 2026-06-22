# Racingline Strategy Step 1 Gap Closure

日期：2026-06-21

范围：

- 前端：`app/racingline_new` `/strategies` Step 1 策略选股。
- 后端：Rearview metric catalog、`POST /rearview/explain`、crossing operator 编译。
- 数据层：`mart_stock_trend_indicator_daily` crossing 前值字段。
- 计划：`docs/plans/archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md`。

## 变更摘要

本次完成 Step 1 vertical slice：

```text
GET /rearview/metrics
  -> /strategies 策略选股草稿
  -> RuleVersionSpec
  -> POST /rearview/explain
  -> explain 结果反馈
```

关键结果：

- 趋势 mart 增加 crossing 所需 `prev_*` 字段，并同步 dbt YAML 和设计文档。
- Rearview metric policy 增加 operator profiles、`cross.previous_metric`、display hint、ignored fields、catalog coverage。
- Rearview 支持 `crosses_above` / `crosses_below` validation 和 SQL planner 编译。
- `app/racingline_new` 增加 Rearview API runtime、TanStack Query、真实 catalog adapter、RuleVersionSpec adapter 和 explain 面板。
- 组内 `AND/OR` 混排固定为 `AND` 高于 `OR`，由 nested `all` / `any` AST 表达。
- 指标类型展示适配中文，例如 `quotes` 显示为“行情与涨跌”，底层 group id 保持稳定英文 key。

## 命令结果

### 数据层

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator_daily --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

结果：全部通过。`mart_stock_trend_indicator_daily` build 用于验证新增 `prev_*` 字段和唯一粒度测试。

### Rearview

```bash
cd engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：

```text
metric catalog check passed: 70 metrics
metric catalog coverage passed: 136 dbt fields checked
cargo fmt --check passed
cargo clippy passed
cargo test --workspace passed
```

### 前端

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
npm run build
```

结果：

```text
Vitest: 1 file passed, 12 tests passed
typecheck passed
lint passed
build passed
```

`npm run build` 仍有 Vite chunk size warning；本次未处理代码拆分。

### 文档

```bash
make docs-check
git diff --check
```

结果：通过。

## Explain 样本

服务启动：

```bash
make rearview-dev
```

结果：

```text
metric catalog sync completed: 70 metrics, 70 rows affected
starting rearview HTTP service bind=127.0.0.1:34057
```

### 普通数值比较成功

规则：`kdj_j_value >= 0`

结果摘要：

```text
sql_hash = 38a3a5ee587a1e4462988a44ff3dbf8a9114b7f06c45ed283fae99edefb1b9a8
required_metrics = [kdj_j_value]
required_marts = [
  fleur_marts.mart_stock_momentum_indicator_daily,
  fleur_marts.mart_stock_quotes_daily
]
```

### 指标比较成功

规则：`price_ma_5 > price_ma_20`

结果摘要：

```text
sql_hash = f4756222367c133e3d0c76630981e67b53c8b4ec00fe8addc7d82e934f63edd7
required_metrics = [price_ma_20, price_ma_5]
required_columns includes price_ma_20, price_ma_5
```

### Crossing 成功

规则：`price_ma_5 crosses_above price_ma_20`

结果摘要：

```text
sql_hash = 1f7825e7438039d6d435ebbf7cd7e9053ec37421ebde598690ab53eeb066235d
required_metrics = [
  prev_price_ma_20,
  prev_price_ma_5,
  price_ma_20,
  price_ma_5
]
required_columns includes prev_price_ma_20, prev_price_ma_5, price_ma_20, price_ma_5
```

### Crossing 失败

规则：`kdj_j_value crosses_above 0`

结果摘要：

```text
error_type = validation
message = validation error: operator CrossesAbove is not allowed for metric kdj_j_value
field_path = null
```

## 浏览器验收

命令：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
playwright-cli --s=default goto http://127.0.0.1:5174/strategies
```

观察项：

- Rearview 未启动时，`/strategies` 仍展示 Step 1 编辑组件和“正在加载真实指标目录”状态；真实校验保持禁用，不把静态 catalog 当成功路径。
- Rearview 启动后，`GET /rearview/metrics` 返回 200，页面使用真实 catalog。
- “指标类型”展示中文，例如“行情与涨跌”；隐藏 value 仍为 `quotes`。
- 创建指标组后默认条件可见，指标显示中文 label，例如“收盘价”。
- 点击“校验规则”后，`POST /rearview/explain` 返回 200，页面展示 `sql_hash`、required metrics/marts/columns 和只读 `RuleVersionSpec` JSON。
- 修改条件值后，页面显示“规则校验已过期”，并禁用“配置权重”。

截图：

- [2026-06-21-racingline-step1-explain.png](assets/2026-06-21-racingline-step1-explain.png)

## 后续未接入范围

以下仍按 Plan 0045 非目标处理，未在本次变成真实 API 闭环：

- 权重配置。
- 股池预览。
- 模拟建仓。
- 策略回测。
- `app/racingline_new` 到正式 `app/racingline` 的工程迁移。

如果要迁移到正式工程，应另起迁移计划并遵守 ADR 0011/0013。
