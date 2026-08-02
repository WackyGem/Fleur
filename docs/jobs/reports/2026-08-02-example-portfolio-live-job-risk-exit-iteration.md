# Example Portfolio Live Job 风控退出策略迭代报告

日期：2026-08-02

- Issue：[#1 — 用例策略缺少获利止盈条件导致收益过低](https://github.com/WackyGem/Fleur/issues/1)
- 范围：`example__portfolio_live_job` 的 `racingline_0051_low_reversal` example fixture 风控退出规则从 v2 迭代到 v8 的完整变更史（均发生在 2026-08-02）。本文档合并历次单版本报告，记录每次策略调整、引擎能力扩展、部署事故、实盘收益对照与最终结论。

---

## 版本演进总览

fixture version 经 `v2 → v3 → v4 → v5 → v6 → v7 → v8`。`rule_hash` 始终不变（选股 rule spec 未改，仅 `exit_rules` 变），所有迭代只动 `execution_config` 的 `risk_exit_policy.exit_rules` 与 version 常量。

| 版本 | exit_rules | execution_config_hash | 变更点 |
|---|---|---|---|
| v2 | 时间止损 20 日 | `6cf814ca...` | 基线（2026-07-03 策略搜索置换） |
| v3 | + 8% 止盈 | `7c28160c...` | 追加止盈 |
| v4 | + 10% 止损 + MA30 破位 | `080b9ade...` | 追加下行保护 |
| v5 | 移除时间止损/止盈；10% 止损 + MA5 下穿 | `50977e30...` | 策略重构，引擎新增 `cross_below_metric` |
| v6 | 10% 止损 + MA10 下穿 + 20% 止盈 | `fb220eb4...` | MA5→MA10 调参，恢复止盈 |
| v7 | = v4（时间止损 + 8% 止盈 + 10% 止损 + MA30） | `080b9ade...` | 回退到 v4 |
| **v8** | = v3（时间止损 20 日 + 8% 止盈） | `7c28160c...` | **当前**：回退到 v3 |

`execution_config_hash` 不含 version（`compute_hash` = `hash_json(self)`，`self` 是 `BacktestExecutionConfig`），故 exit_rules 内容相同的版本 hash 一致（v8=v3=`7c28160c...`，v7=v4=`080b9ade...`）。`fixture_hash` 含 version 输入，每个版本不同。

---

## 实盘收益对照（决策依据）

穷举 prod ClickHouse `fleur_portfolio.live_nav_daily` 全部 26 个 `result_attempt_id`，取各版本全量结算 attempt（2024-01-02 至 2026-07-31，624 交易日，初始 NAV=1.0）。基准 `000905.SH` 同期累计 +38.03%（强势上涨行情）。

| 版本 | exit_rules | 总收益 | 超额(vs基准) | 最大回撤 | alpha | IR |
|---|---|---|---|---|---|---|
| **v3** | 时间止损 + 8% 止盈 | **+42.04%** ⭐ | **+4.01%** ⭐ | −21.46% | **+6.45%** | **+0.05** |
| v4/v7 | + 10% 止损 + MA30 | +17.83% | −20.20% | −30.19% | −0.75% | −0.30 |
| v6 | 10% 止损 + MA10 下穿 + 20% 止盈 | +8.27% | −29.76% | −36.71% | −4.76% | −0.39 |
| v2 | 时间止损 | −5.34% | −43.37% | −34.36% | −12.73% | −0.65 |
| v5 | 10% 止损 + MA5 下穿 | −14.06% | −52.09% | −35.34% | −12.84% | −0.72 |

**结论：v3 在总收益、超额收益、最大回撤、alpha、IR 五项均全面领先，是唯一跑赢基准（+4.01%）的版本。** v4 追加的 10% 固定止损 + MA30 破位退出过早割肉，把仍在上涨的持仓提前卖飞——收益从 +42% 暴跌到 +17.83%，回撤反而从 −21% 恶化到 −30%。v5/v6 继续在此方向走（MA 下穿、移除时间止损），收益更差。最终回退到 v3 策略（v8）。

> v3 的 portfolio 记录已从 Postgres 删除，但其 ClickHouse NAV/snapshot/performance 数据完整保留，策略定义已从 `live_run_snapshot.execution_snapshot` 核实（`source_period_key="example_v3"`、`execution_config_hash="7c28160c..."`）。

---

## 逐版本变更记录

### v3：追加 8% 止盈（v2 → v3）

v2（2026-07-03 策略搜索置换）的 `risk_exit_policy` 仅启用 20 日时间止损，不含止盈。v3 在保留时间止损的前提下追加一条 8% 固定止盈。

- 运行时语义：逐持仓判断 `unrealized_return = close_price / average_entry_price - 1.0`，当 `>= 0.08` 触发卖出。
- `trigger_timing = "close_confirm_next_open"`：收盘确认浮盈达 8% 后次日开盘卖出（T+1）。
- 引擎无新增代码（`TakeProfit` 规则已存在），仅启用。
- 哈希：`rule_hash` 不变；`execution_config_hash` `6cf814ca...` → `7c28160c...`。

```rust
exit_rules: vec![
    ExitRuleConfig::TimeStopLoss { holding_days: 20, max_return_pct: 0.0 },
    ExitRuleConfig::TakeProfit { profit_pct: 0.08 },
],
```

### v4：追加 10% 止损与 MA30 破位（v3 → v4）

v3 缺下行保护：持仓浮亏无硬性止损，且无趋势破位退出。v4 追加两条规则（保留时间止损与 8% 止盈，共四条）。

- `FixedStopLoss { loss_pct: 0.10 }`：`unrealized_return <= -0.10` 触发。
- `IndicatorStopLoss { source: "trend", metric: "price_ma_30", operator: "close_below_metric" }`：当前价（前复权收盘，缺失回退后复权）小于 `price_ma_30` 时触发。`price_ma_30` 在 `TREND_STOP_LOSS_METRICS` 白名单。
- 引擎无新增代码（两条规则已存在并经测试），仅启用。
- 哈希：`rule_hash` 不变；`execution_config_hash` `7c28160c...` → `080b9ade...`。

```rust
exit_rules: vec![
    ExitRuleConfig::TimeStopLoss { holding_days: 20, max_return_pct: 0.0 },
    ExitRuleConfig::TakeProfit { profit_pct: 0.08 },
    ExitRuleConfig::FixedStopLoss { loss_pct: 0.10 },
    ExitRuleConfig::IndicatorStopLoss {
        source: "trend".to_string(),
        metric: "price_ma_30".to_string(),
        operator: "close_below_metric".to_string(),
    },
],
```

### v5：MA5 下穿重构 + 引擎扩展（v4 → v5）

按新风控策略重写：移除时间止损与固定止盈，保留 10% 固定止损，MA 趋势退出由 MA30 单 bar 比较改为 MA5 下穿（crossover）。需求 #4 的"从上方跌破"是**下穿**，不是单 bar 比较——引擎此前只支持单 bar 的 `close_below_metric`，无法表达"昨日收盘 ≥ MA 且 今日收盘 < MA"。

#### 引擎能力扩展：新增 `cross_below_metric` operator

- 新增 operator `"cross_below_metric"`，保留 `"close_below_metric"`（两套语义并存）。
- 建仓首日（无前日 bar）不评估 cross 条件 → 不触发。
- 语义对齐 buy-side 已有的 `CrossesBelow`（`planner/sql.rs:716`），但退出路径基于 `PriceBar` 序列内存求值，不共享其代码。

涉及 3 个引擎文件：

- `engines/crates/rearview-core/src/strategy_backtest.rs`：新增 `validate_one_of_strings`；`ExitRuleConfig::validate` 的 operator 校验放宽到接受 `close_below_metric` / `cross_below_metric`；新增 accept 测试。
- `engines/crates/rearview-core/src/portfolio/mod.rs`：`ExitRule::IndicatorStopLoss` 增加 `operator` 字段（serde 默认 `close_below_metric`，向后兼容）；`triggered_exit_reason` 增加 `prev_price_bar` 参数并按 operator 分支；新增 `indicator_down_cross` 与 `default_indicator_operator` 辅助；`TradeCalendarPlan` 增加 `prev_by_date` 与 `prev_trade_date`；daily loop 退出评估段传入前日 bar；新增 3 个 cross 语义测试（下穿触发 / 仍 below 不触发 / 首日不触发）。
- `engines/crates/rearview-portfolio-worker/src/main.rs`：新增 `validate_one_of_str`；`RiskExitPolicy::exit_rules` 的 indicator 分支放宽 operator 校验并把 operator 塞进 `ExitRule`；新增 `exit_rules_should_convert_cross_below_metric_operator` 测试。

哈希：`rule_hash` 不变；`execution_config_hash` `080b9ade...` → `50977e30...`。

```rust
exit_rules: vec![
    ExitRuleConfig::FixedStopLoss { loss_pct: 0.10 },
    ExitRuleConfig::IndicatorStopLoss {
        source: "trend".to_string(),
        metric: "price_ma_5".to_string(),
        operator: "cross_below_metric".to_string(),
    },
],
```

### v6：MA10 下穿 + 20% 止盈调参（v5 → v6）

调参：保留 10% 固定止损，MA 趋势退出由 MA5 改为 MA10（仍用下穿语义），新增 20% 固定止盈。无引擎代码变更（`cross_below_metric` 已在 v5 实现，`price_ma_10` 在白名单）。

哈希：`rule_hash` 不变；`execution_config_hash` `50977e30...` → `fb220eb4...`。

```rust
exit_rules: vec![
    ExitRuleConfig::FixedStopLoss { loss_pct: 0.10 },
    ExitRuleConfig::IndicatorStopLoss {
        source: "trend".to_string(),
        metric: "price_ma_10".to_string(),
        operator: "cross_below_metric".to_string(),
    },
    ExitRuleConfig::TakeProfit { profit_pct: 0.20 },
],
```

### v7：回退到 v4 策略（v6 → v7）

基于实盘收益对照（v4 +17.83% 在 v2/v4/v5/v6 中最优），回退到 v4 的四条规则，但用新版本号 v7（避免与 archived v4 portfolio 混淆）。无引擎代码变更；部署后归档 active v6。

哈希：`rule_hash` 不变；`execution_config_hash` 回到 `080b9ade...`（= v4）。

### v8：回退到 v3 策略【当前】（v7 → v8）

穷举 ClickHouse 全部 26 个 attempt 后发现 v3 五项指标全面领先（见上文收益对照）。v4 追加的下行保护过早割肉，反而害了收益。回退到 v3 的两条规则，用新版本号 v8。无引擎代码变更；部署后归档 active v7。

哈希：`rule_hash` 不变；`execution_config_hash` 回到 `7c28160c...`（= v3）。

---

## 部署事故与修复（v3/v4 部署期间）

### 事故一：双源 version 常量遗漏导致 409

v3 首次发布后 `dg launch` 失败，ensure 返回 HTTP 409 `conflict: ... v2 already exists with different canonical snapshot`。

**根因**：fixture version 是**双源**的——Python `EXAMPLE_0051_VERSION`（Dagster 端校验）与 Rust `RACINGLINE_0051_LOW_REVERSAL_VERSION`（ensure endpoint 查找键 + fixture_hash 输入）。首版只升了 Python 常量，Rust 仍是 v2，ensure 用 v2 查找键匹配库中 v2 记录但 snapshot 已含新规则，触发冲突。

**修复**：Rust 常量同步升到 v3。此后每次 version bump 都双源同步。

### 事故二：nginx 缓存旧 upstream IP 导致 502

force-recreate 应用容器后，ensure 经 nginx 返回 502。rearview-server 直连 `/healthz` 正常，nginx error log 显示 `connect() failed (111: Connection refused) ... 172.19.0.6:34057`。

**根因**：force-recreate 后容器获新 IP，nginx worker 缓存旧 IP 未重解析。

**修复**：force-recreate nginx 容器。此后部署模式固定为重建 `rearview-server` + `dagster-webserver` 镜像并 force-recreate `dagster-webserver dagster-daemon rearview-server rearview-portfolio-worker nginx` 五个容器。

### 事故三：archived portfolio 触发 410 Gone（跨边界双缺陷）

v4 ensure 成功后 settlement-target 返回 410 `strategy portfolio archived`。

**根因（双缺陷）**：

1. `get_strategy_portfolio_by_example_case` 查询未过滤 `status='active'`，命中已 archived 的旧失败 v4。
2. `strategy_portfolio` 两个 partial unique index 未限定 `status='active'`，archived 行占住唯一键，无法为同一 `(case_id, version)` 创建新 active 组合。

**修复（跨边界一次到位）**：

- Rust 查询 WHERE 追加 `and status = 'active'`。
- Alembic 迁移 `0011_strategy_portfolio_active_uniqueness.py`：两个 partial unique index 重建为带 `WHERE status = 'active'`。
- `check_schema_readiness` 期望版本更新为 `0011_active_unique_idx`。

### 事故四：验证期手动 ensure 误创建 example portfolio

部署验证阶段通过手动 `curl POST /ensure` 确认修复，违反了 example portfolio 生命周期约束——`ensure` 是创建入口，验证期调用即触发创建。误产生 v3×1 + v4×3 共 4 个 portfolio，经用户确认后事务内删除（删除前已查 `pg_constraint` 验证级联链）。

**教训**：部署验证必须严格只读——容器健康、`EXAMPLE_0051_VERSION` 打印、DB head 查询均只读；**不要**对 `/ensure` 发 POST。需要验证 ensure 路径时由 `example__portfolio_live_job` 真正运行覆盖。

---

## 多 active portfolio 处理

`daily__portfolio_nav_liquidation` 不带 `strategy_portfolio_id` 时调用 `list_active_strategy_portfolios()`（`postgres/mod.rs:1514-1539`，无 LIMIT），遍历 ALL active portfolio 同时结算（`:1580-1591`）。因此每次版本切换部署后，必须归档上一个 active portfolio（通过 `PATCH /rearview/strategy-portfolios/{id}` 设 `status=archived`），否则会出现双 active 一次 daily run 结算两个。v5→v6→v7→v8 每次部署后均执行了归档。

---

## 修改文件（累计）

引擎（仅 v5 引入 `cross_below_metric`，其余版本无引擎变更）：

- `engines/crates/rearview-core/src/strategy_backtest.rs`：operator 校验放宽。
- `engines/crates/rearview-core/src/portfolio/mod.rs`：`ExitRule::IndicatorStopLoss` 增加 `operator` 字段；下穿求值逻辑；`TradeCalendarPlan` prev 支持。
- `engines/crates/rearview-portfolio-worker/src/main.rs`：config→runtime 转换携带 operator。

Fixture 与 version（每次迭代）：

- `engines/crates/rearview-core/src/examples.rs`：`RACINGLINE_0051_LOW_REVERSAL_VERSION`（当前 `v8`）；`exit_rules`（当前 v3 两条规则）；`racingline_0051_hashes_should_match_strategy_search_report` 测试断言 `execution_config_hash = 7c28160c...`。
- `pipeline/scheduler/src/scheduler/defs/rearview/assets.py`：`EXAMPLE_0051_VERSION`（当前 `v8`）。
- `pipeline/scheduler/tests/unit/rearview/test_rearview_assets.py`：4 处 ensure response version 断言（当前 `v8`）。

数据库迁移（v4 部署事故修复）：

- `pipeline/migrate/versions/rearview/0011_strategy_portfolio_active_uniqueness.py`：partial unique index 限定 `status='active'`。

---

## 验证（v8 最终态）

```bash
cd engines
cargo fmt
cargo test -p rearview-core
cargo clippy --workspace --all-targets --all-features -- -D warnings

cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler/defs/rearview/assets.py scheduler/tests/unit/rearview/test_rearview_assets.py
uv run pytest scheduler/tests/unit/rearview/test_rearview_assets.py
cd scheduler && uv run dg check defs

cd /storage/program/fleur
make docs-check && git diff --check
```

结果：

- `cargo test -p rearview-core`：187 passed（hash 测试验证 `7c28160c...` = v3）。
- `cargo clippy`：passed。
- ruff / pyright / pytest(16) / dg check / docs-check / git diff --check：passed。
- 运行时 `EXAMPLE_0051_VERSION` → `v8`；DB head `0011_active_unique_idx`；容器全部 healthy；active example portfolio = 0（v7 已归档，待 job 创建 v8）。

---

## 不做

- 不改 `release-manifest.yml`（追踪组件 SemVer，不追踪 example fixture 内部版本号）。
- 不删除 archived 历史 portfolio（v2/v4/v5/v6/v7 保留作收益对照）。
- 不执行 `dg launch --job example__portfolio_live_job` 或 `POST /ensure` 做验证（只由 job 真正运行创建 v8 portfolio）。
- 不改 buy-side 的 `CrossesBelow`（独立路径，仅作语义参照）。
- 不改生产 `daily__portfolio_nav_liquidation`（独立代码路径）。

---

## 关联文档

- Issue：[#1 — 用例策略缺少获利止盈条件导致收益过低](https://github.com/WackyGem/Fleur/issues/1)
- Issue 记录：`docs/issues/0001-example-portfolio-live-job-missing-take-profit-2026-07-03.md`
- v2 置换背景：`docs/jobs/reports/2026-07-03-example-portfolio-live-job-strategy-search-replacement.md`
- v2 部署：`docs/jobs/reports/2026-07-03-prod-docker-example-fixture-v2-deploy.md`
- Rearview 架构事实：`docs/architecture/rearview.md`
