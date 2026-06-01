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


def test_slack_environment_variables_are_owned_by_config_module() -> None:
    allowed = {DEFS_ROOT / "config" / "env.py"}
    prohibited_patterns = (
        'dg.EnvVar("SLACK_',
        'dg.EnvVar("DAGSTER_WEBSERVER_BASE_URL")',
        'dg.EnvVar("DAGSTER_CODE_LOCATION_NAME")',
    )

    for path in DEFS_ROOT.rglob("*.py"):
        if path in allowed:
            continue
        content = path.read_text(encoding="utf-8")
        for pattern in prohibited_patterns:
            assert pattern not in content, str(path)


def test_slack_sdk_usage_is_owned_by_slack_resource() -> None:
    allowed = {DEFS_ROOT / "resources" / "slack.py"}

    for path in DEFS_ROOT.rglob("*.py"):
        if path in allowed:
            continue
        content = path.read_text(encoding="utf-8")
        assert "dagster_slack" not in content, str(path)
        assert "slack_sdk" not in content, str(path)


def test_source_code_does_not_reference_slack_configuration() -> None:
    for source_root in (DEFS_ROOT / "sources", DEFS_ROOT / "baostock"):
        for path in source_root.rglob("*.py"):
            content = path.read_text(encoding="utf-8")
            assert "SLACK_" not in content, str(path)


def test_source_business_code_does_not_parse_contract_registry() -> None:
    for source_root in (DEFS_ROOT / "sources", DEFS_ROOT / "baostock"):
        for path in source_root.rglob("*.py"):
            content = path.read_text(encoding="utf-8")
            assert "fleur_contracts" not in content, str(path)
            assert "pipeline/contracts" not in content, str(path)
