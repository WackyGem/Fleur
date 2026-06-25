# Plan 0023: Contract-driven Parquet schema adapter backfill test plan

日期：2026-06-01

状态：Proposed

关联文档：

- `docs/plans/0022-contract-driven-parquet-schema-adapter-implementation-plan.md`
- `docs/RFC/0011-contract-driven-parquet-schema-adapter.md`
- `docs/skills/fleur-dagster-backfill-runbook/SKILL.md`
- `docs/skills/fleur-dagster-backfill-runbook/references/backfill-matrix.md`
- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`

## 1. 背景

Plan 0022 会把 source 层写入 S3 Parquet 的 schema 事实源收敛到 `pipeline/contracts/datasets/*.yml`，并在 `S3IOManager` 写入前强制 schema equality。这个改动会影响所有通过 source/S3 和 ClickHouse raw sync 链路流转的资产。

本计划定义 0022 实施后的 dev 环境重置、小批量回填测试、失败记录和全量回填准入流程。它不是 0022 的代码开发计划，而是 0022 合入后验证真实外部系统、S3 Parquet 和 ClickHouse raw 表能否按新 schema 正常重建的执行计划。

## 2. 目标

完成后应满足：

1. 使用 `deploy/docker-compose.yml` 清空容器卷并重新初始化 dev 环境。
2. 对所有受影响资产执行小批量编排：
   - 年分区资产默认跑 `2020`。
   - 日分区资产跑 `2026-06-01`。
   - 快照或无分区资产直接跑。
   - OCR 只处理 10 张图。
3. 小批量测试失败时，在 `docs/jobs/reports/` 记录带运行日期的报告，包含命令、Run ID、失败现象和修复状态。
4. 小批量测试全部通过后，再按 `docs/skills/fleur-dagster-backfill-runbook` 执行全量回填。

## 3. 非目标

- 不在本计划中修改 0022 的生产代码。
- 不在未清空 dev 容器卷的旧状态上判定 schema 迁移成功。
- 不跳过 source/S3 层直接验证 ClickHouse raw。
- 不把小批量失败当作可忽略事项；失败必须先记录、修复、重跑。
- 不对生产环境执行 `docker compose down -v`。

## 4. 当前事实基线

根据当前 runbook 和 Dagster definitions，受影响资产为全部 34 个注册资产：

| 层 | 数量 | 范围 |
| --- | ---: | --- |
| Source / S3 | 19 | `source/*`，包含 snapshot、stateful OCR、日分区、年分区和 compacted 年分区资产 |
| ClickHouse raw | 15 | `clickhouse/raw/*`，依赖对应 `source/*` 资产 |

分区例外：

- 标准年分区小批量年份为 `2020`。
- `source/jiuyan__action_field_compacted` 和 `clickhouse/raw/jiuyan__action_field_compacted` 的分区从 `2021` 开始；但它们从日分区输入读取数据，小批量应跟随本计划的日分区测试年，使用 `2026`。
- `source/ths__limit_up_pool_compacted` 和 `clickhouse/raw/ths__limit_up_pool_compacted` 的分区从 `2025` 开始；小批量同样使用 `2026`。
- 日分区小批量日期固定为 `2026-06-01`。若执行时发现该日期被上游交易日过滤为空，需要在失败报告中记录，再选择最近有效交易日重跑，不能静默改日期。

## 5. Phase 0: 0022 合入前置质量门禁

在真实回填前，先确认 0022 的代码和 contract 生成物已一致：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests
uv run ruff format --check scheduler/src scheduler/tests contract_tools/src contract_tools/tests
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests -q
uv run pytest scheduler/tests/unit/test_contract_schemas.py -q
cd scheduler
uv run dg check defs
```

完成标准：

- 所有命令通过。
- `uv run dg list defs --target-path scheduler --json` 能列出全部资产和 raw sync jobs。
- 若门禁失败，不进入容器卷清空和真实回填阶段。

## 6. Phase 1: 清空容器卷并重新初始化 dev 环境

从仓库根目录执行：

```bash
set -a
. ./.env
set +a

docker compose --env-file .env -f deploy/docker-compose.yml down -v --remove-orphans
docker compose --env-file .env -f deploy/docker-compose.yml up -d
make wait-rustfs
make dagster-home
```

PostgreSQL 迁移和 definitions 检查：

```bash
cd pipeline/migrate
uv run alembic upgrade head

cd ../
uv run dg check defs --target-path scheduler
```

健康检查：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml ps
docker compose --env-file .env -f deploy/docker-compose.yml exec -T postgres pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB"
docker compose --env-file .env -f deploy/docker-compose.yml exec -T clickhouse \
  clickhouse-client --user "$CLICKHOUSE_USER" --password "$CLICKHOUSE_PASSWORD" --query "SELECT 1"
```

完成标准：

- RustFS bucket 初始化完成。
- PostgreSQL migration head 已应用。
- ClickHouse 可连接。
- Dagster home 已初始化 pool 限制。

## 7. Phase 2: Source / S3 小批量测试

所有 `dg` 命令前先加载环境并初始化 Dagster home：

```bash
set -a
. ./.env
set +a
make dagster-home
cd pipeline
```

### 7.1 Snapshot 和上游基础资产

```bash
uv run dg launch --target-path scheduler --assets "key:source/sina__trade_calendar"
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_stock_basic"
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_list"
```

### 7.2 年分区 source 资产

BaoStock 年分区：

```bash
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition 2020
```

EastMoney 8 个年分区资产：

```bash
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 2020
```

### 7.3 日分区 source 资产

```bash
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__action_field" \
  --partition 2026-06-01

uv run dg launch --target-path scheduler \
  --assets "key:source/ths__limit_up_pool" \
  --partition 2026-06-01
```

### 7.4 OCR 小批量

下载 10 张图：

```bash
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__industry_images" \
  --config-json '{"ops":{"source__jiuyan__industry_images":{"config":{"limit":10,"force_download":false}}}}'
```

OCR 10 张图：

```bash
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__industry_ocr" \
  --config-json '{"ops":{"source__jiuyan__industry_ocr":{"config":{"limit":10,"force_ocr":false,"max_concurrent_requests":6}}}}'
```

发布 OCR snapshot：

```bash
uv run dg launch --target-path scheduler --job jiuyan__industry_ocr_snapshot_job
```

### 7.5 Compacted source 年分区

这两个资产从日分区输入读取数据，因此小批量跟随 `2026-06-01` 的日分区测试，使用 `2026`：

```bash
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__action_field_compacted" \
  --partition 2026

uv run dg launch --target-path scheduler \
  --assets "key:source/ths__limit_up_pool_compacted" \
  --partition 2026
```

完成标准：

- 每个 run 均成功。
- `S3IOManager` 没有报 schema mismatch。
- 每个通过 `s3_io_manager` 写出的 materialization metadata 至少包含 contract/schema hash。
- OCR 相关 metadata 能看到本轮只处理 10 张图，且 snapshot 发布成功。

## 8. Phase 3: ClickHouse raw 小批量测试

ClickHouse raw 必须在对应 source/S3 小批量成功后执行。

### 8.1 Snapshot raw

```bash
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_snapshot_job
```

覆盖资产：

- `clickhouse/raw/sina__trade_calendar`
- `clickhouse/raw/baostock__query_stock_basic`
- `clickhouse/raw/jiuyan__industry_list`
- `clickhouse/raw/jiuyan__industry_ocr_snapshot`

### 8.2 年分区 raw

BaoStock raw：

```bash
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_baostock_job --partition 2020
```

EastMoney raw：

```bash
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_eastmoney_job --partition 2020
```

Market-event compacted raw：

```bash
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_jiuyan_market_event_job --partition 2026
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_ths_market_event_job --partition 2026
```

完成标准：

- raw sync staging、schema validation、partition replace 或 snapshot swap 均成功。
- 年分区 raw 表只写入目标 partition。
- `fleur-contracts validate-clickhouse --all-available` 通过。

## 9. Phase 4: 小批量后数据核验

小批量全部完成后运行：

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

建议追加人工核验：

- Dagster UI 中确认所有小批量 run 为成功。
- 抽查 S3/RustFS 中对应 Parquet 对象存在。
- 抽查 ClickHouse raw 表行数、目标 year partition 和 snapshot 表存在。
- 保存本轮 Run ID 列表，用于失败报告或全量回填准入记录。

完成标准：

- 小批量 source/S3 与 ClickHouse raw 全部通过。
- 没有 schema mismatch、missing contract boundary schema、unexpected field 或 ClickHouse type mismatch。
- 若存在失败，停止进入 Phase 6，先执行 Phase 5。

## 10. Phase 5: 小批量失败报告

只要小批量任一 run 失败，就在 `docs/jobs/reports/` 新增报告。文件名使用运行日期：

```text
docs/jobs/reports/YYYY-MM-DD-contract-driven-parquet-schema-adapter-small-batch-failure.md
```

报告最小模板：

~~~markdown
# Contract-driven Parquet schema adapter small-batch failure

UTC time: YYYY-MM-DDTHH:MM:SSZ

## Scope

- Plan: `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- Failed asset/job:
- Partition or config:
- Run ID:
- Environment reset: yes/no

## Command

```bash
# exact command
```

## Failure

- Error summary:
- First failing step:
- Relevant traceback or validation message:

## Diagnosis

- Expected schema:
- Actual schema:
- Suspected code path:

## Resolution

- Fix PR or commit:
- Rerun command:
- Rerun Run ID:
- Final result:
~~~

处理规则：

- 报告创建后才能继续修复。
- 修复后只先重跑失败资产及其必要上游。
- 失败资产重跑通过并更新报告后，才能恢复剩余小批量测试。

## 11. Phase 6: 全量回填准入和执行

只有 Phase 2、Phase 3、Phase 4 全部通过，且 Phase 5 没有未关闭失败项时，才能执行全量回填。

全量回填必须按 `docs/skills/fleur-dagster-backfill-runbook/SKILL.md` 和 `docs/skills/fleur-dagster-backfill-runbook/references/backfill-matrix.md` 执行，顺序为：

1. Source snapshot 和基础资产。
2. Source 年分区资产。
3. Source 日分区资产，按各自窗口限制分段。
4. OCR 队列只回填 30 张图，不等待 pending 清零；随后发布 snapshot。
5. Source compacted 年分区资产。
6. ClickHouse raw snapshot。
7. ClickHouse raw 年分区资产。
8. `validate-parquet` 和 `validate-clickhouse` 全量核验。

全量回填窗口：

| 资产类型 | 全量策略 |
| --- | --- |
| snapshot / no partition | 直接跑当前 snapshot |
| BaoStock / EastMoney year | 从起始年到当前年，逐年 `--partition YYYY` |
| Jiuyan action_field daily | 最近 90 个自然日，按 runbook 窗口分段；资产内部过滤交易日 |
| THS limit_up_pool daily | 从 `2025-01-01` 到当前有效交易日，按 runbook 窗口分段 |
| Jiuyan OCR | 只跑一次 `limit=30`，不等待 pending 清零，随后发布 snapshot |
| Jiuyan / THS compacted year | 日分区完成后逐年跑 compacted；Jiuyan action_field compacted 只覆盖最近 90 个自然日输入涉及的年份 |
| ClickHouse raw | 对齐已完成的 source/S3 snapshot 和 year partitions |

全量回填也必须在 `docs/jobs/reports/` 记录报告，包含：

- 运行日期和 UTC 时间窗口。
- 资产范围和分区范围。
- 执行命令或脚本。
- Run ID 汇总。
- Parquet 和 ClickHouse 校验结果。
- 遗留失败、人工接受项或需要补跑的分区。

## 12. 禁止模式

- 禁止在没有清空 dev 容器卷的情况下用旧数据判定迁移成功。
- 禁止在 source/S3 小批量失败时继续跑 ClickHouse raw。
- 禁止年分区使用 `--partition-range`。
- 禁止对 OCR 不设 `limit` 直接触发全量。
- 禁止跳过 `docs/jobs/reports/` 失败记录。
- 禁止把 `2020` 强行用于当前分区定义不存在的 compacted market-event 资产。
- 禁止 full backfill 早于 small batch 全部通过。

## 13. 完成标准

计划完成时应有：

- 一份 clean dev 环境小批量执行报告，或失败报告加最终修复记录。
- 全部 34 个受影响资产的小批量 run 成功记录。
- `validate-parquet --all-available` 和 `validate-clickhouse --all-available` 成功。
- 全量回填报告已写入 `docs/jobs/reports/`。
- 若全量回填发现新问题，报告中明确列出待补跑资产、分区和复现命令。
