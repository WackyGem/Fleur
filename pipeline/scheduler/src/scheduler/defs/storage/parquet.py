from __future__ import annotations

from typing import Protocol

import pyarrow as pa
import pyarrow.dataset as ds
import pyarrow.parquet as pq

from scheduler.defs.storage.s3 import PyArrowFileSystem


class _WrittenFileLike(Protocol):
    path: str


def write_parquet_dataset(
    table: pa.Table,
    base_dir: str,
    filesystem: PyArrowFileSystem,
    *,
    partition_key: str | None = None,
    partition_key_name: str | None = None,
    allow_empty: bool = False,
) -> list[str]:
    if table.num_rows == 0 and not allow_empty:
        msg = "Refusing to write an empty pyarrow.Table"
        raise ValueError(msg)

    if partition_key_name is not None or partition_key is not None:
        if partition_key_name is None or partition_key is None:
            msg = "partition_key and partition_key_name must be provided together"
            raise ValueError(msg)
        base_dir = f"{base_dir.rstrip('/')}/{partition_key_name}={partition_key}"

    if table.num_rows == 0:
        filesystem.delete_dir_contents(base_dir, missing_dir_ok=True)
        filesystem.create_dir(base_dir, recursive=True)
        path = f"{base_dir}/000000_0.parquet"
        with filesystem.open_output_stream(path) as sink:
            pq.write_table(table, sink, compression="zstd")
        return [path]

    written_paths: list[str] = []

    def visit_file(written_file: _WrittenFileLike) -> None:
        written_paths.append(written_file.path)

    ds.write_dataset(
        table,
        base_dir=base_dir,
        filesystem=filesystem,
        format="parquet",
        basename_template="000000_{i}.parquet",
        existing_data_behavior="delete_matching",
        use_threads=True,
        max_rows_per_file=max(table.num_rows, 1),
        max_rows_per_group=max(table.num_rows, 1),
        file_visitor=visit_file,
    )

    extra_files = [
        path
        for path in written_paths
        if not path.endswith("/000000_0.parquet") and path != f"{base_dir}/000000_0.parquet"
    ]
    if extra_files:
        msg = f"PyArrow wrote unexpected parquet files: {extra_files}"
        raise RuntimeError(msg)

    return sorted(written_paths)
