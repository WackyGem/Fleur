"""add strategy portfolio example source metadata

Revision ID: 0010_strategy_portfolio_example
Revises: 0009_strategy_portfolio_ctx
Create Date: 2026-07-02
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op

revision = "0010_strategy_portfolio_example"
down_revision = "0009_strategy_portfolio_ctx"
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "rearview"


def upgrade() -> None:
    if not _is_active_target():
        return

    op.drop_constraint(
        "fk_strategy_portfolio_source_backtest",
        "strategy_portfolio",
        type_="foreignkey",
    )
    op.add_column(
        "strategy_portfolio",
        sa.Column("source_kind", sa.Text(), nullable=False, server_default="backtest_publish"),
    )
    op.add_column("strategy_portfolio", sa.Column("example_case_id", sa.Text()))
    op.add_column("strategy_portfolio", sa.Column("example_version", sa.Text()))
    op.add_column("strategy_portfolio", sa.Column("fixture_hash", sa.Text()))
    op.create_check_constraint(
        "ck_strategy_portfolio_source_kind",
        "strategy_portfolio",
        "source_kind in ('backtest_publish', 'example')",
    )
    op.create_check_constraint(
        "ck_strategy_portfolio_example_metadata",
        "strategy_portfolio",
        "("
        "source_kind = 'example' "
        "and btrim(coalesce(example_case_id, '')) <> '' "
        "and btrim(coalesce(example_version, '')) <> '' "
        "and btrim(coalesce(fixture_hash, '')) <> ''"
        ") or ("
        "source_kind = 'backtest_publish' "
        "and example_case_id is null "
        "and example_version is null "
        "and fixture_hash is null"
        ")",
    )
    op.create_index(
        "uq_strategy_portfolio_example_case",
        "strategy_portfolio",
        ["example_case_id", "example_version", "fixture_hash"],
        unique=True,
        postgresql_where=sa.text("source_kind = 'example'"),
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index("uq_strategy_portfolio_example_case", table_name="strategy_portfolio")
    op.drop_constraint(
        "ck_strategy_portfolio_example_metadata",
        "strategy_portfolio",
        type_="check",
    )
    op.drop_constraint(
        "ck_strategy_portfolio_source_kind",
        "strategy_portfolio",
        type_="check",
    )
    op.drop_column("strategy_portfolio", "fixture_hash")
    op.drop_column("strategy_portfolio", "example_version")
    op.drop_column("strategy_portfolio", "example_case_id")
    op.drop_column("strategy_portfolio", "source_kind")
    op.create_foreign_key(
        "fk_strategy_portfolio_source_backtest",
        "strategy_portfolio",
        "strategy_backtest_run",
        ["source_strategy_backtest_run_id"],
        ["strategy_backtest_run_id"],
        ondelete="RESTRICT",
    )
