# mono-fleur-2026.06.1

日期：2026-06-26

状态：Verified

## 集成快照

| 项 | 值 |
|---|---|
| 集成 release | `mono-fleur-2026.06.1` |
| Git commit | `3c20eb538e8aabc1622bbcaada450868b1f6a61c` |
| Manifest | [deploy/release-manifest.yml](../../deploy/release-manifest.yml) |

## 组件版本

| 组件 | 版本 |
|---|---|
| scheduler | `0.1.0` |
| contract-tools | `0.1.0` |
| elt | `1.0.0` |
| furnace | `0.1.0` |
| rearview-server | `0.1.0` |
| rearview-portfolio-worker | `0.1.0` |
| racingline | `0.0.1` |

`pipeline/pyproject.toml` 的 root version 是 workspace meta package 版本，不作为业务组件版本写入 manifest。

## 数据库与 Contracts

| Target | Alembic head | Target schema head |
|---|---|---|
| pipeline | `0008_strategy_portfolio_cp` | `0001_jiuyan_industry_images` |
| rearview | `0008_strategy_portfolio_cp` | `0008_strategy_portfolio_cp` |

当前 Alembic revision graph 是一条全局链，`alembic heads` 对 `pipeline` 和 `rearview` target 都返回 `0008_strategy_portfolio_cp`。各 migration 通过 `context.config.attributes["target"]` 决定是否执行 DDL，因此 release note 同时记录 target schema head。

Contract registry commit：`3c20eb538e8aabc1622bbcaada450868b1f6a61c`

Changed datasets：无。

## Runtime Version Exposure

| 组件 | 入口 |
|---|---|
| furnace | `furnace --version` |
| rearview-server | `rearview-server --version`、`GET /rearview/version`、`GET /healthz` |
| rearview-portfolio-worker | `rearview-portfolio-worker --version`、启动日志 |
| racingline | `window.__RACINGLINE_BUILD_METADATA__` |
| scheduler | Dagster asset materialization metadata `scheduler_version` |
| contract-tools | `fleur-contracts --version` |

## 验证结果

| 命令 | 结果 |
|---|---|
| `make versions-check` | Passed |
| `make docs-check` | Passed |
| `git diff --check` | Passed |
| `cd engines && cargo metadata --format-version 1 --no-deps` | Passed |
| `cd engines && cargo fmt --check` | Passed |
| `cd engines && cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| `cd engines && cargo test --workspace` | Passed |
| `cd engines && cargo run -q -p furnace -- --version` | Passed: `furnace 0.1.0` |
| `cd engines && cargo run -q -p rearview-server -- --version` | Passed: `rearview-server 0.1.0` |
| `cd engines && cargo run -q -p rearview-portfolio-worker -- --version` | Passed: `rearview-portfolio-worker 0.1.0` |
| `cd pipeline && uv run ruff format scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate` | Passed: no files changed |
| `cd pipeline && uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate` | Passed |
| `cd pipeline && uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests` | Passed |
| `cd pipeline && uv run pytest scheduler/tests contract_tools/tests` | Passed: 424 tests |
| `cd pipeline && uv run fleur-contracts --version` | Passed: `fleur-contracts 0.1.0` |
| `cd pipeline && uv run dbt parse --project-dir elt --profiles-dir elt` | Passed |
| `cd pipeline && uv run fleur-contracts validate` | Passed: 21 dataset contracts |
| `cd pipeline && uv run fleur-contracts generate --check` | Passed |
| `cd pipeline/scheduler && uv run dg check defs` | Passed |
| `cd app/racingline && npm run lint` | Passed |
| `cd app/racingline && npm run typecheck` | Passed |
| `cd app/racingline && npm test` | Passed: 7 files, 51 tests |
| `cd app/racingline && npm run build` | Passed with Vite chunk size warning |

## Version impact

- Components:
  - Rust crates: no version bump; `0.1.0` moved from workspace inheritance to explicit crate versions.
  - scheduler: no version bump; runtime metadata now exposes `0.1.0`.
  - contract-tools: no version bump; CLI now exposes `0.1.0`.
  - racingline: no version bump; build metadata now exposes `0.0.1`.
- Dataset contracts: none.
- Alembic heads: unchanged; target schema heads recorded separately from global revision head.
- Release manifest: created.
- Runtime version exposure: created for executable/deployable components.
- Tags: none.
