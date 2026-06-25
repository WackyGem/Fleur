# Plan 0041: Racingline 虚拟账户与组合调仓净值实施计划

日期：2026-06-16

状态：Superseded

归档说明：2026-06-25 已被后续组合数据面、strategy backtest / strategy portfolio 工作流和 [Plan 0053](0053-racingline-legacy-cleanup-and-rename-plan.md) 的前端替换覆盖。旧 `/portfolios` 前端页面随旧 `app/racingline/` 清理删除；当前 Racingline 前端事实以 `app/racingline/` 的 `/dashboard`、`/dashboard/strategies/:portfolioId` 和 `/strategies` 为准。

领域：racingline, rearview

关联系统：racingline, rearview, data-platform, deploy-ops

代码根：

- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/migrate/`
- `deploy/docker-compose.yml`

关联文档：

- [RFC 0021: Racingline 虚拟账户与组合调仓净值](../../RFC/archive/0021-racingline-virtual-account-portfolio-rebalancing.md)
- [RFC 0018: Rust Rearview 规则选股服务与 mart 指标库](../../RFC/archive/0018-rust-stock-screening-service.md)
- [RFC 0019: Racingline Rearview 前端工作台](../../RFC/archive/0019-racingline-rearview-frontend-workbench.md)
- [RFC 0020: Racingline Run Result 个股分析页](../../RFC/archive/0020-racingline-run-result-security-analysis-page.md)
- [System: Rearview](../../systems/rearview.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Deploy Ops](../../systems/deploy-ops.md)

相关规则：

- `fleur-harness`：计划、系统地图、质量门禁、验收报告和归档规则。
- `rust-patterns` / `rust-async-patterns`：crate 拆分、最小 public surface、typed domain state、Tokio worker 和异步错误处理。
- `shadcn` 和 `playwright-cli`：实施 Racingline UI 和浏览器验收时再按 AGENTS.md 路由使用。

## 目标

1. 完成 RFC 0021 第一版闭环：策略默认虚拟账户、系统默认市场费率模板、组合运行创建、NATS JetStream 分发、worker 计算净值和明细账本、Racingline 展示净值曲线。
2. 新建策略时默认创建或预填 `1,000,000 CNY` 研究型账户模板，费率和滑点从 PostgreSQL `market_fee_template` active 默认模板读取。
3. 组合净值固定使用 `backward_adjusted` 后复权口径，前端不提供切换，API 不接受组合运行级价格口径覆盖。
4. 用 `portfolio_run.account_snapshot` 和 `portfolio_run.execution_snapshot` 固化每次组合运行的账户、费率、滑点、调仓和卖出规则，历史运行不受模板后续修改影响。
5. 把当前 `rearview` 单 crate 拆成且只保留 `rearview-core`、`rearview-server` 和 `rearview-portfolio-worker` 三个 Rearview crate，server 和 worker 只依赖 core，不互相依赖。
6. 第一版必须持久化 `portfolio_target`、`portfolio_order`、`portfolio_trade` 和 `portfolio_position_day`，让净值曲线可追溯到目标组合、虚拟订单、成交和每日持仓。
7. 通过 PostgreSQL outbox + NATS JetStream 实现 at-least-once 任务分发，worker 写入幂等。
8. 完成桌面和移动端组合结果页验收，确保状态、失败、pending dispatch 和成功净值结果都可解释。

## 非目标

1. 不实现实盘交易、券商清算、真实持仓数量调整、企业行动清算或多币种资产。
2. 不实现完整回测报表、归因、benchmark 和跨策略比较；逐笔成交、订单、持仓和调仓目标必须落库并提供查询 API，UI 可以先做轻量展示。
3. 不把组合结果写入 ClickHouse；第一版 PostgreSQL 是组合运行状态和结果权威库。
4. 不在 Racingline 浏览器内计算权威成交、持仓或净值。
5. 不允许任意 SQL、Python、Rust 或前端公式作为卖出规则。
6. 不新增独立 backtester 服务；第一版 worker 是 `rearview-portfolio-worker`。
7. 不在 `rule_version` 中固化账户模板；账户和执行参数只在 `portfolio_run` 快照中固化。

## 当前事实基线

1. RFC 0021 已定义第一版范围、费率模板、滑点、卖出规则、后复权价格口径、NATS 分发和 server / worker crate 拆分方向。
2. 当前 Rearview 实现仍位于 `engines/crates/rearview/`，需要在实施中迁移为 core/server/worker 三包结构；迁移完成后不再保留可构建的 `rearview` package，避免出现第四个 Rearview 入口。
3. `pipeline/migrate/versions/rearview/0002_create_rearview_schema.py` 已管理 Rearview 当前选股 schema，组合相关 migration 应继续放在 rearview target 下。
4. `.env.example` 已包含 `REARVIEW_DATABASE_URL`、`REARVIEW_HTTP_BIND`、`VITE_REARVIEW_API_BASE_URL`、`NATS_CLIENT_PORT` 和 `NATS_MONITOR_PORT`。
5. `deploy/docker-compose.yml` 已包含 NATS 服务并开启 JetStream，第一版不需要新增基础设施容器。
6. Racingline 已有 `/rules`、`/runs`、`/runs/:runId` 和 `/runs/:runId/securities/:securityCode`，尚无 `/portfolios` 和 `/portfolios/:portfolioRunId`。
7. Rearview 已有规则集、规则版本、run、pool、signal、analysis API 和 ClickHouse mart 查询能力，可以复用 source run 成功校验和 mart 读取配置。
8. RFC 0021 的实施口径已补齐：第一版中间交易账本必须持久化为明细表、run 级重算、NATS stream/consumer 由 Rearview 进程幂等 ensure、outbox dispatcher 先在 `rearview-server` 后台运行。

## 实施口径

第一版端到端流程固定为：

```text
创建策略/账户模板
  -> 选股 run succeeded
  -> POST /rearview/portfolio-runs
  -> PostgreSQL 同事务写 portfolio_run + portfolio_task_outbox
  -> rearview-server outbox dispatcher 发布 NATS requested 消息
  -> rearview-portfolio-worker 消费 portfolio_run_id
  -> worker 从 PostgreSQL 读取不可变快照
  -> worker 从 ClickHouse mart 读取 backward_adjusted OHLC
  -> worker 模拟目标、订单、卖出、买入、费用、滑点和持仓
  -> upsert portfolio_target/order/trade/position_day/nav + summary
  -> portfolio_run.status = succeeded 或 failed_*
  -> Racingline 轮询状态并展示净值曲线
```

关键约束：

1. `POST /rearview/portfolio-runs` 返回 `202 Accepted`，不在 HTTP 请求内计算净值。
2. NATS 消息只包含 `portfolio_run_id`、`source_run_id` 和调度 metadata，不包含账户参数、规则快照或行情数据。
3. worker 必须先检查 `portfolio_run.status`，终态 run 直接 ack；非终态 run 用行锁或等价机制推进到 `calculating_nav`。
4. 重投递采用 run 级幂等：同一 run 可删除或覆盖非终态 `portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav` 和事件后重算整段。
5. 缺失价格、现金不足缩放、跳过买入、卖出价格缺失等情况必须进入 summary warning，不得静默吞掉。
6. 第一版必须持久化明细账本，并在 Rust 单元测试中验证费用、滑点、卖出优先级、现金约束和明细到净值的汇总一致性。

明细写入幂等口径固定为 run 级结果替换：

1. worker 抢占 `portfolio_run` 后，先在同一事务或受锁保护的写入窗口内删除该 run 的旧结果行，删除范围包括 `portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav` 和 `portfolio_event` 或 summary warning。
2. worker 按不可变快照重新生成全量结果并批量插入；第一版不做日期级断点续算。
3. `portfolio_order` 和 `portfolio_trade` 必须保存 run 内稳定序号，例如 `order_seq` 和 `trade_seq`，并以 `(portfolio_run_id, order_seq)`、`(portfolio_run_id, trade_seq)` 建唯一约束；主键可以是 UUID，但重复计算后的业务序号必须稳定。
4. `portfolio_run` 只有在所有明细、nav 和 summary 写入成功后才能进入 `succeeded`；结果写入失败但仍能记录终态时进入 `failed_write` 并 ack NATS message，连终态都无法写入时不 ack，依赖 JetStream 重投递。

## 实施阶段

### Phase 0: 冻结契约和测试样本

目标：在改代码前把第一版字段、状态机、价格口径和验收样本固化，避免前后端、worker 和 migration 并行时语义漂移。

任务：

1. 从 RFC 0021 提炼 Rust domain type 和 TypeScript type 清单，至少覆盖 `MarketFeeTemplate`、`VirtualAccountTemplate`、`PortfolioRun`、`PortfolioNavPoint`、`PortfolioTarget`、`PortfolioOrder`、`PortfolioTrade`、`PortfolioPositionDay`、fee/slippage/rebalance/risk exit policy。
2. 固定 enum：
   - `market = CN_A_SHARE`
   - `price_basis = backward_adjusted`
   - `portfolio_run.status`
   - `dispatch_status`
   - `target_weighting`
   - `empty_signal_action`
   - `exit_rule.type`
3. 固定第一版调仓规则：
   - 只支持 TOPN 等权建仓。
   - 从第一个出现买入信号的信号日开始进入建仓尝试；如果该日所有候选都不可成交，则不建仓并等待后续信号日。
   - 实际建仓只由第一个可成交买入信号触发；可成交必须同时满足成交日有后复权开盘价、取整后不少于 1 手、现金足够覆盖成交金额和费用。
   - 已持仓股票不重复买入或加仓，同一成交日已经生成买入订单的股票也不重复下单。
   - 止盈、止损或清仓卖出后产生空闲仓位，才按后续买入信号 rank 顺序递补。
   - 买入数量按 `lot_size = 100` 向下取整，且每笔至少成交 1 手。
   - 价格缺失、目标金额不足 1 手或资金不足买入 1 手时跳过该候选并记录 warning/event；被跳过候选不占用 TOPN 空闲槽位，继续检查下一个 rank 候选。
   - 建仓不要求一次填满 TOPN；每次只对实际可成交候选下单，剩余空闲仓位保留到后续信号日继续按 rank 顺序递补。
4. 固定 API contract，确认 `POST /rearview/portfolio-runs` 不接受 `price_basis`。
5. 选择一个本地已成功的 Rearview run 作为 smoke 样本；如果不存在，实施阶段先创建一个短区间 run 并记录到 job report。
6. 准备最小 deterministic 计算夹具：
   - 3 至 5 个交易日。
   - 至少 2 只证券。
   - 含目标组合、订单、成交、持仓、首次信号日尝试建仓、首个可成交候选触发实际建仓、已持仓不重复买入、止盈或止损卖出后空闲仓位递补、资金不足 1 手跳过、缺失价格 warning。
7. 确认 `indicator_stop_loss` 第一版只允许 metric catalog 或 allowlist 中可解析字段；缺少后复权兼容指标时返回 validation error。
8. 确认首条 `portfolio_nav` 的日期口径：使用组合运行起始交易日，净值为 `1.0`，现金为 `initial_cash`，持仓为 `0`。

完成标准：

1. RFC 0021、本计划、Rust type 草案和前端 type 草案没有字段命名冲突。
2. 测试夹具能人工计算预期净值、费用、目标、订单、成交、持仓、summary、空闲仓位递补路径和 warning。
3. 选股 run smoke 样本、日期区间和证券代码已记录到实施报告草稿或 PR 描述。

验证命令：

```bash
make docs-check
git diff --check
```

### Phase 1: PostgreSQL migration 和默认模板初始化

目标：先建立权威状态表和默认费率数据，让 API、worker 和前端都从同一事实读取配置。

任务：

1. 在 `pipeline/migrate/versions/rearview/` 新增 migration。
2. 创建第一版必需表：
   - `market_fee_template`
   - `virtual_account_template`
   - `portfolio_run`
   - `portfolio_task_outbox`
   - `portfolio_target`
   - `portfolio_order`
   - `portfolio_trade`
   - `portfolio_position_day`
   - `portfolio_nav`
3. 建议新增 `portfolio_event`；如果不新增，warning 必须以结构化数组写入 `portfolio_run.summary`，并能通过 events API 投影出来。
4. 增加约束：
   - 同一 `market` 最多一个 active default `market_fee_template`。
   - 每个 `rule_set` 至少一个 active 默认账户模板的约束可由 service 层维护，数据库至少要支持查询默认模板。
   - `portfolio_target` 主键或唯一键覆盖 `(portfolio_run_id, signal_date, security_code)`。
   - `portfolio_order` 必须有 `order_seq`，唯一键覆盖 `(portfolio_run_id, order_seq)`，并保存 `portfolio_order_id` 作为主键或外部引用 ID。
   - `portfolio_trade` 必须有 `trade_seq`，唯一键覆盖 `(portfolio_run_id, trade_seq)`，并保存费用拆分、滑点成本和成交原因。
   - `portfolio_position_day` 主键为 `(portfolio_run_id, trade_date, security_code)`。
   - `portfolio_nav` 主键为 `(portfolio_run_id, trade_date)`。
   - `portfolio_task_outbox.portfolio_run_id` 第一版唯一。
   - 所有组合结果明细表必须能按 `portfolio_run_id` 高效删除和查询，以支持 run 级结果替换。
5. 初始化 `CN_A_SHARE` active 默认市场模板，值必须符合 RFC：
   - 佣金 `0.0001`
   - 佣金上限 `0.003`
   - 最低佣金 `5`
   - 卖出印花税 `0.0005`
   - 过户费 `0.00001`
   - 买入/卖出滑点 `10 bps`
6. 为 `portfolio_run.status`、`dispatch_status`、`created_at`、`updated_at` 和列表筛选字段建立必要索引。
7. 在 migration 或后端 schema 中保留 `price_basis = backward_adjusted` 快照字段，便于历史审计。

完成标准：

1. 全新 rearview database 执行 migration 后，默认模板存在且只有一个 active default。
2. 重复执行 migration 不产生重复默认模板。
3. 表结构支持 RFC 0021 第一版所有 API，包括 targets、orders、trades、positions、nav 和 events。

验证命令：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
```

### Phase 2: Rearview crate 拆分和共享核心

目标：把现有单 crate 迁移为 server/worker 共享 core 的结构，为 NATS worker 和 HTTP server 分进程部署建立边界，并消除旧 `rearview` package 作为长期入口。

任务：

1. 在 `engines/Cargo.toml` workspace 中新增：
   - `crates/rearview-core`
   - `crates/rearview-server`
   - `crates/rearview-portfolio-worker`
2. 将现有 `engines/crates/rearview/src` 中可共享的 domain、config、error、PostgreSQL repository、ClickHouse client、catalog 和 service 逻辑迁入 `rearview-core`。
3. 将 Axum route、HTTP request/response、server main 迁入 `rearview-server`；旧 `engines/crates/rearview/` 目录只能作为迁移过程中的临时来源，不能在 Phase 2 完成后继续作为 workspace package。
4. 新增 worker main，先支持 `rearview-portfolio-worker --help` 和配置加载。
5. 更新命令入口：
   - `cargo run -p rearview-server -- serve`
   - `cargo run -p rearview-server -- catalog check`
   - `cargo run -p rearview-server -- catalog sync`
   - `cargo run -p rearview-portfolio-worker -- run`
6. 更新 `Makefile`、`make rearview-dev`、`make racingline-dev` 和文档命令，统一使用 `rearview-server`；不再接受 `cargo run -p rearview -- serve` 作为完成后的入口。
7. 保持依赖方向：server -> core，worker -> core，禁止 server <-> worker。
8. `rearview-core` 使用 typed errors、newtype ID、domain enum 和最小 `pub` surface；业务状态匹配不使用 `_` 通配吞掉未来状态。

完成标准：

1. `cargo metadata` 能看到且只看到三个 Rearview workspace package：`rearview-core`、`rearview-server`、`rearview-portfolio-worker`。
2. 原有 Rearview API 测试和 catalog 命令在新 server 包下通过。
3. `rearview-portfolio-worker --help` 可运行但尚不要求消费任务。
4. `Makefile` 和系统地图中的 Rearview 启动命令不再引用旧 `rearview` package。

验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### Phase 3: 组合净值纯计算内核

目标：先在无 PostgreSQL、无 NATS 的纯 Rust 计算层验证净值算法，降低后续基础设施联调成本。

任务：

1. 在 `rearview-core` 增加 portfolio domain：
   - account snapshot
   - execution snapshot
   - fee profile
   - slippage profile
   - rebalance policy
   - risk exit policy
   - target ledger
   - order ledger
   - trade ledger
   - position ledger
   - nav point
   - warning/event model
2. 实现 fee calculator：
   - 买卖双边佣金。
   - 卖出单边印花税。
   - 买卖双边过户费。
   - 每笔最低佣金。
3. 实现 slippage calculator：
   - 买入 `open_price_backward_adj * (1 + buy_slippage)`。
   - 卖出 `open_price_backward_adj * (1 - sell_slippage)`。
4. 实现目标组合生成：
   - 按 `rank` 读取 buy signal。
   - 从首个出现买入信号的信号日开始尝试建仓；首个可成交买入候选触发实际建仓。
   - 已持仓股票和当日已生成买入订单的股票不重复买入。
   - 第一版只实现 TOPN `equal_weight`。
   - `max_positions` 和 `cash_reserve_pct`。
   - 只有卖出后出现空闲仓位才按后续信号 rank 顺序递补买入。
   - 满仓时不因新信号 rank 更高而主动换仓。
   - 每个新买入槽位目标金额为 `total_equity_after_sells * (1 - cash_reserve_pct) / max_positions`。
   - 只有通过价格、手数和现金检查的候选才占用空闲槽位；跳过候选后继续检查下一个 rank。
   - 初始建仓或递补建仓都不要求一次填满 TOPN，未成交候选和剩余空闲槽位保留给后续信号日。
5. 实现卖出规则：
   - 固定止损。
   - 指标止损校验和触发。
   - 止盈。
   - 时间止损。
   - 同日同证券只生成一条卖出，按 RFC 优先级记录原因。
6. 实现现金约束：
   - 同日先卖后买。
   - 买入按 rank 顺序逐个检查，不做多标的比例缩放。
   - 买入数量按 `lot_size = 100` 向下取整，每笔至少 1 手。
   - 价格缺失、目标金额不足 1 手或现金不足 1 手时跳过候选并记录 warning/event，且不占用空闲槽位。
   - 买入后现金不得为负。
7. 实现 NAV 计算：
   - 首条 nav 为 `1.0`。
   - 每日现金、持仓市值、总资产、日收益、回撤。
   - summary 中包含累计收益、年化收益、最大回撤、波动率、换手、交易笔数、费用、滑点成本和 warning count。
8. 计算输出必须同时包含：
   - `portfolio_target` rows。
   - `portfolio_order` rows。
   - `portfolio_trade` rows。
   - `portfolio_position_day` rows。
   - `portfolio_nav` rows。
   - warning/event rows 或 summary warning。
9. 缺失价格处理按 RFC 写 warning，不静默跳过。

完成标准：

1. 纯计算单元测试覆盖费用、滑点、首次建仓、已持仓不重复买入、卖出后空闲仓位按 rank 递补、满仓不主动换仓、1 手最小成交、资金不足跳过、卖出优先级、缺失价格、明细账本和 nav 汇总一致性、run 级重算输入一致性。
2. 夹具预期净值由测试断言固定，后续优化不得改变结果而不更新 RFC 或计划。
3. 计算内核不依赖 Axum、NATS 或数据库连接，但输出结构必须可以直接映射到持久化明细表。

验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy -p rearview-core --all-targets --all-features -- -D warnings
cargo test -p rearview-core
```

### Phase 4: Rearview API、repository 和 outbox

目标：完成账户模板、组合运行创建、状态查询、净值查询和明细账本查询 API，但仍可用 fake dispatcher 或未启动 worker 做联调。

任务：

1. 增加 repository 方法：
   - 查询默认市场费率模板。
   - 查询、创建、更新账户模板。
   - 创建 portfolio run 和 outbox。
   - 查询 portfolio run 列表和详情。
   - 查询 portfolio nav。
   - 查询 portfolio target。
   - 查询 portfolio order。
   - 查询 portfolio trade。
   - 查询 portfolio position day。
   - 查询 portfolio event 或 summary warning projection。
   - 更新 dispatch status 和 run status。
2. 增加 HTTP API：
   - `GET /rearview/market-fee-templates/default?market=CN_A_SHARE`
   - `GET /rearview/rule-sets/{rule_set_id}/account-templates`
   - `POST /rearview/rule-sets/{rule_set_id}/account-templates`
   - `PATCH /rearview/account-templates/{account_template_id}`
   - `POST /rearview/portfolio-runs`
   - `GET /rearview/portfolio-runs`
   - `GET /rearview/portfolio-runs/{portfolio_run_id}`
   - `GET /rearview/portfolio-runs/{portfolio_run_id}/nav`
   - `GET /rearview/portfolio-runs/{portfolio_run_id}/targets`
   - `GET /rearview/portfolio-runs/{portfolio_run_id}/orders`
   - `GET /rearview/portfolio-runs/{portfolio_run_id}/trades`
   - `GET /rearview/portfolio-runs/{portfolio_run_id}/positions`
   - `GET /rearview/portfolio-runs/{portfolio_run_id}/events`
3. 明细 API 第一版必须支持最小查询控制：
   - `targets` 支持 `signal_date`、`limit`、`offset`。
   - `orders` 和 `trades` 支持 `execution_date` / `trade_date`、`security_code`、`limit`、`offset`。
   - `positions` 支持 `trade_date`、`security_code`、`limit`、`offset`，未传 `trade_date` 时默认返回最新持仓日。
   - `events` 支持 `trade_date`、`event_type`、`limit`、`offset`。
4. `POST /rearview/portfolio-runs` 必须校验 source run 已成功。
5. 创建组合运行时复制账户和执行参数到不可变快照，并写入 `price_basis = backward_adjusted`。
6. 在同一个 PostgreSQL 事务中写入 `portfolio_run` 和 `portfolio_task_outbox`。
7. 如果 NATS 暂不可用，API 仍可返回 `202 Accepted` 和 `dispatch_status = pending`，由 outbox dispatcher 后续重试。
8. 错误响应继续使用 Rearview 统一错误结构，字段级 validation error 必须包含 `field_path`。

完成标准：

1. 前端可以不依赖 worker 完成账户模板表单预填、保存和组合运行创建。
2. outbox 记录和 portfolio run 状态可通过 API 查到。
3. 历史 `portfolio_run` 快照不随账户模板更新而变化。
4. 明细查询 API 对未完成 run 返回空列表或 partial 标记，不返回未实现。
5. 明细查询 API 默认分页，不会一次性返回多年全量明细。

验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy -p rearview-server --all-targets --all-features -- -D warnings
cargo test -p rearview-core -p rearview-server
```

本地 smoke：

```bash
make rearview-dev
curl -fsS "http://127.0.0.1:34057/rearview/market-fee-templates/default?market=CN_A_SHARE" | jq .
```

### Phase 5: NATS JetStream、outbox dispatcher 和 worker

目标：完成异步任务分发和幂等消费，让组合运行能从 queued 推进到 succeeded 或 failed_*。

任务：

1. 在 `.env.example` 补充 Rearview NATS 变量，如 implementation 需要：
   - `REARVIEW_NATS_URL`
   - `REARVIEW_PORTFOLIO_STREAM`
   - `REARVIEW_PORTFOLIO_REQUEST_SUBJECT`
   - `REARVIEW_PORTFOLIO_WORKER_DURABLE`
   - `REARVIEW_PORTFOLIO_WORKER_QUEUE`
2. 在 `rearview-core` 增加 NATS config、message type 和 JetStream ensure helper。
3. `rearview-server` 启动时幂等 ensure stream，并启动进程内 outbox dispatcher。
4. outbox dispatcher 周期性发布 pending/failed outbox，成功后写 `published`、`nats_stream_sequence` 和 `portfolio_run.dispatch_status = published`。
5. `rearview-portfolio-worker` 启动时幂等 ensure stream 和 durable consumer。
6. worker 消费 `rearview.portfolio_run.requested` 后：
   - 读取 `portfolio_run`。
   - 终态直接 ack。
   - 非终态用事务推进到 `calculating_nav`。
   - 查询 source run、buy_signal、交易日历和 backward adjusted OHLC。
   - 调用 Phase 3 纯计算内核。
   - run 级清理或 upsert `portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav` 和事件。
   - 更新 summary 和终态。
   - 终态写入成功后 ack。
7. worker 失败时根据错误类型写入 `failed_validation`、`failed_market_data`、`failed_simulation` 或 `failed_write`。
8. 如果 worker 已成功写入 `succeeded` 或 `failed_*` 终态，必须 ack NATS 消息；如果连终态都写入失败，不 ack，依赖 JetStream 重投递。
9. worker 不直接发布完成消息；第一版 UI 以 PostgreSQL 查询为权威。
10. 增加 NATS 重投递/重复消息测试或集成测试，覆盖同一 `portfolio_run_id` 被处理两次不产生重复 nav 或明细行。

完成标准：

1. NATS 未启动时，组合运行可创建为 pending，outbox 保留可恢复记录。
2. NATS 恢复后 dispatcher 能发布 pending 任务。
3. worker 重启或消息重投递不会重复计算出冲突结果。
4. 成功 run 写出完整 `portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav` 和 summary。

验证命令：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d nats
curl -fsS "http://127.0.0.1:${NATS_MONITOR_PORT:-34056}/healthz"

cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p rearview-portfolio-worker -- run --help
```

### Phase 6: Racingline 账户模板和组合页面

目标：把后端能力接入 Racingline，让用户能从策略和 run 页面进入组合净值结果。

任务：

1. 在 `app/racingline/src/types/rearview.ts` 增加账户模板、费率模板、portfolio run、nav、target、order、trade、position 和 event/warning 类型。
2. 在 Rearview API client 和 query hooks 中增加：
   - 默认市场模板查询。
   - 策略账户模板 CRUD。
   - portfolio run 创建、列表、详情、nav 查询。
   - targets、orders、trades、positions 和 events 查询。
3. `/rules` 策略创建和编辑流程新增“虚拟账户”配置区：
   - 初始资金默认 `1,000,000`。
   - 手续费和滑点从默认市场模板预填。
   - 调仓参数和卖出规则使用表单控件，不要求用户编辑 JSON。
4. `/runs/:runId` 在 source run succeeded 后显示“构建组合”入口。
5. 创建组合运行时显示账户模板、可覆盖参数和固定价格口径说明；成功后跳转 `/portfolios/:portfolioRunId`。
6. 新增 `/portfolios` 列表页，支持按策略、状态、日期区间和关键词筛选。
7. 新增 `/portfolios/:portfolioRunId` 详情页：
   - summary。
   - status 和 dispatch status。
   - 净值曲线、回撤、现金、持仓市值和仓位比例。
   - 持仓、成交、调仓目标和订单的轻量明细 tab，使用分页和日期筛选，不一次性拉取全量明细。
   - 参数 tab。
   - warning 或失败信息。
8. 运行中轮询 portfolio run 和 nav；成功后加载明细，失败后加载已写出的 warning 或 partial 明细。
9. `dispatch_status = pending` 时展示等待任务分发，不重复创建 run。
10. 前端不提供价格口径切换控件，只展示只读 `backward_adjusted` 研究口径。

完成标准：

1. 新策略账户模板表单能从后端默认模板预填。
2. 成功 source run 可以创建组合运行并进入详情页。
3. pending、running、succeeded、failed 状态都有明确 UI。
4. 成功 run 的持仓、成交、调仓目标和订单 tab 能显示后端返回的明细；若为空，显示明确 empty state。
5. 桌面和移动端无文本重叠、图表遮挡或按钮溢出。

验证命令：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run test
npm run build
```

浏览器验收：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

### Phase 7: 端到端联调、性能基线和故障恢复

目标：用真实本地依赖验证从选股 run 到 portfolio nav 的完整流程，并记录可复现证据。

任务：

1. 启动本地依赖：
   - PostgreSQL。
   - ClickHouse。
   - NATS。
2. 执行 rearview migration，确认 `CN_A_SHARE` 默认模板存在。
3. 启动 `rearview-server` 和 `rearview-portfolio-worker`。
4. 启动 Racingline。
5. 使用 Phase 0 选定 run 或创建新的短区间 run。
6. 从 UI 创建组合运行，确认 outbox、NATS publish、worker consume、nav 写入、明细表写入和 UI 展示。
7. 人工制造或模拟故障：
   - NATS 暂停时创建 run，恢复后 dispatcher 发布。
   - worker 重启后消息重投递。
   - 缺失价格导致 warning。
8. 调用 nav、targets、orders、trades、positions 和 events API，确认接口可分页返回结果。
9. 核验 summary 与明细账本的基本一致性：
   - `summary.trade_count` 等于 `portfolio_trade` 行数。
   - `summary.total_fee` 等于成交明细费用合计，允许明确的小数精度误差。
   - 最后一日 `portfolio_nav.position_count` 与同日 `portfolio_position_day` 行数一致。
10. 记录代表性计算耗时、nav 点数、source run 日期范围、买入信号数量、成交笔数、持仓日数和 summary。
11. 如果计算时间明显超过交互预期，先记录瓶颈；第一版不绕过 NATS，不把计算塞回 HTTP 请求。

完成标准：

1. 端到端 smoke run 成功生成净值曲线和明细账本。
2. NATS pending 恢复路径可用。
3. worker 重复消费路径幂等。
4. 验收报告包含命令、样本 run、截图或关键响应、明细 API 计数、summary 一致性核验、耗时和已知限制。

验证命令：

```bash
make racingline-dev

docker compose --env-file .env -f deploy/docker-compose.yml ps postgres clickhouse nats
curl -fsS "http://127.0.0.1:${NATS_MONITOR_PORT:-34056}/healthz"
curl -fsS "http://127.0.0.1:34057/rearview/portfolio-runs/<portfolio_run_id>/nav" | jq .
curl -fsS "http://127.0.0.1:34057/rearview/portfolio-runs/<portfolio_run_id>/trades?limit=5" | jq .
curl -fsS "http://127.0.0.1:34057/rearview/portfolio-runs/<portfolio_run_id>/positions?limit=5" | jq .
```

### Phase 8: 文档、系统地图和归档

目标：把实施完成后的真实入口、命令和限制写回仓库，避免 RFC 与当前事实分叉。

任务：

1. 更新 [System: Rearview](../../systems/rearview.md)：
   - 新 crate 路径。
   - server / worker 运行入口。
   - NATS 和 outbox 职责。
   - 新 API。
2. 更新 [System: Racingline](../../systems/racingline.md)：
   - `/portfolios` 和 `/portfolios/:portfolioRunId`。
   - 虚拟账户表单和组合结果页。
3. 更新 `engines/README.md`，说明 Rearview crate 拆分后的命令。
4. 如修改 `Makefile` 或 `.env.example`，同步文档中的本地启动命令。
5. 新增 `docs/jobs/reports/YYYY-MM-DD-racingline-portfolio-nav.md` 验收报告。
6. 实施完成后把本计划移入 `docs/plans/archive/`，状态改为 `Completed`，并更新 `docs/plans/README.md`。

完成标准：

1. 系统地图反映当前代码事实，不继续描述旧单 crate 作为唯一入口。
2. 验收报告可复现 smoke run。
3. RFC 0021 若仍为 Proposed，应根据实际结果调整状态或注明已实现范围。

验证命令：

```bash
make docs-check
git diff --check
```

## 禁止模式

1. 禁止在 Racingline 中直接查询 PostgreSQL、ClickHouse 或 NATS。
2. 禁止在 `rule_version` 中混入账户、费率、滑点或调仓快照。
3. 禁止在 NATS message 中携带完整账户快照、规则快照或行情数据。
4. 禁止绕过 PostgreSQL outbox 直接发布 NATS 后再写 `portfolio_run`。
5. 禁止在 HTTP 请求中同步执行长时间净值计算。
6. 禁止前端提供组合净值价格口径切换。
7. 禁止静默 fallback 到硬编码费率；缺少 active 默认市场模板必须报错。
8. 禁止 worker 使用 `unwrap()`、无限重试 busy loop 或未设边界的并发任务。
9. 禁止为第一版 UI 暴露空的成交、持仓、调仓目标 tab 伪装完整回测已完成。

## 允许保留的第一版例外

1. `portfolio_event` 可以先不建表，warning 可汇总写入 `portfolio_run.summary`，但 events API 必须能返回等价结构化 warning。
2. `indicator_stop_loss` 如果缺少后复权兼容指标，可以先 validation error，不临时重算指标。
3. 涨跌停、停牌和无法成交第一版可以按 warning 和跳过成交处理，直到 RFC 另行决策。
4. outbox dispatcher 第一版可以在 `rearview-server` 内运行，后续再按负载拆独立 binary。
5. 明细 UI 第一版可以轻量展示，不要求高级筛选、导出、归因或图表联动。

## 最小验证矩阵

| 范围 | 命令 |
|---|---|
| 文档 | `make docs-check`、`git diff --check` |
| PostgreSQL migration | `cd pipeline && uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head` |
| Rust 全量 | `cd engines && cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace` |
| Worker 入口 | `cd engines && cargo run -p rearview-portfolio-worker -- run --help` |
| NATS health | `docker compose --env-file .env -f deploy/docker-compose.yml up -d nats && curl -fsS "http://127.0.0.1:${NATS_MONITOR_PORT:-34056}/healthz"` |
| Frontend | `cd app/racingline && npm run lint && npm run typecheck && npm run test && npm run build` |
| E2E smoke | `make racingline-dev` 后通过 UI 创建 portfolio run，查看 `/portfolios/:portfolioRunId`，并调用 nav/trades/positions 明细 API 核验 summary 一致性 |

## 完成标准

1. 创建新策略时，Racingline 以后端 `CN_A_SHARE` 默认市场模板预填 `1,000,000 CNY` 虚拟账户。
2. 用户可以保存账户模板，并在成功选股 run 上创建组合运行。
3. `POST /rearview/portfolio-runs` 写入 `portfolio_run` 和 `portfolio_task_outbox`，返回 `202 Accepted`。
4. NATS 发布失败不会丢任务，outbox pending 可恢复。
5. `rearview-portfolio-worker` 消费任务后写入 `portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav` 和 summary。
6. 重复消费同一 `portfolio_run_id` 不产生重复或冲突净值和明细行。
7. 净值曲线从 `1.0` 开始，固定使用 `backward_adjusted` 后复权价格。
8. summary 中的成交数、费用合计和期末持仓数量能与 `portfolio_trade`、`portfolio_position_day` 和 `portfolio_nav` 明细交叉核验。
9. 费用、滑点、现金约束、1 手最小成交、资金不足跳过、已持仓不重复买入、空闲仓位按 rank 递补和卖出规则都有单元测试覆盖。
10. Racingline `/portfolios` 和 `/portfolios/:portfolioRunId` 能展示列表、状态、净值曲线、summary、参数、持仓、成交、调仓目标、订单和失败信息。
11. 系统地图、engines README 和验收报告已同步。

## 后续计划入口

第一版完成后，后续能力应拆成独立计划，不直接塞进本计划尾部：

1. 明细账本高级 UI：筛选、导出、原因解释、证券分析页联动和异常定位。
2. 基准指数、超额收益、归因和跨策略比较。
3. 涨跌停、停牌、成交量参与率和市场冲击滑点。
4. 企业行动、真实持仓数量调整和分红清算模型。
5. 大规模参数网格回测和独立 backtester 服务评估。
