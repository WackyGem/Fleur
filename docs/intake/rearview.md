# Intake: Rearview

状态：当前需求投递入口（2026-06-13）

当前事实地图：[../systems/rearview.md](../systems/rearview.md)

## 适用需求

- Rearview HTTP API、规则集、规则版本、运行状态和结果查询。
- PostgreSQL `rearview` database schema、migration、状态机和审计字段。
- metric catalog、policy overlay、规则 AST 校验和 ClickHouse 查询规划。
- 股票池、买入信号、score breakdown、chunk/day 运行摘要和错误响应。

## 不适用

- 前端页面和交互：走 [racingline.md](racingline.md)。
- 指标公式和 calculation 写入：走 [furnace.md](furnace.md)。
- mart 模型、字段事实和 raw profile：走 [data-platform.md](data-platform.md) 或 [data-governance.md](data-governance.md)。

## 投递材料

1. 用户目标和 API/工作流入口。
2. 规则 AST、metric catalog、PostgreSQL 表或 ClickHouse mart 影响。
3. 请求/响应样例和错误语义。
4. 运行规模：日期区间、证券 universe、top_n、chunk 策略和性能预期。
5. Racingline 或其他客户端需要的 UI 友好接口。

## 文档落点

| 情况 | 落点 |
|---|---|
| 后端服务能力或规则语言变化 | `docs/RFC/` |
| 已确定的后端实施阶段 | `docs/plans/` |
| 鉴权、数据库边界、规则不可变性等长期约束 | `docs/ADR/` |
| 当前 API、运行命令或质量门禁变化 | [../systems/rearview.md](../systems/rearview.md) |
| smoke run 和代表性规则核验 | `docs/jobs/reports/` |

## 验证要求

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 PostgreSQL schema 时追加 migration 验证；涉及 UI 需求时同步关联 [racingline.md](racingline.md)。
