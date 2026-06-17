"""drop migrated PostgreSQL portfolio result facts

Revision ID: 0005_drop_portfolio_pg_facts
Revises: 0004_current_result_attempt
Create Date: 2026-06-17
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision = "0005_drop_portfolio_pg_facts"
down_revision = "0004_current_result_attempt"
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "rearview"


def _jsonb() -> postgresql.JSONB:
    return postgresql.JSONB(astext_type=sa.Text())


def upgrade() -> None:
    if not _is_active_target():
        return

    for table_name in (
        "portfolio_event",
        "portfolio_nav",
        "portfolio_position_day",
        "portfolio_trade",
        "portfolio_order",
        "portfolio_target",
    ):
        op.execute(sa.text(f"drop table if exists {table_name}"))


def downgrade() -> None:
    if not _is_active_target():
        return

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
