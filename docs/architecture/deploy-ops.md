# Architecture: Deploy And Ops

状态：当前事实入口（2026-07-03）

## 代码根

| 路径 | 角色 |
|---|---|
| [deploy/](../../deploy/) | Docker Compose、本地基础设施和服务配置 |
| [deploy/docker-compose.dev.yaml](../../deploy/docker-compose.dev.yaml) | dev 基础设施入口，使用 `34xxx` 宿主机端口 |
| [deploy/docker-compose.yml](../../deploy/docker-compose.yml) | production-like 入口，使用 `35xxx` 宿主机端口并通过 nginx 统一暴露 Racingline、Rearview 和 Dagster |
| [deploy/docker/](../../deploy/docker/) | production-like 应用镜像 Dockerfile |
| [deploy/nginx/default.conf](../../deploy/nginx/default.conf) | production-like nginx gateway 路由配置 |
| [pipeline/migrate/](../../pipeline/migrate/) | Alembic migration 项目 |
| [docs/jobs/](../jobs/) | runbook、snapshot 和运行报告入口 |

## 职责

1. 维护 dev 和 production-like 两套 Compose 入口。
2. 管理 PostgreSQL migration 执行入口，包括 `pipeline` 和 `rearview` database target。
3. 管理 production-like 启动初始化链路，包括 Alembic migration 和 Rearview catalog sync。
4. 记录回填、重跑、性能基线和 smoke run 的实际命令、范围与结果。
5. 为数据平台、Furnace、Rearview、Dagster 和 Racingline 提供可复现的运行前提。

## 非职责

1. 不定义业务模型、指标公式、规则 AST 或前端交互。
2. 不把 `.env` 中的敏感信息提交到版本控制。
3. 不替代系统各自的质量门禁；ops 文档只串联运行前提和执行记录。

## 常用入口

启动 dev 依赖：

```bash
make infra-up
```

启动 production-like 栈：

```bash
make prod-up
```

production-like 栈默认通过 nginx 暴露：

```text
http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/
http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/rearview/health
http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/dagster/
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
| [../RFC/0047-production-compose-nginx-entrypoint.md](../RFC/0047-production-compose-nginx-entrypoint.md) | production-like Compose 与 nginx 统一入口设计 |
| [../plans/archive/0076-production-compose-nginx-implementation-plan.md](../plans/archive/0076-production-compose-nginx-implementation-plan.md) | RFC 0047 实施计划、阶段验收和验证命令 |
| [../jobs/reports/2026-07-03-production-compose-nginx-smoke.md](../jobs/reports/2026-07-03-production-compose-nginx-smoke.md) | production-like Compose 端到端 smoke、浏览器验证和修复记录 |
| [../jobs/README.md](../jobs/README.md) | jobs runbook 和 reports 入口 |
| [../releases/README.md](../releases/README.md) | 集成 release note、版本 manifest schema 和 tag 前检查 |
| [../../deploy/release-manifest.yml](../../deploy/release-manifest.yml) | 当前集成发布快照的组件版本、migration head 和 contract 变更 |
| [../jobs/dagster-backfill-2026.md](../jobs/dagster-backfill-2026.md) | Dagster 回填入口 |
| [../issues/archive/optimize/docs-governance-inventory-2026-06-10.md](../issues/archive/optimize/docs-governance-inventory-2026-06-10.md) | docs governance 盘点和治理记录 |
| [../RFC/archive/0018-rust-stock-screening-service.md](../RFC/archive/0018-rust-stock-screening-service.md) | Rearview database target 和部署顺序背景 |
| [rearview.md](rearview.md) | Rearview 架构事实文档 |

## 待决问题

1. 是否需要把 production-like compose smoke 固化为单独 runbook 或 CI job。
2. production hardening 阶段是否收敛 PostgreSQL、ClickHouse、NATS 和 RustFS console 的宿主机端口暴露。
