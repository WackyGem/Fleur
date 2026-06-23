# Racingline Strategy Step 4 Draft Handoff

日期：2026-06-23

范围：

- RFC 0027 / Plan 0050 的 `/strategies` Step 4 模拟建仓实现。
- Rearview draft-only strategy backtest validation contract。
- Portfolio simulation `single_position_limit_pct` execution semantics。
- Racingline Step 4 默认市场费率模板、BacktestExecutionDraft、stale gate 和 Step 5 handoff。

## 实现摘要

Rearview:

- 新增 `POST /rearview/strategy-backtests/validate` draft-only endpoint，接收 transient `RuleVersionSpec + BacktestExecutionConfig`，返回 canonical config、`rule_hash`、`execution_config_hash` 和执行摘要。
- Endpoint 不创建 rule set、rule version、run、portfolio run，不写结果事实，也不发 NATS。
- `PortfolioSimulationInput` 和 worker `rebalance_policy` 支持 `single_position_limit_pct`；后端用 `min(equal_weight_after_cash_reserve, cap)` 计算单票目标权重，cap 留下的资金保留为现金。
- 后端 validation 拒绝第一版未支持的 indicator stop loss，并覆盖初始资金、TopN、max positions、单票上限、费用和滑点边界。

Racingline:

- `app/racingline_new` 接入 `GET /rearview/market-fee-templates/default?market=CN_A_SHARE`，默认费用和滑点来自 Rearview 模板。
- 新增 strategy execution adapter，把 UI 百分比转换为后端 decimal/bps snake_case payload，并构造 validate request 和 Step 5 request draft。
- Step 4 使用后端返回的 `BacktestExecutionDraft` 展示 canonical hash、Preview 上下文、TopN、最大持仓、单票目标、现金保留、费用和退出规则摘要。
- Step 4/5 gate 要求非 stale Step 3 applied preview、模板加载成功、后端 validate 成功；Step 1/2 改动后不能沿用旧草稿。
- Step 5 只消费 draft、周期和 benchmark，展示 request boundary；真实回测按钮保持 disabled，不展示静态净值、持仓或绩效样例。

## 已执行检查

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：通过。

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

结果：通过。`npm run build` 仅保留 Vite chunk size warning。

```bash
make docs-check
git diff --check
```

结果：通过。

## 浏览器验收

环境：

- Rearview: `http://127.0.0.1:34057`
- Racingline new: `http://127.0.0.1:5173/strategies`
- Playwright CDP: `http://127.0.0.1:9222`

主路径步骤：

1. 进入 `/strategies`。
2. Step 1 创建默认筛选条件。
3. Step 2 新增默认权重并点击「股池预览」。
4. Step 3 等待 timeline、preview、security analysis 和 backtest validate 请求完成。
5. 进入 Step 4，检查 Rearview 草稿摘要。
6. 点击「进入回测」进入 Step 5。

主路径观察：

- Network 可见 `GET /rearview/market-fee-templates/default?market=CN_A_SHARE => 200`。
- Network 可见 `POST /rearview/strategy-backtests/validate => 200`。
- Step 4 显示 `Rearview 回测草稿`、`Draft ready`、Preview ID、Preview 区间、草稿 Hash、Top 10、最大持仓 10 只、单票目标 10%、现金保留 0%。
- Step 4 没有近三月票池数或本地趋势图。
- 指标止损 checkbox 为 disabled，并提示 Rearview 当前只开放固定止损、固定止盈和时间止损。
- 「进入回测」只在 draft ready 后启用。
- Step 5 显示回测区间、benchmark、TopN、Rule hash、Execution config hash、Price basis 和 Signal timing。
- Step 5 的「真实回测待接入」和「运行策略待接入」均为 disabled；不显示静态净值、持仓或绩效样例。
- `playwright-cli console` 除 React DevTools info 外无错误。

Stale gate:

- 在成功生成 Step 5 draft 后回到 Step 1，点击「添加指标」修改规则草稿。
- 再点击侧栏「模拟建仓」停留在 Step 1，不进入 Step 4。
- Network 没有新增 `POST /rearview/strategy-backtests/validate`。
- 代码 gate 为 `previewSnapshot && !previewSnapshot.stale`，Step 5 还要求成功的后端 draft、模板无 loading/error。

Template error path:

- 用 Playwright route 拦截 `**/rearview/market-fee-templates/default*` 返回 HTTP 500。
- 重载后重新执行 Step 1/2/3 并进入 Step 4。
- Network 可见模板请求两次 `500 Internal Server Error`，预览链路仍为 200，但没有新的 validate 请求。
- Step 4 显示“默认市场费率加载失败”和后端错误消息 `forced template failure`。
- Step 4 摘要显示“默认费率不可用”，草稿 Hash、单票目标、单票金额和现金保留均为“待校验”。
- 「进入回测」为 disabled。

## 文档收尾

- Plan 0050 已归档到 `docs/plans/archive/`。
- `docs/plans/README.md` 已从 Active Plans 移除 Plan 0050，并加入 Recently Completed。
- `docs/systems/racingline.md`、`docs/systems/rearview.md` 和 `engines/README.md` 已补充当前 API 和职责边界。

## 保留限制

1. Step 5 真实回测执行 API、worker、benchmark 绩效和结果页不在本次范围。
2. Step 4 草稿不做刷新恢复；需要持久化时再设计本地或服务端 draft。
3. Indicator stop loss 第一版保持禁用；后续如要开放，需要 Rearview 受控指标退出规则和后端 validation 扩展。
