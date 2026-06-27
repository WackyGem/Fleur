"""add strategy portfolio publish context fields

Revision ID: 0009_strategy_portfolio_ctx
Revises: 0008_strategy_portfolio_cp
Create Date: 2026-06-27
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision = "0009_strategy_portfolio_ctx"
down_revision = "0008_strategy_portfolio_cp"
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "rearview"


def _jsonb() -> postgresql.JSONB:
    return postgresql.JSONB(astext_type=sa.Text())


def upgrade() -> None:
    if not _is_active_target():
        return

    op.add_column(
        "strategy_portfolio",
        sa.Column("initial_signal_date", sa.Date()),
    )
    op.add_column(
        "strategy_portfolio",
        sa.Column(
            "pending_buy_signal_snapshot",
            _jsonb(),
            nullable=False,
            server_default="[]",
        ),
    )
    op.add_column(
        "strategy_portfolio",
        sa.Column("current_live_result_attempt_id", sa.Text()),
    )

    op.execute(
        "update strategy_portfolio set initial_signal_date = source_end_date "
        "where initial_signal_date is null"
    )
    op.alter_column("strategy_portfolio", "initial_signal_date", nullable=False)

    op.create_check_constraint(
        "ck_strategy_portfolio_pending_signal_snapshot_array",
        "strategy_portfolio",
        "jsonb_typeof(pending_buy_signal_snapshot) = 'array'",
    )
    op.create_index(
        "idx_strategy_portfolio_current_live_attempt",
        "strategy_portfolio",
        ["strategy_portfolio_id", "current_live_result_attempt_id"],
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index(
        "idx_strategy_portfolio_current_live_attempt",
        table_name="strategy_portfolio",
    )
    op.drop_constraint(
        "ck_strategy_portfolio_pending_signal_snapshot_array",
        "strategy_portfolio",
        type_="check",
    )
    op.drop_column("strategy_portfolio", "current_live_result_attempt_id")
    op.drop_column("strategy_portfolio", "pending_buy_signal_snapshot")
    op.drop_column("strategy_portfolio", "initial_signal_date")
