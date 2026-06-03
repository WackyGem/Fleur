from __future__ import annotations

import json
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from datetime import date
from typing import Any, Literal

import pyarrow as pa

from scheduler.defs.common.schema import typed_table
from scheduler.defs.common.types import SchemaTypeError, to_date32
from scheduler.defs.contract_schemas import PARQUET_SCHEMAS, SOURCE_FIELD_NAMES
from scheduler.defs.http.schemas import unknown_field_count_for_mapping

ApiFamily = Literal["data_get", "data_v1_get"]


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

EASTMONEY_SCHEMAS: dict[str, pa.Schema] = {
    endpoint.asset_name: PARQUET_SCHEMAS[endpoint.asset_name] for endpoint in ENDPOINT_CONFIGS
}


def endpoint_by_asset_name(asset_name: str) -> EastmoneyEndpointConfig:
    for endpoint in ENDPOINT_CONFIGS:
        if endpoint.asset_name == asset_name:
            return endpoint
    msg = f"Unknown EastMoney asset: {asset_name}"
    raise KeyError(msg)


def eastmoney_business_field_names(asset_name: str) -> tuple[str, ...]:
    if asset_name not in SOURCE_FIELD_NAMES:
        msg = f"EastMoney schema fields are not configured for asset: {asset_name}"
        raise KeyError(msg)
    field_names = SOURCE_FIELD_NAMES[asset_name]
    if not field_names:
        msg = f"EastMoney schema fields are empty for asset: {asset_name}"
        raise ValueError(msg)
    if len(set(field_names)) != len(field_names):
        msg = f"EastMoney schema fields contain duplicates for asset: {asset_name}"
        raise ValueError(msg)
    return field_names


def eastmoney_typed_schema(endpoint: EastmoneyEndpointConfig) -> pa.Schema:
    """为 EastMoney 端点返回 contract schema。"""
    schema = EASTMONEY_SCHEMAS.get(endpoint.asset_name)
    if schema is None:
        msg = f"No explicit schema defined for EastMoney asset: {endpoint.asset_name}"
        raise KeyError(msg)
    return schema


def eastmoney_schema(endpoint: EastmoneyEndpointConfig) -> pa.Schema:
    """返回带类型的 schema（新版本）。"""
    return eastmoney_typed_schema(endpoint)


def eastmoney_rows_to_typed_table(
    endpoint: EastmoneyEndpointConfig,
    rows: Sequence[EastmoneyFetchedRow],
) -> EastmoneyTableResult:
    """将 EastMoney 行数据转换为带类型的 pa.Table。"""
    schema = eastmoney_typed_schema(endpoint)
    business_field_names = eastmoney_business_field_names(endpoint.asset_name)

    unknown_field_count = 0
    row_dicts: list[dict[str, Any]] = []
    for row in rows:
        unknown_field_count += unknown_field_count_for_mapping(
            row.data,
            allowed_top_level=business_field_names,
        )
        row_dicts.append(dict(row.data))

    table = typed_table(row_dicts, schema, converters=_eastmoney_field_converters(endpoint))
    return EastmoneyTableResult(
        table=table,
        unknown_field_count=unknown_field_count,
    )


def eastmoney_rows_to_table(
    endpoint: EastmoneyEndpointConfig,
    rows: Sequence[EastmoneyFetchedRow],
) -> EastmoneyTableResult:
    """将 EastMoney 行数据转换为带类型的 pa.Table。"""
    return eastmoney_rows_to_typed_table(endpoint, rows)


def empty_eastmoney_table(endpoint: EastmoneyEndpointConfig) -> pa.Table:
    schema = eastmoney_typed_schema(endpoint)
    return pa.table({field.name: [] for field in schema}, schema=schema)


def _eastmoney_field_converters(
    endpoint: EastmoneyEndpointConfig,
) -> dict[str, Callable[[Any], Any]]:
    if endpoint.asset_name == "eastmoney__dividend_main":
        return {"REPORT_TIME": _report_time_to_date_or_null}
    return {}


def _report_time_to_date_or_null(value: object) -> date | None:
    try:
        return to_date32(value)
    except SchemaTypeError:
        return None


def eastmoney_string_or_null(value: object) -> str | None:
    """保留原有函数作为 fallback。"""
    if value is None:
        return None
    if isinstance(value, str):
        return value
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int | float):
        return str(value)
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
