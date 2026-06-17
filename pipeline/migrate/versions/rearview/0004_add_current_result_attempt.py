"""add current_result_attempt_id to portfolio_run

Revision ID: 0004_current_result_attempt
Revises: 0003_rearview_portfolio
Create Date: 2026-06-17
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op

revision = "0004_current_result_attempt"
down_revision = "0003_rearview_portfolio"
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "rearview"


def upgrade() -> None:
    if not _is_active_target():
        return

    op.add_column(
        "portfolio_run",
        sa.Column("current_result_attempt_id", sa.Text(), nullable=True),
    )
    op.create_index(
        "idx_portfolio_run_current_attempt",
        "portfolio_run",
        ["portfolio_run_id", "current_result_attempt_id"],
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index("idx_portfolio_run_current_attempt", table_name="portfolio_run")
    op.drop_column("portfolio_run", "current_result_attempt_id")
