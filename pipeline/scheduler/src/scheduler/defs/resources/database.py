from __future__ import annotations

import dagster as dg

from scheduler.defs.config.env import PIPELINE_DATABASE_URL
from scheduler.defs.repositories.industry_images import (
    PostgresIndustryImageRepository,
    connection_factory_from_url,
)


class IndustryImageRepositoryResource(dg.ConfigurableResource):
    database_url: str = PIPELINE_DATABASE_URL

    def repository(self) -> PostgresIndustryImageRepository:
        return PostgresIndustryImageRepository(connection_factory_from_url(self.database_url))
