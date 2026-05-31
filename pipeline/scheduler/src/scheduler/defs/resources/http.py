from __future__ import annotations

import dagster as dg

from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY
from scheduler.defs.http.client import (
    HTTP_CONNECTOR_LIMIT,
    HTTP_CONNECTOR_LIMIT_PER_HOST,
    HTTP_MAX_ATTEMPTS,
    HTTP_READ_TIMEOUT_SECONDS,
    HTTP_TOTAL_TIMEOUT_SECONDS,
)
from scheduler.defs.http.client_factory import HttpClientFactory


class HttpClientFactoryResource(dg.ConfigurableResource):
    max_attempts: int = HTTP_MAX_ATTEMPTS
    total_timeout_seconds: float = HTTP_TOTAL_TIMEOUT_SECONDS
    read_timeout_seconds: float = HTTP_READ_TIMEOUT_SECONDS
    connector_limit: int = HTTP_CONNECTOR_LIMIT
    connector_limit_per_host: int = HTTP_CONNECTOR_LIMIT_PER_HOST
    request_delay: float = 0.0

    def factory(self) -> HttpClientFactory:
        return HttpClientFactory(retry_policy=DEFAULT_RETRY_POLICY)
