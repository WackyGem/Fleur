{% macro normalize_cn_security_code(column_name, input_format="eastmoney_suffix") -%}
  {%- if input_format == "eastmoney_suffix" -%}
    upper(toString({{ column_name }}))
  {%- elif input_format == "baostock_prefix" -%}
    concat(substring(toString({{ column_name }}), 4, 6), '.', upper(substring(toString({{ column_name }}), 1, 2)))
  {%- elif input_format == "compact_prefix" -%}
    concat(substring(toString({{ column_name }}), 3, 6), '.', upper(substring(toString({{ column_name }}), 1, 2)))
  {%- else -%}
    {{ exceptions.raise_compiler_error("Unsupported input_format for normalize_cn_security_code: " ~ input_format) }}
  {%- endif -%}
{%- endmacro %}

{% macro cn_security_local_code(column_name, input_format="eastmoney_suffix") -%}
  {%- if input_format == "eastmoney_suffix" -%}
    substring(toString({{ column_name }}), 1, 6)
  {%- elif input_format == "baostock_prefix" -%}
    substring(toString({{ column_name }}), 4, 6)
  {%- elif input_format == "compact_prefix" -%}
    substring(toString({{ column_name }}), 3, 6)
  {%- else -%}
    {{ exceptions.raise_compiler_error("Unsupported input_format for cn_security_local_code: " ~ input_format) }}
  {%- endif -%}
{%- endmacro %}

{% macro cn_exchange_code(column_name, input_format="eastmoney_suffix") -%}
  {%- if input_format == "eastmoney_suffix" -%}
    upper(substring(toString({{ column_name }}), 8, 2))
  {%- elif input_format == "baostock_prefix" -%}
    upper(substring(toString({{ column_name }}), 1, 2))
  {%- elif input_format == "compact_prefix" -%}
    upper(substring(toString({{ column_name }}), 1, 2))
  {%- else -%}
    {{ exceptions.raise_compiler_error("Unsupported input_format for cn_exchange_code: " ~ input_format) }}
  {%- endif -%}
{%- endmacro %}
