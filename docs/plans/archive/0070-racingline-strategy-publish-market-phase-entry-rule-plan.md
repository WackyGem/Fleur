# Plan 0070: Racingline 盘中/收盘后建仓信号日期规则实施计划

日期：2026-07-02

状态：Completed

完成日期：2026-07-02

领域：racingline, rearview

关联系统：racingline, rearview

代码根：

- `app/racingline/`
- `engines/crates/rearview-core/`

关联文档：

- [RFC 0034: Racingline Step 5 建立策略组合弹层分 Tab 信息架构](../../RFC/archive/0034-racingline-step5-portfolio-publish-dialog-tabs.md)
- [Plan 0069: Racingline 最近信号建仓日期与空位补仓规则实施计划](0069-racingline-strategy-entry-rule-implementation-plan.md)
- [验收报告：2026-07-02 Racingline 最近信号建仓日期与空位补仓规则实施](../../jobs/reports/2026-07-02-racingline-strategy-entry-rule-implementation.md)
- [验收报告：2026-07-02 Racingline 盘中/收盘后建仓信号日期规则实施](../../jobs/reports/2026-07-02-racingline-strategy-publish-market-phase-entry-rule.md)
- [Racingline 系统地图](../../architecture/racingline.md)
- [Rearview 系统地图](../../architecture/rearview.md)

## 背景

Plan 0069 已实现“最后信号日早于服务端当前日期则禁止发布”的规则，并在发布弹层展示 `最后信号日`、`当前日期` 和 `计划建仓日`。这个规则能阻止多日旧行情误建仓，但对交易日白天过于严格：

- 交易日 15:00 前，当天行情尚未收盘，最新有效收盘信号天然只能来自上一交易日。
- 如果仍要求 `source_signal_date == server_current_date`，用户在白天无法基于上一交易日信号创建当天建仓组合。
- 15:00 后则应继续要求当天行情更新；否则仍展示 `待更新行情` 并禁止发布。

RFC 0034 已补充新的目标口径：发布预检按 `Asia/Shanghai` 当前时间和 A 股 15:00 cutoff 解析 `required_source_signal_date`，再要求 `source_signal_date == required_source_signal_date`。

## 目标

1. Rearview publish preview 新增交易阶段感知的 `required_source_signal_date`。
2. 交易日 15:00 前允许使用上一交易日信号发布，计划建仓日为该信号日的下一交易日，通常是当天。
3. 交易日 15:00 后要求当天信号可用；当天行情未更新时继续阻断并返回 `待更新行情` 语义。
4. 非交易日使用最近一个已完成交易日作为允许信号日。
5. 数据落后超过允许信号日时继续禁止发布，不做无限向前顺延。
6. create API 重新执行相同预检，并校验前端提交的 expected required/source/live dates，处理跨 15:00 确认的边界。
7. Racingline 发布弹层展示交易阶段、允许信号日和计划建仓日解释，禁用状态与后端 blockers 保持一致。

## 非目标

1. 不改变空位补仓规则；继续保持“每日候选信号 Top N + 仅空位调入 + 旧持仓由风控退出”。
2. 不实现每日 Top N 目标持仓再平衡。
3. 不让前端计算交易日历、市场阶段或允许信号日。
4. 不修改 backtest worker、portfolio worker、ClickHouse facts、dbt model 或 Dagster 清算链路。
5. 不支持数据多日落后时继续向前寻找可用信号。
6. 不在本计划内改交易时段日历系统，只使用固定 A 股收盘 cutoff `15:00 Asia/Shanghai`。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| Rearview publish preview | `engines/crates/rearview-core/src/api/mod.rs` 已返回 `source_signal_date`、`server_current_date`、`planned_live_start_date`、`can_publish`、`blockers` 和 `pending_buy_signals`。 |
| 当前 stale gate | `strategy_portfolio_publish_date_blocker()` 当前按 `source_signal_date < server_current_date` 阻断，按 `source_signal_date > server_current_date` 判为日期异常。 |
| 当前市场日期 | `AppState::current_market_date()` 当前按 UTC+8 解析服务端市场日期。 |
| create 二次校验 | `create_strategy_portfolio()` 已重新执行 publish preview，并校验 `expected_source_signal_date` 与 `expected_live_start_date`。 |
| 前端发布弹层 | `app/racingline/src/routes/strategy-page.tsx` 已展示最后信号日、当前日期、计划建仓日；blocked 且无计划日期时显示 `待更新行情`。 |
| 前端类型 | `app/racingline/src/types/rearview.ts` 的 publish preview response/request 当前没有 `required_source_signal_date`、`market_phase`、`publish_cutoff_time` 或 `server_current_time`。 |

## 目标规则

后端必须先解析本次发布允许信号日：

```text
market_timezone = Asia/Shanghai
publish_cutoff_time = 15:00:00
server_current_date = current market date in Asia/Shanghai
server_current_time = current wall-clock time in Asia/Shanghai

if server_current_date is trading day and server_current_time < 15:00:
  required_source_signal_date = previous_trade_date(server_current_date)
  market_phase = before_close
else if server_current_date is trading day and server_current_time >= 15:00:
  required_source_signal_date = server_current_date
  market_phase = after_close
else:
  required_source_signal_date = latest_trade_date_before_or_equal(server_current_date)
  market_phase = non_trading_day

if source_signal_date == required_source_signal_date:
  can_publish = true
  planned_live_start_date = next_trade_date(source_signal_date)
else:
  can_publish = false
  planned_live_start_date = null
```

边界约束：

- 15:00 前最多回退到上一交易日，不继续找更早日期。
- 15:00 后当天数据未更新时，`source_signal_date` 会落后于 `required_source_signal_date`，必须阻断。
- `source_signal_date > required_source_signal_date` 仍是日期异常，必须阻断。
- `planned_live_start_date` 仍由交易日历解析，不允许前端自然日 `+1`。

## API Contract 草案

publish preview response 增补字段：

```json
{
  "can_publish": true,
  "blockers": [],
  "server_current_date": "2026-07-02",
  "server_current_time": "14:30:00+08:00",
  "market_phase": "before_close",
  "publish_cutoff_time": "15:00:00+08:00",
  "required_source_signal_date": "2026-07-01",
  "source_signal_date": "2026-07-01",
  "planned_live_start_date": "2026-07-02",
  "pending_buy_signals": []
}
```

create request 增补 expected 字段：

```json
{
  "source_strategy_backtest_run_id": "01J...",
  "source_result_attempt_id": "01J...",
  "name": "策略组合",
  "expected_required_source_signal_date": "2026-07-01",
  "expected_source_signal_date": "2026-07-01",
  "expected_live_start_date": "2026-07-02"
}
```

兼容策略：

- 如果前后端必须同次发布，优先一次性升级 request/response contract。
- 如果需要灰度，后端可以先返回新增字段，但 create 接口在前端升级前不得强依赖缺失的 `expected_required_source_signal_date`；灰度窗口结束后再改为必填。
- 本仓库当前是同仓前后端联动开发，默认按一次性升级处理。

## 实施阶段

### Phase 0: 日期和交易日历事实审计

目标：确认后端能够唯一、可测试地解析当前市场日期、当前市场时间、是否交易日、上一交易日和下一交易日。

实施项：

1. 审计 `AppState::current_market_date()` 当前实现和测试注入方式。
2. 审计 Rearview 当前交易日历查询能力，确认可解析：
   - `is_trading_day(server_current_date)`
   - `previous_trade_date(server_current_date)`
   - `next_trade_date(source_signal_date)`
   - `latest_trade_date_before_or_equal(server_current_date)`
3. 确认 publish preview resolver 里不新增前端传入日期、环境变量或多来源 fallback。
4. 如果当前没有可测试的 clock/time provider，新增最小后端 clock abstraction，供单测固定 `Asia/Shanghai` 时间。

测试策略：

1. Rust 单测覆盖固定时间注入。
2. Rust 单测覆盖交易日、非交易日、交易日前一交易日解析。

完成标准：

1. `required_source_signal_date` 的输入事实来源唯一。
2. 15:00 cutoff 能在单测中固定，不依赖真实系统时间。

### Phase 1: Rearview publish preview 规则调整

目标：把当前 `source_signal_date < server_current_date` stale gate 替换为 `source_signal_date == required_source_signal_date` 校验。

实施项：

1. 新增 `MarketPhase` 或等价枚举：`before_close`、`after_close`、`non_trading_day`。
2. 新增 `resolve_required_source_signal_date()` helper，集中处理 cutoff 和交易日历。
3. 扩展 `StrategyPortfolioPublishPreviewResponse`：
   - `required_source_signal_date`
   - `server_current_time`
   - `market_phase`
   - `publish_cutoff_time`
4. 修改 publish preview blocker：
   - `source_signal_date < required_source_signal_date`：行情未达到允许信号日，提示 `待更新行情` 语义。
   - `source_signal_date > required_source_signal_date`：日期异常。
   - 相等：允许继续解析 `planned_live_start_date` 和 pending signals。
5. 保持 `pending_buy_signals` 只在 allowed preview 中生成；blocked 状态为空。

测试策略：

1. Rust 单测：交易日 14:30，`source_signal_date = previous_trade_date`，允许发布，`planned_live_start_date = server_current_date`。
2. Rust 单测：交易日 14:30，`source_signal_date` 早于上一交易日，阻断。
3. Rust 单测：交易日 15:30，`source_signal_date = previous_trade_date`，阻断并不返回 `planned_live_start_date`。
4. Rust 单测：交易日 15:30，`source_signal_date = server_current_date`，允许发布，建仓日为下一交易日。
5. Rust 单测：非交易日，`source_signal_date = latest completed trade date`，允许发布。
6. Rust 单测：`source_signal_date > required_source_signal_date`，日期异常。

完成标准：

1. publish preview 能准确表达三类市场阶段。
2. 白天建仓不再被 `server_current_date` 直接阻断。
3. 收盘后数据未更新仍被阻断。

### Phase 2: create API 二次校验

目标：确保创建组合时使用同一套交易阶段规则，并处理弹层打开后跨过 15:00 的情况。

实施项：

1. 扩展 create request，新增 `expected_required_source_signal_date`。
2. `create_strategy_portfolio()` 重新执行 publish preview 后校验：
   - `preview.required_source_signal_date == expected_required_source_signal_date`
   - `preview.source_signal_date == expected_source_signal_date`
   - `preview.planned_live_start_date == expected_live_start_date`
   - `preview.can_publish == true`
3. 如果用户 15:00 前打开弹层、15:00 后点击确定，后端返回 `409 Conflict`，提示刷新发布预检。
4. 保持 portfolio 创建字段语义不变：`initial_signal_date = source_signal_date`，`live_start_date = planned_live_start_date`。

测试策略：

1. Rust 单测：preview 14:30 allowed，但 create 时 clock 推进到 15:30 且当天数据未更新，create 返回 conflict。
2. Rust 单测：expected required date 不匹配，create 返回 conflict。
3. Rust 单测：allowed preview 且 expected 三个日期一致，create 成功。

完成标准：

1. preview 和 create 使用同一 `required_source_signal_date` resolver。
2. 跨 cutoff 的陈旧弹层不能创建组合。

### Phase 3: Racingline 类型、请求和弹层展示

目标：前端只消费后端权威结果，并把 15:00 前/后差异解释清楚。

实施项：

1. 更新 `app/racingline/src/types/rearview.ts`：
   - publish preview response 新增 `required_source_signal_date`、`server_current_time`、`market_phase`、`publish_cutoff_time`。
   - create request 新增 `expected_required_source_signal_date`。
2. 更新 `app/racingline/src/api/rearview.ts` 测试 fixture。
3. 更新 `strategy-page.tsx` 发布弹层：
   - 展示“允许信号日”或将其纳入“最后信号日”说明。
   - 15:00 前 allowed 状态标注“基于上一交易日收盘信号”。
   - 15:00 后 blocked 状态继续展示 `待更新行情`。
   - create request 带上 `expected_required_source_signal_date`。
4. 如果 create 返回 conflict，提示用户刷新发布预检，而不是继续展示旧计划建仓日。

测试策略：

1. 前端 API 单测覆盖新增字段序列化和 create request。
2. 前端组件或 route 相关测试覆盖：
   - before close allowed。
   - after close blocked。
   - conflict 错误文案。
3. 保持现有 “计划建仓日显示待更新行情” 测试不回归。

完成标准：

1. 前端没有本地计算 cutoff 或交易日历。
2. 用户能区分“盘中上一交易日信号可用”和“收盘后行情待更新”。

### Phase 4: 集成验证和文档收敛

目标：确认规则在后端、前端和文档中一致。

实施项：

1. 补充或更新 RFC 0034 中的实现状态注记。
2. 更新 Racingline/Rearview 架构事实文档中 Step 5 发布规则摘要。
3. 新增 job report，记录验证命令、关键用例和结果。
4. 完成后将本计划移入 `docs/plans/archive/`，并更新 `docs/plans/README.md`。

验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm test
npm run build
npm run lint

cd ../..
make docs-check
git diff --check
```

完成标准：

1. Rust 和前端测试覆盖 before close、after close、non trading day、multi-day stale 和 cross-cutoff create。
2. 发布弹层和后端 blockers 对同一场景给出一致结果。
3. 计划、RFC、架构事实和验收报告指向同一规则口径。

## 禁止模式

1. 禁止前端自行判断 `15:00`、上一交易日或下一交易日。
2. 禁止把 `source_signal_date < server_current_date` 继续作为唯一 stale 判断。
3. 禁止数据落后多日时继续向前寻找可用信号。
4. 禁止 blocked preview 仍返回可执行 `planned_live_start_date` 或 pending buy signals。
5. 禁止 create API 只信任前端 expected 日期而不重新预检。
6. 禁止改动空位补仓、TopN、风控退出或 live/backtest 隔离语义。

## 最小验证

本文实施完成后已运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm test
npm run build
npm run lint
npm run typecheck

cd ../..
make docs-check
git diff --check
```
