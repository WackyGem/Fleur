"""create portfolio metric config

Revision ID: 0006_portfolio_metric_config
Revises: 0005_drop_portfolio_pg_facts
Create Date: 2026-06-17
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op

revision = "0006_portfolio_metric_config"
down_revision = "0005_drop_portfolio_pg_facts"
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "rearview"


def upgrade() -> None:
    if not _is_active_target():
        return

    op.create_table(
        "portfolio_metric_config",
        sa.Column("portfolio_run_id", sa.Text(), nullable=False),
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
            "portfolio_run_id",
            "result_attempt_id",
            "security_code",
            "window_key",
            name="pk_portfolio_metric_config",
        ),
        sa.ForeignKeyConstraint(
            ["portfolio_run_id"],
            ["portfolio_run.portfolio_run_id"],
            name="fk_portfolio_metric_config_portfolio_run",
            ondelete="CASCADE",
        ),
        sa.CheckConstraint("annualization_days > 0", name="ck_metric_cfg_annualization_days"),
        sa.CheckConstraint("min_observations > 1", name="ck_metric_cfg_min_observations"),
        sa.CheckConstraint(
            "portfolio_return_basis in ('price_return')",
            name="ck_metric_cfg_portfolio_return_basis",
        ),
        sa.CheckConstraint(
            "benchmark_return_basis in ('price_index')",
            name="ck_metric_cfg_benchmark_return_basis",
        ),
        sa.CheckConstraint("risk_free_tenor in ('1y')", name="ck_metric_cfg_risk_free_tenor"),
        sa.CheckConstraint(
            "risk_free_daily_method in ('compound')",
            name="ck_metric_cfg_risk_free_daily_method",
        ),
        sa.CheckConstraint(
            "risk_free_fill_strategy in ('forward_fill')",
            name="ck_metric_cfg_risk_free_fill_strategy",
        ),
        sa.CheckConstraint(
            "benchmark_fill_strategy in ('skip')",
            name="ck_metric_cfg_benchmark_fill_strategy",
        ),
        sa.CheckConstraint("mar_basis in ('fixed')", name="ck_metric_cfg_mar_basis"),
        sa.CheckConstraint(
            "alignment_strategy in ('inner_join_trade_dates')",
            name="ck_metric_cfg_alignment_strategy",
        ),
        sa.CheckConstraint(
            "first_day_return_handling in ('exclude')",
            name="ck_metric_cfg_first_day_return_handling",
        ),
        sa.CheckConstraint(
            "zero_division_policy in ('null')",
            name="ck_metric_cfg_zero_division_policy",
        ),
        sa.CheckConstraint(
            "(window_start is null and window_end is null) "
            "or (window_start is not null and window_end is not null and window_start <= window_end)",
            name="ck_metric_cfg_window_bounds",
        ),
    )
    op.create_index(
        "idx_portfolio_metric_config_run_attempt",
        "portfolio_metric_config",
        ["portfolio_run_id", "result_attempt_id"],
    )
    op.create_index(
        "idx_portfolio_metric_config_hash",
        "portfolio_metric_config",
        ["config_hash"],
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index("idx_portfolio_metric_config_hash", table_name="portfolio_metric_config")
    op.drop_index("idx_portfolio_metric_config_run_attempt", table_name="portfolio_metric_config")
    op.drop_table("portfolio_metric_config")
