import time
from dataclasses import dataclass
from datetime import date, timedelta
from typing import Any

import dagster as dg

from scheduler.defs.asset_contracts import DEFAULT_OWNER
from scheduler.defs.rearview.resources import RearviewApiResource
from scheduler.version import scheduler_version

STRATEGY_PORTFOLIO_DAILY_PARTITIONS = dg.DailyPartitionsDefinition(start_date="2026-06-24")
STRATEGY_PORTFOLIO_DAILY_ASSET_KEY = dg.AssetKey(["rearview", "strategy_portfolio_daily_runs"])


class StrategyPortfolioDailyRunConfig(dg.Config):
    trade_date: str = ""
    start_date: str = ""
    end_date: str = ""
    wait_for_completion: bool = True
    poll_interval_seconds: int = 10
    timeout_seconds: int = 1800
    chunk_size: int = 20


@dataclass(frozen=True)
class DailyRunRangeRequest:
    start_date: str
    end_date: str
    settlement_target: dict[str, Any] | None


@dg.asset(
    key=STRATEGY_PORTFOLIO_DAILY_ASSET_KEY,
    group_name="rearview",
    partitions_def=STRATEGY_PORTFOLIO_DAILY_PARTITIONS,
    owners=[DEFAULT_OWNER],
    tags={
        "source": "rearview",
        "layer": "control_plane",
        "storage": "postgres_clickhouse",
        "state": "async_worker",
        "modality": "strategy_portfolio",
    },
)
def strategy_portfolio_daily_runs(
    context: dg.AssetExecutionContext,
    config: StrategyPortfolioDailyRunConfig,
    rearview_api: RearviewApiResource,
) -> dg.MaterializeResult:
    """Create Rearview daily runs and optionally wait for worker settlement."""

    request = _daily_run_range_request(
        context=context,
        config=config,
        rearview_api=rearview_api,
    )
    if request is None:
        settlement_target = rearview_api.get_strategy_portfolio_settlement_target()
        return dg.MaterializeResult(
            metadata=_daily_run_skip_metadata(
                partition_key=context.partition_key,
                reason="settlement_target_unavailable",
                settlement_target=settlement_target,
            )
        )

    chunk_size = _positive_int(config.chunk_size, "chunk_size")
    responses = []
    for chunk_start, chunk_end in _date_chunks(request.start_date, request.end_date, chunk_size):
        responses.append(
            rearview_api.create_strategy_portfolio_daily_runs_range(
                start_date=chunk_start,
                end_date=chunk_end,
                client_request_id=f"dagster-{context.run_id}-{chunk_start}-{chunk_end}",
                max_trade_dates=chunk_size,
            )
        )

    response = _combine_daily_run_range_responses(
        start_date=request.start_date,
        end_date=request.end_date,
        responses=responses,
    )
    statuses: dict[str, dict[str, Any]] = {}
    fact_counts: dict[str, dict[str, Any]] = {}
    if config.wait_for_completion and response["daily_run_ids"]:
        statuses = _wait_for_daily_runs(
            rearview_api=rearview_api,
            daily_run_ids=response["daily_run_ids"],
            poll_interval_seconds=_positive_int(
                config.poll_interval_seconds, "poll_interval_seconds"
            ),
            timeout_seconds=_positive_int(config.timeout_seconds, "timeout_seconds"),
        )
        fact_counts = _query_fact_counts_for_succeeded_runs(
            rearview_api=rearview_api,
            statuses=statuses,
        )

    return dg.MaterializeResult(
        metadata=_daily_run_metadata(
            partition_key=context.partition_key,
            requested_start_date=request.start_date,
            requested_end_date=request.end_date,
            settlement_target=request.settlement_target,
            response=response,
            statuses=statuses,
            fact_counts=fact_counts,
            wait_for_completion=config.wait_for_completion,
        )
    )


def _daily_run_range_request(
    *,
    context: dg.AssetExecutionContext,
    config: StrategyPortfolioDailyRunConfig,
    rearview_api: RearviewApiResource,
) -> DailyRunRangeRequest | None:
    start_date = config.start_date.strip()
    end_date = config.end_date.strip()
    trade_date = config.trade_date.strip()
    if start_date or end_date:
        if not start_date or not end_date:
            msg = "start_date and end_date must be provided together"
            raise ValueError(msg)
        _validate_date_order(start_date, end_date)
        return DailyRunRangeRequest(
            start_date=start_date,
            end_date=end_date,
            settlement_target=None,
        )
    if trade_date:
        return DailyRunRangeRequest(
            start_date=trade_date,
            end_date=trade_date,
            settlement_target=None,
        )

    settlement_target = rearview_api.get_strategy_portfolio_settlement_target()
    target_date = str(settlement_target.get("settlement_target_date") or "").strip()
    if target_date == "":
        return None
    return DailyRunRangeRequest(
        start_date=target_date,
        end_date=target_date,
        settlement_target=settlement_target,
    )


def _combine_daily_run_range_responses(
    *,
    start_date: str,
    end_date: str,
    responses: list[dict[str, Any]],
) -> dict[str, Any]:
    combined: dict[str, Any] = {
        "start_date": start_date,
        "end_date": end_date,
        "resolved_trade_dates": [],
        "active_portfolio_count": 0,
        "created_run_count": 0,
        "skipped_run_count": 0,
        "daily_run_ids": [],
        "created_daily_run_ids": [],
        "skipped_daily_run_ids": [],
        "trade_date_results": [],
    }
    for response in responses:
        combined["resolved_trade_dates"].extend(response.get("resolved_trade_dates", []))
        combined["active_portfolio_count"] = max(
            int(combined["active_portfolio_count"]),
            int(response.get("active_portfolio_count", 0)),
        )
        combined["created_run_count"] += int(response.get("created_run_count", 0))
        combined["skipped_run_count"] += int(response.get("skipped_run_count", 0))
        combined["daily_run_ids"].extend(response.get("daily_run_ids", []))
        combined["created_daily_run_ids"].extend(response.get("created_daily_run_ids", []))
        combined["skipped_daily_run_ids"].extend(response.get("skipped_daily_run_ids", []))
        combined["trade_date_results"].extend(response.get("trade_date_results", []))
    return combined


def _wait_for_daily_runs(
    *,
    rearview_api: RearviewApiResource,
    daily_run_ids: list[str],
    poll_interval_seconds: int,
    timeout_seconds: int,
) -> dict[str, dict[str, Any]]:
    pending = set(daily_run_ids)
    statuses: dict[str, dict[str, Any]] = {}
    deadline = time.monotonic() + timeout_seconds
    while pending:
        for daily_run_id in list(pending):
            status = rearview_api.get_strategy_portfolio_daily_run_status(daily_run_id)
            statuses[daily_run_id] = status
            run_status = str(status.get("status", ""))
            if run_status == "succeeded":
                pending.remove(daily_run_id)
            elif run_status == "cancelled" or run_status.startswith("failed_"):
                msg = (
                    "strategy portfolio daily run failed: "
                    f"{daily_run_id} status={run_status} "
                    f"error_type={status.get('error_type')} "
                    f"error_message={status.get('error_message')}"
                )
                raise RuntimeError(msg)
        if not pending:
            break
        if time.monotonic() >= deadline:
            msg = (
                f"strategy portfolio daily runs timed out before terminal status: {sorted(pending)}"
            )
            raise TimeoutError(msg)
        time.sleep(poll_interval_seconds)
    return statuses


def _query_fact_counts_for_succeeded_runs(
    *,
    rearview_api: RearviewApiResource,
    statuses: dict[str, dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    fact_counts = {}
    for daily_run_id, status in statuses.items():
        if status.get("status") != "succeeded":
            continue
        counts = rearview_api.get_strategy_portfolio_daily_run_fact_counts(daily_run_id)
        if int(counts.get("nav_row_count", 0)) <= 0:
            msg = f"strategy portfolio daily run wrote no live nav rows: {daily_run_id}"
            raise RuntimeError(msg)
        fact_counts[daily_run_id] = counts
    return fact_counts


def _daily_run_metadata(
    *,
    partition_key: str,
    requested_start_date: str,
    requested_end_date: str,
    settlement_target: dict[str, Any] | None,
    response: dict[str, Any],
    statuses: dict[str, dict[str, Any]],
    fact_counts: dict[str, dict[str, Any]],
    wait_for_completion: bool,
) -> dict[str, Any]:
    daily_run_ids = response.get("daily_run_ids", [])
    status_values = [str(status.get("status", "")) for status in statuses.values()]
    succeeded_run_count = sum(1 for status in status_values if status == "succeeded")
    failed_run_count = sum(
        1 for status in status_values if status == "cancelled" or status.startswith("failed_")
    )
    timeout_run_count = 0
    latest_status = _latest_status_by_trade_date(statuses)
    latest_daily_run_id = str(latest_status.get("strategy_portfolio_daily_run_id", "")).strip()
    latest_result_attempt_id = str(latest_status.get("current_result_attempt_id", "")).strip()
    latest_counts = fact_counts.get(latest_daily_run_id, {})
    return {
        "scheduler_version": scheduler_version(),
        "partition_key": partition_key,
        "requested_start_date": requested_start_date,
        "requested_end_date": requested_end_date,
        "settlement_target_date": (
            settlement_target.get("settlement_target_date") if settlement_target else None
        ),
        "resolved_trade_dates": dg.MetadataValue.json(response.get("resolved_trade_dates", [])),
        "active_portfolio_count": int(response.get("active_portfolio_count", 0)),
        "created_run_count": int(response.get("created_run_count", 0)),
        "skipped_run_count": int(response.get("skipped_run_count", 0)),
        "succeeded_run_count": succeeded_run_count,
        "failed_run_count": failed_run_count,
        "timeout_run_count": timeout_run_count,
        "wait_for_completion": wait_for_completion,
        "latest_daily_run_id": latest_daily_run_id,
        "latest_result_attempt_id": latest_result_attempt_id,
        "nav_row_count": int(latest_counts.get("nav_row_count", 0)),
        "trade_row_count": int(latest_counts.get("trade_row_count", 0)),
        "closed_trade_row_count": int(latest_counts.get("closed_trade_row_count", 0)),
        "daily_run_ids": dg.MetadataValue.json(daily_run_ids),
        "created_daily_run_ids": dg.MetadataValue.json(response.get("created_daily_run_ids", [])),
        "skipped_daily_run_ids": dg.MetadataValue.json(response.get("skipped_daily_run_ids", [])),
        "daily_run_statuses": dg.MetadataValue.json(statuses),
        "daily_run_fact_counts": dg.MetadataValue.json(fact_counts),
        "settlement_target": dg.MetadataValue.json(settlement_target or {}),
        "rearview_response": dg.MetadataValue.json(response),
    }


def _daily_run_skip_metadata(
    *,
    partition_key: str,
    reason: str,
    settlement_target: dict[str, Any],
) -> dict[str, Any]:
    return {
        "scheduler_version": scheduler_version(),
        "partition_key": partition_key,
        "skip_reason": reason,
        "settlement_target_date": settlement_target.get("settlement_target_date"),
        "settlement_target": dg.MetadataValue.json(settlement_target),
        "resolved_trade_dates": dg.MetadataValue.json([]),
        "created_run_count": 0,
        "skipped_run_count": 0,
        "succeeded_run_count": 0,
        "failed_run_count": 0,
        "timeout_run_count": 0,
        "daily_run_ids": dg.MetadataValue.json([]),
    }


def _latest_status_by_trade_date(statuses: dict[str, dict[str, Any]]) -> dict[str, Any]:
    if not statuses:
        return {}
    return max(statuses.values(), key=lambda status: str(status.get("trade_date", "")))


def _date_chunks(start_date: str, end_date: str, chunk_size: int) -> list[tuple[str, str]]:
    _validate_date_order(start_date, end_date)
    size = _positive_int(chunk_size, "chunk_size")
    current = date.fromisoformat(start_date)
    final = date.fromisoformat(end_date)
    chunks = []
    while current <= final:
        chunk_end = min(current + timedelta(days=size - 1), final)
        chunks.append((current.isoformat(), chunk_end.isoformat()))
        current = chunk_end + timedelta(days=1)
    return chunks


def _validate_date_order(start_date: str, end_date: str) -> None:
    if date.fromisoformat(start_date) > date.fromisoformat(end_date):
        msg = "start_date must be earlier than or equal to end_date"
        raise ValueError(msg)


def _positive_int(value: int, name: str) -> int:
    if value <= 0:
        msg = f"{name} must be positive"
        raise ValueError(msg)
    return value


REARVIEW_ASSETS: tuple[dg.AssetsDefinition, ...] = (strategy_portfolio_daily_runs,)
