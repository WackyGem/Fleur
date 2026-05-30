import asyncio

import dagster as dg

from scheduler.defs.asset_contracts import DEFAULT_OWNER, ocr_source_tags
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.resources.database import IndustryImageRepositoryResource
from scheduler.defs.resources.ocr import JiuyanOcrSettingsResource
from scheduler.defs.resources.s3 import ImageObjectStoreResource, S3SettingsResource
from scheduler.defs.sources.jiuyan.industry_list import jiuyan__industry_list
from scheduler.defs.sources.jiuyan.workflows import (
    JiuyanIndustryImageWorkflow,
    JiuyanIndustryOcrWorkflow,
    discover_images_from_table,
)

__all__ = [
    "IndustryImagesConfig",
    "IndustryOcrConfig",
    "discover_images_from_table",
    "jiuyan__industry_images",
    "jiuyan__industry_ocr",
]


class IndustryImagesConfig(dg.Config):
    limit: int | None = None
    force_download: bool = False
    image_filenames: list[str] = []

    @property
    def effective_limit(self) -> int | None:
        if self.limit is None or self.limit <= 0:
            return None
        return self.limit


class IndustryOcrConfig(dg.Config):
    limit: int | None = None
    force_ocr: bool = False
    image_filenames: list[str] = []
    max_concurrent_requests: int | None = None

    @property
    def effective_limit(self) -> int | None:
        if self.limit is None or self.limit <= 0:
            return None
        return self.limit

    @property
    def effective_max_concurrent_requests(self) -> int | None:
        if self.max_concurrent_requests is None or self.max_concurrent_requests <= 0:
            return None
        return self.max_concurrent_requests


@dg.asset(
    name="jiuyan__industry_images",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    deps=[jiuyan__industry_list],
    description="Discovered JiuYan industry-list image objects downloaded to S3 with PostgreSQL state.",
    owners=[DEFAULT_OWNER],
    kinds={"s3", "postgres", "http", "image"},
    tags=ocr_source_tags("jiuyan"),
)
def jiuyan__industry_images(
    context: dg.AssetExecutionContext,
    config: IndustryImagesConfig,
    s3_settings: S3SettingsResource,
    image_object_store: ImageObjectStoreResource,
    industry_image_repository: IndustryImageRepositoryResource,
) -> dg.MaterializeResult:
    result = asyncio.run(
        _materialize_industry_images(
            context,
            config,
            s3_settings=s3_settings,
            image_object_store=image_object_store,
            industry_image_repository=industry_image_repository,
        )
    )
    return dg.MaterializeResult(metadata=result)


@dg.asset(
    name="jiuyan__industry_ocr",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    deps=[jiuyan__industry_images],
    description="OCR result rows extracted from downloaded JiuYan industry-list images.",
    owners=[DEFAULT_OWNER],
    kinds={"s3", "postgres", "ocr"},
    tags=ocr_source_tags("jiuyan"),
)
def jiuyan__industry_ocr(
    context: dg.AssetExecutionContext,
    config: IndustryOcrConfig,
    image_object_store: ImageObjectStoreResource,
    industry_image_repository: IndustryImageRepositoryResource,
    jiuyan_ocr_settings: JiuyanOcrSettingsResource,
) -> dg.MaterializeResult:
    result = asyncio.run(
        _materialize_industry_ocr(
            context,
            config,
            image_object_store=image_object_store,
            industry_image_repository=industry_image_repository,
            jiuyan_ocr_settings=jiuyan_ocr_settings,
        )
    )
    return dg.MaterializeResult(metadata=result)


async def _materialize_industry_images(
    context: dg.AssetExecutionContext,
    config: IndustryImagesConfig,
    *,
    s3_settings: S3SettingsResource,
    image_object_store: ImageObjectStoreResource,
    industry_image_repository: IndustryImageRepositoryResource,
) -> dict[str, RawMetadataValue]:
    workflow = JiuyanIndustryImageWorkflow(
        s3_config=s3_settings.config(),
        repository=industry_image_repository.repository(),
        object_store=image_object_store.image_object_store(),
        upstream_asset_key=jiuyan__industry_list.key,
        log=context.log,
    )
    return await workflow.refresh_images(
        limit=config.effective_limit,
        force_download=config.force_download,
        image_filenames=config.image_filenames,
    )


async def _materialize_industry_ocr(
    context: dg.AssetExecutionContext,
    config: IndustryOcrConfig,
    *,
    image_object_store: ImageObjectStoreResource,
    industry_image_repository: IndustryImageRepositoryResource,
    jiuyan_ocr_settings: JiuyanOcrSettingsResource,
) -> dict[str, RawMetadataValue]:
    workflow = JiuyanIndustryOcrWorkflow(
        repository=industry_image_repository.repository(),
        object_store=image_object_store.image_object_store(),
        ocr_config=jiuyan_ocr_settings.config(),
        log=context.log,
    )
    return await workflow.refresh_ocr(
        limit=config.effective_limit,
        force_ocr=config.force_ocr,
        image_filenames=config.image_filenames,
        max_concurrent_requests=config.effective_max_concurrent_requests,
    )
