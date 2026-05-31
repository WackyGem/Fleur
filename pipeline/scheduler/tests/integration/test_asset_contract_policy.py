from __future__ import annotations

from pathlib import Path

from scheduler.defs.definitions import SOURCE_BUNDLES


def test_source_assets_have_contract_metadata_and_tags() -> None:
    for bundle in SOURCE_BUNDLES:
        for asset in bundle.assets:
            key = asset.key
            spec = asset.specs_by_key[key]
            metadata = spec.metadata
            tags = spec.tags
            kind_tags = {
                tag.removeprefix("dagster/kind/") for tag in tags if tag.startswith("dagster/kind/")
            }
            owners = spec.owners

            assert tags.get("source") == bundle.name
            assert owners, key.to_user_string()
            assert kind_tags, key.to_user_string()
            if tags.get("storage") == "s3":
                assert "s3" in kind_tags
            if tags.get("state") == "postgres":
                assert metadata.get("state_backend") == "postgres"
                assert metadata.get("object_store") == "s3"
            elif tags.get("storage") == "s3":
                assert "storage_mode" in metadata
                if metadata["storage_mode"] == "partitioned":
                    assert metadata.get("partition_key_name")


def test_http_package_does_not_import_source_definitions() -> None:
    http_root = Path("scheduler/src/scheduler/defs/http")
    for path in http_root.glob("*.py"):
        content = path.read_text(encoding="utf-8")
        assert "scheduler.defs.sources" not in content, str(path)


def test_eastmoney_assets_do_not_use_dynamic_global_exports() -> None:
    content = Path("scheduler/src/scheduler/defs/sources/eastmoney/assets.py").read_text(
        encoding="utf-8"
    )

    assert "globals(" not in content
