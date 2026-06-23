# Racingline Strategy Step 4 Drift Remediation

日期：2026-06-23

范围：

- Debt 0006 的 `/strategies` Step 4 模拟建仓漂移修正。
- Racingline Step 4 用户摘要、费用表单、显式 validate handoff 和 Step 5 输入确认。
- Rearview strategy backtest validate contract 的 trend indicator stop loss 支持。
- Portfolio worker / engine 的受控趋势指标止损执行语义。

## 实现摘要

Racingline:

- Step 4 摘要移除 Rearview draft、Draft ready、Preview ID、Preview 状态、Preview 区间、草稿 Hash、账户币种、现金保留、佣金上限和最低佣金。
- `SimulationSettings.transactionFees` 收敛为用户可编辑的佣金率、卖出印花税、过户费和单一成交滑点。
- Adapter 保留 Rearview canonical contract 需要的 `commission_rate_max` 和 `min_commission`，但这些值来自默认市场费率模板，不进入用户表单。
- 单一 `slippageRatePercent` 提交时映射为相同 `buy_bps` 和 `sell_bps`。
- Step 4 编辑期间不再运行 live validate query；点击「进入回测」时才调用 `POST /rearview/strategy-backtests/validate` mutation，成功后进入 Step 5。
- Step 4 恢复 `近三月票池数` 折线图，数据源为 Step 3 applied preview snapshot 的 `timeline.trade_dates[*].pool_count`。该图不使用信号数、不读取 `buyTopN`，也不进入 Step 5 request 或 hash。
- Step 4 恢复趋势指标止损 UI，第一版只允许选择 trend 指标，并固定语义为“收盘价跌破所选趋势指标时卖出”。
- Step 5 保留 canonical `rule_hash`、`execution_config_hash` 和输入摘要，不展示静态净值、持仓或绩效样例。

Rearview:

- `strategy_backtest` validation 接受受控 `indicator_stop_loss` request：

```json
{
  "type": "indicator_stop_loss",
  "source": "trend",
  "metric": "price_ma_10",
  "operator": "close_below_metric"
}
```

- 后端拒绝非 `trend` source、未知 trend metric 和非 `close_below_metric` operator。
- `rearview-portfolio-worker` 不再拒绝 indicator stop loss，会转换为 portfolio engine 的退出规则。
- Portfolio price query left join `mart_stock_trend_indicator_daily`，并读取受控趋势指标字段。
- `PriceBar` 增加趋势指标和前复权收盘价字段；indicator stop loss 优先用前复权收盘价比较，缺失时回退到后复权收盘价。
- Portfolio event 增加 indicator stop loss 退出原因和指标缺失事件。

## 近三月票池数口径

Step 4 中的图表是股票池规模走势，不是买入信号数。

数据路线：

1. Step 3 点击「更新股池」后保存 applied preview snapshot。
2. Step 4 读取 `previewSnapshot.timeline.trade_dates`。
3. 按 timeline 最新交易日向前截取近三个月。
4. 每个点使用 `{ label: trade_date, count: pool_count }`。

前端 helper：

- `app/racingline_new/src/features/strategy/pool-count-trend.ts`
- `app/racingline_new/src/features/strategy/pool-count-trend.test.ts`

已补单测确认：

- 使用最近三个月 Step 3 timeline 的 `pool_count`。
- 当 `timeline` 和 `result.trade_dates` 同时存在时，优先使用 Step 3 timeline。

## 已执行检查

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

结果：通过。`npm run build` 仅保留 Vite chunk size warning。

```bash
cd app/racingline_new
npm test -- --run src/features/strategy/pool-count-trend.test.ts src/features/strategy/execution.test.ts
```

结果：通过，2 个 test files、10 个 tests 通过。

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：通过。

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
2. Step 1 使用默认筛选条件。
3. Step 2 新增默认权重并点击「股池预览」。
4. Step 3 生成 applied preview snapshot。
5. 进入 Step 4，修改指标止损和其他建仓参数。
6. 点击「进入回测」进入 Step 5。

观察：

- Step 4 看不到 `Rearview 回测草稿`、`Draft ready`、`草稿 Hash` 和 `Preview 状态`。
- Step 4 显示 `近三月票池数` 和折线图。
- Step 4 交易费率区只有佣金率、卖出印花税、过户费和单一 `成交滑点`。
- Step 4 可启用 `指标止损`，并只能选择趋势指标。
- 修改指标止损时没有触发 `POST /rearview/strategy-backtests/validate`。
- 点击「进入回测」后只出现一次 `POST /rearview/strategy-backtests/validate => 200`。
- validate 成功后进入 Step 5。
- Step 5 显示 canonical Rule hash 和 Execution config hash。
- Console 无 error/warning，仅有 React DevTools info。

## 保留限制

1. 本次仍不实现真实回测执行、benchmark 绩效、回测结果页或历史结果持久化。
2. `useStrategyBacktestValidateQuery` 作为可复用 API hook 暂时保留，但 Step 4 不再使用 live query。
3. 指标止损第一版只支持受控 trend metric，不支持任意公式、任意 SQL、任意 mart 或自定义 operator。
