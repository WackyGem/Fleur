import dagster as dg

from scheduler.defs.asset_contracts import (
    ocr_source_tags,
    source_owners,
    stateful_asset_metadata,
    stateful_ocr_kinds,
)
from scheduler.defs.common.async_boundary import run_async_boundary
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.market.asset_keys import SOURCE_ASSET_KEY_PREFIX
from scheduler.defs.resources.database import IndustryImageRepositoryResource
from scheduler.defs.resources.http import HttpClientFactoryResource
from scheduler.defs.resources.ocr import JiuyanOcrSettingsResource
from scheduler.defs.resources.s3 import ImageObjectStoreResource, S3SettingsResource
from scheduler.defs.sources.jiuyan.industry_list import jiuyan__industry_list
from scheduler.defs.sources.jiuyan.workflows import (
    JiuyanIndustryImageWorkflow,
    JiuyanIndustryOcrWorkflow,
    discover_images_from_table,
)
from scheduler.defs.storage.dataset_service import S3DatasetService

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
    metadata=stateful_asset_metadata(external_service="jiuyan_http"),
    owners=source_owners(),
    kinds=stateful_ocr_kinds("http", "image"),
    tags=ocr_source_tags("jiuyan"),
)
def jiuyan__industry_images(
    context: dg.AssetExecutionContext,
    config: IndustryImagesConfig,
    s3_settings: S3SettingsResource,
    image_object_store: ImageObjectStoreResource,
    industry_image_repository: IndustryImageRepositoryResource,
    http_client_factory: HttpClientFactoryResource,
) -> dg.MaterializeResult:
    result = run_async_boundary(
        _materialize_industry_images(
            context,
            config,
            s3_settings=s3_settings,
            image_object_store=image_object_store,
            industry_image_repository=industry_image_repository,
            http_client_factory=http_client_factory,
        ),
        context="JiuYan industry image materialization",
    )
    return dg.MaterializeResult(metadata=result)


@dg.asset(
    name="jiuyan__industry_ocr",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    deps=[jiuyan__industry_images],
    description="OCR result rows extracted from downloaded JiuYan industry-list images.",
    metadata=stateful_asset_metadata(external_service="jiuyan_ocr"),
    owners=source_owners(),
    kinds=stateful_ocr_kinds(),
    tags=ocr_source_tags("jiuyan"),
)
def jiuyan__industry_ocr(
    context: dg.AssetExecutionContext,
    config: IndustryOcrConfig,
    image_object_store: ImageObjectStoreResource,
    industry_image_repository: IndustryImageRepositoryResource,
    jiuyan_ocr_settings: JiuyanOcrSettingsResource,
    http_client_factory: HttpClientFactoryResource,
) -> dg.MaterializeResult:
    result = run_async_boundary(
        _materialize_industry_ocr(
            context,
            config,
            image_object_store=image_object_store,
            industry_image_repository=industry_image_repository,
            jiuyan_ocr_settings=jiuyan_ocr_settings,
            http_client_factory=http_client_factory,
        ),
        context="JiuYan OCR materialization",
    )
    return dg.MaterializeResult(metadata=result)


async def _materialize_industry_images(
    context: dg.AssetExecutionContext,
    config: IndustryImagesConfig,
    *,
    s3_settings: S3SettingsResource,
    image_object_store: ImageObjectStoreResource,
    industry_image_repository: IndustryImageRepositoryResource,
    http_client_factory: HttpClientFactoryResource,
) -> dict[str, RawMetadataValue]:
    s3_config = s3_settings.config()
    object_store = image_object_store.image_object_store()
    workflow = JiuyanIndustryImageWorkflow(
        s3_config=s3_config,
        dataset_reader=S3DatasetService(s3_config=s3_config),
        repository=industry_image_repository.repository(),
        object_store=object_store,
        upstream_asset_key=jiuyan__industry_list.key,
        http_client_factory=http_client_factory.factory(),
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
    http_client_factory: HttpClientFactoryResource,
) -> dict[str, RawMetadataValue]:
    workflow = JiuyanIndustryOcrWorkflow(
        repository=industry_image_repository.repository(),
        object_store=image_object_store.image_object_store(),
        ocr_config=jiuyan_ocr_settings.config(),
        http_client_factory=http_client_factory.factory(),
        log=context.log,
    )
    return await workflow.refresh_ocr(
        limit=config.effective_limit,
        force_ocr=config.force_ocr,
        image_filenames=config.image_filenames,
        max_concurrent_requests=config.effective_max_concurrent_requests,
    )
