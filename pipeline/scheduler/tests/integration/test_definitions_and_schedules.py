from __future__ import annotations

import dagster as dg
from scheduler.defs.clickhouse.definitions import CLICKHOUSE_RAW_ASSETS, CLICKHOUSE_RAW_JOBS
from scheduler.defs.clickhouse.specs import ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
from scheduler.defs.dbt_jobs import (
    DBT_JOBS,
    STOCK_JOBS,
    TRANSFORMATION_JOBS,
    TRANSFORMATION_SCHEDULES,
)
from scheduler.defs.definitions import SOURCE_BUNDLES
from scheduler.defs.definitions import defs as scheduler_defs

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
    expected_jobs = {job.name for bundle in SOURCE_BUNDLES for job in bundle.jobs}
    expected_clickhouse_assets = {asset_key(asset) for asset in CLICKHOUSE_RAW_ASSETS}
    expected_clickhouse_jobs = {job.name for job in CLICKHOUSE_RAW_JOBS}
    expected_transformation_jobs = {job.name for job in TRANSFORMATION_JOBS}
    expected_transformation_schedules = {schedule.name for schedule in TRANSFORMATION_SCHEDULES}
    expected_schedules = {
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
    assert len(registered_asset_keys) == len(expected_assets | expected_clickhouse_assets) + 41
    assert {job.name for job in loaded_defs.jobs or []} == (
        expected_jobs | expected_clickhouse_jobs | expected_transformation_jobs
    )
    assert {schedule.name for schedule in loaded_defs.schedules or []} == (
        expected_schedules | expected_transformation_schedules
    )
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
    }


def test_clickhouse_raw_sync_all_job_is_registered_and_covers_enabled_assets() -> None:
    loaded_defs = scheduler_defs.load_fn()
    job_names = {job.name for job in loaded_defs.jobs or []}
    enabled_asset_keys = {
        spec.raw_asset_key.to_user_string() for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
    }
    registered_asset_keys = {asset_key(asset) for asset in CLICKHOUSE_RAW_ASSETS}

    assert "clickhouse__raw_sync_all_job" in job_names
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
    mart_trend_key = dg.AssetKey("mart_stock_trend_indicator")
    mart_momentum_key = dg.AssetKey("mart_stock_momentum_indicator")
    mart_volume_key = dg.AssetKey("mart_stock_volume_indicator")
    dbt_asset_def = next(asset for asset in loaded_defs.assets or [] if stg_ths_key in asset.keys)

    assert len(dbt_asset_def.keys) == 35
    assert len(dbt_asset_def.check_keys) == 253
    assert "stg_ths__limit_up_pool_compacted" in loaded_asset_keys
    assert "int_stock_kdj_daily" in loaded_asset_keys
    assert "int_stock_ma_daily" in loaded_asset_keys
    assert "int_stock_rsi_daily" in loaded_asset_keys
    assert "int_stock_boll_daily" in loaded_asset_keys
    assert "int_stock_macd_daily" in loaded_asset_keys
    assert "int_stock_price_pattern_daily" in loaded_asset_keys
    assert "mart_stock_quotes_daily" in loaded_asset_keys
    assert "mart_stock_trend_indicator" in loaded_asset_keys
    assert "mart_stock_momentum_indicator" in loaded_asset_keys
    assert "mart_stock_volume_indicator" in loaded_asset_keys
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
    assert {job.name for job in DBT_JOBS} == {
        "dbt__marts_build_job",
        "dbt__staging_build_job",
    }
    assert {job.name for job in STOCK_JOBS} == {"stock__daily_build_job"}
    assert {schedule.name for schedule in TRANSFORMATION_SCHEDULES} == {
        "stock__daily_build_schedule"
    }


def test_stock_daily_job_splits_dbt_around_furnace_assets() -> None:
    loaded_defs = scheduler_defs.load_fn()
    job = loaded_defs.resolve_job_def("stock__daily_build_job")
    dbt_node_names = {node.name for node in job.graph.nodes if node.definition.name == "elt"}
    calc_node_names = {
        "fleur_calculation__calc_stock_kdj_daily",
        "fleur_calculation__calc_stock_ma_daily",
        "fleur_calculation__calc_stock_rsi_daily",
        "fleur_calculation__calc_stock_boll_daily",
        "fleur_calculation__calc_stock_macd_daily",
        "fleur_calculation__calc_stock_price_pattern_daily",
    }

    assert len(dbt_node_names) == 2

    dependency_structure = job.graph.dependency_structure
    deps_by_node = {
        node.name: dependency_structure.input_to_upstream_outputs_for_node(node.name)
        for node in job.graph.nodes
    }

    for calc_node_name in calc_node_names:
        upstream_node_names = {
            output.node_name
            for upstream_outputs in deps_by_node[calc_node_name].values()
            for output in upstream_outputs
        }
        assert upstream_node_names & dbt_node_names

    downstream_dbt_nodes = [
        node_name
        for node_name in dbt_node_names
        if calc_node_names
        <= {
            output.node_name
            for upstream_outputs in deps_by_node[node_name].values()
            for output in upstream_outputs
        }
    ]
    assert len(downstream_dbt_nodes) == 1


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
        "source/baostock__query_stock_basic",
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
