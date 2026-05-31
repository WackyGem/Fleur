from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from scheduler.defs.common.retry import DEFAULT_RETRY_POLICY, ExponentialBackoffPolicy
from scheduler.defs.http.client import (
    HTTP_CONNECTOR_LIMIT,
    HTTP_CONNECTOR_LIMIT_PER_HOST,
    HTTP_MAX_ATTEMPTS,
    HTTP_READ_TIMEOUT_SECONDS,
    HTTP_TOTAL_TIMEOUT_SECONDS,
    AioHttpClient,
    HeaderFactory,
)


@dataclass(frozen=True)
class HttpClientFactory:
    retry_policy: ExponentialBackoffPolicy = DEFAULT_RETRY_POLICY

    def json_client(
        self,
        *,
        headers: Mapping[str, str] | HeaderFactory | None = None,
        max_attempts: int = HTTP_MAX_ATTEMPTS,
        total_timeout_seconds: float = HTTP_TOTAL_TIMEOUT_SECONDS,
        read_timeout_seconds: float = HTTP_READ_TIMEOUT_SECONDS,
        request_delay: float = 0.0,
        connector_limit: int = HTTP_CONNECTOR_LIMIT,
        connector_limit_per_host: int = HTTP_CONNECTOR_LIMIT_PER_HOST,
    ) -> AioHttpClient:
        return AioHttpClient(
            headers=headers,
            retry_policy=self.retry_policy,
            max_attempts=max_attempts,
            total_timeout_seconds=total_timeout_seconds,
            read_timeout_seconds=read_timeout_seconds,
            request_delay=request_delay,
            connector_limit=connector_limit,
            connector_limit_per_host=connector_limit_per_host,
        )

    def bytes_client(
        self,
        *,
        headers: Mapping[str, str] | HeaderFactory | None = None,
        max_attempts: int = HTTP_MAX_ATTEMPTS,
        total_timeout_seconds: float = HTTP_TOTAL_TIMEOUT_SECONDS,
        read_timeout_seconds: float = HTTP_READ_TIMEOUT_SECONDS,
        request_delay: float = 0.0,
    ) -> AioHttpClient:
        return self.json_client(
            headers=headers,
            max_attempts=max_attempts,
            total_timeout_seconds=total_timeout_seconds,
            read_timeout_seconds=read_timeout_seconds,
            request_delay=request_delay,
        )
