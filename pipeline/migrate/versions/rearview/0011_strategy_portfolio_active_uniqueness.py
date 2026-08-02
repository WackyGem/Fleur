"""scope strategy portfolio unique indexes to active rows

Revision ID: 0011_active_unique_idx
Revises: 0010_strategy_portfolio_example
Create Date: 2026-08-02
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op

revision = "0011_active_unique_idx"
down_revision = "0010_strategy_portfolio_example"
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "rearview"


def upgrade() -> None:
    if not _is_active_target():
        return

    # The two partial unique indexes were created without an archived exclusion,
    # so an archived portfolio (same client_request_id, or same example
    # case/version/fixture_hash) blocked re-creating an active replacement via
    # the ensure endpoint. Re-scope both to status = 'active' so archived rows
    # no longer occupy the unique key.
    op.drop_index(
        "uq_strategy_portfolio_client_request_id",
        table_name="strategy_portfolio",
    )
    op.create_index(
        "uq_strategy_portfolio_client_request_id",
        "strategy_portfolio",
        ["client_request_id"],
        unique=True,
        postgresql_where=sa.text(
            "client_request_id is not null and status = 'active'"
        ),
    )

    op.drop_index(
        "uq_strategy_portfolio_example_case",
        table_name="strategy_portfolio",
    )
    op.create_index(
        "uq_strategy_portfolio_example_case",
        "strategy_portfolio",
        ["example_case_id", "example_version", "fixture_hash"],
        unique=True,
        postgresql_where=sa.text("source_kind = 'example' and status = 'active'"),
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index(
        "uq_strategy_portfolio_example_case",
        table_name="strategy_portfolio",
    )
    op.create_index(
        "uq_strategy_portfolio_example_case",
        "strategy_portfolio",
        ["example_case_id", "example_version", "fixture_hash"],
        unique=True,
        postgresql_where=sa.text("source_kind = 'example'"),
    )

    op.drop_index(
        "uq_strategy_portfolio_client_request_id",
        table_name="strategy_portfolio",
    )
    op.create_index(
        "uq_strategy_portfolio_client_request_id",
        "strategy_portfolio",
        ["client_request_id"],
        unique=True,
        postgresql_where=sa.text("client_request_id is not null"),
    )
