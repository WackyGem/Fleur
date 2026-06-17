"""Tests for the jiuyan_industry_images database migration.

Verifies the migration defines the expected table structure, indexes, and constraints.
"""

from __future__ import annotations

import importlib.util
import inspect
from pathlib import Path
from types import ModuleType

import pytest
from tests.helpers.paths import find_repo_root

_MIGRATION_PATH = (
    find_repo_root(Path(__file__).resolve())
    / "pipeline"
    / "migrate"
    / "versions"
    / "pipeline"
    / "0001_create_jiuyan_industry_images.py"
)


def _load_migration_module() -> ModuleType:
    spec = importlib.util.spec_from_file_location(
        "_0001_create_jiuyan_industry_images",
        str(_MIGRATION_PATH),
    )
    if spec is None or spec.loader is None:
        msg = f"Failed to load migration module from {_MIGRATION_PATH}"
        raise ImportError(msg)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def migration() -> ModuleType:
    return _load_migration_module()


class TestMigrationMetadata:
    def test_revision_id(self, migration: ModuleType) -> None:
        assert migration.revision == "0001_jiuyan_industry_images"

    def test_down_revision_is_none(self, migration: ModuleType) -> None:
        assert migration.down_revision is None

    def test_upgrade_callable(self, migration: ModuleType) -> None:
        assert callable(getattr(migration, "upgrade", None))

    def test_downgrade_callable(self, migration: ModuleType) -> None:
        assert callable(getattr(migration, "downgrade", None))


class TestMigrationTableStructure:
    def test_creates_jiuyan_industry_images_table(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.upgrade)
        assert "create_table" in source
        assert "jiuyan_industry_images" in source

    @pytest.mark.parametrize(
        "column",
        [
            "image_filename",
            "image_url",
            "image_s3_key",
            "industry_id",
            "image_index",
            "download_status",
            "download_error_type",
            "download_error_message",
            "download_sha256",
            "download_bytes",
            "downloaded_at",
            "ocr_status",
            "ocr_error_type",
            "ocr_error_message",
            "ocr_result_s3_key",
            "ocr_result_row_count",
            "ocr_model",
            "ocr_started_at",
            "ocr_completed_at",
            "created_at",
            "updated_at",
        ],
    )
    def test_defines_required_column(self, migration: ModuleType, column: str) -> None:
        source = inspect.getsource(migration.upgrade)
        assert column in source, f"Missing column: {column}"


class TestMigrationConstraints:
    def test_download_status_check_constraint(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.upgrade)
        assert "ck_jiuyan_industry_images_download_status" in source
        assert "download_status in" in source

    def test_ocr_status_check_constraint(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.upgrade)
        assert "ck_jiuyan_industry_images_ocr_status" in source
        assert "ocr_status in" in source

    def test_image_filename_is_primary_key(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.upgrade)
        assert "primary_key=True" in source


class TestMigrationIndexes:
    def test_download_status_index(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.upgrade)
        assert "idx_jiuyan_industry_images_download_status" in source

    def test_ocr_claim_index(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.upgrade)
        assert "idx_jiuyan_industry_images_ocr_claim" in source

    def test_ocr_claim_index_is_partial(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.upgrade)
        assert "postgresql_where" in source
        assert "download_status = 'success'" in source


class TestMigrationDowngrade:
    def test_downgrade_drops_indexes(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.downgrade)
        assert "drop_index" in source
        assert "idx_jiuyan_industry_images_download_status" in source
        assert "idx_jiuyan_industry_images_ocr_claim" in source

    def test_downgrade_drops_table(self, migration: ModuleType) -> None:
        source = inspect.getsource(migration.downgrade)
        assert "drop_table" in source
        assert "jiuyan_industry_images" in source
