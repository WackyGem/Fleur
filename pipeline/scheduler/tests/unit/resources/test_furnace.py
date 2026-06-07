from __future__ import annotations

import subprocess
from typing import Any

import pytest
from scheduler.defs.resources.furnace import FurnaceCliResource, FurnaceKdjCliRequest


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
