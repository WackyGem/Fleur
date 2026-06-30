from __future__ import annotations

import json
import shlex
import subprocess
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path
from zoneinfo import ZoneInfo

import dagster as dg

DAILY_ASSET_SELECTION = "key:source/baostock__query_history_k_data_plus_daily"
COMPACTED_ASSET_SELECTION = "key:source/baostock__query_history_k_data_plus_daily_compacted"
RAW_ASSET_SELECTION = "key:clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted"
DAILY_OP_NAME = "source__baostock__query_history_k_data_plus_daily"
COMPACTED_OP_NAME = "source__baostock__query_history_k_data_plus_daily_compacted"
BAOSTOCK_DAILY_START_DATE = date(1990, 12, 19)
BACKFILL_CONTROLLER_CONFIG_SCHEMA = {
    "start_year": int,
    "end_year": int,
    "cutoff_trade_date": dg.Field(
        dg.Noneable(str),
        default_value=None,
        is_required=False,
    ),
    "overwrite_existing_partitions": dg.Field(
        bool,
        default_value=False,
        is_required=False,
    ),
    "dry_run": dg.Field(bool, default_value=False, is_required=False),
}


class BaostockHistoryYearRangeBackfillConfig(dg.Config):
    start_year: int
    end_year: int
    cutoff_trade_date: str | None = None
    overwrite_existing_partitions: bool = False
    dry_run: bool = False


@dataclass(frozen=True)
class BackfillCommand:
    label: str
    year: int
    args: tuple[str, ...]


@dataclass(frozen=True)
class YearBackfillWindow:
    year: int
    start_date: date
    end_date: date
    is_partial_year: bool


def build_baostock_history_year_range_backfill_commands(
    config: BaostockHistoryYearRangeBackfillConfig,
    *,
    today: date,
) -> tuple[BackfillCommand, ...]:
    windows = _year_backfill_windows(config, today=today)
    commands: list[BackfillCommand] = []
    for window in windows:
        commands.append(_daily_command(window, config))
        commands.append(_compacted_command(window))
        commands.append(_raw_command(window))
    return tuple(commands)


@dg.op(config_schema=BACKFILL_CONTROLLER_CONFIG_SCHEMA)
def baostock__history_k_data_year_range_backfill_controller(
    context,
) -> None:
    config = BaostockHistoryYearRangeBackfillConfig(**context.op_config)
    commands = build_baostock_history_year_range_backfill_commands(
        config,
        today=_today_shanghai(),
    )
    for command in commands:
        context.log.info("BaoStock history backfill step: %s", command.label)
        context.log.info("Command: %s", shlex.join(command.args))
        if config.dry_run:
            continue
        _run_command(command)


@dg.job(
    name="baostock__history_k_data_year_range_backfill_job",
    description=(
        "Backfill BaoStock daily source partitions by year, compact each year, "
        "then sync the matching ClickHouse raw year partition."
    ),
)
def baostock__history_k_data_year_range_backfill_job() -> None:
    baostock__history_k_data_year_range_backfill_controller()


def _year_backfill_windows(
    config: BaostockHistoryYearRangeBackfillConfig,
    *,
    today: date,
) -> tuple[YearBackfillWindow, ...]:
    if config.start_year > config.end_year:
        msg = "start_year cannot be later than end_year"
        raise ValueError(msg)
    if config.start_year < BAOSTOCK_DAILY_START_DATE.year:
        msg = f"start_year cannot be earlier than {BAOSTOCK_DAILY_START_DATE.year}"
        raise ValueError(msg)

    cutoff_trade_date = _parse_cutoff_trade_date(config.cutoff_trade_date)
    if cutoff_trade_date is None and config.end_year >= today.year:
        msg = "cutoff_trade_date is required when end_year reaches the current or a future year"
        raise ValueError(msg)
    if cutoff_trade_date is not None and cutoff_trade_date.year != config.end_year:
        msg = "cutoff_trade_date must be in end_year"
        raise ValueError(msg)
    if cutoff_trade_date is not None and cutoff_trade_date > today:
        msg = "cutoff_trade_date cannot be later than today's Asia/Shanghai date"
        raise ValueError(msg)

    windows: list[YearBackfillWindow] = []
    for year in range(config.start_year, config.end_year + 1):
        start_date = date(year, 1, 1)
        if year == BAOSTOCK_DAILY_START_DATE.year:
            start_date = BAOSTOCK_DAILY_START_DATE
        end_date = date(year, 12, 31)
        if cutoff_trade_date is not None and year == cutoff_trade_date.year:
            end_date = cutoff_trade_date
        if end_date < start_date:
            msg = f"Year {year} backfill window ends before it starts"
            raise ValueError(msg)
        windows.append(
            YearBackfillWindow(
                year=year,
                start_date=start_date,
                end_date=end_date,
                is_partial_year=end_date != date(year, 12, 31),
            )
        )
    return tuple(windows)


def _parse_cutoff_trade_date(value: str | None) -> date | None:
    if value is None:
        return None
    try:
        return date.fromisoformat(value)
    except ValueError as error:
        msg = f"Invalid cutoff_trade_date: {value!r}"
        raise ValueError(msg) from error


def _daily_command(
    window: YearBackfillWindow,
    config: BaostockHistoryYearRangeBackfillConfig,
) -> BackfillCommand:
    args = [
        "uv",
        "run",
        "dg",
        "launch",
        "--target-path",
        "scheduler",
        "--assets",
        DAILY_ASSET_SELECTION,
        "--partition-range",
        f"{window.start_date.isoformat()}...{window.end_date.isoformat()}",
    ]
    op_config: dict[str, object] = {}
    if config.overwrite_existing_partitions:
        op_config["overwrite_existing_partitions"] = True
    if window.is_partial_year:
        op_config["cutoff_trade_date"] = window.end_date.isoformat()
    if op_config:
        args.extend(["--config-json", _op_config_json(DAILY_OP_NAME, op_config)])
    return BackfillCommand(
        label=f"daily source {window.year}",
        year=window.year,
        args=tuple(args),
    )


def _compacted_command(window: YearBackfillWindow) -> BackfillCommand:
    args = [
        "uv",
        "run",
        "dg",
        "launch",
        "--target-path",
        "scheduler",
        "--assets",
        COMPACTED_ASSET_SELECTION,
        "--partition",
        str(window.year),
    ]
    if window.is_partial_year:
        args.extend(
            [
                "--config-json",
                _op_config_json(
                    COMPACTED_OP_NAME,
                    {"cutoff_trade_date": window.end_date.isoformat()},
                ),
            ]
        )
    return BackfillCommand(
        label=f"compacted source {window.year}",
        year=window.year,
        args=tuple(args),
    )


def _raw_command(window: YearBackfillWindow) -> BackfillCommand:
    return BackfillCommand(
        label=f"clickhouse raw {window.year}",
        year=window.year,
        args=(
            "uv",
            "run",
            "dg",
            "launch",
            "--target-path",
            "scheduler",
            "--assets",
            RAW_ASSET_SELECTION,
            "--partition",
            str(window.year),
        ),
    )


def _op_config_json(op_name: str, config: dict[str, object]) -> str:
    return json.dumps(
        {"ops": {op_name: {"config": config}}},
        sort_keys=True,
        separators=(",", ":"),
    )


def _run_command(command: BackfillCommand) -> None:
    completed = subprocess.run(
        command.args,
        cwd=_pipeline_dir(),
        check=False,
    )
    if completed.returncode != 0:
        msg = f"BaoStock history backfill command failed: {shlex.join(command.args)}"
        raise RuntimeError(msg)


def _pipeline_dir() -> Path:
    return Path(__file__).resolve().parents[5]


def _today_shanghai() -> date:
    return datetime.now(ZoneInfo("Asia/Shanghai")).date()
