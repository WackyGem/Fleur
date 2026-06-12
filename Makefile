COMPOSE_FILE := deploy/docker-compose.yml
PIPELINE_DIR := pipeline
SCHEDULER_TARGET := scheduler
ELT_TARGET := elt
TRADE_CALENDAR_ASSET := sina__trade_calendar
DAGSTER_WEBUI_HOST ?= 127.0.0.1
DAGSTER_WEBUI_PORT ?= 3000
DBT_DOCS_HOST ?= 127.0.0.1
DBT_DOCS_PORT ?= 8580
RUST_DOC_HOST ?= 127.0.0.1
RUST_DOC_PORT ?= 8000
RUST_DOC_CRATE ?= furnace_core

ifneq ("$(wildcard .env)","")
include .env
export
endif

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

.PHONY: help dev-up dev-down dev-logs wait-rustfs dagster-home docs-check check-defs materialize-trade-calendar dev-materialize-trade-calendar webui dbt-docs dbt-docs-serve rust-doc rust-doc-open rust-doc-serve

help:
	@printf '%s\n' 'Available targets:'
	@printf '  %-34s %s\n' 'docs-check' 'Validate docs governance rules'
	@printf '  %-34s %s\n' 'dev-up' 'Start deploy/docker-compose.yml dev services'
	@printf '  %-34s %s\n' 'dev-down' 'Stop dev services'
	@printf '  %-34s %s\n' 'dev-logs' 'Tail dev service logs'
	@printf '  %-34s %s\n' 'check-defs' 'Validate Dagster definitions'
	@printf '  %-34s %s\n' 'materialize-trade-calendar' 'Materialize sina__trade_calendar'
	@printf '  %-34s %s\n' 'dev-materialize-trade-calendar' 'Start dev services, wait for RustFS, then materialize the calendar'
	@printf '  %-34s %s\n' 'webui' 'Start Dagster Web UI for the local dev instance'
	@printf '  %-34s %s\n' 'dbt-docs' 'Generate dbt docs under pipeline/elt/target'
	@printf '  %-34s %s\n' 'dbt-docs-serve' 'Generate and serve dbt docs over http://127.0.0.1:8580'
	@printf '  %-34s %s\n' 'rust-doc' 'Generate Rust API docs under engines/target/doc'
	@printf '  %-34s %s\n' 'rust-doc-open' 'Generate and open Furnace core Rust API docs'
	@printf '  %-34s %s\n' 'rust-doc-serve' 'Serve Rust API docs over http://127.0.0.1:8000'

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

check-defs:
	cd $(PIPELINE_DIR) && uv run dg check defs --target-path $(SCHEDULER_TARGET)

materialize-trade-calendar: dagster-home
	cd $(PIPELINE_DIR) && uv run dg launch --target-path $(SCHEDULER_TARGET) --assets $(TRADE_CALENDAR_ASSET)

dev-materialize-trade-calendar: dev-up wait-rustfs materialize-trade-calendar

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
