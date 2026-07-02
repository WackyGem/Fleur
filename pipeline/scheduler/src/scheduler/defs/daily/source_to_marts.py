from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from datetime import date

import dagster as dg
from pydantic import Field

from scheduler.defs.automation.source_raw_backfill import (
    EXECUTION_MODE_FULL as SOURCE_RAW_EXECUTION_MODE_FULL,
)
from scheduler.defs.automation.source_raw_backfill import (
    BackfillPartitionSelection,
    BackfillRunSubmitter,
    BackfillStep,
    BackfillStepResult,
    InfoLogger,
    InProcessDagsterRunSubmitter,
    RunTagWriter,
    current_shanghai_date,
)
from scheduler.defs.automation.source_to_marts_backfill import (
    ALL_SOURCE_TO_MARTS_SCOPE,
    FURNACE_MODE_DRY_RUN,
    FURNACE_MODE_REPLACE_CASCADE,
    SOURCE_TO_MARTS_TARGET_SCOPES,
    STAGE_FURNACE_CALCULATION,
    SourceToMartsControllerRequest,
    SourceToMartsExecutionMode,
    SourceToMartsTargetScope,
    build_source_to_marts_plan,
)
from scheduler.defs.rearview.assets import DAILY_PORTFOLIO_NAV_LIQUIDATION_ASSET_KEY

DAILY_KIND = "fetch_history_sources_to_marts_schedule"
DAILY_JOB_NAME = "daily__fetch_history_sources_to_marts_schedule_job"
DAILY_CONTROLLER_OP_NAME = "daily__fetch_history_sources_to_marts_schedule_controller"
FURNACE_MODE_APPEND_LATEST = "append-latest"
STAGE_PORTFOLIO_LIVE_LIQUIDATION = "portfolio_live_liquidation"
STEP_PORTFOLIO_LIVE_LIQUIDATION = "asset_materialization"
PORTFOLIO_LIVE_LIQUIDATION_ASSET_KEY = DAILY_PORTFOLIO_NAV_LIQUIDATION_ASSET_KEY.to_user_string()
TERMINAL_SUCCESS_STATUSES = {"SUCCESS"}
TERMINAL_FAILURE_STATUSES = {"FAILURE", "CANCELED"}


@dataclass(frozen=True)
class DailyFetchHistorySourcesToMartsRequest:
    target_scope: str
    target_date: str
    execution_mode: str
    refresh_prerequisite_snapshots: bool
    overwrite_source_partitions: bool
    dry_run: bool


class DailyFetchHistorySourcesToMartsConfig(dg.Config):
    target_scope: SourceToMartsTargetScope = Field(
        default=ALL_SOURCE_TO_MARTS_SCOPE,
        description=(
            "Daily source-to-marts scope. Options: baostock_daily_kline, market_events, "
            "eastmoney_f10, chinabond, snapshot_reference_data, all_raw_yearly, "
            "all_source_to_marts."
        ),
    )
    target_date: str = Field(
        ...,
        description=(
            "Business date for the daily incremental run, formatted as YYYY-MM-DD. "
            "Required for every scope; snapshot_reference_data passes it through but the "
            "source-to-marts planner ignores date windows for snapshot assets."
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
        description="Refresh prerequisite source snapshots declared by the selected scope.",
    )
    overwrite_source_partitions: bool = Field(
        default=False,
        description="Forward overwrite behavior only to source assets that explicitly support it.",
    )
    dry_run: bool = Field(
        default=True,
        description="When true, log the expanded daily plan without submitting child runs.",
    )

    def to_request(self) -> DailyFetchHistorySourcesToMartsRequest:
        return DailyFetchHistorySourcesToMartsRequest(
            target_scope=self.target_scope,
            target_date=self.target_date,
            execution_mode=self.execution_mode,
            refresh_prerequisite_snapshots=self.refresh_prerequisite_snapshots,
            overwrite_source_partitions=self.overwrite_source_partitions,
            dry_run=self.dry_run,
        )


@dataclass(frozen=True)
class DailyStep:
    label: str
    step_kind: str
    stage: str
    asset_keys: tuple[str, ...]
    partition: BackfillPartitionSelection
    run_config: Mapping[str, object]
    tags: Mapping[str, str]

    def to_backfill_step(self) -> BackfillStep:
        return BackfillStep(
            label=self.label,
            step_kind=self.step_kind,
            asset_keys=self.asset_keys,
            partition=self.partition,
            run_config=self.run_config,
            tags=self.tags,
        )

    def to_payload(self) -> dict[str, object]:
        return {
            "label": self.label,
            "step_kind": self.step_kind,
            "stage": self.stage,
            "asset_keys": list(self.asset_keys),
            "partition": self.partition.to_payload(),
            "run_config": dict(self.run_config),
            "tags": dict(self.tags),
        }


@dataclass(frozen=True)
class DailyPlan:
    daily_id: str
    target_scope: str
    execution_mode: str
    target_date: str
    year_partitions: tuple[str, ...]
    steps: tuple[DailyStep, ...]

    def controller_run_tags(self) -> dict[str, str]:
        return {
            "daily.kind": DAILY_KIND,
            "daily.id": self.daily_id,
            "daily.target_scope": self.target_scope,
            "daily.target_date": self.target_date,
            "daily.execution_mode": self.execution_mode,
        }

    def log_lines(self) -> tuple[str, ...]:
        header = (
            "Daily source-to-marts plan: "
            f"daily_id={self.daily_id} "
            f"target_scope={self.target_scope} "
            f"target_date={self.target_date} "
            f"execution_mode={self.execution_mode}"
        )
        lines = [header]
        for index, step in enumerate(self.steps, start=1):
            lines.append(
                f"{index}. {step.stage}/{step.step_kind}: "
                f"{', '.join(step.asset_keys)} {step.partition.label()}"
            )
        return tuple(lines)

    def to_payload(self) -> dict[str, object]:
        return {
            "daily_id": self.daily_id,
            "target_scope": self.target_scope,
            "execution_mode": self.execution_mode,
            "target_date": self.target_date,
            "year_partitions": list(self.year_partitions),
            "steps": [step.to_payload() for step in self.steps],
        }


def build_daily_fetch_history_sources_to_marts_plan(
    config: DailyFetchHistorySourcesToMartsRequest,
    *,
    today: date,
    controller_run_id: str,
) -> DailyPlan:
    target_date = _parse_target_date(config.target_date)
    source_plan = build_source_to_marts_plan(
        SourceToMartsControllerRequest(
            target_scope=config.target_scope,
            start_date=target_date.isoformat(),
            end_date=target_date.isoformat(),
            execution_mode=config.execution_mode,
            refresh_prerequisite_snapshots=config.refresh_prerequisite_snapshots,
            overwrite_source_partitions=config.overwrite_source_partitions,
            dry_run=config.dry_run,
        ),
        today=today,
        controller_run_id=controller_run_id,
    )
    daily_id = _build_daily_id(
        target_scope=config.target_scope,
        target_date=target_date,
        controller_run_id=controller_run_id,
    )
    common_tags = _common_tags(
        daily_id=daily_id,
        target_scope=config.target_scope,
        target_date=target_date,
        execution_mode=source_plan.execution_mode,
        parent_run_id=controller_run_id,
    )
    steps = tuple(
        _daily_step_from_source_to_marts_step(
            step,
            common_tags=common_tags,
            dry_run=config.dry_run,
        )
        for step in source_plan.steps
    )
    if _should_append_portfolio_live_liquidation_step(
        target_scope=config.target_scope,
        execution_mode=source_plan.execution_mode,
    ):
        steps = (
            *steps,
            _portfolio_live_liquidation_step(common_tags=common_tags),
        )

    return DailyPlan(
        daily_id=daily_id,
        target_scope=config.target_scope,
        execution_mode=source_plan.execution_mode,
        target_date=target_date.isoformat(),
        year_partitions=source_plan.year_partitions,
        steps=steps,
    )


def execute_daily_fetch_history_sources_to_marts_plan(
    plan: DailyPlan,
    *,
    submitter: BackfillRunSubmitter,
    log: InfoLogger,
) -> tuple[BackfillStepResult, ...]:
    results: list[BackfillStepResult] = []
    for step in plan.steps:
        log.info(
            "Submitting daily source-to-marts step %s assets=%s partition=%s",
            step.label,
            step.asset_keys,
            step.partition.label(),
        )
        result = submitter.submit_step(step.to_backfill_step())
        results.append(result)
        if result.status not in TERMINAL_SUCCESS_STATUSES | TERMINAL_FAILURE_STATUSES:
            msg = (
                "Daily source-to-marts step returned non-terminal status: "
                f"label={step.label!r} run_id={result.run_id} status={result.status}"
            )
            raise RuntimeError(msg)
        log.info(
            "Daily source-to-marts step finished %s run_id=%s status=%s",
            step.label,
            result.run_id,
            result.status,
        )
        if not result.success:
            msg = (
                "Daily source-to-marts step failed: "
                f"label={step.label!r} stage={step.stage!r} assets={step.asset_keys} "
                f"partition={step.partition.label()} run_id={result.run_id} "
                f"status={result.status}"
            )
            raise RuntimeError(msg)
    return tuple(results)


def add_daily_fetch_history_sources_to_marts_controller_run_tags(
    tag_writer: RunTagWriter,
    *,
    run_id: str,
    plan: DailyPlan,
) -> None:
    tag_writer.add_run_tags(run_id, plan.controller_run_tags())


def execute_daily_fetch_history_sources_to_marts_controller_request(
    config: DailyFetchHistorySourcesToMartsRequest,
    *,
    today: date,
    controller_run_id: str,
    tag_writer: RunTagWriter,
    log: InfoLogger,
    submitter: BackfillRunSubmitter,
) -> DailyPlan:
    plan = build_daily_fetch_history_sources_to_marts_plan(
        config,
        today=today,
        controller_run_id=controller_run_id,
    )
    add_daily_fetch_history_sources_to_marts_controller_run_tags(
        tag_writer,
        run_id=controller_run_id,
        plan=plan,
    )
    for line in plan.log_lines():
        log.info(line)
    log.info("Daily source-to-marts plan payload: %s", plan.to_payload())
    if config.dry_run:
        return plan
    execute_daily_fetch_history_sources_to_marts_plan(
        plan,
        submitter=submitter,
        log=log,
    )
    return plan


def source_to_marts_target_scopes() -> tuple[str, ...]:
    return SOURCE_TO_MARTS_TARGET_SCOPES


@dg.op(name=DAILY_CONTROLLER_OP_NAME)
def daily__fetch_history_sources_to_marts_schedule_controller(
    context: dg.OpExecutionContext,
    config: DailyFetchHistorySourcesToMartsConfig,
) -> None:
    execute_daily_fetch_history_sources_to_marts_controller_request(
        config.to_request(),
        today=current_shanghai_date(),
        controller_run_id=context.run_id,
        tag_writer=context.instance,
        log=context.log,
        submitter=InProcessDagsterRunSubmitter(context),
    )


@dg.job(
    name=DAILY_JOB_NAME,
    description=(
        "Daily incremental source-to-marts controller that expands a target_date into "
        "source/raw/dbt/Furnace/mart asset materialization steps, then runs portfolio "
        "live NAV liquidation as the terminal step for the full daily scope."
    ),
)
def daily__fetch_history_sources_to_marts_schedule_job() -> None:
    daily__fetch_history_sources_to_marts_schedule_controller()


def _daily_step_from_source_to_marts_step(
    step: BackfillStep,
    *,
    common_tags: Mapping[str, str],
    dry_run: bool,
) -> DailyStep:
    stage = _stage_for_source_to_marts_step(step)
    return DailyStep(
        label=step.label,
        step_kind=step.step_kind,
        stage=stage,
        asset_keys=step.asset_keys,
        partition=step.partition,
        run_config=_daily_run_config_for_step(step, stage=stage, dry_run=dry_run),
        tags=_step_tags(
            source_tags=step.tags,
            common_tags=common_tags,
            stage=stage,
            step_kind=step.step_kind,
        ),
    )


def _portfolio_live_liquidation_step(*, common_tags: Mapping[str, str]) -> DailyStep:
    return DailyStep(
        label="portfolio live nav liquidation",
        step_kind=STEP_PORTFOLIO_LIVE_LIQUIDATION,
        stage=STAGE_PORTFOLIO_LIVE_LIQUIDATION,
        asset_keys=(PORTFOLIO_LIVE_LIQUIDATION_ASSET_KEY,),
        partition=BackfillPartitionSelection(),
        run_config={},
        tags=_step_tags(
            source_tags={},
            common_tags=common_tags,
            stage=STAGE_PORTFOLIO_LIVE_LIQUIDATION,
            step_kind=STEP_PORTFOLIO_LIVE_LIQUIDATION,
        ),
    )


def _should_append_portfolio_live_liquidation_step(
    *,
    target_scope: str,
    execution_mode: str,
) -> bool:
    return (
        target_scope == ALL_SOURCE_TO_MARTS_SCOPE
        and execution_mode == SOURCE_RAW_EXECUTION_MODE_FULL
    )


def _daily_run_config_for_step(
    step: BackfillStep,
    *,
    stage: str,
    dry_run: bool,
) -> Mapping[str, object]:
    if stage != STAGE_FURNACE_CALCULATION:
        return step.run_config
    return _daily_furnace_run_config(step.run_config, dry_run=dry_run)


def _daily_furnace_run_config(
    run_config: Mapping[str, object],
    *,
    dry_run: bool,
) -> Mapping[str, object]:
    if "ops" not in run_config:
        msg = "Furnace calculation step must include op run config"
        raise ValueError(msg)
    ops = run_config["ops"]
    if not isinstance(ops, Mapping):
        msg = f"Expected Furnace run_config ops mapping, got {type(ops)}"
        raise TypeError(msg)

    expected_mode = FURNACE_MODE_DRY_RUN if dry_run else FURNACE_MODE_REPLACE_CASCADE
    target_mode = FURNACE_MODE_DRY_RUN if dry_run else FURNACE_MODE_APPEND_LATEST
    converted_ops: dict[str, object] = {}
    for op_name, op_config in ops.items():
        if not isinstance(op_name, str):
            msg = f"Expected Furnace op name string, got {type(op_name)}"
            raise TypeError(msg)
        if not isinstance(op_config, Mapping):
            msg = f"Expected Furnace op config mapping for {op_name}, got {type(op_config)}"
            raise TypeError(msg)
        if "config" not in op_config:
            msg = f"Furnace op config is missing config block: {op_name}"
            raise ValueError(msg)
        config = op_config["config"]
        if not isinstance(config, Mapping):
            msg = f"Expected Furnace config mapping for {op_name}, got {type(config)}"
            raise TypeError(msg)
        if "mode" not in config:
            msg = f"Furnace config is missing mode: {op_name}"
            raise ValueError(msg)
        if config["mode"] != expected_mode:
            msg = (
                "Unexpected Furnace mode while converting daily plan: "
                f"op={op_name!r} expected={expected_mode!r} actual={config['mode']!r}"
            )
            raise ValueError(msg)

        converted_config = dict(config)
        converted_config["mode"] = target_mode
        converted_op_config = dict(op_config)
        converted_op_config["config"] = converted_config
        converted_ops[op_name] = converted_op_config

    converted_run_config = dict(run_config)
    converted_run_config["ops"] = converted_ops
    return converted_run_config


def _stage_for_source_to_marts_step(step: BackfillStep) -> str:
    if "backfill.stage" not in step.tags:
        msg = f"Source-to-marts step is missing backfill.stage tag: {step.label!r}"
        raise ValueError(msg)
    return step.tags["backfill.stage"]


def _step_tags(
    *,
    source_tags: Mapping[str, str],
    common_tags: Mapping[str, str],
    stage: str,
    step_kind: str,
) -> dict[str, str]:
    tags = dict(common_tags)
    tags["daily.stage"] = stage
    tags["daily.step"] = step_kind
    if "backfill.year" in source_tags:
        tags["daily.year"] = source_tags["backfill.year"]
    return tags


def _common_tags(
    *,
    daily_id: str,
    target_scope: str,
    target_date: date,
    execution_mode: str,
    parent_run_id: str,
) -> dict[str, str]:
    return {
        "daily.kind": DAILY_KIND,
        "daily.id": daily_id,
        "daily.target_scope": target_scope,
        "daily.target_date": target_date.isoformat(),
        "daily.execution_mode": execution_mode,
        "daily.parent_run_id": parent_run_id,
    }


def _build_daily_id(
    *,
    target_scope: str,
    target_date: date,
    controller_run_id: str,
) -> str:
    short_run_id = controller_run_id.replace("-", "")[:12]
    return f"{target_scope}-{target_date.isoformat()}-{short_run_id}"


def _parse_target_date(value: str) -> date:
    try:
        return date.fromisoformat(value)
    except ValueError as error:
        msg = f"Invalid target_date: {value!r}"
        raise ValueError(msg) from error


def _furnace_modes_from_step(step: DailyStep) -> set[object]:
    if "ops" not in step.run_config:
        return set()
    ops = step.run_config["ops"]
    if not isinstance(ops, Mapping):
        msg = f"Expected run_config ops mapping, got {type(ops)}"
        raise TypeError(msg)
    modes: set[object] = set()
    for op_config in ops.values():
        if not isinstance(op_config, Mapping):
            msg = f"Expected op config mapping, got {type(op_config)}"
            raise TypeError(msg)
        config = op_config["config"]
        if not isinstance(config, Mapping):
            msg = f"Expected config mapping, got {type(config)}"
            raise TypeError(msg)
        modes.add(config["mode"])
    return modes


def furnace_modes_for_daily_steps(steps: Sequence[DailyStep]) -> set[object]:
    modes: set[object] = set()
    for step in steps:
        if step.stage == STAGE_FURNACE_CALCULATION:
            modes |= _furnace_modes_from_step(step)
    return modes
