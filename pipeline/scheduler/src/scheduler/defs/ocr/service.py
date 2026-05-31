from __future__ import annotations

from collections.abc import Awaitable, Callable, Sequence
from dataclasses import dataclass, field

from scheduler.defs.common.concurrency import BoundedTaskRunner


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

    async def worker(item: T) -> T:
        await process_item(item)
        return item

    runner_result = await BoundedTaskRunner(max_concurrent_requests).run(
        items,
        item_key=str,
        worker=worker,
    )
    result.successes.extend(runner_result.successes)
    result.failure_count = runner_result.failure_count
    if runner_result.failures:
        failure = runner_result.failures[0]
        msg = failure.error_message
        raise RuntimeError(msg)
    return result
