from __future__ import annotations

import asyncio
import time
from collections.abc import Awaitable, Callable, Sequence
from dataclasses import dataclass, field

from scheduler.defs.common.clock import elapsed_seconds
from scheduler.defs.common.metadata import FailureMetadataBuilder, RawMetadataValue


@dataclass(frozen=True)
class BoundedTaskOptions:
    max_concurrent_tasks: int
    fail_fast: bool = False
    max_failure_ratio: float | None = None
    fail_when_all_failed: bool = True
    preserve_order: bool = False

    def validate(self) -> None:
        if self.max_concurrent_tasks < 1:
            msg = "max_concurrent_tasks must be positive"
            raise ValueError(msg)
        if self.max_failure_ratio is not None and not 0 <= self.max_failure_ratio <= 1:
            msg = "max_failure_ratio must be between 0 and 1"
            raise ValueError(msg)


@dataclass(frozen=True)
class TaskFailure:
    item_key: str
    error_type: str
    error_message: str

    def as_dict(self) -> dict[str, str]:
        return {
            "type": self.error_type,
            "message": self.error_message,
        }


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

    def metadata(self, *, item_name: str) -> dict[str, RawMetadataValue]:
        return FailureMetadataBuilder().build(
            item_name=item_name,
            success_count=self.success_count,
            failures={failure.item_key: failure.as_dict() for failure in self.failures},
            elapsed_seconds=self.elapsed_seconds,
        )


@dataclass(frozen=True)
class BoundedTaskRunner:
    max_concurrent_tasks: int | BoundedTaskOptions

    @property
    def options(self) -> BoundedTaskOptions:
        if isinstance(self.max_concurrent_tasks, BoundedTaskOptions):
            return self.max_concurrent_tasks
        return BoundedTaskOptions(max_concurrent_tasks=self.max_concurrent_tasks)

    async def run[T, R](
        self,
        items: Sequence[T],
        *,
        item_key: Callable[[T], str],
        worker: Callable[[T], Awaitable[R]],
    ) -> BoundedTaskResult[R]:
        options = self.options
        options.validate()
        started_at = time.perf_counter()
        semaphore = asyncio.Semaphore(options.max_concurrent_tasks)
        successes_by_index: dict[int, R] = {}
        failures: list[TaskFailure] = []
        stop_scheduling = asyncio.Event()

        async def run_one(index: int, item: T) -> None:
            if stop_scheduling.is_set():
                return
            async with semaphore:
                if stop_scheduling.is_set():
                    return
                try:
                    successes_by_index[index] = await worker(item)
                except Exception as error:
                    failures.append(
                        TaskFailure(
                            item_key=item_key(item),
                            error_type=type(error).__name__,
                            error_message=str(error),
                        )
                    )
                    if options.fail_fast:
                        stop_scheduling.set()

        await asyncio.gather(*(run_one(index, item) for index, item in enumerate(items)))
        finished_at = time.perf_counter()
        if options.preserve_order:
            successes = [successes_by_index[index] for index in sorted(successes_by_index)]
        else:
            successes = list(successes_by_index.values())
        result = BoundedTaskResult(
            successes=successes,
            failures=failures,
            elapsed_seconds=elapsed_seconds(started_at, finished_at),
        )
        _validate_failure_threshold(
            item_count=len(items),
            failure_count=result.failure_count,
            options=options,
        )
        return result


def _validate_failure_threshold(
    *,
    item_count: int,
    failure_count: int,
    options: BoundedTaskOptions,
) -> None:
    if item_count == 0:
        return
    if options.fail_when_all_failed and failure_count == item_count:
        msg = f"All {item_count} bounded tasks failed"
        raise RuntimeError(msg)
    if options.max_failure_ratio is None:
        return
    failure_ratio = failure_count / item_count
    if failure_ratio > options.max_failure_ratio:
        percentage = options.max_failure_ratio * 100
        msg = f"Bounded task failure rate exceeded {percentage:g}%"
        raise RuntimeError(msg)
