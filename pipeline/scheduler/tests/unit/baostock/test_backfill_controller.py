from __future__ import annotations

import json
from datetime import date

import pytest
from scheduler.defs.baostock.backfill_controller import (
    BackfillCommand,
    BaostockHistoryYearRangeBackfillConfig,
    build_baostock_history_year_range_backfill_commands,
)


def test_baostock_history_backfill_controller_builds_year_ordered_commands() -> None:
    config = BaostockHistoryYearRangeBackfillConfig(
        start_year=2024,
        end_year=2026,
        cutoff_trade_date="2026-06-30",
    )

    commands = build_baostock_history_year_range_backfill_commands(
        config,
        today=date(2026, 6, 30),
    )

    assert [command.label for command in commands] == [
        "daily source 2024",
        "compacted source 2024",
        "clickhouse raw 2024",
        "daily source 2025",
        "compacted source 2025",
        "clickhouse raw 2025",
        "daily source 2026",
        "compacted source 2026",
        "clickhouse raw 2026",
    ]
    assert "2024-01-01...2024-12-31" in commands[0].args
    assert "--config-json" not in commands[0].args
    assert "2026-01-01...2026-06-30" in commands[6].args
    assert _config_json(commands[6]) == {
        "ops": {
            "source__baostock__query_history_k_data_plus_daily": {
                "config": {"cutoff_trade_date": "2026-06-30"}
            }
        }
    }
    assert _config_json(commands[7]) == {
        "ops": {
            "source__baostock__query_history_k_data_plus_daily_compacted": {
                "config": {"cutoff_trade_date": "2026-06-30"}
            }
        }
    }
    assert "--config-json" not in commands[8].args


def test_baostock_history_backfill_controller_passes_overwrite_to_daily_only() -> None:
    config = BaostockHistoryYearRangeBackfillConfig(
        start_year=2024,
        end_year=2024,
        overwrite_existing_partitions=True,
    )

    commands = build_baostock_history_year_range_backfill_commands(
        config,
        today=date(2026, 6, 30),
    )

    assert _config_json(commands[0]) == {
        "ops": {
            "source__baostock__query_history_k_data_plus_daily": {
                "config": {"overwrite_existing_partitions": True}
            }
        }
    }
    assert "--config-json" not in commands[1].args
    assert "--config-json" not in commands[2].args


def test_baostock_history_backfill_controller_requires_cutoff_for_current_year() -> None:
    config = BaostockHistoryYearRangeBackfillConfig(
        start_year=2026,
        end_year=2026,
    )

    with pytest.raises(ValueError, match="cutoff_trade_date is required"):
        build_baostock_history_year_range_backfill_commands(
            config,
            today=date(2026, 6, 30),
        )


def _config_json(command: BackfillCommand) -> dict[str, object]:
    config_index = command.args.index("--config-json")
    return json.loads(command.args[config_index + 1])
