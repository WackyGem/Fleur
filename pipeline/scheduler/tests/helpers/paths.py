from __future__ import annotations

from pathlib import Path


def find_repo_root(start: Path) -> Path:
    for path in (start, *start.parents):
        if (path / "docs").is_dir() and (path / "pipeline").is_dir():
            return path
    msg = f"Could not find repository root from {start}"
    raise RuntimeError(msg)
