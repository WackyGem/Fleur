from typing import Any

import dagster as dg

from scheduler.defs.asset_contracts import DEFAULT_OWNER
from scheduler.defs.rearview.resources import RearviewApiResource
from scheduler.version import scheduler_version

STRATEGY_PORTFOLIO_DAILY_PARTITIONS = dg.DailyPartitionsDefinition(start_date="2026-06-24")
STRATEGY_PORTFOLIO_DAILY_ASSET_KEY = dg.AssetKey(["rearview", "strategy_portfolio_daily_runs"])


class StrategyPortfolioDailyRunConfig(dg.Config):
    trade_date: str = ""


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
    """Create Rearview daily runs for active strategy portfolios."""

    trade_date = config.trade_date.strip() or context.partition_key
    response = rearview_api.create_strategy_portfolio_daily_runs(
        trade_date=trade_date,
        client_request_id=f"dagster-{context.run_id}-{trade_date}",
    )
    return dg.MaterializeResult(
        metadata=_daily_run_metadata(trade_date=trade_date, response=response)
    )


def _daily_run_metadata(
    *,
    trade_date: str,
    response: dict[str, Any],
) -> dict[str, dg.MetadataValue | int | str]:
    daily_run_ids = response.get("daily_run_ids", [])
    return {
        "scheduler_version": scheduler_version(),
        "trade_date": trade_date,
        "active_portfolio_count": int(response.get("active_portfolio_count", 0)),
        "created_run_count": int(response.get("created_run_count", 0)),
        "skipped_run_count": int(response.get("skipped_run_count", 0)),
        "daily_run_ids": dg.MetadataValue.json(daily_run_ids),
        "rearview_response": dg.MetadataValue.json(response),
    }


REARVIEW_ASSETS: tuple[dg.AssetsDefinition, ...] = (strategy_portfolio_daily_runs,)
