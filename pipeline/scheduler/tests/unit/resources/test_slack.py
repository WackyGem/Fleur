from __future__ import annotations

from unittest.mock import patch

from dagster import EnvVar
from dagster_slack import SlackResource
from scheduler.defs.config import env
from scheduler.defs.resources.slack import SlackAlertResource, resolve_optional_resource_value


def test_slack_alert_resource_extends_dagster_slack_resource() -> None:
    assert issubclass(SlackAlertResource, SlackResource)


def test_slack_alert_resource_defaults_come_from_env_module() -> None:
    fields = SlackAlertResource.model_fields

    assert fields["token"].default is env.SLACK_BOT_TOKEN
    assert fields["channel_id"].default is env.SLACK_CHANNEL_ID
    assert fields["http_proxy"].default is env.SLACK_HTTP_PROXY
    assert fields["webserver_base_url"].default is env.DAGSTER_WEBSERVER_BASE_URL
    assert fields["code_location_name"].default is env.DAGSTER_CODE_LOCATION_NAME


def test_get_client_passes_token_and_proxy_to_web_client() -> None:
    resource = SlackAlertResource(
        token="xoxb-test",
        channel_id="C123",
        http_proxy="http://proxy.example:7890",
        webserver_base_url="http://dagster.example",
        code_location_name="scheduler",
    )

    with patch("scheduler.defs.resources.slack.WebClient") as web_client:
        client = resource.get_client()

    assert client is web_client.return_value
    web_client.assert_called_once_with(token="xoxb-test", proxy="http://proxy.example:7890")


def test_blank_proxy_is_normalized_to_none() -> None:
    resource = SlackAlertResource(
        token="xoxb-test",
        channel_id="C123",
        http_proxy="  ",
        webserver_base_url="",
        code_location_name="scheduler",
    )

    with patch("scheduler.defs.resources.slack.WebClient") as web_client:
        resource.get_client()

    web_client.assert_called_once_with(token="xoxb-test", proxy=None)


def test_slack_resource_helpers_resolve_optional_values() -> None:
    resource = SlackAlertResource(
        token="xoxb-test",
        channel_id=" C123 ",
        http_proxy="",
        webserver_base_url=" http://dagster.example/ ",
        code_location_name=" ",
    )

    assert resource.channel() == "C123"
    assert resource.proxy_url() is None
    assert resource.run_url("run-1") == "http://dagster.example/runs/run-1"
    assert resource.code_location() == "-"


def test_optional_env_value_resolution_uses_env_var_get_value(monkeypatch) -> None:
    monkeypatch.setenv("SLACK_HTTP_PROXY", "http://proxy.local:7890")

    assert resolve_optional_resource_value(EnvVar("SLACK_HTTP_PROXY")) == "http://proxy.local:7890"
