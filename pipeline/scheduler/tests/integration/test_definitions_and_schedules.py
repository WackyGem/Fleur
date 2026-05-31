from __future__ import annotations

import dagster as dg
from scheduler.defs.definitions import SOURCE_BUNDLES
from scheduler.defs.definitions import defs as scheduler_defs

from scheduler import definitions as top_level_definitions


def asset_key(asset: dg.AssetsDefinition) -> str:
    return asset.key.to_user_string()


def test_source_bundles_have_unique_names_and_defs() -> None:
    bundle_names = [bundle.name for bundle in SOURCE_BUNDLES]
    assert bundle_names == ["sina", "jiuyan", "ths", "baostock", "eastmoney"]
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
    expected_schedules = {
        schedule.name for bundle in SOURCE_BUNDLES for schedule in bundle.schedules
    }

    assert top_level_definitions.defs is scheduler_defs
    assert {asset_key(asset) for asset in loaded_defs.assets or []} == expected_assets
    assert {job.name for job in loaded_defs.jobs or []} == expected_jobs
    assert {schedule.name for schedule in loaded_defs.schedules or []} == expected_schedules
    assert {sensor.name for sensor in loaded_defs.sensors or []} == {"slack_asset_failure_sensor"}
    assert set(loaded_defs.resources) >= {
        "s3_io_manager",
        "s3_settings",
        "image_object_store",
        "industry_image_repository",
        "jiuyan_ocr_settings",
        "baostock_client_factory",
        "http_client_factory",
        "slack",
    }


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
        "source/eastmoney__income_sq",
        "source/eastmoney__income_ytd",
    ]
