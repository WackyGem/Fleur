"""create rearview portfolio schema

Revision ID: 0003_rearview_portfolio
Revises: 0002_create_rearview_schema
Create Date: 2026-06-16
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision = "0003_rearview_portfolio"
down_revision = "0002_create_rearview_schema"
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
        "market_fee_template",
        sa.Column("market_fee_template_id", sa.Text(), primary_key=True),
        sa.Column("market", sa.Text(), nullable=False),
        sa.Column("name", sa.Text(), nullable=False),
        sa.Column("currency", sa.Text(), nullable=False),
        sa.Column("fee_profile", _jsonb(), nullable=False),
        sa.Column("slippage_profile", _jsonb(), nullable=False),
        sa.Column("is_default", sa.Boolean(), nullable=False, server_default=sa.false()),
        sa.Column("status", sa.Text(), nullable=False, server_default="active"),
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
        sa.CheckConstraint(
            "status in ('active', 'archived')",
            name="ck_market_fee_template_status",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(fee_profile) = 'object'",
            name="ck_market_fee_template_fee_profile_object",
        ),
        sa.CheckConstraint(
            "jsonb_typeof(slippage_profile) = 'object'",
            name="ck_market_fee_template_slippage_profile_object",
        ),
    )
    op.create_index(
        "uq_market_fee_template_active_default",
        "market_fee_template",
        ["market"],
        unique=True,
        postgresql_where=sa.text("is_default = true and status = 'active'"),
    )
    op.create_index(
        "idx_market_fee_template_market_status",
        "market_fee_template",
        ["market", "status"],
    )

    op.create_table(
        "virtual_account_template",
        sa.Column("account_template_id", sa.Text(), primary_key=True),
        sa.Column("rule_set_id", sa.Text(), nullable=False),
        sa.Column("market_fee_template_id", sa.Text()),
        sa.Column("name", sa.Text(), nullable=False),
        sa.Column("initial_cash", sa.Numeric(20, 4), nullable=False),
        sa.Column("currency", sa.Text(), nullable=False),
        sa.Column("fee_profile", _jsonb(), nullable=False),
        sa.Column("slippage_profile", _jsonb(), nullable=False),
        sa.Column("rebalance_policy", _jsonb(), nullable=False),
        sa.Column("risk_exit_policy", _jsonb(), nullable=False),
        sa.Column("is_default", sa.Boolean(), nullable=False, server_default=sa.false()),
        sa.Column("status", sa.Text(), nullable=False, server_default="active"),
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
        sa.ForeignKeyConstraint(
            ["rule_set_id"],
            ["rule_set.rule_set_id"],
            name="fk_virtual_account_template_rule_set",
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["market_fee_template_id"],
            ["market_fee_template.market_fee_template_id"],
            name="fk_virtual_account_template_market_fee_template",
            ondelete="SET NULL",
        ),
        sa.CheckConstraint("initial_cash > 0", name="ck_virtual_account_initial_cash_positive"),
        sa.CheckConstraint(
            "status in ('active', 'archived')",
            name="ck_virtual_account_template_status",
        ),
    )
    op.create_index(
        "uq_virtual_account_template_active_default",
        "virtual_account_template",
        ["rule_set_id"],
        unique=True,
        postgresql_where=sa.text("is_default = true and status = 'active'"),
    )
    op.create_index(
        "idx_virtual_account_template_rule_set_status",
        "virtual_account_template",
        ["rule_set_id", "status"],
    )

    op.create_table(
        "portfolio_run",
        sa.Column("portfolio_run_id", sa.Text(), primary_key=True),
        sa.Column("source_run_id", sa.Text(), nullable=False),
        sa.Column("rule_version_id", sa.Text(), nullable=False),
        sa.Column("rule_hash", sa.Text(), nullable=False),
        sa.Column("account_template_id", sa.Text()),
        sa.Column("account_snapshot", _jsonb(), nullable=False),
        sa.Column("execution_snapshot", _jsonb(), nullable=False),
        sa.Column("price_basis", sa.Text(), nullable=False, server_default="backward_adjusted"),
        sa.Column("start_date", sa.Date(), nullable=False),
        sa.Column("end_date", sa.Date(), nullable=False),
        sa.Column("status", sa.Text(), nullable=False, server_default="created"),
        sa.Column("dispatch_status", sa.Text(), nullable=False, server_default="pending"),
        sa.Column("nats_stream_sequence", sa.BigInteger()),
        sa.Column("summary", _jsonb(), nullable=False, server_default="{}"),
        sa.Column("error_type", sa.Text()),
        sa.Column("error_message", sa.Text()),
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
        sa.Column("completed_at", sa.DateTime(timezone=True)),
        sa.ForeignKeyConstraint(
            ["source_run_id"],
            ["run.run_id"],
            name="fk_portfolio_run_source_run",
            ondelete="RESTRICT",
        ),
        sa.ForeignKeyConstraint(
            ["rule_version_id"],
            ["rule_version.rule_version_id"],
            name="fk_portfolio_run_rule_version",
            ondelete="RESTRICT",
        ),
        sa.ForeignKeyConstraint(
            ["account_template_id"],
            ["virtual_account_template.account_template_id"],
            name="fk_portfolio_run_account_template",
            ondelete="SET NULL",
        ),
        sa.CheckConstraint("start_date <= end_date", name="ck_portfolio_run_date_range"),
        sa.CheckConstraint(
            "price_basis = 'backward_adjusted'",
            name="ck_portfolio_run_price_basis_backward_adjusted",
        ),
        sa.CheckConstraint(
            "status in ("
            "'created', 'dispatching', 'queued', 'validating', 'loading_signals', "
            "'building_targets', 'calculating_nav', 'writing_results', 'succeeded', "
            "'failed_validation', 'failed_market_data', 'failed_simulation', "
            "'failed_write', 'cancelled'"
            ")",
            name="ck_portfolio_run_status",
        ),
        sa.CheckConstraint(
            "dispatch_status in ('pending', 'published', 'publish_failed')",
            name="ck_portfolio_run_dispatch_status",
        ),
    )
    op.create_index(
        "idx_portfolio_run_source_run_created", "portfolio_run", ["source_run_id", "created_at"]
    )
    op.create_index("idx_portfolio_run_status_created", "portfolio_run", ["status", "created_at"])
    op.create_index(
        "idx_portfolio_run_dispatch_status_created",
        "portfolio_run",
        ["dispatch_status", "created_at"],
    )

    op.create_table(
        "portfolio_task_outbox",
        sa.Column("outbox_id", sa.Text(), primary_key=True),
        sa.Column("portfolio_run_id", sa.Text(), nullable=False),
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
            ["portfolio_run_id"],
            ["portfolio_run.portfolio_run_id"],
            name="fk_portfolio_task_outbox_portfolio_run",
            ondelete="CASCADE",
        ),
        sa.UniqueConstraint("portfolio_run_id", name="uq_portfolio_task_outbox_run"),
        sa.CheckConstraint(
            "status in ('pending', 'published', 'failed')",
            name="ck_portfolio_task_outbox_status",
        ),
        sa.CheckConstraint("attempt_count >= 0", name="ck_portfolio_task_outbox_attempt_count"),
    )
    op.create_index(
        "idx_portfolio_task_outbox_status_created",
        "portfolio_task_outbox",
        ["status", "created_at"],
    )

    op.create_table(
        "portfolio_target",
        sa.Column("portfolio_run_id", sa.Text(), nullable=False),
        sa.Column("signal_date", sa.Date(), nullable=False),
        sa.Column("execution_date", sa.Date(), nullable=False),
        sa.Column("security_code", sa.Text(), nullable=False),
        sa.Column("source_rank", sa.Integer()),
        sa.Column("source_score", sa.Numeric(12, 6)),
        sa.Column("target_weight", sa.Numeric(20, 10), nullable=False),
        sa.Column("target_amount", sa.Numeric(20, 4), nullable=False),
        sa.Column("target_quantity", sa.Numeric(20, 4)),
        sa.Column("target_reason", sa.Text(), nullable=False),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.PrimaryKeyConstraint(
            "portfolio_run_id", "signal_date", "security_code", name="pk_portfolio_target"
        ),
        sa.ForeignKeyConstraint(
            ["portfolio_run_id"],
            ["portfolio_run.portfolio_run_id"],
            name="fk_portfolio_target_portfolio_run",
            ondelete="CASCADE",
        ),
        sa.CheckConstraint("target_weight >= 0", name="ck_portfolio_target_weight_non_negative"),
        sa.CheckConstraint("target_amount >= 0", name="ck_portfolio_target_amount_non_negative"),
        sa.CheckConstraint(
            "target_reason in ('buy_signal', 'clear_empty_signal', 'rebalance')",
            name="ck_portfolio_target_reason",
        ),
    )
    op.create_index(
        "idx_portfolio_target_run_execution_date",
        "portfolio_target",
        ["portfolio_run_id", "execution_date"],
    )

    op.create_table(
        "portfolio_order",
        sa.Column("portfolio_order_id", sa.Text(), primary_key=True),
        sa.Column("portfolio_run_id", sa.Text(), nullable=False),
        sa.Column("order_seq", sa.Integer(), nullable=False),
        sa.Column("signal_date", sa.Date()),
        sa.Column("execution_date", sa.Date(), nullable=False),
        sa.Column("security_code", sa.Text(), nullable=False),
        sa.Column("side", sa.Text(), nullable=False),
        sa.Column("order_quantity", sa.Numeric(20, 4), nullable=False),
        sa.Column("order_amount", sa.Numeric(20, 4), nullable=False),
        sa.Column("reference_price", sa.Numeric(20, 6)),
        sa.Column("reason", sa.Text(), nullable=False),
        sa.Column("status", sa.Text(), nullable=False),
        sa.Column("event_ref", sa.Text()),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.ForeignKeyConstraint(
            ["portfolio_run_id"],
            ["portfolio_run.portfolio_run_id"],
            name="fk_portfolio_order_portfolio_run",
            ondelete="CASCADE",
        ),
        sa.UniqueConstraint("portfolio_run_id", "order_seq", name="uq_portfolio_order_run_seq"),
        sa.CheckConstraint("order_seq > 0", name="ck_portfolio_order_seq_positive"),
        sa.CheckConstraint("side in ('buy', 'sell')", name="ck_portfolio_order_side"),
        sa.CheckConstraint("order_quantity >= 0", name="ck_portfolio_order_quantity_non_negative"),
        sa.CheckConstraint("order_amount >= 0", name="ck_portfolio_order_amount_non_negative"),
        sa.CheckConstraint(
            "reason in ('rebalance', 'fixed_stop_loss', 'indicator_stop_loss', "
            "'take_profit', 'time_stop_loss')",
            name="ck_portfolio_order_reason",
        ),
        sa.CheckConstraint(
            "status in ('planned', 'filled', 'skipped_price_missing', "
            "'cancelled_cash_scaled', 'skipped_cash_insufficient', "
            "'skipped_below_min_lot')",
            name="ck_portfolio_order_status",
        ),
    )
    op.create_index(
        "idx_portfolio_order_run_execution_date",
        "portfolio_order",
        ["portfolio_run_id", "execution_date"],
    )
    op.create_index(
        "idx_portfolio_order_run_security",
        "portfolio_order",
        ["portfolio_run_id", "security_code"],
    )

    op.create_table(
        "portfolio_trade",
        sa.Column("portfolio_trade_id", sa.Text(), primary_key=True),
        sa.Column("portfolio_run_id", sa.Text(), nullable=False),
        sa.Column("trade_seq", sa.Integer(), nullable=False),
        sa.Column("portfolio_order_id", sa.Text()),
        sa.Column("trade_date", sa.Date(), nullable=False),
        sa.Column("signal_date", sa.Date()),
        sa.Column("security_code", sa.Text(), nullable=False),
        sa.Column("side", sa.Text(), nullable=False),
        sa.Column("quantity", sa.Numeric(20, 4), nullable=False),
        sa.Column("reference_price", sa.Numeric(20, 6), nullable=False),
        sa.Column("execution_price", sa.Numeric(20, 6), nullable=False),
        sa.Column("gross_amount", sa.Numeric(20, 4), nullable=False),
        sa.Column("commission", sa.Numeric(20, 4), nullable=False),
        sa.Column("stamp_duty", sa.Numeric(20, 4), nullable=False),
        sa.Column("transfer_fee", sa.Numeric(20, 4), nullable=False),
        sa.Column("total_fee", sa.Numeric(20, 4), nullable=False),
        sa.Column("slippage_cost", sa.Numeric(20, 4), nullable=False),
        sa.Column("reason", sa.Text(), nullable=False),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.ForeignKeyConstraint(
            ["portfolio_run_id"],
            ["portfolio_run.portfolio_run_id"],
            name="fk_portfolio_trade_portfolio_run",
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["portfolio_order_id"],
            ["portfolio_order.portfolio_order_id"],
            name="fk_portfolio_trade_order",
            ondelete="SET NULL",
        ),
        sa.UniqueConstraint("portfolio_run_id", "trade_seq", name="uq_portfolio_trade_run_seq"),
        sa.CheckConstraint("trade_seq > 0", name="ck_portfolio_trade_seq_positive"),
        sa.CheckConstraint("side in ('buy', 'sell')", name="ck_portfolio_trade_side"),
        sa.CheckConstraint("quantity > 0", name="ck_portfolio_trade_quantity_positive"),
        sa.CheckConstraint("gross_amount >= 0", name="ck_portfolio_trade_gross_non_negative"),
        sa.CheckConstraint("total_fee >= 0", name="ck_portfolio_trade_fee_non_negative"),
        sa.CheckConstraint(
            "reason in ('rebalance', 'fixed_stop_loss', 'indicator_stop_loss', "
            "'take_profit', 'time_stop_loss')",
            name="ck_portfolio_trade_reason",
        ),
    )
    op.create_index(
        "idx_portfolio_trade_run_trade_date",
        "portfolio_trade",
        ["portfolio_run_id", "trade_date"],
    )
    op.create_index(
        "idx_portfolio_trade_run_security",
        "portfolio_trade",
        ["portfolio_run_id", "security_code"],
    )

    op.create_table(
        "portfolio_position_day",
        sa.Column("portfolio_run_id", sa.Text(), nullable=False),
        sa.Column("trade_date", sa.Date(), nullable=False),
        sa.Column("security_code", sa.Text(), nullable=False),
        sa.Column("quantity", sa.Numeric(20, 4), nullable=False),
        sa.Column("cost_basis", sa.Numeric(20, 4), nullable=False),
        sa.Column("average_entry_price", sa.Numeric(20, 6), nullable=False),
        sa.Column("close_price", sa.Numeric(20, 6), nullable=False),
        sa.Column("market_value", sa.Numeric(20, 4), nullable=False),
        sa.Column("unrealized_pnl", sa.Numeric(20, 4), nullable=False),
        sa.Column("unrealized_return", sa.Numeric(20, 10), nullable=False),
        sa.Column("holding_days", sa.Integer(), nullable=False),
        sa.Column("is_stale_price", sa.Boolean(), nullable=False, server_default=sa.false()),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.PrimaryKeyConstraint(
            "portfolio_run_id",
            "trade_date",
            "security_code",
            name="pk_portfolio_position_day",
        ),
        sa.ForeignKeyConstraint(
            ["portfolio_run_id"],
            ["portfolio_run.portfolio_run_id"],
            name="fk_portfolio_position_day_portfolio_run",
            ondelete="CASCADE",
        ),
        sa.CheckConstraint("quantity > 0", name="ck_portfolio_position_day_quantity_positive"),
        sa.CheckConstraint("holding_days >= 0", name="ck_portfolio_position_day_holding_days"),
    )
    op.create_index(
        "idx_portfolio_position_day_run_security",
        "portfolio_position_day",
        ["portfolio_run_id", "security_code"],
    )

    op.create_table(
        "portfolio_nav",
        sa.Column("portfolio_run_id", sa.Text(), nullable=False),
        sa.Column("trade_date", sa.Date(), nullable=False),
        sa.Column("cash_balance", sa.Numeric(20, 4), nullable=False),
        sa.Column("position_market_value", sa.Numeric(20, 4), nullable=False),
        sa.Column("total_equity", sa.Numeric(20, 4), nullable=False),
        sa.Column("nav", sa.Numeric(20, 10), nullable=False),
        sa.Column("daily_return", sa.Numeric(20, 10)),
        sa.Column("drawdown", sa.Numeric(20, 10), nullable=False),
        sa.Column("gross_exposure", sa.Numeric(20, 10), nullable=False),
        sa.Column("position_count", sa.Integer(), nullable=False),
        sa.Column("turnover", sa.Numeric(20, 10), nullable=False),
        sa.Column("fee_amount", sa.Numeric(20, 4), nullable=False),
        sa.Column("warning_count", sa.Integer(), nullable=False, server_default="0"),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.PrimaryKeyConstraint("portfolio_run_id", "trade_date", name="pk_portfolio_nav"),
        sa.ForeignKeyConstraint(
            ["portfolio_run_id"],
            ["portfolio_run.portfolio_run_id"],
            name="fk_portfolio_nav_portfolio_run",
            ondelete="CASCADE",
        ),
        sa.CheckConstraint("total_equity >= 0", name="ck_portfolio_nav_total_equity_non_negative"),
        sa.CheckConstraint("nav >= 0", name="ck_portfolio_nav_non_negative"),
        sa.CheckConstraint("position_count >= 0", name="ck_portfolio_nav_position_count"),
        sa.CheckConstraint("warning_count >= 0", name="ck_portfolio_nav_warning_count"),
    )

    op.create_table(
        "portfolio_event",
        sa.Column("portfolio_event_id", sa.Text(), primary_key=True),
        sa.Column("portfolio_run_id", sa.Text(), nullable=False),
        sa.Column("event_seq", sa.Integer(), nullable=False),
        sa.Column("trade_date", sa.Date()),
        sa.Column("security_code", sa.Text()),
        sa.Column("event_type", sa.Text(), nullable=False),
        sa.Column("severity", sa.Text(), nullable=False, server_default="warning"),
        sa.Column("message", sa.Text(), nullable=False),
        sa.Column("payload", _jsonb(), nullable=False, server_default="{}"),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.ForeignKeyConstraint(
            ["portfolio_run_id"],
            ["portfolio_run.portfolio_run_id"],
            name="fk_portfolio_event_portfolio_run",
            ondelete="CASCADE",
        ),
        sa.UniqueConstraint("portfolio_run_id", "event_seq", name="uq_portfolio_event_run_seq"),
        sa.CheckConstraint("event_seq > 0", name="ck_portfolio_event_seq_positive"),
        sa.CheckConstraint(
            "severity in ('info', 'warning', 'error')",
            name="ck_portfolio_event_severity",
        ),
    )
    op.create_index(
        "idx_portfolio_event_run_trade_date",
        "portfolio_event",
        ["portfolio_run_id", "trade_date"],
    )
    op.create_index(
        "idx_portfolio_event_run_type",
        "portfolio_event",
        ["portfolio_run_id", "event_type"],
    )

    op.execute(
        sa.text(
            """
            insert into market_fee_template (
                market_fee_template_id,
                market,
                name,
                currency,
                fee_profile,
                slippage_profile,
                is_default,
                status
            )
            values (
                'cn-a-share-default',
                'CN_A_SHARE',
                'A-share default',
                'CNY',
                '{
                    "commission_rate": 0.0001,
                    "commission_rate_max": 0.003,
                    "min_commission": 5,
                    "stamp_duty_rate_sell": 0.0005,
                    "transfer_fee_rate": 0.00001
                }'::jsonb,
                '{
                    "mode": "bps",
                    "buy_bps": 10,
                    "sell_bps": 10
                }'::jsonb,
                true,
                'active'
            )
            on conflict (market_fee_template_id) do update
            set
                market = excluded.market,
                name = excluded.name,
                currency = excluded.currency,
                fee_profile = excluded.fee_profile,
                slippage_profile = excluded.slippage_profile,
                is_default = excluded.is_default,
                status = excluded.status,
                updated_at = now()
            """
        )
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index("idx_portfolio_event_run_type", table_name="portfolio_event")
    op.drop_index("idx_portfolio_event_run_trade_date", table_name="portfolio_event")
    op.drop_table("portfolio_event")
    op.drop_table("portfolio_nav")
    op.drop_index("idx_portfolio_position_day_run_security", table_name="portfolio_position_day")
    op.drop_table("portfolio_position_day")
    op.drop_index("idx_portfolio_trade_run_security", table_name="portfolio_trade")
    op.drop_index("idx_portfolio_trade_run_trade_date", table_name="portfolio_trade")
    op.drop_table("portfolio_trade")
    op.drop_index("idx_portfolio_order_run_security", table_name="portfolio_order")
    op.drop_index("idx_portfolio_order_run_execution_date", table_name="portfolio_order")
    op.drop_table("portfolio_order")
    op.drop_index("idx_portfolio_target_run_execution_date", table_name="portfolio_target")
    op.drop_table("portfolio_target")
    op.drop_index("idx_portfolio_task_outbox_status_created", table_name="portfolio_task_outbox")
    op.drop_table("portfolio_task_outbox")
    op.drop_index("idx_portfolio_run_dispatch_status_created", table_name="portfolio_run")
    op.drop_index("idx_portfolio_run_status_created", table_name="portfolio_run")
    op.drop_index("idx_portfolio_run_source_run_created", table_name="portfolio_run")
    op.drop_table("portfolio_run")
    op.drop_index(
        "idx_virtual_account_template_rule_set_status",
        table_name="virtual_account_template",
    )
    op.drop_index(
        "uq_virtual_account_template_active_default",
        table_name="virtual_account_template",
    )
    op.drop_table("virtual_account_template")
    op.drop_index("idx_market_fee_template_market_status", table_name="market_fee_template")
    op.drop_index("uq_market_fee_template_active_default", table_name="market_fee_template")
    op.drop_table("market_fee_template")
