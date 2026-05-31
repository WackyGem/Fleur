"""Tests for OCR state flow and transitions.

This module tests the state machine logic for OCR processing:
- pending → running → success
- pending → running → failed
- Stale running state reclamation
- Force OCR reclamation
"""

from __future__ import annotations

import pytest
from scheduler.defs.repositories.industry_images import (
    OcrSuccessUpdate,
    PostgresIndustryImageRepository,
    connection_factory_from_url,
)
from scheduler.defs.sources.jiuyan.ocr_schema import OcrStatus
from tests.fakes.database import mock_database_connection


class TestOcrStateFlow:
    """Test OCR state transitions and reclamation logic."""

    def test_claim_images_transitions_pending_to_running(self) -> None:
        """Test that claiming images transitions them from pending to running."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            mock_cursor.fetchall.return_value = [
                {
                    "image_filename": "test1.jpg",
                    "image_url": "https://example.com/test1.jpg",
                    "image_s3_key": "images/test1.jpg",
                    "industry_id": "industry1",
                    "image_index": 0,
                    "download_status": "success",
                    "ocr_status": "running",
                    "ocr_result_s3_key": None,
                },
                {
                    "image_filename": "test2.jpg",
                    "image_url": "https://example.com/test2.jpg",
                    "image_s3_key": "images/test2.jpg",
                    "industry_id": "industry2",
                    "image_index": 0,
                    "download_status": "success",
                    "ocr_status": "running",
                    "ocr_result_s3_key": None,
                },
            ]

            claimed = repo.claim_ocr_images(
                limit=2,
                image_filenames=None,
                stale_after_seconds=3600,
                force_ocr=False,
            )

            assert len(claimed) == 2
            assert all(img.ocr_status is OcrStatus.RUNNING for img in claimed)

            # Verify the SQL query was executed
            assert mock_cursor.execute.called
            sql = mock_cursor.execute.call_args[0][0].lower()
            assert "update jiuyan_industry_images" in sql
            assert "ocr_status = 'running'" in sql
            assert "pending" in sql

    def test_mark_ocr_success_transitions_running_to_success(self) -> None:
        """Test that marking OCR success transitions from running to success."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            repo.mark_ocr_success(
                image_filename="test.jpg",
                ocr_result_s3_key_value="ocr_results/test.parquet",
                ocr_result_row_count=42,
                ocr_model="qwen-vl-max",
            )

            # Verify the SQL query was executed
            assert mock_cursor.executemany.called
            sql = mock_cursor.executemany.call_args[0][0].lower()
            params = mock_cursor.executemany.call_args[0][1][0]

            assert "update jiuyan_industry_images" in sql
            assert "ocr_status = 'success'" in sql
            assert "ocr_result_s3_key" in sql
            assert "ocr_result_row_count" in sql
            assert "ocr_model" in sql

            assert params["image_filename"] == "test.jpg"
            assert params["ocr_result_s3_key"] == "ocr_results/test.parquet"
            assert params["ocr_result_row_count"] == 42
            assert params["ocr_model"] == "qwen-vl-max"

    def test_mark_ocr_failed_transitions_running_to_failed(self) -> None:
        """Test that marking OCR failure transitions from running to failed."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            repo.mark_ocr_failed(
                image_filename="test.jpg",
                error_type="HTTPError",
                error_message="Timeout after 30s",
            )

            # Verify the SQL query was executed
            assert mock_cursor.executemany.called
            sql = mock_cursor.executemany.call_args[0][0].lower()
            params = mock_cursor.executemany.call_args[0][1][0]

            assert "update jiuyan_industry_images" in sql
            assert "ocr_status = 'failed'" in sql
            assert "ocr_error_type" in sql
            assert "ocr_error_message" in sql

            assert params["image_filename"] == "test.jpg"
            assert params["ocr_error_type"] == "HTTPError"
            assert params["ocr_error_message"] == "Timeout after 30s"

    def test_batch_status_updates_reuse_one_connection(self) -> None:
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            mock_cursor.rowcount = 2
            success_count = repo.mark_ocr_success_many(
                [
                    OcrSuccessUpdate(
                        image_filename="a.jpg",
                        ocr_result_s3_key="ocr/a.parquet",
                        ocr_result_row_count=1,
                        ocr_model="qwen-vl-max",
                    ),
                    OcrSuccessUpdate(
                        image_filename="b.jpg",
                        ocr_result_s3_key="ocr/b.parquet",
                        ocr_result_row_count=0,
                        ocr_model="qwen-vl-max",
                    ),
                ]
            )

            assert success_count == 2
            assert mock_cursor.executemany.call_count == 1
            first_sql = mock_cursor.executemany.call_args[0][0].lower()
            second_params = mock_cursor.executemany.call_args[0][1][1]
            assert "ocr_status = 'success'" in first_sql
            assert second_params["image_filename"] == "b.jpg"
            assert second_params["ocr_result_row_count"] == 0

    def test_claim_stale_running_images_reclaims_timeout(self) -> None:
        """Test that stale running images are reclaimed back to pending."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))
        stale_threshold_seconds = 3600  # 1 hour

        with mock_database_connection() as mock_cursor:
            mock_cursor.fetchall.return_value = [
                {
                    "image_filename": "stale.jpg",
                    "image_url": "https://example.com/stale.jpg",
                    "image_s3_key": "images/stale.jpg",
                    "industry_id": "industry1",
                    "image_index": 0,
                    "download_status": "success",
                    "ocr_status": "running",
                    "ocr_result_s3_key": None,
                },
            ]

            claimed = repo.claim_ocr_images(
                limit=1,
                image_filenames=None,
                stale_after_seconds=stale_threshold_seconds,
                force_ocr=False,
            )

            assert len(claimed) == 1
            assert claimed[0].image_filename == "stale.jpg"

            # Verify the SQL query includes stale threshold logic
            assert mock_cursor.execute.called
            sql = mock_cursor.execute.call_args[0][0].lower()
            params = mock_cursor.execute.call_args[0][1]

            assert "update jiuyan_industry_images" in sql
            assert "ocr_status = 'pending'" in sql or "ocr_status = 'running'" in sql
            assert params["stale_after_seconds"] == stale_threshold_seconds

    def test_force_ocr_reclaims_success_images(self) -> None:
        """Test that force OCR reclaims success images for reprocessing."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            mock_cursor.fetchall.return_value = [
                {
                    "image_filename": "already_done.jpg",
                    "image_url": "https://example.com/already_done.jpg",
                    "image_s3_key": "images/already_done.jpg",
                    "industry_id": "industry1",
                    "image_index": 0,
                    "download_status": "success",
                    "ocr_status": "running",
                    "ocr_result_s3_key": "ocr_results/already_done.parquet",
                },
            ]

            claimed = repo.claim_ocr_images(
                limit=1,
                image_filenames=["already_done.jpg"],
                stale_after_seconds=3600,
                force_ocr=True,
            )

            assert len(claimed) == 1
            assert claimed[0].image_filename == "already_done.jpg"

            # Verify the SQL query was executed with force_ocr flag
            assert mock_cursor.execute.called
            params = mock_cursor.execute.call_args[0][1]
            assert params["force_ocr"] is True
            assert "already_done.jpg" in params["image_filenames"]

    def test_claim_respects_limit_parameter(self) -> None:
        """Test that claim operations respect the limit parameter."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            mock_cursor.fetchall.return_value = []

            repo.claim_ocr_images(
                limit=5,
                image_filenames=None,
                stale_after_seconds=3600,
                force_ocr=False,
            )

            assert mock_cursor.execute.called
            params = mock_cursor.execute.call_args[0][1]
            assert params["limit_value"] == 5

    def test_claim_with_specific_filenames(self) -> None:
        """Test that claim operations can target specific filenames."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            mock_cursor.fetchall.return_value = []

            target_filenames = ["img1.jpg", "img2.jpg", "img3.jpg"]
            repo.claim_ocr_images(
                limit=3,
                image_filenames=target_filenames,
                stale_after_seconds=3600,
                force_ocr=False,
            )

            assert mock_cursor.execute.called
            sql = mock_cursor.execute.call_args[0][0]
            params = mock_cursor.execute.call_args[0][1]

            assert "image_filename = any" in sql
            assert params["image_filenames"] == target_filenames

    def test_full_state_flow_pending_to_success(self) -> None:
        """Test the complete state flow: pending → running → success."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            # Step 1: Claim pending images (pending → running)
            mock_cursor.fetchall.return_value = [
                {
                    "image_filename": "flow_test.jpg",
                    "image_url": "https://example.com/flow_test.jpg",
                    "image_s3_key": "images/flow_test.jpg",
                    "industry_id": "industry1",
                    "image_index": 0,
                    "download_status": "success",
                    "ocr_status": "running",
                    "ocr_result_s3_key": None,
                },
            ]

            claimed = repo.claim_ocr_images(
                limit=1,
                image_filenames=None,
                stale_after_seconds=3600,
                force_ocr=False,
            )
            assert len(claimed) == 1
            assert claimed[0].ocr_status is OcrStatus.RUNNING

            # Step 2: Mark OCR success (running → success)
            mock_cursor.execute.reset_mock()
            repo.mark_ocr_success(
                image_filename="flow_test.jpg",
                ocr_result_s3_key_value="ocr_results/flow_test.parquet",
                ocr_result_row_count=10,
                ocr_model="qwen-vl-max",
            )

            # Verify the success update was executed
            assert mock_cursor.executemany.called
            sql = mock_cursor.executemany.call_args[0][0]
            assert "ocr_status = 'success'" in sql

    def test_full_state_flow_pending_to_failed(self) -> None:
        """Test the complete state flow: pending → running → failed."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            # Step 1: Claim pending images (pending → running)
            mock_cursor.fetchall.return_value = [
                {
                    "image_filename": "fail_test.jpg",
                    "image_url": "https://example.com/fail_test.jpg",
                    "image_s3_key": "images/fail_test.jpg",
                    "industry_id": "industry1",
                    "image_index": 0,
                    "download_status": "success",
                    "ocr_status": "running",
                    "ocr_result_s3_key": None,
                },
            ]

            claimed = repo.claim_ocr_images(
                limit=1,
                image_filenames=None,
                stale_after_seconds=3600,
                force_ocr=False,
            )
            assert len(claimed) == 1
            assert claimed[0].ocr_status is OcrStatus.RUNNING

            # Step 2: Mark OCR failed (running → failed)
            mock_cursor.execute.reset_mock()
            repo.mark_ocr_failed(
                image_filename="fail_test.jpg",
                error_type="ValidationError",
                error_message="Invalid image format",
            )

            # Verify the failure update was executed
            assert mock_cursor.executemany.called
            sql = mock_cursor.executemany.call_args[0][0]
            assert "ocr_status = 'failed'" in sql

    def test_concurrent_claims_use_select_for_update(self) -> None:
        """Test that concurrent claims use FOR UPDATE SKIP LOCKED to prevent duplicates."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            mock_cursor.fetchall.return_value = []

            repo.claim_ocr_images(
                limit=10,
                image_filenames=None,
                stale_after_seconds=3600,
                force_ocr=False,
            )

            assert mock_cursor.execute.called
            sql = mock_cursor.execute.call_args[0][0].lower()

            # Verify the SQL uses FOR UPDATE SKIP LOCKED for concurrency safety
            assert "for update skip locked" in sql

    def test_claim_rejects_invalid_limit(self) -> None:
        """Test that claim operations reject invalid limit values."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with pytest.raises(ValueError, match="limit must be positive"):
            repo.claim_ocr_images(
                limit=0,
                image_filenames=None,
                stale_after_seconds=3600,
                force_ocr=False,
            )

        with pytest.raises(ValueError, match="limit must be positive"):
            repo.claim_ocr_images(
                limit=-1,
                image_filenames=None,
                stale_after_seconds=3600,
                force_ocr=False,
            )

    def test_claim_rejects_negative_stale_threshold(self) -> None:
        """Test that claim operations reject negative stale threshold."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with pytest.raises(ValueError, match="stale_after_seconds must be non-negative"):
            repo.claim_ocr_images(
                limit=10,
                image_filenames=None,
                stale_after_seconds=-1,
                force_ocr=False,
            )

    def test_claim_returns_empty_list_when_no_images_available(self) -> None:
        """Test that claim returns empty list when no images are available."""
        repo = PostgresIndustryImageRepository(connection_factory_from_url("postgresql://test"))

        with mock_database_connection() as mock_cursor:
            mock_cursor.fetchall.return_value = []

            claimed = repo.claim_ocr_images(
                limit=10,
                image_filenames=None,
                stale_after_seconds=3600,
                force_ocr=False,
            )

            assert claimed == []


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
