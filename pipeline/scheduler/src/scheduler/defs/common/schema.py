"""Schema 构建工具，支持声明式类型定义。"""

from __future__ import annotations

from collections.abc import Callable, Mapping, Sequence
from typing import Any

import pyarrow as pa


def typed_schema(
    fields: Sequence[tuple[str, pa.DataType]],
) -> pa.Schema:
    """从 (字段名, 类型) 元组列表构建 schema。"""
    return pa.schema([pa.field(name, dtype) for name, dtype in fields])


def typed_table(
    rows: Sequence[Mapping[str, Any]],
    schema: pa.Schema,
    converters: dict[str, Callable[[Any], Any]] | None = None,
) -> pa.Table:
    """从行数据和 schema 构建 pa.Table，自动应用类型转换。

    None 值直接追加，不经过转换函数（转换函数会抛出异常）。
    """
    if converters is None:
        converters = {}

    columns: dict[str, list[Any]] = {}
    for field in schema:
        converter = converters.get(field.name, _default_converter(field.type))
        converted_values = []
        for row in rows:
            value = row.get(field.name)
            # None 值直接追加，不经过转换函数
            if value is None:
                converted_values.append(None)
            else:
                converted_values.append(converter(value))
        columns[field.name] = converted_values

    return pa.table(columns, schema=schema)


def _default_converter(dtype: pa.DataType) -> Callable[[Any], Any]:
    """根据 PyArrow 类型返回默认转换函数。"""
    if pa.types.is_date(dtype):
        from scheduler.defs.common.types import to_date32

        return to_date32
    if pa.types.is_floating(dtype):
        from scheduler.defs.common.types import to_float64

        return to_float64
    if pa.types.is_integer(dtype):
        from scheduler.defs.common.types import to_int64

        return to_int64
    if pa.types.is_boolean(dtype):
        from scheduler.defs.common.types import to_bool

        return to_bool
    if pa.types.is_timestamp(dtype):
        from scheduler.defs.common.types import to_timestamp

        return to_timestamp
    if pa.types.is_time(dtype):
        from scheduler.defs.common.types import to_time32_ms

        return to_time32_ms
    # 默认：string
    from scheduler.defs.common.types import to_string

    return to_string
