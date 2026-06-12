# Plan 0036: Rust Rearview 规则选股服务实施计划

日期：2026-06-12

状态：Completed

关联文档：

- `docs/RFC/0018-rust-stock-screening-service.md`
- `docs/plans/archive/0035-stock-technical-indicator-marts-implementation-plan.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_trend_indicator.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_momentum_indicator.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_volume_indicator.md`
- `docs/design/dbt_layer/fleur_marts/mart_stock_price_pattern_daily.md`
- `engines/README.md`
- `.env.example`
- `deploy/docker-compose.yml`
- `pipeline/migrate/env.py`

相关 skills：

- `fleur-harness`：计划、文档治理、质量门禁和归档规则。
- `rust-best-practices` / `rust-patterns` / `rust-async-patterns` / `rust-testing`：Rust HTTP 服务、领域类型、异步执行和测试策略。
- `clickhouse-best-practices` / `clickhouse-architecture-advisor`：ClickHouse 查询规划、runtime join、ORDER BY 和未来专用宽表评估。
- `running-dbt-commands` / `using-dbt-for-analytics-engineering`：metric catalog 从 dbt mart YAML 读取或校验时使用。

## 1. 背景

RFC 0018 定义了 Rust `rearview` 规则选股 HTTP 服务。第一版目标是消费已有 `fleur_marts` 日频行情和指标 mart，按规则版本对日期区间逐日生成股票池，再对每日股票池评分并取 TopN 买入信号。

Rearview 第一版不是交易系统、回测引擎或指标计算引擎。它的核心工作是：

1. 用 PostgreSQL `rearview` database 保存规则、版本、运行、股票池、买入信号、指标目录和审计状态。
2. 用 Rust HTTP 服务校验规则 AST、编译受控 ClickHouse SQL、执行 runtime join、写入结果。
3. 用 ClickHouse `fleur_marts` 作为只读指标库。
4. 保证每次运行能追溯到不可变规则版本、`rule_hash`、metric dependency snapshot、compiled SQL hash 和 ClickHouse query id。

## 2. 目标

完成后应满足：

1. 新增单 crate Rust HTTP 服务 `engines/crates/rearview/`，包内按 `domain`、`api`、`planner`、`clickhouse`、`postgres`、`service` 分目录。
2. `pipeline/migrate` 支持同一 PostgreSQL 实例下的 `pipeline` 与 `rearview` 两个 database target。
3. PostgreSQL `rearview` database 拥有第一版业务表：`rule_set`、`rule_version`、`metric_catalog`、`run`、`run_chunk`、`run_day`、`pool_member`、`buy_signal`。
4. `metric_catalog` 采用半自动维护：dbt mart YAML 提供或校验基础字段事实，Rearview policy overlay 控制 allowlist、操作符、NULL 策略和 canonical 来源。
5. HTTP API 支持创建规则集、创建不可变规则版本、发起运行、查询 run/chunk/day/pool/signals，以及 explain。
6. 规则 DSL 支持第一版代表性用例需要的过滤和评分能力：字段比较、字段间比较、字段乘常数、布尔条件、`AND`、`conditional_points`、`weighted_sum`、clamp、每日 TopN。
7. 多年区间默认按自然年 chunk 执行；每个 chunk 记录日期范围、ClickHouse query id、状态、耗时和错误摘要。
8. `pool_member.selected_metrics` 只保存规则版本 `output_metrics`；`buy_signal.score_breakdown.raw_values` 保存评分解释所需的运行时指标值。
9. 代表性用例 `n_structure_low_reversal_screen` 可以创建规则版本、通过 explain、执行短区间 smoke run，并生成可解释的股票池和买入信号。
10. 本地开发和 smoke run 复用 `deploy/docker-compose.yml` 提供的 PostgreSQL 与 ClickHouse 设施，并通过根目录 `.env` / `.env.example` 统一配置连接信息。

## 3. 非目标

本计划不做以下事情：

1. 不实现交易、下单、风控、组合调仓或完整回测。
2. 不在 Rust 服务中重算 MA、RSI、BOLL、MACD、KDJ 或价格形态指标。
3. 不允许用户提交任意 SQL。
4. 不直接读取 `fleur_calculation`、`fleur_intermediate`、`fleur_staging` 或 `fleur_raw`。
5. 第一版不新增 `fleur_marts.mart_stock_rearview_metric_daily`；先用现有 mart runtime join 验证需求和性能。
6. 第一版不支持评分 veto 因子、上限封顶分组或跨因子依赖；硬排除条件放入 `pool_filters`。
7. 第一版不实现 Web UI、用户体系、权限模型或多租户隔离。
8. 第一版不把运行结果写入 ClickHouse；结果事实先保存在 PostgreSQL `rearview` database。

## 4. 当前事实基线

### 4.1 ClickHouse mart 输入

第一版允许 Rearview 读取以下 mart：

| Mart | 用途 | 当前排序特征 |
|---|---|---|
| `fleur_marts.mart_stock_quotes_daily` | 行情、成交量、ST/停牌、估值、市值和 universe 基础过滤 | `ORDER BY (security_code, trade_date)` |
| `fleur_marts.mart_stock_trend_indicator` | MA、组合 MA、EMA、BOLL、MACD | `ORDER BY (trade_date, security_code)` |
| `fleur_marts.mart_stock_momentum_indicator` | RSI、KDJ | `ORDER BY (trade_date, security_code)` |
| `fleur_marts.mart_stock_volume_indicator` | 均量 | `ORDER BY (trade_date, security_code)` |
| `fleur_marts.mart_stock_price_pattern_daily` | 连涨连跌和 N 结构形态 | `ORDER BY (trade_date, security_code)` |

第一版查询必须先按 `trade_date BETWEEN start_date AND end_date` 过滤各 mart，再参与 join。`mart_stock_quotes_daily` 的排序键对 date-only 全市场扫描不是最优，因此代表性多年区间必须记录 `EXPLAIN indexes = 1` 和 query log 观测。

### 4.2 PostgreSQL 迁移现状

当前 `pipeline/migrate/env.py` 只读取 `PIPELINE_DATABASE_URL`，并对单个 database 执行 Alembic migration。Rearview 需要同一 PostgreSQL 实例下的独立 `rearview` database，但 DDL 仍由 `pipeline/migrate` 统一管理。

### 4.3 Rust workspace 现状

`engines/` 当前只有 Furnace 相关 crates。Rearview 应新增单 crate：

```text
engines/crates/rearview/
```

不要把 Rearview 放入 `pipeline/` 的 Python uv workspace。

### 4.4 第一版实现假设

1. `mart_stock_rearview_metric_daily` 作为后续优化项；第一版 runtime join 现有 mart。
2. universe 第一版支持 `all_a_shares`、`exclude_st`、`exclude_suspend`、`include_security_codes`、`exclude_security_codes`，以及 metric catalog 明确开放的市值、板块或上市天数类字段。无法从当前 mart 或 catalog 解析的 universe 条件必须校验失败。
3. scoring rules 独立求值并累加，最后 clamp；分段互斥需要在规则条件中显式写出。
4. 多年区间默认自然年 chunk；不超过 90 个交易日的短区间可以单次 range query。
5. 月度 chunk 只作为年度 chunk 超时、内存压力过高或单年结果写入过大时的 fallback。

### 4.5 本地设施和 `.env` 基线

Rearview 第一版不新增一套本地基础设施。开发、迁移验证和 smoke run 复用根目录 `.env` 驱动的 `deploy/docker-compose.yml`：

| Compose 服务 | Rearview 用途 | 必需性 |
|---|---|---|
| `postgres` | `pipeline` 与 `rearview` 两个 PostgreSQL database，共用同一实例 | 必需 |
| `clickhouse` | 读取 `fleur_marts` 指标 mart，执行 explain 和 runtime join | 必需 |
| `rustfs` / `rustfs-init` | 仅在需要通过现有 pipeline 刷新上游 raw/mart 数据时使用 | 可选 |
| `nats` | 第一版 Rearview HTTP 服务不直接依赖 | 非必需 |

本地最小启动命令：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d postgres clickhouse
docker compose --env-file .env -f deploy/docker-compose.yml ps postgres clickhouse
```

`.env.example` 需要在 Phase 1 同步补齐 Rearview 相关变量，并把 PostgreSQL database 命名与分库决策对齐。建议第一版示例：

```env
POSTGRES_DB=pipeline
PIPELINE_DATABASE_URL=postgresql://mono_fleur:change-me-postgres-password@127.0.0.1:34054/pipeline
REARVIEW_DATABASE_URL=postgresql://mono_fleur:change-me-postgres-password@127.0.0.1:34054/rearview

REARVIEW_HTTP_BIND=127.0.0.1:34057
REARVIEW_MAX_CONCURRENT_RUNS=1
REARVIEW_CHUNK_SMALL_RANGE_TRADING_DAYS=90
REARVIEW_CLICKHOUSE_MARTS_DATABASE=fleur_marts
REARVIEW_CLICKHOUSE_MAX_EXECUTION_TIME_SECONDS=300
REARVIEW_CLICKHOUSE_MAX_ROWS_TO_READ=1000000000
REARVIEW_CLICKHOUSE_MAX_BYTES_TO_READ=100000000000
```

实现时不要把 `CLICKHOUSE_DATABASE` 直接当作 mart database 使用；现有 `.env.example` 中该变量服务于 pipeline / dbt 语境，可能指向 raw 或默认库。Rearview 必须通过 `metric_catalog` 保存完整 mart 来源，或通过 `REARVIEW_CLICKHOUSE_MARTS_DATABASE` 明确指定 mart database，避免把 `fleur_marts` 表编译到错误 database。

## 5. Phase 0: 冻结第一版执行边界

实施前先确认 RFC 与本计划的一致性：

1. 服务名固定为 `rearview`。
2. Rust crate 路径固定为 `engines/crates/rearview/`。
3. PostgreSQL database 固定为 `rearview`，表名不带 `screening_` 前缀。
4. 第一版不新增 ClickHouse 专用筛选宽表。
5. 第一版评分不支持 veto、封顶分组和跨因子依赖。
6. 第一版多年运行默认自然年 chunk。
7. 代表性用例固定为 `n_structure_low_reversal_screen`。
8. 本地基础设施固定复用 `deploy/docker-compose.yml` 的 `postgres` 与 `clickhouse` 服务，连接信息从根目录 `.env` 注入。

完成标准：

- RFC 0018 与本计划没有互相冲突的第一版边界。
- `.env.example`、`deploy/docker-compose.yml`、Alembic target 和 Rust 配置项对 database 命名、端口和 mart database 的表达一致。
- 如 RFC 再调整 `mart_stock_rearview_metric_daily`、universe 或 scoring 机制，本计划先更新再开始代码实现。

验证命令：

```bash
make docs-check
git diff --check
```

## 6. Phase 1: 改造 PostgreSQL Alembic 多 database 迁移

目标：让 `pipeline/migrate` 同时管理 `pipeline` 与 `rearview` database，但保持一个迁移权威入口。

建议设计：

1. 保留 `PIPELINE_DATABASE_URL`，新增 `REARVIEW_DATABASE_URL`。
2. `PIPELINE_DATABASE_URL` 和 `REARVIEW_DATABASE_URL` 指向同一 PostgreSQL 实例的不同 database。
3. `pipeline/migrate` 支持 target 选择，例如：
   - `pipeline`
   - `rearview`
   - `all`
4. 使用 Alembic branch label、version locations 或等价机制隔离两个 database 的 migration history。
5. 每个 database 维护自己的 `alembic_version` 状态。
6. 迁移工具负责确保目标 database 存在；Rust 服务运行时不创建 database、不建表。
7. 本地执行 migration 时从根目录 `.env` 读取 `PIPELINE_DATABASE_URL` 和 `REARVIEW_DATABASE_URL`；可以由 shell 显式导出，也可以在 migrate 工具中以仅本地开发方式加载 `.env`。

建议路径：

```text
pipeline/migrate/env.py
pipeline/migrate/versions/pipeline/
pipeline/migrate/versions/rearview/
.env.example
```

需要调整：

1. 将现有 OCR / pipeline migration 归入 `pipeline` target。
2. 新增 `rearview` target 的初始 migration。
3. `.env.example` 增加 `REARVIEW_DATABASE_URL` 示例，并把 `PIPELINE_DATABASE_URL` 示例从旧的单库名调整为 `/pipeline`。
4. 明确现有本地开发库如果仍使用 `/mono_fleur` 的处理方式：要么一次性重建 dev PostgreSQL volume，要么提供只迁 OCR / pipeline 表到 `pipeline` database 的人工步骤；不要让新迁移继续默默写入旧单库。
5. README 或 migration 注释说明部署顺序：先启动 compose 基础设施，再 Alembic，最后启动 Rearview。
6. 记录 root `.env` 到 Alembic 命令的加载方式，避免在 `pipeline/migrate` 子目录运行时找不到环境变量。

完成标准：

- 使用 `docker compose --env-file .env -f deploy/docker-compose.yml up -d postgres` 后，migration 可以只依赖 `.env` 里的 URL 连接本地 PostgreSQL。
- `pipeline` target 可以在现有 dev database 上应用现有 OCR 表 migration。
- `rearview` target 可以创建独立 `rearview` database 并应用 Rearview 初始 DDL。
- Rust 运行时账号只需要 DML 权限；DDL 权限只给 migration 账号。

验证命令：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d postgres

cd pipeline/migrate
uv run alembic upgrade head

# 如果实现了显式 target 参数，追加验证：
uv run alembic -x target=pipeline upgrade head
uv run alembic -x target=rearview upgrade head
```

如果最终实现选择非 Alembic `-x` 的 target 机制，应在本计划完成时把命令替换为真实命令。

## 7. Phase 2: 新增 Rearview PostgreSQL schema

目标：在 `rearview` database 中创建第一版业务表。

建议表：

| 表 | 粒度 | 关键字段 |
|---|---|---|
| `rule_set` | 每规则集一行 | `rule_set_id`、`name`、`description`、`owner`、`status`、`tags`、`current_version_id`、审计时间 |
| `rule_version` | 每不可变规则版本一行 | `rule_version_id`、`rule_set_id`、`version_no`、rule AST、universe snapshot、metric dependency snapshot、`rule_hash`、状态 |
| `metric_catalog` | 每 logical metric 一行 | `logical_metric`、mart table、column、类型、`allowed_ops`、`allow_filter`、`allow_scoring`、`null_policy`、canonical 标记 |
| `run` | 每次区间运行一行 | `run_id`、`rule_version_id`、日期区间、`top_n`、universe snapshot、状态、`compiled_sql_hash`、汇总数量 |
| `run_chunk` | 每次运行、每 chunk 一行 | `run_id`、`chunk_no`、`start_date`、`end_date`、状态、ClickHouse query id、耗时、错误摘要 |
| `run_day` | 每次运行、每交易日一行 | `run_id`、`trade_date`、状态、universe/pool/signal count、chunk metadata |
| `pool_member` | 每次运行、每交易日、每入池证券一行 | `run_id`、`trade_date`、`security_code`、`score`、`signal_rank`、`selected_metrics`、filter snapshot |
| `buy_signal` | 每次运行、每交易日、每 TopN 证券一行 | `run_id`、`trade_date`、`security_code`、`rank`、`score`、`score_breakdown`、`selected_metrics` |

约束和索引：

1. `rule_version` 创建后不可修改结果相关 JSON；应用层禁止更新。
2. `rule_set.current_version_id` 只作为当前默认版本指针，不作为历史事实。
3. `rule_version` 建议唯一约束：
   - `(rule_set_id, version_no)`
   - `(rule_set_id, rule_hash)`
4. `run_day` 主键：`(run_id, trade_date)`。
5. `pool_member` 主键：`(run_id, trade_date, security_code)`。
6. `buy_signal` 主键：`(run_id, trade_date, security_code)`，并对 `(run_id, trade_date, rank)` 建唯一约束。
7. 常用查询索引：
   - `run(rule_version_id, created_at)`
   - `run(status, created_at)`
   - `pool_member(run_id, trade_date, signal_rank)`
   - `buy_signal(run_id, trade_date, rank)`
8. JSONB 字段第一版不默认建 GIN 索引，除非 API 需要按 JSON 内容筛选。

完成标准：

- 新 migration 可重复应用到空 `rearview` database。
- 表、主键、外键、唯一约束和状态 check 约束清晰。
- `run_chunk` 能表达自然年 chunk 的执行状态和 query id，不需要把同一个 chunk query id 重复写到每个 `run_day`。

验证命令：

```bash
cd pipeline/migrate
uv run alembic -x target=rearview upgrade head
uv run alembic -x target=rearview downgrade -1
uv run alembic -x target=rearview upgrade head
```

如果 downgrade 策略决定不支持生产回滚，应在 migration 文档中明确，并至少验证空库 upgrade。

## 8. Phase 3: 建立 metric catalog 半自动维护流程

目标：让 `metric_catalog` 不成为第二套字段事实源，同时给 Rearview 提供运行时 allowlist。

建议输入：

```text
pipeline/elt/models/marts/*.yml
engines/crates/rearview/config/metric_policy.yml
```

`metric_policy.yml` 建议字段：

```yaml
metrics:
  - logical_metric: kdj_j_value
    source:
      mart_table: mart_stock_momentum_indicator
      column_name: kdj_j_value
    value_kind: numeric
    allow_filter: true
    allow_scoring: true
    allowed_ops: [lt, lte, gt, gte, between, eq, is_null]
    null_policy: no_match
    default_output: true
```

实现内容：

1. 从 dbt mart YAML 读取候选字段事实：mart table、column、描述、类型。
2. 读取 Rearview policy overlay。
3. 校验 overlay 引用字段存在、类型兼容、canonical 来源不冲突。
4. 将 active catalog 写入 PostgreSQL `rearview.metric_catalog`。
5. 支持 check 模式，用于 CI 和本地验证。

第一版必须覆盖代表性用例字段：

| 字段组 | 字段 |
|---|---|
| 行情 | `close_price`、`prev_volume`、`volume` |
| 动量 | `kdj_j_value`、`rsi_6` |
| 趋势 | `price_ema2_10`、`price_avg_ma_14_28_57_114`、`price_avg_ma_3_6_12_24`、`price_ma_20`、`price_ma_60`、`boll_dn_20_2` |
| 成交量 | `volume_ma_5` |
| 形态 | `close_down_streak_days`、`n_structure_20_is_valid` |

完成标准：

- 代表性用例字段全部进入 `metric_catalog`。
- Boolean 字段 `n_structure_20_is_valid` 只允许布尔相关操作符。
- RHS metric 也经过 catalog 校验，不能只校验左侧 metric。
- overlay 引用不存在字段时，check 模式失败。

验证命令：

```bash
cd engines
cargo test -p rearview metric_catalog

cd ../pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
```

如果 catalog sync 先实现为 Rust 子命令，追加：

```bash
cd engines
cargo run -p rearview -- catalog check
cargo run -p rearview -- catalog sync
```

## 9. Phase 4: 新增 Rust crate 和基础 HTTP 服务

目标：建立 Rearview 单 crate 服务骨架。

建议结构：

```text
engines/crates/rearview/
├── Cargo.toml
├── config/
│   └── metric_policy.yml
└── src/
    ├── main.rs
    ├── app.rs
    ├── config.rs
    ├── error.rs
    ├── domain/
    ├── api/
    ├── planner/
    ├── clickhouse/
    ├── postgres/
    └── service/
```

边界：

1. `domain` 不依赖 HTTP、PostgreSQL、ClickHouse 或环境变量。
2. `planner` 负责 AST 校验、依赖收集和查询计划。
3. `clickhouse` 负责 SQL 编译、query id 和执行。
4. `postgres` 负责 repository、事务和批量写入。
5. `service` 负责编排 create rule version、create run、execute run、explain。
6. HTTP handler 只做 DTO 转换、鉴权占位和调用 service。

配置项：

| 配置 | 用途 |
|---|---|
| `REARVIEW_DATABASE_URL` | PostgreSQL `rearview` runtime 连接 |
| `CLICKHOUSE_HOST` / `CLICKHOUSE_PORT` / `CLICKHOUSE_USER` / `CLICKHOUSE_PASSWORD` / `CLICKHOUSE_SECURE` | 复用 `.env` 中的 ClickHouse 连接参数 |
| `REARVIEW_CLICKHOUSE_MARTS_DATABASE` | mart database，默认 `fleur_marts`，不要复用语义不清的 `CLICKHOUSE_DATABASE` |
| `REARVIEW_CLICKHOUSE_MAX_EXECUTION_TIME_SECONDS` | 每个 ClickHouse 查询的执行时间上限 |
| `REARVIEW_CLICKHOUSE_MAX_ROWS_TO_READ` | 每个 ClickHouse 查询的扫描行数上限 |
| `REARVIEW_CLICKHOUSE_MAX_BYTES_TO_READ` | 每个 ClickHouse 查询的扫描字节上限 |
| `REARVIEW_HTTP_BIND` | HTTP bind 地址 |
| `REARVIEW_MAX_CONCURRENT_RUNS` | 并发运行上限 |
| `REARVIEW_CHUNK_SMALL_RANGE_TRADING_DAYS` | 单次 range query 阈值，默认 90 |

环境加载要求：

1. 本地开发可以选择在 Rust binary 启动时加载 repo root `.env`，或提供明确的 shell wrapper / runbook 先导出 `.env`；两种方式只能保留一种作为文档化路径。
2. 生产和 CI 不依赖隐式 `.env` 文件，所有配置由进程环境注入。
3. 单元测试不读取真实 `.env`；需要数据库或 ClickHouse 的测试标记为 integration test，并允许通过环境变量跳过。
4. 启动日志可以打印 host、port、database、开关状态，但不能打印密码、完整 DSN 或 token。

完成标准：

- `cargo check -p rearview` 通过。
- `GET /healthz` 返回服务健康状态。
- 服务启动时对 PostgreSQL schema 做只读 readiness check；缺表或版本不兼容时 fail fast。
- `GET /healthz` 或 readiness 能区分 PostgreSQL 不可用、schema 不兼容、ClickHouse 不可用和 mart database 不存在。
- 本地 runbook 能从根 `.env` 启动 compose 设施、执行 migration、启动 Rust 服务。
- stdout/stderr 和 HTTP response 不泄露数据库连接敏感信息。

验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## 10. Phase 5: 实现规则 AST、hash 和校验

目标：实现不可变规则版本所需的领域模型和校验逻辑。

第一版 AST 能力：

1. 布尔组合：`all`、`any`、`not`。
2. 比较操作符：`eq`、`ne`、`lt`、`lte`、`gt`、`gte`、`between`、`is_null`。
3. 字段间比较：`metric` 与 `rhs_metric`。
4. 受控 RHS 表达式：metric 乘常数，例如 `prev_volume * 0.8`。
5. Boolean 字段比较，例如 `n_structure_20_is_valid = true`。
6. `weighted_sum` scoring。
7. `conditional_points` scoring。
8. `clamp`，第一版范围为 `[0, 99]`。
9. `output_metrics` 列表。

禁止：

1. 任意 SQL 片段。
2. 未登记 metric。
3. 未授权操作符。
4. scoring rule 之间的隐式依赖。
5. veto、封顶分组和跨因子依赖。

`rule_hash`：

1. 对影响结果的字段做 canonical JSON。
2. 纳入 `universe_snapshot`、`pool_filters`、`scoring`、`top_n_default`、`output_metrics`、`metric_dependency_snapshot`。
3. 不纳入 `name`、`description`、tags 和 UI 字段。
4. 使用 SHA-256。

完成标准：

- 同一规则内容在字段顺序变化时产生相同 hash。
- 任一结果相关字段变化都会产生不同 hash。
- 代表性用例 AST 通过校验。
- 未登记字段、错误类型、错误操作符、NULL 策略缺失时返回结构化 validation error。

验证命令：

```bash
cd engines
cargo test -p rearview domain
cargo test -p rearview planner
```

## 11. Phase 6: 实现 PostgreSQL repository 和运行状态机

目标：实现规则、版本、运行、chunk、day、pool、signal 的事务边界。

状态流转：

```text
created
  -> validating
  -> compiling
  -> running_clickhouse
  -> writing_pool
  -> writing_signals
  -> succeeded
```

失败状态：

```text
failed_validation
failed_compile
failed_clickhouse
failed_write
cancelled
```

事务要求：

1. 创建规则版本时一次事务写入 `rule_version`，并可选更新 `rule_set.current_version_id`。
2. 发起运行时冻结 `rule_version_id`、`rule_hash`、日期区间、`top_n` 和 universe snapshot。
3. 每个自然年 chunk 有独立 `run_chunk` 状态。
4. 每个 anchor mart 实际交易日有独立 `run_day` 状态；零入池交易日也必须保留并记录 `pool_count = 0`、`signal_count = 0`。
5. `pool_member` 和 `buy_signal` 按 `(run_id, trade_date, security_code)` 幂等写入。
6. chunk 失败不应破坏已成功 chunk 的事实；run 汇总为失败状态并记录错误摘要。

完成标准：

- repository 单元测试覆盖状态流转、重复写入、失败写入和汇总更新。
- run 创建后即使异步执行失败，也能查询到失败状态和错误摘要。
- `buy_signal` 必须来自同一批 ranked pool rows，不独立重算评分。

验证命令：

```bash
cd engines
cargo test -p rearview postgres
cargo test -p rearview service
```

## 12. Phase 7: 实现 ClickHouse 查询规划和执行

目标：把已校验 AST 编译为受控 ClickHouse SQL，并读取现有 mart runtime join 结果。

查询规划要求：

1. 从 AST 和 `output_metrics` 收集 required metrics。
2. 通过 `metric_catalog` 推导最小 mart 表集合和列集合。
3. 每个 mart CTE 先按 chunk 日期范围过滤，再参与 join。
4. join key 固定为 `(security_code, trade_date)`。
5. 字段名、表名、排序方向全部来自 allowlist。
6. 字面量使用参数绑定或等价安全机制，不拼接用户输入。
7. 查询输出包含 pool rows、score、signal_rank、score_breakdown 所需 raw values 和 selected metrics。
8. query id 包含 `run_id` 和 chunk 信息，便于查 ClickHouse query log。
9. 所有 mart 表名编译为明确 database + table，database 来自 `metric_catalog` 或 `REARVIEW_CLICKHOUSE_MARTS_DATABASE`。
10. 生成 SQL 不使用 `SELECT *`；只输出 required metrics、score、rank 和写入 PostgreSQL 所需字段。
11. 每个 ClickHouse 查询都附带查询级安全设置，至少包含 `max_execution_time`、`max_rows_to_read` 或 `max_bytes_to_read`、`timeout_before_checking_execution_speed = 0`。
12. Runtime join 默认允许 ClickHouse 使用 `join_algorithm = 'auto'`；如年度 chunk 触发内存错误，再通过配置或 fallback 切到更小 chunk，而不是放宽无界内存。

chunk 策略：

1. 不超过 90 个交易日的短区间可以单次 range query。
2. 跨多年或超过阈值的区间按自然年 chunk。
3. 年度 chunk 失败后可以 fallback 到月度 chunk，但第一版不默认拆月。

完成标准：

- explain 模式可以输出 required marts、required columns、compiled SQL hash、chunk plan 和可选 `EXPLAIN indexes = 1` 摘要。
- 代表性用例能编译出只读取必要 mart 和必要列的 SQL。
- `score_breakdown.raw_values` 与评分条件使用的指标一致。
- 编译 SQL 中每个 mart CTE 都先过滤 `trade_date` chunk 范围；`mart_stock_quotes_daily` 因排序键为 `(security_code, trade_date)`，必须在 explain 或 query log 中记录扫描量观测。
- TopN 的 `LIMIT` 只用于结果裁剪，不作为扫描保护；扫描保护必须来自日期 chunk、列裁剪和查询级 safety settings。
- ClickHouse 连接失败、mart database 不存在、字段不存在、查询超时和内存错误能映射为可审计的 run/chunk 错误。

验证命令：

```bash
cd engines
cargo test -p rearview clickhouse
cargo test -p rearview planner
```

真实 ClickHouse 可用时追加：

```bash
cd engines
cargo run -p rearview -- explain --sample-rule n_structure_low_reversal_screen
```

如果第一版不提供 CLI explain，则用 HTTP `POST /rearview/explain` 代替。

## 13. Phase 8: 实现 HTTP API

目标：提供 RFC 0018 定义的第一版 HTTP API。

接口：

| Method | Path | 必须落地 |
|---|---|---|
| `POST` | `/rearview/rule-sets` | 创建规则集 |
| `POST` | `/rearview/rule-sets/{rule_set_id}/versions` | 创建不可变规则版本 |
| `POST` | `/rearview/runs` | 发起区间运行，返回 `run_id` |
| `GET` | `/rearview/runs/{run_id}` | 查询运行状态和汇总 |
| `GET` | `/rearview/runs/{run_id}/chunks` | 查询 chunk 日期范围、状态、ClickHouse query id、耗时和错误摘要 |
| `GET` | `/rearview/runs/{run_id}/days` | 查询日粒度状态 |
| `GET` | `/rearview/runs/{run_id}/pool?trade_date=...` | 查询某日股票池 |
| `GET` | `/rearview/runs/{run_id}/signals?trade_date=...` | 查询某日买入信号 |
| `POST` | `/rearview/explain` | 校验和解释规则，不写股票池和信号；带日期区间时返回 chunk plan |

执行语义：

1. `POST /rearview/runs` 默认异步执行并立即返回 `run_id`。
2. 如果请求只传 `rule_set_id`，服务解析 `rule_set.current_version_id`，并把实际 `rule_version_id` 和 `rule_hash` 写入 `run`。
3. HTTP response DTO 只读 PostgreSQL 事实，不重新计算评分。
4. explain 不写 `pool_member` 和 `buy_signal`。
5. explain 请求可以只传规则 AST，也可以传 `{rule, start_date, end_date, top_n}`；后者必须返回自然年 chunk plan。
6. 第一版可以不实现取消接口，但状态机保留 `cancelled`。

完成标准：

- API 集成测试覆盖成功创建规则、创建版本、发起运行、查询结果和 explain。
- 错误响应结构化，能区分 validation、compile、ClickHouse、PostgreSQL 写入错误。
- 对不存在的 run、日期无结果、无当前版本等情况返回明确错误。

验证命令：

```bash
cd engines
cargo test -p rearview api
cargo test -p rearview service
```

## 14. Phase 9: 代表性用例验收

目标：用 RFC 0018 的 `n_structure_low_reversal_screen` 验证第一版闭环。

过滤条件：

```text
kdj_j_value < -10
close_down_streak_days < 4
price_ema2_10 > price_avg_ma_14_28_57_114
volume > prev_volume * 0.8
n_structure_20_is_valid = true
```

评分条件：

```text
close_price <= boll_dn_20_2 * 1.02 => +0.25
rsi_6 < 25 => +10
kdj_j_value < -15 => +35
-15 <= kdj_j_value < -10 => +25
volume < volume_ma_5 * 0.5 => +20
close_price < price_avg_ma_3_6_12_24 => +15
price_ma_20 < close_price < price_ma_60 => +15
close_price > price_avg_ma_3_6_12_24 * 1.05 => -15
clamp => [0, 99]
```

验收步骤：

1. 从根目录 `.env` 启动本地 `postgres` 和 `clickhouse`。
2. 执行 `pipeline` 与 `rearview` Alembic target migration。
3. 确认 ClickHouse 中 `fleur_marts` 五张输入 mart 存在，并且代表性日期范围内有数据；如 mart 未准备好，job report 只能记录 readiness/explain 结果，不能宣称 smoke run 通过。
4. catalog check 覆盖所有用例字段。
5. 创建规则集和规则版本。
6. 调用 explain，确认 required marts 为现有五张 mart 的子集，required columns 不包含无关列。
7. 短区间 smoke run，例如 30 个交易日。
8. 多年区间 dry benchmark，例如 `2021-01-01` 到 `2025-12-31`，默认自然年 chunk。
9. 抽样检查某个 `buy_signal` 的 `score_breakdown.raw_values` 能解释最终 score。
10. 记录 ClickHouse `EXPLAIN indexes = 1` 和 query log 摘要。

完成标准：

- 短区间 run 成功，`run`、`run_chunk`、`run_day`、`pool_member`、`buy_signal` 均有一致结果。
- 多年 run 至少完成 explain 和 chunk plan；真实执行如受环境限制，可在 job report 中记录限制。
- `buy_signal` 每日数量不超过 `top_n`，股票池少于 `top_n` 时等于实际股票池数量。
- 分数排序稳定：`score DESC, security_code ASC`。

运行报告：

```text
docs/jobs/reports/YYYY-MM-DD-rearview-n-structure-low-reversal-smoke-run.md
```

报告至少包含：

1. 使用的 `.env` 变量名清单、compose 服务状态、命令或 HTTP 请求摘要；不要记录敏感值。
2. 日期范围和 `top_n`。
3. run id、rule version id、rule hash。
4. chunk 计划和 ClickHouse query id。
5. pool count、signal count。
6. `EXPLAIN indexes = 1` 或无法执行的原因。
7. 失败和修复记录。

## 15. Phase 10: 质量门禁和文档收敛

目标：完成第一版合入前的验证闭环。

文档更新：

1. 更新 `engines/README.md`，加入 `rearview` crate、服务边界和命令。
2. 如实际实现与 RFC 0018 不一致，先更新 RFC 或新增 ADR，再合入代码。
3. 如 catalog overlay 路径或 Alembic target 命令与本计划不同，更新本计划。
4. 完成后新增 job report，并将本计划归档到 `docs/plans/archive/`。

最小代码质量门禁：

```bash
make docs-check
git diff --check

docker compose --env-file .env -f deploy/docker-compose.yml ps postgres clickhouse

cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../pipeline
uv run dbt parse --project-dir elt --profiles-dir elt

cd migrate
uv run alembic upgrade head
```

如果 Alembic 已实现显式 target 参数，最终门禁应替换为：

```bash
cd pipeline/migrate
uv run alembic -x target=pipeline upgrade head
uv run alembic -x target=rearview upgrade head
```

如修改 `pipeline/migrate` 测试或 Python 工具，追加：

```bash
cd pipeline
uv run ruff check migrate
uv run ruff format --check migrate
uv run pyright migrate
```

如新增 Dagster 编排或 schedule，追加：

```bash
cd pipeline/scheduler
uv run dg check defs
```

完成标准：

- 所有适用门禁通过。
- 第一版 API 和代表性用例验收结果有 job report。
- active plan 状态改为 `Completed` 后移入 archive，并更新 `docs/plans/README.md`。

## 16. 风险和缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| Alembic 多 database target 设计不清 | 迁移命令和部署顺序混乱 | Phase 1 先落 target 机制和命令文档，再建 Rearview 表 |
| `.env.example` 与实际配置漂移 | 本地 compose、Alembic 和 Rust 服务使用不同库名或端口 | Phase 1/4 同步更新 `.env.example`、配置解析和 runbook；job report 记录变量名清单 |
| 旧本地单库名 `mono_fleur` 未处理 | OCR / pipeline 表和 Rearview 表可能混写到旧 database | Phase 1 明确 dev reset 或迁移路径，`PIPELINE_DATABASE_URL` 示例改为 `/pipeline` |
| `CLICKHOUSE_DATABASE` 被误用为 mart database | SQL 编译到 raw/default database，导致表不存在或读错层 | 使用 `metric_catalog` 的完整来源或 `REARVIEW_CLICKHOUSE_MARTS_DATABASE=fleur_marts` |
| `run` 表名与 SQL 语义混淆 | 查询和 ORM 映射易读性下降 | 所有 SQL 显式引用表名；如实现阶段发现冲突，改名为 `run_record` 并同步 RFC/计划 |
| runtime join 多年运行过慢 | HTTP run 长时间 running 或 ClickHouse 内存压力高 | 自然年 chunk、query log 观测；若仍不够，另起计划建设 `mart_stock_rearview_metric_daily` |
| ClickHouse 查询只靠 TopN `LIMIT` 控制规模 | 仍可能扫描大量 mart 数据并拖垮本地或生产实例 | Phase 7 必须带日期 chunk、列裁剪、查询级 scan/time safety settings |
| metric catalog 漂移 | 规则引用不存在或语义变化字段 | overlay 必须由 dbt mart YAML 校验；catalog check 进入 CI |
| 规则 DSL 过早复杂化 | planner、解释和测试成本增加 | 第一版只支持独立 scoring rule 加总；复杂评分机制需要真实策略案例和新 RFC |
| 历史结果解释漂移 | ClickHouse mart 回填后解释不一致 | `buy_signal.score_breakdown.raw_values` 和 `selected_metrics` 保存运行时快照 |
| 多年运行失败恢复困难 | 需要整段重跑 | `run_chunk` 自然年边界记录状态和 query id，失败 chunk 可单独重跑 |

## 17. 完成标准

本计划完成时应满足：

1. PostgreSQL `rearview` database DDL 已由 `pipeline/migrate` 管理。
2. `.env.example`、`deploy/docker-compose.yml`、Alembic target 和 Rust 配置可以共同支撑本地 PostgreSQL / ClickHouse smoke run。
3. Rust `rearview` HTTP 服务可以启动并通过 readiness check。
4. metric catalog 半自动流程可校验 dbt mart YAML 与 policy overlay。
5. 代表性用例规则版本可创建、可 explain、可运行短区间 smoke。
6. 多年区间生成自然年 chunk plan，并记录每个 chunk 的 query id 和状态。
7. `pool_member` 和 `buy_signal` 写入 PostgreSQL，且 `score_breakdown` 可解释分数。
8. 质量门禁通过，并有运行报告记录真实验收结果。
