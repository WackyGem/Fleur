from __future__ import annotations

import argparse
import sys
import tomllib
from importlib import metadata
from pathlib import Path

from fleur_contracts.clickhouse_layer_migration import (
    DEFAULT_MAX_CONCURRENT_RAW_SYNC_DATASETS,
    render_migration_report_skeleton,
    run_baseline,
    run_migrate,
    run_reset,
    validate_dbt_layer,
    validate_empty,
    validate_raw,
)
from fleur_contracts.generate import generate_outputs
from fleur_contracts.loader import DEFAULT_CONTRACT_ROOT
from fleur_contracts.validate import validate_contracts
from fleur_contracts.validate_clickhouse import validate_available_clickhouse
from fleur_contracts.validate_parquet import validate_available_parquet

PACKAGE_NAME = "contract-tools"
SOURCE_PYPROJECT = Path(__file__).resolve().parents[2] / "pyproject.toml"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="fleur-contracts")
    parser.add_argument(
        "--version",
        action="version",
        version=f"%(prog)s {_package_version()}",
    )
    parser.add_argument(
        "--contract-root",
        type=Path,
        default=DEFAULT_CONTRACT_ROOT,
        help="Path to pipeline/contracts.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("validate")
    generate_parser = subparsers.add_parser("generate")
    generate_parser.add_argument("--check", action="store_true")
    parquet_parser = subparsers.add_parser("validate-parquet")
    parquet_parser.add_argument("--all-available", action="store_true")
    clickhouse_parser = subparsers.add_parser("validate-clickhouse")
    clickhouse_parser.add_argument("--all-available", action="store_true")
    clickhouse_layer_parser = subparsers.add_parser("clickhouse-layer")
    clickhouse_layer_subparsers = clickhouse_layer_parser.add_subparsers(
        dest="clickhouse_layer_command",
        required=True,
    )
    clickhouse_layer_subparsers.add_parser("baseline")
    reset_parser = clickhouse_layer_subparsers.add_parser("reset")
    reset_parser.add_argument("--manifest", type=Path, required=True)
    reset_parser.add_argument("--confirm", required=True)
    clickhouse_layer_subparsers.add_parser("validate-empty")
    validate_raw_parser = clickhouse_layer_subparsers.add_parser("validate-raw")
    validate_raw_parser.add_argument("--manifest", type=Path)
    clickhouse_layer_subparsers.add_parser("validate-dbt")
    clickhouse_layer_subparsers.add_parser("report")
    migrate_parser = clickhouse_layer_subparsers.add_parser("migrate")
    migrate_parser.add_argument("--manifest", type=Path, required=True)
    migrate_parser.add_argument("--confirm", required=True)
    migrate_parser.add_argument(
        "--max-concurrent-raw-sync-datasets",
        type=int,
        default=DEFAULT_MAX_CONCURRENT_RAW_SYNC_DATASETS,
        help=(
            "Maximum ClickHouse raw datasets to sync concurrently. "
            "Partitions within the same dataset remain serial."
        ),
    )

    args = parser.parse_args(argv)
    if args.command == "validate":
        try:
            count = validate_contracts(args.contract_root)
        except ValueError as error:
            print(str(error), file=sys.stderr)
            return 1
        print(f"Validated {count} dataset contracts.")
        return 0
    if args.command == "generate":
        changed = generate_outputs(contract_root=args.contract_root, check=args.check)
        if args.check and changed:
            for path in changed:
                print(f"Generated output is stale: {path}", file=sys.stderr)
            return 1
        print(f"Generated outputs are current for {len(changed)} changed files.")
        return 0
    if args.command == "validate-parquet":
        count = validate_available_parquet(args.contract_root)
        print(f"Parquet validator loaded {count} dataset contracts.")
        return 0
    if args.command == "validate-clickhouse":
        count = validate_available_clickhouse(args.contract_root)
        print(f"ClickHouse validator loaded {count} dataset contracts.")
        return 0
    if args.command == "clickhouse-layer":
        return _run_clickhouse_layer_command(args)
    parser.error(f"Unsupported command: {args.command}")
    return 2


def _run_clickhouse_layer_command(args: argparse.Namespace) -> int:
    if args.clickhouse_layer_command == "baseline":
        artifacts = run_baseline(contract_root=args.contract_root)
        print(f"Baseline report: {artifacts.baseline_report}")
        print(f"Partition manifest: {artifacts.partition_manifest}")
        print(f"Confirmation token: {artifacts.confirmation_token}")
        return 0
    if args.clickhouse_layer_command == "reset":
        report = run_reset(confirm=args.confirm, baseline_manifest_path=args.manifest)
        print(f"Reset report: {report}")
        return 0
    if args.clickhouse_layer_command == "validate-empty":
        remaining = validate_empty()
        if remaining:
            print(f"Layer databases still exist: {', '.join(remaining)}", file=sys.stderr)
            return 1
        print("Layer databases are absent.")
        return 0
    if args.clickhouse_layer_command == "validate-raw":
        issues = validate_raw(
            contract_root=args.contract_root,
            baseline_manifest_path=args.manifest,
        )
        if issues:
            for issue in issues:
                print(issue, file=sys.stderr)
            return 1
        print("fleur_raw raw table validation passed.")
        return 0
    if args.clickhouse_layer_command == "validate-dbt":
        issues = validate_dbt_layer()
        if issues:
            for issue in issues:
                print(issue, file=sys.stderr)
            return 1
        print("dbt layer database validation passed.")
        return 0
    if args.clickhouse_layer_command == "report":
        report = render_migration_report_skeleton()
        print(f"Migration report skeleton: {report}")
        return 0
    if args.clickhouse_layer_command == "migrate":
        report = run_migrate(
            confirm=args.confirm,
            baseline_manifest_path=args.manifest,
            contract_root=args.contract_root,
            max_concurrent_raw_sync_datasets=args.max_concurrent_raw_sync_datasets,
        )
        print(f"Migration report: {report}")
        return 0
    return 2


def _package_version() -> str:
    try:
        return metadata.version(PACKAGE_NAME)
    except metadata.PackageNotFoundError:
        if not SOURCE_PYPROJECT.exists():
            raise
        project = tomllib.loads(SOURCE_PYPROJECT.read_text(encoding="utf-8"))["project"]
        return str(project["version"])


if __name__ == "__main__":
    raise SystemExit(main())
