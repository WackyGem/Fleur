# engines Rust 代码质量与结构优化实施方案

日期：2026-06-08

## 背景

本方案针对 `engines/` Rust workspace 的代码质量和项目结构做一次彻底整理。项目当前无历史包袱，优化策略采用一刀切重构：优先建立长期清晰的 crate/module 边界，不为旧内部路径保留兼容层；代码迁移完成后再统一运行质量门禁，避免边拆边测形成低效反馈循环。

本文件同时记录扫描结论、实施方案和本次落地结果。

## 当前基线

当前 workspace 包含三个 crate：

- `furnace`：CLI binary。
- `furnace-core`：纯指标计算库。
- `furnace-io`：ClickHouse I/O、RowBinary、调度、写入和 summary。

扫描事实：

- `engines/crates/furnace-io/src/lib.rs` 已达到 6,436 行，聚合了多指标 SQL、请求模型、执行器、读取、分组、并行计算、写入、校验和测试。
- `engines/crates/furnace/src/main.rs` 已达到 1,095 行，聚合了 CLI 入口、参数解析、校验、帮助文本和测试。
- `furnace-core` 已拆出 `indicators/` 和 `operators/`，结构方向正确，但公开 API 仍由 `lib.rs` 大量 re-export，模块级 API 治理还不够严格。
- `engines/README.md` 仍描述为 KDJ 第一阶段，但源码已经包含 KDJ、MA、RSI、BOLL，文档与实现状态不一致。
- `engines/Cargo.toml` 仅设置了 `unsafe_code = "forbid"`，缺少 workspace 级 Clippy、rustdoc 和公开 API 文档约束。
- 根 `.gitignore` 已忽略 `target/`，本地存在的 `engines/target/` 是 ignored 构建产物，不是已跟踪污染。

统一质量基线结果：

- `cargo fmt --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace`：通过，93 个单元测试和 4 个 doctest 全部通过。

结论：当前不是“红灯修复”问题，而是结构债务已经超过后续指标扩展的舒适边界。应趁无历史包袱时一次性重整模块、公共 API 和测试布局。

## 主要问题

### 1. `furnace-io` 单文件承担过多职责

`furnace-io/src/lib.rs` 同时包含：

- 默认表名、字段名、批量大小等常量。
- KDJ/MA/RSI/BOLL 的 DDL 和 staging SQL。
- 写入模式、请求结构、summary 结构。
- ClickHouse executor 和环境变量解析。
- 输入 SQL、previous state SQL、append safety、replace cascade。
- RowBinary decode/encode。
- 指标输出计算和 Rayon 并行调度。
- 插入、staging 校验、分区替换。
- 31 个单元测试。

这会导致后续新增指标时持续复制 `resolve_*`、`read_*`、`calculate_*`、`insert_*`、`retain_old_*` 模式，单文件审查成本快速上升。

### 2. CLI 参数解析重复且不可扩展

`furnace/src/main.rs` 中 KDJ、MA、RSI、BOLL 各自维护 `CommandConfig::parse/validate/to_request`，字段重复度高。帮助文本是单个长字符串，新增参数容易漏改测试或文档。

### 3. 多指标共享概念没有上升为领域类型

目前多处通过字符串和函数名前缀表达指标差异，例如 table、column、write mode、input range、effective output range、lookback。缺少统一的指标规格或运行计划类型，导致“同一工作流，不同指标”的关系隐含在函数命名中。

### 4. 公共 API 面过宽

`furnace-core` 和 `furnace-io` 的 `lib.rs` 暴露了较多结构、常量和函数。部分 public item 是测试或 CLI 需要用到的内部构件，并不一定应该成为长期 crate API。现在没有 `missing_docs` 或 rustdoc link 检查，公共 API 扩张缺少门槛。

### 5. 错误模型仍偏手写

库 crate 里存在手写 `fmt::Display` 和 `std::error::Error` 实现。当前可以工作，但长期维护上不如统一 typed error 策略清晰。后续应对 `furnace-core` 与 `furnace-io` 使用结构化错误模型，CLI 只负责将错误映射为 exit code 和用户可读消息。

### 6. 测试与实现同文件绑定过重

当前测试覆盖不错，但 `furnace-io` 单文件测试跟实现混在一起。随着模块拆分，应把测试按能力分层，否则一次结构变化会让测试文件也继续膨胀。

## 目标结构

### workspace

保留当前三个 crate，不新增 crate，先通过 module 重构解决主要问题：

```text
engines/
└── crates/
    ├── furnace/
    │   └── src/
    │       ├── main.rs          # 只保留进程入口和 exit code 映射
    │       ├── cli.rs           # 命令枚举、全局解析入口
    │       ├── commands/        # kdj/ma/rsi/boll 命令配置
    │       └── output.rs        # help/json 输出策略
    ├── furnace-core/
    │   └── src/
    │       ├── indicators/      # kdj/ma/rsi/boll 纯计算
    │       ├── operators/       # sma/ema/stddev 基础算子
    │       └── lib.rs           # 最小稳定 facade
    └── furnace-io/
        └── src/
            ├── lib.rs           # 最小 public facade
            ├── clickhouse/      # executor、env、SQL 执行边界
            ├── rowbinary/       # RowBinary read/write
            ├── sql/             # DDL、select、replace、validation SQL
            ├── request.rs       # request/write mode/common config
            ├── summary.rs       # run summary/performance/validation
            ├── planner.rs       # input range、effective range、lookback planning
            ├── runners/         # kdj/ma/rsi/boll orchestration
            └── tests/           # 或按模块保留 cfg(test)，但禁止再堆回 lib.rs
```

### API 边界

- `furnace-core` 只公开指标输入、参数、输出、状态和计算函数。
- `furnace-io` 只公开 Dagster/CLI 需要调用的 request、summary、executor trait 和 `run_*` facade。
- SQL 构造、RowBinary cursor、staging helper、lookback resolver 默认保持 `pub(crate)`。
- 不保留旧内部 module path；实施完成后统一修正调用方和测试。

## 实施步骤

### 阶段 1：建立质量约束

一次性调整 `engines/Cargo.toml`：

- 增加 workspace Clippy lint：`all = deny`，对 `pedantic` 先设为 `warn` 或按团队可接受程度选择性开启。
- 增加 rustdoc 约束：至少启用 `broken_intra_doc_links = deny`；对 library crate 分阶段引入 `missing_docs`。
- 约定所有 lint 例外使用局部 `#[expect(...)]` 并写明原因。

### 阶段 2：拆 `furnace-io`

按职责移动代码，不改变业务语义：

1. 抽出 `clickhouse/`：`ClickHouseExecutor`、`ClickHouseCliExecutor`、环境变量解析、命令执行。
2. 抽出 `rowbinary/`：string、var_uint、date、nullable f64 的读写；结果行编码测试迁入该模块。
3. 抽出 `sql/`：DDL、staging、partition replace、symbol where、identifier/table validation。
4. 抽出 `request.rs` 和 `summary.rs`：四类指标 request、write mode、summary、performance metrics、validation summary。
5. 抽出 `planner.rs`：resolve symbols、effective output range、lookback input range、append safety、replace cascade 年份规划。
6. 抽出 `runners/{kdj,ma,rsi,boll}.rs`：每个指标只保留 orchestration，调用共享 planner/sql/rowbinary/clickhouse。
7. `lib.rs` 缩减为 facade 和必要 re-export。

阶段完成标准：`furnace-io/src/lib.rs` 低于 150 行；任何新指标不需要修改 `lib.rs` 大段逻辑。

### 阶段 3：拆 CLI

按命令拆出 `furnace/src/commands/`：

1. `main.rs` 只做 `env::args`、调用 CLI、打印 stdout/stderr、返回 exit code。
2. `cli.rs` 定义命令分发和公共解析工具。
3. `commands/kdj.rs`、`commands/ma.rs`、`commands/rsi.rs`、`commands/boll.rs` 各自维护本指标参数和 request 转换。
4. 去掉重复的日期范围、output format、symbols、run_id、insert_batch_size 解析逻辑，收敛到共享 helper。
5. 帮助文本按命令生成或拆分，避免一个长字符串承载所有命令。

阶段完成标准：`furnace/src/main.rs` 低于 80 行；每个命令文件只包含本指标差异。

### 阶段 4：收敛领域类型

引入共享领域概念：

- `IndicatorKind`：`Kdj`、`Ma`、`Rsi`、`Boll`。
- `WriteMode` 或每指标 write mode 的统一 trait/方法：`DryRun`、`AppendLatest`、`ReplaceCascade`。
- `DateRange`：封装 `from/to` 校验，禁止直接比较裸字符串散落在 CLI 和 I/O。
- `TableName`、`ColumnName`：集中校验 ClickHouse 标识符，避免重复 `validate_table_name`/`validate_identifier` 调用。
- `RunPlan`：把 resolved symbols、input_from、effective_to、affected_years、all_symbols_requested 等规划结果显式化。

目标是让“先规划、再读取、再计算、再写入”的管线可读，而不是通过函数调用顺序隐式表达。

### 阶段 5：整理错误模型

- `furnace-core` 保持纯计算 typed error，不暴露 I/O 概念。
- `furnace-io` 使用一个顶层 `FurnaceIoError`，下挂 validation、ClickHouse、RowBinary、indicator calculation、write safety 等分类。
- `furnace` CLI 使用 `CliError` 区分 usage/runtime，并集中完成 exit code 映射。
- 错误消息中保留指标名、表名、日期范围、symbol scope 等定位信息。

### 阶段 6：测试重排

测试随模块迁移，覆盖保持不降：

- `furnace-core`：继续以指标公式、边界输入、previous state 为主。
- `rowbinary`：集中测试编码/解码，不再散在 runner 测试里。
- `sql`：集中测试 DDL、staging、partition、where clause 和 identifier validation。
- `planner`：集中测试 append safety、replace cascade、lookback、effective range。
- `runners`：只测 dry-run/write orchestration 和 summary。
- CLI：只测参数解析、usage error、runtime error 映射和 request 转换。

不追求边移动边新增大量测试；目标是先保持现有覆盖，再补缺失的模块级边界测试。

### 阶段 7：同步文档

- 更新 `engines/README.md`，从“KDJ 第一阶段”改为“多指标 Furnace 引擎”。
- 补充 crate/module 边界图。
- 更新 Dagster 调用说明，明确 CLI facade 不承载指标公式和 ClickHouse 细节。
- 如 public API 收敛导致 rustdoc 页面变化，重新生成并检查 `make rust-doc`。

## 本次实施记录

已完成的结构治理：

- `furnace/src/main.rs` 缩减为进程入口和 exit code 映射。
- `furnace/src/cli.rs` 只保留命令分发和 CLI error，命令参数拆到 `furnace/src/commands/{kdj,ma,rsi,boll}.rs`。
- CLI 单元测试迁移到 `furnace/src/cli_tests.rs`。
- `furnace-io/src/lib.rs` 缩减为 public facade。
- ClickHouse executor、错误类型、schema/DDL、SQL helper、RowBinary、行模型、request、summary、validation 从 `furnace-io/src/lib.rs` 拆出。
- `furnace-io/src/request.rs` 改为 request facade，指标请求模型拆到 `request/{kdj,ma,rsi,boll}.rs`。
- `furnace-io/src/summary.rs` 改为 summary facade，指标 summary 拆到 `summary/{kdj,ma,rsi,boll}.rs`，共享性能和校验摘要拆到 `summary/common.rs`。
- `furnace-io/src/runners/mod.rs` 改为 runner facade，四个指标 orchestration 拆到 `runners/{kdj,ma,rsi,boll}.rs`。
- runner 的 planning、calculation、writing 按指标拆分，shared helper 保留在 `*_shared.rs`。
- runner 单元测试迁移到 `furnace-io/src/runners/tests.rs`。
- runner 二次结构治理已完成：`runners/{kdj,ma,rsi,boll}/` 改为按指标目录组织，每个指标目录包含 `mod.rs`、`planning.rs`、`materialize.rs`、`writing.rs`。
- `runners/shared/` 已承载真正共享的 `planning`、`grouping`、`parallel`、`writing` helper。
- 原 `calculation_*` 命名已改为 `materialize.rs`，明确该层职责是调用 `furnace-core` 后映射 result rows 和统计摘要，不承载指标公式。
- 生产 runner 模块已消除 `use super::*`，每个文件显式导入实际依赖；`runners/mod.rs` 不再作为 import hub。
- runner 测试已拆到 `runners/tests/{fixtures,schema,kdj,ma,rsi,boll}.rs`。
- `engines/Cargo.toml` 增加 workspace rustdoc/clippy lint 约束。
- `engines/README.md` 从 KDJ 第一阶段更新为 KDJ/MA/RSI/BOLL 多指标引擎说明。

当前结构扫描结果：

- `furnace-io/src/lib.rs`：41 行。
- `furnace/src/main.rs`：32 行。
- `furnace-io/src/runners/mod.rs`：13 行。
- 生产路径未发现 `unwrap()`、`expect()`、`panic!()`、`todo!()`、`dbg!()`；剩余 `unwrap/expect` 位于测试或 doctest。
- 最大生产模块集中在 `furnace-core` 指标公式文件，属于单指标领域实现；最大非生产文件是 runner 和 CLI 测试。

最终验收结果：

- `cargo fmt --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace`：通过，93 个单元测试和 4 个 doctest 全部通过。
- `cargo build --release -p furnace`：通过。
- `uv run pytest scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/unit/resources/test_furnace.py`：通过，20 个测试全部通过。

## 统一验收

代码迁移全部完成后，再统一运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release -p furnace
```

涉及 Dagster resource 或 CLI 参数兼容时，再运行：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/unit/resources/test_furnace.py
```

验收标准：

- 所有质量门禁通过。
- `furnace-io/src/lib.rs` 低于 150 行。
- `furnace/src/main.rs` 低于 80 行。
- `furnace-core` 不依赖 ClickHouse、Dagster、dbt、Rayon 或环境变量。
- 新增指标只需新增 core indicator、I/O runner、CLI command，不需要复制整条执行链。
- `engines/README.md` 与实际指标能力一致。

## 风险与处理

- 大规模文件移动会造成 diff 较大：通过“先纯移动、再收敛抽象”的提交顺序降低审查成本，但不保留旧内部路径。
- Public API 收敛可能影响测试：同步修正测试调用点，不为测试暴露内部 helper。
- 统一质量门禁最后执行可能一次暴露多类编译错误：按模块边界修复，不回退到单文件结构。
- 如果后续确实需要引入 CLI/error 辅助依赖，应在实施当时确认最新官方文档和版本，再落 Cargo 变更。
