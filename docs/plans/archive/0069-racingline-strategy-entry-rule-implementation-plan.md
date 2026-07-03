# Plan 0069: Racingline 最近信号建仓日期与空位补仓规则实施计划

日期：2026-07-02

状态：Completed

完成日期：2026-07-02

领域：racingline, rearview

关联系统：racingline, rearview, rearview-portfolio-worker

代码根：

- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`

关联文档：

- [RFC 0041: Racingline 最近信号建仓日期与空位补仓规则](../../RFC/archive/0041-racingline-strategy-backtest-entry-rule-baseline.md)
- [RFC 0034: Racingline Step 5 建立策略组合弹层分 Tab 信息架构](../../RFC/archive/0034-racingline-step5-portfolio-publish-dialog-tabs.md)
- [Plan 0060: Racingline Step 5 建立组合弹层与 T+1 建仓语义实施计划](0060-racingline-step5-portfolio-publish-dialog-tplus1-plan.md)
- [验收报告：2026-07-02 Racingline 最近信号建仓日期与空位补仓规则实施](../../jobs/reports/2026-07-02-racingline-strategy-entry-rule-implementation.md)
- [Racingline 系统地图](../../architecture/racingline.md)
- [Rearview 系统地图](../../architecture/rearview.md)

## 背景

RFC 0041 已接受两个建仓规则决策：

1. 调整最近信号建仓日期规则：发布组合时，如果最后信号日早于服务端当前日期，说明行情或 mart 数据滞后，不允许发布组合，不生成 pending 首仓信号，也不把首个建仓日顺延到未来交易日。
2. 保持当前空位补仓执行规则：回测和首仓继续采用“每日候选信号 Top N + 仅空位调入 + 旧持仓由风控退出”的模型，不改为每日 Top N 目标持仓再平衡。

当前实现缺口集中在发布预检和解释口径：

- Rearview publish preview 使用 `source_run.end_date` 作为 `source_signal_date`，再解析 `next_trade_date_after(source_signal_date)`，没有校验信号日是否早于服务端当前日期。
- create API 只校验前端提交的 expected 日期与重新计算 preview 一致，没有二次阻断 stale signal。
- Racingline UI 中“买入信号 Top N”容易被理解成每日目标持仓集合；实际模型是每日候选信号和空位补仓。

## 目标

1. Rearview publish preview 返回权威的 `server_current_date`、`can_publish` 和 `blockers`。
2. 当 `source_signal_date < server_current_date` 时，publish preview 阻止确认，不返回可创建的首仓执行计划。
3. 当 `source_signal_date == server_current_date` 时，允许发布，首个建仓日继续为下一交易日。
4. 当 `source_signal_date > server_current_date` 时，阻止发布并提示日期异常。
5. create API 重新执行相同 stale 校验，不能只信任前端 expected 日期。
6. Racingline 发布弹层展示“最后信号日”“当前日期”“计划建仓日”和 blocker。
7. Step 4/Step 5 把 Top N 命名和解释调整为“每日候选信号 Top N”，明确旧持仓由风控退出。
8. 保持现有 portfolio simulation、worker TopN materialization、T+1 execution 和空位补仓算法不变。

## 非目标

1. 不实现每日 Top N 目标持仓再平衡。
2. 不引入 rank band、score 加权、现金再分配或自动加仓。
3. 不修改 ClickHouse portfolio facts、dbt model 或 Dagster 清算链路。
4. 不改变 `buy_signal_top_n`、`max_positions`、`signal_timing`、`target_weighting` 的 execution config 语义。
5. 不让前端自行计算交易日历、服务端当前日期或是否 stale。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| Step 4 execution config | `app/racingline/src/features/strategy/execution.ts` 固化 `signal_timing = "close_confirm_next_open"`、`target_weighting = "equal_weight_capped"`、`empty_signal_action = "hold"`、`price_basis = "backward_adjusted"`。 |
| Step 4 UI 文案 | `app/racingline/src/features/strategy/components/simulation-position-panel.tsx` 当前展示“买入信号 Top N”和“仓位空余按信号调入”。 |
| 后端 config 校验 | `engines/crates/rearview-core/src/strategy_backtest.rs` 只允许当前建仓执行模型，并计算单票目标权重。 |
| 信号 SQL | `engines/crates/rearview-core/src/planner/sql.rs` 对每日股票池按 `score DESC, security_code ASC` 排名，并过滤 `signal_rank <= buy_signal_top_n`。 |
| Worker T+1 映射 | `engines/crates/rearview-portfolio-worker/src/main.rs` 把 T 日信号映射到下一交易日执行，回测尾日无下一交易日或超过 end date 时丢弃。 |
| 模拟器补仓 | `engines/crates/rearview-core/src/portfolio/mod.rs` 每日先卖出风险退出，再按 rank 买入空位；旧持仓不会因跌出 TopN 被卖出。 |
| 发布预检 | `engines/crates/rearview-core/src/api/mod.rs` 当前 `source_signal_date = source_run.end_date`，`planned_live_start_date = next_trade_date_after(source_signal_date)`，没有 stale signal gate。 |

## 目标规则

```text
source_signal_date = source_run.end_date
server_current_date = Rearview 按市场日期解析的服务端当前日期

if source_signal_date < server_current_date:
  can_publish = false
  blockers += "行情数据未更新，最后信号日早于当前日期；请先更新行情并重跑回测"
else if source_signal_date == server_current_date:
  can_publish = true
  planned_live_start_date = next_trade_date_after(source_signal_date)
else:
  can_publish = false
  blockers += "最后信号日晚于当前日期，请检查行情日期或系统时间"
```

`server_current_date` 必须由 Rearview 返回，Racingline 只能展示和消费该结果。第一版使用后端服务时钟解析为 CN A Share 市场日期；如实施时发现代码中已有 clock/date provider，应复用既有 provider，不新增前端判断。

## API Contract 草案

publish preview response 增补字段：

```json
{
  "source_signal_date": "2026-06-29",
  "server_current_date": "2026-07-02",
  "planned_live_start_date": null,
  "can_publish": false,
  "blockers": [
    "行情数据未更新，最后信号日早于当前日期；请先更新行情并重跑回测"
  ],
  "pending_buy_signals": []
}
```

允许发布时：

```json
{
  "source_signal_date": "2026-07-02",
  "server_current_date": "2026-07-02",
  "planned_live_start_date": "2026-07-03",
  "can_publish": true,
  "blockers": [],
  "pending_buy_signals": [
    {
      "signal_date": "2026-07-02",
      "execution_date": "2026-07-03",
      "security_code": "000001.SZ",
      "rank": 1,
      "score": 92.4
    }
  ]
}
```

约束：

- `can_publish = false` 时，前端不得允许确认创建组合。
- create API 必须重新计算 preview，并在 `can_publish = false` 时返回业务错误。
- `planned_live_start_date` 在 blocked 状态下可以为 `null`；如为了兼容旧前端暂时保留字段，也不得被前端用于确认发布。
- `pending_buy_signals` 在 stale blocked 状态下应为空，避免 UI 展示可执行的历史首仓信号。

## 实施阶段

### Phase 0: 日期来源和现有 contract 审计

目标：先确认 Rearview 当前日期、交易日历和 publish preview response 的唯一事实来源，避免在代码里写多来源 fallback。

实施项：

1. 梳理 `engines/crates/rearview-core/src/api/mod.rs` 中 publish preview/create 的调用链。
2. 查找 Rearview 是否已有 clock/date provider、市场时区 helper 或测试注入方式。
3. 确认 publish preview response 类型、前端类型和 create request expected 字段当前定义。
4. 明确 `server_current_date` 使用的日期来源，并在测试中可控注入。

完成标准：

1. 找到唯一的服务端当前市场日期来源。
2. publish preview/create 的 stale 判断不依赖前端传入日期。
3. 若没有可注入 clock，先新增最小 clock/date resolver，而不是在业务函数中直接散落 `now()` 调用。

### Phase 1: Rearview publish preview stale gate

目标：让 preview 成为发布能否继续的权威判断。

实施项：

1. 扩展 publish preview response 类型，新增 `server_current_date`、`can_publish`、`blockers`。
2. 在解析 `planned_live_start_date` 前执行 stale 判断。
3. `source_signal_date < server_current_date` 时返回 blocked，不生成可确认的 pending buy signals。
4. `source_signal_date == server_current_date` 时保留现有 T+1 建仓解析。
5. `source_signal_date > server_current_date` 时返回 blocked，提示日期异常。
6. 保持 `source_signal_date` 仍来自 `source_run.end_date`，不在本阶段改变 backtest end date 语义。

测试策略：

1. Rust 单测：`source_signal_date < server_current_date` 返回 `can_publish=false`、blocker 和空 pending signals。
2. Rust 单测：`source_signal_date == server_current_date` 返回 `can_publish=true` 和下一交易日。
3. Rust 单测：`source_signal_date > server_current_date` 返回 blocked。
4. Rust 单测：交易日历找不到下一交易日时仍按现有错误语义处理，不被 stale gate 掩盖。

完成标准：

1. preview response 能清楚区分 allowed 和 blocked。
2. blocked preview 不展示可执行的历史首仓信号。
3. 旧的 T+1 allowed 场景不回归。

### Phase 2: create API 二次校验

目标：防止用户打开旧弹层或绕过前端时创建 stale 组合。

实施项：

1. `create_strategy_portfolio()` 重新调用 publish preview resolver。
2. 如果重新计算的 preview `can_publish=false`，返回业务错误，不写入 portfolio。
3. 继续校验前端提交的 `expected_source_signal_date` 和 `expected_live_start_date` 与最新 preview 一致。
4. blocked 场景下错误 response 返回 blockers，方便前端复用提示。

测试策略：

1. Rust 单测：preview 曾经 allowed，但 create 时日期推进导致 stale，create 被拒绝。
2. Rust 单测：expected 日期匹配但 `can_publish=false`，create 仍被拒绝。
3. Rust 单测：同日信号 allowed 且 expected 日期匹配，create 成功。

完成标准：

1. create API 与 preview 使用同一 stale 判断。
2. 任何 stale signal 都不能创建 pending 首仓组合。
3. create 不新增空位补仓、TopN 或 simulation 行为变化。

### Phase 3: Racingline 发布弹层接入

目标：把 stale signal 解释成数据新鲜度问题，而不是建仓日可选择问题。

实施项：

1. 更新 `app/racingline/src/types/rearview.ts` 中 publish preview 类型。
2. 更新 `app/racingline/src/api/rearview.ts` 和相关 hook，透传 `server_current_date`、`can_publish`、`blockers`。
3. 发布弹层展示：
   - 最后信号日。
   - 当前日期。
   - 计划建仓日。
   - 阻断原因。
4. 当 `can_publish=false` 时禁用确认按钮。
5. 文案使用“行情数据未更新，最后信号日早于当前日期；请先更新行情并重跑回测”。

测试策略：

1. 前端组件测试：blocked preview 禁用确认按钮并展示 blocker。
2. 前端组件测试：allowed preview 展示计划建仓日并允许确认。
3. API 类型测试或编译检查：新增字段被消费，不使用本地日期自行判断 stale。

完成标准：

1. 用户不能从 UI 发布 stale signal 组合。
2. UI 明确提示问题是行情/信号日期滞后。
3. 前端不自行计算交易日历或当前市场日期。

### Phase 4: Step 4/Step 5 空位补仓命名和解释

目标：保留当前执行规则，只修正用户理解入口。

实施项：

1. Step 4 把“买入信号 Top N”改为“每日候选信号 Top N”。
2. Step 4 把“仓位空余按信号调入”改为“仅空位调入；旧持仓由风控退出”。
3. Step 4 摘要补充：Top N 是每日候选信号数量，不是每日目标持仓集合。
4. Step 5 结果页和发布弹层补充同一口径：旧持仓不会因跌出 TopN 自动卖出。
5. 保持 `simulationSettingsToBacktestExecutionConfig()` 输出不变。

测试策略：

1. 前端 execution 测试：文案调整不改变 execution config。
2. 前端组件测试：新标签和说明可见。
3. 手工检查小屏布局，确认长中文文案不挤压核心控件。

完成标准：

1. UI 不再把 TopN 暗示成目标持仓集合。
2. 空位补仓和风控退出的执行模型在 Step 4/Step 5 一致。
3. 后端 simulation、worker 和 ClickHouse 写入无行为变更。

### Phase 5: 回归和验收

目标：用自动化测试和最小手工验收证明规则变化完整生效。

实施项：

1. 构造 stale preview 样本：`source_signal_date = 2026-06-29`、`server_current_date = 2026-07-02`。
2. 构造 allowed preview 样本：`source_signal_date = server_current_date`。
3. 构造 future signal 样本：`source_signal_date > server_current_date`。
4. 回归一个已有成功发布样本，确认同日信号发布和 T+1 建仓仍可用。
5. 验证空位补仓相关 execution config hash 不因文案变更改变。

完成标准：

1. stale 样本 preview 和 create 均被阻止。
2. allowed 样本能创建组合，计划建仓日为下一交易日。
3. future signal 样本被阻止。
4. Step 4/Step 5 文案更新完成，execution config 不变。
5. 质量门禁通过。

## 验证命令

文档和格式：

```bash
make docs-check
git diff --check
```

Rust 后端：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

前端：

```bash
cd app/racingline
pnpm lint
pnpm test
pnpm build
```

端到端手工验收：

1. 打开一个最后信号日早于当前日期的回测结果，发布弹层应展示 blocker 且确认按钮不可用。
2. 更新行情并重跑回测后，最后信号日等于当前日期时，应展示下一交易日为计划建仓日且允许发布。
3. Step 4/Step 5 中 TopN 文案应明确表达每日候选信号和空位补仓。

## 风险和约束

1. 日期来源必须可测试。若直接使用系统当前日期，会导致单测随日期漂移。
2. 服务端当前日期应按 CN A Share 市场日期解释，不能由浏览器本地时区决定。
3. create API 必须重复 stale 校验，否则旧 preview 弹层仍可绕过阻断。
4. 文案变更不能改变 execution config，否则历史策略配置 hash 和回测复现会产生无关漂移。
5. blocked 状态下不应展示可执行 pending buy signals，避免用户误以为只是建仓日需要调整。

## 完成后处理

1. 实施完成后新增运行或验收报告，记录 stale、allowed 和 future signal 三类样本结果。
2. 将本计划移入 `docs/plans/archive/`，状态改为 `Completed`。
3. 在 `docs/plans/README.md` 的 Recently Completed 中添加归档入口。
4. 如 API contract 变化影响外部文档，同步更新 Racingline/Rearview 架构事实文档。
