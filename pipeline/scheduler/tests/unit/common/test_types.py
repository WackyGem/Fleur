"""类型转换函数单元测试。"""

from datetime import UTC, date, datetime, time

import pytest
from scheduler.defs.common.types import (
    SchemaTypeError,
    to_bool,
    to_date32,
    to_float64,
    to_int8,
    to_int64,
    to_string,
    to_time32_ms,
    to_timestamp,
)


class TestToDate32:
    """to_date32 函数测试。"""

    def test_yyyy_mm_dd(self):
        assert to_date32("2024-01-15") == date(2024, 1, 15)

    def test_yyyy_mm_dd_hh_mm_ss(self):
        """EastMoney 格式: '2026-03-31 00:00:00'"""
        assert to_date32("2026-03-31 00:00:00") == date(2026, 3, 31)

    def test_yyyymmdd(self):
        """THS 格式: '20260508'"""
        assert to_date32("20260508") == date(2026, 5, 8)

    def test_date_object(self):
        assert to_date32(date(2024, 1, 15)) == date(2024, 1, 15)

    def test_datetime_object(self):
        assert to_date32(datetime(2024, 1, 15, 10, 30)) == date(2024, 1, 15)

    def test_empty_string_returns_none(self):
        assert to_date32("") is None
        assert to_date32("   ") is None

    def test_raises_on_invalid_format(self):
        with pytest.raises(SchemaTypeError, match="Cannot convert"):
            to_date32("not-a-date")

    def test_returns_none_for_none(self):
        assert to_date32(None) is None


class TestToFloat64:
    """to_float64 函数测试。"""

    def test_integer_string(self):
        assert to_float64("19471149555") == 19471149555.0

    def test_float_string(self):
        assert to_float64("19471149555.9") == 19471149555.9

    def test_integer_value(self):
        assert to_float64(123) == 123.0

    def test_float_value(self):
        assert to_float64(123.45) == 123.45

    def test_bool_value(self):
        assert to_float64(True) == 1.0
        assert to_float64(False) == 0.0

    def test_empty_string_returns_none(self):
        assert to_float64("") is None
        assert to_float64("   ") is None

    def test_raises_on_invalid_value(self):
        with pytest.raises(SchemaTypeError, match="Cannot convert"):
            to_float64("not-a-number")

    def test_returns_none_for_none(self):
        assert to_float64(None) is None


class TestToInt64:
    """to_int64 函数测试。"""

    def test_integer_string(self):
        assert to_int64("123") == 123

    def test_integer_value(self):
        assert to_int64(123) == 123

    def test_float_value(self):
        assert to_int64(123.7) == 123

    def test_bool_value(self):
        assert to_int64(True) == 1
        assert to_int64(False) == 0

    def test_empty_string_returns_none(self):
        assert to_int64("") is None
        assert to_int64("   ") is None

    def test_raises_on_invalid_value(self):
        with pytest.raises(SchemaTypeError, match="Cannot convert"):
            to_int64("not-an-integer")

    def test_returns_none_for_none(self):
        assert to_int64(None) is None


class TestToInt8:
    """to_int8 函数测试。"""

    def test_valid_values(self):
        assert to_int8("1") == 1
        assert to_int8("0") == 0
        assert to_int8("-128") == -128
        assert to_int8("127") == 127

    def test_raises_on_out_of_range(self):
        with pytest.raises(SchemaTypeError, match="out of int8 range"):
            to_int8("999")
        with pytest.raises(SchemaTypeError, match="out of int8 range"):
            to_int8("-129")


class TestToBool:
    """to_bool 函数测试。"""

    def test_string_true(self):
        assert to_bool("true") is True
        assert to_bool("1") is True
        assert to_bool("yes") is True

    def test_string_false(self):
        assert to_bool("false") is False
        assert to_bool("0") is False
        assert to_bool("no") is False

    def test_integer_value(self):
        assert to_bool(1) is True
        assert to_bool(0) is False

    def test_bool_value(self):
        assert to_bool(True) is True
        assert to_bool(False) is False

    def test_raises_on_invalid_value(self):
        with pytest.raises(SchemaTypeError, match="Cannot convert"):
            to_bool("maybe")

    def test_returns_none_for_none(self):
        assert to_bool(None) is None


class TestToString:
    """to_string 函数测试。"""

    def test_string_value(self):
        assert to_string("hello") == "hello"

    def test_integer_value(self):
        assert to_string(123) == "123"

    def test_float_value(self):
        assert to_string(123.45) == "123.45"

    def test_bool_value(self):
        assert to_string(True) == "true"
        assert to_string(False) == "false"

    def test_none_value(self):
        assert to_string(None) is None


class TestToTimestamp:
    """to_timestamp 函数测试。"""

    def test_unix_timestamp_string(self):
        result = to_timestamp("1776130842")
        assert result is not None
        assert result.tzinfo is not None
        assert result.year == 2026
        assert result.month == 4
        assert result.day == 14
        assert result.hour == 1  # UTC 时间

    def test_unix_timestamp_integer(self):
        result = to_timestamp(1776130842)
        assert result is not None
        assert result.tzinfo == UTC

    def test_unix_millisecond_timestamp_string(self):
        result = to_timestamp("1776130842000")
        assert result is not None
        assert result.tzinfo == UTC
        assert result.year == 2026
        assert result.month == 4
        assert result.day == 14

    def test_yyyy_mm_dd_hh_mm_ss_string(self):
        result = to_timestamp("2026-05-29 12:00:15")
        assert result is not None
        assert result == datetime(2026, 5, 29, 12, 0, 15)
        assert result.tzinfo is None

    def test_iso_datetime_string(self):
        assert to_timestamp("2026-05-29T12:00:15") == datetime(2026, 5, 29, 12, 0, 15)

    def test_datetime_object(self):
        dt = datetime(2026, 4, 14, 2, 0, 42, tzinfo=UTC)
        assert to_timestamp(dt) == dt

    def test_empty_string_returns_none(self):
        assert to_timestamp("") is None
        assert to_timestamp("   ") is None

    def test_raises_on_invalid_value(self):
        with pytest.raises(SchemaTypeError, match="Cannot convert"):
            to_timestamp("not-a-timestamp")

    def test_returns_none_for_none(self):
        assert to_timestamp(None) is None


class TestToTime32Ms:
    """to_time32_ms 函数测试。"""

    def test_hh_mm_ss_string(self):
        assert to_time32_ms("09:37:51") == time(9, 37, 51)

    def test_time_object(self):
        value = time(9, 37, 51)
        assert to_time32_ms(value) == value

    def test_empty_string_returns_none(self):
        assert to_time32_ms("") is None
        assert to_time32_ms("   ") is None

    def test_raises_on_invalid_value(self):
        with pytest.raises(SchemaTypeError, match="Cannot convert"):
            to_time32_ms("not-a-time")

    def test_returns_none_for_none(self):
        assert to_time32_ms(None) is None
