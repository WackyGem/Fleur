from __future__ import annotations

import json
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from datetime import date
from typing import Literal

import pyarrow as pa

from scheduler.defs.eastmoney.fields import EASTMONEY_FIELD_NAMES

ApiFamily = Literal["data_get", "data_v1_get"]

REQUEST_FIELD_NAMES = (
    "request_code",
    "request_start_date",
    "request_end_date",
    "partition_year",
    "source_endpoint",
    "ingested_at",
)


@dataclass(frozen=True)
class EastmoneyEndpointConfig:
    asset_name: str
    api_family: ApiFamily
    date_field: str
    sort_fields: tuple[str, ...]
    sort_directions: tuple[str, ...]
    page_size: int
    fixed_params: Mapping[str, str]

    @property
    def source_endpoint(self) -> str:
        if self.api_family == "data_get":
            return "https://datacenter.eastmoney.com/securities/api/data/get"
        if self.api_family == "data_v1_get":
            return "https://datacenter.eastmoney.com/securities/api/data/v1/get"
        msg = f"Unsupported EastMoney API family: {self.api_family}"
        raise ValueError(msg)


@dataclass(frozen=True)
class EastmoneyFetchedRow:
    data: Mapping[str, object]
    request_code: str
    request_start_date: date
    request_end_date: date


@dataclass(frozen=True)
class EastmoneyTableResult:
    table: pa.Table
    unknown_field_count: int


ENDPOINT_CONFIGS: tuple[EastmoneyEndpointConfig, ...] = (
    EastmoneyEndpointConfig(
        asset_name="eastmoney__balance",
        api_family="data_get",
        date_field="NOTICE_DATE",
        sort_fields=("REPORT_DATE", "SECURITY_CODE"),
        sort_directions=("-1", "-1"),
        page_size=500,
        fixed_params={
            "type": "RPT_F10_FINANCE_GBALANCE",
            "sty": "F10_FINANCE_GBALANCE",
        },
    ),
    EastmoneyEndpointConfig(
        asset_name="eastmoney__cashflow_sq",
        api_family="data_get",
        date_field="NOTICE_DATE",
        sort_fields=("REPORT_DATE", "SECURITY_CODE"),
        sort_directions=("-1", "-1"),
        page_size=500,
        fixed_params={
            "type": "RPT_F10_FINANCE_GCASHFLOWQC",
            "sty": "PC_F10_GCASHFLOWQC",
        },
    ),
    EastmoneyEndpointConfig(
        asset_name="eastmoney__cashflow_ytd",
        api_family="data_get",
        date_field="NOTICE_DATE",
        sort_fields=("REPORT_DATE", "SECURITY_CODE"),
        sort_directions=("-1", "-1"),
        page_size=500,
        fixed_params={
            "type": "RPT_F10_FINANCE_GCASHFLOW",
            "sty": "APP_F10_GCASHFLOW",
        },
    ),
    EastmoneyEndpointConfig(
        asset_name="eastmoney__dividend_allotment",
        api_family="data_v1_get",
        date_field="NOTICE_DATE",
        sort_fields=("NOTICE_DATE", "SECURITY_CODE"),
        sort_directions=("-1", "-1"),
        page_size=500,
        fixed_params={
            "reportName": "RPT_F10_DIVIDEND_ALLOTMENT",
            "columns": "ALL",
        },
    ),
    EastmoneyEndpointConfig(
        asset_name="eastmoney__dividend_main",
        api_family="data_v1_get",
        date_field="NOTICE_DATE",
        sort_fields=("NOTICE_DATE", "SECURITY_CODE"),
        sort_directions=("-1", "-1"),
        page_size=500,
        fixed_params={
            "reportName": "RPT_F10_DIVIDEND_MAIN",
            "columns": "ALL",
        },
    ),
    EastmoneyEndpointConfig(
        asset_name="eastmoney__equity_history",
        api_family="data_v1_get",
        date_field="NOTICE_DATE",
        sort_fields=("NOTICE_DATE", "SECURITY_CODE"),
        sort_directions=("-1", "-1"),
        page_size=500,
        fixed_params={
            "reportName": "RPT_F10_EH_EQUITY",
            "columns": "ALL",
        },
    ),
    EastmoneyEndpointConfig(
        asset_name="eastmoney__income_sq",
        api_family="data_get",
        date_field="NOTICE_DATE",
        sort_fields=("REPORT_DATE", "SECURITY_CODE"),
        sort_directions=("-1", "-1"),
        page_size=500,
        fixed_params={
            "type": "RPT_F10_FINANCE_GINCOMEQC",
            "sty": "PC_F10_GINCOMEQC",
        },
    ),
    EastmoneyEndpointConfig(
        asset_name="eastmoney__income_ytd",
        api_family="data_get",
        date_field="NOTICE_DATE",
        sort_fields=("REPORT_DATE", "SECURITY_CODE"),
        sort_directions=("-1", "-1"),
        page_size=500,
        fixed_params={
            "type": "RPT_F10_FINANCE_GINCOME",
            "sty": "APP_F10_GINCOME",
        },
    ),
)


def endpoint_by_asset_name(asset_name: str) -> EastmoneyEndpointConfig:
    for endpoint in ENDPOINT_CONFIGS:
        if endpoint.asset_name == asset_name:
            return endpoint
    msg = f"Unknown EastMoney asset: {asset_name}"
    raise KeyError(msg)


def eastmoney_business_field_names(asset_name: str) -> tuple[str, ...]:
    if asset_name not in EASTMONEY_FIELD_NAMES:
        msg = f"EastMoney schema fields are not configured for asset: {asset_name}"
        raise KeyError(msg)
    field_names = EASTMONEY_FIELD_NAMES[asset_name]
    if not field_names:
        msg = f"EastMoney schema fields are empty for asset: {asset_name}"
        raise ValueError(msg)
    if len(set(field_names)) != len(field_names):
        msg = f"EastMoney schema fields contain duplicates for asset: {asset_name}"
        raise ValueError(msg)
    return field_names


def eastmoney_schema(endpoint: EastmoneyEndpointConfig) -> pa.Schema:
    field_names = (*eastmoney_business_field_names(endpoint.asset_name), *REQUEST_FIELD_NAMES)
    return pa.schema((field_name, pa.string()) for field_name in field_names)


def eastmoney_rows_to_table(
    endpoint: EastmoneyEndpointConfig,
    rows: Sequence[EastmoneyFetchedRow],
    *,
    partition_year: str,
    ingested_at: str,
) -> EastmoneyTableResult:
    business_field_names = eastmoney_business_field_names(endpoint.asset_name)
    business_field_set = set(business_field_names)
    columns: dict[str, list[str | None]] = {
        field_name: [] for field_name in (*business_field_names, *REQUEST_FIELD_NAMES)
    }
    unknown_field_count = 0

    for row in rows:
        unknown_field_count += len(set(row.data) - business_field_set)
        for field_name in business_field_names:
            columns[field_name].append(_stringify_value(row.data.get(field_name)))
        columns["request_code"].append(row.request_code)
        columns["request_start_date"].append(row.request_start_date.isoformat())
        columns["request_end_date"].append(row.request_end_date.isoformat())
        columns["partition_year"].append(partition_year)
        columns["source_endpoint"].append(endpoint.source_endpoint)
        columns["ingested_at"].append(ingested_at)

    return EastmoneyTableResult(
        table=pa.table(columns, schema=eastmoney_schema(endpoint)),
        unknown_field_count=unknown_field_count,
    )


def empty_eastmoney_table(endpoint: EastmoneyEndpointConfig) -> pa.Table:
    schema = eastmoney_schema(endpoint)
    return pa.table({field.name: [] for field in schema}, schema=schema)


def _stringify_value(value: object) -> str | None:
    if value is None:
        return None
    if isinstance(value, str):
        return value
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int | float):
        return str(value)
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))

