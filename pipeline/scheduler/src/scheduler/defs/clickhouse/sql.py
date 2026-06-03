from __future__ import annotations

from dataclasses import dataclass, field

from scheduler.defs.clickhouse.specs import ClickHouseRawTableSpec, validate_identifier
from scheduler.defs.config.models import S3Config

PARQUET_FORMAT = "Parquet"


@dataclass(frozen=True)
class ClickHouseS3InputConfig:
    endpoint: str
    bucket: str
    access_key: str
    secret_key: str = field(repr=False)

    @classmethod
    def from_s3_config(
        cls,
        config: S3Config,
        *,
        endpoint_override: str | None = None,
    ) -> ClickHouseS3InputConfig:
        return cls(
            endpoint=endpoint_override or config.endpoint,
            bucket=config.bucket,
            access_key=config.access_key,
            secret_key=config.secret_key,
        )

    def object_url(self, object_key: str) -> str:
        endpoint = self.endpoint.rstrip("/")
        bucket = self.bucket.strip("/")
        key = object_key.lstrip("/")
        return f"{endpoint}/{bucket}/{key}"

    def redacted_object_url(self, object_key: str) -> str:
        return self.object_url(object_key)


def quote_identifier(identifier: str) -> str:
    validate_identifier(identifier, field_name="identifier")
    return f"`{identifier}`"


def quote_table(database: str, table: str) -> str:
    return f"{quote_identifier(database)}.{quote_identifier(table)}"


def quote_string_literal(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace("'", "\\'")
    return f"'{escaped}'"


def render_create_database_sql(database: str) -> str:
    return f"CREATE DATABASE IF NOT EXISTS {quote_identifier(database)}"


def render_create_raw_table_sql(spec: ClickHouseRawTableSpec) -> str:
    column_definitions = ",\n    ".join(
        f"{quote_identifier(column.name)} {column.clickhouse_type}" for column in spec.table_columns
    )
    order_by = ", ".join(quote_identifier(column_name) for column_name in spec.order_by)
    partition_by = _render_partition_by(spec)

    return "\n".join(
        (
            f"CREATE TABLE IF NOT EXISTS {quote_table(spec.clickhouse_database, spec.clickhouse_table)}",
            "(",
            f"    {column_definitions}",
            ")",
            "ENGINE = MergeTree",
            partition_by,
            f"ORDER BY ({order_by})",
        )
    )


def render_drop_staging_table_sql(spec: ClickHouseRawTableSpec) -> str:
    return f"DROP TABLE IF EXISTS {quote_table(spec.clickhouse_database, spec.staging_table)}"


def render_create_staging_table_sql(spec: ClickHouseRawTableSpec) -> str:
    return (
        f"CREATE TABLE {quote_table(spec.clickhouse_database, spec.staging_table)} "
        f"AS {quote_table(spec.clickhouse_database, spec.clickhouse_table)}"
    )


def render_insert_stage_from_s3_sql(
    spec: ClickHouseRawTableSpec,
    *,
    s3_input: ClickHouseS3InputConfig,
    object_key: str,
    partition_key: str | None,
) -> str:
    target_columns = [column.name for column in spec.table_columns]
    source_selects = [quote_identifier(column.name) for column in spec.columns]
    if spec.partition_strategy == "year":
        if partition_key is None:
            msg = "partition_key is required for year-partitioned raw sync"
            raise ValueError(msg)
        source_selects.append(f"toUInt16({int(partition_key)}) AS {quote_identifier('year')}")

    target_column_sql = ", ".join(quote_identifier(column_name) for column_name in target_columns)
    select_sql = ",\n    ".join(source_selects)
    s3_sql = render_s3_table_function_sql(
        s3_input=s3_input,
        object_key=object_key,
        structure=render_s3_structure(spec),
    )
    return "\n".join(
        (
            f"INSERT INTO {quote_table(spec.clickhouse_database, spec.staging_table)}",
            f"({target_column_sql})",
            "SELECT",
            f"    {select_sql}",
            f"FROM {s3_sql}",
            "SETTINGS input_format_null_as_default = 1",
        )
    )


def render_s3_table_function_sql(
    *,
    s3_input: ClickHouseS3InputConfig,
    object_key: str,
    structure: str,
) -> str:
    return (
        "s3("
        f"{quote_string_literal(s3_input.object_url(object_key))}, "
        f"{quote_string_literal(s3_input.access_key)}, "
        f"{quote_string_literal(s3_input.secret_key)}, "
        f"{quote_string_literal(PARQUET_FORMAT)}, "
        f"{quote_string_literal(structure)}"
        ")"
    )


def render_s3_structure(spec: ClickHouseRawTableSpec) -> str:
    return ", ".join(f"{column.name} {column.clickhouse_type}" for column in spec.columns)


def render_schema_validation_query(spec: ClickHouseRawTableSpec) -> str:
    return render_column_types_query(
        database=spec.clickhouse_database,
        table=spec.staging_table,
    )


def render_raw_table_schema_query(spec: ClickHouseRawTableSpec) -> str:
    return render_column_types_query(
        database=spec.clickhouse_database,
        table=spec.clickhouse_table,
    )


def render_column_types_query(*, database: str, table: str) -> str:
    return "\n".join(
        (
            "SELECT name, type",
            "FROM system.columns",
            f"WHERE database = {quote_string_literal(database)}",
            f"  AND table = {quote_string_literal(table)}",
            "ORDER BY position",
        )
    )


def render_modify_column_sql(
    spec: ClickHouseRawTableSpec,
    *,
    column_name: str,
    clickhouse_type: str,
) -> str:
    return (
        f"ALTER TABLE {quote_table(spec.clickhouse_database, spec.clickhouse_table)} "
        f"MODIFY COLUMN {quote_identifier(column_name)} {clickhouse_type}"
    )


def render_year_partition_validation_query(spec: ClickHouseRawTableSpec) -> str:
    return (
        "SELECT count(), min("
        f"{quote_identifier('year')}), max({quote_identifier('year')}) "
        f"FROM {quote_table(spec.clickhouse_database, spec.staging_table)}"
    )


def render_low_cardinality_validation_query(
    spec: ClickHouseRawTableSpec,
    *,
    column_name: str,
) -> str:
    return (
        f"SELECT uniq({quote_identifier(column_name)}) "
        f"FROM {quote_table(spec.clickhouse_database, spec.staging_table)}"
    )


def render_replace_partition_sql(
    spec: ClickHouseRawTableSpec,
    *,
    partition_key: str,
) -> str:
    return (
        f"ALTER TABLE {quote_table(spec.clickhouse_database, spec.clickhouse_table)} "
        f"REPLACE PARTITION {int(partition_key)} "
        f"FROM {quote_table(spec.clickhouse_database, spec.staging_table)}"
    )


def render_snapshot_exchange_sql(spec: ClickHouseRawTableSpec) -> str:
    return (
        f"EXCHANGE TABLES {quote_table(spec.clickhouse_database, spec.clickhouse_table)} "
        f"AND {quote_table(spec.clickhouse_database, spec.staging_table)}"
    )


def render_raw_year_partition_count_query(
    spec: ClickHouseRawTableSpec,
    *,
    partition_key: str,
) -> str:
    return (
        f"SELECT count() FROM {quote_table(spec.clickhouse_database, spec.clickhouse_table)} "
        f"WHERE {quote_identifier('year')} = {int(partition_key)}"
    )


def render_snapshot_count_query(spec: ClickHouseRawTableSpec) -> str:
    return f"SELECT count() FROM {quote_table(spec.clickhouse_database, spec.clickhouse_table)}"


def render_staging_count_query(spec: ClickHouseRawTableSpec) -> str:
    return f"SELECT count() FROM {quote_table(spec.clickhouse_database, spec.staging_table)}"


def _render_partition_by(spec: ClickHouseRawTableSpec) -> str:
    if spec.partition_strategy == "year":
        return f"PARTITION BY {quote_identifier('year')}"
    return "PARTITION BY tuple()"
