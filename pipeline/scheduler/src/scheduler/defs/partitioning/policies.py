from __future__ import annotations

from collections.abc import Callable, Sequence
from dataclasses import dataclass
from datetime import date

PartitionFilter = Callable[[str], bool]


@dataclass(frozen=True)
class PartitionSelectionPolicy:
    partition_filter: PartitionFilter | None = None

    def select(self, partition_keys: Sequence[str]) -> tuple[list[str], list[str]]:
        processed = [
            partition_key
            for partition_key in partition_keys
            if self.partition_filter is None or self.partition_filter(partition_key)
        ]
        processed_set = set(processed)
        skipped = [
            partition_key for partition_key in partition_keys if partition_key not in processed_set
        ]
        return processed, skipped


@dataclass(frozen=True)
class BackfillLimitPolicy:
    max_partitions: int | None = None

    def validate(self, partition_keys: Sequence[str]) -> None:
        if self.max_partitions is None or len(partition_keys) <= self.max_partitions:
            return
        msg = f"Single-run partition backfill is limited to {self.max_partitions} partitions"
        raise ValueError(msg)

    def keep_latest_dates(self, values: Sequence[date]) -> tuple[list[date], list[date]]:
        if self.max_partitions is None or len(values) <= self.max_partitions:
            return list(values), []
        kept = list(values)[-self.max_partitions :]
        kept_set = set(kept)
        skipped = [value for value in values if value not in kept_set]
        return kept, skipped


@dataclass(frozen=True)
class TradeDateFilterPolicy:
    calendar_dates: set[date]
    backfill_limit: BackfillLimitPolicy = BackfillLimitPolicy()

    def select(self, requested_dates: Sequence[date]) -> tuple[list[date], list[date], list[date]]:
        requested_trade_dates = [item for item in requested_dates if item in self.calendar_dates]
        processed_trade_dates, skipped_window_trade_dates = self.backfill_limit.keep_latest_dates(
            requested_trade_dates
        )
        skipped_non_trade_dates = [
            item for item in requested_dates if item not in self.calendar_dates
        ]
        return processed_trade_dates, skipped_window_trade_dates, skipped_non_trade_dates


@dataclass(frozen=True)
class FailureThreshold:
    max_failure_ratio: float | None = None
    fail_when_all_failed: bool = True

    def validate(self) -> None:
        if self.max_failure_ratio is not None and not 0 <= self.max_failure_ratio <= 1:
            msg = "max_failure_ratio must be between 0 and 1"
            raise ValueError(msg)


@dataclass(frozen=True)
class PartialFailurePolicy:
    threshold: FailureThreshold = FailureThreshold()

    def validate(self, *, total_count: int, failure_count: int, context: str) -> None:
        self.threshold.validate()
        if total_count < 0:
            msg = "total_count must be non-negative"
            raise ValueError(msg)
        if failure_count < 0:
            msg = "failure_count must be non-negative"
            raise ValueError(msg)
        if failure_count > total_count:
            msg = "failure_count cannot exceed total_count"
            raise ValueError(msg)
        if total_count == 0:
            return
        if self.threshold.fail_when_all_failed and failure_count == total_count:
            raise RuntimeError(f"All {context} failed")
        if self.threshold.max_failure_ratio is None:
            return
        failure_ratio = failure_count / total_count
        if failure_ratio > self.threshold.max_failure_ratio:
            percentage = self.threshold.max_failure_ratio * 100
            raise RuntimeError(f"{context} failure rate exceeded {percentage:g}%")
