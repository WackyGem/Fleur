from __future__ import annotations

import dagster as dg

from scheduler.defs.baostock.definitions import baostock_bundle
from scheduler.defs.io_managers.s3_io_manager import S3IOManager
from scheduler.defs.resources.database import IndustryImageRepositoryResource
from scheduler.defs.resources.ocr import JiuyanOcrSettingsResource
from scheduler.defs.resources.s3 import ImageObjectStoreResource, S3SettingsResource
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
    return dg.Definitions(
        assets=bundle_assets(SOURCE_BUNDLES),
        jobs=bundle_jobs(SOURCE_BUNDLES),
        schedules=bundle_schedules(SOURCE_BUNDLES),
        resources={
            "s3_io_manager": S3IOManager(),
            "s3_settings": S3SettingsResource(),
            "image_object_store": ImageObjectStoreResource(),
            "industry_image_repository": IndustryImageRepositoryResource(),
            "jiuyan_ocr_settings": JiuyanOcrSettingsResource(),
        },
    )
