ENV_FILE ?= env
COMPOSE_FILE := deploy/docker-compose.yml
PIPELINE_DIR := pipeline
SCHEDULER_TARGET := scheduler
TRADE_CALENDAR_ASSET := sina__trade_calendar
DAGSTER_HOME ?= $(CURDIR)/.dagster
DAGSTER_WEBUI_HOST ?= 127.0.0.1
DAGSTER_WEBUI_PORT ?= 3000

ifneq ("$(wildcard $(ENV_FILE))","")
include $(ENV_FILE)
export
endif
export DAGSTER_HOME

.PHONY: help dev-up dev-down dev-logs wait-rustfs dagster-home check-defs materialize-trade-calendar dev-materialize-trade-calendar webui

help:
	@printf '%s\n' 'Available targets:'
	@printf '  %-34s %s\n' 'dev-up' 'Start deploy/docker-compose.yml dev services'
	@printf '  %-34s %s\n' 'dev-down' 'Stop dev services'
	@printf '  %-34s %s\n' 'dev-logs' 'Tail dev service logs'
	@printf '  %-34s %s\n' 'check-defs' 'Validate Dagster definitions'
	@printf '  %-34s %s\n' 'materialize-trade-calendar' 'Materialize sina__trade_calendar'
	@printf '  %-34s %s\n' 'dev-materialize-trade-calendar' 'Start dev services, wait for RustFS, then materialize the calendar'
	@printf '  %-34s %s\n' 'webui' 'Start Dagster Web UI for the local dev instance'

dev-up:
	docker compose --env-file $(ENV_FILE) -f $(COMPOSE_FILE) up -d

dev-down:
	docker compose --env-file $(ENV_FILE) -f $(COMPOSE_FILE) down

dev-logs:
	docker compose --env-file $(ENV_FILE) -f $(COMPOSE_FILE) logs -f

wait-rustfs:
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
	@mkdir -p '$(DAGSTER_HOME)'
	@touch '$(DAGSTER_HOME)/dagster.yaml'

check-defs:
	cd $(PIPELINE_DIR) && uv run dg check defs --target-path $(SCHEDULER_TARGET)

materialize-trade-calendar: dagster-home
	cd $(PIPELINE_DIR) && uv run dg launch --target-path $(SCHEDULER_TARGET) --assets $(TRADE_CALENDAR_ASSET)

dev-materialize-trade-calendar: dev-up wait-rustfs materialize-trade-calendar

webui: dagster-home
	@printf 'Starting Dagster Web UI at http://%s:%s\n' '$(DAGSTER_WEBUI_HOST)' '$(DAGSTER_WEBUI_PORT)'
	cd $(PIPELINE_DIR)/$(SCHEDULER_TARGET) && uv run dg dev --host $(DAGSTER_WEBUI_HOST) --port $(DAGSTER_WEBUI_PORT)
