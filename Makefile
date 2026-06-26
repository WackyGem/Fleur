SHELL := bash

COMPOSE_FILE := deploy/docker-compose.yml
PIPELINE_DIR := pipeline
SCHEDULER_TARGET := scheduler
ELT_TARGET := elt
TRADE_CALENDAR_ASSET := sina__trade_calendar
DAGSTER_WEBUI_HOST ?= 127.0.0.1
DAGSTER_WEBUI_PORT ?= 3001
DBT_DOCS_HOST ?= 127.0.0.1
DBT_DOCS_PORT ?= 8580
RUST_DOC_HOST ?= 127.0.0.1
RUST_DOC_PORT ?= 8000
RUST_DOC_CRATE ?= furnace_core
REARVIEW_HTTP_BIND ?= 127.0.0.1:34057
REARVIEW_DEV_PORT ?= $(lastword $(subst :, ,$(REARVIEW_HTTP_BIND)))
RACINGLINE_DEV_HOST ?= 127.0.0.1
RACINGLINE_DEV_PORT ?= 5173
RACINGLINE_APP_DIR ?= app/racingline

ifneq ("$(wildcard .env)","")
include .env
export
endif

POSTGRES_DB ?= pipeline
POSTGRES_USER ?= mono_fleur
POSTGRES_PASSWORD ?= change-me-postgres-password
POSTGRES_PORT ?= 34054
PIPELINE_DATABASE_URL ?= postgresql://$(POSTGRES_USER):$(POSTGRES_PASSWORD)@127.0.0.1:$(POSTGRES_PORT)/$(POSTGRES_DB)
REARVIEW_DATABASE_URL ?= postgresql://$(POSTGRES_USER):$(POSTGRES_PASSWORD)@127.0.0.1:$(POSTGRES_PORT)/rearview
export PIPELINE_DATABASE_URL REARVIEW_DATABASE_URL

define require-env-file
	@if [ ! -f .env ]; then \
		printf '%s\n' 'Missing .env; this target requires local environment configuration.' >&2; \
		exit 1; \
	fi
endef

define require-dagster-home
	@if [ "$(origin DAGSTER_HOME)" != "file" ]; then \
		printf '%s\n' 'DAGSTER_HOME must be defined in .env for this target.' >&2; \
		exit 1; \
	fi
endef

define stop-listening-port
	@port='$(1)'; label='$(2)'; \
	if [ -z "$$port" ]; then \
		printf 'Cannot stop %s: empty TCP port\n' "$$label" >&2; \
		exit 1; \
	fi; \
	if command -v lsof >/dev/null 2>&1; then \
		pids=$$(lsof -tiTCP:"$$port" -sTCP:LISTEN 2>/dev/null || true); \
	elif command -v fuser >/dev/null 2>&1; then \
		pids=$$(fuser "$$port"/tcp 2>/dev/null || true); \
	else \
		printf 'Cannot check %s on port %s: lsof or fuser is required\n' "$$label" "$$port" >&2; \
		exit 1; \
	fi; \
	if [ -z "$$pids" ]; then \
		printf 'No existing %s process on port %s\n' "$$label" "$$port"; \
		exit 0; \
	fi; \
	printf 'Stopping existing %s process(es) on port %s: %s\n' "$$label" "$$port" "$$pids"; \
	kill $$pids 2>/dev/null || true; \
	for _attempt in $$(seq 1 10); do \
		alive=''; \
		for pid in $$pids; do \
			if kill -0 "$$pid" 2>/dev/null; then \
				alive="$$alive $$pid"; \
			fi; \
		done; \
		if [ -z "$$alive" ]; then \
			break; \
		fi; \
		sleep 0.2; \
	done; \
	alive=''; \
	for pid in $$pids; do \
		if kill -0 "$$pid" 2>/dev/null; then \
			alive="$$alive $$pid"; \
		fi; \
	done; \
	if [ -n "$$alive" ]; then \
		printf 'Force stopping %s process(es):%s\n' "$$label" "$$alive"; \
		kill -9 $$alive 2>/dev/null || true; \
	fi
endef

.PHONY: help dev-up dev-down dev-logs wait-rustfs wait-postgres wait-clickhouse dagster-home docs-check versions-check check-defs materialize-trade-calendar dev-materialize-trade-calendar webui dbt-docs dbt-docs-serve rust-doc rust-doc-open rust-doc-serve rearview-migrate rearview-catalog-sync rearview-prepare rearview-dev racingline-frontend-dev racingline-app-dev racingline-dev racingline-dev-stop

help:
	@printf '%s\n' 'Available targets:'
	@printf '  %-34s %s\n' 'docs-check' 'Validate docs governance rules'
	@printf '  %-34s %s\n' 'versions-check' 'Validate component versions and release manifest consistency'
	@printf '  %-34s %s\n' 'dev-up' 'Start deploy/docker-compose.yml dev services'
	@printf '  %-34s %s\n' 'dev-down' 'Stop dev services'
	@printf '  %-34s %s\n' 'dev-logs' 'Tail dev service logs'
	@printf '  %-34s %s\n' 'wait-postgres' 'Wait for the local PostgreSQL container'
	@printf '  %-34s %s\n' 'wait-clickhouse' 'Wait for the local ClickHouse container'
	@printf '  %-34s %s\n' 'check-defs' 'Validate Dagster definitions'
	@printf '  %-34s %s\n' 'materialize-trade-calendar' 'Materialize sina__trade_calendar'
	@printf '  %-34s %s\n' 'dev-materialize-trade-calendar' 'Start dev services, wait for RustFS, then materialize the calendar'
	@printf '  %-34s %s\n' 'webui' 'Start Dagster Web UI for the local dev instance'
	@printf '  %-34s %s\n' 'dbt-docs' 'Generate dbt docs under pipeline/elt/target'
	@printf '  %-34s %s\n' 'dbt-docs-serve' 'Generate and serve dbt docs over http://127.0.0.1:8580'
	@printf '  %-34s %s\n' 'rust-doc' 'Generate Rust API docs under engines/target/doc'
	@printf '  %-34s %s\n' 'rust-doc-open' 'Generate and open Furnace core Rust API docs'
	@printf '  %-34s %s\n' 'rust-doc-serve' 'Serve Rust API docs over http://127.0.0.1:8000'
	@printf '  %-34s %s\n' 'rearview-migrate' 'Run pipeline + rearview PostgreSQL migrations'
	@printf '  %-34s %s\n' 'rearview-catalog-sync' 'Sync Rearview metric catalog into PostgreSQL'
	@printf '  %-34s %s\n' 'rearview-dev' 'Start Docker dev services + Rearview after clearing its port'
	@printf '  %-34s %s\n' 'racingline-frontend-dev' 'Start Racingline Vite dev server after clearing its port'
	@printf '  %-34s %s\n' 'racingline-app-dev' 'Start Rearview server/worker + Racingline using existing .env infrastructure'
	@printf '  %-34s %s\n' 'racingline-dev' 'Start Docker dev services + Rearview server/worker + Racingline after clearing ports'
	@printf '  %-34s %s\n' 'racingline-dev-stop' 'Stop Rearview/Racingline dev servers by listening port'

dev-up:
	$(require-env-file)
	docker compose --env-file .env -f $(COMPOSE_FILE) up -d

dev-down:
	$(require-env-file)
	docker compose --env-file .env -f $(COMPOSE_FILE) down

dev-logs:
	$(require-env-file)
	docker compose --env-file .env -f $(COMPOSE_FILE) logs -f

wait-rustfs:
	$(require-env-file)
	@printf 'Waiting for RustFS at %s\n' '$(RUSTFS_ENDPOINT)'
	@for attempt in $$(seq 1 60); do \
		if curl -fsS '$(RUSTFS_ENDPOINT)/health' >/dev/null; then \
			printf 'RustFS is healthy\n'; \
			exit 0; \
		fi; \
		sleep 1; \
	done; \
	printf 'Timed out waiting for RustFS at %s\n' '$(RUSTFS_ENDPOINT)' >&2; \
	exit 1

wait-postgres:
	$(require-env-file)
	@printf '%s\n' 'Waiting for PostgreSQL container'
	@for attempt in $$(seq 1 60); do \
		if docker exec mono-fleur-postgres sh -c 'pg_isready -U "$$POSTGRES_USER" -d "$$POSTGRES_DB"' >/dev/null 2>&1; then \
			printf '%s\n' 'PostgreSQL is healthy'; \
			exit 0; \
		fi; \
		sleep 1; \
	done; \
	printf '%s\n' 'Timed out waiting for PostgreSQL container' >&2; \
	exit 1

wait-clickhouse:
	$(require-env-file)
	@printf '%s\n' 'Waiting for ClickHouse container'
	@for attempt in $$(seq 1 60); do \
		if docker exec mono-fleur-clickhouse sh -c 'clickhouse-client --host 127.0.0.1 --user "$$CLICKHOUSE_USER" --password "$$CLICKHOUSE_PASSWORD" --query "SELECT 1"' >/dev/null 2>&1; then \
			printf '%s\n' 'ClickHouse is healthy'; \
			exit 0; \
		fi; \
		sleep 1; \
	done; \
	printf '%s\n' 'Timed out waiting for ClickHouse container' >&2; \
	exit 1

dagster-home:
	$(require-dagster-home)
	@mkdir -p '$(DAGSTER_HOME)'
	@if [ ! -s '$(DAGSTER_HOME)/dagster.yaml' ]; then \
		printf '%s\n' \
			'concurrency:' \
			'  pools:' \
			'    granularity: run' \
			> '$(DAGSTER_HOME)/dagster.yaml'; \
	fi

docs-check:
	python3 scripts/validate_docs_governance.py

versions-check:
	python3 scripts/check_component_versions.py

check-defs:
	cd $(PIPELINE_DIR) && uv run dg check defs --target-path $(SCHEDULER_TARGET)

materialize-trade-calendar: dagster-home
	cd $(PIPELINE_DIR) && uv run dg launch --target-path $(SCHEDULER_TARGET) --assets $(TRADE_CALENDAR_ASSET)

dev-materialize-trade-calendar: dev-up wait-rustfs materialize-trade-calendar

rearview-migrate: wait-postgres
	$(require-env-file)
	cd $(PIPELINE_DIR) && uv run alembic -c migrate/alembic.ini -x target=all upgrade head

rearview-catalog-sync: rearview-migrate
	$(require-env-file)
	cd engines && cargo run -p rearview-server -- catalog sync

rearview-prepare:
	$(require-env-file)
	$(MAKE) --no-print-directory dev-up
	$(MAKE) --no-print-directory rearview-catalog-sync
	$(MAKE) --no-print-directory wait-clickhouse

rearview-dev:
	$(require-env-file)
	$(call stop-listening-port,$(REARVIEW_DEV_PORT),Rearview dev server)
	$(MAKE) --no-print-directory rearview-prepare
	@printf 'Starting Rearview dev server at http://%s\n' '$(REARVIEW_HTTP_BIND)'
	cd engines && cargo run -p rearview-server -- serve

racingline-frontend-dev:
	$(call stop-listening-port,$(RACINGLINE_DEV_PORT),Racingline frontend dev server)
	@printf 'Starting Racingline frontend at http://%s:%s\n' '$(RACINGLINE_DEV_HOST)' '$(RACINGLINE_DEV_PORT)'
	cd $(RACINGLINE_APP_DIR) && npm run dev -- --host $(RACINGLINE_DEV_HOST) --port $(RACINGLINE_DEV_PORT)

racingline-dev-stop:
	$(call stop-listening-port,$(RACINGLINE_DEV_PORT),Racingline frontend dev server)
	$(call stop-listening-port,$(REARVIEW_DEV_PORT),Rearview dev server)
	@pids=$$(pgrep -f '[r]earview-portfolio-worker' 2>/dev/null || true); \
	if [ -z "$$pids" ]; then \
		printf '%s\n' 'No existing Rearview portfolio worker process'; \
		exit 0; \
	fi; \
	printf 'Stopping existing Rearview portfolio worker process(es): %s\n' "$$pids"; \
	kill $$pids 2>/dev/null || true

racingline-app-dev:
	$(require-env-file)
	$(MAKE) --no-print-directory racingline-dev-stop
	@set -euo pipefail; \
		backend_pid=''; \
		worker_pid=''; \
		frontend_pid=''; \
		cleanup() { \
			status=$$?; \
			trap - INT TERM EXIT; \
			if [ -n "$${frontend_pid:-}" ] && kill -0 "$$frontend_pid" 2>/dev/null; then \
				printf '\nStopping Racingline frontend dev server (pid %s)\n' "$$frontend_pid"; \
				kill "$$frontend_pid" 2>/dev/null || true; \
			fi; \
			if [ -n "$${backend_pid:-}" ] && kill -0 "$$backend_pid" 2>/dev/null; then \
				printf 'Stopping Rearview dev server (pid %s)\n' "$$backend_pid"; \
				kill "$$backend_pid" 2>/dev/null || true; \
			fi; \
			if [ -n "$${worker_pid:-}" ] && kill -0 "$$worker_pid" 2>/dev/null; then \
				printf 'Stopping Rearview portfolio worker (pid %s)\n' "$$worker_pid"; \
				kill "$$worker_pid" 2>/dev/null || true; \
			fi; \
			wait "$${frontend_pid:-}" "$${backend_pid:-}" "$${worker_pid:-}" 2>/dev/null || true; \
			exit "$$status"; \
		}; \
		trap cleanup INT TERM EXIT; \
		printf 'Starting Rearview dev server at http://%s\n' '$(REARVIEW_HTTP_BIND)'; \
		(cd engines && cargo run -p rearview-server -- serve) & \
		backend_pid=$$!; \
		sleep 2; \
		if ! kill -0 "$$backend_pid" 2>/dev/null; then \
			printf 'Rearview dev server failed to start\n' >&2; \
			wait "$$backend_pid"; \
			exit 1; \
		fi; \
		printf 'Starting Rearview portfolio worker\n'; \
		(cd engines && cargo run -p rearview-portfolio-worker -- run) & \
		worker_pid=$$!; \
		sleep 2; \
		if ! kill -0 "$$worker_pid" 2>/dev/null; then \
			printf 'Rearview portfolio worker failed to start\n' >&2; \
			wait "$$worker_pid"; \
			exit 1; \
		fi; \
		printf 'Starting Racingline frontend at http://%s:%s\n' '$(RACINGLINE_DEV_HOST)' '$(RACINGLINE_DEV_PORT)'; \
		(cd $(RACINGLINE_APP_DIR) && npm run dev -- --host $(RACINGLINE_DEV_HOST) --port $(RACINGLINE_DEV_PORT)) & \
		frontend_pid=$$!; \
		set +e; \
		wait -n "$$backend_pid" "$$worker_pid" "$$frontend_pid"; \
		status=$$?; \
		set -e; \
		exit "$$status"

racingline-dev:
	$(require-env-file)
	$(MAKE) --no-print-directory rearview-prepare
	$(MAKE) --no-print-directory racingline-app-dev

webui: dagster-home
	@printf 'Starting Dagster Web UI at http://%s:%s\n' '$(DAGSTER_WEBUI_HOST)' '$(DAGSTER_WEBUI_PORT)'
	cd $(PIPELINE_DIR)/$(SCHEDULER_TARGET) && uv run dg dev --host $(DAGSTER_WEBUI_HOST) --port $(DAGSTER_WEBUI_PORT)

dbt-docs:
	$(require-env-file)
	cd $(PIPELINE_DIR) && uv run dbt docs generate --project-dir $(ELT_TARGET) --profiles-dir $(ELT_TARGET) --static
	@printf 'dbt docs generated at %s\n' '$(PIPELINE_DIR)/$(ELT_TARGET)/target/index.html'
	@printf 'Static dbt docs generated at %s\n' '$(PIPELINE_DIR)/$(ELT_TARGET)/target/static_index.html'

dbt-docs-serve: dbt-docs
	@printf 'Serving dbt docs at http://%s:%s\n' '$(DBT_DOCS_HOST)' '$(DBT_DOCS_PORT)'
	cd $(PIPELINE_DIR) && uv run dbt docs serve --project-dir $(ELT_TARGET) --profiles-dir $(ELT_TARGET) --host $(DBT_DOCS_HOST) --port $(DBT_DOCS_PORT) --no-browser

rust-doc:
	cd engines && cargo doc --workspace --no-deps
	@printf 'Rust docs generated at %s\n' 'engines/target/doc/$(RUST_DOC_CRATE)/index.html'

rust-doc-open: rust-doc
	xdg-open engines/target/doc/$(RUST_DOC_CRATE)/index.html

rust-doc-serve: rust-doc
	@printf 'Serving Rust docs at http://%s:%s/%s/\n' '$(RUST_DOC_HOST)' '$(RUST_DOC_PORT)' '$(RUST_DOC_CRATE)'
	python3 -m http.server $(RUST_DOC_PORT) --bind $(RUST_DOC_HOST) --directory engines/target/doc
