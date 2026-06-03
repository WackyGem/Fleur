from __future__ import annotations


def effective_clickhouse_type(
    clickhouse_type: str,
    *,
    nullable: bool,
) -> str:
    """Return the physical ClickHouse type implied by contract nullable facts."""
    normalized = clickhouse_type.strip()
    if not nullable:
        return normalized
    if _is_nullable_type(normalized):
        return normalized
    low_cardinality_inner = _low_cardinality_inner_type(normalized)
    if low_cardinality_inner is not None:
        return f"LowCardinality(Nullable({low_cardinality_inner}))"
    return f"Nullable({normalized})"


def validate_clickhouse_type_nullability(
    *,
    field_name: str,
    clickhouse_type: str,
    nullable: bool,
) -> None:
    normalized = clickhouse_type.strip()
    if not normalized:
        msg = f"ClickHouse field {field_name!r} type must be non-empty"
        raise ValueError(msg)
    if _is_nullable_type(normalized) and not nullable:
        msg = f"ClickHouse field {field_name!r} uses Nullable(...) but nullable is false"
        raise ValueError(msg)
    low_cardinality_inner = _low_cardinality_inner_type(normalized)
    if low_cardinality_inner is not None and _is_nullable_type(low_cardinality_inner):
        msg = (
            f"ClickHouse field {field_name!r} must express nullability with "
            "nullable: true and a non-nullable LowCardinality inner type"
        )
        raise ValueError(msg)


def _is_nullable_type(clickhouse_type: str) -> bool:
    return clickhouse_type.startswith("Nullable(") and clickhouse_type.endswith(")")


def _low_cardinality_inner_type(clickhouse_type: str) -> str | None:
    prefix = "LowCardinality("
    if not clickhouse_type.startswith(prefix) or not clickhouse_type.endswith(")"):
        return None
    return clickhouse_type[len(prefix) : -1].strip()
