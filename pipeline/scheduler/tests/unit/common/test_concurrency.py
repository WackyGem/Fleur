from __future__ import annotations

import asyncio

from scheduler.defs.common.concurrency import BoundedTaskRunner


def test_bounded_task_runner_limits_concurrency_and_collects_successes() -> None:
    active_count = 0
    max_seen = 0

    async def worker(item: int) -> int:
        nonlocal active_count, max_seen
        active_count += 1
        max_seen = max(max_seen, active_count)
        await asyncio.sleep(0)
        active_count -= 1
        return item * 10

    result = asyncio.run(
        BoundedTaskRunner(max_concurrent_tasks=2).run(
            [1, 2, 3, 4],
            item_key=str,
            worker=worker,
        )
    )

    assert result.successes == [10, 20, 30, 40]
    assert result.failures == []
    assert max_seen == 2


def test_bounded_task_runner_records_failures_without_stopping_other_tasks() -> None:
    async def worker(item: int) -> int:
        if item == 2:
            raise RuntimeError("boom")
        return item

    result = asyncio.run(
        BoundedTaskRunner(max_concurrent_tasks=2).run(
            [1, 2, 3],
            item_key=lambda item: f"item-{item}",
            worker=worker,
        )
    )

    assert result.successes == [1, 3]
    assert len(result.failures) == 1
    assert result.failures[0].item_key == "item-2"
    assert result.failures[0].error_type == "RuntimeError"
