from __future__ import annotations

import dagster as dg
import pytest
from dagster._core.events import DagsterEvent, RunFailureReason
from dagster._core.instance import DagsterInstance
from scheduler.defs.automation.slack_alerts import (
    FailureMessageInput,
    build_run_url,
    build_slack_failure_message,
    slack_asset_failure_sensor,
    truncate_error,
)
from scheduler.defs.daily.source_to_marts import (
    DAILY_JOB_NAME,
    daily__fetch_history_sources_to_marts_schedule_job,
)


class FakeSlackClient:
    def __init__(self) -> None:
        self.messages: list[dict[str, object]] = []

    def chat_postMessage(self, **kwargs: object) -> None:
        self.messages.append(kwargs)


class FailingSlackClient:
    def chat_postMessage(self, **kwargs: object) -> None:
        raise RuntimeError("slack unavailable")


class FakeSlackResource:
    def __init__(self, client: object) -> None:
        self.client = client

    def code_location(self) -> str:
        return "scheduler"

    def run_url(self, run_id: str) -> str:
        return f"http://localhost:3000/runs/{run_id}"

    def get_client(self) -> object:
        return self.client

    def channel(self) -> str:
        return "C123"


def test_build_slack_failure_message_includes_required_fields() -> None:
    message = build_slack_failure_message(
        FailureMessageInput(
            code_location_name="scheduler",
            job_name="eastmoney__daily_job",
            run_id="run-1",
            partition_key="2026",
            failed_steps=["fetch_balance"],
            asset_keys=["source/eastmoney__balance"],
            error_message="remote API failed",
            run_url="http://localhost:3000/runs/run-1",
        )
    )

    assert message.text == "Dagster asset run failed: eastmoney__daily_job (run-1)"
    block_text = message.blocks[1]["text"]["text"]
    assert "*Code location:* scheduler" in block_text
    assert "*Job:* eastmoney__daily_job" in block_text
    assert "*Run:* run-1" in block_text
    assert "*Partition:* 2026" in block_text
    assert "*Failed steps:* fetch_balance" in block_text
    assert "*Assets:* source/eastmoney__balance" in block_text
    assert "*Error:* remote API failed" in block_text
    assert "*Open run:* http://localhost:3000/runs/run-1" in block_text


def test_build_slack_failure_message_handles_missing_optional_fields() -> None:
    message = build_slack_failure_message(
        FailureMessageInput(
            code_location_name="scheduler",
            job_name="sina__trade_calendar_job",
            run_id="run-1",
        )
    )

    block_text = message.blocks[1]["text"]["text"]
    assert "*Partition:* -" in block_text
    assert "*Failed steps:* -" in block_text
    assert "*Assets:* -" in block_text
    assert "*Error:* -" in block_text
    assert "*Open run:* -" in block_text


def test_error_message_is_truncated() -> None:
    long_error = "x" * 1600

    truncated = truncate_error(long_error)

    assert len(truncated) == 1500
    assert truncated.endswith("...")


@pytest.mark.parametrize(
    ("base_url", "expected"),
    [
        ("http://localhost:3000", "http://localhost:3000/runs/run-1"),
        ("http://localhost:3000/", "http://localhost:3000/runs/run-1"),
        ("", None),
        (None, None),
    ],
)
def test_build_run_url(base_url: str | None, expected: str | None) -> None:
    assert build_run_url(base_url, "run-1") == expected


def test_slack_asset_failure_sensor_posts_message() -> None:
    client = FakeSlackClient()
    context = _failure_context(slack=FakeSlackResource(client))
    slack_asset_failure_sensor(context)

    assert client.messages == [
        {
            "channel": "C123",
            "text": f"Dagster asset run failed: {DAILY_JOB_NAME} (run-1)",
            "blocks": [
                {
                    "type": "header",
                    "text": {"type": "plain_text", "text": "Dagster asset run failed"},
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": "*Code location:* scheduler\n"
                        f"*Job:* {DAILY_JOB_NAME}\n"
                        "*Run:* run-1\n"
                        "*Partition:* 2026-01-01\n"
                        "*Failed steps:* -\n"
                        "*Assets:* source/test_asset\n"
                        f'*Error:* Execution of run for "{DAILY_JOB_NAME}" failed. run failed\n'
                        "*Open run:* http://localhost:3000/runs/run-1",
                    },
                },
            ],
        }
    ]


def test_slack_asset_failure_sensor_only_monitors_daily_job() -> None:
    assert slack_asset_failure_sensor.default_status == dg.DefaultSensorStatus.STOPPED
    assert slack_asset_failure_sensor._monitored_jobs == [
        daily__fetch_history_sources_to_marts_schedule_job
    ]


def test_slack_asset_failure_sensor_swallows_send_errors() -> None:
    context = _failure_context(slack=FakeSlackResource(FailingSlackClient()))

    slack_asset_failure_sensor(context)


def _failure_context(slack: object | None = None) -> dg.RunFailureSensorContext:
    run = dg.DagsterRun(
        job_name=DAILY_JOB_NAME,
        run_id="run-1",
        asset_selection={dg.AssetKey(["source", "test_asset"])},
    )
    event = DagsterEvent.job_failure(
        DAILY_JOB_NAME,
        "run failed",
        RunFailureReason.RUN_EXCEPTION,
    )
    resources = {"slack": slack} if slack is not None else None
    instance = DagsterInstance.ephemeral()
    context = dg.build_run_status_sensor_context(
        sensor_name="slack_asset_failure_sensor",
        dagster_event=event,
        dagster_instance=instance,
        dagster_run=run,
        partition_key="2026-01-01",
        resources=resources,
    )
    return context.for_run_failure()
