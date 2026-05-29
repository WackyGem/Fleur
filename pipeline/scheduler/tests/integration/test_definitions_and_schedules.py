from __future__ import annotations

from datetime import date, datetime
from zoneinfo import ZoneInfo

import dagster as dg
import pytest
from scheduler.defs.definitions import defs as scheduler_defs
from scheduler.defs.http import schedules
from scheduler.defs.market import schedules as market_schedules

from scheduler import definitions as top_level_definitions


def schedule_result(schedule: dg.ScheduleDefinition, scheduled_time: datetime) -> object:
    execution_fn = schedule.__dict__["_execution_fn"]
    assert execution_fn is not None
    return execution_fn(dg.build_schedule_context(scheduled_execution_time=scheduled_time))


def test_definitions_register_expected_jobs_schedules_assets_and_resources() -> None:
    loaded_defs = scheduler_defs.load_fn()

    assert top_level_definitions.defs is scheduler_defs
    assets = loaded_defs.assets or []
    asset_contracts = {
        asset.key.to_user_string(): {
            "group": asset.group_names_by_key[asset.key],
            "partitions_def": asset.partitions_def.__class__.__name__
            if asset.partitions_def is not None
            else None,
            "metadata": asset.metadata_by_key.get(asset.key, {}),
        }
        for asset in assets
    }
    asset_dependency_contracts = {
        asset.key.to_user_string(): sorted(key.to_user_string() for key in asset.dependency_keys)
        for asset in assets
    }
    assert asset_dependency_contracts == {
        "source/sina__trade_calendar": [],
        "source/jiuyan__action_field": ["source/sina__trade_calendar"],
        "source/jiuyan__action_field_compacted": [
            "source/jiuyan__action_field",
            "source/sina__trade_calendar",
        ],
        "source/ths__limit_up_pool": ["source/sina__trade_calendar"],
        "source/ths__limit_up_pool_compacted": [
            "source/sina__trade_calendar",
            "source/ths__limit_up_pool",
        ],
        "source/jiuyan__industry_list": [],
        "source/jiuyan__industry_images": ["source/jiuyan__industry_list"],
        "source/jiuyan__industry_ocr": ["source/jiuyan__industry_images"],
        "source/baostock__query_stock_basic": [],
        "source/baostock__query_history_k_data_plus_daily": [
            "source/baostock__query_stock_basic",
            "source/sina__trade_calendar",
        ],
        "source/eastmoney__balance": ["source/baostock__query_stock_basic"],
        "source/eastmoney__cashflow_sq": [
            "source/baostock__query_stock_basic",
            "source/eastmoney__balance",
        ],
        "source/eastmoney__cashflow_ytd": [
            "source/baostock__query_stock_basic",
            "source/eastmoney__cashflow_sq",
        ],
        "source/eastmoney__dividend_allotment": [
            "source/baostock__query_stock_basic",
            "source/eastmoney__cashflow_ytd",
        ],
        "source/eastmoney__dividend_main": [
            "source/baostock__query_stock_basic",
            "source/eastmoney__dividend_allotment",
        ],
        "source/eastmoney__equity_history": [
            "source/baostock__query_stock_basic",
            "source/eastmoney__dividend_main",
        ],
        "source/eastmoney__income_sq": [
            "source/baostock__query_stock_basic",
            "source/eastmoney__equity_history",
        ],
        "source/eastmoney__income_ytd": [
            "source/baostock__query_stock_basic",
            "source/eastmoney__income_sq",
        ],
    }
    assert asset_contracts == {
        "source/sina__trade_calendar": {
            "group": "s3_sources",
            "partitions_def": None,
            "metadata": {},
        },
        "source/jiuyan__action_field": {
            "group": "s3_sources",
            "partitions_def": "DailyPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "trade_date",
                "partitions_def": "daily_partitions",
                "trade_date_filter": "source/sina__trade_calendar",
                "allow_empty": True,
                "sparse_partition_output": True,
                "flatten_column_naming": "shortest_leaf",
            },
        },
        "source/jiuyan__action_field_compacted": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "partitions_def": "year_partitions",
                "input_partition_key_name": "trade_date",
                "input_asset": "source/jiuyan__action_field",
            },
        },
        "source/ths__limit_up_pool": {
            "group": "s3_sources",
            "partitions_def": "DailyPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "trade_date",
                "partitions_def": "daily_partitions",
                "trade_date_filter": "source/sina__trade_calendar",
                "allow_empty": True,
                "sparse_partition_output": True,
                "flatten_column_naming": "shortest_leaf",
            },
        },
        "source/ths__limit_up_pool_compacted": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "partitions_def": "year_partitions",
                "input_partition_key_name": "trade_date",
                "input_asset": "source/ths__limit_up_pool",
            },
        },
        "source/jiuyan__industry_list": {
            "group": "s3_sources",
            "partitions_def": None,
            "metadata": {
                "storage_mode": "latest_snapshot",
                "flatten_column_naming": "shortest_leaf",
            },
        },
        "source/jiuyan__industry_images": {
            "group": "s3_sources",
            "partitions_def": None,
            "metadata": {},
        },
        "source/jiuyan__industry_ocr": {
            "group": "s3_sources",
            "partitions_def": None,
            "metadata": {},
        },
        "source/baostock__query_stock_basic": {
            "group": "s3_sources",
            "partitions_def": None,
            "metadata": {"storage_mode": "latest_snapshot"},
        },
        "source/baostock__query_history_k_data_plus_daily": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
            },
        },
        "source/eastmoney__balance": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "allow_empty": True,
            },
        },
        "source/eastmoney__cashflow_sq": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "allow_empty": True,
                "execution_ordering_dependency": "source/eastmoney__balance",
            },
        },
        "source/eastmoney__cashflow_ytd": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "allow_empty": True,
                "execution_ordering_dependency": "source/eastmoney__cashflow_sq",
            },
        },
        "source/eastmoney__dividend_allotment": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "allow_empty": True,
                "execution_ordering_dependency": "source/eastmoney__cashflow_ytd",
            },
        },
        "source/eastmoney__dividend_main": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "allow_empty": True,
                "execution_ordering_dependency": "source/eastmoney__dividend_allotment",
            },
        },
        "source/eastmoney__equity_history": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "allow_empty": True,
                "execution_ordering_dependency": "source/eastmoney__dividend_main",
            },
        },
        "source/eastmoney__income_sq": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "allow_empty": True,
                "execution_ordering_dependency": "source/eastmoney__equity_history",
            },
        },
        "source/eastmoney__income_ytd": {
            "group": "s3_sources",
            "partitions_def": "TimeWindowPartitionsDefinition",
            "metadata": {
                "storage_mode": "partitioned",
                "partition_key_name": "year",
                "allow_empty": True,
                "execution_ordering_dependency": "source/eastmoney__income_sq",
            },
        },
    }
    assert {job.name for job in loaded_defs.jobs or []} == {
        "sina__trade_calendar_job",
        "jiuyan__action_field_daily_job",
        "jiuyan__action_field_compacted_job",
        "ths__limit_up_pool_daily_job",
        "ths__limit_up_pool_compacted_job",
        "jiuyan__industry_list_snapshot_job",
        "jiuyan__industry_ocr_pipeline_job",
        "baostock__daily_job",
        "eastmoney__daily_job",
    }
    assert {schedule.name for schedule in loaded_defs.schedules or []} == {
        "sina__trade_calendar_schedule",
        "jiuyan__action_field_daily_schedule",
        "ths__limit_up_pool_daily_schedule",
        "jiuyan__industry_list_snapshot_schedule",
        "jiuyan__industry_ocr_pipeline_schedule",
        "baostock__daily_schedule",
        "eastmoney__daily_schedule",
    }
    assert "s3_io_manager" in loaded_defs.resources


def test_trade_date_schedule_returns_run_request_for_a_market_trade_date(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(market_schedules.S3Config, "from_env", classmethod(lambda cls: object()))
    monkeypatch.setattr(
        market_schedules,
        "read_trade_dates_from_s3",
        lambda config: {date(2026, 5, 8)},
    )

    result = schedule_result(
        schedules.jiuyan__action_field_daily_schedule,
        datetime(2026, 5, 8, 16, 45, tzinfo=ZoneInfo("Asia/Shanghai")),
    )

    assert isinstance(result, dg.RunRequest)
    assert result.partition_key == "2026-05-08"
    assert result.tags == {
        "source": "jiuyan",
        "market.trade_date": "2026-05-08",
    }
    assert result.run_config == {}


def test_trade_date_schedule_skips_non_trade_date(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(market_schedules.S3Config, "from_env", classmethod(lambda cls: object()))
    monkeypatch.setattr(
        market_schedules,
        "read_trade_dates_from_s3",
        lambda config: {date(2026, 5, 8)},
    )

    result = schedule_result(
        schedules.ths__limit_up_pool_daily_schedule,
        datetime(2026, 5, 9, 16, 45, tzinfo=ZoneInfo("Asia/Shanghai")),
    )

    assert isinstance(result, dg.SkipReason)
    assert result.skip_message == "2026-05-09 is not an A-share trade date"


def test_trade_date_schedule_skips_when_calendar_is_unavailable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def raise_calendar_error(config: object) -> set[date]:
        msg = "missing calendar"
        raise RuntimeError(msg)

    monkeypatch.setattr(market_schedules.S3Config, "from_env", classmethod(lambda cls: object()))
    monkeypatch.setattr(market_schedules, "read_trade_dates_from_s3", raise_calendar_error)

    result = schedule_result(
        schedules.jiuyan__action_field_daily_schedule,
        datetime(2026, 5, 8, 16, 45, tzinfo=ZoneInfo("Asia/Shanghai")),
    )

    assert isinstance(result, dg.SkipReason)
    assert result.skip_message is not None
    assert "materialize sina__trade_calendar first" in result.skip_message
    assert "missing calendar" in result.skip_message


def test_year_refresh_schedule_builds_partitioned_run_config() -> None:
    result = schedule_result(
        schedules.eastmoney__daily_schedule,
        datetime(2026, 5, 8, 16, 0, tzinfo=ZoneInfo("Asia/Shanghai")),
    )

    assert isinstance(result, dg.RunRequest)
    assert result.partition_key == "2026"
    assert result.tags == {
        "market.natural_date": "2026-05-08",
        "market.year": "2026",
        "source": "eastmoney",
    }
    for asset_name in schedules.EASTMONEY_DAILY_OP_NAMES:
        assert result.run_config["ops"][asset_name]["config"] == {
            "refresh_until_date": "2026-05-08"
        }
