from __future__ import annotations

import os
from pathlib import Path

from fleur_contracts.loader import PIPELINE_ROOT

REPO_ROOT = PIPELINE_ROOT.parent
DEFAULT_LOCAL_RUSTFS_API_PORT = 34050
DEFAULT_LOCAL_CLICKHOUSE_HTTP_PORT = 34052


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


def env_value(name: str) -> str | None:
    value = os.environ.get(name)
    if value is None or not value.strip():
        return None
    return value.strip()


def env_int_or_default(name: str, default: int) -> int:
    value = env_value(name)
    if value is None:
        return default
    if value.isdecimal():
        return int(value)
    msg = f"{name} must be an integer"
    raise RuntimeError(msg)


def local_rustfs_endpoint() -> str:
    value = env_value("RUSTFS_ENDPOINT")
    if value is not None:
        return value
    port = env_int_or_default("RUSTFS_API_PORT", DEFAULT_LOCAL_RUSTFS_API_PORT)
    return f"http://127.0.0.1:{port}"


def local_clickhouse_host() -> str:
    return env_value("CLICKHOUSE_HOST") or "127.0.0.1"


def local_clickhouse_port() -> int:
    default_port = env_int_or_default(
        "CLICKHOUSE_HTTP_PORT",
        DEFAULT_LOCAL_CLICKHOUSE_HTTP_PORT,
    )
    return env_int_or_default("CLICKHOUSE_PORT", default_port)
