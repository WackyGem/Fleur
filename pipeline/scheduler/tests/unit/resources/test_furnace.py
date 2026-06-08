from __future__ import annotations

import subprocess
from typing import Any

import pytest
from scheduler.defs.resources.furnace import (
    FurnaceBollCliRequest,
    FurnaceCliResource,
    FurnaceKdjCliRequest,
    FurnaceMaCliRequest,
    FurnaceRsiCliRequest,
)


def test_furnace_cli_resource_builds_stable_kdj_command() -> None:
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")
    request = FurnaceKdjCliRequest(
        request_from="2026-01-01",
        request_to="2026-01-02",
        mode="dry-run",
        symbols=("sh.600000", "sz.000001"),
        run_id="run-1",
    )

    command = resource.command_for_request(request)

    assert command == [
        "/bin/furnace",
        "kdj",
        "--from",
        "2026-01-01",
        "--to",
        "2026-01-02",
        "--mode",
        "dry-run",
        "--rsv-window",
        "9",
        "--k-smoothing",
        "3",
        "--d-smoothing",
        "3",
        "--insert-batch-size",
        "10000",
        "--output-format",
        "json",
        "--symbols",
        "sh.600000,sz.000001",
        "--run-id",
        "run-1",
    ]


def test_furnace_cli_resource_builds_stable_ma_command() -> None:
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")
    request = FurnaceMaCliRequest(
        request_from="2026-01-01",
        request_to="2026-01-02",
        mode="dry-run",
        symbols=("sh.600000", "sz.000001"),
        run_id="run-1",
    )

    command = resource.command_for_ma_request(request)

    assert command == [
        "/bin/furnace",
        "ma",
        "--from",
        "2026-01-01",
        "--to",
        "2026-01-02",
        "--mode",
        "dry-run",
        "--input-table",
        "fleur_intermediate.int_stock_quotes_daily_adj",
        "--volume-input-table",
        "fleur_intermediate.int_stock_quotes_daily_unadj",
        "--output-table",
        "fleur_calculation.calc_stock_ma_daily",
        "--price-column",
        "close_price_forward_adj",
        "--volume-column",
        "volume",
        "--insert-batch-size",
        "10000",
        "--output-format",
        "json",
        "--symbols",
        "sh.600000,sz.000001",
        "--run-id",
        "run-1",
    ]


def test_furnace_cli_resource_builds_stable_rsi_command() -> None:
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")
    request = FurnaceRsiCliRequest(
        request_from="2026-01-01",
        request_to="2026-01-02",
        mode="dry-run",
        symbols=("sh.600000", "sz.000001"),
        run_id="run-1",
    )

    command = resource.command_for_rsi_request(request)

    assert command == [
        "/bin/furnace",
        "rsi",
        "--from",
        "2026-01-01",
        "--to",
        "2026-01-02",
        "--mode",
        "dry-run",
        "--input-table",
        "fleur_intermediate.int_stock_quotes_daily_adj",
        "--output-table",
        "fleur_calculation.calc_stock_rsi_daily",
        "--price-column",
        "close_price_forward_adj",
        "--insert-batch-size",
        "10000",
        "--output-format",
        "json",
        "--symbols",
        "sh.600000,sz.000001",
        "--run-id",
        "run-1",
    ]


def test_furnace_cli_resource_builds_stable_boll_command() -> None:
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")
    request = FurnaceBollCliRequest(
        request_from="2026-01-01",
        request_to="2026-01-02",
        mode="dry-run",
        symbols=("sh.600000", "sz.000001"),
        run_id="run-1",
    )

    command = resource.command_for_boll_request(request)

    assert command == [
        "/bin/furnace",
        "boll",
        "--from",
        "2026-01-01",
        "--to",
        "2026-01-02",
        "--mode",
        "dry-run",
        "--input-table",
        "fleur_intermediate.int_stock_quotes_daily_adj",
        "--output-table",
        "fleur_calculation.calc_stock_boll_daily",
        "--price-column",
        "close_price_forward_adj",
        "--insert-batch-size",
        "10000",
        "--output-format",
        "json",
        "--symbols",
        "sh.600000,sz.000001",
        "--run-id",
        "run-1",
    ]


def test_furnace_cli_resource_parses_json_summary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_run(*args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(
            args=args[0],
            returncode=0,
            stdout='{"request_from":"2026-01-01","output_rows":10}',
            stderr="",
        )

    monkeypatch.setattr("scheduler.defs.resources.furnace.subprocess.run", fake_run)
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")

    result = resource.run_kdj(
        FurnaceKdjCliRequest(request_from="2026-01-01", request_to="2026-01-02")
    )

    assert result.summary["output_rows"] == 10


def test_furnace_cli_resource_parses_ma_json_summary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_run(*args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(
            args=args[0],
            returncode=0,
            stdout='{"indicator":"ma","request_from":"2026-01-01","output_rows":10}',
            stderr="",
        )

    monkeypatch.setattr("scheduler.defs.resources.furnace.subprocess.run", fake_run)
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")

    result = resource.run_ma(
        FurnaceMaCliRequest(request_from="2026-01-01", request_to="2026-01-02")
    )

    assert result.summary["indicator"] == "ma"
    assert result.summary["output_rows"] == 10


def test_furnace_cli_resource_parses_rsi_json_summary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_run(*args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(
            args=args[0],
            returncode=0,
            stdout='{"indicator":"rsi","request_from":"2026-01-01","output_rows":10}',
            stderr="",
        )

    monkeypatch.setattr("scheduler.defs.resources.furnace.subprocess.run", fake_run)
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")

    result = resource.run_rsi(
        FurnaceRsiCliRequest(request_from="2026-01-01", request_to="2026-01-02")
    )

    assert result.summary["indicator"] == "rsi"
    assert result.summary["output_rows"] == 10


def test_furnace_cli_resource_parses_boll_json_summary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_run(*args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(
            args=args[0],
            returncode=0,
            stdout='{"indicator":"boll","request_from":"2026-01-01","output_rows":10}',
            stderr="",
        )

    monkeypatch.setattr("scheduler.defs.resources.furnace.subprocess.run", fake_run)
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")

    result = resource.run_boll(
        FurnaceBollCliRequest(request_from="2026-01-01", request_to="2026-01-02")
    )

    assert result.summary["indicator"] == "boll"
    assert result.summary["output_rows"] == 10


def test_furnace_cli_resource_injects_default_rayon_threads(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured_env: dict[str, str] = {}

    def fake_run(*args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        captured_env.update(kwargs["env"])
        return subprocess.CompletedProcess(
            args=args[0],
            returncode=0,
            stdout='{"request_from":"2026-01-01","output_rows":10}',
            stderr="",
        )

    monkeypatch.delenv("RAYON_NUM_THREADS", raising=False)
    monkeypatch.setattr("scheduler.defs.resources.furnace.subprocess.run", fake_run)
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")

    resource.run_kdj(FurnaceKdjCliRequest(request_from="2026-01-01", request_to="2026-01-02"))

    assert captured_env["RAYON_NUM_THREADS"] == "8"


def test_furnace_cli_resource_respects_existing_rayon_threads(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured_env: dict[str, str] = {}

    def fake_run(*args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        captured_env.update(kwargs["env"])
        return subprocess.CompletedProcess(
            args=args[0],
            returncode=0,
            stdout='{"request_from":"2026-01-01","output_rows":10}',
            stderr="",
        )

    monkeypatch.setenv("RAYON_NUM_THREADS", "4")
    monkeypatch.setattr("scheduler.defs.resources.furnace.subprocess.run", fake_run)
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")

    resource.run_kdj(FurnaceKdjCliRequest(request_from="2026-01-01", request_to="2026-01-02"))

    assert captured_env["RAYON_NUM_THREADS"] == "4"


def test_furnace_cli_resource_rejects_invalid_json_stdout(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_run(*args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args=args[0], returncode=0, stdout="not-json", stderr="")

    monkeypatch.setattr("scheduler.defs.resources.furnace.subprocess.run", fake_run)
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")

    with pytest.raises(RuntimeError, match="not a valid JSON summary"):
        resource.run_kdj(FurnaceKdjCliRequest(request_from="2026-01-01", request_to="2026-01-02"))


def test_furnace_cli_resource_fails_on_nonzero_exit_code(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_run(*args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args=args[0], returncode=3, stdout="", stderr="failed")

    monkeypatch.setattr("scheduler.defs.resources.furnace.subprocess.run", fake_run)
    resource = FurnaceCliResource(binary_path="/bin/furnace", working_dir="/tmp")

    with pytest.raises(RuntimeError, match="exit code 3"):
        resource.run_kdj(FurnaceKdjCliRequest(request_from="2026-01-01", request_to="2026-01-02"))
