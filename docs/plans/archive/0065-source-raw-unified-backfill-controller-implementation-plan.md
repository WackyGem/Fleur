# Plan 0065: Source/Raw 统一回填 Controller 实施计划

日期：2026-06-30

状态：Completed

## 背景

RFC 0039 已经确认当前 Dagster 注册资产基线：22 个 `source/*` 资产、17 个 `clickhouse/raw/*` 资产，并提出统一手动回填入口：

- job：`backfill__fetch_sources_to_raw_job`
- controller op：`backfill__fetch_sources_to_raw_controller`

用户只在 Dagster Web UI 中配置 `target_scope`、日期区间、执行模式和少量专项参数，controller 负责生成 source、compacted source 与 ClickHouse raw sync 的真实 materialization runs。

相关依据：

- RFC：[0039-source-raw-backfill-complexity-baseline.md](../../RFC/archive/0039-source-raw-backfill-complexity-baseline.md)
- 当前聚合入口：[definitions.py](../../../pipeline/scheduler/src/scheduler/defs/definitions.py)
- 当前 SourceBundle 契约：[source_bundle.py](../../../pipeline/scheduler/src/scheduler/defs/source_bundle.py)
- 当前 raw specs：[specs.py](../../../pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py)
- 当前 BaoStock 专用 controller：[backfill_controller.py](../../../pipeline/scheduler/src/scheduler/defs/baostock/backfill_controller.py)
- 回填 runbook：[fleur-dagster-backfill-runbook](../../skills/fleur-dagster-backfill-runbook/SKILL.md)

## 目标

- 新增 `backfill__fetch_sources_to_raw_job`，支持在 Dagster Web UI 手动 launch。
- 新增 `backfill__fetch_sources_to_raw_controller`，暴露 RFC 0039 中定义的 typed config。
- 用显式策略注册表实现 `target_scope` 到资产链路、前置 snapshot、分区策略和执行阶段的映射。
- `target_scope` 首期覆盖当前全部 22 个 source assets 和 17 个 raw assets。
- 支持 `dry_run=true`，只输出结构化 `BackfillPlan`，不提交子 run。
- 支持 `execution_mode=full` 和 `execution_mode=raw_only`。
- 支持 `refresh_prerequisite_snapshots` 只刷新当前 `target_scope` 的显式 source snapshot prerequisites。
- 明确 Web UI config 契约、字段默认值、`backfill.id` 生成规则和实际传给子资产的 op config 映射。
- 为 controller run 和所有子 run 写入统一 `backfill.*` tags，支持在 Web UI 按 `backfill.id` 检索。
- 用依赖顺序串行推进子 run：上游失败时不提交依赖它的下游阶段。
- 替换或收敛当前 BaoStock 专用 shell-out controller，避免长期在 Dagster job 内调用 `uv run dg launch`。
- 增加单元测试和 definition 检查，防止 registry 与真实 source/raw 资产漂移。

## 非目标

- 不改变现有 source asset 的业务抓取逻辑。
- 不改变 S3 layout、Parquet schema、ClickHouse raw 表、contract registry 或 dbt source。
- 不把 snapshot 资产改造成日期分区资产。
- 不在 controller op 内直接调用 source 业务函数、raw sync service 或 ClickHouse 写入逻辑。
- 不默认触发 dbt staging、intermediate、mart 或 Furnace 计算。
- 不在首期实现 `source_only`、`resume_from_failed_step` 等额外恢复模式。
- 不在 controller 实现前重写 `docs/jobs/dagster-backfill-2026.md`；runbook 更新应基于 dry run 和真实运行验证结果。

## 当前事实基线

| 主题 | 当前事实 |
| --- | --- |
| Source assets | 22 个已注册 `source/*` assets，来自 `SOURCE_BUNDLES`。 |
| Raw assets | 17 个已注册 `clickhouse/raw/*` assets，来自 enabled ClickHouse raw specs。 |
| 资产形态 | source 层包含 snapshot、daily、year 和 source-only 中间资产；raw 层包含 snapshot 和 year。 |
| 三段链路 | BaoStock 日 K、Jiuyan action field、THS limit up pool 走 daily source -> year compacted source -> year raw。 |
| 直接 year 链路 | EastMoney 9 个 F10 资产和 ChinaBond government bond 走 year source -> year raw。 |
| Snapshot 链路 | Sina trade calendar、BaoStock stock basic、Jiuyan industry list、Jiuyan OCR snapshot 进入 snapshot raw。 |
| OCR 链路 | Jiuyan industry images 和 OCR 是 source-only 中间资产，由 `jiuyan_ocr_pipeline` 覆盖。 |
| 现有专用 controller | `baostock__history_k_data_year_range_backfill_job` 通过 `subprocess.run("uv run dg launch ...")` 串行提交多段命令。 |

## RFC 目标覆盖审计

2026-06-30 review 结论：本计划已经覆盖 RFC 0039 的主要设计目标，当前没有发现需要推翻方案的逻辑矛盾。需要补充明确的是 year 分区资产的区间精度、in-process 子 run 的等待语义，以及真实回填验收的外部数据前置条件。

| RFC 0039 设计目标 | 本计划覆盖方式 | 审计结论 |
| --- | --- | --- |
| 用户只在 Web UI 配置目标范围和日期区间 | Phase 2 定义 typed config；Phase 3 注册 `backfill__fetch_sources_to_raw_job`。 | 已覆盖。 |
| Controller 将业务区间映射为 daily range、year partitions 和 snapshot prerequisites | Phase 1/2 建立 registry、日期校验、year 展开和 snapshot prerequisite 展开。 | 已覆盖。 |
| 实际写数据仍通过 Dagster asset materialization 保留 event、日志、partition 状态和 lineage | Phase 4 通过 child materialization runs 提交 `BackfillStep`，不在 controller 内直接写 source/raw。 | 已覆盖。 |
| 回填入口按数据域或 source bundle 收敛，降低手工 asset key 命令复杂度 | 目标明确为数据域 `target_scope`；source bundle、raw table 和 partition strategy 只作为 registry 内部字段。 | 已覆盖。 |
| 首期覆盖当前全部 source/raw 资产 | Phase 1 完成标准要求 `all_fetch_sources_to_raw` 覆盖 22 个 source assets 和 17 个 raw assets，并用测试防漂移。 | 已覆盖。 |
| 支持 `dry_run` 和可审阅计划 | Phase 2/3 要求结构化 `BackfillPlan`、人类可读日志和不提交子 run。 | 已覆盖。 |
| `refresh_prerequisite_snapshots` 只影响当前 scope 的显式前置 source snapshots | 设计约束和 Phase 2 完成标准均要求 scope-local prerequisite 展开；`raw_only` 不执行 prerequisite。 | 已覆盖。 |
| BaoStock 专用 shell-out controller 收敛到统一入口 | Phase 5 要求迁移或停止注册 `baostock__history_k_data_year_range_backfill_job`。 | 已覆盖。 |

审计补充后的边界：

- 对 daily source，`start_date` / `end_date` 精确映射为 Dagster daily partition range。
- 对 year source/raw，日期区间只用于选择受影响的 year partitions；year partition 本身不是日粒度过滤。若某个 source asset 支持 `refresh_until_date` 或 `cutoff_trade_date`，controller 只收窄该 year 的上界，不承诺跳过同一年 `start_date` 之前的数据。
- 首期真实执行路径使用 in-process implicit asset job。它同步返回终态，因此不需要异步轮询；如果未来改为异步 run submission adapter，则必须补显式轮询间隔、超时和终态判断。
- 小范围真实回填验收依赖 source S3 partition 或远端接口可用性。2026-06-30 已用 `target_scope=chinabond`、`2006` 年 full 模式完成成功真实验收，记录见 [Source/Raw 统一回填 Controller 验证记录](../../jobs/reports/2026-06-30-source-raw-unified-backfill-controller.md)。

## 设计约束

- `target_scope` 必须是显式枚举或显式注册值；不允许通过模糊 asset key 匹配兜底。
- 策略注册表必须能机械验证：所有当前 source/raw 资产被至少一个首期 scope 覆盖。
- 组合 scope 必须去重 asset key，并保留依赖顺序。
- `all_raw_yearly` 只覆盖 year raw 链路，不包含 snapshot reference data 和 OCR pipeline。
- `all_fetch_sources_to_raw` 显式组合全部首期 scope，覆盖当前全部 source/raw 资产。
- `raw_only` 只提交 raw sync runs，假设对应 source/compacted source materialization 已存在。
- `refresh_prerequisite_snapshots=true` 只作用于当前 `target_scope` 的 `prerequisite_snapshots`，不刷新无关 snapshot。
- Snapshot prerequisite 只刷新 source snapshot；同步到 raw 需要显式选择 `snapshot_reference_data` 或包含它的组合 scope。
- `BackfillStep` 必须支持一个或多个 asset keys；同一个 step 只能包含同一自然阶段、同一 partition 选择和兼容 run config 的资产集合。例如 EastMoney 9 个 year source assets 可以按同一年份作为一组 source step，不能与 raw sync 或 snapshot 混在同一个 step。
- 子 run config 不允许凭字符串猜 op name。需要从当前 `AssetsDefinition.node_def.name`、已有 job 定义或显式 registry 字段取得唯一 op name；找不到唯一来源时实施应停止并补事实。
- `overwrite_source_partitions`、`jiuyan_ocr_limit`、`jiuyan_force_download` 和 `jiuyan_force_ocr` 必须通过 registry 中的 per-asset config mapping 传递；不支持对应 config 的资产不得收到无效 config。
- `backfill.id` 由 controller run 生成并写入 controller 日志和所有 child run tags。首期不新增用户输入字段；格式应包含 controller run id 或其稳定短 id，避免同一 `target_scope` 和日期区间重复运行时 tag 冲突。
- “当前日期之后的区间”校验使用现有回填语义一致的 Asia/Shanghai 日期。
- Controller 应优先使用 Dagster 内部 run submission 能力；若实现阶段确认本地 Dagster OSS 限制导致无法稳定落地，允许短期保留 CLI launch 兼容路径，但必须隔离在 submission adapter 内并在计划执行报告中记录原因。
- CLI launch 兼容路径只有在能保留 RFC 要求的 `backfill.*` tags 时才可作为真实执行路径；如果无法写入 tags，只能作为临时 dry-run/dev 诊断路径，不能算作本计划完成。
- 子 run 提交后，controller 必须等待当前依赖阶段到终态，再决定是否提交下游阶段。

## 实施阶段

### Phase 1: 建立 BackfillPlan 领域模型和策略注册表

修改范围：

- 新增 `pipeline/scheduler/src/scheduler/defs/automation/source_raw_backfill.py` 或等价跨源 automation 模块。
- 新增 `pipeline/scheduler/tests/unit/automation/test_source_raw_backfill.py`。

开发任务：

1. 定义纯 Python 数据结构：
   - `BackfillControllerConfig`
   - `BackfillTargetSpec`
   - `BackfillStep`
   - `BackfillPlan`
   - `BackfillPartitionSelection`
   - `BackfillAssetOpConfigMapping`
2. 定义首期 `target_scope` 注册表：
   - `baostock_daily_kline`
   - `market_events`
   - `eastmoney_f10`
   - `chinabond`
   - `snapshot_reference_data`
   - `jiuyan_ocr_pipeline`
   - `all_raw_yearly`
   - `all_fetch_sources_to_raw`
3. 为每个 scope 显式声明：
   - 策略类型
   - source assets
   - compacted source assets
   - raw assets
   - prerequisite snapshots
   - 是否需要日期区间
   - 是否允许 OCR 专项参数
   - 可接收的 source config 字段和对应 op config key
   - step asset grouping 规则
4. 实现组合 scope 展开、去重和依赖排序。
5. 建立 raw spec 反查表，确保 raw asset 与 source dependency 来自 `ENABLED_CLICKHOUSE_RAW_TABLE_SPECS`。
6. 建立 asset key 到 op name 的事实映射，来源必须是当前 Dagster asset definitions 或显式已有 job，不允许从 asset key 拼接猜测。

完成标准：

- 注册表覆盖 RFC 0039 声明的全部 scope。
- 单元测试证明 `all_fetch_sources_to_raw` 覆盖当前 22 个 source assets 和 17 个 raw assets。
- 单元测试证明 `all_raw_yearly` 不包含 snapshot 和 OCR pipeline。
- 单元测试证明 duplicate snapshot 在组合 scope 中只出现一次。
- 单元测试证明同一 snapshot 同时作为 prerequisite 和 scope 目标时只 materialize 一次，再按依赖顺序执行后续 raw sync。
- 单元测试证明 registry 中每个需要 op config 的 asset 都能解析到唯一 op name。
- invalid `target_scope` 明确失败。

### Phase 2: 实现区间映射、配置校验和 dry-run 计划生成

修改范围：

- `pipeline/scheduler/src/scheduler/defs/automation/source_raw_backfill.py`
- `pipeline/scheduler/tests/unit/automation/test_source_raw_backfill.py`

开发任务：

1. 实现 Web UI typed config，字段与 RFC 0039 保持一致：
   - `target_scope`
   - `start_date`
   - `end_date`
   - `execution_mode`，默认 `full`
   - `refresh_prerequisite_snapshots`，默认 `false`
   - `overwrite_source_partitions`，默认 `false`
   - `jiuyan_ocr_limit`，默认 `100`，显式 `null` 表示不限制
   - `jiuyan_force_download`，默认 `false`
   - `jiuyan_force_ocr`，默认 `false`
   - `dry_run`，默认 `true`
2. 校验 `start_date`、`end_date`：
   - daily/year scope 必填。
   - pure snapshot/OCR scope 可为空。
   - mixed scope 只要包含 daily/year 子 scope 就必填。
   - `start_date <= end_date`。
   - 默认拒绝 Asia/Shanghai 当前日期之后的区间。
3. 实现日期区间到 year partitions 的映射。
4. 实现 daily partition range 表达：`YYYY-MM-DD...YYYY-MM-DD`。
5. 为 partial current year 生成资产 config：
   - BaoStock 日 K daily/compacted 继续使用 `cutoff_trade_date`。
   - 其他 year source/raw 仅按 year partition 运行，除非对应资产已经有明确 cutoff 配置。
6. 实现 `execution_mode=full`：
   - prerequisite source snapshots
   - source snapshot targets
   - daily source ranges
   - year source partitions
   - compacted source partitions
   - OCR pipeline steps
   - raw sync partitions/snapshots
7. 实现 `execution_mode=raw_only`：
   - 只生成有 raw spec 的 raw sync steps。
   - daily -> compacted -> raw 链路按 year partitions 选择 raw assets。
8. 实现 OCR config 传递：
   - `jiuyan_ocr_limit`
   - `jiuyan_force_download`
   - `jiuyan_force_ocr`
9. 实现 `backfill.id` 生成，并写入 dry-run payload。
10. 生成结构化 `BackfillPlan` 日志 payload，供 dry run 输出，必须包含 date range 到 year partitions 的映射。

完成标准：

- dry-run plan 能复现 RFC 0039 中 `baostock_daily_kline` 的预期顺序。
- `refresh_prerequisite_snapshots=true` 只展开当前 scope 的 prerequisite source snapshots。
- `raw_only` 不包含 source 或 compacted source steps。
- OCR 默认 `jiuyan_ocr_limit=100`，只有显式 `null` 才表示不限制。
- 不支持覆盖或 OCR config 的 asset 不会收到对应 op config。
- dry-run payload 包含唯一 `backfill.id`、用户输入区间、展开后的 year partitions 和每个 step 的 asset selection。
- 所有计划生成逻辑不依赖 Dagster instance，可以纯单元测试。

### Phase 3: 注册 Dagster job/op 并接入 definitions

修改范围：

- `pipeline/scheduler/src/scheduler/defs/automation/source_raw_backfill.py`
- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/tests/integration/test_definitions_and_schedules.py`

开发任务：

1. 新增 `backfill__fetch_sources_to_raw_controller` op，暴露 typed config。
2. 新增 `backfill__fetch_sources_to_raw_job`，只包含 controller op。
3. 将该 job 注册到顶层 `defs()` 的 jobs 集合，而不是挂到某一个 source bundle。
4. `dry_run=true` 时：
   - 生成 `BackfillPlan`。
   - 逐步打印人类可读计划。
   - 记录结构化 metadata 或 JSON 日志。
   - 不提交任何子 run。
5. `dry_run=false` 时先接入 submission adapter 接口，但 adapter 可在 Phase 4 完成真实提交。

完成标准：

- `uv run dg list defs --target-path scheduler --json` 能看到 `backfill__fetch_sources_to_raw_job`。
- Web UI launch config 字段与 RFC 0039 一致。
- dry run 不需要外部服务即可执行。
- definition integration test 覆盖 job 已注册。

### Phase 4: 实现子 run submission adapter 和依赖阶段等待

修改范围：

- `pipeline/scheduler/src/scheduler/defs/automation/source_raw_backfill.py`
- `pipeline/scheduler/tests/unit/automation/test_source_raw_backfill.py`
- 需要时新增 fakes/helpers。

开发任务：

1. 先用代码和 Dagster 本地验证确认可用的内部 run submission API。
2. 定义 `BackfillRunSubmitter` 接口，隔离 controller 与具体 Dagster submission 机制。
3. 实现首选内部 submission：
   - 根据 `BackfillStep` 选择一个或多个 asset keys。
   - 设置 partition 或 partition range。
   - 设置 step-level run config。
   - 写入 RFC 0039 规定的 `backfill.*` tags。
4. 实现阶段等待和状态轮询：
   - 明确 terminal statuses。
   - 日志记录 child run id、asset selection、partition selection 和当前状态。
   - 首期 in-process adapter 通过 `execute_in_process` 同步返回终态，不需要轮询。
   - 如果后续新增异步 run submission adapter，轮询间隔和超时必须使用显式常量或 config，不能无声无限等待。
   - prerequisite snapshot 失败则不继续。
   - daily source 失败则不提交对应 year compacted/raw。
   - compacted source 失败则不提交对应 year raw。
   - raw sync 失败保留已完成 source/compacted materialization。
5. 子 run 失败时，controller run 失败，并在日志中输出失败 asset、partition、step 和 child run id。
6. 如果内部 submission 无法在本地 Dagster OSS 稳定使用，实现临时 CLI adapter：
   - 只在 adapter 内构造 `uv run dg launch`。
   - 继续写入统一 tags。
   - 保持与内部 adapter 相同的 `BackfillStep` 输入和测试契约。

完成标准：

- 子 run tag 包含 `backfill.kind`、`backfill.id`、`backfill.target_scope`、`backfill.start_date`、`backfill.end_date`、`backfill.step`，year 阶段包含 `backfill.year`。
- 同一个 `backfill.id` 下的 child runs 能在 Web UI 中按 tag 聚合检索。
- 单元测试用 fake submitter 验证提交顺序和失败短路。
- `raw_only` 失败恢复路径可单独提交 raw steps。
- CLI adapter 若存在，必须证明能写入 `backfill.*` tags；否则只能作为临时诊断路径，且必须有明确 TODO 和运行报告说明，不散落在业务逻辑里。

### Phase 5: 迁移 BaoStock 专用 controller 并收敛旧入口

修改范围：

- `pipeline/scheduler/src/scheduler/defs/baostock/backfill_controller.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/definitions.py`
- `pipeline/scheduler/tests/unit/baostock/test_backfill_controller.py`
- `pipeline/scheduler/tests/unit/automation/test_source_raw_backfill.py`

开发任务：

1. 将 BaoStock 日 K year-range 回填能力迁移到 `target_scope=baostock_daily_kline`。
2. 删除或停止注册 `baostock__history_k_data_year_range_backfill_job`。
3. 保留必要的纯函数测试用例，迁移到通用 `BackfillPlan` 测试。
4. 确认 `overwrite_source_partitions` 只传给支持覆盖的 source asset。
5. 确认 partial current year 的 `cutoff_trade_date` 继续作用于 BaoStock daily 和 compacted source。

完成标准：

- definitions 中不再注册 BaoStock 专用 year-range controller job。
- BaoStock 的 dry-run plan 与旧 controller 命令序列语义等价。
- BaoStock 现有 controller 单元测试被通用 plan tests 覆盖或删除。

### Phase 6: 文档、runbook 和运行报告闭环

修改范围：

- `docs/RFC/archive/0039-source-raw-backfill-complexity-baseline.md`
- `docs/skills/fleur-dagster-backfill-runbook/SKILL.md`
- `docs/skills/fleur-dagster-backfill-runbook/references/backfill-matrix.md`
- `docs/jobs/dagster-backfill-2026.md`
- `docs/jobs/reports/`
- `docs/architecture/scheduler-architecture.md`
- 需要时更新 `docs/architecture/scheduler-module-boundaries.md`

开发任务：

1. 在 RFC 0039 中记录实现偏差和最终确认项。
2. 用真实 dry run 输出更新回填 runbook。
3. 完成一次小范围 dry run 运行报告，建议先用 `snapshot_reference_data` 或单年 `chinabond`。
4. 完成一次小范围真实提交路径运行报告；如果真实写入因缺少 source S3 partition 或远端数据不可用失败，报告必须记录 controller run id、`backfill.id`、child run id、失败原因和恢复建议，并将“成功写入 raw 的小范围真实回填”保留为剩余运行验收项。
5. 如果新增了 `defs/automation/source_raw_backfill.py` 之外的新目录，同步更新 scheduler module boundaries。
6. 将旧 BaoStock 专用命令标注为被统一入口替代。

完成标准：

- 用户可以按 runbook 在 Web UI 中手动启动 `backfill__fetch_sources_to_raw_job`。
- runbook 不再要求用户记忆 source/raw 多段 asset key 命令作为首选路径。
- 运行报告包含 controller run id、`backfill.id`、目标范围、日期区间、dry-run/真实结果和失败恢复建议。

## 测试策略

| 层级 | 覆盖内容 |
| --- | --- |
| 单元测试 | scope registry 覆盖、组合 scope 去重、日期到 year 映射、config 校验、dry-run plan、`raw_only`、OCR config、失败短路。 |
| 集成测试 | `backfill__fetch_sources_to_raw_job` 注册、definitions 可加载、raw specs 与 registry dependency 一致。 |
| 手动 dry run | Web UI 或 `dg launch` 启动 controller，确认日志计划和 tags。 |
| 小范围真实回填 | 选择低风险 snapshot 或单年 year raw 链路，确认子 run、lineage、partition 状态和 ClickHouse raw sync 结果。 |

## 验证命令

文档变更：

```bash
make docs-check
git diff --check
```

代码实施后至少运行：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/automation scheduler/tests/unit/baostock scheduler/tests/unit/clickhouse scheduler/tests/integration/test_definitions_and_schedules.py
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

手动 dry-run 验证示例：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_sources_to_raw_job \
  --config-file /tmp/backfill-fetch-sources-to-raw-dry-run.yaml
```

## 完成标准

- `backfill__fetch_sources_to_raw_job` 已注册并可在 Dagster Web UI 手动启动。
- 用户只需配置 `target_scope`、日期区间和 RFC 0039 中定义的少量参数。
- `dry_run=true` 输出完整执行计划且不提交子 run。
- `dry_run=false` 能提交真实 asset materialization runs，并按依赖顺序等待和短路。
- `target_scope` 覆盖当前全部 source/raw 资产，并有测试防漂移。
- BaoStock 专用 shell-out controller 已迁移或停止注册。
- runbook 和至少一份运行报告已基于实际 dry run/真实运行更新。
- 文档、Python、Dagster definitions 检查通过。

## 完成记录

2026-06-30 完成首期实施并归档。验收证据：

- 成功 dry run：`target_scope=baostock_daily_kline`，`2026-01-01..2026-06-30`，controller run id `404c43d0-6e6a-412d-8f52-9fd353da3fa1`。
- 失败传播验证：`target_scope=chinabond`，`execution_mode=raw_only`，source parquet 缺失时 child run 失败并由 controller 失败短路。
- 成功真实回填：`target_scope=chinabond`，`execution_mode=full`，`2006-01-01..2006-12-31`，controller run `d0eb1436-e984-4b04-b423-8d7bf49b42ce` 成功，source child run `e093c9bd-dc70-4054-8afa-e0422b521acc` 成功，raw child run `d7f393c5-d901-4e9b-8074-ec90460a4bf4` 成功，三者均带同一个 `backfill.id=chinabond-2006-01-01-2006-12-31-d0eb1436e984`。
- ClickHouse 核验：`fleur_raw.chinabond__government_bond WHERE year = 2006` 返回 `count=214`，日期范围 `2006-03-01..2006-12-31`。
- 运行报告：[2026-06-30-source-raw-unified-backfill-controller.md](../../jobs/reports/2026-06-30-source-raw-unified-backfill-controller.md)。
