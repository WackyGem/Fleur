"""create strategy backtest control plane

Revision ID: 0007_strategy_backtest_control_plane
Revises: 0006_portfolio_metric_config
Create Date: 2026-06-23
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision = "0007_strategy_backtest_control_plane"
down_revision = "0006_portfolio_metric_config"
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "rearview"


def _jsonb() -> postgresql.JSONB:
    return postgresql.JSONB(astext_type=sa.Text())


def upgrade() -> None:
    if not _is_active_target():
        return

    op.create_table(
        "strategy_backtest_run",
        sa.Column("strategy_backtest_run_id", sa.Text(), primary_key=True),
        sa.Column("rule_snapshot", _jsonb(), nullable=False),
        sa.Column("rule_hash", sa.Text(), nullable=False),
        sa.Column("execution_config", _jsonb(), nullable=False),
        sa.Column("execution_config_hash", sa.Text(), nullable=False),
        sa.Column("catalog_hash", sa.Text()),
        sa.Column("compiled_sql_hash", sa.Text()),
        sa.Column("required_metrics", _jsonb(), nullable=False, server_default="[]"),
        sa.Column("required_marts", _jsonb(), nullable=False, server_default="[]"),
        sa.Column("data_preflight_snapshot", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("preview_id", sa.Text()),
        sa.Column("preview_range", _jsonb()),
        sa.Column("period_key", sa.Text(), nullable=False),
        sa.Column("range_as_of_date", sa.Date()),
        sa.Column("range_resolved_at", sa.DateTime(timezone=True)),
        sa.Column("range_resolution_snapshot", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("start_date", sa.Date(), nullable=False),
        sa.Column("end_date", sa.Date(), nullable=False),
        sa.Column("benchmark_security_code", sa.Text(), nullable=False),
        sa.Column("price_basis", sa.Text(), nullable=False, server_default="backward_adjusted"),
        sa.Column("ui_display_snapshot", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("client_request_id", sa.Text()),
        sa.Column("request_hash", sa.Text(), nullable=False),
        sa.Column("status", sa.Text(), nullable=False, server_default="created"),
        sa.Column("dispatch_status", sa.Text(), nullable=False, server_default="pending"),
        sa.Column("nats_stream_sequence", sa.BigInteger()),
        sa.Column("worker_attempt_no", sa.Integer(), nullable=False, server_default="0"),
        sa.Column("claimed_at", sa.DateTime(timezone=True)),
        sa.Column("heartbeat_at", sa.DateTime(timezone=True)),
        sa.Column("claim_expires_at", sa.DateTime(timezone=True)),
        sa.Column("progress", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("summary", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("signal_summary", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("data_coverage_summary", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("error_type", sa.Text()),
        sa.Column("error_message", sa.Text()),
        sa.Column("current_result_attempt_id", sa.Text()),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.Column(
            "updated_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.Column("started_at", sa.DateTime(timezone=True)),
        sa.Column("completed_at", sa.DateTime(timezone=True)),
        sa.CheckConstraint(
            "jsonb_typeof(rule_snapshot) = 'object'",
            name="ck_strategy_backtest_rule_snapshot_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(execution_config) = 'object'",
            name="ck_strategy_backtest_execution_config_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(required_metrics) = 'array'",
            name="ck_strategy_backtest_required_metrics_array",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(required_marts) = 'array'",
            name="ck_strategy_backtest_required_marts_array",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(data_preflight_snapshot) = 'object'",
            name="ck_strategy_backtest_preflight_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(range_resolution_snapshot) = 'object'",
            name="ck_strategy_backtest_range_snapshot_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(ui_display_snapshot) = 'object'",
            name="ck_strategy_backtest_ui_snapshot_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(progress) = 'object'",
            name="ck_strategy_backtest_progress_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(summary) = 'object'",
            name="ck_strategy_backtest_summary_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(signal_summary) = 'object'",
            name="ck_strategy_backtest_signal_summary_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(data_coverage_summary) = 'object'",
            name="ck_strategy_backtest_data_coverage_object",
        ),
        sa.CheckConstraint("period_key in ('1y', '2y', '3y')", name="ck_strategy_backtest_period"),
        sa.CheckConstraint("start_date <= end_date", name="ck_strategy_backtest_date_range"),
        sa.CheckConstraint(
            "price_basis = 'backward_adjusted'",
            name="ck_strategy_backtest_price_basis_backward_adjusted",
        ),
        sa.CheckConstraint(
            "btrim(benchmark_security_code) <> ''",
            name="ck_strategy_backtest_benchmark_non_empty",
        ),
        sa.CheckConstraint(
            "status in ("
            "'created', 'queued', 'compiling_signals', 'running_clickhouse', "
            "'loading_market_data', 'calculating_nav', 'computing_performance', "
            "'writing_results', 'succeeded', 'failed_validation', 'failed_compile', "
            "'failed_market_data', 'failed_simulation', 'failed_write', 'cancelled'"
            ")",
            name="ck_strategy_backtest_status",
        ),
        sa.CheckConstraint(
            "dispatch_status in ('pending', 'published', 'publish_failed')",
            name="ck_strategy_backtest_dispatch_status",
        ),
        sa.CheckConstraint(
            "worker_attempt_no >= 0",
            name="ck_strategy_backtest_worker_attempt_non_negative",
        ),
        sa.CheckConstraint(
            "claim_expires_at is null or claimed_at is not null",
            name="ck_strategy_backtest_claim_requires_claimed_at",
        ),
        sa.CheckConstraint(
            "claim_expires_at is null or claim_expires_at > claimed_at",
            name="ck_strategy_backtest_claim_expires_after_claimed",
        ),
    )
    op.create_index(
        "idx_strategy_backtest_status_created",
        "strategy_backtest_run",
        ["status", "created_at"],
    )
    op.create_index(
        "idx_strategy_backtest_dispatch_status_created",
        "strategy_backtest_run",
        ["dispatch_status", "created_at"],
    )
    op.create_index(
        "idx_strategy_backtest_request_hash",
        "strategy_backtest_run",
        ["request_hash"],
    )
    op.create_index(
        "idx_strategy_backtest_client_request",
        "strategy_backtest_run",
        ["client_request_id", "request_hash"],
    )
    op.create_index(
        "idx_strategy_backtest_current_attempt",
        "strategy_backtest_run",
        ["strategy_backtest_run_id", "current_result_attempt_id"],
    )
    op.create_index(
        "idx_strategy_backtest_claim_expires",
        "strategy_backtest_run",
        ["claim_expires_at"],
    )

    op.create_table(
        "strategy_backtest_task_outbox",
        sa.Column("outbox_id", sa.Text(), primary_key=True),
        sa.Column("strategy_backtest_run_id", sa.Text(), nullable=False),
        sa.Column("subject", sa.Text(), nullable=False),
        sa.Column("payload", _jsonb(), nullable=False),
        sa.Column("status", sa.Text(), nullable=False, server_default="pending"),
        sa.Column("attempt_count", sa.Integer(), nullable=False, server_default="0"),
        sa.Column("last_error", sa.Text()),
        sa.Column("nats_stream_sequence", sa.BigInteger()),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.Column("published_at", sa.DateTime(timezone=True)),
        sa.Column(
            "updated_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.ForeignKeyConstraint(
            ["strategy_backtest_run_id"],
            ["strategy_backtest_run.strategy_backtest_run_id"],
            name="fk_strategy_backtest_task_outbox_run",
            ondelete="CASCADE",
        ),
        sa.UniqueConstraint(
            "strategy_backtest_run_id",
            name="uq_strategy_backtest_task_outbox_run",
        ),
        sa.CheckConstraint(
            "status in ('pending', 'published', 'failed')",
            name="ck_strategy_backtest_task_outbox_status",
        ),
        sa.CheckConstraint(
            "attempt_count >= 0",
            name="ck_strategy_backtest_task_attempt_count",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(payload) = 'object'",
            name="ck_strategy_backtest_task_payload_object",
        ),
    )
    op.create_index(
        "idx_strategy_backtest_task_outbox_status_created",
        "strategy_backtest_task_outbox",
        ["status", "created_at"],
    )

    op.create_table(
        "strategy_backtest_metric_config",
        sa.Column("strategy_backtest_run_id", sa.Text(), nullable=False),
        sa.Column("result_attempt_id", sa.Text(), nullable=False),
        sa.Column("security_code", sa.Text(), nullable=False),
        sa.Column("window_key", sa.Text(), nullable=False),
        sa.Column("window_start", sa.Date()),
        sa.Column("window_end", sa.Date()),
        sa.Column("annualization_days", sa.Integer(), nullable=False, server_default="252"),
        sa.Column("min_observations", sa.Integer(), nullable=False, server_default="20"),
        sa.Column(
            "portfolio_return_basis",
            sa.Text(),
            nullable=False,
            server_default="price_return",
        ),
        sa.Column(
            "benchmark_return_basis",
            sa.Text(),
            nullable=False,
            server_default="price_index",
        ),
        sa.Column("risk_free_tenor", sa.Text(), nullable=False, server_default="1y"),
        sa.Column(
            "risk_free_daily_method",
            sa.Text(),
            nullable=False,
            server_default="compound",
        ),
        sa.Column(
            "risk_free_fill_strategy",
            sa.Text(),
            nullable=False,
            server_default="forward_fill",
        ),
        sa.Column(
            "benchmark_fill_strategy",
            sa.Text(),
            nullable=False,
            server_default="skip",
        ),
        sa.Column("mar", sa.Numeric(20, 10), nullable=False, server_default="0"),
        sa.Column("mar_basis", sa.Text(), nullable=False, server_default="fixed"),
        sa.Column(
            "alignment_strategy",
            sa.Text(),
            nullable=False,
            server_default="inner_join_trade_dates",
        ),
        sa.Column(
            "first_day_return_handling",
            sa.Text(),
            nullable=False,
            server_default="exclude",
        ),
        sa.Column("zero_division_policy", sa.Text(), nullable=False, server_default="null"),
        sa.Column("config_version", sa.Integer(), nullable=False, server_default="1"),
        sa.Column("config_hash", sa.Text(), nullable=False),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.PrimaryKeyConstraint(
            "strategy_backtest_run_id",
            "result_attempt_id",
            "security_code",
            "window_key",
            name="pk_strategy_backtest_metric_config",
        ),
        sa.ForeignKeyConstraint(
            ["strategy_backtest_run_id"],
            ["strategy_backtest_run.strategy_backtest_run_id"],
            name="fk_strategy_backtest_metric_config_run",
            ondelete="CASCADE",
        ),
        sa.CheckConstraint(
            "annualization_days > 0",
            name="ck_strategy_metric_cfg_annualization_days",
        ),
        sa.CheckConstraint(
            "min_observations > 1",
            name="ck_strategy_metric_cfg_min_observations",
        ),
        sa.CheckConstraint(
            "portfolio_return_basis in ('price_return')",
            name="ck_strategy_metric_cfg_portfolio_return_basis",
        ),
        sa.CheckConstraint(
            "benchmark_return_basis in ('price_index')",
            name="ck_strategy_metric_cfg_benchmark_return_basis",
        ),
        sa.CheckConstraint("risk_free_tenor in ('1y')", name="ck_strategy_metric_cfg_tenor"),
        sa.CheckConstraint(
            "risk_free_daily_method in ('compound')",
            name="ck_strategy_metric_cfg_rf_daily_method",
        ),
        sa.CheckConstraint(
            "risk_free_fill_strategy in ('forward_fill')",
            name="ck_strategy_metric_cfg_rf_fill_strategy",
        ),
        sa.CheckConstraint(
            "benchmark_fill_strategy in ('skip')",
            name="ck_strategy_metric_cfg_benchmark_fill_strategy",
        ),
        sa.CheckConstraint("mar_basis in ('fixed')", name="ck_strategy_metric_cfg_mar_basis"),
        sa.CheckConstraint(
            "alignment_strategy in ('inner_join_trade_dates')",
            name="ck_strategy_metric_cfg_alignment_strategy",
        ),
        sa.CheckConstraint(
            "first_day_return_handling in ('exclude')",
            name="ck_strategy_metric_cfg_first_day_return_handling",
        ),
        sa.CheckConstraint(
            "zero_division_policy in ('null')",
            name="ck_strategy_metric_cfg_zero_division_policy",
        ),
        sa.CheckConstraint(
            "(window_start is null and window_end is null) "
            "or (window_start is not null and window_end is not null and window_start <= window_end)",
            name="ck_strategy_metric_cfg_window_bounds",
        ),
    )
    op.create_index(
        "idx_strategy_metric_config_run_attempt",
        "strategy_backtest_metric_config",
        ["strategy_backtest_run_id", "result_attempt_id"],
    )
    op.create_index(
        "idx_strategy_metric_config_hash",
        "strategy_backtest_metric_config",
        ["config_hash"],
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index(
        "idx_strategy_metric_config_hash",
        table_name="strategy_backtest_metric_config",
    )
    op.drop_index(
        "idx_strategy_metric_config_run_attempt",
        table_name="strategy_backtest_metric_config",
    )
    op.drop_table("strategy_backtest_metric_config")
    op.drop_index(
        "idx_strategy_backtest_task_outbox_status_created",
        table_name="strategy_backtest_task_outbox",
    )
    op.drop_table("strategy_backtest_task_outbox")
    op.drop_index("idx_strategy_backtest_claim_expires", table_name="strategy_backtest_run")
    op.drop_index("idx_strategy_backtest_current_attempt", table_name="strategy_backtest_run")
    op.drop_index("idx_strategy_backtest_client_request", table_name="strategy_backtest_run")
    op.drop_index("idx_strategy_backtest_request_hash", table_name="strategy_backtest_run")
    op.drop_index(
        "idx_strategy_backtest_dispatch_status_created",
        table_name="strategy_backtest_run",
    )
    op.drop_index("idx_strategy_backtest_status_created", table_name="strategy_backtest_run")
    op.drop_table("strategy_backtest_run")
