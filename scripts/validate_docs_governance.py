#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote

REPO_ROOT = Path(__file__).resolve().parents[1]
DOCS_DIR = REPO_ROOT / "docs"
ACTIVE_PLAN_DIR = DOCS_DIR / "plans"
JOB_REPORTS_DIR = DOCS_DIR / "jobs" / "reports"

PLAN_FILE_PATTERN = re.compile(r"^(?P<number>\d{4})-[a-z0-9][a-z0-9.-]*\.md$")
STATUS_PATTERN = re.compile(r"^状态[:：](?P<status>.+)$", re.MULTILINE)
MARKDOWN_LINK_PATTERN = re.compile(r"!?\[[^\]]*]\((?P<target>[^)]+)\)")

ACTIVE_PLAN_STATUSES = {"Proposed", "In Progress", "Blocked"}
ARCHIVE_REQUIRED_STATUSES = {"Completed", "Superseded", "Archived"}
EXTERNAL_LINK_PREFIXES = ("http://", "https://", "mailto:", "tel:")


@dataclass(frozen=True)
class Finding:
    path: Path
    message: str

    def format(self) -> str:
        return f"{self.path.relative_to(REPO_ROOT)}: {self.message}"


def main() -> int:
    findings: list[Finding] = []
    findings.extend(check_markdown_trailing_whitespace())
    findings.extend(check_active_plan_numbers())
    findings.extend(check_active_plan_statuses())
    findings.extend(check_job_reports())
    findings.extend(check_local_markdown_links())

    if findings:
        for finding in findings:
            print(finding.format(), file=sys.stderr)
        print(f"docs governance validation failed: {len(findings)} issue(s)", file=sys.stderr)
        return 1

    print("docs governance validation passed")
    return 0


def markdown_files() -> list[Path]:
    files = [REPO_ROOT / "AGENTS.md"]
    files.extend(sorted(DOCS_DIR.rglob("*.md")))
    return [path for path in files if path.exists()]


def check_markdown_trailing_whitespace() -> list[Finding]:
    findings: list[Finding] = []
    for path in markdown_files():
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if line.rstrip(" \t") != line:
                findings.append(Finding(path, f"line {line_number} has trailing whitespace"))
    return findings


def check_active_plan_numbers() -> list[Finding]:
    findings: list[Finding] = []
    numbers: dict[str, Path] = {}
    for path in sorted(ACTIVE_PLAN_DIR.glob("*.md")):
        if path.name == "README.md":
            continue
        match = PLAN_FILE_PATTERN.match(path.name)
        if match is None:
            findings.append(Finding(path, "active plan filename must start with NNNN-"))
            continue
        number = match.group("number")
        if number in numbers:
            findings.append(
                Finding(path, f"duplicate active plan number {number}; first seen in {numbers[number].name}")
            )
        else:
            numbers[number] = path
    return findings


def check_active_plan_statuses() -> list[Finding]:
    findings: list[Finding] = []
    for path in sorted(ACTIVE_PLAN_DIR.glob("*.md")):
        if path.name == "README.md":
            continue
        content = path.read_text(encoding="utf-8")
        status = first_status(content)
        if status is None:
            findings.append(Finding(path, "active plan is missing 状态："))
            continue
        if status in ACTIVE_PLAN_STATUSES:
            continue
        if status in ARCHIVE_REQUIRED_STATUSES:
            findings.append(Finding(path, f"status {status!r} belongs in docs/plans/archive/"))
        else:
            findings.append(
                Finding(path, f"unsupported active plan status {status!r}; use {sorted(ACTIVE_PLAN_STATUSES)}")
            )
    return findings


def first_status(content: str) -> str | None:
    match = STATUS_PATTERN.search(content)
    if match is None:
        return None
    return match.group("status").strip()


def check_job_reports() -> list[Finding]:
    findings: list[Finding] = []
    if not JOB_REPORTS_DIR.exists():
        return findings

    for path in sorted(JOB_REPORTS_DIR.glob("*.md")):
        content = path.read_text(encoding="utf-8")
        missing: list[str] = []
        if not re.search(r"日期|时间|date|time", content, re.IGNORECASE):
            missing.append("date/time")
        if not re.search(r"范围|输入|输出|分区|资产|证券|request|scope|table", content, re.IGNORECASE):
            missing.append("scope")
        if not re.search(r"命令|```bash|command", content, re.IGNORECASE):
            missing.append("command")
        if not re.search(r"结果|状态|摘要|验证|passed|failed|summary|result", content, re.IGNORECASE):
            missing.append("result")
        if missing:
            findings.append(Finding(path, f"job report missing required section hint(s): {', '.join(missing)}"))
    return findings


def check_local_markdown_links() -> list[Finding]:
    findings: list[Finding] = []
    for path in markdown_files():
        content = path.read_text(encoding="utf-8")
        for match in MARKDOWN_LINK_PATTERN.finditer(content):
            target = normalize_link_target(match.group("target"))
            if should_skip_link(target):
                continue
            if not local_link_exists(path, target):
                findings.append(Finding(path, f"broken local markdown link: {target}"))
    return findings


def normalize_link_target(raw_target: str) -> str:
    target = raw_target.strip()
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1]
    if " " in target and not target.startswith("#"):
        target = target.split(" ", maxsplit=1)[0]
    target = target.split("#", maxsplit=1)[0]
    return unquote(target)


def should_skip_link(target: str) -> bool:
    if target == "" or target.startswith("#"):
        return True
    if "\n" in target:
        return True
    return target.startswith(EXTERNAL_LINK_PREFIXES)


def local_link_exists(source_path: Path, target: str) -> bool:
    candidate_paths = []
    target_path = Path(target)
    if target_path.is_absolute():
        candidate_paths.append(REPO_ROOT / target_path.relative_to("/"))
    else:
        candidate_paths.append((source_path.parent / target_path).resolve())
        candidate_paths.append((REPO_ROOT / target_path).resolve())

    return any(candidate.exists() for candidate in candidate_paths)


if __name__ == "__main__":
    raise SystemExit(main())
