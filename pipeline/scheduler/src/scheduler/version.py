from __future__ import annotations

import tomllib
from importlib import metadata
from pathlib import Path

PACKAGE_NAME = "scheduler"
SOURCE_PYPROJECT = Path(__file__).resolve().parents[2] / "pyproject.toml"


def scheduler_version() -> str:
    try:
        return metadata.version(PACKAGE_NAME)
    except metadata.PackageNotFoundError:
        if not SOURCE_PYPROJECT.exists():
            raise
        project = tomllib.loads(SOURCE_PYPROJECT.read_text(encoding="utf-8"))["project"]
        return str(project["version"])
