from __future__ import annotations

import re
from dataclasses import dataclass

from fleur_contracts.schema import ContractRegistry

CJK_RE = re.compile(r"[\u4e00-\u9fff]")
IDENTIFIER_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
PLACEHOLDER_RE = re.compile(r"^(TODO|TBD|UNKNOWN|N/?A|未知|待补充|待定)$", re.IGNORECASE)
UNKNOWN_FORMAT_RE = re.compile(r"^待核实：供应商字段 [A-Za-z_][A-Za-z0-9_]*，.+")


@dataclass(frozen=True)
class DescriptionQualityIssue:
    path: str
    reason: str
    value: str

    def format(self) -> str:
        return f"{self.path}: {self.reason}: {self.value!r}"


def validate_description_quality(registry: ContractRegistry) -> list[DescriptionQualityIssue]:
    issues: list[DescriptionQualityIssue] = []

    for dataset in registry.datasets:
        for field in dataset.source.fields:
            issues.extend(
                _check_description(
                    path=(
                        f"datasets/{dataset.dataset}.yml "
                        f"source.fields[{field.name}].external_description_zh"
                    ),
                    field_name=field.name,
                    value=field.external_description_zh,
                )
            )

    return issues


def format_description_quality_error(issues: list[DescriptionQualityIssue]) -> str:
    lines = ["Description quality failed:"]
    lines.extend(f"- {issue.format()}" for issue in issues)
    return "\n".join(lines)


def _check_description(
    *,
    path: str,
    field_name: str,
    value: str,
) -> list[DescriptionQualityIssue]:
    normalized = value.strip()
    if not normalized:
        return [DescriptionQualityIssue(path=path, reason="missing", value=value)]
    if PLACEHOLDER_RE.fullmatch(normalized):
        return [DescriptionQualityIssue(path=path, reason="known_placeholder", value=value)]
    if normalized.casefold() == field_name.casefold():
        return [DescriptionQualityIssue(path=path, reason="same_as_field_name", value=value)]
    if IDENTIFIER_RE.fullmatch(normalized):
        return [DescriptionQualityIssue(path=path, reason="identifier_only", value=value)]
    if not CJK_RE.search(normalized):
        return [DescriptionQualityIssue(path=path, reason="no_cjk", value=value)]
    if normalized.startswith("待核实："):
        if UNKNOWN_FORMAT_RE.fullmatch(normalized):
            return []
        return [DescriptionQualityIssue(path=path, reason="invalid_unknown_format", value=value)]
    if len(CJK_RE.findall(normalized)) < 3:
        return [DescriptionQualityIssue(path=path, reason="too_short", value=value)]
    return []
