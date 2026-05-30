"""Schema 构建工具测试。"""

from datetime import date

import pyarrow as pa
import pytest
from scheduler.defs.common.schema import typed_schema, typed_table
from scheduler.defs.common.types import SchemaTypeError


class TestTypedSchema:
    """typed_schema 函数测试。"""

    def test_creates_schema_from_tuples(self):
        schema = typed_schema(
            [
                ("date", pa.date32()),
                ("value", pa.float64()),
                ("name", pa.string()),
            ]
        )
        assert len(schema) == 3
        assert schema.field("date").type == pa.date32()
        assert schema.field("value").type == pa.float64()
        assert schema.field("name").type == pa.string()


class TestTypedTable:
    """typed_table 函数测试。"""

    def test_creates_table_with_correct_types(self):
        schema = pa.schema(
            [
                pa.field("date", pa.date32()),
                pa.field("value", pa.float64()),
            ]
        )
        rows = [
            {"date": "2024-01-15", "value": "123.45"},
            {"date": "2024-01-16", "value": "678.90"},
        ]
        table = typed_table(rows, schema)
        assert table.schema.field("date").type == pa.date32()
        assert table.schema.field("value").type == pa.float64()
        assert table.num_rows == 2

    def test_handles_none_values(self):
        schema = pa.schema(
            [
                pa.field("date", pa.date32()),
                pa.field("value", pa.float64()),
            ]
        )
        rows = [
            {"date": "2024-01-15", "value": None},
            {"date": None, "value": "678.90"},
        ]
        table = typed_table(rows, schema)
        assert table.column("date")[0].as_py() == date(2024, 1, 15)
        assert table.column("date")[1].as_py() is None
        assert table.column("value")[0].as_py() is None
        assert table.column("value")[1].as_py() == 678.90

    def test_raises_on_conversion_error(self):
        schema = pa.schema(
            [
                pa.field("value", pa.float64()),
            ]
        )
        rows = [{"value": "not-a-number"}]
        with pytest.raises(SchemaTypeError):
            typed_table(rows, schema)

    def test_empty_rows(self):
        schema = pa.schema(
            [
                pa.field("date", pa.date32()),
                pa.field("value", pa.float64()),
            ]
        )
        rows = []
        table = typed_table(rows, schema)
        assert table.num_rows == 0
        assert table.schema.field("date").type == pa.date32()
        assert table.schema.field("value").type == pa.float64()
