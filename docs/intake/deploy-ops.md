# Intake: Deploy And Ops

状态：当前需求投递入口（2026-06-13）

当前事实地图：[../systems/deploy-ops.md](../systems/deploy-ops.md)

## 适用需求

- Docker Compose、本地服务端口、环境变量和 dev/smoke 环境约定。
- PostgreSQL migration 执行、database target 和 schema readiness。
- 回填、重跑、smoke run、性能基线和生产核验记录。
- Playwright CDP 浏览器环境、VNC 调试环境和运行前提。

## 不适用

- 系统业务逻辑和模型设计：走对应业务领域 intake。
- 数据字段事实：走 [data-governance.md](data-governance.md)。
- 前端页面体验：走 [racingline.md](racingline.md)。

## 投递材料

1. 目标环境：dev、smoke、production-like 或一次性运行。
2. 受影响服务、端口、database、env var 和启动顺序。
3. migration 或 backfill 的命令、范围、输入和预期结果。
4. 失败恢复、重跑策略和观测方式。
5. 是否需要记录到 `docs/jobs/reports/`。

## 文档落点

| 情况 | 落点 |
|---|---|
| 部署/运行架构或环境矩阵变化 | `docs/RFC/` 或 `docs/ADR/` |
| 已确定的运行治理或迁移实施步骤 | `docs/plans/` |
| 实际运行、回填、smoke、性能和核验记录 | `docs/jobs/reports/` |
| 当前 ops 入口、命令或环境变量变化 | [../systems/deploy-ops.md](../systems/deploy-ops.md) |
| 可复用运行手册变化 | `docs/skills/` 或 `docs/jobs/README.md` |

## 验证要求

文档-only 变更至少运行：

```bash
make docs-check
git diff --check
```

涉及具体系统运行时，追加对应系统地图中的质量门禁。
