from __future__ import annotations

import asyncio
from collections.abc import Coroutine
from typing import Any


def run_async_boundary[T](awaitable: Coroutine[Any, Any, T], *, context: str) -> T:
    try:
        return asyncio.run(awaitable)
    except RuntimeError as error:
        msg = f"{context} failed at async boundary: {error}"
        raise RuntimeError(msg) from error
