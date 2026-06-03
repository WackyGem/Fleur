{% test unique_combination_of_columns(model, combination_of_columns) %}

select
    {% for column_name in combination_of_columns %}
        {{ column_name }}{% if not loop.last %},{% endif %}
    {% endfor %},
    count(*) as row_count
from {{ model }}
group by
    {% for column_name in combination_of_columns %}
        {{ column_name }}{% if not loop.last %},{% endif %}
    {% endfor %}
having row_count > 1

{% endtest %}
