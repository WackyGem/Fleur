# Architecture: Deploy And Ops

状态：当前事实入口（2026-06-13）

## 代码根

| 路径 | 角色 |
|---|---|
| [deploy/](../../deploy/) | Docker Compose、本地基础设施和服务配置 |
| [deploy/docker-compose.yml](../../deploy/docker-compose.yml) | PostgreSQL、ClickHouse、S3-compatible storage 等本地服务入口 |
| [pipeline/migrate/](../../pipeline/migrate/) | Alembic migration 项目 |
| [docs/jobs/](../jobs/) | runbook、snapshot 和运行报告入口 |

## 职责

1. 维护本地开发和 smoke run 所需的基础设施配置。
2. 管理 PostgreSQL migration 执行入口，包括 `pipeline` 和 `rearview` database target。
3. 记录回填、重跑、性能基线和 smoke run 的实际命令、范围与结果。
4. 为数据平台、Furnace、Rearview 和后续 Racingline 提供可复现的运行前提。

## 非职责

1. 不定义业务模型、指标公式、规则 AST 或前端交互。
2. 不把 `.env` 中的敏感信息提交到版本控制。
3. 不替代系统各自的质量门禁；ops 文档只串联运行前提和执行记录。

## 常用入口

启动本地依赖：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d postgres clickhouse
```

执行 migration：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=pipeline upgrade head
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
```

检查 Playwright CDP 浏览器端点：

```bash
node scripts/check_playwright_cdp.mjs
```

文档-only 校验：

```bash
make docs-check
git diff --check
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [../jobs/README.md](../jobs/README.md) | jobs runbook 和 reports 入口 |
| [../releases/README.md](../releases/README.md) | 集成 release note、版本 manifest schema 和 tag 前检查 |
| [../../deploy/release-manifest.yml](../../deploy/release-manifest.yml) | 当前集成发布快照的组件版本、migration head 和 contract 变更 |
| [../jobs/dagster-backfill-2026.md](../jobs/dagster-backfill-2026.md) | Dagster 回填入口 |
| [../issues/archive/optimize/docs-governance-inventory-2026-06-10.md](../issues/archive/optimize/docs-governance-inventory-2026-06-10.md) | docs governance 盘点和治理记录 |
| [../RFC/archive/0018-rust-stock-screening-service.md](../RFC/archive/0018-rust-stock-screening-service.md) | Rearview database target 和部署顺序背景 |
| [rearview.md](rearview.md) | Rearview 架构事实文档 |

## 待决问题

1. 是否需要单独的环境矩阵文档，覆盖 dev、smoke、production-like 的服务端口、database target 和密钥注入方式。
2. 是否需要把 migration readiness、service health check 和 smoke run 串成统一 runbook。
