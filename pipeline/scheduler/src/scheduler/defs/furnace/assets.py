from collections.abc import Mapping
from typing import Any

import dagster as dg
from pydantic import Field

from scheduler.defs.asset_contracts import DEFAULT_OWNER
from scheduler.defs.resources.furnace import (
    FurnaceBollCliRequest,
    FurnaceCliResource,
    FurnaceKdjCliRequest,
    FurnaceMaCliRequest,
    FurnaceRsiCliRequest,
)

FURNACE_KDJ_ASSET_KEY = dg.AssetKey(["fleur_calculation", "calc_stock_kdj_daily"])
FURNACE_KDJ_UPSTREAM_ASSET_KEY = dg.AssetKey(["int_stock_quotes_daily_adj"])
FURNACE_MA_ASSET_KEY = dg.AssetKey(["fleur_calculation", "calc_stock_ma_daily"])
FURNACE_MA_UPSTREAM_ASSET_KEY = dg.AssetKey(["int_stock_quotes_daily_adj"])
FURNACE_MA_VOLUME_UPSTREAM_ASSET_KEY = dg.AssetKey(["int_stock_quotes_daily_unadj"])
FURNACE_RSI_ASSET_KEY = dg.AssetKey(["fleur_calculation", "calc_stock_rsi_daily"])
FURNACE_RSI_UPSTREAM_ASSET_KEY = dg.AssetKey(["int_stock_quotes_daily_adj"])
FURNACE_BOLL_ASSET_KEY = dg.AssetKey(["fleur_calculation", "calc_stock_boll_daily"])
FURNACE_BOLL_UPSTREAM_ASSET_KEY = dg.AssetKey(["int_stock_quotes_daily_adj"])
FURNACE_KDJ_GROUP = "calculation"
FURNACE_MA_GROUP = "calculation"
FURNACE_RSI_GROUP = "calculation"
FURNACE_BOLL_GROUP = "calculation"


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


class FurnaceMaRunConfig(dg.Config):
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: list[str] = Field(default_factory=list)
    input_table: str = "fleur_intermediate.int_stock_quotes_daily_adj"
    volume_input_table: str = "fleur_intermediate.int_stock_quotes_daily_unadj"
    output_table: str = "fleur_calculation.calc_stock_ma_daily"
    price_column: str = "close_price_forward_adj"
    volume_column: str = "volume"
    insert_batch_size: int = 10_000

    def to_cli_request(self, *, run_id: str) -> FurnaceMaCliRequest:
        if self.mode not in {"dry-run", "append-latest", "replace-cascade"}:
            msg = f"Unsupported Furnace MA mode: {self.mode}"
            raise ValueError(msg)
        return FurnaceMaCliRequest(
            request_from=self.request_from,
            request_to=self.request_to,
            mode=self.mode,
            symbols=tuple(self.symbols),
            input_table=self.input_table,
            volume_input_table=self.volume_input_table,
            output_table=self.output_table,
            price_column=self.price_column,
            volume_column=self.volume_column,
            insert_batch_size=self.insert_batch_size,
            run_id=run_id,
        )


class FurnaceRsiRunConfig(dg.Config):
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: list[str] = Field(default_factory=list)
    input_table: str = "fleur_intermediate.int_stock_quotes_daily_adj"
    output_table: str = "fleur_calculation.calc_stock_rsi_daily"
    price_column: str = "close_price_forward_adj"
    insert_batch_size: int = 10_000

    def to_cli_request(self, *, run_id: str) -> FurnaceRsiCliRequest:
        if self.mode not in {"dry-run", "append-latest", "replace-cascade"}:
            msg = f"Unsupported Furnace RSI mode: {self.mode}"
            raise ValueError(msg)
        return FurnaceRsiCliRequest(
            request_from=self.request_from,
            request_to=self.request_to,
            mode=self.mode,
            symbols=tuple(self.symbols),
            input_table=self.input_table,
            output_table=self.output_table,
            price_column=self.price_column,
            insert_batch_size=self.insert_batch_size,
            run_id=run_id,
        )


class FurnaceBollRunConfig(dg.Config):
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: list[str] = Field(default_factory=list)
    input_table: str = "fleur_intermediate.int_stock_quotes_daily_adj"
    output_table: str = "fleur_calculation.calc_stock_boll_daily"
    price_column: str = "close_price_forward_adj"
    insert_batch_size: int = 10_000

    def to_cli_request(self, *, run_id: str) -> FurnaceBollCliRequest:
        if self.mode not in {"dry-run", "append-latest", "replace-cascade"}:
            msg = f"Unsupported Furnace BOLL mode: {self.mode}"
            raise ValueError(msg)
        return FurnaceBollCliRequest(
            request_from=self.request_from,
            request_to=self.request_to,
            mode=self.mode,
            symbols=tuple(self.symbols),
            input_table=self.input_table,
            output_table=self.output_table,
            price_column=self.price_column,
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


def build_furnace_ma_asset() -> dg.AssetsDefinition:
    def furnace__calc_stock_ma_daily(
        context: dg.AssetExecutionContext,
        config: FurnaceMaRunConfig,
        furnace_cli: FurnaceCliResource,
    ) -> dg.MaterializeResult:
        result = furnace_cli.run_ma(config.to_cli_request(run_id=context.run_id))
        return dg.MaterializeResult(metadata=_metadata_from_summary(result.summary))

    return dg.asset(
        key=FURNACE_MA_ASSET_KEY,
        deps=[FURNACE_MA_UPSTREAM_ASSET_KEY, FURNACE_MA_VOLUME_UPSTREAM_ASSET_KEY],
        group_name=FURNACE_MA_GROUP,
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
            "table": "calc_stock_ma_daily",
            "indicator": "ma",
            "price_adjustment": "forward",
            "price_column": "close_price_forward_adj",
            "volume_input_table": "fleur_intermediate.int_stock_quotes_daily_unadj",
            "volume_column": "volume",
        },
    )(furnace__calc_stock_ma_daily)


def build_furnace_rsi_asset() -> dg.AssetsDefinition:
    def furnace__calc_stock_rsi_daily(
        context: dg.AssetExecutionContext,
        config: FurnaceRsiRunConfig,
        furnace_cli: FurnaceCliResource,
    ) -> dg.MaterializeResult:
        result = furnace_cli.run_rsi(config.to_cli_request(run_id=context.run_id))
        return dg.MaterializeResult(metadata=_metadata_from_summary(result.summary))

    return dg.asset(
        key=FURNACE_RSI_ASSET_KEY,
        deps=[FURNACE_RSI_UPSTREAM_ASSET_KEY],
        group_name=FURNACE_RSI_GROUP,
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
            "table": "calc_stock_rsi_daily",
            "indicator": "rsi",
            "price_adjustment": "forward",
            "price_column": "close_price_forward_adj",
        },
    )(furnace__calc_stock_rsi_daily)


def build_furnace_boll_asset() -> dg.AssetsDefinition:
    def furnace__calc_stock_boll_daily(
        context: dg.AssetExecutionContext,
        config: FurnaceBollRunConfig,
        furnace_cli: FurnaceCliResource,
    ) -> dg.MaterializeResult:
        result = furnace_cli.run_boll(config.to_cli_request(run_id=context.run_id))
        return dg.MaterializeResult(metadata=_metadata_from_summary(result.summary))

    return dg.asset(
        key=FURNACE_BOLL_ASSET_KEY,
        deps=[FURNACE_BOLL_UPSTREAM_ASSET_KEY],
        group_name=FURNACE_BOLL_GROUP,
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
            "table": "calc_stock_boll_daily",
            "indicator": "boll",
            "price_adjustment": "forward",
            "price_column": "close_price_forward_adj",
        },
    )(furnace__calc_stock_boll_daily)


def _metadata_from_summary(summary: Mapping[str, Any]) -> Mapping[str, Any]:
    return {
        "indicator": summary.get("indicator"),
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
        "valid_close_rows": summary.get("valid_close_rows", 0),
        "valid_volume_rows": summary.get("valid_volume_rows", 0),
        "input_valid_close_rows": summary.get("input_valid_close_rows", 0),
        "output_valid_close_rows": summary.get("output_valid_close_rows", 0),
        "null_indicator_rows": summary.get("null_indicator_rows", 0),
        "affected_years": summary.get("affected_years", []),
        "retained_rows": summary.get("retained_rows", 0),
        "kdj_params": summary.get("kdj_params", {}),
        "price_ma_windows": summary.get("price_ma_windows", []),
        "volume_ma_windows": summary.get("volume_ma_windows", []),
        "ema_window": summary.get("ema_window"),
        "ema_state_source": summary.get("ema_state_source"),
        "rsi_windows": summary.get("rsi_windows", []),
        "rsi_state_source": summary.get("rsi_state_source"),
        "gap_symbols_count": summary.get("gap_symbols_count", 0),
        "gap_fill_from": summary.get("gap_fill_from"),
        "boll_configs": summary.get("boll_configs", []),
        "max_window": summary.get("max_window"),
        "stddev_ddof": summary.get("stddev_ddof"),
        "state_source": summary.get("state_source"),
        "staging_validation": summary.get("staging_validation", {}),
        "partition_replace": summary.get("partition_replace", {}),
        "performance_metrics": summary.get("performance_metrics", {}),
        "furnace_exit_code": 0,
        "writes_applied": summary.get("writes_applied", False),
    }


FURNACE_KDJ_ASSETS: tuple[dg.AssetsDefinition, ...] = (build_furnace_kdj_asset(),)
FURNACE_MA_ASSETS: tuple[dg.AssetsDefinition, ...] = (build_furnace_ma_asset(),)
FURNACE_RSI_ASSETS: tuple[dg.AssetsDefinition, ...] = (build_furnace_rsi_asset(),)
FURNACE_BOLL_ASSETS: tuple[dg.AssetsDefinition, ...] = (build_furnace_boll_asset(),)
FURNACE_ASSETS: tuple[dg.AssetsDefinition, ...] = (
    FURNACE_KDJ_ASSETS + FURNACE_MA_ASSETS + FURNACE_RSI_ASSETS + FURNACE_BOLL_ASSETS
)
