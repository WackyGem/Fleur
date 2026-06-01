from __future__ import annotations

from pathlib import Path

from fleur_contracts.adapters.data_dict import render_data_dict_markdown
from fleur_contracts.adapters.dbt import render_sources_yaml, render_staging_yaml
from fleur_contracts.loader import DEFAULT_CONTRACT_ROOT, PIPELINE_ROOT, load_registry

REPO_ROOT = PIPELINE_ROOT.parent
ELT_MODELS_DIR = PIPELINE_ROOT / "elt" / "models"
DATA_DICT_DIR = REPO_ROOT / "docs" / "references" / "data_dict"


def generate_outputs(
    *,
    contract_root: Path = DEFAULT_CONTRACT_ROOT,
    check: bool = False,
) -> list[Path]:
    registry = load_registry(contract_root)
    rendered: dict[Path, str] = {
        ELT_MODELS_DIR / "sources.yml": render_sources_yaml(registry),
        ELT_MODELS_DIR / "staging" / "staging.yml": render_staging_yaml(registry),
    }
    for contract in registry.datasets:
        rendered[DATA_DICT_DIR / f"{contract.dataset}.md"] = render_data_dict_markdown(
            registry,
            contract,
        )

    changed = []
    for path, content in rendered.items():
        existing = path.read_text(encoding="utf-8") if path.exists() else None
        if existing == content:
            continue
        changed.append(path)
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
    return changed
