from __future__ import annotations

import dagster as dg
from dagster_slack import SlackResource
from slack_sdk.web.client import WebClient

from scheduler.defs.config import env


def _resolve_optional_env_value(value: str | dg.EnvVar | None) -> str | None:
    if value is None:
        return None
    resolved = value.get_value() if isinstance(value, dg.EnvVar) else value
    if resolved is None:
        return None
    stripped = resolved.strip()
    return stripped or None


def _resolve_required_env_value(value: str | dg.EnvVar, *, field_name: str) -> str:
    resolved = _resolve_optional_env_value(value)
    if resolved is None:
        msg = f"Slack resource field {field_name} is required"
        raise RuntimeError(msg)
    return resolved


class SlackAlertResource(SlackResource):
    token: str = env.SLACK_BOT_TOKEN
    channel_id: str = env.SLACK_CHANNEL_ID
    http_proxy: str = env.SLACK_HTTP_PROXY
    webserver_base_url: str | None = None
    code_location_name: str | None = None

    def get_client(self) -> WebClient:
        token = _resolve_required_env_value(self.token, field_name="token")
        return WebClient(token=token, proxy=self.proxy_url())

    def channel(self) -> str:
        return _resolve_required_env_value(self.channel_id, field_name="channel_id")

    def proxy_url(self) -> str | None:
        return _resolve_optional_env_value(self.http_proxy)

    def run_url(self, run_id: str) -> str | None:
        base_url = _resolve_optional_env_value(
            self.webserver_base_url
        ) or _resolve_optional_env_value(env.DAGSTER_WEBSERVER_BASE_URL)
        if base_url is None:
            return None
        return f"{base_url.rstrip('/')}/runs/{run_id}"

    def code_location(self) -> str:
        return (
            _resolve_optional_env_value(self.code_location_name)
            or _resolve_optional_env_value(env.DAGSTER_CODE_LOCATION_NAME)
            or "-"
        )


def resolve_optional_resource_value(value: str | dg.EnvVar | None) -> str | None:
    return _resolve_optional_env_value(value)
