from __future__ import annotations

import asyncio

import pytest
from dagster._core.definitions.metadata.metadata_value import JsonMetadataValue
from scheduler.defs.common.concurrency import BoundedTaskOptions, BoundedTaskRunner


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


def test_bounded_task_runner_raises_when_all_tasks_fail_by_default() -> None:
    async def worker(item: int) -> int:
        msg = f"boom-{item}"
        raise RuntimeError(msg)

    with pytest.raises(RuntimeError, match="All 2 bounded tasks failed"):
        asyncio.run(
            BoundedTaskRunner(max_concurrent_tasks=2).run(
                [1, 2],
                item_key=str,
                worker=worker,
            )
        )


def test_bounded_task_runner_supports_partial_all_failure_result_when_configured() -> None:
    async def worker(item: int) -> int:
        msg = f"boom-{item}"
        raise RuntimeError(msg)

    result = asyncio.run(
        BoundedTaskRunner(
            BoundedTaskOptions(max_concurrent_tasks=2, fail_when_all_failed=False)
        ).run(
            [1, 2],
            item_key=lambda item: f"item-{item}",
            worker=worker,
        )
    )

    assert result.successes == []
    assert [failure.item_key for failure in result.failures] == ["item-1", "item-2"]


def test_bounded_task_runner_stops_queued_work_on_fail_fast() -> None:
    started: list[int] = []

    async def worker(item: int) -> int:
        started.append(item)
        if item == 1:
            raise RuntimeError("boom")
        await asyncio.sleep(0.01)
        return item

    result = asyncio.run(
        BoundedTaskRunner(
            BoundedTaskOptions(max_concurrent_tasks=1, fail_fast=True, fail_when_all_failed=False)
        ).run(
            [1, 2, 3],
            item_key=str,
            worker=worker,
        )
    )

    assert started == [1]
    assert result.successes == []
    assert result.failure_count == 1
    assert result.skipped_due_to_stop_count == 2


def test_bounded_task_runner_stops_after_failure_threshold_for_error_type() -> None:
    class NetworkError(RuntimeError):
        pass

    started: list[int] = []

    async def worker(item: int) -> int:
        started.append(item)
        raise NetworkError("timeout")

    result = asyncio.run(
        BoundedTaskRunner(
            BoundedTaskOptions(
                max_concurrent_tasks=1,
                stop_on_error_types=(NetworkError,),
                max_failures_before_stop=2,
                fail_when_all_failed=False,
            )
        ).run(
            [1, 2, 3, 4, 5],
            item_key=str,
            worker=worker,
        )
    )

    assert started == [1, 2]
    assert result.failure_count == 2
    assert result.skipped_due_to_stop_count == 3
    assert result.stopped_due_to_failure_threshold is True


def test_bounded_task_runner_preserves_success_order_when_requested() -> None:
    async def worker(item: int) -> int:
        if item == 1:
            await asyncio.sleep(0.01)
        return item

    result = asyncio.run(
        BoundedTaskRunner(BoundedTaskOptions(max_concurrent_tasks=2, preserve_order=True)).run(
            [1, 2, 3],
            item_key=str,
            worker=worker,
        )
    )

    assert result.successes == [1, 2, 3]


def test_bounded_task_runner_raises_when_failure_ratio_exceeds_threshold() -> None:
    async def worker(item: int) -> int:
        if item in {2, 3}:
            raise RuntimeError("boom")
        return item

    with pytest.raises(RuntimeError, match="failure rate exceeded 25%"):
        asyncio.run(
            BoundedTaskRunner(
                BoundedTaskOptions(max_concurrent_tasks=2, max_failure_ratio=0.25)
            ).run(
                [1, 2, 3, 4],
                item_key=str,
                worker=worker,
            )
        )


def test_bounded_task_result_metadata_uses_item_name() -> None:
    async def worker(item: int) -> int:
        if item == 2:
            raise RuntimeError("boom")
        return item

    result = asyncio.run(
        BoundedTaskRunner(max_concurrent_tasks=2).run(
            [1, 2],
            item_key=lambda item: f"task-{item}",
            worker=worker,
        )
    )

    metadata = result.metadata(item_name="partition")

    assert metadata["successful_partition_count"] == 1
    assert metadata["failed_partition_count"] == 1
    failed_keys = metadata["failed_partition_keys"]
    failed_errors = metadata["failed_partition_errors"]
    assert isinstance(failed_keys, JsonMetadataValue)
    assert isinstance(failed_errors, JsonMetadataValue)
    assert failed_keys.data == ["task-2"]
    assert failed_errors.data == {"task-2": {"type": "RuntimeError", "message": "boom"}}
    assert metadata["task_runner_skipped_partition_count"] == 0
    assert metadata["task_runner_stopped_due_to_failure_threshold"] is False
