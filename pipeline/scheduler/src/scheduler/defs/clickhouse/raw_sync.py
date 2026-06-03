from __future__ import annotations

import hashlib
import time
from dataclasses import dataclass

from scheduler.defs.clickhouse import sql
from scheduler.defs.clickhouse.protocols import ClickHouseClientProtocol, ClickHouseQueryResult
from scheduler.defs.clickhouse.specs import ClickHouseRawTableSpec
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.storage.s3 import asset_key_to_parquet_object_key


@dataclass(frozen=True)
class RawSyncRequest:
    spec: ClickHouseRawTableSpec
    s3_input: sql.ClickHouseS3InputConfig
    partition_key: str | None


@dataclass(frozen=True)
class RawSyncResult:
    spec: ClickHouseRawTableSpec
    storage_mode: str
    partition_key: str | None
    s3_object_key: str
    loaded_row_count: int
    raw_row_count_after_replace: int
    schema_hash: str
    clickhouse_insert_seconds: float
    clickhouse_validation_seconds: float
    clickhouse_replace_seconds: float

    def metadata(self) -> dict[str, RawMetadataValue]:
        return {
            "contract_dataset": self.spec.contract_dataset,
            "contract_version": self.spec.contract_version,
            "contract_schema_hash": self.spec.contract_schema_hash,
            "source_schema_hash": self.spec.source_schema_hash,
            "clickhouse_schema_hash": self.spec.clickhouse_schema_hash,
            "clickhouse_database": self.spec.clickhouse_database,
            "clickhouse_table": self.spec.clickhouse_table,
            "staging_table": self.spec.staging_table,
            "storage_mode": self.storage_mode,
            "partition_key_name": self.spec.source_partition_key_name,
            "partition_key": self.partition_key,
            "s3_object_key": self.s3_object_key,
            "loaded_row_count": self.loaded_row_count,
            "raw_row_count_after_replace": self.raw_row_count_after_replace,
            "schema_hash": self.schema_hash,
            "clickhouse_insert_seconds": self.clickhouse_insert_seconds,
            "clickhouse_validation_seconds": self.clickhouse_validation_seconds,
            "clickhouse_replace_seconds": self.clickhouse_replace_seconds,
        }


class RawSyncService:
    def __init__(self, client: ClickHouseClientProtocol) -> None:
        self._client = client

    def sync(self, request: RawSyncRequest) -> RawSyncResult:
        if request.spec.partition_strategy == "year":
            return self._sync_year_partition(request)
        return self._sync_snapshot(request)

    def _sync_year_partition(self, request: RawSyncRequest) -> RawSyncResult:
        partition_key = _validate_year_partition_key(request.partition_key)
        spec = request.spec
        object_key = asset_key_to_parquet_object_key(
            spec.source_asset_key,
            partition_key=partition_key,
            partition_key_name=spec.source_partition_key_name,
            storage_mode=spec.storage_mode,
        )

        self._prepare_staging(spec)
        insert_started_at = time.perf_counter()
        self._client.command(
            sql.render_insert_stage_from_s3_sql(
                spec,
                s3_input=request.s3_input,
                object_key=object_key,
                partition_key=partition_key,
            )
        )
        insert_seconds = elapsed_seconds(insert_started_at)

        validation_started_at = time.perf_counter()
        loaded_row_count = self._validate_schema_and_year_partition(
            spec,
            partition_key=partition_key,
        )
        self._validate_low_cardinality_columns(spec)
        validation_seconds = elapsed_seconds(validation_started_at)

        replace_started_at = time.perf_counter()
        self._client.command(
            sql.render_replace_partition_sql(
                spec,
                partition_key=partition_key,
            )
        )
        raw_row_count = _first_int(
            self._client.query(
                sql.render_raw_year_partition_count_query(
                    spec,
                    partition_key=partition_key,
                )
            )
        )
        self._drop_staging(spec)
        replace_seconds = elapsed_seconds(replace_started_at)

        return RawSyncResult(
            spec=spec,
            storage_mode=spec.storage_mode,
            partition_key=partition_key,
            s3_object_key=object_key,
            loaded_row_count=loaded_row_count,
            raw_row_count_after_replace=raw_row_count,
            schema_hash=spec.clickhouse_schema_hash,
            clickhouse_insert_seconds=insert_seconds,
            clickhouse_validation_seconds=validation_seconds,
            clickhouse_replace_seconds=replace_seconds,
        )

    def _sync_snapshot(self, request: RawSyncRequest) -> RawSyncResult:
        spec = request.spec
        object_key = asset_key_to_parquet_object_key(
            spec.source_asset_key,
            storage_mode=spec.storage_mode,
        )

        self._prepare_staging(spec)
        insert_started_at = time.perf_counter()
        self._client.command(
            sql.render_insert_stage_from_s3_sql(
                spec,
                s3_input=request.s3_input,
                object_key=object_key,
                partition_key=None,
            )
        )
        insert_seconds = elapsed_seconds(insert_started_at)

        validation_started_at = time.perf_counter()
        self._validate_schema(spec)
        loaded_row_count = _first_int(self._client.query(sql.render_staging_count_query(spec)))
        if loaded_row_count <= 0 and not spec.allow_empty:
            msg = f"Staging table {spec.staging_table} is empty"
            raise RuntimeError(msg)
        self._validate_low_cardinality_columns(spec)
        validation_seconds = elapsed_seconds(validation_started_at)

        replace_started_at = time.perf_counter()
        self._client.command(sql.render_snapshot_exchange_sql(spec))
        raw_row_count = _first_int(self._client.query(sql.render_snapshot_count_query(spec)))
        self._drop_staging(spec)
        replace_seconds = elapsed_seconds(replace_started_at)

        return RawSyncResult(
            spec=spec,
            storage_mode=spec.storage_mode,
            partition_key=None,
            s3_object_key=object_key,
            loaded_row_count=loaded_row_count,
            raw_row_count_after_replace=raw_row_count,
            schema_hash=spec.clickhouse_schema_hash,
            clickhouse_insert_seconds=insert_seconds,
            clickhouse_validation_seconds=validation_seconds,
            clickhouse_replace_seconds=replace_seconds,
        )

    def _prepare_staging(self, spec: ClickHouseRawTableSpec) -> None:
        self._client.command(sql.render_create_database_sql(spec.clickhouse_database))
        self._client.command(sql.render_create_raw_table_sql(spec))
        self._reconcile_raw_table_schema(spec)
        self._client.command(sql.render_drop_staging_table_sql(spec))
        self._client.command(sql.render_create_staging_table_sql(spec))

    def _drop_staging(self, spec: ClickHouseRawTableSpec) -> None:
        self._client.command(sql.render_drop_staging_table_sql(spec))

    def _validate_schema_and_year_partition(
        self,
        spec: ClickHouseRawTableSpec,
        *,
        partition_key: str,
    ) -> int:
        self._validate_schema(spec)
        row = _first_row(self._client.query(sql.render_year_partition_validation_query(spec)))
        if len(row) != 3:
            msg = f"Expected year partition validation to return 3 values, got {len(row)}"
            raise RuntimeError(msg)

        row_count = _to_int(row[0], field_name="loaded_row_count")
        min_year = _to_int(row[1], field_name="min_year")
        max_year = _to_int(row[2], field_name="max_year")
        expected_year = int(partition_key)
        if row_count <= 0 and not spec.allow_empty:
            msg = f"Staging table {spec.staging_table} is empty"
            raise RuntimeError(msg)
        if row_count == 0:
            return row_count
        if min_year != expected_year or max_year != expected_year:
            msg = (
                f"Staging table {spec.staging_table} contains partition range "
                f"{min_year}..{max_year}, expected {expected_year}"
            )
            raise RuntimeError(msg)
        return row_count

    def _validate_schema(self, spec: ClickHouseRawTableSpec) -> None:
        result = self._client.query(sql.render_schema_validation_query(spec))
        actual_types = {str(row[0]): str(row[1]) for row in result.result_rows}
        expected_types = {column.name: column.clickhouse_type for column in spec.table_columns}
        for column_name, expected_type in expected_types.items():
            actual_type = actual_types.get(column_name)
            if actual_type != expected_type:
                msg = (
                    f"Staging table {spec.staging_table} column {column_name!r} "
                    f"has type {actual_type!r}, expected {expected_type!r}"
                )
                raise RuntimeError(msg)

    def _reconcile_raw_table_schema(self, spec: ClickHouseRawTableSpec) -> None:
        result = self._client.query(sql.render_raw_table_schema_query(spec))
        actual_types = {str(row[0]): str(row[1]) for row in result.result_rows}
        if not actual_types:
            msg = f"Raw table {spec.clickhouse_database}.{spec.clickhouse_table} has no columns"
            raise RuntimeError(msg)

        for column in spec.table_columns:
            actual_type = actual_types.get(column.name)
            if actual_type is None:
                msg = (
                    f"Raw table {spec.clickhouse_table} is missing expected column {column.name!r}"
                )
                raise RuntimeError(msg)
            if actual_type == column.clickhouse_type:
                continue
            self._client.command(
                sql.render_modify_column_sql(
                    spec,
                    column_name=column.name,
                    clickhouse_type=column.clickhouse_type,
                )
            )

    def _validate_low_cardinality_columns(self, spec: ClickHouseRawTableSpec) -> None:
        for column in spec.low_cardinality_columns:
            unique_count = _first_int(
                self._client.query(
                    sql.render_low_cardinality_validation_query(
                        spec,
                        column_name=column.name,
                    )
                )
            )
            if unique_count > spec.low_cardinality_unique_limit:
                msg = (
                    f"Column {column.name!r} has {unique_count} unique values, "
                    f"above LowCardinality limit {spec.low_cardinality_unique_limit}"
                )
                raise RuntimeError(msg)


def elapsed_seconds(started_at: float) -> float:
    return time.perf_counter() - started_at


def schema_hash(spec: ClickHouseRawTableSpec) -> str:
    schema_text = "\n".join(
        f"{column.name}:{column.clickhouse_type}" for column in spec.table_columns
    )
    return hashlib.sha256(schema_text.encode("utf-8")).hexdigest()


def _validate_year_partition_key(partition_key: str | None) -> str:
    if partition_key is None:
        msg = "Year-partitioned raw sync requires a partition key"
        raise RuntimeError(msg)
    if len(partition_key) != 4 or not partition_key.isdigit():
        msg = f"Year partition key must be a four-digit year: {partition_key!r}"
        raise RuntimeError(msg)
    return partition_key


def _first_row(result: ClickHouseQueryResult) -> tuple[object, ...]:
    if not result.result_rows:
        msg = "ClickHouse query returned no rows"
        raise RuntimeError(msg)
    return tuple(result.result_rows[0])


def _first_int(result: ClickHouseQueryResult) -> int:
    row = _first_row(result)
    if not row:
        msg = "ClickHouse query returned an empty row"
        raise RuntimeError(msg)
    return _to_int(row[0], field_name="query result")


def _to_int(value: object, *, field_name: str) -> int:
    if isinstance(value, bool) or value is None:
        msg = f"ClickHouse {field_name} must be an integer-compatible value"
        raise RuntimeError(msg)
    if isinstance(value, int):
        return value
    if isinstance(value, float | str):
        return int(value)
    msg = f"ClickHouse {field_name} must be an integer-compatible value"
    raise RuntimeError(msg)
