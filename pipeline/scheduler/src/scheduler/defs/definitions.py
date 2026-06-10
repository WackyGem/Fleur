from __future__ import annotations

from pathlib import Path

import dagster as dg

from scheduler.defs.automation.slack_alerts import slack_asset_failure_sensor
from scheduler.defs.baostock.definitions import baostock_bundle
from scheduler.defs.clickhouse.definitions import CLICKHOUSE_RAW_ASSETS, CLICKHOUSE_RAW_JOBS
from scheduler.defs.dbt_jobs import TRANSFORMATION_JOBS, TRANSFORMATION_SCHEDULES
from scheduler.defs.io_managers.s3_io_manager import S3IOManager
from scheduler.defs.resources.baostock import BaostockClientFactoryResource
from scheduler.defs.resources.clickhouse import ClickHouseResource
from scheduler.defs.resources.database import IndustryImageRepositoryResource
from scheduler.defs.resources.http import HttpClientFactoryResource
from scheduler.defs.resources.ocr import JiuyanOcrSettingsResource
from scheduler.defs.resources.s3 import ImageObjectStoreResource, S3SettingsResource
from scheduler.defs.resources.slack import SlackAlertResource
from scheduler.defs.source_bundle import (
    SourceBundle,
    bundle_assets,
    bundle_jobs,
    bundle_schedules,
)
from scheduler.defs.sources.eastmoney.definitions import eastmoney_bundle
from scheduler.defs.sources.jiuyan.definitions import jiuyan_bundle
from scheduler.defs.sources.sina.definitions import sina_bundle
from scheduler.defs.sources.ths.definitions import ths_bundle

SOURCE_BUNDLES: tuple[SourceBundle, ...] = (
    sina_bundle,
    jiuyan_bundle,
    ths_bundle,
    baostock_bundle,
    eastmoney_bundle,
)


@dg.definitions
def defs() -> dg.Definitions:
    base_defs = dg.Definitions(
        assets=[*bundle_assets(SOURCE_BUNDLES), *CLICKHOUSE_RAW_ASSETS],
        jobs=[*bundle_jobs(SOURCE_BUNDLES), *CLICKHOUSE_RAW_JOBS, *TRANSFORMATION_JOBS],
        schedules=[*bundle_schedules(SOURCE_BUNDLES), *TRANSFORMATION_SCHEDULES],
        sensors=[slack_asset_failure_sensor],
        resources={
            "s3_io_manager": S3IOManager(),
            "s3_settings": S3SettingsResource(),
            "image_object_store": ImageObjectStoreResource(),
            "industry_image_repository": IndustryImageRepositoryResource(),
            "jiuyan_ocr_settings": JiuyanOcrSettingsResource(),
            "baostock_client_factory": BaostockClientFactoryResource(),
            "http_client_factory": HttpClientFactoryResource(),
            "clickhouse": ClickHouseResource(),
            "slack": SlackAlertResource(),
        },
    )
    component_tree = dg.ComponentTree.for_project(path_within_project=Path(__file__))
    dbt_defs = component_tree.build_defs("dbt")
    furnace_defs = component_tree.build_defs("furnace")

    return dg.Definitions.merge(base_defs, dbt_defs, furnace_defs)
