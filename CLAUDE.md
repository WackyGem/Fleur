# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Data pipeline monorepo using Dagster for orchestration and dbt for transformations. Managed by `uv` workspace with Python 3.12.

## Essential Commands

**Infrastructure:**
```bash
make dev-up          # Start Docker services (Postgres, ClickHouse, RustFS, NATS)
make dev-down        # Stop services
make dev-logs        # View service logs
```

**Dagster:**
```bash
cd pipeline
uv run dg dev                              # Start Dagster Web UI
uv run dg check defs --target-path scheduler  # Validate definitions
uv run dg launch --target-path scheduler --assets <asset_name>  # Materialize asset
```

**dbt:**
```bash
cd pipeline
uv run dbt build --select <model>    # Build specific models
uv run dbt test                      # Run tests
```

**Dependencies:**
```bash
cd pipeline
uv sync --all-packages --all-groups  # Sync all workspace dependencies
```

**Testing:**
```bash
cd pipeline
uv run pytest scheduler/tests/           # Run all tests
uv run pytest scheduler/tests/test_<name>.py  # Run specific test
```

## Architecture

**Dagster Project** (`pipeline/scheduler/`):
- Definitions load from `src/scheduler/defs/` folder
- Pipeline modules: `baostock/`, `eastmoney/`, `jiuyan_industry_ocr/`
- Uses concurrency pools: `baostock_run_pool` (1), `eastmoney_run_pool` (3)
- Configuration via environment variables in `.env`

**dbt Project** (`pipeline/elt/`):
- Standard dbt project structure
- Models in `models/`, macros in `macros/`
- Target database configured via `PIPELINE_DATABASE_URL`

**Storage Layer:**
- PostgreSQL: metadata and transactional data
- ClickHouse: analytical queries
- RustFS: S3-compatible object storage
- NATS: event messaging

## Key Directories

- `pipeline/scheduler/src/scheduler/defs/` - Dagster assets, schedules, resources
- `pipeline/elt/models/` - dbt transformation models
- `deploy/` - Docker Compose infrastructure
- `docs/` - RFC, plans, ADR documentation

## Environment Configuration

Copy `.env.example` to `.env` and update credentials. Key services:
- RustFS: object storage (S3 API)
- ClickHouse: analytics database
- PostgreSQL: metadata database
- NATS: message broker

## Development Workflow

1. Start services: `make dev-up`
2. Sync dependencies: `cd pipeline && uv sync --all-packages --all-groups`
3. Run Dagster: `cd pipeline && uv run dg dev`
4. Materialize assets via Web UI or CLI
5. Run dbt models: `cd pipeline && uv run dbt build --select <model>`

## Important

- Reasoning effort is set to xhigh. Please think carefully through the task, validate key assumptions, consider plausible alternatives, and prioritize correctness, consistency, and clarity in the final answer

See `AGENTS.md` for detailed tooling and MCP routing instructions.
