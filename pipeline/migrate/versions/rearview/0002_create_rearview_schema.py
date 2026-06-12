"""create rearview rule screening schema

Revision ID: 0002_create_rearview_schema
Revises: 0001_jiuyan_industry_images
Create Date: 2026-06-12
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision = "0002_create_rearview_schema"
down_revision = "0001_jiuyan_industry_images"
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "rearview"


def upgrade() -> None:
    if not _is_active_target():
        return

    op.create_table(
        "rule_set",
        sa.Column("rule_set_id", sa.Text(), primary_key=True),
        sa.Column("name", sa.Text(), nullable=False),
        sa.Column("description", sa.Text()),
        sa.Column("owner", sa.Text()),
        sa.Column("status", sa.Text(), nullable=False, server_default="draft"),
        sa.Column(
            "tags", postgresql.JSONB(astext_type=sa.Text()), nullable=False, server_default="[]"
        ),
        sa.Column("current_version_id", sa.Text()),
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
            "status in ('draft', 'active', 'archived')",
            name="ck_rule_set_status",
        ),
    )

    op.create_table(
        "metric_catalog",
        sa.Column("logical_metric", sa.Text(), primary_key=True),
        sa.Column("mart_database", sa.Text(), nullable=False),
        sa.Column("mart_table", sa.Text(), nullable=False),
        sa.Column("column_name", sa.Text(), nullable=False),
        sa.Column("value_kind", sa.Text(), nullable=False),
        sa.Column("allow_filter", sa.Boolean(), nullable=False, server_default=sa.false()),
        sa.Column("allow_scoring", sa.Boolean(), nullable=False, server_default=sa.false()),
        sa.Column("allowed_ops", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("null_policy", sa.Text(), nullable=False),
        sa.Column("default_output", sa.Boolean(), nullable=False, server_default=sa.false()),
        sa.Column("description", sa.Text()),
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
        sa.UniqueConstraint(
            "mart_database",
            "mart_table",
            "column_name",
            name="uq_metric_catalog_source_column",
        ),
        sa.CheckConstraint(
            "value_kind in ('numeric', 'integer', 'boolean', 'string', 'date')",
            name="ck_metric_catalog_value_kind",
        ),
        sa.CheckConstraint(
            "null_policy in ('no_match', 'match', 'error')",
            name="ck_metric_catalog_null_policy",
        ),
    )

    op.create_table(
        "rule_version",
        sa.Column("rule_version_id", sa.Text(), primary_key=True),
        sa.Column("rule_set_id", sa.Text(), nullable=False),
        sa.Column("version_no", sa.Integer(), nullable=False),
        sa.Column("status", sa.Text(), nullable=False, server_default="draft"),
        sa.Column("rule_ast", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("universe_snapshot", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("pool_filters", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("scoring", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("top_n_default", sa.Integer(), nullable=False),
        sa.Column("output_metrics", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column(
            "metric_dependency_snapshot", postgresql.JSONB(astext_type=sa.Text()), nullable=False
        ),
        sa.Column("rule_hash", sa.Text(), nullable=False),
        sa.Column("created_by", sa.Text()),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.ForeignKeyConstraint(
            ["rule_set_id"],
            ["rule_set.rule_set_id"],
            name="fk_rule_version_rule_set",
            ondelete="CASCADE",
        ),
        sa.UniqueConstraint("rule_set_id", "version_no", name="uq_rule_version_version_no"),
        sa.UniqueConstraint("rule_set_id", "rule_hash", name="uq_rule_version_rule_hash"),
        sa.CheckConstraint(
            "status in ('draft', 'active', 'archived')",
            name="ck_rule_version_status",
        ),
        sa.CheckConstraint("top_n_default > 0", name="ck_rule_version_top_n_positive"),
    )

    op.create_foreign_key(
        "fk_rule_set_current_version",
        "rule_set",
        "rule_version",
        ["current_version_id"],
        ["rule_version_id"],
        ondelete="SET NULL",
    )

    op.create_table(
        "run",
        sa.Column("run_id", sa.Text(), primary_key=True),
        sa.Column("rule_version_id", sa.Text(), nullable=False),
        sa.Column("rule_hash", sa.Text(), nullable=False),
        sa.Column("start_date", sa.Date(), nullable=False),
        sa.Column("end_date", sa.Date(), nullable=False),
        sa.Column("top_n", sa.Integer(), nullable=False),
        sa.Column("universe_snapshot", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("resolved_universe_hash", sa.Text()),
        sa.Column("status", sa.Text(), nullable=False, server_default="created"),
        sa.Column("compiled_sql_hash", sa.Text()),
        sa.Column(
            "summary", postgresql.JSONB(astext_type=sa.Text()), nullable=False, server_default="{}"
        ),
        sa.Column("error_type", sa.Text()),
        sa.Column("error_message", sa.Text()),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.Column("started_at", sa.DateTime(timezone=True)),
        sa.Column("completed_at", sa.DateTime(timezone=True)),
        sa.ForeignKeyConstraint(
            ["rule_version_id"],
            ["rule_version.rule_version_id"],
            name="fk_run_rule_version",
            ondelete="RESTRICT",
        ),
        sa.CheckConstraint("start_date <= end_date", name="ck_run_date_range"),
        sa.CheckConstraint("top_n > 0", name="ck_run_top_n_positive"),
        sa.CheckConstraint(
            "status in ("
            "'created', 'validating', 'compiling', 'running_clickhouse', "
            "'writing_pool', 'writing_signals', 'succeeded', "
            "'failed_validation', 'failed_compile', 'failed_clickhouse', "
            "'failed_write', 'cancelled'"
            ")",
            name="ck_run_status",
        ),
    )
    op.create_index("idx_run_rule_version_created", "run", ["rule_version_id", "created_at"])
    op.create_index("idx_run_status_created", "run", ["status", "created_at"])

    op.create_table(
        "run_chunk",
        sa.Column("run_id", sa.Text(), nullable=False),
        sa.Column("chunk_no", sa.Integer(), nullable=False),
        sa.Column("start_date", sa.Date(), nullable=False),
        sa.Column("end_date", sa.Date(), nullable=False),
        sa.Column("status", sa.Text(), nullable=False, server_default="created"),
        sa.Column("clickhouse_query_id", sa.Text()),
        sa.Column("elapsed_ms", sa.BigInteger()),
        sa.Column("error_type", sa.Text()),
        sa.Column("error_message", sa.Text()),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.Column("started_at", sa.DateTime(timezone=True)),
        sa.Column("completed_at", sa.DateTime(timezone=True)),
        sa.PrimaryKeyConstraint("run_id", "chunk_no", name="pk_run_chunk"),
        sa.ForeignKeyConstraint(
            ["run_id"],
            ["run.run_id"],
            name="fk_run_chunk_run",
            ondelete="CASCADE",
        ),
        sa.CheckConstraint("chunk_no >= 0", name="ck_run_chunk_no_non_negative"),
        sa.CheckConstraint("start_date <= end_date", name="ck_run_chunk_date_range"),
        sa.CheckConstraint(
            "status in ('created', 'running', 'succeeded', 'failed', 'cancelled')",
            name="ck_run_chunk_status",
        ),
    )

    op.create_table(
        "run_day",
        sa.Column("run_id", sa.Text(), nullable=False),
        sa.Column("trade_date", sa.Date(), nullable=False),
        sa.Column("chunk_no", sa.Integer(), nullable=False),
        sa.Column("status", sa.Text(), nullable=False, server_default="created"),
        sa.Column("universe_count", sa.Integer()),
        sa.Column("pool_count", sa.Integer()),
        sa.Column("signal_count", sa.Integer()),
        sa.Column("elapsed_ms", sa.BigInteger()),
        sa.Column("error_type", sa.Text()),
        sa.Column("error_message", sa.Text()),
        sa.PrimaryKeyConstraint("run_id", "trade_date", name="pk_run_day"),
        sa.ForeignKeyConstraint(
            ["run_id", "chunk_no"],
            ["run_chunk.run_id", "run_chunk.chunk_no"],
            name="fk_run_day_run_chunk",
            ondelete="CASCADE",
        ),
        sa.CheckConstraint(
            "status in ('created', 'succeeded', 'failed', 'cancelled')",
            name="ck_run_day_status",
        ),
        sa.CheckConstraint(
            "universe_count is null or universe_count >= 0", name="ck_run_day_universe_count"
        ),
        sa.CheckConstraint("pool_count is null or pool_count >= 0", name="ck_run_day_pool_count"),
        sa.CheckConstraint(
            "signal_count is null or signal_count >= 0", name="ck_run_day_signal_count"
        ),
    )

    op.create_table(
        "pool_member",
        sa.Column("run_id", sa.Text(), nullable=False),
        sa.Column("trade_date", sa.Date(), nullable=False),
        sa.Column("security_code", sa.Text(), nullable=False),
        sa.Column("score", sa.Numeric(8, 4)),
        sa.Column("signal_rank", sa.Integer()),
        sa.Column(
            "selected_metrics",
            postgresql.JSONB(astext_type=sa.Text()),
            nullable=False,
            server_default="{}",
        ),
        sa.Column(
            "filter_snapshot",
            postgresql.JSONB(astext_type=sa.Text()),
            nullable=False,
            server_default="{}",
        ),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.PrimaryKeyConstraint("run_id", "trade_date", "security_code", name="pk_pool_member"),
        sa.ForeignKeyConstraint(
            ["run_id", "trade_date"],
            ["run_day.run_id", "run_day.trade_date"],
            name="fk_pool_member_run_day",
            ondelete="CASCADE",
        ),
        sa.CheckConstraint(
            "signal_rank is null or signal_rank > 0", name="ck_pool_member_signal_rank"
        ),
    )
    op.create_index(
        "idx_pool_member_run_date_rank",
        "pool_member",
        ["run_id", "trade_date", "signal_rank"],
    )

    op.create_table(
        "buy_signal",
        sa.Column("run_id", sa.Text(), nullable=False),
        sa.Column("trade_date", sa.Date(), nullable=False),
        sa.Column("security_code", sa.Text(), nullable=False),
        sa.Column("rank", sa.Integer(), nullable=False),
        sa.Column("score", sa.Numeric(8, 4), nullable=False),
        sa.Column("score_breakdown", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column(
            "selected_metrics",
            postgresql.JSONB(astext_type=sa.Text()),
            nullable=False,
            server_default="{}",
        ),
        sa.Column(
            "created_at",
            sa.DateTime(timezone=True),
            nullable=False,
            server_default=sa.text("now()"),
        ),
        sa.PrimaryKeyConstraint("run_id", "trade_date", "security_code", name="pk_buy_signal"),
        sa.ForeignKeyConstraint(
            ["run_id", "trade_date", "security_code"],
            ["pool_member.run_id", "pool_member.trade_date", "pool_member.security_code"],
            name="fk_buy_signal_pool_member",
            ondelete="CASCADE",
        ),
        sa.UniqueConstraint("run_id", "trade_date", "rank", name="uq_buy_signal_day_rank"),
        sa.CheckConstraint("rank > 0", name="ck_buy_signal_rank_positive"),
        sa.CheckConstraint("score >= 0 and score <= 99", name="ck_buy_signal_score_clamp"),
    )
    op.create_index("idx_buy_signal_run_date_rank", "buy_signal", ["run_id", "trade_date", "rank"])


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index("idx_buy_signal_run_date_rank", table_name="buy_signal")
    op.drop_table("buy_signal")
    op.drop_index("idx_pool_member_run_date_rank", table_name="pool_member")
    op.drop_table("pool_member")
    op.drop_table("run_day")
    op.drop_table("run_chunk")
    op.drop_index("idx_run_status_created", table_name="run")
    op.drop_index("idx_run_rule_version_created", table_name="run")
    op.drop_table("run")
    op.drop_constraint("fk_rule_set_current_version", "rule_set", type_="foreignkey")
    op.drop_table("rule_version")
    op.drop_table("metric_catalog")
    op.drop_table("rule_set")
