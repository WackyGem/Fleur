"""类型感知的值转换函数，替代全 string 转换。

转换失败时抛出 SchemaTypeError，便于开发调试时发现问题。
"""

from __future__ import annotations

import json
from datetime import UTC, date, datetime, time
from typing import Any


class SchemaTypeError(Exception):
    """Schema 类型转换失败。"""


def to_date32(value: Any) -> date | None:
    """将值转换为 date，用于 pa.date32() 列。

    支持格式：
    - 'YYYY-MM-DD' (BaoStock, Sina)
    - 'YYYY-MM-DD 00:00:00' (EastMoney)
    - 'YYYYMMDD' (THS)
    - date/datetime 对象

    None → 返回 None（缺失值）
    无法转换时抛出 SchemaTypeError。
    """
    if value is None:
        return None
    if isinstance(value, date) and not isinstance(value, datetime):
        return value
    if isinstance(value, datetime):
        return value.date()
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        # 支持 YYYYMMDD 格式（THS）
        if len(cleaned) == 8 and cleaned.isdigit():
            return date(int(cleaned[:4]), int(cleaned[4:6]), int(cleaned[6:8]))
        # 支持 YYYY-MM-DD 和 YYYY-MM-DD 00:00:00 格式
        if "-" in cleaned:
            # 截取日期部分（忽略时间部分）
            date_part = cleaned.split(" ")[0]
            parts = date_part.split("-")
            try:
                return date(int(parts[0]), int(parts[1]), int(parts[2]))
            except (ValueError, IndexError) as err:
                raise SchemaTypeError(f"Cannot convert '{value}' to date32") from err
    raise SchemaTypeError(f"Cannot convert {type(value).__name__} '{value}' to date32")


def to_float64(value: Any) -> float | None:
    """将值转换为 float，用于 pa.float64() 列。

    None → 返回 None（缺失值）
    转换失败时抛出 SchemaTypeError。
    """
    if value is None:
        return None
    if isinstance(value, bool):
        return 1.0 if value else 0.0
    if isinstance(value, int | float):
        return float(value)
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        try:
            return float(cleaned)
        except ValueError as err:
            raise SchemaTypeError(f"Cannot convert '{value}' to float64") from err
    raise SchemaTypeError(f"Cannot convert {type(value).__name__} '{value}' to float64")


def to_int64(value: Any) -> int | None:
    """将值转换为 int，用于 pa.int64() 列。

    None → 返回 None（缺失值）
    转换失败时抛出 SchemaTypeError。
    """
    if value is None:
        return None
    if isinstance(value, bool):
        return 1 if value else 0
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        try:
            return int(cleaned)
        except ValueError as err:
            raise SchemaTypeError(f"Cannot convert '{value}' to int64") from err
    raise SchemaTypeError(f"Cannot convert {type(value).__name__} '{value}' to int64")


def to_int8(value: Any) -> int | None:
    """将值转换为 int8，用于小整数列（标志位、状态码）。

    None → 返回 None（缺失值）
    转换失败时抛出 SchemaTypeError。
    超出 int8 范围时抛出 SchemaTypeError。
    """
    if value is None:
        return None
    result = to_int64(value)
    if result is None:
        return None
    if -128 <= result <= 127:
        return result
    raise SchemaTypeError(f"Value {result} out of int8 range (-128 to 127)")


def to_bool(value: Any) -> bool | None:
    """将值转换为 bool，用于 pa.bool_() 列。

    None → 返回 None（缺失值）
    转换失败时抛出 SchemaTypeError。
    """
    if value is None:
        return None
    if isinstance(value, bool):
        return value
    if isinstance(value, int | float):
        return bool(value)
    if isinstance(value, str):
        cleaned = value.strip().lower()
        if not cleaned:
            return None
        if cleaned in ("true", "1", "yes"):
            return True
        if cleaned in ("false", "0", "no"):
            return False
        raise SchemaTypeError(f"Cannot convert '{value}' to bool")
    raise SchemaTypeError(f"Cannot convert {type(value).__name__} '{value}' to bool")


def to_string(value: Any) -> str | None:
    """将值转换为 string，用于 pa.string() 列。

    None → 返回 None（缺失值）
    永不抛出异常，任何值都能转为字符串。
    """
    if value is None:
        return None
    if isinstance(value, str):
        return value
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int | float):
        return str(value)
    return json.dumps(value, ensure_ascii=False, sort_keys=True, default=str)


def to_timestamp(value: Any) -> datetime | None:
    """将时间值转为 datetime。

    支持格式：
    - Unix 秒/毫秒时间戳（THS）: '1776130842'
    - 'YYYY-MM-DD HH:MM:SS'（JiuYan）: '2026-05-29 12:00:15'
    - ISO datetime 字符串: '2026-05-29T12:00:15'

    None → 返回 None（缺失值）
    转换失败时抛出 SchemaTypeError。
    """
    if value is None:
        return None
    if isinstance(value, datetime):
        return value
    if isinstance(value, int | float):
        # Unix 时间戳 → UTC datetime
        return datetime.fromtimestamp(value, tz=UTC)
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        # Unix 时间戳（纯数字）
        if cleaned.isdigit():
            try:
                numeric_value = int(cleaned)
                if len(cleaned) >= 13:
                    numeric_value = numeric_value / 1000
                return to_timestamp(numeric_value)
            except (ValueError, OSError, SchemaTypeError) as err:
                raise SchemaTypeError(f"Cannot convert '{value}' to timestamp") from err
        try:
            return datetime.fromisoformat(cleaned)
        except ValueError as err:
            raise SchemaTypeError(f"Cannot convert '{value}' to timestamp") from err
    raise SchemaTypeError(f"Cannot convert {type(value).__name__} '{value}' to timestamp")


def to_time32_ms(value: Any) -> time | None:
    """将 'HH:MM:SS' 转为 time 对象，用于 pa.time32('ms') 列。

    JiuYan action_info.time 格式：'09:37:51'

    None → 返回 None（缺失值）
    转换失败时抛出 SchemaTypeError。
    """
    if value is None:
        return None
    if isinstance(value, time):
        return value
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        parts = cleaned.split(":")
        if len(parts) == 3:
            try:
                return time(int(parts[0]), int(parts[1]), int(parts[2]))
            except (ValueError, IndexError) as err:
                raise SchemaTypeError(f"Cannot convert '{value}' to time32[ms]") from err
    raise SchemaTypeError(f"Cannot convert {type(value).__name__} '{value}' to time32[ms]")
