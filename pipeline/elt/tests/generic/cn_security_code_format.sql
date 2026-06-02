{% test cn_security_code_format(model, column_name) %}

select *
from {{ model }}
where {{ column_name }} is not null
  and not match(toString({{ column_name }}), '^[0-9]{6}\\.(SH|SZ|BJ)$')

{% endtest %}
