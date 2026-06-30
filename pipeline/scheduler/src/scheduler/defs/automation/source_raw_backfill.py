from abc import ABC, abstractmethod
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from datetime import date, datetime
from typing import Literal, Protocol
from zoneinfo import ZoneInfo

import dagster as dg
from dagster._core.storage.tags import (  # noqa: PLC2701 - Dagster exposes no public equivalent.
    ASSET_PARTITION_RANGE_END_TAG,
    ASSET_PARTITION_RANGE_START_TAG,
)
from pydantic import Field

from scheduler.defs.baostock.assets import (
    baostock__query_history_k_data_plus_daily,
    baostock__query_history_k_data_plus_daily_compacted,
    baostock__query_stock_basic,
)
from scheduler.defs.clickhouse.assets import CLICKHOUSE_RAW_ASSETS
from scheduler.defs.clickhouse.specs import ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
from scheduler.defs.sources.chinabond.assets import chinabond__government_bond
from scheduler.defs.sources.eastmoney.assets import EASTMONEY_ASSETS
from scheduler.defs.sources.jiuyan.action_field import jiuyan__action_field
from scheduler.defs.sources.jiuyan.action_field_compact import jiuyan__action_field_compacted
from scheduler.defs.sources.jiuyan.industry_list import jiuyan__industry_list
from scheduler.defs.sources.jiuyan.industry_ocr import (
    jiuyan__industry_images,
    jiuyan__industry_ocr,
)
from scheduler.defs.sources.jiuyan.industry_ocr_snapshot import (
    jiuyan__industry_ocr_snapshot,
)
from scheduler.defs.sources.sina.trade_calendar import sina__trade_calendar
from scheduler.defs.sources.ths.limit_up_pool import ths__limit_up_pool
from scheduler.defs.sources.ths.limit_up_pool_compact import ths__limit_up_pool_compacted

BACKFILL_KIND = "fetch_sources_to_raw"
BACKFILL_JOB_NAME = "backfill__fetch_sources_to_raw_job"
BACKFILL_SNAPSHOT_JOB_NAME = "backfill__fetch_snapshot_sources_to_raw_job"
BACKFILL_CONTROLLER_OP_NAME = "backfill__fetch_sources_to_raw_controller"
BACKFILL_SNAPSHOT_CONTROLLER_OP_NAME = "backfill__fetch_snapshot_sources_to_raw_controller"
DEFAULT_JIUYAN_OCR_LIMIT = 100
BACKFILL_TIMEZONE = ZoneInfo("Asia/Shanghai")

BAOSTOCK_DAILY_KLINE_SCOPE = "baostock_daily_kline"
MARKET_EVENTS_SCOPE = "market_events"
EASTMONEY_F10_SCOPE = "eastmoney_f10"
CHINABOND_SCOPE = "chinabond"
SNAPSHOT_REFERENCE_DATA_SCOPE = "snapshot_reference_data"
JIUYAN_OCR_PIPELINE_SCOPE = "jiuyan_ocr_pipeline"
ALL_RAW_YEARLY_SCOPE = "all_raw_yearly"
ALL_FETCH_SOURCES_TO_RAW_SCOPE = "all_fetch_sources_to_raw"

EXECUTION_MODE_FULL = "full"
EXECUTION_MODE_RAW_ONLY = "raw_only"
DATE_REQUIRED_TARGET_SCOPES = (
    BAOSTOCK_DAILY_KLINE_SCOPE,
    MARKET_EVENTS_SCOPE,
    EASTMONEY_F10_SCOPE,
    CHINABOND_SCOPE,
    ALL_RAW_YEARLY_SCOPE,
)
DATE_OPTIONAL_TARGET_SCOPES = (
    SNAPSHOT_REFERENCE_DATA_SCOPE,
    JIUYAN_OCR_PIPELINE_SCOPE,
)

DateRequiredTargetScope = Literal[
    "baostock_daily_kline",
    "market_events",
    "eastmoney_f10",
    "chinabond",
    "all_raw_yearly",
]
DateOptionalTargetScope = Literal[
    "snapshot_reference_data",
    "jiuyan_ocr_pipeline",
]
BackfillExecutionMode = Literal["full", "raw_only"]

STEP_SNAPSHOT_PREREQUISITE = "snapshot_prerequisite"
STEP_SOURCE_SNAPSHOT = "source_snapshot"
STEP_SOURCE_DAILY = "source_daily"
STEP_SOURCE_YEAR = "source_year"
STEP_SOURCE_COMPACTED = "source_compacted"
STEP_OCR = "ocr_step"
STEP_RAW = "raw"
TERMINAL_SUCCESS_STATUSES = {"SUCCESS"}
TERMINAL_FAILURE_STATUSES = {"FAILURE", "CANCELED"}


@dataclass(frozen=True)
class BackfillControllerRequest:
    target_scope: str
    start_date: str | None
    end_date: str | None
    execution_mode: str
    refresh_prerequisite_snapshots: bool
    overwrite_source_partitions: bool
    jiuyan_ocr_limit: int | None
    jiuyan_force_download: bool
    jiuyan_force_ocr: bool
    dry_run: bool


class BackfillControllerConfig(dg.Config):
    target_scope: DateRequiredTargetScope = Field(
        ...,
        description=(
            "Date-partitioned source/raw backfill scope. Options: "
            "baostock_daily_kline, market_events, eastmoney_f10, chinabond, "
            "all_raw_yearly."
        ),
    )
    start_date: str = Field(
        ...,
        description="Inclusive business date, formatted as YYYY-MM-DD.",
    )
    end_date: str = Field(
        ...,
        description="Inclusive business date, formatted as YYYY-MM-DD.",
    )
    execution_mode: BackfillExecutionMode = Field(
        default=EXECUTION_MODE_FULL,
        description="full runs source and raw steps; raw_only runs only raw sync steps.",
    )
    refresh_prerequisite_snapshots: bool = Field(
        default=False,
        description="Refresh only prerequisite source snapshots declared by the selected scope.",
    )
    overwrite_source_partitions: bool = Field(
        default=False,
        description="Forward overwrite behavior only to source assets that explicitly support it.",
    )
    dry_run: bool = Field(
        default=True,
        description="When true, log the expanded backfill plan without submitting child runs.",
    )

    def to_request(self) -> BackfillControllerRequest:
        return BackfillControllerRequest(
            target_scope=self.target_scope,
            start_date=self.start_date,
            end_date=self.end_date,
            execution_mode=self.execution_mode,
            refresh_prerequisite_snapshots=self.refresh_prerequisite_snapshots,
            overwrite_source_partitions=self.overwrite_source_partitions,
            jiuyan_ocr_limit=DEFAULT_JIUYAN_OCR_LIMIT,
            jiuyan_force_download=False,
            jiuyan_force_ocr=False,
            dry_run=self.dry_run,
        )


class SnapshotBackfillControllerConfig(dg.Config):
    target_scope: DateOptionalTargetScope = Field(
        ...,
        description="Non-date-partitioned scope. Options: snapshot_reference_data, jiuyan_ocr_pipeline.",
    )
    execution_mode: BackfillExecutionMode = Field(
        default=EXECUTION_MODE_FULL,
        description="full runs source and raw steps; raw_only runs only raw sync steps.",
    )
    jiuyan_ocr_limit: int | None = Field(
        default=DEFAULT_JIUYAN_OCR_LIMIT,
        description="Jiuyan OCR item limit. Set null only when an unrestricted OCR run is intended.",
    )
    jiuyan_force_download: bool = Field(
        default=False,
        description="Force Jiuyan image downloads when the selected scope includes OCR steps.",
    )
    jiuyan_force_ocr: bool = Field(
        default=False,
        description="Force Jiuyan OCR when the selected scope includes OCR steps.",
    )
    dry_run: bool = Field(
        default=True,
        description="When true, log the expanded backfill plan without submitting child runs.",
    )

    def to_request(self) -> BackfillControllerRequest:
        return BackfillControllerRequest(
            target_scope=self.target_scope,
            start_date=None,
            end_date=None,
            execution_mode=self.execution_mode,
            refresh_prerequisite_snapshots=False,
            overwrite_source_partitions=False,
            jiuyan_ocr_limit=self.jiuyan_ocr_limit,
            jiuyan_force_download=self.jiuyan_force_download,
            jiuyan_force_ocr=self.jiuyan_force_ocr,
            dry_run=self.dry_run,
        )


@dataclass(frozen=True)
class BackfillAssetOpConfigMapping:
    asset_key: str
    op_name: str
    config_keys: tuple[str, ...]


@dataclass(frozen=True)
class BackfillPartitionSelection:
    partition_key: str | None = None
    partition_range_start: str | None = None
    partition_range_end: str | None = None

    @property
    def is_partitioned(self) -> bool:
        return self.partition_key is not None or self.partition_range_start is not None

    def label(self) -> str:
        if self.partition_key is not None:
            return f"partition={self.partition_key}"
        if self.partition_range_start is not None and self.partition_range_end is not None:
            return f"partition_range={self.partition_range_start}...{self.partition_range_end}"
        return "unpartitioned"

    def to_payload(self) -> dict[str, str | None]:
        return {
            "partition_key": self.partition_key,
            "partition_range_start": self.partition_range_start,
            "partition_range_end": self.partition_range_end,
        }


@dataclass(frozen=True)
class BackfillStep:
    label: str
    step_kind: str
    asset_keys: tuple[str, ...]
    partition: BackfillPartitionSelection
    run_config: Mapping[str, object]
    tags: Mapping[str, str]

    def to_payload(self) -> dict[str, object]:
        return {
            "label": self.label,
            "step_kind": self.step_kind,
            "asset_keys": list(self.asset_keys),
            "partition": self.partition.to_payload(),
            "run_config": dict(self.run_config),
            "tags": dict(self.tags),
        }


@dataclass(frozen=True)
class BackfillPlan:
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
            "Backfill plan: "
            f"backfill_id={self.backfill_id} "
            f"target_scope={self.target_scope} "
            f"start_date={self.start_date} "
            f"end_date={self.end_date} "
            f"execution_mode={self.execution_mode}"
        )
        lines = [header]
        for index, step in enumerate(self.steps, start=1):
            lines.append(
                f"{index}. {step.step_kind}: {', '.join(step.asset_keys)} {step.partition.label()}"
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
class BackfillStepResult:
    run_id: str
    status: str
    success: bool


class BackfillRunSubmitter(ABC):
    @abstractmethod
    def submit_step(self, step: BackfillStep) -> BackfillStepResult:
        """Submit or execute a planned materialization step and return its terminal result."""


class InfoLogger(Protocol):
    def info(self, msg: object, *args: object) -> None: ...


class RunTagWriter(Protocol):
    def add_run_tags(self, run_id: str, new_tags: Mapping[str, str]) -> None: ...


class InProcessDagsterRunSubmitter(BackfillRunSubmitter):
    def __init__(self, context: dg.OpExecutionContext) -> None:
        self._context = context

    def submit_step(self, step: BackfillStep) -> BackfillStepResult:
        asset_selection = [_asset_key_from_user_string(asset_key) for asset_key in step.asset_keys]
        job_def = self._context.repository_def.get_implicit_job_def_for_assets(asset_selection)
        if job_def is None:
            msg = f"No implicit asset job can materialize assets: {step.asset_keys}"
            raise RuntimeError(msg)

        tags = _execution_tags_for_step(step)
        result = job_def.execute_in_process(
            run_config=step.run_config,
            instance=self._context.instance,
            partition_key=step.partition.partition_key,
            raise_on_error=False,
            asset_selection=asset_selection,
            tags=tags,
        )
        status = result.dagster_run.status.value
        return BackfillStepResult(
            run_id=result.dagster_run.run_id,
            status=status,
            success=result.success,
        )


def execute_backfill_plan(
    plan: BackfillPlan,
    *,
    submitter: BackfillRunSubmitter,
    log: InfoLogger,
) -> tuple[BackfillStepResult, ...]:
    results: list[BackfillStepResult] = []
    for step in plan.steps:
        log.info(
            "Submitting backfill step %s assets=%s partition=%s",
            step.label,
            step.asset_keys,
            step.partition.label(),
        )
        result = submitter.submit_step(step)
        results.append(result)
        if result.status not in TERMINAL_SUCCESS_STATUSES | TERMINAL_FAILURE_STATUSES:
            msg = (
                "Backfill step returned non-terminal status: "
                f"label={step.label!r} run_id={result.run_id} status={result.status}"
            )
            raise RuntimeError(msg)
        log.info(
            "Backfill step finished %s run_id=%s status=%s",
            step.label,
            result.run_id,
            result.status,
        )
        if not result.success:
            msg = (
                "Backfill step failed: "
                f"label={step.label!r} assets={step.asset_keys} "
                f"partition={step.partition.label()} run_id={result.run_id} "
                f"status={result.status}"
            )
            raise RuntimeError(msg)
    return tuple(results)


def add_controller_run_tags(
    tag_writer: RunTagWriter,
    *,
    run_id: str,
    plan: BackfillPlan,
) -> None:
    tag_writer.add_run_tags(run_id, plan.controller_run_tags())


@dataclass(frozen=True)
class BackfillTargetSpec:
    target_scope: str
    child_scopes: tuple[str, ...] = ()
    prerequisite_snapshots: tuple[str, ...] = ()
    requires_date_range: bool = False


@dataclass(frozen=True)
class AssetStepSpec:
    label: str
    step_kind: str
    asset_keys: tuple[str, ...]
    partition: BackfillPartitionSelection
    year: str | None = None
    run_config_by_asset_key: Mapping[str, Mapping[str, object]] | None = None
    dedupe_source_materialization: bool = False


def build_backfill_plan(
    config: BackfillControllerRequest,
    *,
    today: date,
    controller_run_id: str,
) -> BackfillPlan:
    execution_mode = _validated_execution_mode(config.execution_mode)
    expanded_scopes = expand_target_scope(config.target_scope)
    requires_date_range = any(TARGET_SPECS[scope].requires_date_range for scope in expanded_scopes)
    start_date, end_date = _validated_date_range(
        config,
        requires_date_range=requires_date_range,
        today=today,
    )
    years = _year_partitions(start_date, end_date) if requires_date_range else ()
    backfill_id = _build_backfill_id(
        target_scope=config.target_scope,
        start_date=start_date,
        end_date=end_date,
        controller_run_id=controller_run_id,
    )

    step_specs: list[AssetStepSpec] = []
    if config.refresh_prerequisite_snapshots and execution_mode == EXECUTION_MODE_FULL:
        prerequisite_snapshots = _dedupe(
            snapshot
            for scope in expanded_scopes
            for snapshot in TARGET_SPECS[scope].prerequisite_snapshots
        )
        if prerequisite_snapshots:
            step_specs.append(
                AssetStepSpec(
                    label="snapshot prerequisites",
                    step_kind=STEP_SNAPSHOT_PREREQUISITE,
                    asset_keys=prerequisite_snapshots,
                    partition=BackfillPartitionSelection(),
                    dedupe_source_materialization=True,
                )
            )

    for scope in expanded_scopes:
        step_specs.extend(
            _scope_step_specs(
                scope,
                config=config,
                execution_mode=execution_mode,
                start_date=start_date,
                end_date=end_date,
            )
        )

    common_tags = _common_tags(
        backfill_id=backfill_id,
        target_scope=config.target_scope,
        start_date=start_date,
        end_date=end_date,
    )
    steps = _materialize_steps(step_specs, common_tags=common_tags)
    return BackfillPlan(
        backfill_id=backfill_id,
        target_scope=config.target_scope,
        execution_mode=execution_mode,
        start_date=start_date.isoformat() if start_date is not None else None,
        end_date=end_date.isoformat() if end_date is not None else None,
        year_partitions=years,
        steps=steps,
    )


def expand_target_scope(target_scope: str) -> tuple[str, ...]:
    if target_scope not in TARGET_SPECS:
        msg = f"Unsupported target_scope: {target_scope!r}"
        raise ValueError(msg)
    child_scopes = TARGET_SPECS[target_scope].child_scopes
    if not child_scopes:
        return (target_scope,)
    return _dedupe(
        scope for child_scope in child_scopes for scope in expand_target_scope(child_scope)
    )


def current_shanghai_date() -> date:
    return datetime.now(BACKFILL_TIMEZONE).date()


def source_asset_keys_covered_by_all_fetch_scope() -> set[str]:
    return _covered_source_asset_keys(expand_target_scope(ALL_FETCH_SOURCES_TO_RAW_SCOPE))


def raw_asset_keys_covered_by_all_fetch_scope() -> set[str]:
    return _covered_raw_asset_keys(expand_target_scope(ALL_FETCH_SOURCES_TO_RAW_SCOPE))


def all_registered_source_asset_keys() -> set[str]:
    return set(SOURCE_ASSET_BY_KEY)


def all_registered_raw_asset_keys() -> set[str]:
    return set(RAW_ASSET_BY_KEY)


def source_raw_op_config_mappings() -> tuple[BackfillAssetOpConfigMapping, ...]:
    mappings: list[BackfillAssetOpConfigMapping] = []
    for asset_key, config_keys in sorted(CHILD_CONFIG_KEYS_BY_ASSET_KEY.items()):
        mappings.append(
            BackfillAssetOpConfigMapping(
                asset_key=asset_key,
                op_name=op_name_for_asset_key(asset_key),
                config_keys=tuple(sorted(config_keys)),
            )
        )
    return tuple(mappings)


@dg.op(name=BACKFILL_CONTROLLER_OP_NAME)
def backfill__fetch_sources_to_raw_controller(
    context: dg.OpExecutionContext,
    config: BackfillControllerConfig,
) -> None:
    _execute_controller_request(context, config.to_request())


@dg.op(name=BACKFILL_SNAPSHOT_CONTROLLER_OP_NAME)
def backfill__fetch_snapshot_sources_to_raw_controller(
    context: dg.OpExecutionContext,
    config: SnapshotBackfillControllerConfig,
) -> None:
    _execute_controller_request(context, config.to_request())


def _execute_controller_request(
    context: dg.OpExecutionContext,
    config: BackfillControllerRequest,
) -> None:
    plan = build_backfill_plan(
        config,
        today=current_shanghai_date(),
        controller_run_id=context.run_id,
    )
    add_controller_run_tags(context.instance, run_id=context.run_id, plan=plan)
    for line in plan.log_lines():
        context.log.info(line)
    context.log.info("Backfill plan payload: %s", plan.to_payload())
    if config.dry_run:
        return
    execute_backfill_plan(
        plan,
        submitter=InProcessDagsterRunSubmitter(context),
        log=context.log,
    )


@dg.job(
    name=BACKFILL_JOB_NAME,
    description="Manually plan and submit source-to-ClickHouse-raw backfill materialization runs.",
)
def backfill__fetch_sources_to_raw_job() -> None:
    backfill__fetch_sources_to_raw_controller()


@dg.job(
    name=BACKFILL_SNAPSHOT_JOB_NAME,
    description="Manually plan and submit snapshot/OCR source-to-ClickHouse-raw backfill runs.",
)
def backfill__fetch_snapshot_sources_to_raw_job() -> None:
    backfill__fetch_snapshot_sources_to_raw_controller()


def op_name_for_asset_key(asset_key: str) -> str:
    if asset_key in SOURCE_ASSET_BY_KEY:
        return SOURCE_ASSET_BY_KEY[asset_key].node_def.name
    if asset_key in RAW_ASSET_BY_KEY:
        return RAW_ASSET_BY_KEY[asset_key].node_def.name
    msg = f"Asset key is not registered for source/raw backfill: {asset_key}"
    raise ValueError(msg)


def _execution_tags_for_step(step: BackfillStep) -> dict[str, str]:
    tags = dict(step.tags)
    if (
        step.partition.partition_range_start is not None
        and step.partition.partition_range_end is not None
    ):
        tags[ASSET_PARTITION_RANGE_START_TAG] = step.partition.partition_range_start
        tags[ASSET_PARTITION_RANGE_END_TAG] = step.partition.partition_range_end
    return tags


def _asset_key_from_user_string(asset_key: str) -> dg.AssetKey:
    return dg.AssetKey(asset_key.split("/"))


def _scope_step_specs(
    scope: str,
    *,
    config: BackfillControllerRequest,
    execution_mode: str,
    start_date: date | None,
    end_date: date | None,
) -> tuple[AssetStepSpec, ...]:
    if scope == SNAPSHOT_REFERENCE_DATA_SCOPE:
        return _snapshot_reference_step_specs(execution_mode=execution_mode)
    if scope == BAOSTOCK_DAILY_KLINE_SCOPE:
        return _baostock_daily_kline_step_specs(
            config=config,
            execution_mode=execution_mode,
            start_date=_required_date(start_date, field_name="start_date"),
            end_date=_required_date(end_date, field_name="end_date"),
        )
    if scope == MARKET_EVENTS_SCOPE:
        return _market_event_step_specs(
            execution_mode=execution_mode,
            start_date=_required_date(start_date, field_name="start_date"),
            end_date=_required_date(end_date, field_name="end_date"),
        )
    if scope == EASTMONEY_F10_SCOPE:
        return _year_source_to_raw_step_specs(
            label_prefix="eastmoney f10",
            source_asset_keys=EASTMONEY_SOURCE_KEYS,
            raw_asset_keys=tuple(raw_key_for_source_key(key) for key in EASTMONEY_SOURCE_KEYS),
            execution_mode=execution_mode,
            start_date=_required_date(start_date, field_name="start_date"),
            end_date=_required_date(end_date, field_name="end_date"),
            refresh_until_config_key="refresh_until_date",
        )
    if scope == CHINABOND_SCOPE:
        return _year_source_to_raw_step_specs(
            label_prefix="chinabond",
            source_asset_keys=(CHINABOND_SOURCE_KEY,),
            raw_asset_keys=(raw_key_for_source_key(CHINABOND_SOURCE_KEY),),
            execution_mode=execution_mode,
            start_date=_required_date(start_date, field_name="start_date"),
            end_date=_required_date(end_date, field_name="end_date"),
            refresh_until_config_key="refresh_until_date",
        )
    if scope == JIUYAN_OCR_PIPELINE_SCOPE:
        return _jiuyan_ocr_step_specs(config=config, execution_mode=execution_mode)
    msg = f"Unsupported expanded target_scope: {scope!r}"
    raise ValueError(msg)


def _snapshot_reference_step_specs(*, execution_mode: str) -> tuple[AssetStepSpec, ...]:
    source_keys = (
        SINA_TRADE_CALENDAR_KEY,
        BAOSTOCK_STOCK_BASIC_KEY,
        JIUYAN_INDUSTRY_LIST_KEY,
    )
    raw_keys = tuple(raw_key_for_source_key(key) for key in source_keys)
    if execution_mode == EXECUTION_MODE_RAW_ONLY:
        return (
            AssetStepSpec(
                label="snapshot reference raw",
                step_kind=STEP_RAW,
                asset_keys=raw_keys,
                partition=BackfillPartitionSelection(),
            ),
        )
    return (
        AssetStepSpec(
            label="snapshot reference source",
            step_kind=STEP_SOURCE_SNAPSHOT,
            asset_keys=source_keys,
            partition=BackfillPartitionSelection(),
            dedupe_source_materialization=True,
        ),
        AssetStepSpec(
            label="snapshot reference raw",
            step_kind=STEP_RAW,
            asset_keys=raw_keys,
            partition=BackfillPartitionSelection(),
        ),
    )


def _baostock_daily_kline_step_specs(
    *,
    config: BackfillControllerRequest,
    execution_mode: str,
    start_date: date,
    end_date: date,
) -> tuple[AssetStepSpec, ...]:
    raw_asset_key = raw_key_for_source_key(BAOSTOCK_DAILY_K_COMPACTED_KEY)
    specs: list[AssetStepSpec] = []
    for window in _year_windows(start_date, end_date):
        partition = BackfillPartitionSelection(partition_key=window.year)
        if execution_mode == EXECUTION_MODE_FULL:
            daily_config: dict[str, object] = {}
            if config.overwrite_source_partitions:
                daily_config["overwrite_existing_partitions"] = True
            if window.is_partial_year:
                daily_config["cutoff_trade_date"] = window.end_date.isoformat()
            compacted_config: dict[str, object] = {}
            if window.is_partial_year:
                compacted_config["cutoff_trade_date"] = window.end_date.isoformat()
            specs.extend(
                [
                    AssetStepSpec(
                        label=f"baostock daily source {window.year}",
                        step_kind=STEP_SOURCE_DAILY,
                        asset_keys=(BAOSTOCK_DAILY_K_KEY,),
                        partition=BackfillPartitionSelection(
                            partition_range_start=window.start_date.isoformat(),
                            partition_range_end=window.end_date.isoformat(),
                        ),
                        year=window.year,
                        run_config_by_asset_key={BAOSTOCK_DAILY_K_KEY: daily_config},
                        dedupe_source_materialization=True,
                    ),
                    AssetStepSpec(
                        label=f"baostock compacted source {window.year}",
                        step_kind=STEP_SOURCE_COMPACTED,
                        asset_keys=(BAOSTOCK_DAILY_K_COMPACTED_KEY,),
                        partition=partition,
                        year=window.year,
                        run_config_by_asset_key={BAOSTOCK_DAILY_K_COMPACTED_KEY: compacted_config},
                        dedupe_source_materialization=True,
                    ),
                ]
            )
        specs.append(
            AssetStepSpec(
                label=f"baostock raw {window.year}",
                step_kind=STEP_RAW,
                asset_keys=(raw_asset_key,),
                partition=partition,
                year=window.year,
            )
        )
    return tuple(specs)


def _market_event_step_specs(
    *,
    execution_mode: str,
    start_date: date,
    end_date: date,
) -> tuple[AssetStepSpec, ...]:
    raw_asset_keys = (
        raw_key_for_source_key(JIUYAN_ACTION_FIELD_COMPACTED_KEY),
        raw_key_for_source_key(THS_LIMIT_UP_POOL_COMPACTED_KEY),
    )
    specs: list[AssetStepSpec] = []
    for window in _year_windows(start_date, end_date):
        partition = BackfillPartitionSelection(partition_key=window.year)
        if execution_mode == EXECUTION_MODE_FULL:
            specs.extend(
                [
                    AssetStepSpec(
                        label=f"market event daily source {window.year}",
                        step_kind=STEP_SOURCE_DAILY,
                        asset_keys=(JIUYAN_ACTION_FIELD_KEY, THS_LIMIT_UP_POOL_KEY),
                        partition=BackfillPartitionSelection(
                            partition_range_start=window.start_date.isoformat(),
                            partition_range_end=window.end_date.isoformat(),
                        ),
                        year=window.year,
                        dedupe_source_materialization=True,
                    ),
                    AssetStepSpec(
                        label=f"market event compacted source {window.year}",
                        step_kind=STEP_SOURCE_COMPACTED,
                        asset_keys=(
                            JIUYAN_ACTION_FIELD_COMPACTED_KEY,
                            THS_LIMIT_UP_POOL_COMPACTED_KEY,
                        ),
                        partition=partition,
                        year=window.year,
                        dedupe_source_materialization=True,
                    ),
                ]
            )
        specs.append(
            AssetStepSpec(
                label=f"market event raw {window.year}",
                step_kind=STEP_RAW,
                asset_keys=raw_asset_keys,
                partition=partition,
                year=window.year,
            )
        )
    return tuple(specs)


def _year_source_to_raw_step_specs(
    *,
    label_prefix: str,
    source_asset_keys: tuple[str, ...],
    raw_asset_keys: tuple[str, ...],
    execution_mode: str,
    start_date: date,
    end_date: date,
    refresh_until_config_key: str,
) -> tuple[AssetStepSpec, ...]:
    specs: list[AssetStepSpec] = []
    for window in _year_windows(start_date, end_date):
        partition = BackfillPartitionSelection(partition_key=window.year)
        source_config: dict[str, object] = {}
        if window.is_partial_year:
            source_config[refresh_until_config_key] = window.end_date.isoformat()
        if execution_mode == EXECUTION_MODE_FULL:
            specs.append(
                AssetStepSpec(
                    label=f"{label_prefix} source {window.year}",
                    step_kind=STEP_SOURCE_YEAR,
                    asset_keys=source_asset_keys,
                    partition=partition,
                    year=window.year,
                    run_config_by_asset_key={
                        source_asset_key: source_config for source_asset_key in source_asset_keys
                    },
                    dedupe_source_materialization=True,
                )
            )
        specs.append(
            AssetStepSpec(
                label=f"{label_prefix} raw {window.year}",
                step_kind=STEP_RAW,
                asset_keys=raw_asset_keys,
                partition=partition,
                year=window.year,
            )
        )
    return tuple(specs)


def _jiuyan_ocr_step_specs(
    *,
    config: BackfillControllerRequest,
    execution_mode: str,
) -> tuple[AssetStepSpec, ...]:
    raw_asset_key = raw_key_for_source_key(JIUYAN_INDUSTRY_OCR_SNAPSHOT_KEY)
    if execution_mode == EXECUTION_MODE_RAW_ONLY:
        return (
            AssetStepSpec(
                label="jiuyan ocr raw",
                step_kind=STEP_RAW,
                asset_keys=(raw_asset_key,),
                partition=BackfillPartitionSelection(),
            ),
        )
    image_config: dict[str, object] = {
        "limit": config.jiuyan_ocr_limit,
        "force_download": config.jiuyan_force_download,
    }
    ocr_config: dict[str, object] = {
        "limit": config.jiuyan_ocr_limit,
        "force_ocr": config.jiuyan_force_ocr,
    }
    return (
        AssetStepSpec(
            label="jiuyan industry list",
            step_kind=STEP_SOURCE_SNAPSHOT,
            asset_keys=(JIUYAN_INDUSTRY_LIST_KEY,),
            partition=BackfillPartitionSelection(),
            dedupe_source_materialization=True,
        ),
        AssetStepSpec(
            label="jiuyan industry images",
            step_kind=STEP_OCR,
            asset_keys=(JIUYAN_INDUSTRY_IMAGES_KEY,),
            partition=BackfillPartitionSelection(),
            run_config_by_asset_key={JIUYAN_INDUSTRY_IMAGES_KEY: image_config},
            dedupe_source_materialization=True,
        ),
        AssetStepSpec(
            label="jiuyan industry ocr",
            step_kind=STEP_OCR,
            asset_keys=(JIUYAN_INDUSTRY_OCR_KEY,),
            partition=BackfillPartitionSelection(),
            run_config_by_asset_key={JIUYAN_INDUSTRY_OCR_KEY: ocr_config},
            dedupe_source_materialization=True,
        ),
        AssetStepSpec(
            label="jiuyan industry ocr snapshot",
            step_kind=STEP_SOURCE_SNAPSHOT,
            asset_keys=(JIUYAN_INDUSTRY_OCR_SNAPSHOT_KEY,),
            partition=BackfillPartitionSelection(),
            dedupe_source_materialization=True,
        ),
        AssetStepSpec(
            label="jiuyan ocr raw",
            step_kind=STEP_RAW,
            asset_keys=(raw_asset_key,),
            partition=BackfillPartitionSelection(),
        ),
    )


def _materialize_steps(
    step_specs: Sequence[AssetStepSpec],
    *,
    common_tags: Mapping[str, str],
) -> tuple[BackfillStep, ...]:
    materialized_source_identities: set[tuple[str, str, str | None, str | None, str | None]] = set()
    steps: list[BackfillStep] = []
    for step_spec in step_specs:
        asset_keys = step_spec.asset_keys
        if step_spec.dedupe_source_materialization:
            filtered_asset_keys = []
            for asset_key in asset_keys:
                identity = _materialization_identity(asset_key, step_spec.partition)
                if identity in materialized_source_identities:
                    continue
                materialized_source_identities.add(identity)
                filtered_asset_keys.append(asset_key)
            asset_keys = tuple(filtered_asset_keys)
        if not asset_keys:
            continue
        step_tags = dict(common_tags)
        step_tags["backfill.step"] = step_spec.step_kind
        if step_spec.year is not None:
            step_tags["backfill.year"] = step_spec.year
        run_config = _run_config_for_step(
            asset_keys,
            run_config_by_asset_key=step_spec.run_config_by_asset_key or {},
        )
        steps.append(
            BackfillStep(
                label=step_spec.label,
                step_kind=step_spec.step_kind,
                asset_keys=asset_keys,
                partition=step_spec.partition,
                run_config=run_config,
                tags=step_tags,
            )
        )
    return tuple(steps)


def _run_config_for_step(
    asset_keys: Iterable[str],
    *,
    run_config_by_asset_key: Mapping[str, Mapping[str, object]],
) -> Mapping[str, object]:
    op_configs: dict[str, object] = {}
    for asset_key in asset_keys:
        config = dict(run_config_by_asset_key.get(asset_key, {}))
        if not config:
            continue
        op_configs[op_name_for_asset_key(asset_key)] = {"config": config}
    if not op_configs:
        return {}
    return {"ops": op_configs}


def _common_tags(
    *,
    backfill_id: str,
    target_scope: str,
    start_date: date | None,
    end_date: date | None,
) -> dict[str, str]:
    return {
        "backfill.kind": BACKFILL_KIND,
        "backfill.id": backfill_id,
        "backfill.target_scope": target_scope,
        "backfill.start_date": start_date.isoformat() if start_date is not None else "",
        "backfill.end_date": end_date.isoformat() if end_date is not None else "",
    }


def _validated_execution_mode(execution_mode: str) -> str:
    if execution_mode in {EXECUTION_MODE_FULL, EXECUTION_MODE_RAW_ONLY}:
        return execution_mode
    msg = f"Unsupported execution_mode: {execution_mode!r}"
    raise ValueError(msg)


def _validated_date_range(
    config: BackfillControllerRequest,
    *,
    requires_date_range: bool,
    today: date,
) -> tuple[date | None, date | None]:
    if config.start_date is None or config.end_date is None:
        if requires_date_range:
            msg = f"target_scope {config.target_scope!r} requires start_date and end_date"
            raise ValueError(msg)
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


@dataclass(frozen=True)
class YearWindow:
    year: str
    start_date: date
    end_date: date
    is_partial_year: bool


def _year_windows(start_date: date, end_date: date) -> tuple[YearWindow, ...]:
    windows: list[YearWindow] = []
    for year in range(start_date.year, end_date.year + 1):
        window_start = max(start_date, date(year, 1, 1))
        window_end = min(end_date, date(year, 12, 31))
        windows.append(
            YearWindow(
                year=str(year),
                start_date=window_start,
                end_date=window_end,
                is_partial_year=window_start != date(year, 1, 1)
                or window_end != date(year, 12, 31),
            )
        )
    return tuple(windows)


def _year_partitions(start_date: date | None, end_date: date | None) -> tuple[str, ...]:
    if start_date is None or end_date is None:
        return ()
    return tuple(window.year for window in _year_windows(start_date, end_date))


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


def _required_date(value: date | None, *, field_name: str) -> date:
    if value is None:
        msg = f"{field_name} is required"
        raise ValueError(msg)
    return value


def _dedupe(values: Iterable[str]) -> tuple[str, ...]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if value in seen:
            continue
        seen.add(value)
        result.append(value)
    return tuple(result)


def _materialization_identity(
    asset_key: str,
    partition: BackfillPartitionSelection,
) -> tuple[str, str, str | None, str | None, str | None]:
    return (
        asset_key,
        partition.partition_key or "",
        partition.partition_range_start,
        partition.partition_range_end,
        None,
    )


def _single_asset_key(asset: dg.AssetsDefinition) -> str:
    if len(asset.keys) != 1:
        msg = "source/raw backfill registry only supports single-asset definitions"
        raise ValueError(msg)
    return next(iter(asset.keys)).to_user_string()


def _assets_by_key(assets: Iterable[dg.AssetsDefinition]) -> dict[str, dg.AssetsDefinition]:
    return {_single_asset_key(asset): asset for asset in assets}


def raw_key_for_source_key(source_asset_key: str) -> str:
    if source_asset_key not in RAW_ASSET_KEY_BY_SOURCE_KEY:
        msg = f"No enabled ClickHouse raw spec depends on source asset: {source_asset_key}"
        raise ValueError(msg)
    return RAW_ASSET_KEY_BY_SOURCE_KEY[source_asset_key]


def _covered_source_asset_keys(scopes: Iterable[str]) -> set[str]:
    covered: set[str] = set()
    for scope in scopes:
        covered.update(SOURCE_COVERAGE_BY_SCOPE[scope])
    return covered


def _covered_raw_asset_keys(scopes: Iterable[str]) -> set[str]:
    covered: set[str] = set()
    for scope in scopes:
        covered.update(RAW_COVERAGE_BY_SCOPE[scope])
    return covered


SINA_TRADE_CALENDAR_KEY = _single_asset_key(sina__trade_calendar)
BAOSTOCK_STOCK_BASIC_KEY = _single_asset_key(baostock__query_stock_basic)
BAOSTOCK_DAILY_K_KEY = _single_asset_key(baostock__query_history_k_data_plus_daily)
BAOSTOCK_DAILY_K_COMPACTED_KEY = _single_asset_key(
    baostock__query_history_k_data_plus_daily_compacted
)
JIUYAN_ACTION_FIELD_KEY = _single_asset_key(jiuyan__action_field)
JIUYAN_ACTION_FIELD_COMPACTED_KEY = _single_asset_key(jiuyan__action_field_compacted)
JIUYAN_INDUSTRY_LIST_KEY = _single_asset_key(jiuyan__industry_list)
JIUYAN_INDUSTRY_IMAGES_KEY = _single_asset_key(jiuyan__industry_images)
JIUYAN_INDUSTRY_OCR_KEY = _single_asset_key(jiuyan__industry_ocr)
JIUYAN_INDUSTRY_OCR_SNAPSHOT_KEY = _single_asset_key(jiuyan__industry_ocr_snapshot)
THS_LIMIT_UP_POOL_KEY = _single_asset_key(ths__limit_up_pool)
THS_LIMIT_UP_POOL_COMPACTED_KEY = _single_asset_key(ths__limit_up_pool_compacted)
CHINABOND_SOURCE_KEY = _single_asset_key(chinabond__government_bond)
EASTMONEY_SOURCE_KEYS = tuple(_single_asset_key(asset) for asset in EASTMONEY_ASSETS)

SOURCE_ASSET_BY_KEY = _assets_by_key(
    (
        sina__trade_calendar,
        jiuyan__action_field,
        jiuyan__action_field_compacted,
        jiuyan__industry_list,
        jiuyan__industry_images,
        jiuyan__industry_ocr,
        jiuyan__industry_ocr_snapshot,
        ths__limit_up_pool,
        ths__limit_up_pool_compacted,
        baostock__query_stock_basic,
        baostock__query_history_k_data_plus_daily,
        baostock__query_history_k_data_plus_daily_compacted,
        *EASTMONEY_ASSETS,
        chinabond__government_bond,
    )
)
RAW_ASSET_BY_KEY = _assets_by_key(CLICKHOUSE_RAW_ASSETS)
RAW_ASSET_KEY_BY_SOURCE_KEY = {
    spec.source_asset_key.to_user_string(): spec.raw_asset_key.to_user_string()
    for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
}

TARGET_SPECS: dict[str, BackfillTargetSpec] = {
    BAOSTOCK_DAILY_KLINE_SCOPE: BackfillTargetSpec(
        target_scope=BAOSTOCK_DAILY_KLINE_SCOPE,
        prerequisite_snapshots=(SINA_TRADE_CALENDAR_KEY, BAOSTOCK_STOCK_BASIC_KEY),
        requires_date_range=True,
    ),
    MARKET_EVENTS_SCOPE: BackfillTargetSpec(
        target_scope=MARKET_EVENTS_SCOPE,
        prerequisite_snapshots=(SINA_TRADE_CALENDAR_KEY,),
        requires_date_range=True,
    ),
    EASTMONEY_F10_SCOPE: BackfillTargetSpec(
        target_scope=EASTMONEY_F10_SCOPE,
        prerequisite_snapshots=(BAOSTOCK_STOCK_BASIC_KEY,),
        requires_date_range=True,
    ),
    CHINABOND_SCOPE: BackfillTargetSpec(
        target_scope=CHINABOND_SCOPE,
        requires_date_range=True,
    ),
    SNAPSHOT_REFERENCE_DATA_SCOPE: BackfillTargetSpec(
        target_scope=SNAPSHOT_REFERENCE_DATA_SCOPE,
    ),
    JIUYAN_OCR_PIPELINE_SCOPE: BackfillTargetSpec(
        target_scope=JIUYAN_OCR_PIPELINE_SCOPE,
    ),
    ALL_RAW_YEARLY_SCOPE: BackfillTargetSpec(
        target_scope=ALL_RAW_YEARLY_SCOPE,
        child_scopes=(
            BAOSTOCK_DAILY_KLINE_SCOPE,
            MARKET_EVENTS_SCOPE,
            EASTMONEY_F10_SCOPE,
            CHINABOND_SCOPE,
        ),
        requires_date_range=True,
    ),
    ALL_FETCH_SOURCES_TO_RAW_SCOPE: BackfillTargetSpec(
        target_scope=ALL_FETCH_SOURCES_TO_RAW_SCOPE,
        child_scopes=(
            SNAPSHOT_REFERENCE_DATA_SCOPE,
            BAOSTOCK_DAILY_KLINE_SCOPE,
            MARKET_EVENTS_SCOPE,
            EASTMONEY_F10_SCOPE,
            CHINABOND_SCOPE,
            JIUYAN_OCR_PIPELINE_SCOPE,
        ),
        requires_date_range=True,
    ),
}

SOURCE_COVERAGE_BY_SCOPE: dict[str, tuple[str, ...]] = {
    SNAPSHOT_REFERENCE_DATA_SCOPE: (
        SINA_TRADE_CALENDAR_KEY,
        BAOSTOCK_STOCK_BASIC_KEY,
        JIUYAN_INDUSTRY_LIST_KEY,
    ),
    BAOSTOCK_DAILY_KLINE_SCOPE: (
        BAOSTOCK_DAILY_K_KEY,
        BAOSTOCK_DAILY_K_COMPACTED_KEY,
    ),
    MARKET_EVENTS_SCOPE: (
        JIUYAN_ACTION_FIELD_KEY,
        JIUYAN_ACTION_FIELD_COMPACTED_KEY,
        THS_LIMIT_UP_POOL_KEY,
        THS_LIMIT_UP_POOL_COMPACTED_KEY,
    ),
    EASTMONEY_F10_SCOPE: EASTMONEY_SOURCE_KEYS,
    CHINABOND_SCOPE: (CHINABOND_SOURCE_KEY,),
    JIUYAN_OCR_PIPELINE_SCOPE: (
        JIUYAN_INDUSTRY_LIST_KEY,
        JIUYAN_INDUSTRY_IMAGES_KEY,
        JIUYAN_INDUSTRY_OCR_KEY,
        JIUYAN_INDUSTRY_OCR_SNAPSHOT_KEY,
    ),
}
RAW_COVERAGE_BY_SCOPE: dict[str, tuple[str, ...]] = {
    SNAPSHOT_REFERENCE_DATA_SCOPE: tuple(
        raw_key_for_source_key(key)
        for key in (SINA_TRADE_CALENDAR_KEY, BAOSTOCK_STOCK_BASIC_KEY, JIUYAN_INDUSTRY_LIST_KEY)
    ),
    BAOSTOCK_DAILY_KLINE_SCOPE: (raw_key_for_source_key(BAOSTOCK_DAILY_K_COMPACTED_KEY),),
    MARKET_EVENTS_SCOPE: tuple(
        raw_key_for_source_key(key)
        for key in (JIUYAN_ACTION_FIELD_COMPACTED_KEY, THS_LIMIT_UP_POOL_COMPACTED_KEY)
    ),
    EASTMONEY_F10_SCOPE: tuple(raw_key_for_source_key(key) for key in EASTMONEY_SOURCE_KEYS),
    CHINABOND_SCOPE: (raw_key_for_source_key(CHINABOND_SOURCE_KEY),),
    JIUYAN_OCR_PIPELINE_SCOPE: (raw_key_for_source_key(JIUYAN_INDUSTRY_OCR_SNAPSHOT_KEY),),
}
SOURCE_COVERAGE_BY_SCOPE[ALL_RAW_YEARLY_SCOPE] = tuple(
    sorted(_covered_source_asset_keys(TARGET_SPECS[ALL_RAW_YEARLY_SCOPE].child_scopes))
)
RAW_COVERAGE_BY_SCOPE[ALL_RAW_YEARLY_SCOPE] = tuple(
    sorted(_covered_raw_asset_keys(TARGET_SPECS[ALL_RAW_YEARLY_SCOPE].child_scopes))
)
SOURCE_COVERAGE_BY_SCOPE[ALL_FETCH_SOURCES_TO_RAW_SCOPE] = tuple(
    sorted(_covered_source_asset_keys(TARGET_SPECS[ALL_FETCH_SOURCES_TO_RAW_SCOPE].child_scopes))
)
RAW_COVERAGE_BY_SCOPE[ALL_FETCH_SOURCES_TO_RAW_SCOPE] = tuple(
    sorted(_covered_raw_asset_keys(TARGET_SPECS[ALL_FETCH_SOURCES_TO_RAW_SCOPE].child_scopes))
)

CHILD_CONFIG_KEYS_BY_ASSET_KEY: dict[str, tuple[str, ...]] = {
    BAOSTOCK_DAILY_K_KEY: ("cutoff_trade_date", "overwrite_existing_partitions"),
    BAOSTOCK_DAILY_K_COMPACTED_KEY: ("cutoff_trade_date",),
    **{asset_key: ("refresh_until_date",) for asset_key in EASTMONEY_SOURCE_KEYS},
    CHINABOND_SOURCE_KEY: ("refresh_until_date",),
    JIUYAN_INDUSTRY_IMAGES_KEY: ("force_download", "limit"),
    JIUYAN_INDUSTRY_OCR_KEY: ("force_ocr", "limit"),
}
