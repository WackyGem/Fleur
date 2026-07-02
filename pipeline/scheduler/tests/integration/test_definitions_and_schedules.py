from __future__ import annotations

from typing import Any, cast

import dagster as dg
from scheduler.defs.automation.source_raw_backfill import (
    BACKFILL_JOB_NAME,
)
from scheduler.defs.automation.source_to_marts_backfill import (
    BACKFILL_JOB_NAME as SOURCE_TO_MARTS_BACKFILL_JOB_NAME,
)
from scheduler.defs.baostock.assets import baostock__query_history_k_data_plus_daily_compacted
from scheduler.defs.baostock.schedules import baostock__daily_job
from scheduler.defs.clickhouse.definitions import CLICKHOUSE_RAW_ASSETS, CLICKHOUSE_RAW_JOBS
from scheduler.defs.clickhouse.specs import ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
from scheduler.defs.daily.definitions import DAILY_SCHEDULE_NAME
from scheduler.defs.daily.source_to_marts import DAILY_JOB_NAME
from scheduler.defs.dbt_jobs import (
    DBT_JOBS,
    STOCK_JOBS,
    TRANSFORMATION_JOBS,
    TRANSFORMATION_SCHEDULES,
)
from scheduler.defs.definitions import SOURCE_BUNDLES
from scheduler.defs.definitions import defs as scheduler_defs
from scheduler.defs.rearview.assets import (
    DAILY_PORTFOLIO_NAV_LIQUIDATION_ASSET_KEY,
    REARVIEW_ASSETS,
)
from scheduler.defs.rearview.definitions import (
    EXAMPLE_PORTFOLIO_LIVE_JOB,
)

from scheduler import definitions as top_level_definitions


def asset_key(asset: dg.AssetsDefinition) -> str:
    return asset.key.to_user_string()


def asset_keys(asset: dg.AssetsDefinition) -> set[str]:
    return {key.to_user_string() for key in asset.keys}


def test_source_bundles_have_unique_names_and_defs() -> None:
    bundle_names = [bundle.name for bundle in SOURCE_BUNDLES]
    assert bundle_names == ["sina", "jiuyan", "ths", "baostock", "eastmoney", "chinabond"]
    assert len(bundle_names) == len(set(bundle_names))

    asset_keys = [asset_key(asset) for bundle in SOURCE_BUNDLES for asset in bundle.assets]
    job_names = [job.name for bundle in SOURCE_BUNDLES for job in bundle.jobs]
    schedule_names = [schedule.name for bundle in SOURCE_BUNDLES for schedule in bundle.schedules]

    assert len(asset_keys) == len(set(asset_keys))
    assert len(job_names) == len(set(job_names))
    assert len(schedule_names) == len(set(schedule_names))


def test_registered_definitions_match_source_bundles() -> None:
    loaded_defs = scheduler_defs.load_fn()

    expected_assets = {asset_key(asset) for bundle in SOURCE_BUNDLES for asset in bundle.assets}
    legacy_source_jobs = {job.name for bundle in SOURCE_BUNDLES for job in bundle.jobs}
    expected_clickhouse_assets = {asset_key(asset) for asset in CLICKHOUSE_RAW_ASSETS}
    legacy_clickhouse_jobs = {job.name for job in CLICKHOUSE_RAW_JOBS}
    expected_rearview_assets = {asset_key(asset) for asset in REARVIEW_ASSETS}
    legacy_source_schedules = {
        schedule.name for bundle in SOURCE_BUNDLES for schedule in bundle.schedules
    }

    assert top_level_definitions.defs is scheduler_defs
    registered_asset_keys = {key for asset in loaded_defs.assets or [] for key in asset_keys(asset)}
    assert registered_asset_keys >= expected_assets | expected_clickhouse_assets
    assert "stg_ths__limit_up_pool_compacted" in registered_asset_keys
    assert "mart_stock_quotes_daily" in registered_asset_keys
    assert "fleur_calculation/calc_stock_kdj_daily" in registered_asset_keys
    assert "fleur_calculation/calc_stock_ma_daily" in registered_asset_keys
    assert "fleur_calculation/calc_stock_rsi_daily" in registered_asset_keys
    assert "fleur_calculation/calc_stock_boll_daily" in registered_asset_keys
    assert "fleur_calculation/calc_stock_macd_daily" in registered_asset_keys
    assert "fleur_calculation/calc_stock_price_pattern_daily" in registered_asset_keys
    assert DAILY_PORTFOLIO_NAV_LIQUIDATION_ASSET_KEY.to_user_string() in registered_asset_keys
    assert "rearview/strategy_portfolio_daily_runs" not in registered_asset_keys
    assert "rearview/example_0051_portfolio_live_run" in registered_asset_keys
    assert registered_asset_keys >= expected_rearview_assets
    assert (
        len(registered_asset_keys)
        == len(expected_assets | expected_clickhouse_assets | expected_rearview_assets) + 58
    )
    registered_job_names = {job.name for job in loaded_defs.jobs or []}
    assert registered_job_names == {
        EXAMPLE_PORTFOLIO_LIVE_JOB.name,
        BACKFILL_JOB_NAME,
        SOURCE_TO_MARTS_BACKFILL_JOB_NAME,
        DAILY_JOB_NAME,
    }
    assert "strategy_portfolio__daily_run_job" not in registered_job_names
    assert not registered_job_names & legacy_source_jobs
    assert not registered_job_names & legacy_clickhouse_jobs
    assert not registered_job_names & {job.name for job in TRANSFORMATION_JOBS}
    assert "backfill__fetch_snapshot_sources_to_raw_job" not in {
        job.name for job in loaded_defs.jobs or []
    }
    assert "baostock__history_k_data_year_range_backfill_job" not in {
        job.name for job in loaded_defs.jobs or []
    }
    registered_schedule_names = {schedule.name for schedule in loaded_defs.schedules or []}
    assert registered_schedule_names == {DAILY_SCHEDULE_NAME}
    assert "portfolio__daily_run_schedule" not in registered_schedule_names
    assert EXAMPLE_PORTFOLIO_LIVE_JOB.name not in registered_schedule_names
    assert not registered_schedule_names & legacy_source_schedules
    assert not registered_schedule_names & {schedule.name for schedule in TRANSFORMATION_SCHEDULES}
    assert {sensor.name for sensor in loaded_defs.sensors or []} == {"slack_asset_failure_sensor"}
    assert set(loaded_defs.resources) >= {
        "s3_io_manager",
        "s3_settings",
        "image_object_store",
        "industry_image_repository",
        "jiuyan_ocr_settings",
        "baostock_client_factory",
        "http_client_factory",
        "clickhouse",
        "slack",
        "furnace_cli",
        "rearview_api",
    }


def test_clickhouse_raw_assets_cover_enabled_specs_without_registering_raw_sync_jobs() -> None:
    loaded_defs = scheduler_defs.load_fn()
    job_names = {job.name for job in loaded_defs.jobs or []}
    enabled_asset_keys = {
        spec.raw_asset_key.to_user_string() for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
    }
    registered_asset_keys = {asset_key(asset) for asset in CLICKHOUSE_RAW_ASSETS}

    assert "clickhouse__raw_sync_all_job" not in job_names
    assert not job_names & {job.name for job in CLICKHOUSE_RAW_JOBS}
    assert len(enabled_asset_keys) == 17
    assert enabled_asset_keys == registered_asset_keys


def test_dbt_assets_are_registered_with_raw_lineage_and_checks() -> None:
    loaded_defs = scheduler_defs.load_fn()
    loaded_asset_keys = {key for asset in loaded_defs.assets or [] for key in asset_keys(asset)}
    stg_ths_key = dg.AssetKey("stg_ths__limit_up_pool_compacted")
    int_kdj_key = dg.AssetKey("int_stock_kdj_daily")
    int_ma_key = dg.AssetKey("int_stock_ma_daily")
    int_rsi_key = dg.AssetKey("int_stock_rsi_daily")
    int_boll_key = dg.AssetKey("int_stock_boll_daily")
    int_macd_key = dg.AssetKey("int_stock_macd_daily")
    int_price_pattern_key = dg.AssetKey("int_stock_price_pattern_daily")
    mart_key = dg.AssetKey("mart_stock_quotes_daily")
    mart_trend_key = dg.AssetKey("mart_stock_trend_indicator_daily")
    mart_momentum_key = dg.AssetKey("mart_stock_momentum_indicator_daily")
    mart_volume_key = dg.AssetKey("mart_stock_volume_indicator_daily")
    dbt_asset_def = next(asset for asset in loaded_defs.assets or [] if stg_ths_key in asset.keys)

    assert len(dbt_asset_def.keys) == 52
    assert len(dbt_asset_def.check_keys) == 388
    assert "stg_ths__limit_up_pool_compacted" in loaded_asset_keys
    assert "int_stock_kdj_daily" in loaded_asset_keys
    assert "int_stock_ma_daily" in loaded_asset_keys
    assert "int_stock_rsi_daily" in loaded_asset_keys
    assert "int_stock_boll_daily" in loaded_asset_keys
    assert "int_stock_macd_daily" in loaded_asset_keys
    assert "int_stock_price_pattern_daily" in loaded_asset_keys
    assert "mart_stock_quotes_daily" in loaded_asset_keys
    assert "mart_stock_trend_indicator_daily" in loaded_asset_keys
    assert "mart_stock_momentum_indicator_daily" in loaded_asset_keys
    assert "mart_stock_volume_indicator_daily" in loaded_asset_keys
    assert dbt_asset_def.specs_by_key[stg_ths_key].group_name == "dbt_staging"
    assert dbt_asset_def.specs_by_key[int_kdj_key].group_name == "dbt_intermediate"
    assert dbt_asset_def.specs_by_key[int_ma_key].group_name == "dbt_intermediate"
    assert dbt_asset_def.specs_by_key[int_rsi_key].group_name == "dbt_intermediate"
    assert dbt_asset_def.specs_by_key[int_boll_key].group_name == "dbt_intermediate"
    assert dbt_asset_def.specs_by_key[int_macd_key].group_name == "dbt_intermediate"
    assert dbt_asset_def.specs_by_key[int_price_pattern_key].group_name == "dbt_intermediate"
    assert dbt_asset_def.specs_by_key[mart_key].group_name == "dbt_marts"
    assert dbt_asset_def.specs_by_key[mart_trend_key].group_name == "dbt_marts"
    assert dbt_asset_def.specs_by_key[mart_momentum_key].group_name == "dbt_marts"
    assert dbt_asset_def.specs_by_key[mart_volume_key].group_name == "dbt_marts"
    assert (
        dbt_asset_def.tags_by_key[stg_ths_key].items()
        >= {
            "layer": "staging",
            "owner": "dbt",
            "storage": "clickhouse",
        }.items()
    )
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[stg_ths_key]} == {
        "clickhouse/raw/ths__limit_up_pool_compacted"
    }
    assert {
        key.to_user_string()
        for key in dbt_asset_def.asset_deps[dg.AssetKey("int_stock_basic_snapshot")]
    } == {"stg_baostock__query_stock_basic"}
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[mart_key]} == {
        "int_stock_financial_valuation",
        "int_stock_kdj_daily",
        "int_stock_quotes_daily_adj",
        "int_stock_quotes_daily_unadj",
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[mart_trend_key]} == {
        "int_stock_boll_daily",
        "int_stock_ma_daily",
        "int_stock_macd_daily",
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[mart_momentum_key]} == {
        "int_stock_kdj_daily",
        "int_stock_rsi_daily",
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[mart_volume_key]} == {
        "int_stock_ma_daily",
    }
    assert not {
        dep_key.to_user_string()
        for deps in dbt_asset_def.asset_deps.values()
        for dep_key in deps
        if dep_key.path[0] in {"fleur_intermediate", "fleur_staging"}
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[int_kdj_key]} == {
        "fleur_calculation/calc_stock_kdj_daily"
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[int_ma_key]} == {
        "fleur_calculation/calc_stock_ma_daily"
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[int_rsi_key]} == {
        "fleur_calculation/calc_stock_rsi_daily"
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[int_boll_key]} == {
        "fleur_calculation/calc_stock_boll_daily"
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[int_macd_key]} == {
        "fleur_calculation/calc_stock_macd_daily"
    }
    assert {key.to_user_string() for key in dbt_asset_def.asset_deps[int_price_pattern_key]} == {
        "fleur_calculation/calc_stock_price_pattern_daily"
    }
    assert DBT_JOBS == ()
    assert STOCK_JOBS == ()
    assert TRANSFORMATION_JOBS == ()
    assert TRANSFORMATION_SCHEDULES == ()


def test_source_bundle_contracts_are_stable() -> None:
    bundle_contracts = {
        bundle.name: {
            "assets": sorted(asset_key(asset) for asset in bundle.assets),
            "jobs": sorted(job.name for job in bundle.jobs),
            "schedules": sorted(schedule.name for schedule in bundle.schedules),
        }
        for bundle in SOURCE_BUNDLES
    }

    assert bundle_contracts["sina"] == {
        "assets": ["source/sina__trade_calendar"],
        "jobs": ["sina__trade_calendar_job"],
        "schedules": ["sina__trade_calendar_schedule"],
    }
    assert bundle_contracts["jiuyan"]["assets"] == [
        "source/jiuyan__action_field",
        "source/jiuyan__action_field_compacted",
        "source/jiuyan__industry_images",
        "source/jiuyan__industry_list",
        "source/jiuyan__industry_ocr",
        "source/jiuyan__industry_ocr_snapshot",
    ]
    assert bundle_contracts["jiuyan"]["jobs"] == [
        "jiuyan__action_field_compacted_job",
        "jiuyan__action_field_daily_job",
        "jiuyan__industry_list_snapshot_job",
        "jiuyan__industry_ocr_pipeline_job",
        "jiuyan__industry_ocr_snapshot_job",
    ]
    assert bundle_contracts["ths"]["assets"] == [
        "source/ths__limit_up_pool",
        "source/ths__limit_up_pool_compacted",
    ]
    assert bundle_contracts["baostock"]["assets"] == [
        "source/baostock__query_history_k_data_plus_daily",
        "source/baostock__query_history_k_data_plus_daily_compacted",
        "source/baostock__query_stock_basic",
    ]
    assert bundle_contracts["baostock"]["jobs"] == [
        "baostock__daily_job",
        "baostock__query_history_k_data_plus_daily_compacted_job",
    ]
    assert bundle_contracts["eastmoney"]["assets"] == [
        "source/eastmoney__balance",
        "source/eastmoney__cashflow_sq",
        "source/eastmoney__cashflow_ytd",
        "source/eastmoney__dividend_allotment",
        "source/eastmoney__dividend_main",
        "source/eastmoney__equity_history",
        "source/eastmoney__freeholders",
        "source/eastmoney__income_sq",
        "source/eastmoney__income_ytd",
    ]
    assert bundle_contracts["chinabond"] == {
        "assets": ["source/chinabond__government_bond"],
        "jobs": ["chinabond__government_bond_job"],
        "schedules": ["chinabond__government_bond_schedule"],
    }


def test_baostock_daily_job_stays_trade_date_partition_compatible() -> None:
    selection = str(baostock__daily_job.selection)

    assert 'key:"source/baostock__query_stock_basic"' in selection
    assert 'key:"source/baostock__query_history_k_data_plus_daily"' in selection
    assert "source/baostock__query_history_k_data_plus_daily_compacted" not in selection


def test_baostock_daily_kline_compacted_uses_eager_automation_condition() -> None:
    asset = baostock__query_history_k_data_plus_daily_compacted
    condition = asset.automation_conditions_by_key[asset.key]

    assert cast(Any, condition).label == "eager"


def test_chinabond_government_bond_schedule_runs_at_16_00_shanghai_time() -> None:
    from scheduler.defs.sources.chinabond.definitions import (
        chinabond__government_bond_schedule,
    )

    assert chinabond__government_bond_schedule.cron_schedule == "0 16 * * *"
    assert chinabond__government_bond_schedule.execution_timezone == "Asia/Shanghai"


def test_jiuyan_ocr_pipeline_includes_snapshot_and_schedule_limits_ocr_batch() -> None:
    from scheduler.defs.sources.jiuyan.definitions import (
        jiuyan__industry_ocr_pipeline_job,
        jiuyan__industry_ocr_pipeline_schedule,
    )

    assert str(jiuyan__industry_ocr_pipeline_job.selection) == (
        'key:"source/jiuyan__industry_list" or '
        'key:"source/jiuyan__industry_images" or '
        'key:"source/jiuyan__industry_ocr" or '
        'key:"source/jiuyan__industry_ocr_snapshot"'
    )

    tick = jiuyan__industry_ocr_pipeline_schedule.evaluate_tick(dg.build_schedule_context())
    run_requests = tick.run_requests
    assert run_requests is not None
    assert len(run_requests) == 1
    assert run_requests[0].run_config == {
        "ops": {
            "source__jiuyan__industry_ocr": {
                "config": {
                    "limit": 100,
                    "force_ocr": False,
                }
            }
        }
    }
