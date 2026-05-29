from __future__ import annotations

from collections.abc import Generator
from contextlib import contextmanager
from unittest.mock import MagicMock, patch


@contextmanager
def mock_database_connection() -> Generator[MagicMock, None, None]:
    with patch(
        "scheduler.defs.repositories.industry_images.connect_pipeline_database"
    ) as mock_connect:
        mock_cursor = MagicMock()
        mock_connection = MagicMock()
        mock_connection.cursor.return_value = mock_cursor
        mock_connection.__enter__ = MagicMock(return_value=mock_connection)
        mock_connection.__exit__ = MagicMock(return_value=False)
        mock_cursor.__enter__ = MagicMock(return_value=mock_cursor)
        mock_cursor.__exit__ = MagicMock(return_value=False)
        mock_connect.return_value = mock_connection
        yield mock_cursor
