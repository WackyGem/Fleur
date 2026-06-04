{% macro normalize_cn_security_code(column_name, input_format="eastmoney_suffix") -%}
  {%- if input_format == "eastmoney_suffix" -%}
    upper(toString({{ column_name }}))
  {%- elif input_format == "baostock_prefix" -%}
    concat(substring(toString({{ column_name }}), 4, 6), '.', upper(substring(toString({{ column_name }}), 1, 2)))
  {%- elif input_format == "compact_prefix" -%}
    concat(substring(toString({{ column_name }}), 3, 6), '.', upper(substring(toString({{ column_name }}), 1, 2)))
  {%- elif input_format == "a_share_local_code" -%}
    multiIf(
      match(toString({{ column_name }}), '^(600|601|603|605|688|689)[0-9]{3}$'),
      concat(toString({{ column_name }}), '.SH'),
      match(toString({{ column_name }}), '^(000|001|002|003|300|301)[0-9]{3}$'),
      concat(toString({{ column_name }}), '.SZ'),
      match(toString({{ column_name }}), '^(43|83|87|88|92)[0-9]{4}$'),
      concat(toString({{ column_name }}), '.BJ'),
      null
    )
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
  {%- elif input_format == "a_share_local_code" -%}
    if(match(toString({{ column_name }}), '^[0-9]{6}$'), toString({{ column_name }}), null)
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
  {%- elif input_format == "a_share_local_code" -%}
    multiIf(
      match(toString({{ column_name }}), '^(600|601|603|605|688|689)[0-9]{3}$'),
      'SH',
      match(toString({{ column_name }}), '^(000|001|002|003|300|301)[0-9]{3}$'),
      'SZ',
      match(toString({{ column_name }}), '^(43|83|87|88|92)[0-9]{4}$'),
      'BJ',
      null
    )
  {%- else -%}
    {{ exceptions.raise_compiler_error("Unsupported input_format for cn_exchange_code: " ~ input_format) }}
  {%- endif -%}
{%- endmacro %}
