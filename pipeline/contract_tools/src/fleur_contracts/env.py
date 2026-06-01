from __future__ import annotations

import os
from pathlib import Path

from fleur_contracts.loader import PIPELINE_ROOT

REPO_ROOT = PIPELINE_ROOT.parent


def load_repo_dotenv_if_present(path: Path = REPO_ROOT / ".env") -> None:
    if not path.exists():
        return
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", maxsplit=1)
        key = key.strip()
        if not key or key in os.environ:
            continue
        os.environ[key] = value.strip().strip('"').strip("'")
