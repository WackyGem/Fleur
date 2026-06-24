"""create strategy portfolio control plane

Revision ID: 0008_strategy_portfolio_cp
Revises: 0007_strategy_backtest_cp
Create Date: 2026-06-24
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision = "0008_strategy_portfolio_cp"
down_revision = "0007_strategy_backtest_cp"
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
        "strategy_portfolio",
        sa.Column("strategy_portfolio_id", sa.Text(), primary_key=True),
        sa.Column("portfolio_code", sa.Text(), nullable=False),
        sa.Column("name", sa.Text(), nullable=False),
        sa.Column("status", sa.Text(), nullable=False, server_default="active"),
        sa.Column("rule_snapshot", _jsonb(), nullable=False),
        sa.Column("rule_hash", sa.Text(), nullable=False),
        sa.Column("execution_config", _jsonb(), nullable=False),
        sa.Column("execution_config_hash", sa.Text(), nullable=False),
        sa.Column("benchmark_security_code", sa.Text(), nullable=False),
        sa.Column("price_basis", sa.Text(), nullable=False, server_default="backward_adjusted"),
        sa.Column("catalog_hash", sa.Text()),
        sa.Column("required_metrics", _jsonb(), nullable=False, server_default="[]"),
        sa.Column("required_marts", _jsonb(), nullable=False, server_default="[]"),
        sa.Column("source_strategy_backtest_run_id", sa.Text(), nullable=False),
        sa.Column("source_result_attempt_id", sa.Text(), nullable=False),
        sa.Column("source_period_key", sa.Text(), nullable=False),
        sa.Column("source_start_date", sa.Date(), nullable=False),
        sa.Column("source_end_date", sa.Date(), nullable=False),
        sa.Column("live_start_date", sa.Date(), nullable=False),
        sa.Column("latest_daily_run_id", sa.Text()),
        sa.Column("current_result_attempt_id", sa.Text()),
        sa.Column("ui_display_snapshot", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("client_request_id", sa.Text()),
        sa.Column("request_hash", sa.Text(), nullable=False),
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
        sa.Column("archived_at", sa.DateTime(timezone=True)),
        sa.ForeignKeyConstraint(
            ["source_strategy_backtest_run_id"],
            ["strategy_backtest_run.strategy_backtest_run_id"],
            name="fk_strategy_portfolio_source_backtest",
            ondelete="RESTRICT",
        ),
        sa.CheckConstraint(
            "btrim(portfolio_code) <> ''", name="ck_strategy_portfolio_code_non_empty"
        ),
        sa.CheckConstraint("btrim(name) <> ''", name="ck_strategy_portfolio_name_non_empty"),
        sa.CheckConstraint(
            "status in ('active', 'archived')",
            name="ck_strategy_portfolio_status",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(rule_snapshot) = 'object'",
            name="ck_strategy_portfolio_rule_snapshot_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(execution_config) = 'object'",
            name="ck_strategy_portfolio_execution_config_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(required_metrics) = 'array'",
            name="ck_strategy_portfolio_required_metrics_array",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(required_marts) = 'array'",
            name="ck_strategy_portfolio_required_marts_array",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(ui_display_snapshot) = 'object'",
            name="ck_strategy_portfolio_ui_snapshot_object",
        ),
        sa.CheckConstraint(
            "price_basis = 'backward_adjusted'",
            name="ck_strategy_portfolio_price_basis_backward_adjusted",
        ),
        sa.CheckConstraint(
            "source_start_date <= source_end_date",
            name="ck_strategy_portfolio_source_date_range",
        ),
        sa.CheckConstraint(
            "live_start_date >= source_end_date",
            name="ck_strategy_portfolio_live_start_after_source",
        ),
        sa.CheckConstraint(
            "archived_at is null or status = 'archived'",
            name="ck_strategy_portfolio_archived_status",
        ),
    )
    op.create_index(
        "uq_strategy_portfolio_code",
        "strategy_portfolio",
        ["portfolio_code"],
        unique=True,
    )
    op.create_index(
        "idx_strategy_portfolio_status_created",
        "strategy_portfolio",
        ["status", "created_at"],
    )
    op.create_index(
        "idx_strategy_portfolio_source_backtest",
        "strategy_portfolio",
        ["source_strategy_backtest_run_id"],
    )
    op.create_index(
        "idx_strategy_portfolio_client_request",
        "strategy_portfolio",
        ["client_request_id", "request_hash"],
    )
    op.create_index(
        "uq_strategy_portfolio_client_request_id",
        "strategy_portfolio",
        ["client_request_id"],
        unique=True,
        postgresql_where=sa.text("client_request_id is not null"),
    )
    op.create_index(
        "idx_strategy_portfolio_current_attempt",
        "strategy_portfolio",
        ["strategy_portfolio_id", "current_result_attempt_id"],
    )

    op.create_table(
        "strategy_portfolio_daily_run",
        sa.Column("strategy_portfolio_daily_run_id", sa.Text(), primary_key=True),
        sa.Column("strategy_portfolio_id", sa.Text(), nullable=False),
        sa.Column("run_start_date", sa.Date(), nullable=False),
        sa.Column("trade_date", sa.Date(), nullable=False),
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
        sa.ForeignKeyConstraint(
            ["strategy_portfolio_id"],
            ["strategy_portfolio.strategy_portfolio_id"],
            name="fk_strategy_portfolio_daily_run_portfolio",
            ondelete="CASCADE",
        ),
        sa.UniqueConstraint(
            "strategy_portfolio_id",
            "trade_date",
            name="uq_strategy_portfolio_daily_run_portfolio_date",
        ),
        sa.CheckConstraint(
            "status in ("
            "'created', 'queued', 'compiling_signals', 'running_clickhouse', "
            "'loading_market_data', 'calculating_nav', 'computing_performance', "
            "'writing_results', 'succeeded', 'failed_validation', 'failed_compile', "
            "'failed_market_data', 'failed_simulation', 'failed_write', 'cancelled'"
            ")",
            name="ck_strategy_portfolio_daily_run_status",
        ),
        sa.CheckConstraint(
            "dispatch_status in ('pending', 'published', 'publish_failed')",
            name="ck_strategy_portfolio_daily_run_dispatch_status",
        ),
        sa.CheckConstraint(
            "run_start_date <= trade_date",
            name="ck_strategy_portfolio_daily_run_date_range",
        ),
        sa.CheckConstraint(
            "worker_attempt_no >= 0",
            name="ck_strategy_portfolio_daily_run_worker_attempt_non_negative",
        ),
        sa.CheckConstraint(
            "claim_expires_at is null or claimed_at is not null",
            name="ck_strategy_portfolio_daily_run_claim_requires_claimed_at",
        ),
        sa.CheckConstraint(
            "claim_expires_at is null or claim_expires_at > claimed_at",
            name="ck_strategy_portfolio_daily_run_claim_expires_after_claimed",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(progress) = 'object'",
            name="ck_strategy_portfolio_daily_run_progress_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(summary) = 'object'",
            name="ck_strategy_portfolio_daily_run_summary_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(signal_summary) = 'object'",
            name="ck_strategy_portfolio_daily_run_signal_summary_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(data_coverage_summary) = 'object'",
            name="ck_strategy_portfolio_daily_run_data_coverage_object",
        ),
    )
    op.create_index(
        "idx_strategy_portfolio_daily_status_created",
        "strategy_portfolio_daily_run",
        ["status", "created_at"],
    )
    op.create_index(
        "idx_strategy_portfolio_daily_trade_date",
        "strategy_portfolio_daily_run",
        ["trade_date"],
    )
    op.create_index(
        "idx_strategy_portfolio_daily_claim_expires",
        "strategy_portfolio_daily_run",
        ["claim_expires_at"],
    )
    op.create_index(
        "idx_strategy_portfolio_daily_current_attempt",
        "strategy_portfolio_daily_run",
        ["strategy_portfolio_daily_run_id", "current_result_attempt_id"],
    )

    op.create_table(
        "strategy_portfolio_daily_task_outbox",
        sa.Column("outbox_id", sa.Text(), primary_key=True),
        sa.Column("strategy_portfolio_daily_run_id", sa.Text(), nullable=False),
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
            ["strategy_portfolio_daily_run_id"],
            ["strategy_portfolio_daily_run.strategy_portfolio_daily_run_id"],
            name="fk_strategy_portfolio_daily_task_outbox_run",
            ondelete="CASCADE",
        ),
        sa.UniqueConstraint(
            "strategy_portfolio_daily_run_id",
            name="uq_strategy_portfolio_daily_task_outbox_run",
        ),
        sa.CheckConstraint(
            "status in ('pending', 'published', 'failed')",
            name="ck_strategy_portfolio_daily_task_outbox_status",
        ),
        sa.CheckConstraint(
            "attempt_count >= 0",
            name="ck_strategy_portfolio_daily_task_attempt_count",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(payload) = 'object'",
            name="ck_strategy_portfolio_daily_task_payload_object",
        ),
    )
    op.create_index(
        "idx_strategy_portfolio_daily_outbox_status_created",
        "strategy_portfolio_daily_task_outbox",
        ["status", "created_at"],
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index(
        "idx_strategy_portfolio_daily_outbox_status_created",
        table_name="strategy_portfolio_daily_task_outbox",
    )
    op.drop_table("strategy_portfolio_daily_task_outbox")
    op.drop_index(
        "idx_strategy_portfolio_daily_current_attempt",
        table_name="strategy_portfolio_daily_run",
    )
    op.drop_index(
        "idx_strategy_portfolio_daily_claim_expires",
        table_name="strategy_portfolio_daily_run",
    )
    op.drop_index(
        "idx_strategy_portfolio_daily_trade_date",
        table_name="strategy_portfolio_daily_run",
    )
    op.drop_index(
        "idx_strategy_portfolio_daily_status_created",
        table_name="strategy_portfolio_daily_run",
    )
    op.drop_table("strategy_portfolio_daily_run")
    op.drop_index("idx_strategy_portfolio_current_attempt", table_name="strategy_portfolio")
    op.drop_index("uq_strategy_portfolio_client_request_id", table_name="strategy_portfolio")
    op.drop_index("idx_strategy_portfolio_client_request", table_name="strategy_portfolio")
    op.drop_index("idx_strategy_portfolio_source_backtest", table_name="strategy_portfolio")
    op.drop_index("idx_strategy_portfolio_status_created", table_name="strategy_portfolio")
    op.drop_index("uq_strategy_portfolio_code", table_name="strategy_portfolio")
    op.drop_table("strategy_portfolio")
