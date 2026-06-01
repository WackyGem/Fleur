from __future__ import annotations

import argparse
import sys
from pathlib import Path

from fleur_contracts.generate import generate_outputs
from fleur_contracts.loader import DEFAULT_CONTRACT_ROOT
from fleur_contracts.validate import validate_contracts
from fleur_contracts.validate_clickhouse import validate_available_clickhouse
from fleur_contracts.validate_parquet import validate_available_parquet


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="fleur-contracts")
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
    parser.error(f"Unsupported command: {args.command}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
