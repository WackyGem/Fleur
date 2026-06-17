# Plan 0042: 指数与 Benchmark Intermediate 模型实施计划

日期：2026-06-16

状态：Completed

## 目标

1. 在 dbt intermediate 层新增四个模型：`int_index_basic_snapshot`、`int_index_quotes_daily`、`int_benchmark_basic_snapshot` 和 `int_benchmark_returns_daily`。
2. `int_index_*` 从 BaoStock 现有 staging 模型获取指数基础信息和日行情，不新增 source asset 或 raw contract。
3. `int_benchmark_*` 从指数 intermediate 模型过滤当前组合绩效需要的 benchmark 清单，不重复读取 staging 或 raw。
4. 日行情 / 日收益模型只保留日频事实所需字段，不输出纯数字本地指数代码和交易所代码。
5. benchmark 代码选择默认优先采用上交所 `SH` 代码；指数本身只有深交所代码时使用 `SZ` 代码。
6. 明确 benchmark 与组合 NAV 当前同为价格收益口径，均不包含分红再投资。
7. 为后续 portfolio worker / ClickHouse portfolio data plane 提供稳定输入表，而不是让 worker 直接过滤 raw 或 staging 表。

## 非目标

1. 不新增国债收益率、无风险利率采集或 risk-free mart；该部分另行计划。
2. 不构造全收益指数，不处理分红再投资 benchmark。
3. 不改 `int_stock_basic_snapshot`、`int_stock_quotes_daily_unadj` 或 `mart_stock_quotes_daily` 的股票 universe 口径。
4. 不新增 Dagster source asset、ClickHouse raw 表或 contract dataset。
5. 不在本计划内实现 portfolio performance metric 表、alpha / beta 计算或 Rearview API。

## 关联文档

- [Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane](../../Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md)
- [Plan 0041: Racingline 虚拟账户与组合调仓净值实施计划](../0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md)
- [stg_baostock__query_stock_basic 设计](../../design/dbt_layer/fleur_staging/stg_baostock__query_stock_basic.md)
- [int_stock_basic_snapshot 设计](../../design/dbt_layer/fleur_intermediate/int_stock_basic_snapshot.md)

## 当前事实基线

1. `stg_baostock__query_stock_basic` 已将 BaoStock `type = 2` 映射为 `security_type = 'index'`。
2. `stg_baostock__query_history_k_data_plus_daily` 已提供标准化后的 `security_code`、`trade_date`、`open_price`、`high_price`、`low_price`、`close_price`、`prev_close_price`、`volume`、`amount`、`is_suspend` 和 `is_st`。
3. `int_stock_basic_snapshot` 明确只服务股票 universe，并在设计文档中说明如需指数、ETF、可转债等统一基础信息，应另设模型或调整命名和口径。
4. 当前 benchmark 采用价格指数口径；当前组合 NAV 也不包含分红再投资口径，因此价格指数 benchmark 可作为主 benchmark。

## Raw 数据 Profile

查询时间：2026-06-16

查询范围：

- `fleur_raw.baostock__query_stock_basic`
- `fleur_raw.baostock__query_history_k_data_plus_daily`

有效判定：

- `baostock__query_stock_basic.type = 2`
- `baostock__query_stock_basic.status = 1`
- 日行情存在，且 `adjustflag = 3` 行数大于 0
- 选中代码按 `SH` 优先；国证1000 只有 SZ 代码，使用 `399311.SZ`

当前可用性：

| benchmark_key | 指数名称 | 状态 | 选中代码 | BaoStock raw code | raw 名称 | adjustflag=3 行数 | 日期范围 |
|---|---|---|---|---|---|---:|---|
| `cnindex_1000` | 国证1000 | usable | `399311.SZ` | `sz.399311` | 国证1000指数 | 4,955 | 2006-01-04 至 2026-06-01 |
| `csi_1000` | 中证1000 | usable | `000852.SH` | `sh.000852` | 中证1000指数 | 2,824 | 2014-10-17 至 2026-06-01 |
| `csi_300` | 沪深300 | usable | `000300.SH` | `sh.000300` | 沪深300指数 | 4,955 | 2006-01-04 至 2026-06-01 |
| `csi_500` | 中证500 | usable | `000905.SH` | `sh.000905` | 中证500指数 | 4,707 | 2007-01-15 至 2026-06-01 |
| `csi_800` | 中证800 | usable | `000906.SH` | `sh.000906` | 中证800指数 | 4,707 | 2007-01-15 至 2026-06-01 |
| `csi_a100` | 中证A100 | usable | `000903.SH` | `sh.000903` | 中证A100指数 | 4,864 | 2006-05-29 至 2026-06-01 |

补充观察：

- `csi_300`、`csi_500`、`csi_a100` 的沪深双代码都可用，第一版按 SH 代码作为选中代码。
- `cnindex_1000` 当前只有 SZ 候选，第一版使用 `399311.SZ`。

## 模型边界

| 模型 | 粒度 | 直接依赖 | 用途 |
|---|---|---|---|
| `int_index_basic_snapshot` | 每个指数一行 | `stg_baostock__query_stock_basic` | BaoStock 指数 universe 和基础信息快照 |
| `int_index_quotes_daily` | 每 `security_code`、`trade_date` 一行 | `int_index_basic_snapshot`、`stg_baostock__query_history_k_data_plus_daily` | 指数日行情和价格指数日收益 |
| `int_benchmark_basic_snapshot` | 每个 benchmark 选中指数一行 | `int_index_basic_snapshot` | 当前允许的组合绩效 benchmark 维表 |
| `int_benchmark_returns_daily` | 每 `benchmark_key`、`security_code`、`trade_date` 一行 | `int_benchmark_basic_snapshot`、`int_index_quotes_daily` | benchmark 日收益输入 |

## 模型设计

### `int_index_basic_snapshot`

位置：

- SQL：`pipeline/elt/models/intermediate/int_index_basic_snapshot.sql`
- YAML：`pipeline/elt/models/intermediate/int_index_basic_snapshot.yml`
- 设计文档：`docs/design/dbt_layer/fleur_intermediate/int_index_basic_snapshot.md`

粒度：每 `security_code` 一行。

直接依赖：

- `ref('stg_baostock__query_stock_basic')`

核心逻辑：

1. 从 `stg_baostock__query_stock_basic` 过滤 `security_type = 'index'`。
2. 保留指数基础信息、上市状态和 source-local 类型字段。
3. 不处理跨源指数主数据归并，不做沪深双代码合并。

建议字段：

| 字段 | 说明 |
|---|---|
| `security_code` | canonical 指数代码，例如 `000300.SH` 或 `399300.SZ` |
| `security_local_code` | 6 位本地指数代码，仅在 basic snapshot 中保留 |
| `exchange_code` | `SH` / `SZ` / `BJ`，仅在 basic snapshot 中保留 |
| `index_name` | BaoStock 当前快照中的指数名称 |
| `ipo_date` | BaoStock 指数上市日期 |
| `out_date` | BaoStock 指数退市日期，NULL 保留 |
| `listing_status_code` | BaoStock 上市状态编码 |
| `listing_status` | BaoStock 上市状态标签 |
| `is_listed` | 当前快照是否上市 |
| `security_type_code` | BaoStock 证券类型编码，指数应为 `2` |
| `security_type` | 固定为 `index` |

### `int_index_quotes_daily`

位置：

- SQL：`pipeline/elt/models/intermediate/int_index_quotes_daily.sql`
- YAML：`pipeline/elt/models/intermediate/int_index_quotes_daily.yml`
- 设计文档：`docs/design/dbt_layer/fleur_intermediate/int_index_quotes_daily.md`

粒度：每 `security_code`、`trade_date` 一行。

直接依赖：

- `ref('int_index_basic_snapshot')`
- `ref('stg_baostock__query_history_k_data_plus_daily')`

核心逻辑：

1. 用 `security_code` 将指数 basic snapshot join 到 BaoStock 日行情 staging。
2. 只输出指数行情，不让股票、ETF 或可转债混入。
3. 使用 BaoStock 日行情中的 `prev_close_price` 计算价格指数日收益：

```sql
if(prev_close_price > 0, close_price / prev_close_price - 1, null)
```

4. 日行情表不输出 `security_local_code` 和 `exchange_code`；需要这些维度时由下游按 `security_code` join `int_index_basic_snapshot`。

建议字段：

| 字段 | 说明 |
|---|---|
| `security_code` | canonical 指数代码 |
| `trade_date` | 指数行情交易日 |
| `open_price` | 开盘点位 |
| `high_price` | 最高点位 |
| `low_price` | 最低点位 |
| `close_price` | 收盘点位 |
| `prev_close_price` | BaoStock 原始前收盘点位 |
| `return_daily` | 价格指数简单日收益 |
| `volume` | 成交量，沿用 BaoStock source-local 口径 |
| `amount` | 成交金额，沿用 BaoStock source-local 口径 |
| `is_suspend` | BaoStock `tradestatus = 0` 派生状态 |

### `int_benchmark_basic_snapshot`

位置：

- SQL：`pipeline/elt/models/intermediate/int_benchmark_basic_snapshot.sql`
- YAML：`pipeline/elt/models/intermediate/int_benchmark_basic_snapshot.yml`
- 设计文档：`docs/design/dbt_layer/fleur_intermediate/int_benchmark_basic_snapshot.md`

粒度：每 `benchmark_key`、`security_code` 一行；第一版每个 `benchmark_key` 只保留一个选中 `security_code`。

直接依赖：

- `ref('int_index_basic_snapshot')`

核心逻辑：

1. 在模型内维护一个小型 benchmark 映射 CTE，列出当前允许的 benchmark 名称、选中代码和 benchmark key。
2. 从 `int_index_basic_snapshot` 过滤映射表中的指数代码。
3. 同一指数存在沪深双代码时只保留 SH 选中代码；不在第一版输出备用代码多行。
4. 输出 `benchmark_key` 供下游表达“沪深 300”这类业务基准，输出 `security_code` 供追溯具体 BaoStock 指数。

当前 benchmark 清单：

| benchmark_key | 指数名称 | 选中代码 |
|---|---|---|
| `csi_a100` | 中证A100 | `000903.SH` |
| `csi_300` | 沪深300 | `000300.SH` |
| `csi_500` | 中证500 | `000905.SH` |
| `csi_800` | 中证800 | `000906.SH` |
| `csi_1000` | 中证1000 | `000852.SH` |
| `cnindex_1000` | 国证1000 | `399311.SZ` |

实现前必须用 `dbt show` 或等价查询确认以上候选代码是否存在于 `int_index_basic_snapshot`。当前 benchmark 清单只保留 raw profile 已验证可用的指数；模型实现不得通过 NULL 补齐或空记录把不可用 benchmark 伪装为可用。

建议字段：

| 字段 | 说明 |
|---|---|
| `benchmark_key` | 稳定业务 key，例如 `csi_300` |
| `benchmark_name` | 中文 benchmark 名称 |
| `security_code` | 具体 BaoStock 指数 canonical 代码 |
| `security_local_code` | 6 位本地指数代码，仅在 basic snapshot 中保留 |
| `exchange_code` | 指数代码所属交易所，仅在 basic snapshot 中保留 |
| `index_name` | BaoStock 当前快照中的指数名称 |
| `listing_status` | BaoStock 当前快照上市状态标签 |
| `is_listed` | 当前快照是否上市 |

### `int_benchmark_returns_daily`

位置：

- SQL：`pipeline/elt/models/intermediate/int_benchmark_returns_daily.sql`
- YAML：`pipeline/elt/models/intermediate/int_benchmark_returns_daily.yml`
- 设计文档：`docs/design/dbt_layer/fleur_intermediate/int_benchmark_returns_daily.md`

粒度：每 `benchmark_key`、`security_code`、`trade_date` 一行。

直接依赖：

- `ref('int_benchmark_basic_snapshot')`
- `ref('int_index_quotes_daily')`

核心逻辑：

1. 用 `security_code` 将 benchmark basic snapshot join 到指数日行情。
2. 输出 benchmark 日收益，不重新计算指数 universe。
3. 日收益表不输出 `security_local_code` 和 `exchange_code`；需要这些维度时由下游按 `benchmark_key`、`security_code` join `int_benchmark_basic_snapshot`。

建议字段：

| 字段 | 说明 |
|---|---|
| `benchmark_key` | 稳定业务 key，例如 `csi_300` |
| `security_code` | 具体 BaoStock 指数 canonical 代码 |
| `trade_date` | 交易日 |
| `close_price` | benchmark 收盘点位 |
| `prev_close_price` | benchmark 前收盘点位 |
| `return_daily` | 价格指数简单日收益 |

## 实施阶段

### 阶段 1：数据存在性和代码映射 profile

目标：确认用户指定指数代码在 BaoStock staging 中是否存在，并记录实际 canonical 代码。

工作项：

1. 用 `dbt show` 查询 `stg_baostock__query_stock_basic` 中 `security_type = 'index'` 的候选代码。
2. 对沪深双代码指数分别确认是否都有基础信息和日行情。
3. 对未标明交易所的代码确认实际存在后缀，特别是 `930050`、`000510`、`000852` 和 `931643`。
4. 将 profile 结果写入 `docs/design/dbt_layer/fleur_intermediate/int_benchmark_basic_snapshot.md` 和 `docs/design/dbt_layer/fleur_intermediate/int_benchmark_returns_daily.md`。

完成标准：

- 可用 benchmark_key 的选中代码已按 SH 优先规则确定。
- 不存在或无行情的候选代码不进入第一版 benchmark 清单。

### 阶段 2：新增 `int_index_basic_snapshot`

目标：建立 BaoStock 指数基础信息 intermediate 模型。

工作项：

1. 新增 SQL、YAML 和设计文档。
2. 使用 `security_type = 'index'` 过滤指数 universe。
3. 保留 `security_local_code` 和 `exchange_code`，作为指数基础维度字段。
4. 添加结构性测试。

测试建议：

- `security_code`: `not_null`、`unique`、`cn_security_code_format`。
- `security_local_code`: `not_null`。
- `exchange_code`: `not_null`、accepted values。
- `security_type`: accepted values，仅允许 `index`。
- `security_type_code`: accepted values，仅允许 `2`。

完成标准：

- `int_index_basic_snapshot` 一行一个 BaoStock 指数。
- 输出中没有股票、ETF 或可转债。

### 阶段 3：新增 `int_index_quotes_daily`

目标：建立 BaoStock 指数日行情 intermediate 模型。

工作项：

1. 新增 SQL、YAML 和设计文档。
2. 从 `int_index_basic_snapshot` join BaoStock 日行情 staging。
3. 计算 `return_daily`。
4. 明确不输出 `security_local_code` 和 `exchange_code`。
5. 添加结构性测试。

测试建议：

- `security_code`: `not_null`、`cn_security_code_format`。
- 组合键 `security_code`, `trade_date`: 唯一且非空。
- `trade_date`: `not_null`。
- `close_price`: `not_null`，如果 profile 证明指数行情收盘点位无缺失。

完成标准：

- `int_index_quotes_daily` 可以按指数代码和日期稳定返回价格指数日收益。
- 日行情表不携带本地代码和交易所代码。

### 阶段 4：新增 `int_benchmark_basic_snapshot`

目标：从指数基础信息中过滤组合绩效所需 benchmark 清单。

工作项：

1. 新增 SQL、YAML 和设计文档。
2. 在模型内使用显式 benchmark 映射 CTE，保留 `benchmark_key`、`benchmark_name` 和 `security_code`。
3. 从 `int_index_basic_snapshot` join 映射表。
4. 对同一 benchmark 的沪深双代码不保留备用代码多行；第一版只输出选中代码。

测试建议：

- 组合键 `benchmark_key`, `security_code`: 唯一且非空。
- `benchmark_key`: accepted values，取值为本计划清单。
- `security_code`: `not_null`、`cn_security_code_format`。

完成标准：

- 所有当前 benchmark_key 至少映射到一个已验证存在的指数代码。
- benchmark 维度字段只在 basic snapshot 中维护。

### 阶段 5：新增 `int_benchmark_returns_daily`

目标：从 benchmark basic 和指数日行情模型生成 benchmark 日收益。

工作项：

1. 新增 SQL、YAML 和设计文档。
2. 从 `int_benchmark_basic_snapshot` join `int_index_quotes_daily`。
3. 输出 benchmark 日收益，不输出 `security_local_code` 和 `exchange_code`。
4. 添加结构性测试。

测试建议：

- 组合键 `benchmark_key`, `security_code`, `trade_date`: 唯一且非空。
- `benchmark_key`: accepted values，取值为本计划清单。
- `return_daily`: 允许 NULL，但 NULL 行应能通过 `prev_close_price <= 0`、首日或源缺失解释。

完成标准：

- 下游可以只读取 `int_benchmark_returns_daily` 获取 benchmark 价格指数日收益。
- 下游如需交易所或本地代码，通过 `int_benchmark_basic_snapshot` join 获取。

### 阶段 6：文档、验证和交接

目标：让模型边界、收益口径和缺口处理可被后续 worker / mart / API 使用。

工作项：

1. 更新 `docs/design/dbt_layer/fleur_intermediate/` 下四个模型设计文档。
2. 在 YAML column descriptions 中说明价格指数、不含分红再投资和 source lineage。
3. 如后续新增 benchmark 且 profile 发现候选代码不存在，新增待决问题或 job report，而不是在模型中静默补空值。
4. 记录 dbt build 和 sample output。

完成标准：

- 设计文档、YAML 文档和 SQL 逻辑的 grain 一致。
- Q&A 0001 中的 benchmark 输入已能映射到实际 intermediate 模型。

## 禁止模式

1. 禁止直接从 `fleur_raw` 或 `source('raw', ...)` 构造 benchmark；必须经过 staging / intermediate。
2. 禁止修改股票 universe 模型来承载指数行情。
3. 禁止把价格指数 benchmark 伪装为全收益 benchmark。
4. 禁止在 worker 中硬编码 BaoStock 指数过滤逻辑；worker 只消费清洗后的 benchmark return。
5. 禁止在未 profile 前静默删除不存在或无行情的候选指数代码。
6. 禁止在日行情 / 日收益模型中重复输出 `security_local_code` 和 `exchange_code`。

## 最小验证命令

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_index_basic_snapshot int_index_quotes_daily int_benchmark_basic_snapshot int_benchmark_returns_daily
uv run dbt show --project-dir elt --profiles-dir elt \
  --inline "select * from {{ ref('int_benchmark_returns_daily') }}" \
  --limit 50
uv run python elt/scripts/validate_field_glossary.py
```

文档-only 检查：

```bash
make docs-check
git diff --check
```

## 完成标准

1. `int_index_basic_snapshot`、`int_index_quotes_daily`、`int_benchmark_basic_snapshot` 和 `int_benchmark_returns_daily` 均有 SQL、YAML、设计文档和必要测试。
2. benchmark 清单中的每个 `benchmark_key` 至少映射到一个已验证存在的指数代码。
3. `return_daily` 使用价格指数简单收益口径。
4. 日行情 / 日收益模型不输出纯数字本地指数代码和交易所代码。
5. 下游 portfolio performance 计算只需读取 `int_benchmark_returns_daily`，不需要理解 BaoStock raw / staging 细节。
6. 本计划完成后移入 `docs/plans/archive/`，并在相关 job report 中记录执行命令和样例输出。

## 待决问题

1. benchmark basic 第一版使用 inline mapping CTE；若清单继续增长，是否迁移为 dbt seed 或独立小维表。
2. 是否在后续 mart 层新增 `mart_benchmark_returns_daily`，作为 portfolio worker 的长期读取入口。
