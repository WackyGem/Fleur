from __future__ import annotations

import asyncio
import time
from collections.abc import Awaitable, Callable, Sequence
from dataclasses import dataclass, field

from scheduler.defs.common.clock import elapsed_seconds


@dataclass(frozen=True)
class TaskFailure:
    item_key: str
    error_type: str
    error_message: str


@dataclass(frozen=True)
class BoundedTaskResult[T]:
    successes: list[T] = field(default_factory=list)
    failures: list[TaskFailure] = field(default_factory=list)
    elapsed_seconds: float = 0.0

    @property
    def success_count(self) -> int:
        return len(self.successes)

    @property
    def failure_count(self) -> int:
        return len(self.failures)


@dataclass(frozen=True)
class BoundedTaskRunner:
    max_concurrent_tasks: int

    async def run[T, R](
        self,
        items: Sequence[T],
        *,
        item_key: Callable[[T], str],
        worker: Callable[[T], Awaitable[R]],
    ) -> BoundedTaskResult[R]:
        if self.max_concurrent_tasks < 1:
            msg = "max_concurrent_tasks must be positive"
            raise ValueError(msg)

        started_at = time.perf_counter()
        semaphore = asyncio.Semaphore(self.max_concurrent_tasks)
        successes: list[R] = []
        failures: list[TaskFailure] = []

        async def run_one(item: T) -> None:
            async with semaphore:
                try:
                    successes.append(await worker(item))
                except Exception as error:
                    failures.append(
                        TaskFailure(
                            item_key=item_key(item),
                            error_type=type(error).__name__,
                            error_message=str(error),
                        )
                    )

        await asyncio.gather(*(run_one(item) for item in items))
        finished_at = time.perf_counter()
        return BoundedTaskResult(
            successes=successes,
            failures=failures,
            elapsed_seconds=elapsed_seconds(started_at, finished_at),
        )
