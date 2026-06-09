from __future__ import annotations

import json
import os
import subprocess
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import dagster as dg


@dataclass(frozen=True)
class FurnaceKdjCliRequest:
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: Sequence[str] = field(default_factory=tuple)
    rsv_window: int = 9
    k_smoothing: int = 3
    d_smoothing: int = 3
    insert_batch_size: int = 10_000
    run_id: str | None = None


@dataclass(frozen=True)
class FurnaceKdjCliResult:
    summary: Mapping[str, Any]
    stdout: str
    stderr: str
    exit_code: int


@dataclass(frozen=True)
class FurnaceMaCliRequest:
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: Sequence[str] = field(default_factory=tuple)
    input_table: str = "fleur_intermediate.int_stock_quotes_daily_adj"
    volume_input_table: str = "fleur_intermediate.int_stock_quotes_daily_unadj"
    output_table: str = "fleur_calculation.calc_stock_ma_daily"
    price_column: str = "close_price_forward_adj"
    volume_column: str = "volume"
    insert_batch_size: int = 10_000
    run_id: str | None = None


@dataclass(frozen=True)
class FurnaceMaCliResult:
    summary: Mapping[str, Any]
    stdout: str
    stderr: str
    exit_code: int


@dataclass(frozen=True)
class FurnaceRsiCliRequest:
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: Sequence[str] = field(default_factory=tuple)
    input_table: str = "fleur_intermediate.int_stock_quotes_daily_adj"
    output_table: str = "fleur_calculation.calc_stock_rsi_daily"
    price_column: str = "close_price_forward_adj"
    insert_batch_size: int = 10_000
    run_id: str | None = None


@dataclass(frozen=True)
class FurnaceRsiCliResult:
    summary: Mapping[str, Any]
    stdout: str
    stderr: str
    exit_code: int


@dataclass(frozen=True)
class FurnaceBollCliRequest:
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: Sequence[str] = field(default_factory=tuple)
    input_table: str = "fleur_intermediate.int_stock_quotes_daily_adj"
    output_table: str = "fleur_calculation.calc_stock_boll_daily"
    price_column: str = "close_price_forward_adj"
    insert_batch_size: int = 10_000
    run_id: str | None = None


@dataclass(frozen=True)
class FurnaceBollCliResult:
    summary: Mapping[str, Any]
    stdout: str
    stderr: str
    exit_code: int


@dataclass(frozen=True)
class FurnacePricePatternCliRequest:
    request_from: str
    request_to: str
    mode: str = "dry-run"
    symbols: Sequence[str] = field(default_factory=tuple)
    structure_input_table: str = "fleur_intermediate.int_stock_quotes_daily_adj"
    streak_input_table: str = "fleur_intermediate.int_stock_quotes_daily_unadj"
    output_table: str = "fleur_calculation.calc_stock_price_pattern_daily"
    high_column: str = "high_price_forward_adj"
    low_column: str = "low_price_forward_adj"
    close_column: str = "close_price"
    prev_close_column: str = "prev_close_price"
    insert_batch_size: int = 10_000
    run_id: str | None = None


@dataclass(frozen=True)
class FurnacePricePatternCliResult:
    summary: Mapping[str, Any]
    stdout: str
    stderr: str
    exit_code: int


class FurnaceCliResource(dg.ConfigurableResource):
    binary_path: str = "engines/target/debug/furnace"
    working_dir: str = "."
    timeout_seconds: int = 300
    rayon_num_threads: int | None = 8

    def run_kdj(self, request: FurnaceKdjCliRequest) -> FurnaceKdjCliResult:
        command = self.command_for_request(request)
        try:
            completed = subprocess.run(
                command,
                cwd=self._resolved_working_dir(),
                env=self._subprocess_env(),
                text=True,
                capture_output=True,
                timeout=self.timeout_seconds,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            msg = f"Furnace CLI timed out after {self.timeout_seconds} seconds"
            raise RuntimeError(msg) from error

        if completed.returncode != 0:
            msg = f"Furnace CLI failed with exit code {completed.returncode}: {completed.stderr}"
            raise RuntimeError(msg)

        summary = self._parse_summary(completed.stdout)
        return FurnaceKdjCliResult(
            summary=summary,
            stdout=completed.stdout,
            stderr=completed.stderr,
            exit_code=completed.returncode,
        )

    def run_ma(self, request: FurnaceMaCliRequest) -> FurnaceMaCliResult:
        command = self.command_for_ma_request(request)
        try:
            completed = subprocess.run(
                command,
                cwd=self._resolved_working_dir(),
                env=self._subprocess_env(),
                text=True,
                capture_output=True,
                timeout=self.timeout_seconds,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            msg = f"Furnace CLI timed out after {self.timeout_seconds} seconds"
            raise RuntimeError(msg) from error

        if completed.returncode != 0:
            msg = f"Furnace CLI failed with exit code {completed.returncode}: {completed.stderr}"
            raise RuntimeError(msg)

        summary = self._parse_summary(completed.stdout)
        return FurnaceMaCliResult(
            summary=summary,
            stdout=completed.stdout,
            stderr=completed.stderr,
            exit_code=completed.returncode,
        )

    def run_rsi(self, request: FurnaceRsiCliRequest) -> FurnaceRsiCliResult:
        command = self.command_for_rsi_request(request)
        try:
            completed = subprocess.run(
                command,
                cwd=self._resolved_working_dir(),
                env=self._subprocess_env(),
                text=True,
                capture_output=True,
                timeout=self.timeout_seconds,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            msg = f"Furnace CLI timed out after {self.timeout_seconds} seconds"
            raise RuntimeError(msg) from error

        if completed.returncode != 0:
            msg = f"Furnace CLI failed with exit code {completed.returncode}: {completed.stderr}"
            raise RuntimeError(msg)

        summary = self._parse_summary(completed.stdout)
        return FurnaceRsiCliResult(
            summary=summary,
            stdout=completed.stdout,
            stderr=completed.stderr,
            exit_code=completed.returncode,
        )

    def run_boll(self, request: FurnaceBollCliRequest) -> FurnaceBollCliResult:
        command = self.command_for_boll_request(request)
        try:
            completed = subprocess.run(
                command,
                cwd=self._resolved_working_dir(),
                env=self._subprocess_env(),
                text=True,
                capture_output=True,
                timeout=self.timeout_seconds,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            msg = f"Furnace CLI timed out after {self.timeout_seconds} seconds"
            raise RuntimeError(msg) from error

        if completed.returncode != 0:
            msg = f"Furnace CLI failed with exit code {completed.returncode}: {completed.stderr}"
            raise RuntimeError(msg)

        summary = self._parse_summary(completed.stdout)
        return FurnaceBollCliResult(
            summary=summary,
            stdout=completed.stdout,
            stderr=completed.stderr,
            exit_code=completed.returncode,
        )

    def run_price_pattern(
        self, request: FurnacePricePatternCliRequest
    ) -> FurnacePricePatternCliResult:
        command = self.command_for_price_pattern_request(request)
        try:
            completed = subprocess.run(
                command,
                cwd=self._resolved_working_dir(),
                env=self._subprocess_env(),
                text=True,
                capture_output=True,
                timeout=self.timeout_seconds,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            msg = f"Furnace CLI timed out after {self.timeout_seconds} seconds"
            raise RuntimeError(msg) from error

        if completed.returncode != 0:
            msg = f"Furnace CLI failed with exit code {completed.returncode}: {completed.stderr}"
            raise RuntimeError(msg)

        summary = self._parse_summary(completed.stdout)
        return FurnacePricePatternCliResult(
            summary=summary,
            stdout=completed.stdout,
            stderr=completed.stderr,
            exit_code=completed.returncode,
        )

    def command_for_request(self, request: FurnaceKdjCliRequest) -> list[str]:
        command = [
            self._resolved_binary_path(),
            "kdj",
            "--from",
            request.request_from,
            "--to",
            request.request_to,
            "--mode",
            request.mode,
            "--rsv-window",
            str(request.rsv_window),
            "--k-smoothing",
            str(request.k_smoothing),
            "--d-smoothing",
            str(request.d_smoothing),
            "--insert-batch-size",
            str(request.insert_batch_size),
            "--output-format",
            "json",
        ]
        if request.symbols:
            command.extend(["--symbols", ",".join(request.symbols)])
        if request.run_id is not None:
            command.extend(["--run-id", request.run_id])
        return command

    def command_for_ma_request(self, request: FurnaceMaCliRequest) -> list[str]:
        command = [
            self._resolved_binary_path(),
            "ma",
            "--from",
            request.request_from,
            "--to",
            request.request_to,
            "--mode",
            request.mode,
            "--input-table",
            request.input_table,
            "--volume-input-table",
            request.volume_input_table,
            "--output-table",
            request.output_table,
            "--price-column",
            request.price_column,
            "--volume-column",
            request.volume_column,
            "--insert-batch-size",
            str(request.insert_batch_size),
            "--output-format",
            "json",
        ]
        if request.symbols:
            command.extend(["--symbols", ",".join(request.symbols)])
        if request.run_id is not None:
            command.extend(["--run-id", request.run_id])
        return command

    def command_for_rsi_request(self, request: FurnaceRsiCliRequest) -> list[str]:
        command = [
            self._resolved_binary_path(),
            "rsi",
            "--from",
            request.request_from,
            "--to",
            request.request_to,
            "--mode",
            request.mode,
            "--input-table",
            request.input_table,
            "--output-table",
            request.output_table,
            "--price-column",
            request.price_column,
            "--insert-batch-size",
            str(request.insert_batch_size),
            "--output-format",
            "json",
        ]
        if request.symbols:
            command.extend(["--symbols", ",".join(request.symbols)])
        if request.run_id is not None:
            command.extend(["--run-id", request.run_id])
        return command

    def command_for_boll_request(self, request: FurnaceBollCliRequest) -> list[str]:
        command = [
            self._resolved_binary_path(),
            "boll",
            "--from",
            request.request_from,
            "--to",
            request.request_to,
            "--mode",
            request.mode,
            "--input-table",
            request.input_table,
            "--output-table",
            request.output_table,
            "--price-column",
            request.price_column,
            "--insert-batch-size",
            str(request.insert_batch_size),
            "--output-format",
            "json",
        ]
        if request.symbols:
            command.extend(["--symbols", ",".join(request.symbols)])
        if request.run_id is not None:
            command.extend(["--run-id", request.run_id])
        return command

    def command_for_price_pattern_request(
        self, request: FurnacePricePatternCliRequest
    ) -> list[str]:
        command = [
            self._resolved_binary_path(),
            "price-pattern",
            "--from",
            request.request_from,
            "--to",
            request.request_to,
            "--mode",
            request.mode,
            "--structure-input-table",
            request.structure_input_table,
            "--streak-input-table",
            request.streak_input_table,
            "--output-table",
            request.output_table,
            "--high-column",
            request.high_column,
            "--low-column",
            request.low_column,
            "--close-column",
            request.close_column,
            "--prev-close-column",
            request.prev_close_column,
            "--insert-batch-size",
            str(request.insert_batch_size),
            "--output-format",
            "json",
        ]
        if request.symbols:
            command.extend(["--symbols", ",".join(request.symbols)])
        if request.run_id is not None:
            command.extend(["--run-id", request.run_id])
        return command

    def _resolved_binary_path(self) -> str:
        binary_path = Path(self.binary_path)
        if binary_path.is_absolute():
            return str(binary_path)
        working_dir = self._resolved_working_dir()
        candidate = (working_dir / binary_path).resolve()
        if candidate.exists():
            return str(candidate)
        for parent in (working_dir, *working_dir.parents):
            candidate = (parent / binary_path).resolve()
            if candidate.exists():
                return str(candidate)
        return str((working_dir / binary_path).resolve())

    def _resolved_working_dir(self) -> Path:
        working_dir = Path(self.working_dir)
        if working_dir.is_absolute():
            return working_dir
        return Path.cwd() / working_dir

    def _subprocess_env(self) -> dict[str, str]:
        env = dict(os.environ)
        if self.rayon_num_threads is not None and "RAYON_NUM_THREADS" not in env:
            env["RAYON_NUM_THREADS"] = str(self.rayon_num_threads)
        return env

    @staticmethod
    def _parse_summary(stdout: str) -> Mapping[str, Any]:
        try:
            summary = json.loads(stdout.strip())
        except json.JSONDecodeError as error:
            msg = "Furnace CLI stdout is not a valid JSON summary"
            raise RuntimeError(msg) from error
        if not isinstance(summary, dict):
            msg = "Furnace CLI JSON summary must be an object"
            raise RuntimeError(msg)
        return summary
