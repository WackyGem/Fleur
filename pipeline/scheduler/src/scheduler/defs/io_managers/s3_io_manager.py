from __future__ import annotations

from datetime import date
from typing import Any

import boto3
import dagster as dg
import pyarrow as pa
import pyarrow.parquet as pq
from botocore.config import Config
from botocore.exceptions import ClientError

SINA_TRADE_CALENDAR_KEY = "raw/sina__trade_calendar/000000_0.parquet"
PARQUET_CONTENT_TYPE = "application/vnd.apache.parquet"


class S3IOManager(dg.ConfigurableIOManager):
    endpoint: str = dg.EnvVar("RUSTFS_ENDPOINT")
    bucket: str = dg.EnvVar("RUSTFS_BUCKET")
    access_key: str = dg.EnvVar("RUSTFS_ACCESS_KEY")
    secret_key: str = dg.EnvVar("RUSTFS_SECRET_KEY")
    region_name: str = "us-east-1"

    def handle_output(self, context: dg.OutputContext, obj: Any) -> None:
        rows = self._validate_trade_calendar_rows(obj)
        parquet_bytes = trade_calendar_rows_to_parquet_bytes(rows)
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

        trade_dates = [row[0] for row in rows]
        context.add_output_metadata(
            {
                "s3_bucket": bucket,
                "s3_key": key,
                "s3_endpoint": endpoint,
                "row_count": len(rows),
                "min_trade_date": min(trade_dates),
                "max_trade_date": max(trade_dates),
                "content_type": PARQUET_CONTENT_TYPE,
                "file_format": "parquet",
                "compression": "zstd",
            }
        )

    def load_input(self, context: dg.InputContext) -> Any:
        msg = "S3IOManager does not implement input loading yet"
        raise NotImplementedError(msg)

    def _object_key(self, context: dg.OutputContext) -> str:
        if context.asset_key is None:
            msg = "S3IOManager requires an asset output"
            raise RuntimeError(msg)

        if context.asset_key.to_user_string() == "sina__trade_calendar":
            return SINA_TRADE_CALENDAR_KEY

        msg = f"No S3 object key configured for asset {context.asset_key.to_user_string()}"
        raise RuntimeError(msg)

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

    def _validate_trade_calendar_rows(self, obj: Any) -> list[list[str]]:
        if not isinstance(obj, list):
            msg = "S3IOManager expected a list of trade-date rows"
            raise TypeError(msg)

        rows: list[list[str]] = []
        for row in obj:
            if not isinstance(row, list) or len(row) != 1 or not isinstance(row[0], str):
                msg = "S3IOManager expected rows shaped like [['YYYY-MM-DD']]"
                raise TypeError(msg)
            date.fromisoformat(row[0])
            rows.append(row)

        if not rows:
            msg = "S3IOManager refuses to write an empty trade calendar"
            raise ValueError(msg)

        return rows


def trade_calendar_rows_to_parquet_bytes(rows: list[list[str]]) -> bytes:
    trade_dates = [date.fromisoformat(row[0]) for row in rows]
    schema = pa.schema([pa.field("trade_date", pa.date32())])
    table = pa.Table.from_arrays(
        [pa.array(trade_dates, type=pa.date32())],
        schema=schema,
    )
    sink = pa.BufferOutputStream()
    pq.write_table(table, sink, compression="zstd")
    return sink.getvalue().to_pybytes()
