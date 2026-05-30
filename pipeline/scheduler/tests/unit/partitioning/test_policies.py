from __future__ import annotations

from datetime import date

from scheduler.defs.partitioning.policies import (
    BackfillLimitPolicy,
    PartitionSelectionPolicy,
    TradeDateFilterPolicy,
)


def test_partition_selection_policy_splits_processed_and_skipped_keys() -> None:
    policy = PartitionSelectionPolicy(lambda partition_key: partition_key.endswith("2"))

    processed, skipped = policy.select(["2026-01-01", "2026-01-02", "2026-01-03"])

    assert processed == ["2026-01-02"]
    assert skipped == ["2026-01-01", "2026-01-03"]


def test_backfill_limit_policy_keeps_latest_dates_and_reports_skipped_window() -> None:
    policy = BackfillLimitPolicy(max_partitions=2)

    kept, skipped = policy.keep_latest_dates([date(2026, 1, 1), date(2026, 1, 2), date(2026, 1, 3)])

    assert kept == [date(2026, 1, 2), date(2026, 1, 3)]
    assert skipped == [date(2026, 1, 1)]


def test_backfill_limit_policy_validates_hard_limit() -> None:
    policy = BackfillLimitPolicy(max_partitions=2)

    try:
        policy.validate(["a", "b", "c"])
    except ValueError as error:
        assert "limited to 2 partitions" in str(error)
    else:
        raise AssertionError("Expected backfill limit validation to fail")


def test_trade_date_filter_policy_splits_trade_non_trade_and_window_skips() -> None:
    policy = TradeDateFilterPolicy(
        calendar_dates={date(2026, 1, 1), date(2026, 1, 3), date(2026, 1, 5)},
        backfill_limit=BackfillLimitPolicy(max_partitions=2),
    )

    processed, skipped_window, skipped_non_trade = policy.select(
        [
            date(2026, 1, 1),
            date(2026, 1, 2),
            date(2026, 1, 3),
            date(2026, 1, 4),
            date(2026, 1, 5),
        ]
    )

    assert processed == [date(2026, 1, 3), date(2026, 1, 5)]
    assert skipped_window == [date(2026, 1, 1)]
    assert skipped_non_trade == [date(2026, 1, 2), date(2026, 1, 4)]
