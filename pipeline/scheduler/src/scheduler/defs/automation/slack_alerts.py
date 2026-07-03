from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Any

import dagster as dg

from scheduler.defs.daily.source_to_marts import daily__fetch_history_sources_to_marts_schedule_job
from scheduler.defs.resources.slack import SlackAlertResource

MAX_ERROR_LENGTH = 1500


@dataclass(frozen=True)
class SlackFailureMessage:
    text: str
    blocks: list[dict[str, Any]]


@dataclass(frozen=True)
class FailureMessageInput:
    code_location_name: str
    job_name: str
    run_id: str
    partition_key: str | None = None
    failed_steps: Sequence[str] = ()
    asset_keys: Sequence[str] = ()
    error_message: str | None = None
    run_url: str | None = None


def truncate_error(error_message: str | None, *, max_length: int = MAX_ERROR_LENGTH) -> str:
    if error_message is None:
        return "-"
    normalized = error_message.strip()
    if not normalized:
        return "-"
    if len(normalized) <= max_length:
        return normalized
    return f"{normalized[: max_length - 3]}..."


def build_run_url(webserver_base_url: str | None, run_id: str) -> str | None:
    if webserver_base_url is None:
        return None
    stripped = webserver_base_url.strip()
    if not stripped:
        return None
    return f"{stripped.rstrip('/')}/runs/{run_id}"


def build_slack_failure_message(message_input: FailureMessageInput) -> SlackFailureMessage:
    text = f"Dagster asset run failed: {message_input.job_name} ({message_input.run_id})"
    fields = [
        ("Code location", message_input.code_location_name or "-"),
        ("Job", message_input.job_name),
        ("Run", message_input.run_id),
        ("Partition", message_input.partition_key or "-"),
        ("Failed steps", _format_sequence(message_input.failed_steps)),
        ("Assets", _format_sequence(message_input.asset_keys)),
        ("Error", truncate_error(message_input.error_message)),
        ("Open run", message_input.run_url or "-"),
    ]
    block_text = "\n".join(f"*{label}:* {value}" for label, value in fields)
    return SlackFailureMessage(
        text=text,
        blocks=[
            {
                "type": "header",
                "text": {"type": "plain_text", "text": "Dagster asset run failed"},
            },
            {"type": "section", "text": {"type": "mrkdwn", "text": block_text}},
        ],
    )


@dg.run_failure_sensor(
    name="slack_asset_failure_sensor",
    default_status=dg.DefaultSensorStatus.STOPPED,
    minimum_interval_seconds=30,
    monitored_jobs=[daily__fetch_history_sources_to_marts_schedule_job],
)
def slack_asset_failure_sensor(
    context: dg.RunFailureSensorContext,
    slack: SlackAlertResource,
) -> None:
    message = build_slack_failure_message(
        FailureMessageInput(
            code_location_name=slack.code_location(),
            job_name=context.dagster_run.job_name,
            run_id=context.dagster_run.run_id,
            partition_key=context.partition_key,
            failed_steps=_step_keys(context.get_step_failure_events()),
            asset_keys=_asset_selection_keys(context.dagster_run.asset_selection),
            error_message=_failure_message(context),
            run_url=slack.run_url(context.dagster_run.run_id),
        )
    )
    try:
        slack.get_client().chat_postMessage(
            channel=slack.channel(),
            text=message.text,
            blocks=message.blocks,
        )
    except Exception:
        context.log.exception("Failed to send Dagster run failure alert to Slack")


def _format_sequence(values: Sequence[str]) -> str:
    formatted = [value for value in values if value]
    return ", ".join(formatted) if formatted else "-"


def _step_keys(events: Sequence[dg.DagsterEvent]) -> list[str]:
    return [event.step_key for event in events if event.step_key is not None]


def _asset_selection_keys(asset_selection: Any) -> list[str]:
    if not asset_selection:
        return []
    return sorted(asset_key.to_user_string() for asset_key in asset_selection)


def _failure_message(context: dg.RunFailureSensorContext) -> str | None:
    for event in context.get_step_failure_events():
        event_data = event.event_specific_data
        error = getattr(event_data, "error", None)
        if error is not None:
            return error.to_string()
        if event.message:
            return event.message
    return context.failure_event.message
