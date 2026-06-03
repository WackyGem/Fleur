from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

PIPELINE_ROOT = Path(__file__).resolve().parents[2]
PROFILE_SCRIPT = PIPELINE_ROOT / "elt" / "scripts" / "profile_raw_source.py"


def _load_profile_module():
    spec = importlib.util.spec_from_file_location("profile_raw_source", PROFILE_SCRIPT)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_profile_raw_source_parses_date_null_and_placeholder_counts() -> None:
    module = _load_profile_module()
    result = module.QueryResult(
        title="日期范围",
        sql="select ...",
        succeeded=True,
        output="""
Previewing inline node:
| min_outdate | max_outdate | null_outdate | placeholder_outdate |
| ----------- | ----------- | ------------ | ------------------- |
| 1990-01-01  | 2026-06-01  |        7,644 |                   0 |
""",
    )

    profiles = module._date_profiles_from_results(
        date_columns=["outDate"],
        results=[result],
    )

    assert profiles == [
        module.DateColumnProfile(
            column="outDate",
            min_value="1990-01-01",
            max_value="2026-06-01",
            null_count=7644,
            placeholder_count=0,
        )
    ]
    assert "未发现 `1970-01-01` 占位值" in module._render_placeholder_summary(profiles)
    assert "未发现需要 staging 静默修正" in module._render_quality_issue_rows(profiles)


def test_profile_raw_source_does_not_claim_no_issues_without_executed_profiles() -> None:
    module = _load_profile_module()

    assert module._render_quality_issue_rows([]) == (
        "| 待补充 | 待补充 | 待补充 | 待补充 | 待补充 |"
    )


def test_profile_raw_source_only_renders_placeholder_issue_when_count_is_positive() -> None:
    module = _load_profile_module()
    profiles = [
        module.DateColumnProfile(
            column="delete_time",
            min_value="1970-01-01 00:00:00",
            max_value="1970-01-01 00:00:00",
            null_count=0,
            placeholder_count=5853,
        )
    ]

    quality_rows = module._render_quality_issue_rows(profiles)

    assert "`delete_time` 使用 `1970-01-01`" in quality_rows
    assert "5853 行" in quality_rows


def test_profile_raw_source_uses_string_placeholder_check_for_date_name_columns() -> None:
    module = _load_profile_module()
    column = module.SourceColumn(
        name="REPORT_DATE_NAME",
        data_type="LowCardinality(String)",
        description="",
    )

    expression = module._placeholder_count_expression(column)

    assert expression == (
        "countIf(toString(`REPORT_DATE_NAME`) = '1970-01-01') "
        "as placeholder_report_date_name"
    )


def test_profile_raw_source_normalizes_output_trailing_whitespace() -> None:
    module = _load_profile_module()

    assert module._normalize_output("21:23:17  \nvalue\t \n") == "21:23:17\nvalue"


def test_profile_raw_source_selects_nullable_datetime_columns_for_date_profiles() -> None:
    module = _load_profile_module()
    table = module.SourceTable(
        source_name="raw",
        table_name="jiuyan__action_field_compacted",
        description="",
        meta={},
        columns=[
            module.SourceColumn(
                name="delete_time",
                data_type="Nullable(DateTime64(3))",
                description="",
            ),
            module.SourceColumn(
                name="code",
                data_type="LowCardinality(String)",
                description="",
            ),
        ],
    )

    selected = module._selected_columns(
        table=table,
        keys=(),
        date_columns=(),
        enum_columns=(),
        format_columns=(),
        numeric_columns=(),
    )

    assert selected["date_columns"] == ["delete_time"]
