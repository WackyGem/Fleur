# 2026-07-02 Racingline 0051 Low Reversal Example Live Job

日期：2026-07-02

范围：Rearview example ensure API、`example__portfolio_live_job`、portfolio worker live 清算、0051 低位反转空仓建仓与后续 T+1 买入验收。

## 结论

0051 低位反转已固化为 `racingline_0051_low_reversal` / `v1` example portfolio。组合通过 Rearview example ensure API 幂等创建或复用，Dagster 只通过手动 `example__portfolio_live_job` 触发正式 daily-run range API，worker 走正式 live 清算路径写入 `fleur_portfolio.live_*` facts。

本轮验证确认：

- `live_start_date = 2024-01-02` 固定不漂移。
- `2024-01-02` 无可执行买入信号时，job 成功，产出现金空仓 NAV。
- 扩展清算窗口到 `2024-01-12` 后，`2024-01-11` 信号在 `2024-01-12` T+1 买入成交。
- example job 不挂 schedule，用户是否执行只由是否显式 launch `example__portfolio_live_job` 控制。

Review 后修正：最初实现把未配置 `end_date` 的默认窗口设为 `2024-01-02..2024-01-02`，这只适合作为“无初始信号也能建仓”的最小 smoke。当前实现已改为：未显式传 `end_date` 时，`example__portfolio_live_job` 先调用 portfolio-specific settlement target，然后只创建一个 `trade_date = latest settlement target` 的 daily run。worker 对这个 run 使用持久化 portfolio 的建仓上下文执行 full-window simulation，一次性产出从 `live_start_date = 2024-01-02` 到 latest 的 live facts。需要复现单日 smoke 时，应显式传 `end_date = "2024-01-02"`。

## 环境

- Rearview server：`127.0.0.1:34057`
- PostgreSQL：`fleur-postgres`
- ClickHouse：`fleur-clickhouse`
- Worker：`rearview-portfolio-worker run`
- Scheduler 命令目录：`pipeline/scheduler`

## Example Portfolio

`POST /rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure` 返回：

| 字段 | 值 |
|---|---|
| case_id | `racingline_0051_low_reversal` |
| version | `v1` |
| fixture_hash | `81c2e68b371114164e642ede55b9cc4f2f632377b68d0ed8e0bb04917d3ab4eb` |
| strategy_portfolio_id | `01KWGR9KZX1H8K0MY8Y7J3EXHH` |
| portfolio_code | `SP-20260702-W9X5R` |
| rule_hash | `67246d58dd15ff09b80f3078863fe15a415bf13102776851625888f2438aeaac` |
| execution_config_hash | `102b8131dd87a48ffde870e30327288395a6714e51c84dea20dd229b0353ab70` |
| initial_signal_date | `2023-12-29` |
| live_start_date | `2024-01-02` |
| created | `false` |

PostgreSQL `strategy_portfolio` 中仅有 1 条 `source_kind='example'` 记录，对应 `example_case_id='racingline_0051_low_reversal'`、`example_version='v1'` 和上述三个 hash。

## Smoke 1: 初始单日建仓 smoke

命令：

```bash
cd pipeline/scheduler
uv run dg launch --job example__portfolio_live_job
```

结果：

- Dagster run id：`95d38684-ad49-4d0d-b601-386d4af28b55`
- Dagster 状态：`RUN_SUCCESS`
- materialized asset：`rearview/example_0051_portfolio_live_run`
- daily run id：`01KWGR9M1ZTHDCCE2PNX9W1R6K`
- result attempt id：`01KWGR9NV9JXAFH958DD5DMBKQ`
- trade_date：`2024-01-02`
- status：`succeeded`

Fact counts：

| nav_row_count | trade_row_count | closed_trade_row_count |
|---:|---:|---:|
| 1 | 0 | 0 |

Signal summary：

| 字段 | 值 |
|---|---:|
| top_n_row_count | 0 |
| signal_date_count | 0 |
| top_n_candidate_count | 0 |
| executable_signal_count | 0 |
| dropped_signal_count | 0 |

解释：`2024-01-02` 没有买入信号不是失败条件；该日产出现金空仓 NAV，现金余额、总权益和 NAV 分别为 `1000000`、`1000000`、`1`。

注意：该命令在本报告首次运行时代表默认单日行为；review 后当前默认行为已变更为清算到 latest settlement target。单日 smoke 现在需用显式 `end_date` 配置触发。

## Smoke 2: 扩展到后续信号日

命令：

```bash
cd pipeline/scheduler
uv run dg launch --job example__portfolio_live_job --config-json '{"ops":{"rearview__example_0051_portfolio_live_run":{"config":{"end_date":"2024-01-12","max_trade_dates":20}}}}'
```

结果：

- Dagster run id：`4d9b6f62-c76f-4974-83ae-7a4446ac0d44`
- Dagster 状态：`RUN_SUCCESS`
- latest daily run id：`01KWGRFQ5YJ7VAE8K5ACH13XTG`
- result attempt id：`01KWGRFSKF87021YBZNWZE8SCZ`
- trade_date：`2024-01-12`
- status：`succeeded`

Fact counts：

| nav_row_count | trade_row_count | closed_trade_row_count |
|---:|---:|---:|
| 9 | 1 | 0 |

Signal summary：

| 字段 | 值 |
|---|---:|
| top_n_row_count | 2 |
| signal_date_count | 2 |
| top_n_candidate_count | 2 |
| executable_signal_count | 1 |
| dropped_signal_count | 1 |

T+1 买入证据：

| signal_date | execution/trade_date | security_code | side | quantity | reference_price | execution_price | total_fee | reason |
|---|---|---|---|---:|---:|---:|---:|---|
| 2024-01-11 | 2024-01-12 | `688685.SH` | buy | 7100 | 27.88915094409542 | 27.91704009503951 | 21.803208314225856 | rebalance |

NAV 证据：

- `2024-01-02` 到 `2024-01-11`：现金 `1000000`、持仓市值 `0`、position_count `0`。
- `2024-01-12`：现金 `801767.2121169053`、持仓市值 `214574.05660915305`、总权益 `1016341.2687260583`、NAV `1.0163412687260582`、position_count `1`。

## Smoke 3: Latest Full-Window Run

Review 后按“单次 full-window 清算”模式重新运行默认 `example__portfolio_live_job`。该模式只创建一个 latest trade date 的 daily run，由 worker 从 `run_start_date = 2023-12-29` 计算到 `trade_date = 2026-07-01`，再从 `live_start_date = 2024-01-02` 输出 live facts。

命令：

```bash
cd pipeline/scheduler
/usr/bin/time -p uv run dg launch --job example__portfolio_live_job
```

结果：

- Dagster run id：`9679db59-eb49-4c69-a62b-1e39cf8a82ad`
- Dagster 状态：`RUN_SUCCESS`
- command wall clock：`48s`
- `/usr/bin/time real`：`47.85s`
- Dagster step duration：`20.53s`
- daily run id：`01KWGTMAJV488MK74CSYPBB073`
- result attempt id：`01KWGTMQ4P98M21QNPJFDWA7SW`
- worker started_at：`2026-07-02T07:10:29.410418Z`
- worker completed_at：`2026-07-02T07:10:41.733989Z`
- worker duration：约 `12.32s`

Fact counts：

| nav_row_count | trade_row_count | closed_trade_row_count |
|---:|---:|---:|
| 602 | 1268 | 633 |

Coverage and summary：

| 字段 | 值 |
|---|---:|
| live nav date range | `2024-01-02..2026-07-01` |
| first_trade | `2024-01-12` |
| last_trade | `2026-07-01` |
| price_bar_count | 448575 |
| price_bar_security_count | 746 |
| top_n_row_count | 882 |
| executable_signal_count | 882 |
| signal_date_count | 298 |
| total_return | -0.06319943935888961 |
| ending_equity | 936800.5606411104 |
| max_drawdown | -0.19592498578574524 |
| total_fee | 89095.63528504774 |

## 验证命令

通过：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

通过：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run ruff format --check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests --cov=scheduler/src/scheduler --cov=contract_tools/src/fleur_contracts --cov-report=term-missing
uv run pytest scheduler/tests/unit/rearview/test_rearview_assets.py
cd scheduler
uv run dg check defs
```

通过：

```bash
make docs-check
git diff --check
```

## 备注

当前 `dg launch` 版本的 inline 配置参数是 `--config-json`；`--config` 被解析为配置文件路径。
