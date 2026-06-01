from __future__ import annotations

from pathlib import Path

from fleur_contracts.description_quality import (
    format_description_quality_error,
    validate_description_quality,
)
from fleur_contracts.loader import DEFAULT_CONTRACT_ROOT, load_registry


def validate_contracts(contract_root: Path = DEFAULT_CONTRACT_ROOT) -> int:
    registry = load_registry(contract_root)
    description_issues = validate_description_quality(registry)
    if description_issues:
        raise ValueError(format_description_quality_error(description_issues))
    return len(registry.datasets)
