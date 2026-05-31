---
name: dg-backfill-runbook
description: mono-fleur 的 Dagster 回填操作手册。用于选择 dg launch 命令、资产选择、partition 或 partition-range 参数，以及各数据源的回填模板。
---

# DG 回填手册

当 `pipeline/scheduler` 里的回填可以用 `dg launch` 表达时，使用这个 skill。

## 规则

- 所有 `dg` / `dagster` 命令必须使用根目录 `.env` 中的 `DAGSTER_HOME` 作为 Dagster home
- 执行前先在仓库根目录加载 `.env`：`set -a; . ./.env; set +a`
- 运行回填前先执行 `make dagster-home`，确保 Dagster home 和 pool 限制已初始化
- 在 `pipeline/` 下执行命令
- 使用 `uv run dg ...`
- 通过 `--target-path scheduler` 指向 scheduler 项目
- 临时回填优先用明确的 asset selection
- 只有当 job 和目标工作负载完全一致时才用 job

## 流程

1. 确定目标 asset 或 job。
2. 判断它是否分区。
3. 从 [references/backfill-matrix.md](references/backfill-matrix.md) 里选命令模板。
4. 需要时先用 `uv run dg list defs --target-path scheduler --json` 验证选择。
5. 先跑一个小切片，再扩展成完整回填。

## 选择规则

- 能精确选 asset 时优先精确选：`key:source/ths__limit_up_pool`
- 需要按数据源放大范围时用 tag：`tag:source=ths`
- 只有想选整个源 bundle 时才用 `group:s3_sources`

## 分区规则

- 日分区资产用包含式范围：`--partition-range "2024-01-01...2024-01-31"`
- 年分区资产每次跑一个年分区：`--partition 2024`
- 年分区资产跨很多年时，按年份循环，不要直接拉长范围
- 遵守每个 asset 自己的回填窗口限制
- Eastmoney 的并行度依赖 `eastmoney_run_pool`，当前上限为 3 个 run

## 常用命令

```bash
cd pipeline

uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition-range "2024-01-01...2024-01-31"
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition 2024
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 2024
```

## 什么时候改用 Python CLI

如果回填需要下面这些能力，就改用 Python 包装器：

- 自动展开分区
- 多 run 重试
- 进度记录
- 可恢复执行
- 多 asset 批量提交
