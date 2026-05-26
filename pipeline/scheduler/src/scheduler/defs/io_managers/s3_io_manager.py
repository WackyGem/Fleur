from __future__ import annotations

from typing import Any

import boto3
import dagster as dg
import pyarrow as pa
import pyarrow.parquet as pq
from botocore.config import Config
from botocore.exceptions import ClientError

PARQUET_CONTENT_TYPE = "application/vnd.apache.parquet"


def asset_key_to_parquet_object_key(
    asset_key: dg.AssetKey,
    object_prefix: str,
) -> str:
    asset_path = "/".join(asset_key.path)
    if object_prefix:
        return f"{object_prefix.strip('/')}/{asset_path}/000000_0.parquet"
    return f"{asset_path}/000000_0.parquet"


def table_to_parquet_bytes(table: pa.Table) -> bytes:
    sink = pa.BufferOutputStream()
    pq.write_table(table, sink, compression="zstd")
    return sink.getvalue().to_pybytes()


class S3IOManager(dg.ConfigurableIOManager):
    endpoint: str = dg.EnvVar("RUSTFS_ENDPOINT")
    bucket: str = dg.EnvVar("RUSTFS_BUCKET")
    access_key: str = dg.EnvVar("RUSTFS_ACCESS_KEY")
    secret_key: str = dg.EnvVar("RUSTFS_SECRET_KEY")
    region_name: str = "us-east-1"
    object_prefix: str = "raw"

    def handle_output(self, context: dg.OutputContext, obj: Any) -> None:
        table = self._validate_table(obj)
        parquet_bytes = table_to_parquet_bytes(table)
        key = self._object_key(context)
        bucket = self._bucket()
        endpoint = self._endpoint()
        client = self._client()
        self._ensure_bucket_exists(client, bucket)
        client.put_object(
            Bucket=bucket,
            Key=key,
            Body=parquet_bytes,
            ContentType=PARQUET_CONTENT_TYPE,
        )

        context.add_output_metadata(
            {
                "s3_bucket": bucket,
                "s3_key": key,
                "s3_endpoint": endpoint,
                "content_type": PARQUET_CONTENT_TYPE,
                "file_format": "parquet",
                "compression": "zstd",
                "row_count": table.num_rows,
                "column_count": table.num_columns,
            }
        )

    def load_input(self, context: dg.InputContext) -> Any:
        msg = "S3IOManager does not implement input loading yet"
        raise NotImplementedError(msg)

    def _client(self) -> Any:
        return boto3.client(
            "s3",
            endpoint_url=self._endpoint(),
            aws_access_key_id=self.access_key,
            aws_secret_access_key=self.secret_key,
            region_name=self.region_name,
            config=Config(s3={"addressing_style": "path"}),
        )

    def _bucket(self) -> str:
        return self.bucket

    def _endpoint(self) -> str:
        return self.endpoint

    def _ensure_bucket_exists(self, client: Any, bucket: str) -> None:
        try:
            client.head_bucket(Bucket=bucket)
        except ClientError as error:
            error_code = error.response.get("Error", {}).get("Code", "")
            if error_code not in {"404", "NoSuchBucket", "NotFound"}:
                raise
            client.create_bucket(Bucket=bucket)

    def _object_key(self, context: dg.OutputContext) -> str:
        if context.asset_key is None:
            msg = "S3IOManager requires an asset output"
            raise RuntimeError(msg)

        return asset_key_to_parquet_object_key(context.asset_key, self.object_prefix)

    def _validate_table(self, obj: Any) -> pa.Table:
        if not isinstance(obj, pa.Table):
            msg = "S3IOManager expected a pyarrow.Table"
            raise TypeError(msg)

        if obj.num_rows == 0:
            msg = "S3IOManager refuses to write an empty pyarrow.Table"
            raise ValueError(msg)

        return obj
