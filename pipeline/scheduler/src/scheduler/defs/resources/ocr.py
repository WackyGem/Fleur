from __future__ import annotations

import dagster as dg

from scheduler.defs.config.env import (
    JIUYAN_OCR_BASE_URL,
    JIUYAN_OCR_MAX_CONCURRENT_REQUESTS,
    JIUYAN_OCR_MAX_RETRIES,
    JIUYAN_OCR_MODEL_NAME,
    JIUYAN_OCR_STALE_RUNNING_SECONDS,
    JIUYAN_OCR_TIMEOUT_SECONDS,
)
from scheduler.defs.config.models import JiuyanOcrConfig


class JiuyanOcrSettingsResource(dg.ConfigurableResource):
    base_url: str = JIUYAN_OCR_BASE_URL
    model_name: str = JIUYAN_OCR_MODEL_NAME
    timeout_seconds: int = JIUYAN_OCR_TIMEOUT_SECONDS
    max_retries: int = JIUYAN_OCR_MAX_RETRIES
    max_concurrent_requests: int = JIUYAN_OCR_MAX_CONCURRENT_REQUESTS
    stale_running_seconds: int = JIUYAN_OCR_STALE_RUNNING_SECONDS

    def config(self) -> JiuyanOcrConfig:
        return JiuyanOcrConfig(
            base_url=self.base_url,
            model_name=self.model_name,
            timeout_seconds=self.timeout_seconds,
            max_retries=self.max_retries,
            max_concurrent_requests=self.max_concurrent_requests,
            stale_running_seconds=self.stale_running_seconds,
        )
