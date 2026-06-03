from __future__ import annotations

from scheduler.defs.clickhouse import sql
from scheduler.defs.clickhouse.specs import (
    BAOSTOCK_DAILY_K_SPEC,
    ENABLED_CLICKHOUSE_RAW_TABLE_SPECS,
)
from scheduler.defs.config.models import S3Config


def test_raw_table_ddl_uses_merge_tree_year_partition_and_order_by() -> None:
    ddl = sql.render_create_raw_table_sql(BAOSTOCK_DAILY_K_SPEC)

    assert (
        "CREATE TABLE IF NOT EXISTS `fleur_raw`.`baostock__query_history_k_data_plus_daily`"
    ) in ddl
    assert "`year` UInt16" in ddl
    assert "ENGINE = MergeTree" in ddl
    assert "PARTITION BY `year`" in ddl
    assert "ORDER BY (`code`, `date`)" in ddl


def test_staging_insert_reads_s3_parquet_and_injects_year_partition() -> None:
    insert_sql = sql.render_insert_stage_from_s3_sql(
        BAOSTOCK_DAILY_K_SPEC,
        s3_input=sql.ClickHouseS3InputConfig(
            endpoint="http://127.0.0.1:9000",
            bucket="bucket",
            access_key="access",
            secret_key="secret",
        ),
        object_key=("source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet"),
        partition_key="2026",
    )

    assert (
        "INSERT INTO `fleur_raw`.`baostock__query_history_k_data_plus_daily__stage`"
    ) in insert_sql
    assert "FROM s3(" in insert_sql
    assert "'http://127.0.0.1:9000/bucket/source/baostock__query_history_k_data_plus_daily" in (
        insert_sql
    )
    assert "toUInt16(2026) AS `year`" in insert_sql
    assert "FORMAT" not in insert_sql


def test_s3_structure_preserves_nullable_date_types() -> None:
    spec = next(
        spec
        for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
        if spec.contract_dataset == "baostock__query_stock_basic"
    )

    structure = sql.render_s3_structure(spec)

    assert "outDate Nullable(Date)" in structure


def test_modify_column_sql_targets_raw_table_and_expected_type() -> None:
    spec = next(
        spec
        for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
        if spec.contract_dataset == "baostock__query_stock_basic"
    )

    alter_sql = sql.render_modify_column_sql(
        spec,
        column_name="outDate",
        clickhouse_type="Nullable(Date)",
    )

    assert alter_sql == (
        "ALTER TABLE `fleur_raw`.`baostock__query_stock_basic` "
        "MODIFY COLUMN `outDate` Nullable(Date)"
    )


def test_staging_insert_keeps_null_as_default_setting_but_uses_nullable_structure() -> None:
    spec = next(
        spec
        for spec in ENABLED_CLICKHOUSE_RAW_TABLE_SPECS
        if spec.contract_dataset == "jiuyan__action_field_compacted"
    )

    insert_sql = sql.render_insert_stage_from_s3_sql(
        spec,
        s3_input=sql.ClickHouseS3InputConfig(
            endpoint="http://127.0.0.1:9000",
            bucket="bucket",
            access_key="access",
            secret_key="secret",
        ),
        object_key="source/jiuyan__action_field_compacted/year=2026/000000_0.parquet",
        partition_key="2026",
    )

    assert "delete_time Nullable(DateTime64(3))" in insert_sql
    assert "update_time Nullable(DateTime64(3))" in insert_sql
    assert "SETTINGS input_format_null_as_default = 1" in insert_sql


def test_replace_partition_sql_does_not_drop_or_mutate_rows() -> None:
    replace_sql = sql.render_replace_partition_sql(
        BAOSTOCK_DAILY_K_SPEC,
        partition_key="2026",
    )

    assert replace_sql == (
        "ALTER TABLE `fleur_raw`.`baostock__query_history_k_data_plus_daily` "
        "REPLACE PARTITION 2026 "
        "FROM `fleur_raw`.`baostock__query_history_k_data_plus_daily__stage`"
    )
    assert "DROP PARTITION" not in replace_sql
    assert " DELETE " not in replace_sql
    assert " UPDATE " not in replace_sql


def test_s3_input_config_renders_path_style_url_without_exposing_secret_in_url() -> None:
    s3_input = sql.ClickHouseS3InputConfig(
        endpoint="http://rustfs.test/",
        bucket="/mono-fleur/",
        access_key="access",
        secret_key="secret",
    )

    assert s3_input.object_url("source/a/000000_0.parquet") == (
        "http://rustfs.test/mono-fleur/source/a/000000_0.parquet"
    )
    assert "secret" not in s3_input.redacted_object_url("source/a/000000_0.parquet")
    assert "secret" not in repr(s3_input)


def test_s3_input_config_supports_clickhouse_endpoint_override() -> None:
    s3_input = sql.ClickHouseS3InputConfig.from_s3_config(
        S3Config(
            endpoint="http://127.0.0.1:34050",
            bucket="mono-fleur",
            access_key="access",
            secret_key="secret",
        ),
        endpoint_override="http://rustfs:9000",
    )

    assert s3_input.object_url("source/a/000000_0.parquet") == (
        "http://rustfs:9000/mono-fleur/source/a/000000_0.parquet"
    )
