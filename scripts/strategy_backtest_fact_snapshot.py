#!/usr/bin/env python3
"""Export stable ClickHouse fact snapshots for a strategy backtest attempt.

This helper is read-only. It queries portfolio fact tables for one
portfolio_run_id/result_attempt_id pair, removes volatile identifier columns,
sorts rows by business keys, and emits row counts, stable hashes, and compact
summaries suitable for before/after comparison.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from decimal import Decimal
from typing import Any

VOLATILE_COLUMNS = {
    "portfolio_run_id",
    "result_attempt_id",
    "portfolio_order_id",
    "portfolio_trade_id",
    "portfolio_event_id",
}


@dataclass(frozen=True)
class FactSpec:
    table: str
    order_by: tuple[str, ...]
    summary_fields: tuple[str, ...] = ()


FACTS = (
    FactSpec(
        "portfolio_nav_daily",
        ("trade_date",),
        ("trade_date", "nav", "total_equity", "daily_return", "drawdown"),
    ),
    FactSpec("portfolio_target", ("signal_date", "source_rank", "security_code")),
    FactSpec("portfolio_order", ("order_seq",)),
    FactSpec(
        "portfolio_trade",
        ("trade_seq",),
        ("side", "gross_amount", "total_fee", "slippage_cost"),
    ),
    FactSpec("portfolio_position_day", ("trade_date", "security_code")),
    FactSpec("portfolio_event", ("event_seq",), ("event_type",)),
)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create stable hashes for strategy backtest ClickHouse facts."
    )
    parser.add_argument("--portfolio-run-id", required=True)
    parser.add_argument("--result-attempt-id", required=True)
    parser.add_argument(
        "--database",
        default=os.environ.get("REARVIEW_CLICKHOUSE_PORTFOLIO_DATABASE", "fleur_portfolio"),
    )
    parser.add_argument(
        "--compose-file",
        default=os.environ.get("COMPOSE_FILE", "deploy/docker-compose.yml"),
    )
    parser.add_argument(
        "--env-file",
        default=os.environ.get("COMPOSE_ENV_FILE", ".env"),
    )
    parser.add_argument(
        "--service",
        default=os.environ.get("CLICKHOUSE_COMPOSE_SERVICE", "clickhouse"),
    )
    args = parser.parse_args()

    output: dict[str, Any] = {
        "portfolio_run_id": args.portfolio_run_id,
        "result_attempt_id": args.result_attempt_id,
        "database": args.database,
        "facts": {},
    }
    for spec in FACTS:
        rows = query_fact_rows(args, spec)
        stable_rows = [stable_row(row) for row in rows]
        stable_rows.sort(key=lambda row: tuple(row.get(key) for key in spec.order_by))
        output["facts"][spec.table] = {
            "row_count": len(stable_rows),
            "stable_hash": stable_hash(stable_rows),
            "summary": fact_summary(spec, stable_rows),
        }

    json.dump(output, sys.stdout, ensure_ascii=False, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


def query_fact_rows(args: argparse.Namespace, spec: FactSpec) -> list[dict[str, Any]]:
    query = f"""
SELECT *
FROM {quote_identifier(args.database)}.{quote_identifier(spec.table)}
WHERE portfolio_run_id = {quote_literal(args.portfolio_run_id)}
  AND result_attempt_id = {quote_literal(args.result_attempt_id)}
ORDER BY {", ".join(quote_identifier(column) for column in spec.order_by)}
FORMAT JSONEachRow
"""
    command = [
        "docker",
        "compose",
        "--env-file",
        args.env_file,
        "-f",
        args.compose_file,
        "exec",
        "-T",
        args.service,
        "clickhouse-client",
        "--query",
        query,
    ]
    result = subprocess.run(
        command,
        check=True,
        capture_output=True,
        text=True,
    )
    return [json.loads(line) for line in result.stdout.splitlines() if line.strip()]


def stable_row(row: dict[str, Any]) -> dict[str, Any]:
    return {
        key: normalize_value(value)
        for key, value in row.items()
        if key not in VOLATILE_COLUMNS
    }


def normalize_value(value: Any) -> Any:
    if isinstance(value, float):
        return format(Decimal(str(value)).normalize(), "f")
    if isinstance(value, list):
        return [normalize_value(item) for item in value]
    if isinstance(value, dict):
        return {key: normalize_value(value[key]) for key in sorted(value)}
    return value


def stable_hash(rows: list[dict[str, Any]]) -> str:
    payload = json.dumps(rows, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def fact_summary(spec: FactSpec, rows: list[dict[str, Any]]) -> dict[str, Any]:
    summary: dict[str, Any] = {}
    if not rows:
        return summary
    if spec.table == "portfolio_nav_daily":
        summary["last_row"] = {
            field: rows[-1].get(field)
            for field in spec.summary_fields
            if field in rows[-1]
        }
    if spec.table == "portfolio_trade":
        for field in ("gross_amount", "total_fee", "slippage_cost"):
            summary[f"{field}_sum"] = decimal_sum(rows, field)
    if spec.table == "portfolio_order":
        status_counts: dict[str, int] = {}
        for row in rows:
            status = str(row.get("status"))
            status_counts[status] = status_counts.get(status, 0) + 1
        summary["status_counts"] = dict(sorted(status_counts.items()))
    if spec.table == "portfolio_event":
        event_counts: dict[str, int] = {}
        for row in rows:
            event_type = str(row.get("event_type"))
            event_counts[event_type] = event_counts.get(event_type, 0) + 1
        summary["event_type_counts"] = dict(sorted(event_counts.items()))
    return summary


def decimal_sum(rows: list[dict[str, Any]], field: str) -> str:
    total = Decimal("0")
    for row in rows:
        value = row.get(field)
        if value is not None:
            total += Decimal(str(value))
    return format(total.normalize(), "f")


def quote_identifier(identifier: str) -> str:
    if not identifier.replace("_", "").isalnum():
        raise ValueError(f"invalid ClickHouse identifier: {identifier}")
    return f"`{identifier}`"


def quote_literal(value: str) -> str:
    return "'" + value.replace("\\", "\\\\").replace("'", "\\'") + "'"


if __name__ == "__main__":
    raise SystemExit(main())
