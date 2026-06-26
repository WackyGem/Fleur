#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")
DATASET_VERSION_RE = re.compile(r"^version:\s*(\d+)\s*$", re.MULTILINE)
DBT_VERSION_RE = re.compile(r"^version:\s*['\"]?([^'\"\s]+)['\"]?\s*$", re.MULTILINE)
MARKDOWN_LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")

EXPECTED_MANIFEST_COMPONENTS = {
    "scheduler": ("pipeline/scheduler/pyproject.toml", "project"),
    "contract-tools": ("pipeline/contract_tools/pyproject.toml", "project"),
    "elt": ("pipeline/elt/dbt_project.yml", "dbt"),
    "furnace": ("engines/crates/furnace/Cargo.toml", "cargo"),
    "rearview-server": ("engines/crates/rearview-server/Cargo.toml", "cargo"),
    "rearview-portfolio-worker": (
        "engines/crates/rearview-portfolio-worker/Cargo.toml",
        "cargo",
    ),
    "racingline": ("app/racingline/package.json", "npm"),
}


def main() -> int:
    errors: list[str] = []
    component_versions = collect_component_versions(errors)
    check_racingline_lockfile(component_versions, errors)
    check_rust_crate_versions(errors)
    check_dataset_contract_versions(errors)
    check_release_manifest(component_versions, errors)
    check_markdown_links(REPO_ROOT / "docs/RFC/README.md", errors)
    check_markdown_links(REPO_ROOT / "docs/plans/README.md", errors)

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("Component version checks passed.")
    return 0


def collect_component_versions(errors: list[str]) -> dict[str, str]:
    versions: dict[str, str] = {}
    for component, (relative_path, source_type) in EXPECTED_MANIFEST_COMPONENTS.items():
        path = REPO_ROOT / relative_path
        if source_type == "project":
            version = read_pyproject_version(path, errors)
        elif source_type == "dbt":
            version = read_dbt_project_version(path, errors)
        elif source_type == "cargo":
            version = read_cargo_package_version(path, errors)
        elif source_type == "npm":
            version = read_package_json_version(path, errors)
        else:
            errors.append(f"unsupported source type for {component}: {source_type}")
            continue
        if version is None:
            continue
        if not SEMVER_RE.fullmatch(version):
            errors.append(f"{relative_path}: version is not SemVer: {version}")
            continue
        versions[component] = version
    return versions


def read_pyproject_version(path: Path, errors: list[str]) -> str | None:
    project = read_toml(path, errors).get("project")
    if not isinstance(project, dict):
        errors.append(f"{path.relative_to(REPO_ROOT)}: missing [project]")
        return None
    version = project.get("version")
    if not isinstance(version, str):
        errors.append(f"{path.relative_to(REPO_ROOT)}: missing project.version")
        return None
    return version


def read_cargo_package_version(path: Path, errors: list[str]) -> str | None:
    package = read_toml(path, errors).get("package")
    if not isinstance(package, dict):
        errors.append(f"{path.relative_to(REPO_ROOT)}: missing [package]")
        return None
    version = package.get("version")
    if isinstance(version, dict) and version.get("workspace") is True:
        errors.append(f"{path.relative_to(REPO_ROOT)}: package version uses workspace inheritance")
        return None
    if not isinstance(version, str):
        errors.append(f"{path.relative_to(REPO_ROOT)}: missing package.version")
        return None
    return version


def read_dbt_project_version(path: Path, errors: list[str]) -> str | None:
    text = read_text(path, errors)
    if text is None:
        return None
    match = DBT_VERSION_RE.search(text)
    if match is None:
        errors.append(f"{path.relative_to(REPO_ROOT)}: missing dbt project version")
        return None
    return match.group(1)


def read_package_json_version(path: Path, errors: list[str]) -> str | None:
    data = read_json(path, errors)
    version = data.get("version")
    if not isinstance(version, str):
        errors.append(f"{path.relative_to(REPO_ROOT)}: missing package version")
        return None
    return version


def check_racingline_lockfile(component_versions: dict[str, str], errors: list[str]) -> None:
    expected = component_versions.get("racingline")
    if expected is None:
        return
    path = REPO_ROOT / "app/racingline/package-lock.json"
    lockfile = read_json(path, errors)
    root_version = lockfile.get("version")
    packages = lockfile.get("packages")
    package_root = packages.get("") if isinstance(packages, dict) else None
    package_root_version = package_root.get("version") if isinstance(package_root, dict) else None
    if root_version != expected:
        errors.append(f"{path.relative_to(REPO_ROOT)}: root version {root_version} != {expected}")
    if package_root_version != expected:
        errors.append(
            f"{path.relative_to(REPO_ROOT)}: packages[''].version {package_root_version} != {expected}"
        )


def check_rust_crate_versions(errors: list[str]) -> None:
    for path in sorted((REPO_ROOT / "engines/crates").glob("*/Cargo.toml")):
        text = read_text(path, errors)
        if text is not None and "version.workspace" in text:
            errors.append(f"{path.relative_to(REPO_ROOT)}: version.workspace is not allowed")
        version = read_cargo_package_version(path, errors)
        if version is not None and not SEMVER_RE.fullmatch(version):
            errors.append(f"{path.relative_to(REPO_ROOT)}: version is not SemVer: {version}")


def check_dataset_contract_versions(errors: list[str]) -> None:
    dataset_paths = sorted((REPO_ROOT / "pipeline/contracts/datasets").glob("*.yml"))
    if not dataset_paths:
        errors.append("pipeline/contracts/datasets: no dataset contracts found")
        return
    for path in dataset_paths:
        text = read_text(path, errors)
        if text is None:
            continue
        match = DATASET_VERSION_RE.search(text)
        if match is None:
            errors.append(f"{path.relative_to(REPO_ROOT)}: missing integer version")


def check_release_manifest(component_versions: dict[str, str], errors: list[str]) -> None:
    path = REPO_ROOT / "deploy/release-manifest.yml"
    manifest = read_simple_yaml_manifest(path, errors)
    components = manifest.get("components")
    if not isinstance(components, dict):
        errors.append("deploy/release-manifest.yml: missing components map")
        return
    if "pipeline" in components:
        errors.append("deploy/release-manifest.yml: components must not include pipeline root")
    missing = sorted(set(EXPECTED_MANIFEST_COMPONENTS) - set(components))
    if missing:
        errors.append(f"deploy/release-manifest.yml: missing components: {', '.join(missing)}")
    for component, source_version in sorted(component_versions.items()):
        manifest_version = components.get(component)
        if manifest_version != source_version:
            errors.append(
                "deploy/release-manifest.yml: "
                f"components.{component} {manifest_version} != source {source_version}"
            )
    database_heads = manifest.get("database_heads")
    if not isinstance(database_heads, dict):
        errors.append("deploy/release-manifest.yml: missing database_heads map")
    else:
        for target in ("pipeline", "rearview"):
            if target not in database_heads:
                errors.append(f"deploy/release-manifest.yml: missing database_heads.{target}")
    contracts = manifest.get("contracts")
    if not isinstance(contracts, dict):
        errors.append("deploy/release-manifest.yml: missing contracts map")
    elif "registry_commit" not in contracts or "changed_datasets" not in contracts:
        errors.append(
            "deploy/release-manifest.yml: contracts requires registry_commit and changed_datasets"
        )


def check_markdown_links(path: Path, errors: list[str]) -> None:
    text = read_text(path, errors)
    if text is None:
        return
    for match in MARKDOWN_LINK_RE.finditer(text):
        target = match.group(1)
        if target.startswith(("http://", "https://", "mailto:", "#")):
            continue
        target_path = target.split("#", 1)[0]
        if not target_path:
            continue
        resolved = (path.parent / target_path).resolve()
        if not resolved.exists():
            errors.append(f"{path.relative_to(REPO_ROOT)}: broken link: {target}")


def read_simple_yaml_manifest(path: Path, errors: list[str]) -> dict[str, Any]:
    text = read_text(path, errors)
    if text is None:
        return {}
    manifest: dict[str, Any] = {}
    current_section: str | None = None
    for raw_line in text.splitlines():
        line = raw_line.rstrip()
        if not line or line.lstrip().startswith("#"):
            continue
        if not line.startswith(" "):
            key, value = split_yaml_key_value(line, path, errors)
            if key is None:
                continue
            if value == "":
                manifest[key] = {}
                current_section = key
            else:
                manifest[key] = parse_yaml_scalar(value)
                current_section = None
            continue
        if current_section is None:
            errors.append(f"{path.relative_to(REPO_ROOT)}: unexpected nested line: {line}")
            continue
        section = manifest[current_section]
        if not isinstance(section, dict):
            errors.append(f"{path.relative_to(REPO_ROOT)}: section is not a map: {current_section}")
            continue
        key, value = split_yaml_key_value(line.strip(), path, errors)
        if key is None:
            continue
        section[key] = parse_yaml_scalar(value)
    return manifest


def split_yaml_key_value(
    line: str,
    path: Path,
    errors: list[str],
) -> tuple[str | None, str]:
    if ":" not in line:
        errors.append(f"{path.relative_to(REPO_ROOT)}: invalid yaml line: {line}")
        return None, ""
    key, value = line.split(":", 1)
    return key.strip(), value.strip()


def parse_yaml_scalar(value: str) -> Any:
    if value == "[]":
        return []
    if (value.startswith('"') and value.endswith('"')) or (
        value.startswith("'") and value.endswith("'")
    ):
        return value[1:-1]
    return value


def read_toml(path: Path, errors: list[str]) -> dict[str, Any]:
    text = read_text(path, errors)
    if text is None:
        return {}
    return tomllib.loads(text)


def read_json(path: Path, errors: list[str]) -> dict[str, Any]:
    text = read_text(path, errors)
    if text is None:
        return {}
    data = json.loads(text)
    if not isinstance(data, dict):
        errors.append(f"{path.relative_to(REPO_ROOT)}: root JSON value must be an object")
        return {}
    return data


def read_text(path: Path, errors: list[str]) -> str | None:
    if not path.exists():
        errors.append(f"{path.relative_to(REPO_ROOT)}: file does not exist")
        return None
    return path.read_text(encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
