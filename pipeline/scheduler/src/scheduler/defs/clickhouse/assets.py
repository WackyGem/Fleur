from __future__ import annotations

from collections.abc import Mapping

import dagster as dg

from scheduler.defs.asset_contracts import DEFAULT_OWNER
from scheduler.defs.baostock.assets import year_partitions
from scheduler.defs.clickhouse import sql
from scheduler.defs.clickhouse.raw_sync import RawSyncRequest, RawSyncService
from scheduler.defs.clickhouse.specs import (
    CLICKHOUSE_RAW_GROUP,
    ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
    ClickHouseRawTableSpec,
    clickhouse_raw_pool_name,
)
from scheduler.defs.common.metadata import RawMetadataValue
from scheduler.defs.resources.clickhouse import ClickHouseResource
from scheduler.defs.resources.s3 import S3SettingsResource


def build_clickhouse_raw_asset(spec: ClickHouseRawTableSpec) -> dg.AssetsDefinition:
    def materialize_clickhouse_raw_asset(
        context,
        clickhouse: ClickHouseResource,
        s3_settings: S3SettingsResource,
    ) -> dg.MaterializeResult:
        partition_key = context.partition_key if spec.partition_strategy == "year" else None
        client = clickhouse.client()
        try:
            result = RawSyncService(client).sync(
                RawSyncRequest(
                    spec=spec,
                    s3_input=sql.ClickHouseS3InputConfig.from_s3_config(
                        s3_settings.config(),
                        endpoint_override=s3_settings.clickhouse_endpoint,
                    ),
                    partition_key=partition_key,
                )
            )
        finally:
            client.close()
        return dg.MaterializeResult(metadata=result.metadata())

    materialize_clickhouse_raw_asset.__name__ = _asset_op_name(spec)
    return dg.asset(
        key=spec.raw_asset_key,
        group_name=CLICKHOUSE_RAW_GROUP,
        deps=[spec.source_asset_key],
        partitions_def=_partitions_def_for_spec(spec),
        backfill_policy=_backfill_policy_for_spec(spec),
        metadata=_metadata_for_spec(spec),
        owners=[DEFAULT_OWNER],
        kinds={"clickhouse", "raw"},
        pool=clickhouse_raw_pool_name(spec.raw_asset_table_name),
        tags=_tags_for_spec(spec),
        automation_condition=dg.AutomationCondition.eager(),
    )(materialize_clickhouse_raw_asset)


def _asset_op_name(spec: ClickHouseRawTableSpec) -> str:
    return f"clickhouse__raw__{spec.raw_asset_table_name}"


def _partitions_def_for_spec(
    spec: ClickHouseRawTableSpec,
) -> dg.PartitionsDefinition | None:
    if spec.partition_strategy != "year":
        return None
    return year_partitions


def _backfill_policy_for_spec(spec: ClickHouseRawTableSpec) -> dg.BackfillPolicy | None:
    if spec.partition_strategy != "year":
        return None
    return dg.BackfillPolicy.multi_run(max_partitions_per_run=1)


def _metadata_for_spec(spec: ClickHouseRawTableSpec) -> Mapping[str, RawMetadataValue]:
    return {
        "contract_dataset": spec.contract_dataset,
        "contract_version": spec.contract_version,
        "contract_schema_hash": spec.contract_schema_hash,
        "source_schema_hash": spec.source_schema_hash,
        "clickhouse_schema_hash": spec.clickhouse_schema_hash,
        "storage_mode": spec.storage_mode,
        "partition_key_name": spec.source_partition_key_name,
        "clickhouse_database": spec.clickhouse_database,
        "clickhouse_table": spec.clickhouse_table,
        "staging_table": spec.staging_table,
        "partition_strategy": spec.partition_strategy,
    }


def _tags_for_spec(spec: ClickHouseRawTableSpec) -> dict[str, str]:
    source_name = spec.source_asset_key.path[-1].split("__", maxsplit=1)[0]
    return {
        "source": source_name,
        "layer": "raw",
        "storage": "clickhouse",
    }


CLICKHOUSE_RAW_ASSETS: tuple[dg.AssetsDefinition, ...] = tuple(
    build_clickhouse_raw_asset(spec) for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
)
