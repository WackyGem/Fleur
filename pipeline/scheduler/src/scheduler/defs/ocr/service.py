from __future__ import annotations

import asyncio
from collections.abc import Awaitable, Callable, Sequence
from dataclasses import dataclass, field


@dataclass
class OcrBatchResult[T]:
    successes: list[T] = field(default_factory=list)
    failure_count: int = 0


async def run_bounded_ocr_batch[T](
    items: Sequence[T],
    *,
    max_concurrent_requests: int,
    process_item: Callable[[T], Awaitable[None]],
) -> OcrBatchResult[T]:
    if max_concurrent_requests < 1:
        msg = "max_concurrent_requests must be positive"
        raise ValueError(msg)

    result = OcrBatchResult[T]()
    semaphore = asyncio.Semaphore(max_concurrent_requests)

    async def run_one(item: T) -> None:
        async with semaphore:
            try:
                await process_item(item)
            except Exception:
                result.failure_count += 1
                raise
            result.successes.append(item)

    await asyncio.gather(*(run_one(item) for item in items))
    return result
