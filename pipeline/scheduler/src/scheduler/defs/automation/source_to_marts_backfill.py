from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from datetime import date
from typing import Literal

import dagster as dg
from pydantic import Field

from scheduler.defs.automation.source_raw_backfill import (
    ALL_FETCH_SOURCES_TO_RAW_SCOPE,
    ALL_RAW_YEARLY_SCOPE,
    BAOSTOCK_DAILY_KLINE_SCOPE,
    CHINABOND_SCOPE,
    DEFAULT_JIUYAN_OCR_LIMIT,
    EASTMONEY_F10_SCOPE,
    MARKET_EVENTS_SCOPE,
    SNAPSHOT_REFERENCE_DATA_SCOPE,
    BackfillControllerRequest,
    BackfillPartitionSelection,
    BackfillRunSubmitter,
    BackfillStep,
    BackfillStepResult,
    InfoLogger,
    InProcessDagsterRunSubmitter,
    RunTagWriter,
    build_backfill_plan,
    current_shanghai_date,
    op_name_for_asset_key,
)
from scheduler.defs.automation.source_raw_backfill import (
    EXECUTION_MODE_FULL as SOURCE_RAW_EXECUTION_MODE_FULL,
)
from scheduler.defs.furnace.assets import FURNACE_ASSETS

BACKFILL_KIND = "fetch_history_sources_to_marts"
BACKFILL_JOB_NAME = "backfill__fetch_history_sources_to_marts_job"
BACKFILL_CONTROLLER_OP_NAME = "backfill__fetch_history_sources_to_marts_controller"

ALL_SOURCE_TO_MARTS_SCOPE = "all_source_to_marts"

EXECUTION_MODE_SOURCE_RAW_ONLY = "source_raw_only"
EXECUTION_MODE_DOWNSTREAM_ONLY = "downstream_only"

STAGE_SOURCE_RAW = "source_raw"
STAGE_DBT_STAGING = "dbt_staging"
STAGE_DBT_INTERMEDIATE = "dbt_intermediate"
STAGE_FURNACE_CALCULATION = "furnace_calculation"
STAGE_DBT_CALCULATION_WRAPPERS = "dbt_calculation_wrappers"
STAGE_DBT_MARTS = "dbt_marts"

FURNACE_MODE_DRY_RUN = "dry-run"
FURNACE_MODE_REPLACE_CASCADE = "replace-cascade"

TERMINAL_SUCCESS_STATUSES = {"SUCCESS"}
TERMINAL_FAILURE_STATUSES = {"FAILURE", "CANCELED"}

SOURCE_TO_MARTS_TARGET_SCOPES = (
    BAOSTOCK_DAILY_KLINE_SCOPE,
    MARKET_EVENTS_SCOPE,
    EASTMONEY_F10_SCOPE,
    CHINABOND_SCOPE,
    SNAPSHOT_REFERENCE_DATA_SCOPE,
    ALL_RAW_YEARLY_SCOPE,
    ALL_SOURCE_TO_MARTS_SCOPE,
)

SourceToMartsTargetScope = Literal[
    "baostock_daily_kline",
    "market_events",
    "eastmoney_f10",
    "chinabond",
    "snapshot_reference_data",
    "all_raw_yearly",
    "all_source_to_marts",
]
SourceToMartsExecutionMode = Literal["full", "source_raw_only", "downstream_only"]


@dataclass(frozen=True)
class SourceToMartsControllerRequest:
    target_scope: str
    start_date: str
    end_date: str
    execution_mode: str
    refresh_prerequisite_snapshots: bool
    overwrite_source_partitions: bool
    dry_run: bool


class SourceToMartsBackfillConfig(dg.Config):
    target_scope: SourceToMartsTargetScope = Field(
        ...,
        description=(
            "History source-to-marts backfill scope. Options: baostock_daily_kline, "
            "market_events, eastmoney_f10, chinabond, snapshot_reference_data, "
            "all_raw_yearly, all_source_to_marts."
        ),
    )
    start_date: str = Field(
        ...,
        description=(
            "Inclusive business date, formatted as YYYY-MM-DD. Ignored by snapshot_reference_data."
        ),
    )
    end_date: str = Field(
        ...,
        description=(
            "Inclusive business date, formatted as YYYY-MM-DD. Ignored by snapshot_reference_data."
        ),
    )
    execution_mode: SourceToMartsExecutionMode = Field(
        default=SOURCE_RAW_EXECUTION_MODE_FULL,
        description=(
            "full runs source/raw and downstream; source_raw_only runs only source/raw; "
            "downstream_only assumes source/raw already finished."
        ),
    )
    refresh_prerequisite_snapshots: bool = Field(
        default=False,
        description="Refresh only prerequisite source snapshots declared by the selected raw scope.",
    )
    overwrite_source_partitions: bool = Field(
        default=False,
        description="Forward overwrite behavior only to source assets that explicitly support it.",
    )
    dry_run: bool = Field(
        default=True,
        description="When true, log the expanded backfill plan without submitting child runs.",
    )

    def to_request(self) -> SourceToMartsControllerRequest:
        return SourceToMartsControllerRequest(
            target_scope=self.target_scope,
            start_date=self.start_date,
            end_date=self.end_date,
            execution_mode=self.execution_mode,
            refresh_prerequisite_snapshots=self.refresh_prerequisite_snapshots,
            overwrite_source_partitions=self.overwrite_source_partitions,
            dry_run=self.dry_run,
        )


@dataclass(frozen=True)
class SourceToMartsPlan:
    backfill_id: str
    target_scope: str
    execution_mode: str
    start_date: str | None
    end_date: str | None
    year_partitions: tuple[str, ...]
    steps: tuple[BackfillStep, ...]

    def controller_run_tags(self) -> dict[str, str]:
        return {
            "backfill.kind": BACKFILL_KIND,
            "backfill.id": self.backfill_id,
            "backfill.target_scope": self.target_scope,
            "backfill.start_date": self.start_date or "",
            "backfill.end_date": self.end_date or "",
        }

    def log_lines(self) -> tuple[str, ...]:
        header = (
            "Backfill source-to-marts plan: "
            f"backfill_id={self.backfill_id} "
            f"target_scope={self.target_scope} "
            f"start_date={self.start_date} "
            f"end_date={self.end_date} "
            f"execution_mode={self.execution_mode}"
        )
        lines = [header]
        for index, step in enumerate(self.steps, start=1):
            stage = step.tags.get("backfill.stage", "")
            lines.append(
                f"{index}. {stage}/{step.step_kind}: "
                f"{', '.join(step.asset_keys)} {step.partition.label()}"
            )
        return tuple(lines)

    def to_payload(self) -> dict[str, object]:
        return {
            "backfill_id": self.backfill_id,
            "target_scope": self.target_scope,
            "execution_mode": self.execution_mode,
            "start_date": self.start_date,
            "end_date": self.end_date,
            "year_partitions": list(self.year_partitions),
            "steps": [step.to_payload() for step in self.steps],
        }


@dataclass(frozen=True)
class DownstreamStageSpec:
    label: str
    stage: str
    asset_keys: tuple[str, ...]
    run_config: Mapping[str, object]


def build_source_to_marts_plan(
    config: SourceToMartsControllerRequest,
    *,
    today: date,
    controller_run_id: str,
) -> SourceToMartsPlan:
    execution_mode = _validated_execution_mode(config.execution_mode)
    start_date, end_date = _validated_date_range(config, today=today)
    backfill_id = _build_backfill_id(
        target_scope=config.target_scope,
        start_date=start_date,
        end_date=end_date,
        controller_run_id=controller_run_id,
    )
    common_tags = _common_tags(
        backfill_id=backfill_id,
        target_scope=config.target_scope,
        start_date=start_date,
        end_date=end_date,
        parent_run_id=controller_run_id,
    )

    steps: list[BackfillStep] = []
    if execution_mode in {SOURCE_RAW_EXECUTION_MODE_FULL, EXECUTION_MODE_SOURCE_RAW_ONLY}:
        steps.extend(
            _source_raw_steps(
                config,
                common_tags=common_tags,
                today=today,
                controller_run_id=controller_run_id,
            )
        )
    if execution_mode in {SOURCE_RAW_EXECUTION_MODE_FULL, EXECUTION_MODE_DOWNSTREAM_ONLY}:
        steps.extend(
            _downstream_steps(
                config,
                common_tags=common_tags,
                start_date=start_date,
                end_date=end_date,
            )
        )

    return SourceToMartsPlan(
        backfill_id=backfill_id,
        target_scope=config.target_scope,
        execution_mode=execution_mode,
        start_date=start_date.isoformat() if start_date is not None else None,
        end_date=end_date.isoformat() if end_date is not None else None,
        year_partitions=_year_partitions(start_date, end_date),
        steps=tuple(steps),
    )


def execute_source_to_marts_plan(
    plan: SourceToMartsPlan,
    *,
    submitter: BackfillRunSubmitter,
    log: InfoLogger,
) -> tuple[BackfillStepResult, ...]:
    results: list[BackfillStepResult] = []
    for step in plan.steps:
        log.info(
            "Submitting source-to-marts backfill step %s assets=%s partition=%s",
            step.label,
            step.asset_keys,
            step.partition.label(),
        )
        result = submitter.submit_step(step)
        results.append(result)
        if result.status not in TERMINAL_SUCCESS_STATUSES | TERMINAL_FAILURE_STATUSES:
            msg = (
                "Source-to-marts backfill step returned non-terminal status: "
                f"label={step.label!r} run_id={result.run_id} status={result.status}"
            )
            raise RuntimeError(msg)
        log.info(
            "Source-to-marts backfill step finished %s run_id=%s status=%s",
            step.label,
            result.run_id,
            result.status,
        )
        if not result.success:
            msg = (
                "Source-to-marts backfill step failed: "
                f"label={step.label!r} stage={step.tags.get('backfill.stage', '')!r} "
                f"assets={step.asset_keys} partition={step.partition.label()} "
                f"run_id={result.run_id} status={result.status}"
            )
            raise RuntimeError(msg)
    return tuple(results)


def add_source_to_marts_controller_run_tags(
    tag_writer: RunTagWriter,
    *,
    run_id: str,
    plan: SourceToMartsPlan,
) -> None:
    tag_writer.add_run_tags(run_id, plan.controller_run_tags())


def dbt_asset_keys_covered_by_all_source_to_marts_scope() -> set[str]:
    coverage = DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[ALL_SOURCE_TO_MARTS_SCOPE]
    return set().union(
        coverage[STAGE_DBT_STAGING],
        coverage[STAGE_DBT_INTERMEDIATE],
        coverage[STAGE_DBT_CALCULATION_WRAPPERS],
        coverage[STAGE_DBT_MARTS],
    )


def calculation_asset_keys_covered_by_all_source_to_marts_scope() -> set[str]:
    return set(
        DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[ALL_SOURCE_TO_MARTS_SCOPE][STAGE_FURNACE_CALCULATION]
    )


@dg.op(name=BACKFILL_CONTROLLER_OP_NAME)
def backfill__fetch_history_sources_to_marts_controller(
    context: dg.OpExecutionContext,
    config: SourceToMartsBackfillConfig,
) -> None:
    _execute_controller_request(context, config.to_request())


@dg.job(
    name=BACKFILL_JOB_NAME,
    description=("Manually plan and submit history source-to-marts backfill materialization runs."),
)
def backfill__fetch_history_sources_to_marts_job() -> None:
    backfill__fetch_history_sources_to_marts_controller()


def _execute_controller_request(
    context: dg.OpExecutionContext,
    config: SourceToMartsControllerRequest,
) -> None:
    plan = build_source_to_marts_plan(
        config,
        today=current_shanghai_date(),
        controller_run_id=context.run_id,
    )
    add_source_to_marts_controller_run_tags(context.instance, run_id=context.run_id, plan=plan)
    for line in plan.log_lines():
        context.log.info(line)
    context.log.info("Backfill source-to-marts plan payload: %s", plan.to_payload())
    if config.dry_run:
        return
    execute_source_to_marts_plan(
        plan,
        submitter=InProcessDagsterRunSubmitter(context),
        log=context.log,
    )


def _source_raw_steps(
    config: SourceToMartsControllerRequest,
    *,
    common_tags: Mapping[str, str],
    today: date,
    controller_run_id: str,
) -> tuple[BackfillStep, ...]:
    raw_plan = build_backfill_plan(
        BackfillControllerRequest(
            target_scope=_source_raw_scope_for_source_to_marts_scope(config.target_scope),
            start_date=config.start_date,
            end_date=config.end_date,
            execution_mode=SOURCE_RAW_EXECUTION_MODE_FULL,
            refresh_prerequisite_snapshots=config.refresh_prerequisite_snapshots,
            overwrite_source_partitions=config.overwrite_source_partitions,
            jiuyan_ocr_limit=DEFAULT_JIUYAN_OCR_LIMIT,
            jiuyan_force_download=False,
            jiuyan_force_ocr=False,
            dry_run=config.dry_run,
        ),
        today=today,
        controller_run_id=controller_run_id,
    )
    steps: list[BackfillStep] = []
    for step in raw_plan.steps:
        filtered_asset_keys = tuple(
            asset_key for asset_key in step.asset_keys if not _is_jiuyan_asset_key(asset_key)
        )
        if not filtered_asset_keys:
            continue
        tags = _step_tags(
            common_tags,
            stage=STAGE_SOURCE_RAW,
            step_kind=step.step_kind,
            source_step_tags=step.tags,
        )
        steps.append(
            BackfillStep(
                label=step.label,
                step_kind=step.step_kind,
                asset_keys=filtered_asset_keys,
                partition=step.partition,
                run_config=_filtered_source_raw_run_config(
                    step.run_config,
                    allowed_asset_keys=filtered_asset_keys,
                ),
                tags=tags,
            )
        )
    return tuple(steps)


def _downstream_steps(
    config: SourceToMartsControllerRequest,
    *,
    common_tags: Mapping[str, str],
    start_date: date | None,
    end_date: date | None,
) -> tuple[BackfillStep, ...]:
    specs = _downstream_stage_specs(
        target_scope=config.target_scope,
        start_date=start_date,
        end_date=end_date,
        dry_run=config.dry_run,
    )
    steps: list[BackfillStep] = []
    for spec in specs:
        if not spec.asset_keys:
            continue
        steps.append(
            BackfillStep(
                label=spec.label,
                step_kind=spec.stage,
                asset_keys=spec.asset_keys,
                partition=BackfillPartitionSelection(),
                run_config=spec.run_config,
                tags=_step_tags(common_tags, stage=spec.stage, step_kind=spec.stage),
            )
        )
    return tuple(steps)


def _downstream_stage_specs(
    *,
    target_scope: str,
    start_date: date | None,
    end_date: date | None,
    dry_run: bool,
) -> tuple[DownstreamStageSpec, ...]:
    if target_scope not in DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE:
        msg = f"Unsupported target_scope: {target_scope!r}"
        raise ValueError(msg)
    coverage = DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[target_scope]
    calculation_asset_keys = coverage[STAGE_FURNACE_CALCULATION]
    furnace_run_config: Mapping[str, object] = {}
    if calculation_asset_keys:
        furnace_run_config = _furnace_run_config(
            calculation_asset_keys=calculation_asset_keys,
            start_date=_required_date(start_date, field_name="start_date"),
            end_date=_required_date(end_date, field_name="end_date"),
            dry_run=dry_run,
        )
    return (
        DownstreamStageSpec(
            label="dbt staging",
            stage=STAGE_DBT_STAGING,
            asset_keys=coverage[STAGE_DBT_STAGING],
            run_config={},
        ),
        DownstreamStageSpec(
            label="dbt intermediate",
            stage=STAGE_DBT_INTERMEDIATE,
            asset_keys=coverage[STAGE_DBT_INTERMEDIATE],
            run_config={},
        ),
        DownstreamStageSpec(
            label="furnace calculation",
            stage=STAGE_FURNACE_CALCULATION,
            asset_keys=calculation_asset_keys,
            run_config=furnace_run_config,
        ),
        DownstreamStageSpec(
            label="dbt calculation wrappers",
            stage=STAGE_DBT_CALCULATION_WRAPPERS,
            asset_keys=coverage[STAGE_DBT_CALCULATION_WRAPPERS],
            run_config={},
        ),
        DownstreamStageSpec(
            label="dbt marts",
            stage=STAGE_DBT_MARTS,
            asset_keys=coverage[STAGE_DBT_MARTS],
            run_config={},
        ),
    )


def _furnace_run_config(
    *,
    calculation_asset_keys: Sequence[str],
    start_date: date,
    end_date: date,
    dry_run: bool,
) -> Mapping[str, object]:
    mode = FURNACE_MODE_DRY_RUN if dry_run else FURNACE_MODE_REPLACE_CASCADE
    return {
        "ops": {
            FURNACE_OP_NAME_BY_ASSET_KEY[asset_key]: {
                "config": {
                    "request_from": start_date.isoformat(),
                    "request_to": end_date.isoformat(),
                    "mode": mode,
                    "symbols": [],
                }
            }
            for asset_key in calculation_asset_keys
        }
    }


def _source_raw_scope_for_source_to_marts_scope(target_scope: str) -> str:
    if target_scope == ALL_SOURCE_TO_MARTS_SCOPE:
        return ALL_FETCH_SOURCES_TO_RAW_SCOPE
    if target_scope in SOURCE_TO_MARTS_TARGET_SCOPES:
        return target_scope
    msg = f"Unsupported target_scope: {target_scope!r}"
    raise ValueError(msg)


def _validated_execution_mode(execution_mode: str) -> str:
    if execution_mode in {
        SOURCE_RAW_EXECUTION_MODE_FULL,
        EXECUTION_MODE_SOURCE_RAW_ONLY,
        EXECUTION_MODE_DOWNSTREAM_ONLY,
    }:
        return execution_mode
    msg = f"Unsupported execution_mode: {execution_mode!r}"
    raise ValueError(msg)


def _validated_date_range(
    config: SourceToMartsControllerRequest,
    *,
    today: date,
) -> tuple[date | None, date | None]:
    if config.target_scope == SNAPSHOT_REFERENCE_DATA_SCOPE:
        return None, None
    start_date = _parse_date(config.start_date, field_name="start_date")
    end_date = _parse_date(config.end_date, field_name="end_date")
    if start_date > end_date:
        msg = "start_date cannot be later than end_date"
        raise ValueError(msg)
    if end_date > today:
        msg = f"end_date cannot be later than today's Asia/Shanghai date: {today.isoformat()}"
        raise ValueError(msg)
    return start_date, end_date


def _parse_date(value: str, *, field_name: str) -> date:
    try:
        return date.fromisoformat(value)
    except ValueError as error:
        msg = f"Invalid {field_name}: {value!r}"
        raise ValueError(msg) from error


def _build_backfill_id(
    *,
    target_scope: str,
    start_date: date | None,
    end_date: date | None,
    controller_run_id: str,
) -> str:
    start = start_date.isoformat() if start_date is not None else "snapshot"
    end = end_date.isoformat() if end_date is not None else "snapshot"
    short_run_id = controller_run_id.replace("-", "")[:12]
    return f"{target_scope}-{start}-{end}-{short_run_id}"


def _year_partitions(start_date: date | None, end_date: date | None) -> tuple[str, ...]:
    if start_date is None or end_date is None:
        return ()
    return tuple(str(year) for year in range(start_date.year, end_date.year + 1))


def _common_tags(
    *,
    backfill_id: str,
    target_scope: str,
    start_date: date | None,
    end_date: date | None,
    parent_run_id: str,
) -> dict[str, str]:
    return {
        "backfill.kind": BACKFILL_KIND,
        "backfill.id": backfill_id,
        "backfill.target_scope": target_scope,
        "backfill.start_date": start_date.isoformat() if start_date is not None else "",
        "backfill.end_date": end_date.isoformat() if end_date is not None else "",
        "backfill.parent_run_id": parent_run_id,
    }


def _step_tags(
    common_tags: Mapping[str, str],
    *,
    stage: str,
    step_kind: str,
    source_step_tags: Mapping[str, str] | None = None,
) -> dict[str, str]:
    tags = dict(common_tags)
    tags["backfill.stage"] = stage
    tags["backfill.step"] = step_kind
    if source_step_tags is not None and "backfill.year" in source_step_tags:
        tags["backfill.year"] = source_step_tags["backfill.year"]
    return tags


def _filtered_source_raw_run_config(
    run_config: Mapping[str, object],
    *,
    allowed_asset_keys: Iterable[str],
) -> Mapping[str, object]:
    if "ops" not in run_config:
        return {}
    ops = run_config["ops"]
    if not isinstance(ops, Mapping):
        msg = f"Expected source/raw run_config ops mapping, got {type(ops)}"
        raise TypeError(msg)
    allowed_op_names = {op_name_for_asset_key(asset_key) for asset_key in allowed_asset_keys}
    filtered_ops = {
        op_name: op_config
        for op_name, op_config in ops.items()
        if isinstance(op_name, str) and op_name in allowed_op_names
    }
    if not filtered_ops:
        return {}
    return {"ops": filtered_ops}


def _is_jiuyan_asset_key(asset_key: str) -> bool:
    return asset_key.startswith("source/jiuyan__") or asset_key.startswith(
        "clickhouse/raw/jiuyan__"
    )


def _required_date(value: date | None, *, field_name: str) -> date:
    if value is None:
        msg = f"{field_name} is required"
        raise ValueError(msg)
    return value


def _single_asset_key(asset: dg.AssetsDefinition) -> str:
    if len(asset.keys) != 1:
        msg = "source-to-marts backfill registry only supports single-asset Furnace definitions"
        raise ValueError(msg)
    return next(iter(asset.keys)).to_user_string()


def _dedupe(values: Iterable[str]) -> tuple[str, ...]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if value in seen:
            continue
        seen.add(value)
        result.append(value)
    return tuple(result)


def _union_stage(
    scopes: Iterable[str],
    *,
    stage: str,
) -> tuple[str, ...]:
    return _dedupe(
        asset_key
        for scope in scopes
        for asset_key in DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[scope][stage]
    )


BAOSTOCK_STAGING_ASSET_KEYS = (
    "stg_sina__trade_calendar",
    "stg_baostock__query_stock_basic",
    "stg_baostock__query_history_k_data_plus_daily",
)
THS_STAGING_ASSET_KEYS = ("stg_ths__limit_up_pool_compacted",)
EASTMONEY_STAGING_ASSET_KEYS = (
    "stg_eastmoney__balance",
    "stg_eastmoney__cashflow_sq",
    "stg_eastmoney__cashflow_ytd",
    "stg_eastmoney__dividend_allotment",
    "stg_eastmoney__dividend_main",
    "stg_eastmoney__equity_history",
    "stg_eastmoney__freeholders",
    "stg_eastmoney__income_sq",
    "stg_eastmoney__income_ytd",
)
CHINABOND_STAGING_ASSET_KEYS = ("stg_chinabond__government_bond",)
SNAPSHOT_REFERENCE_STAGING_ASSET_KEYS = (
    "stg_sina__trade_calendar",
    "stg_baostock__query_stock_basic",
)

BAOSTOCK_INTERMEDIATE_ASSET_KEYS = (
    "int_trade_calendar",
    "int_stock_basic_snapshot",
    "int_index_basic_snapshot",
    "int_index_quotes_daily",
    "int_benchmark_basic_snapshot",
    "int_benchmark_returns_daily",
    "int_stock_quotes_daily_unadj",
    "int_stock_adjustment_factor",
    "int_stock_quotes_daily_adj",
)
THS_INTERMEDIATE_ASSET_KEYS = ("int_stock_limit_up_pool_daily",)
EASTMONEY_INTERMEDIATE_ASSET_KEYS = (
    "int_stock_shares_history",
    "int_stock_exrights_event",
    "int_stock_quotes_daily_unadj",
    "int_stock_adjustment_factor",
    "int_stock_quotes_daily_adj",
    "int_stock_financial_valuation",
)
F10_PASSTHROUGH_INTERMEDIATE_ASSET_KEYS = (
    "int_stock_balance_sheet",
    "int_stock_cashflow_statement_quarterly",
    "int_stock_cashflow_statement_ytd",
    "int_stock_allotment_event",
    "int_stock_dividend_plan",
    "int_stock_share_capital_history",
    "int_stock_free_float_shareholder_top10",
    "int_stock_income_statement_quarterly",
    "int_stock_income_statement_ytd",
)
CHINABOND_INTERMEDIATE_ASSET_KEYS = (
    "int_government_bond_yields_daily",
    "int_risk_free_rate_daily",
)
SNAPSHOT_REFERENCE_INTERMEDIATE_ASSET_KEYS = (
    "int_trade_calendar",
    "int_stock_basic_snapshot",
    "int_index_basic_snapshot",
    "int_benchmark_basic_snapshot",
)

CALCULATION_WRAPPER_ASSET_KEYS = (
    "int_stock_kdj_daily",
    "int_stock_ma_daily",
    "int_stock_rsi_daily",
    "int_stock_boll_daily",
    "int_stock_macd_daily",
    "int_stock_price_pattern_daily",
)

BAOSTOCK_MART_ASSET_KEYS = (
    "mart_trade_calendar",
    "mart_stock_basic_snapshot",
    "mart_benchmark_returns_daily",
    "mart_stock_quotes_daily",
    "mart_stock_trend_indicator_daily",
    "mart_stock_momentum_indicator_daily",
    "mart_stock_volume_indicator_daily",
    "mart_stock_price_pattern_daily",
)
THS_MART_ASSET_KEYS = ("mart_stock_limit_up_pool_daily",)
EASTMONEY_MART_ASSET_KEYS = ("mart_stock_quotes_daily",)
F10_PASSTHROUGH_MART_ASSET_KEYS = (
    "mart_stock_balance_sheet",
    "mart_stock_cashflow_statement_quarterly",
    "mart_stock_cashflow_statement_ytd",
    "mart_stock_allotment_event",
    "mart_stock_dividend_plan",
    "mart_stock_share_capital_history",
    "mart_stock_free_float_shareholder_top10",
    "mart_stock_income_statement_quarterly",
    "mart_stock_income_statement_ytd",
)
CHINABOND_MART_ASSET_KEYS = (
    "mart_government_bond_yields_daily",
    "mart_risk_free_rate_daily",
)
SNAPSHOT_REFERENCE_MART_ASSET_KEYS = (
    "mart_trade_calendar",
    "mart_stock_basic_snapshot",
)

FURNACE_ASSET_KEYS = tuple(_single_asset_key(asset) for asset in FURNACE_ASSETS)
FURNACE_OP_NAME_BY_ASSET_KEY = {
    _single_asset_key(asset): asset.node_def.name for asset in FURNACE_ASSETS
}

DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE: dict[str, dict[str, tuple[str, ...]]] = {
    BAOSTOCK_DAILY_KLINE_SCOPE: {
        STAGE_DBT_STAGING: BAOSTOCK_STAGING_ASSET_KEYS,
        STAGE_DBT_INTERMEDIATE: BAOSTOCK_INTERMEDIATE_ASSET_KEYS,
        STAGE_FURNACE_CALCULATION: FURNACE_ASSET_KEYS,
        STAGE_DBT_CALCULATION_WRAPPERS: CALCULATION_WRAPPER_ASSET_KEYS,
        STAGE_DBT_MARTS: BAOSTOCK_MART_ASSET_KEYS,
    },
    MARKET_EVENTS_SCOPE: {
        STAGE_DBT_STAGING: THS_STAGING_ASSET_KEYS,
        STAGE_DBT_INTERMEDIATE: THS_INTERMEDIATE_ASSET_KEYS,
        STAGE_FURNACE_CALCULATION: (),
        STAGE_DBT_CALCULATION_WRAPPERS: (),
        STAGE_DBT_MARTS: THS_MART_ASSET_KEYS,
    },
    EASTMONEY_F10_SCOPE: {
        STAGE_DBT_STAGING: EASTMONEY_STAGING_ASSET_KEYS,
        STAGE_DBT_INTERMEDIATE: (
            EASTMONEY_INTERMEDIATE_ASSET_KEYS + F10_PASSTHROUGH_INTERMEDIATE_ASSET_KEYS
        ),
        STAGE_FURNACE_CALCULATION: (),
        STAGE_DBT_CALCULATION_WRAPPERS: (),
        STAGE_DBT_MARTS: EASTMONEY_MART_ASSET_KEYS + F10_PASSTHROUGH_MART_ASSET_KEYS,
    },
    CHINABOND_SCOPE: {
        STAGE_DBT_STAGING: CHINABOND_STAGING_ASSET_KEYS,
        STAGE_DBT_INTERMEDIATE: CHINABOND_INTERMEDIATE_ASSET_KEYS,
        STAGE_FURNACE_CALCULATION: (),
        STAGE_DBT_CALCULATION_WRAPPERS: (),
        STAGE_DBT_MARTS: CHINABOND_MART_ASSET_KEYS,
    },
    SNAPSHOT_REFERENCE_DATA_SCOPE: {
        STAGE_DBT_STAGING: SNAPSHOT_REFERENCE_STAGING_ASSET_KEYS,
        STAGE_DBT_INTERMEDIATE: SNAPSHOT_REFERENCE_INTERMEDIATE_ASSET_KEYS,
        STAGE_FURNACE_CALCULATION: (),
        STAGE_DBT_CALCULATION_WRAPPERS: (),
        STAGE_DBT_MARTS: SNAPSHOT_REFERENCE_MART_ASSET_KEYS,
    },
}

ALL_DOWNSTREAM_SCOPES = (
    SNAPSHOT_REFERENCE_DATA_SCOPE,
    BAOSTOCK_DAILY_KLINE_SCOPE,
    MARKET_EVENTS_SCOPE,
    EASTMONEY_F10_SCOPE,
    CHINABOND_SCOPE,
)
DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[ALL_RAW_YEARLY_SCOPE] = {
    stage: _union_stage(ALL_DOWNSTREAM_SCOPES, stage=stage)
    for stage in (
        STAGE_DBT_STAGING,
        STAGE_DBT_INTERMEDIATE,
        STAGE_FURNACE_CALCULATION,
        STAGE_DBT_CALCULATION_WRAPPERS,
        STAGE_DBT_MARTS,
    )
}
DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[ALL_SOURCE_TO_MARTS_SCOPE] = dict(
    DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[ALL_RAW_YEARLY_SCOPE]
)
