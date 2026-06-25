# Plan 0008 Code Framework Review

日期：2026-05-29

关联计划：

- `docs/plans/0008-pipeline-rfc0006-quality-reusability-implementation.md`
- `docs/RFC/archive/0006-pipeline-code-quality-and-reusability.md`

## Review 结论

当前代码框架已经完成结构迁移和第二轮框架调整，允许进入并继续推进测试框架重写。

本轮 review 结论基于当前工作树和门禁输出：

- `defs/util.py`、`defs/config.py`、`defs/pipeline_defs.py`、旧 `defs/http_resources/`、`io_managers/postgres.py`、`io_managers/image_object_store.py` 已从生产代码中移除。
- 新结构已落地到 `automation/`、`common/`、`config/`、`storage/`、`market/`、`http/`、`sources/`、`repositories/`、`defs/definitions.py`。
- RFC 0006 点名的重复 helper 已收敛到单一 canonical module。
- `ObjectStore` 已从图片业务中提取，`ImageObjectStore` 只保留图片/OCR 业务映射。
- `materialize_partition_range()` 已泛化为任意 partition key 的分区物化框架，`materialize_trade_date_range()` 是其交易日包装。
- HTTP 业务 fetch 边界已通过 protocol 与 fake 测试客户端对齐。
- 测试已迁移到 `tests/fakes/`、`tests/helpers/`、`tests/unit/`、`tests/integration/` 分层目录；数据库、HTTP、Dagster、storage、BaoStock fake 已集中在共享测试工具中。
- `scheduler/definitions.py` 已收敛为直接 re-export `scheduler.defs.definitions.defs` 的顶层入口。
- 生产代码中被测试直接覆盖的请求构造、分区解释、S3 validation、fetch helper 已提升为公共命名边界；测试不再直接调用生产 private helper。
- BaoStock 测试 fake 已改为 `send_once` protocol 注入，不再通过继承真实 client 并 override 私有方法来绕过边界。

## 已验证证据

完整代码检查：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
```

结果：通过。

完整类型检查：

```bash
cd pipeline
uv run pyright scheduler/src/scheduler scheduler/tests
```

结果：`0 errors, 0 warnings, 0 informations`。

Dagster definitions 检查：

```bash
cd pipeline/scheduler
uv run dg check defs
```

结果：

```text
All component YAML validated successfully.
All definitions loaded successfully.
```

测试与覆盖率：

```bash
cd pipeline
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
```

结果：`141 passed`，总覆盖率 `71.82%`，达到 `fail_under = 70`。

旧入口和重复 helper 搜索：

```bash
rg "scheduler\.defs\.http_resources|scheduler\.defs\.util\b|pipeline_defs|io_managers\.postgres|io_managers\.image_object_store" pipeline/pyproject.toml pipeline/scheduler/src/scheduler pipeline/scheduler/tests -n
rg "def _?elapsed_seconds|def _?required_string|def _?positive_int_or_default|def _?row_fingerprint|def _?string_or_null|def _?stringify_value|parse_required_date|parse_optional_date" pipeline/scheduler/src/scheduler pipeline/scheduler/tests -n
```

结果：生产代码、测试代码和 pyright 配置中无旧 import path；重复 helper 只剩 `common/` 中的 canonical 定义。

## 旧模块到新模块映射

| 旧模块 | 新模块 |
| --- | --- |
| `defs/config.py` | `defs/config/env.py`, `defs/config/models.py` |
| `defs/util.py` retry | `defs/common/retry.py` |
| `defs/util.py` elapsed helper | `defs/common/clock.py` |
| `defs/util.py` string/date/number/fingerprint helpers | `defs/common/strings.py`, `defs/common/dates.py`, `defs/common/numbers.py`, `defs/common/fingerprint.py` |
| `defs/util.py` asset keys | `defs/market/asset_keys.py` |
| `defs/util.py` BaoStock security filtering | `defs/market/securities.py` |
| `defs/util.py` trade calendar reader facade | `defs/market/trade_calendar.py` |
| `defs/util.py` S3 filesystem/object key | `defs/storage/s3.py` |
| `defs/util.py` parquet write/read helpers | `defs/storage/parquet.py`, `defs/storage/parquet_readers.py` |
| `defs/http_resources/client.py` | `defs/http/client.py` |
| `defs/http_resources/partitioned.py` | `defs/http/partitioning.py` |
| `defs/http_resources/schedules.py` 通用 Dagster job/schedule 工厂 | `defs/automation/schedules.py` |
| `defs/http_resources/schedules.py` 交易日调度工厂 | `defs/market/schedules.py` |
| `defs/http_resources/schedules.py` HTTP 数据源 job/schedule 实例 | `defs/http/schedules.py` |
| `defs/http_resources/schemas.py` | `defs/http/schemas.py` |
| `defs/http_resources/flatten.py` | `defs/http/flatten.py` |
| `defs/http_resources/sina__trade_calendar.py` | `defs/sources/sina/trade_calendar.py` |
| `defs/http_resources/jiuyan__action_field.py` | `defs/sources/jiuyan/action_field.py` |
| `defs/http_resources/jiuyan__industry_list.py` | `defs/sources/jiuyan/industry_list.py` |
| `defs/http_resources/jiuyan__industry_ocr.py` | `defs/sources/jiuyan/industry_ocr.py` |
| `defs/http_resources/jiuyan_image_urls.py` | `defs/sources/jiuyan/image_urls.py` |
| `defs/http_resources/jiuyan_ocr_client.py` | `defs/sources/jiuyan/ocr_client.py` |
| `defs/http_resources/jiuyan_ocr_schema.py` | `defs/sources/jiuyan/ocr_schema.py` |
| `defs/http_resources/jiuyan_ocr_services.py` | `defs/sources/jiuyan/ocr_services.py` |
| `defs/http_resources/ths__limit_up_pool.py` | `defs/sources/ths/limit_up_pool.py` |
| `defs/http_resources/eastmoney.py` | `defs/sources/eastmoney/assets.py` |
| `defs/http_resources/eastmoney_client.py` | `defs/sources/eastmoney/client.py` |
| `defs/http_resources/eastmoney_schema.py` | `defs/sources/eastmoney/schema.py` |
| `defs/http_resources/eastmoney_fields.py` | `defs/sources/eastmoney/fields.py` |
| `defs/io_managers/postgres.py` | `defs/repositories/industry_images.py` |
| `defs/io_managers/image_object_store.py` | `defs/storage/object_store.py` |
| `defs/pipeline_defs.py` | `defs/definitions.py` |
| `scheduler/definitions.py` defs-folder 扫描入口 | 直接 re-export `defs/definitions.py` |

## RFC 0006 原问题处理状态

| 问题 | 当前状态 |
| --- | --- |
| `_elapsed_seconds()` 分散定义 | 已收敛到 `common/clock.py` |
| `_required_string()` 分散定义 | 已收敛到 `common/strings.py` |
| `_positive_int_or_default()` 分散定义 | 已收敛到 `common/numbers.py` |
| `_row_fingerprint()` 分散定义 | 已收敛到 `common/fingerprint.py`，分页使用 `http/pagination.py` |
| `defs/util.py` 万能模块 | 已删除，能力拆入 `common/`, `storage/`, `market/` |
| `config.py` 混合 EnvVar 和数据类 | 已拆分为 `config/env.py` 和 `config/models.py` |
| `io_managers/postgres.py` 9 个 wrapper | 已删除模块级 wrapper，仅保留 repository 类 API |
| HTTP schema helper 分散 | 已收敛到 `http/schemas.py`；EastMoney 保留 `eastmoney_string_or_null()` 以保持布尔值语义 |
| 分页重复检测 | 已引入 `http/pagination.py` 并接入 EastMoney、THS |
| metadata builder | 已提供 `AssetMetadataBuilder`、HTTP stats 和 storage metadata helper；部分资产仍直接构造 metadata，作为可接受的后续收敛项 |
| schedule/job 工厂 | 已引入 `automation.schedules` 中的 `AssetJobSpec`, `ScheduleSpec`, `build_asset_job`, `build_schedule`, `build_year_refresh_schedule`；交易日调度 `build_trade_date_schedule` 已放入 `market.schedules`，HTTP 与 BaoStock 共同复用且 BaoStock 不再依赖 `http` 包 |
| 测试 fake/helper 分散 | 已迁移到 `tests/fakes/` 和 `tests/helpers/`，测试文件已按 `unit/` / `integration/` 分层；数据库 fake 重复实现已删除 |

## 剩余风险

- `common/metadata.py` 还没有覆盖所有 asset metadata 构造。现阶段门禁和 contract 测试已证明输出字段稳定，进一步统一属于增量质量改进。
- 顶层 `scheduler/src/scheduler/definitions.py` 只负责 re-export Dagster Definitions，integration contract test 已断言它与 `defs.definitions` 指向同一入口。

## 进入测试重写结论

结论：**允许进入并继续推进测试框架重写**。

当前代码框架已经满足阶段 6 的核心要求：旧结构清理完成、目标抽象可复用、Dagster definitions 可加载、类型检查通过、测试与覆盖率门禁通过。剩余事项不阻塞计划继续推进，但应作为后续 cleanup 跟踪。
