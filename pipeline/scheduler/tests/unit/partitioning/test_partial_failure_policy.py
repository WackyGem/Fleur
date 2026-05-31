from __future__ import annotations

import pytest
from scheduler.defs.partitioning.policies import FailureThreshold, PartialFailurePolicy


def test_partial_failure_policy_allows_failures_below_threshold() -> None:
    PartialFailurePolicy(FailureThreshold(max_failure_ratio=0.2)).validate(
        total_count=10,
        failure_count=2,
        context="OCR requests",
    )


def test_partial_failure_policy_fails_when_all_items_fail() -> None:
    with pytest.raises(RuntimeError, match="All OCR requests failed"):
        PartialFailurePolicy(FailureThreshold(max_failure_ratio=0.2)).validate(
            total_count=3,
            failure_count=3,
            context="OCR requests",
        )


def test_partial_failure_policy_fails_above_ratio_threshold() -> None:
    with pytest.raises(RuntimeError, match="failure rate exceeded 20%"):
        PartialFailurePolicy(FailureThreshold(max_failure_ratio=0.2)).validate(
            total_count=10,
            failure_count=3,
            context="OCR requests",
        )
