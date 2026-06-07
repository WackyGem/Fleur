from collections.abc import Mapping
from typing import Any

import dagster as dg
from pydantic import Field

from scheduler.defs.asset_contracts import DEFAULT_OWNER
from scheduler.defs.resources.furnace import FurnaceCliResource, FurnaceKdjCliRequest

FURNACE_KDJ_ASSET_KEY = dg.AssetKey(["fleur_calculation", "calc_stock_kdj_daily"])
FURNACE_KDJ_UPSTREAM_ASSET_KEY = dg.AssetKey(["int_stock_quotes_daily_adj"])
FURNACE_KDJ_GROUP = "calculation"


class FurnaceKdjRunConfig(dg.Config):
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: list[str] = Field(default_factory=list)
    rsv_window: int = 9
    k_smoothing: int = 3
    d_smoothing: int = 3
    insert_batch_size: int = 10_000

    def to_cli_request(self, *, run_id: str) -> FurnaceKdjCliRequest:
        if self.mode not in {"dry-run", "append-latest", "replace-cascade"}:
            msg = f"Unsupported Furnace KDJ mode: {self.mode}"
            raise ValueError(msg)
        return FurnaceKdjCliRequest(
            request_from=self.request_from,
            request_to=self.request_to,
            mode=self.mode,
            symbols=tuple(self.symbols),
            rsv_window=self.rsv_window,
            k_smoothing=self.k_smoothing,
            d_smoothing=self.d_smoothing,
            insert_batch_size=self.insert_batch_size,
            run_id=run_id,
        )


def build_furnace_kdj_asset() -> dg.AssetsDefinition:
    def furnace__calc_stock_kdj_daily(
        context: dg.AssetExecutionContext,
        config: FurnaceKdjRunConfig,
        furnace_cli: FurnaceCliResource,
    ) -> dg.MaterializeResult:
        result = furnace_cli.run_kdj(config.to_cli_request(run_id=context.run_id))
        return dg.MaterializeResult(metadata=_metadata_from_summary(result.summary))

    return dg.asset(
        key=FURNACE_KDJ_ASSET_KEY,
        deps=[FURNACE_KDJ_UPSTREAM_ASSET_KEY],
        group_name=FURNACE_KDJ_GROUP,
        owners=[DEFAULT_OWNER],
        kinds={"rust", "clickhouse"},
        tags={
            "owner": "furnace",
            "layer": "calculation",
            "storage": "clickhouse",
            "modality": "batch",
        },
        metadata={
            "database": "fleur_calculation",
            "table": "calc_stock_kdj_daily",
            "indicator": "kdj",
            "price_adjustment": "forward",
        },
    )(furnace__calc_stock_kdj_daily)


def _metadata_from_summary(summary: Mapping[str, Any]) -> Mapping[str, Any]:
    return {
        "request_range": {
            "from": summary.get("request_from"),
            "to": summary.get("request_to"),
        },
        "effective_output_range": {
            "from": summary.get("effective_output_from"),
            "to": summary.get("effective_output_to"),
        },
        "input_range": {
            "from": summary.get("input_from"),
            "to": summary.get("input_to"),
        },
        "mode": summary.get("mode"),
        "symbols_count": summary.get("symbols_count", 0),
        "input_rows": summary.get("input_rows", 0),
        "output_rows": summary.get("output_rows", 0),
        "null_indicator_rows": summary.get("null_indicator_rows", 0),
        "affected_years": summary.get("affected_years", []),
        "retained_rows": summary.get("retained_rows", 0),
        "kdj_params": summary.get("kdj_params", {}),
        "state_source": summary.get("state_source"),
        "staging_validation": summary.get("staging_validation", {}),
        "partition_replace": summary.get("partition_replace", {}),
        "furnace_exit_code": 0,
        "writes_applied": summary.get("writes_applied", False),
    }


FURNACE_KDJ_ASSETS: tuple[dg.AssetsDefinition, ...] = (build_furnace_kdj_asset(),)
