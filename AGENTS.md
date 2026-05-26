# AGENTS.md instructions for /storage/program/mono-fleur

## Project Layout

- `pipeline/`: Python data workspace managed by `uv`.
- `pipeline/elt/`: dbt project named `elt`.
- `pipeline/scheduler/`: Dagster project named `scheduler`.
- `app/`: reserved application directory.
- `deploy/`: reserved deployment directory.

## Python And Workspace

- Use `uv` for Python dependency and environment management.
- The pipeline workspace is pinned to Python `3.12.13` in `pipeline/.python-version`.
- Run Python, dbt, Dagster, and dg commands from `pipeline/` with `uv run`.
- Sync the full workspace with:

```bash
cd pipeline
uv sync --all-packages --all-groups
```

## dbt

- dbt project path: `pipeline/elt`.
- dbt project name: `elt`.
- Use `uv run dbt ...` from `pipeline/`.
- Prefer targeted commands. Do not run the entire dbt project unless explicitly requested.
- For model execution during development, prefer `dbt build --select ...` over `dbt run`.
- The starter `models/example` content has been removed; preserve the empty scaffold directories.

## Dagster

- Dagster project path: `pipeline/scheduler`.
- Dagster project name: `scheduler`.
- Use `uv run dg ...` and `uv run dagster ...` from `pipeline/`.
- Prefer `dg` for Dagster project inspection and scaffolding when a suitable command exists.
- The current scheduler scaffold has an empty `defs` package and no example assets/jobs.

## Git And Generated Files

- Do not create nested Git repositories inside project subdirectories.
- Keep template-generated `.git`, `.gitignore`, `.dg`, logs, and example files out of scaffold projects unless explicitly needed.
- Root `.gitignore` excludes virtual environments, dbt build artifacts, dbt logs/packages, and Dagster local state.

## MCP Routing

- `context7`: Use for current documentation for libraries, frameworks, SDKs, APIs, CLI tools, and cloud services. Always resolve the library ID first, then query docs.
- `ace_tool.search_context`: Use as the first-choice semantic codebase search when file locations are unknown.
- `clickhouse`: Use only when ClickHouse database inspection or read-only SQL queries are needed.
- Web search: Use only when current external information is required and Context7 is not the correct source.

## Skills Routing

- `dagster-expert`: Use before any Dagster-specific task, including assets, materializations, components, data pipelines, schedules, sensors, jobs, project structure, `dg` CLI usage, or Dagster concept questions.
- `dignified-python`: Use for Python code quality, type hints, modern Python style, pathlib, exception handling, interfaces, CLI patterns, or Python review/refactoring.
- `using-dbt-for-analytics-engineering`: Use for building or modifying dbt models, sources, tests, SQL transformations, dbt project debugging, data exploration, or impact analysis.
- `running-dbt-commands`: Use when formatting or executing dbt CLI commands, choosing the dbt executable, selecting resources, compiling, building, testing, or showing query output.
- `adding-dbt-unit-test`: Use when adding dbt unit tests or practicing TDD for dbt model logic.
- `answering-natural-language-questions-with-dbt`: Use for business/analytics questions answered from warehouse data, metrics, KPIs, Semantic Layer, or ad-hoc SQL. Do not use for dbt model development.
- `fetching-dbt-docs`: Use for dbt documentation lookup about dbt Core, dbt Cloud/platform, or dbt Semantic Layer.
- `configuring-dbt-mcp-server`: Use for setting up, configuring, or troubleshooting dbt MCP servers for AI tools.

