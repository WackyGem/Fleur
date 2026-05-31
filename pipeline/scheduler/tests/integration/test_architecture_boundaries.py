from __future__ import annotations

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFS_ROOT = REPO_ROOT / "scheduler" / "src" / "scheduler" / "defs"


def test_storage_package_does_not_import_source_definitions() -> None:
    for path in (DEFS_ROOT / "storage").rglob("*.py"):
        assert "scheduler.defs.sources" not in path.read_text(encoding="utf-8"), str(path)


def test_repositories_do_not_import_dagster() -> None:
    for path in (DEFS_ROOT / "repositories").rglob("*.py"):
        assert "import dagster" not in path.read_text(encoding="utf-8"), str(path)


def test_source_services_do_not_read_s3_env_or_parquet_helpers_directly() -> None:
    for source_root in (DEFS_ROOT / "sources", DEFS_ROOT / "baostock"):
        for path in source_root.rglob("*.py"):
            content = path.read_text(encoding="utf-8")
            assert "S3Config.from_env(" not in content, str(path)
            assert "scheduler.defs.storage.parquet_readers" not in content, str(path)


def test_source_code_uses_resources_for_generic_client_construction() -> None:
    allowed = {
        DEFS_ROOT / "resources" / "http.py",
        DEFS_ROOT / "resources" / "baostock.py",
        DEFS_ROOT / "http" / "client_factory.py",
    }
    for source_root in (DEFS_ROOT / "sources", DEFS_ROOT / "baostock"):
        for path in source_root.rglob("*.py"):
            if path in allowed:
                continue
            content = path.read_text(encoding="utf-8")
            assert "HttpClientFactory(" not in content, str(path)
            assert "BaostockAioTcpClient(" not in content, str(path)
            assert " AioHttpClient(" not in content, str(path)


def test_eastmoney_assets_do_not_encode_rate_limit_ordering_as_lineage() -> None:
    content = (DEFS_ROOT / "sources" / "eastmoney" / "assets.py").read_text(encoding="utf-8")

    assert "ordering_dependency" not in content
    assert "previous_asset" not in content
    assert "execution_ordering_dependency" not in content
