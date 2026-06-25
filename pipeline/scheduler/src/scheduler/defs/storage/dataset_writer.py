from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field

import pyarrow as pa

from scheduler.defs.config.models import S3Config
from scheduler.defs.storage.parquet import write_parquet_dataset
from scheduler.defs.storage.s3 import PyArrowFileSystem, build_s3_filesystem


@dataclass(frozen=True)
class DatasetWriteResult:
    written_paths: list[str]
    row_count: int
    column_count: int
    partition_row_counts: dict[str, int] = field(default_factory=dict)

    @property
    def empty_partition_keys(self) -> list[str]:
        return sorted(
            partition_key
            for partition_key, row_count in self.partition_row_counts.items()
            if row_count == 0
        )

    def object_keys(self, bucket: str) -> list[str]:
        bucket_prefix = f"{bucket}/"
        return [
            path.removeprefix(bucket_prefix) if path.startswith(bucket_prefix) else path
            for path in self.written_paths
        ]


class DatasetPartitionWriteError(RuntimeError):
    def __init__(
        self,
        *,
        attempted_partition_keys: list[str],
        written_partition_keys: list[str],
        failed_partition_keys: list[str],
        written_paths: list[str],
        cause: BaseException,
    ) -> None:
        self.attempted_partition_keys = attempted_partition_keys
        self.written_partition_keys = written_partition_keys
        self.failed_partition_keys = failed_partition_keys
        self.written_paths = written_paths
        message = (
            "Partitioned dataset write failed; "
            f"attempted_partition_keys={attempted_partition_keys}; "
            f"written_partition_keys={written_partition_keys}; "
            f"failed_partition_keys={failed_partition_keys}"
        )
        super().__init__(message)
        self.__cause__ = cause


class S3DatasetWriter:
    def __init__(
        self,
        *,
        s3_config: S3Config,
        filesystem: PyArrowFileSystem | None = None,
    ) -> None:
        self.s3_config = s3_config
        self.filesystem = filesystem or build_s3_filesystem(s3_config)

    def write_latest_snapshot(
        self,
        *,
        table: pa.Table,
        base_dir: str,
        allow_empty: bool = False,
    ) -> DatasetWriteResult:
        written_paths = write_parquet_dataset(
            table,
            base_dir,
            self.filesystem,
            allow_empty=allow_empty,
        )
        return DatasetWriteResult(
            written_paths=written_paths,
            row_count=table.num_rows,
            column_count=table.num_columns,
        )

    def write_partitioned(
        self,
        *,
        partition_tables: Mapping[str, pa.Table],
        base_dir: str,
        partition_key_name: str,
        allow_empty: bool = False,
    ) -> DatasetWriteResult:
        written_paths: list[str] = []
        partition_row_counts: dict[str, int] = {}
        column_count = partition_column_count(partition_tables)
        attempted_partition_keys = sorted(partition_tables)
        for partition_key in attempted_partition_keys:
            partition_table = partition_tables[partition_key]
            try:
                written_paths.extend(
                    write_parquet_dataset(
                        partition_table,
                        base_dir,
                        self.filesystem,
                        partition_key=partition_key,
                        partition_key_name=partition_key_name,
                        allow_empty=allow_empty,
                    )
                )
            except Exception as error:
                written_partition_keys = sorted(partition_row_counts)
                failed_partition_keys = [
                    key for key in attempted_partition_keys if key not in partition_row_counts
                ]
                raise DatasetPartitionWriteError(
                    attempted_partition_keys=attempted_partition_keys,
                    written_partition_keys=written_partition_keys,
                    failed_partition_keys=failed_partition_keys,
                    written_paths=written_paths,
                    cause=error,
                ) from error
            partition_row_counts[partition_key] = partition_table.num_rows

        return DatasetWriteResult(
            written_paths=written_paths,
            row_count=sum(partition_row_counts.values()),
            column_count=column_count,
            partition_row_counts=partition_row_counts,
        )


def partition_column_count(partition_tables: Mapping[str, pa.Table]) -> int:
    first_table = next(iter(partition_tables.values()))
    first_columns = first_table.column_names
    for partition_key, table in partition_tables.items():
        if table.column_names != first_columns:
            msg = (
                f"Partition {partition_key!r} columns differ from first partition: "
                f"{table.column_names} != {first_columns}"
            )
            raise ValueError(msg)
    return first_table.num_columns
