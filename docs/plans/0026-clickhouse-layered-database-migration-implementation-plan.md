# Plan 0026: ClickHouse 四层 database 迁移实施计划

日期：2026-06-02

状态：Draft

关联文档：

- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- `docs/ADR/0009-clickhouse-layered-databases.md`
- `docs/RFC/0014-clickhouse-layered-database-migration.md`
- `docs/skills/dg-backfill-runbook/SKILL.md`
- `pipeline/contracts/README.md`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/raw_sync.py`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/clickhouse.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- `pipeline/elt/dbt_project.yml`
- `pipeline/elt/profiles.yml`

相关 skills：

- `fleur-harness`：计划、文档、质量门禁和运行报告治理。
- `dagster-expert`：Dagster asset/job/partition materialization、`dg launch` 和 definitions 校验。
- `dg-backfill-runbook`：回填命令、分区范围和运行记录。
- `using-dbt-for-analytics-engineering`：dbt source/model routing、staging build 和 manifest 校验。
- `running-dbt-commands`：dbt parse/build/list 命令格式。
- `clickhouse-best-practices`：ClickHouse schema、insert、mutation、partition 规则。

## 1. 目标

把 RFC 0014 落成可执行迁移：将 ClickHouse 从历史 `raw` / `analytics` 命名切换到四层 database：

```text
fleur_raw
fleur_staging
fleur_intermediate
fleur_marts
```

完成后应满足：

1. `fleur_raw` 由 Dagster ClickHouse raw sync 执行路径自动创建。
2. 15 个 raw-enabled contracts 的 ClickHouse raw database 全部为 `fleur_raw`。
3. generated dbt `sources.yml` 的 source name 仍为 `raw`，但 physical schema/database 指向 `fleur_raw`。
4. dbt `staging`、`intermediate`、`marts` 分别 materialize 到 `fleur_staging`、`fleur_intermediate`、`fleur_marts`。
5. 迁移执行采用“清除旧 ClickHouse 层库，然后从 S3 通过 Dagster raw sync 重新物化”的主路径。
6. raw 迁移完成后有详细验收报告，证明 Dagster 物化、S3 读取、ClickHouse 建库建表、staging 校验和替换协议都正常。
7. 用户不需要手工敲 ClickHouse DDL/DML；破坏性清库和验收 SQL 必须由受控 CLI、脚本或 Dagster asset/job 封装执行。

## 2. 非目标

本计划不做以下事情：

1. 不新增业务口径的 intermediate/mart 模型。
2. 不把 dbt staging/intermediate/marts 字段事实写入 `pipeline/contracts`。
3. 不重设每张 raw 表的 `ORDER BY`、partition、type，除非 contract 当前事实有明确错误。
4. 不让 dbt 装载 raw 或替换 raw partition。
5. 不保留 `raw` / `analytics` 作为长期业务 database。
6. 不在 definitions 加载阶段连接 ClickHouse、S3 或外部服务。
7. 不通过人工复制 SQL 完成迁移；任何例外 SQL 都必须进入受控工具和运行报告。

## 3. 当前事实基线

当前扫描结果：

- raw-enabled contracts：15 个。
- `pipeline/contracts/datasets/*.yml` 中 raw-enabled datasets 仍使用 `clickhouse_raw.database: raw`。
- `pipeline/elt/models/sources.yml` 仍使用 `schema: raw`。
- `pipeline/elt/profiles.yml` 默认 `CLICKHOUSE_DBT_SCHEMA` 为 `analytics`。
- 直接使用 `source('raw', ...)` 的 staging SQL 当前有 3 个：
  - `pipeline/elt/models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.sql`
  - `pipeline/elt/models/staging/eastmoney/stg_eastmoney__equity_history.sql`
  - `pipeline/elt/models/staging/sina/stg_sina__trade_calendar.sql`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/raw_sync.py` 已在 `_prepare_staging()` 中执行 `CREATE DATABASE IF NOT EXISTS <spec.clickhouse_database>`。
- `pipeline/scheduler/tests/unit/clickhouse/test_raw_sync.py` 当前仍断言 raw sync 会创建 `raw` database。
- `pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py` 当前有分组 raw sync jobs，但没有显式 `clickhouse__raw_sync_all_job`。

raw-enabled datasets：

| Dataset | 当前目标 | 目标目标 |
| --- | --- | --- |
| `baostock__query_history_k_data_plus_daily` | `raw` | `fleur_raw` |
| `baostock__query_stock_basic` | `raw` | `fleur_raw` |
| `eastmoney__balance` | `raw` | `fleur_raw` |
| `eastmoney__cashflow_sq` | `raw` | `fleur_raw` |
| `eastmoney__cashflow_ytd` | `raw` | `fleur_raw` |
| `eastmoney__dividend_allotment` | `raw` | `fleur_raw` |
| `eastmoney__dividend_main` | `raw` | `fleur_raw` |
| `eastmoney__equity_history` | `raw` | `fleur_raw` |
| `eastmoney__income_sq` | `raw` | `fleur_raw` |
| `eastmoney__income_ytd` | `raw` | `fleur_raw` |
| `jiuyan__action_field_compacted` | `raw` | `fleur_raw` |
| `jiuyan__industry_list` | `raw` | `fleur_raw` |
| `jiuyan__industry_ocr_snapshot` | `raw` | `fleur_raw` |
| `sina__trade_calendar` | `raw` | `fleur_raw` |
| `ths__limit_up_pool_compacted` | `raw` | `fleur_raw` |

## 4. 迁移策略

### 4.1 主路径：清库后从 S3 重物化

本计划采用破坏性但更可验证的主迁移路径：

```text
记录旧库基线
  -> 校验 S3 source 覆盖范围
  -> 代码切换到 fleur_raw / fleur_* 层
  -> 受控清除旧 ClickHouse 层库
  -> Dagster raw sync 从 S3 重新物化 fleur_raw
  -> dbt build staging 到 fleur_staging
  -> 验收并写 job report
```

清库范围：

- 必须清除：`raw`、`analytics`、`fleur_raw`、`fleur_staging`、`fleur_intermediate`、`fleur_marts`。
- 清除前必须采集旧库基线并确认 S3 可重建。
- 清除动作必须由受控工具执行，不允许要求用户手工执行 `DROP DATABASE`。

采用该路径的理由：

- 同时验证 Dagster raw sync 是否能从 S3 恢复 raw。
- 同时验证 ClickHouse resource、`s3()` 读取、建库建表、staging 表、schema 校验、LowCardinality 校验、snapshot exchange 和 year partition replace。
- 避免历史 `raw` 中的手工修补或漂移被复制到新 `fleur_raw`。

### 4.2 例外路径

只有当某个 dataset 无法从 S3 重建时，才允许临时 table-to-table copy。例外必须满足：

- 在报告中记录 dataset、原因、缺失的 S3 object 或分区。
- copy 逻辑由受控脚本生成执行。
- copy 后仍按同一验收清单校验。
- 后续必须补 S3 backfill 或单独记录技术债。

### 4.3 单入口迁移 runner

迁移执行应提供一个单入口 runner，把 baseline、reset、Dagster 重物化、dbt build、验收和 report 串起来。推荐形态：

```bash
cd pipeline
uv run fleur-contracts clickhouse-layer migrate --confirm <token-from-baseline-report>
```

runner 内部职责：

- 加载 `.env` 或明确校验必要环境变量已存在。
- 校验 `DAGSTER_HOME`，并提示先运行 `make dagster-home`；如实现允许，也可以由 runner 调用等价初始化逻辑。
- 执行 Phase 0 baseline 或读取指定 baseline artifact。
- 执行 Phase 4 reset。
- 按 baseline 中的 snapshot/year 范围提交 Dagster raw sync runs。
- 等待或轮询 run 结果，记录 run id、asset key、partition key、状态和失败原因。
- 执行 dbt parse/list/build 和项目校验。
- 执行 raw/dbt 层验收查询。
- 生成或更新 migration report。

本文档中的 `dg`、`dbt` 和 SQL 命令是 runner 内部实现和故障排查模板，不是要求用户逐条手工执行的迁移流程。

## 5. ClickHouse 规则约束

实施时必须遵守：

- Per `schema-pk-plan-before-creation`：不借四层迁移重新随意改 raw 表 `ORDER BY`；如需改，单独开 schema 迁移。
- Per `schema-partition-lifecycle`：year partition 用于回填、替换和生命周期管理，不能把 database 分层当 partition 策略。
- Per `insert-batch-size`：raw 重物化继续通过 ClickHouse server-side `INSERT ... SELECT FROM s3(...)` 批量装载。
- Per `insert-mutation-avoid-update` / `insert-mutation-avoid-delete`：清库重建和分区替换优先，不做频繁 row-level mutation。
- Per `insert-optimize-avoid-final`：迁移后不常规执行 `OPTIMIZE TABLE ... FINAL` 作为修复手段。

## 6. 实施阶段

### Phase 0：迁移前基线和安全闸

目标：在任何破坏性动作前，证明当前数据范围、S3 可重建范围和 Dagster asset 范围都已知。

实施内容：

- 新增或扩展受控验收脚本，建议命名为：

```text
pipeline/contract_tools/src/fleur_contracts/clickhouse_layer_migration.py
```

或拆成 CLI 子命令：

```bash
cd pipeline
uv run fleur-contracts clickhouse-layer baseline
uv run fleur-contracts clickhouse-layer reset --confirm <generated-token>
uv run fleur-contracts clickhouse-layer validate
uv run fleur-contracts clickhouse-layer report
```

- baseline 命令采集：
  - `system.databases` 中 `raw`、`analytics`、`fleur_*` 是否存在。
  - `raw` 中所有 raw-enabled table 的 row count。
  - year-partitioned tables 的 `year -> row_count`。
  - `system.columns` schema 指纹。
  - `system.parts` active parts 数和 partition 清单。
  - Dagster raw sync asset key 清单。
  - contract raw-enabled dataset 清单。
- 校验 S3 覆盖范围：
  - snapshot dataset 的 expected object key 存在。
  - year-partitioned dataset 的 expected years 存在。
  - compacted market-event yearly object 存在。
- 生成迁移分区清单：
  - 输出机器可读 manifest，例如 `docs/jobs/reports/<date>-clickhouse-layered-database-partitions.json`。
  - snapshot dataset 记录 expected object key。
  - year-partitioned dataset 记录 expected years、每个 year 的 S3 object key、object size、last modified 和可选 parquet row count。
  - compacted market-event dataset 同样按 year 记录 expected object。
  - manifest 是 Phase 5 重物化和 Phase 7 验收的唯一分区范围输入。
- 输出 baseline artifact：

```text
docs/jobs/reports/<date>-clickhouse-layered-database-baseline.md
```

安全闸：

- 如果 S3 覆盖范围无法解释，不允许进入清库阶段。
- 如果 ClickHouse 连接权限不足以 `DROP DATABASE` / `CREATE DATABASE` / `CREATE TABLE` / `ALTER TABLE REPLACE PARTITION`，不允许进入清库阶段。
- 如果当前工作树有未纳入本迁移的生产代码改动，报告必须记录。

完成标准：

- baseline report 已生成。
- 15 个 raw-enabled datasets 全部在 baseline 中出现。
- snapshot tables 和 year-partitioned tables 的迁移范围明确。
- 分区 manifest 已生成，并覆盖所有要迁移的 S3 snapshot/year objects。
- 破坏性清库确认 token 已生成但未执行。

验证命令：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run dg list defs --target-path scheduler --json
```

### Phase 1：contract 和 generated catalog 切换到 `fleur_raw`

目标：让所有 contract-driven raw specs 和 dbt raw source catalog 指向 `fleur_raw`。

实施内容：

- 修改 15 个 raw-enabled dataset contract：

```yaml
clickhouse_raw:
  database: fleur_raw
```

- 修改或确认 `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`：
  - source name 保持 `raw`。
  - physical `schema` 从 contract raw database 读取，目标为 `fleur_raw`。
  - `meta.clickhouse_raw_table` 输出 `fleur_raw.<table>`。
- 重新生成：
  - `pipeline/elt/models/sources.yml`
  - `docs/references/data_dict/*.md`
- 更新 contract/generate 相关测试：
  - 不再断言 `clickhouse_raw_table: raw.demo__raw_table`。
  - 新增或更新断言 `schema: fleur_raw` 和 `clickhouse_raw_table: fleur_raw.<table>`。

完成标准：

- `rg -n "database: raw" pipeline/contracts/datasets` 无 raw-enabled 结果。
- `pipeline/elt/models/sources.yml` 中 source name 仍为 `raw`，schema 为 `fleur_raw`。
- generated data dictionary 展示 `fleur_raw.<table>`。
- scheduler specs 加载后 `spec.clickhouse_database == "fleur_raw"`。

验证命令：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests
```

### Phase 2：Dagster raw sync 调整和全量 job 覆盖

目标：确保 Dagster raw sync 能完整重建 `fleur_raw`，且测试不再绑定历史 `raw`。

实施内容：

- 更新 `pipeline/scheduler/tests/unit/clickhouse/test_raw_sync.py` 中数据库断言：
  - 从 `CREATE DATABASE IF NOT EXISTS raw` 改为 `fleur_raw`。
  - metadata 断言包含 `clickhouse_database=fleur_raw`。
- 在 `pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py` 新增：

```text
clickhouse__raw_sync_all_job
```

selection 覆盖所有 `ENABLED_CLICKHOUSE_RAW_TABLE_SPECS` 对应 assets。

- 确认现有分组 jobs 仍保留：
  - snapshot
  - baostock
  - eastmoney
  - jiuyan market event
  - ths market event
- 更新 definitions integration test，确保 all job 注册。
- 确认 raw sync materialization metadata 中：
  - `clickhouse_database`
  - `clickhouse_table`
  - `staging_table`
  - `partition_key`
  - `s3_object_key`
  - `loaded_row_count`
  - `raw_row_count_after_replace`
  - schema hash

完成标准：

- `uv run dg list defs --target-path scheduler --json` 中能看到 `clickhouse__raw_sync_all_job`。
- 15 个 `clickhouse/raw/*` assets 仍注册。
- raw sync 单元测试覆盖 `fleur_raw`。

验证命令：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/clickhouse scheduler/tests/integration/test_definitions_and_schedules.py
uv run dg check defs --target-path scheduler
```

### Phase 3：dbt 三层 database routing

目标：dbt 自动创建或准备 `fleur_staging`、`fleur_intermediate`、`fleur_marts`，并把模型落到对应 database。

实施内容：

- 新增目录：

```text
pipeline/elt/models/intermediate/
pipeline/elt/models/marts/
```

- 更新 `pipeline/elt/dbt_project.yml`：

```yaml
models:
  elt:
    staging:
      +materialized: view
      +schema: fleur_staging
    intermediate:
      +materialized: view
      +schema: fleur_intermediate
    marts:
      +materialized: table
      +schema: fleur_marts
```

- 验证 dbt-clickhouse 是否会把 `target.schema` 和 model `+schema` 拼接。如果会拼接出非目标名称，新增 `pipeline/elt/macros/generate_schema_name.sql`，让 `fleur_staging`、`fleur_intermediate`、`fleur_marts` 作为完整 ClickHouse database 名。
- 如 adapter 无法在空库场景可靠创建三层 database，新增幂等 macro/hook：

```sql
CREATE DATABASE IF NOT EXISTS fleur_staging;
CREATE DATABASE IF NOT EXISTS fleur_intermediate;
CREATE DATABASE IF NOT EXISTS fleur_marts;
```

- profile 默认 schema 从 `analytics` 改为非业务兜底库，例如 `fleur_dbt_scratch`，或保留但确保没有模型落入 `analytics`。推荐改为 `fleur_dbt_scratch`，避免继续使用历史目标。
- 新增 manifest/routing 校验脚本或扩展现有 `pipeline/elt/scripts/validate_field_glossary.py`：
  - staging nodes schema/database 为 `fleur_staging`。
  - intermediate nodes schema/database 为 `fleur_intermediate`。
  - marts nodes schema/database 为 `fleur_marts`。
  - raw sources physical schema 为 `fleur_raw`。

完成标准：

- 现有 staging models materialize 到 `fleur_staging`。
- 当前没有 intermediate/marts 模型也能通过目录和 manifest routing 校验。
- `analytics` 不再是任何 model 的目标 schema。
- staging SQL 中 `source('raw', ...)` 不被改名。

验证命令：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt list --project-dir elt --profiles-dir elt --select "source:raw.*" --output json
uv run dbt build --project-dir elt --profiles-dir elt --select staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
uv run python elt/scripts/validate_field_glossary.py
```

### Phase 4：受控清库

目标：清除历史和目标 ClickHouse 层库，为从 S3 完整重物化创造干净环境。

实施内容：

- 通过受控 CLI 执行清库，不要求用户手工敲 SQL。
- CLI 必须要求显式确认 token，例如：

```bash
cd pipeline
uv run fleur-contracts clickhouse-layer reset --confirm <token-from-baseline-report>
```

- 清库 SQL 由工具执行，范围固定：

```sql
DROP DATABASE IF EXISTS raw;
DROP DATABASE IF EXISTS analytics;
DROP DATABASE IF EXISTS fleur_raw;
DROP DATABASE IF EXISTS fleur_staging;
DROP DATABASE IF EXISTS fleur_intermediate;
DROP DATABASE IF EXISTS fleur_marts;
```

- 清库后立即验证这些 database 不存在。
- 清库动作写入 report：
  - 执行时间。
  - 清库 database 列表。
  - 执行用户，不记录 password。
  - baseline report 路径。
  - confirmation token hash。

安全要求：

- 只允许在明确的迁移窗口执行。
- 必须先完成 Phase 0 baseline。
- 必须先完成 Phase 1-3 代码和静态校验。
- 如果 ClickHouse 里存在非本项目 database，不得触碰。

完成标准：

- `raw`、`analytics`、`fleur_*` 均不存在。
- reset report 已记录。
- 没有用户手工 DDL。

验证命令：

```bash
cd pipeline
uv run fleur-contracts clickhouse-layer validate-empty
```

### Phase 5：Dagster 从 S3 重物化 `fleur_raw`

目标：通过 Dagster raw sync 全链路从 S3 重建 `fleur_raw`，同时验证 Dagster 与 ClickHouse 对接。

执行方式：

- 首选由 Phase 4.3 的单入口 runner 自动执行。
- runner 必须按 Phase 0 生成的分区 manifest 展开 snapshot assets 和 year partitions。
- 直接运行 `dg launch` 只作为开发调试、单分区重试或 runner 故障排查手段。

runner 执行前置：

```bash
set -a; . ./.env; set +a
make dagster-home
cd pipeline
```

runner 内部推荐执行顺序：

1. 先跑 snapshot raw sync：

```bash
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_snapshot_job
```

2. 对 year-partitioned assets 先跑一个小切片：

```bash
uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily" --partition 2026
```

3. 小切片通过后，按分区 manifest 的 dataset/year 循环运行完整范围。跨很多年时按年份循环，不用超长 partition-range。

4. EastMoney 使用现有 `clickhouse__raw_sync_eastmoney_job`，但仍按年份逐年运行，避免一次性难以恢复。

5. compacted market-event assets 按 compacted year partitions 逐年运行。

可选实现：

- 包装 CLI 自动展开 Phase 0 分区 manifest，并逐个调用 `dg launch` 或 Dagster API。
- 每个 run 的 run id、asset key、partition key、状态写入运行报告草稿。

禁止把 `clickhouse__raw_sync_all_job` 当作“所有历史分区已迁移”的证明。该 job 只能证明 selection 覆盖所有 raw sync assets；partitioned assets 仍必须由 runner 显式传入每个 expected partition key。

完成标准：

- `fleur_raw` 自动创建。
- 15 个 raw-enabled tables 全部存在。
- snapshot tables row count > 0，除非 contract 明确 `allow_empty`。
- year-partitioned tables 的 expected years 与 Phase 0 分区 manifest 完全一致。
- Dagster materialization metadata 中 database 全部为 `fleur_raw`。

验证命令：

```bash
cd pipeline
uv run dg list defs --target-path scheduler --json
uv run fleur-contracts clickhouse-layer validate-raw
```

### Phase 6：dbt 三层建库和 staging build

目标：验证 dbt 能在清库后自动创建建模层 database，并从 `fleur_raw` 构建现有 staging。

实施内容：

- 由单入口 runner 执行 dbt parse/list/build。
- 下列命令作为 runner 内部步骤和故障排查模板：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt list --project-dir elt --profiles-dir elt --select "source:raw.*" --output json
uv run dbt build --project-dir elt --profiles-dir elt --select staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

- 运行 field glossary 和 staging readiness 校验：

```bash
uv run python elt/scripts/validate_field_glossary.py
uv run python elt/scripts/validate_staging_readiness.py
```

- 验证 ClickHouse database：
  - `fleur_staging` 存在。
  - 当前 3 个 staging views/tables 存在。
  - `fleur_intermediate` 和 `fleur_marts` 至少被 dbt hook 或 schema creation 准备出来；如果没有模型触发创建，必须由幂等 macro/hook 创建。
  - `analytics` 不被重建为模型目标库。

完成标准：

- dbt staging build 成功。
- `source('raw', ...)` 解析到 `fleur_raw`。
- 当前 staging models 落到 `fleur_staging`。
- 三层建模 database 均存在。

### Phase 7：迁移验收报告

目标：把迁移结果变成可审计事实，而不是口头确认。

输出：

```text
docs/jobs/reports/<date>-clickhouse-layered-database-migration-report.md
```

报告结构：

```markdown
# ClickHouse Layered Database Migration Report

日期：
执行人：
Git commit / working tree：
环境：

## 1. Scope

## 2. Baseline Summary

## 3. Reset Summary

## 4. Dagster Rematerialization Runs

## 5. Raw Table Validation

## 6. dbt Layer Validation

## 7. Failures / Exceptions

## 8. Acceptance Checklist

## 9. Follow-ups
```

完成标准：

- 报告包含所有验收项结果。
- 任何失败、跳过和例外都有 owner 和后续动作。
- 报告不包含 secrets。
- 报告引用执行命令、run id、dataset、partition range 和校验摘要。

## 7. 详细验收清单

### 7.1 静态代码和生成物

- [ ] 15 个 raw-enabled contracts 的 `clickhouse_raw.database` 均为 `fleur_raw`。
- [ ] `pipeline/elt/models/sources.yml` 只有 source name `raw`，physical schema 为 `fleur_raw`。
- [ ] `pipeline/elt/models/sources.yml` 中 `meta.clickhouse_raw_table` 均为 `fleur_raw.<table>`。
- [ ] `docs/references/data_dict/*.md` 不再把 raw-enabled table 展示为 `raw.<table>`。
- [ ] `pipeline/elt/dbt_project.yml` 配置 staging/intermediate/marts 三层 schema。
- [ ] `pipeline/elt/profiles.yml` 不再默认把模型落到 `analytics`。
- [ ] `pipeline/scheduler/tests/unit/clickhouse/test_raw_sync.py` 不再断言 `raw` database。
- [ ] `clickhouse__raw_sync_all_job` 已注册，且覆盖所有 enabled raw sync assets。
- [ ] 未把 staging/intermediate/marts 字段事实写入 contracts。

### 7.2 质量门禁

- [ ] `uv run fleur-contracts validate` 通过。
- [ ] `uv run fleur-contracts generate --check` 通过。
- [ ] `uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests` 通过。
- [ ] `uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests` 通过。
- [ ] `uv run pytest scheduler/tests contract_tools/tests` 通过。
- [ ] `uv run dg check defs --target-path scheduler` 通过。
- [ ] `uv run dbt parse --project-dir elt --profiles-dir elt` 通过。
- [ ] `uv run python elt/scripts/validate_field_glossary.py` 通过。
- [ ] `uv run python elt/scripts/validate_staging_readiness.py` 通过。

### 7.3 清库前安全

- [ ] baseline report 已生成。
- [ ] baseline 记录旧 `raw` row counts。
- [ ] baseline 记录旧 `raw` year partition row counts。
- [ ] baseline 记录旧 `raw` schema fingerprints。
- [ ] baseline 记录 S3 snapshot object 覆盖情况。
- [ ] baseline 记录 S3 year partition object 覆盖情况。
- [ ] 分区 manifest 已生成，包含 snapshot object 和 year object 的 expected key。
- [ ] 分区 manifest 中没有 unexplained missing object。
- [ ] baseline 记录 ClickHouse user 具备所需 DDL/DML 权限。
- [ ] 清库 confirmation token 已生成。
- [ ] 迁移窗口已确认。

### 7.4 清库验收

- [ ] reset 由受控 CLI/脚本执行，不是用户手工 SQL。
- [ ] `raw` 不存在。
- [ ] `analytics` 不存在。
- [ ] `fleur_raw` 不存在。
- [ ] `fleur_staging` 不存在。
- [ ] `fleur_intermediate` 不存在。
- [ ] `fleur_marts` 不存在。
- [ ] reset report 记录清库范围和时间。

### 7.5 Dagster raw 重物化

- [ ] `fleur_raw` 由 Dagster raw sync 自动创建。
- [ ] 15 个 raw-enabled tables 均存在于 `fleur_raw`。
- [ ] 每个 snapshot asset 至少有一次成功 materialization。
- [ ] 每个 year-partitioned asset 的 manifest expected years 均成功 materialize。
- [ ] 没有只运行 `clickhouse__raw_sync_all_job` 就跳过 partition manifest 展开的情况。
- [ ] materialization metadata 中 `clickhouse_database` 均为 `fleur_raw`。
- [ ] materialization metadata 中 `s3_object_key` 与 expected object key 匹配。
- [ ] materialization metadata 中 `loaded_row_count` 与 `raw_row_count_after_replace` 一致。
- [ ] 失败 run 已重试或记录为例外。

### 7.6 raw 表级数据验收

对每张 raw-enabled table：

- [ ] `fleur_raw.<table>` 存在。
- [ ] `system.columns` 字段名、顺序和类型与 contract 一致。
- [ ] schema hash 与 contract `clickhouse_schema_hash` 一致。
- [ ] row count > 0，除非 `allow_empty=True`。
- [ ] snapshot table 总行数与 Dagster materialization metadata 一致。
- [ ] year-partitioned table 每个 year row count 与 Dagster metadata 或 S3 source profile 一致。
- [ ] year-partitioned table 的 actual year 集合等于分区 manifest expected year 集合。
- [ ] year-partitioned table 中 `min(year) = max(year) = partition_key` 对每个分区成立。
- [ ] LowCardinality 校验未超过项目阈值。
- [ ] active parts 数无异常爆炸。
- [ ] staging 临时表按设计清理或只保留最近一次可解释 staging。

### 7.7 dbt 层验收

- [ ] `fleur_staging` 存在。
- [ ] `fleur_intermediate` 存在。
- [ ] `fleur_marts` 存在。
- [ ] 现有 staging models build 成功。
- [ ] staging relations 位于 `fleur_staging`。
- [ ] raw sources 在 manifest 中指向 `fleur_raw`。
- [ ] 没有 model relation 指向 `analytics`。
- [ ] `source('raw', ...)` 未被改名。
- [ ] `validate_field_glossary.py` 通过。
- [ ] `validate_staging_readiness.py` 通过。

### 7.8 文档和运行报告

- [ ] `docs/jobs/reports/<date>-clickhouse-layered-database-baseline.md` 已提交。
- [ ] `docs/jobs/reports/<date>-clickhouse-layered-database-migration-report.md` 已提交。
- [ ] 报告包含命令、run id、dataset、partition range、结果和异常。
- [ ] 报告不包含 secrets。
- [ ] ADR/RFC/plan 链接互相可追溯。
- [ ] 后续是否删除 residual staging tables 或旧 run artifacts 有明确结论。

## 8. 回滚和失败处理

### 8.1 清库前失败

处理：

- 不执行 reset。
- 保留现有 `raw` / `analytics`。
- 修复 contract/dbt/Dagster 问题后重新 baseline。

### 8.2 清库后、重物化前失败

处理：

- 优先修复 ClickHouse 连接、权限或 Dagster definitions。
- 因为旧库已删除，恢复路径是从 S3 继续重物化，不回滚到旧库。
- 如果发现 S3 严重缺失，使用 baseline report 标记影响范围，并决定是否用备份或例外 copy 恢复。

### 8.3 部分 raw asset 重物化失败

处理：

- 按 dataset/partition 重试。
- 不用手工修表。
- 对 year partition 可以重复运行同一 partition；raw sync 的 replace 协议应保持幂等。
- 失败超过阈值时暂停 dbt build，避免 downstream 消费不完整 raw。

### 8.4 dbt build 失败

处理：

- 不改 raw。
- 修复 dbt routing、source schema、field glossary 或 staging SQL。
- 重新运行定向 `dbt build --select staging`。

## 9. 禁止模式

- 禁止用户手工执行 `DROP DATABASE`、`CREATE DATABASE`、`INSERT SELECT` 完成迁移。
- 禁止把 dbt source name 从 `raw` 改成 `fleur_raw`。
- 禁止在 `fleur_raw` 中 materialize dbt staging/intermediate/marts。
- 禁止在 migration 中顺手改 raw 表字段语义、`ORDER BY` 或类型，除非有单独 schema 变更说明。
- 禁止用 row-level UPDATE/DELETE 修复 raw 迁移缺口。
- 禁止跳过 baseline 直接清库。
- 禁止验收只检查表存在，不检查数据和 partition。
- 禁止在报告中写入 ClickHouse password、S3 secret 或完整敏感连接串。

## 10. 允许保留的例外

- 如果当前没有 intermediate/marts 业务模型，目录和 database routing 仍必须存在；可以不新增业务模型。
- `fleur_intermediate` 和 `fleur_marts` 可通过 dbt hook/macro 预创建，即使暂时无 relation。
- 如果某个 historical year 的 S3 object 不存在，可在报告中记录为 deferred，但不得假装已迁移。
- 如果某个 source-only dataset 没有 `clickhouse_raw`，不进入本迁移范围。

## 11. 实施计划自审

### 11.1 已发现并修复的逻辑缺口

| 缺口 | 修复 |
| --- | --- |
| RFC 0014 说“不默认删除旧库”，但本次需求要求可彻底清除旧库 | 本计划把清库设为主路径，但加上 baseline、安全 token、受控 CLI 和报告要求 |
| 清库会导致无法与旧 `raw` 对比 | Phase 0 先采集旧库 row count、partition、schema fingerprint；清库后用 baseline 和 Dagster metadata 验收 |
| 只靠现有分组 jobs 可能漏跑 dataset | Phase 2 要求新增 `clickhouse__raw_sync_all_job`，并在 definitions test 中校验覆盖 |
| dbt 没有 intermediate/marts 模型时可能不会建库 | Phase 3 要求 macro/hook 或 routing 校验确保 `fleur_intermediate`、`fleur_marts` 存在 |
| dbt `+schema` 可能与 target schema 拼接 | Phase 3 要求验证并在必要时实现 `generate_schema_name` macro 固定完整 database 名 |
| 单元测试仍绑定 `raw` database | Phase 2 明确更新 raw sync 单测和 metadata 断言 |
| 用户不想手工敲命令，但清库/验收需要 SQL | Phase 0/4/7 要求把 SQL 封装在受控 CLI/脚本中，文档里的 SQL 只表达语义 |
| 计划中列出多个 `dg` / `dbt` 命令，容易被理解为手工步骤 | 新增 Phase 4.3 单入口 runner；Phase 5/6 命令改为 runner 内部步骤和排障模板 |
| `clickhouse__raw_sync_all_job` 容易被误认为能自动迁移所有历史分区 | Phase 0 新增分区 manifest；Phase 5 明确 partitioned assets 必须按 manifest 逐分区展开 |
| 清库后部分 S3 缺失会造成不可用 | Phase 0 加 S3 覆盖闸门；Phase 8 规定缺失时暂停或走例外路径 |

### 11.2 仍需实施时确认的问题

- `fleur-contracts clickhouse-layer ...` 子命令放在 `contract_tools` 是否最终合适；如果运行时依赖 Dagster instance 太重，可能需要拆到 scheduler CLI。
- `clickhouse__raw_sync_all_job` 仅用于 asset selection 覆盖检查；完整历史迁移必须由 runner 按分区 manifest 逐 partition 展开。
- `analytics` 是否存在非本项目临时对象；Phase 0 必须列出后再决定是否清除。
- ClickHouse 用户是否有 `DROP DATABASE` 权限；没有时需要运维侧预授权，但仍不要求用户手工执行 DDL。
- 是否要把 migration report 生成自动化为命令输出，还是先由脚本产出 Markdown 草稿再人工补结论。

## 12. 完成后的维护动作

- 将本计划状态改为 `Completed`，写入完成日期。
- 如果执行产生可复用命令，沉淀到 `docs/skills/dg-backfill-runbook/SKILL.md` 或新增 ClickHouse migration runbook。
- 如果 `fleur-contracts clickhouse-layer` 成为长期工具，补充到 `AGENTS.md` 的质量门禁或运行手册入口。
- 将旧 `raw` / `analytics` 命名相关历史文档标注为迁移前状态，避免后续误用。
