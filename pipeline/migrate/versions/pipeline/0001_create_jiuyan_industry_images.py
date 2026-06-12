"""create jiuyan industry image state table

Revision ID: 0001_jiuyan_industry_images
Revises:
Create Date: 2026-05-28
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import context, op

revision = "0001_jiuyan_industry_images"
down_revision = None
branch_labels = None
depends_on = None


def _is_active_target() -> bool:
    return context.config.attributes.get("target", "pipeline") == "pipeline"


def upgrade() -> None:
    if not _is_active_target():
        return

    op.create_table(
        "jiuyan_industry_images",
        sa.Column("image_filename", sa.Text(), primary_key=True),
        sa.Column("image_url", sa.Text(), nullable=False),
        sa.Column("image_s3_key", sa.Text(), nullable=False),
        sa.Column("industry_id", sa.Text(), nullable=False),
        sa.Column("image_index", sa.Integer(), nullable=False),
        sa.Column("download_status", sa.Text(), nullable=False, server_default="pending"),
        sa.Column("download_error_type", sa.Text()),
        sa.Column("download_error_message", sa.Text()),
        sa.Column("download_sha256", sa.Text()),
        sa.Column("download_bytes", sa.BigInteger()),
        sa.Column("downloaded_at", sa.DateTime(timezone=True)),
        sa.Column("ocr_status", sa.Text(), nullable=False, server_default="pending"),
        sa.Column("ocr_error_type", sa.Text()),
        sa.Column("ocr_error_message", sa.Text()),
        sa.Column("ocr_result_s3_key", sa.Text()),
        sa.Column("ocr_result_row_count", sa.Integer()),
        sa.Column("ocr_model", sa.Text()),
        sa.Column("ocr_started_at", sa.DateTime(timezone=True)),
        sa.Column("ocr_completed_at", sa.DateTime(timezone=True)),
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
            "download_status in ('pending', 'success', 'failed')",
            name="ck_jiuyan_industry_images_download_status",
        ),
        sa.CheckConstraint(
            "ocr_status in ('pending', 'running', 'success', 'failed')",
            name="ck_jiuyan_industry_images_ocr_status",
        ),
    )
    op.create_index(
        "idx_jiuyan_industry_images_download_status",
        "jiuyan_industry_images",
        ["download_status"],
    )
    op.create_index(
        "idx_jiuyan_industry_images_ocr_claim",
        "jiuyan_industry_images",
        ["ocr_status", "ocr_started_at", "image_filename"],
        postgresql_where=sa.text("download_status = 'success'"),
    )


def downgrade() -> None:
    if not _is_active_target():
        return

    op.drop_index(
        "idx_jiuyan_industry_images_ocr_claim",
        table_name="jiuyan_industry_images",
    )
    op.drop_index(
        "idx_jiuyan_industry_images_download_status",
        table_name="jiuyan_industry_images",
    )
    op.drop_table("jiuyan_industry_images")
