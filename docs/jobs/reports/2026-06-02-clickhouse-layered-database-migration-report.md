# ClickHouse Layered Database Migration Report

日期：2026-06-02T16:54:34.941746+00:00
执行人：fleur-contracts clickhouse-layer migrate
Git commit / working tree：7fc30a1 / M docs/references/data_dict/baostock__query_history_k_data_plus_daily.md
 M docs/references/data_dict/baostock__query_stock_basic.md
 M docs/references/data_dict/eastmoney__balance.md
 M docs/references/data_dict/eastmoney__cashflow_sq.md
 M docs/references/data_dict/eastmoney__cashflow_ytd.md
 M docs/references/data_dict/eastmoney__dividend_allotment.md
 M docs/references/data_dict/eastmoney__dividend_main.md
 M docs/references/data_dict/eastmoney__equity_history.md
 M docs/references/data_dict/eastmoney__income_sq.md
 M docs/references/data_dict/eastmoney__income_ytd.md
 M docs/references/data_dict/jiuyan__action_field_compacted.md
 M docs/references/data_dict/jiuyan__industry_list.md
 M docs/references/data_dict/jiuyan__industry_ocr_snapshot.md
 M docs/references/data_dict/sina__trade_calendar.md
 M docs/references/data_dict/ths__limit_up_pool_compacted.md
 M pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py
 M pipeline/contract_tools/src/fleur_contracts/cli.py
 M pipeline/contract_tools/src/fleur_contracts/validate_clickhouse.py
 M pipeline/contract_tools/tests/test_contract_registry.py
 M pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml
 M pipeline/contracts/datasets/baostock__query_stock_basic.yml
 M pipeline/contracts/datasets/eastmoney__balance.yml
 M pipeline/contracts/datasets/eastmoney__cashflow_sq.yml
 M pipeline/contracts/datasets/eastmoney__cashflow_ytd.yml
 M pipeline/contracts/datasets/eastmoney__dividend_allotment.yml
 M pipeline/contracts/datasets/eastmoney__dividend_main.yml
 M pipeline/contracts/datasets/eastmoney__equity_history.yml
 M pipeline/contracts/datasets/eastmoney__income_sq.yml
 M pipeline/contracts/datasets/eastmoney__income_ytd.yml
 M pipeline/contracts/datasets/jiuyan__action_field_compacted.yml
 M pipeline/contracts/datasets/jiuyan__industry_list.yml
 M pipeline/contracts/datasets/jiuyan__industry_ocr_snapshot.yml
 M pipeline/contracts/datasets/sina__trade_calendar.yml
 M pipeline/contracts/datasets/ths__limit_up_pool_compacted.yml
 M pipeline/elt/dbt_project.yml
 M pipeline/elt/models/sources.yml
 M pipeline/elt/profiles.yml
 M pipeline/scheduler/src/scheduler/defs/clickhouse/assets.py
 M pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py
 M pipeline/scheduler/tests/integration/test_definitions_and_schedules.py
 M pipeline/scheduler/tests/unit/clickhouse/test_clickhouse_sql.py
 M pipeline/scheduler/tests/unit/clickhouse/test_raw_sync.py
?? docs/jobs/reports/2026-06-02-clickhouse-layered-database-baseline.md
?? docs/jobs/reports/2026-06-02-clickhouse-layered-database-migration-report.md
?? docs/jobs/reports/2026-06-02-clickhouse-layered-database-partitions.json
?? docs/jobs/reports/2026-06-02-clickhouse-layered-database-reset.md
?? pipeline/contract_tools/src/fleur_contracts/clickhouse_layer_migration.py
?? pipeline/contract_tools/tests/test_clickhouse_layer_migration.py
?? pipeline/elt/macros/generate_schema_name.sql
?? pipeline/elt/scripts/validate_layer_routing.py
环境：ClickHouse + Dagster + dbt local runner

## 1. Scope

- Reset databases: `raw`, `analytics`, `fleur_raw`, `fleur_staging`, `fleur_intermediate`, `fleur_marts`.
- Rematerialize all `clickhouse/raw/*` assets from the partition manifest.

## 2. Baseline Summary

- Partition manifest: `../docs/jobs/reports/2026-06-02-clickhouse-layered-database-partitions.json`

## 3. Reset Summary

- Reset report: `/storage/program/mono-fleur/docs/jobs/reports/2026-06-02-clickhouse-layered-database-reset.md`

## 4. Dagster Rematerialization Runs

| Step | Status | Command |
| --- | --- | --- |
| dagster home initialization | success | `make dagster-home` |
| dagster definitions check | success | `uv run dg check defs --target-path scheduler` |
| raw sync baostock__query_history_k_data_plus_daily partition 1990 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1990` |
| raw sync baostock__query_history_k_data_plus_daily partition 1991 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1991` |
| raw sync baostock__query_history_k_data_plus_daily partition 1992 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1992` |
| raw sync baostock__query_history_k_data_plus_daily partition 1993 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1993` |
| raw sync baostock__query_history_k_data_plus_daily partition 1994 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1994` |
| raw sync baostock__query_history_k_data_plus_daily partition 1995 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1995` |
| raw sync baostock__query_history_k_data_plus_daily partition 1996 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1996` |
| raw sync baostock__query_history_k_data_plus_daily partition 1997 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1997` |
| raw sync baostock__query_history_k_data_plus_daily partition 1998 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1998` |
| raw sync baostock__query_history_k_data_plus_daily partition 1999 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 1999` |
| raw sync baostock__query_history_k_data_plus_daily partition 2000 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2000` |
| raw sync baostock__query_history_k_data_plus_daily partition 2001 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2001` |
| raw sync baostock__query_history_k_data_plus_daily partition 2002 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2002` |
| raw sync baostock__query_history_k_data_plus_daily partition 2003 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2003` |
| raw sync baostock__query_history_k_data_plus_daily partition 2004 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2004` |
| raw sync baostock__query_history_k_data_plus_daily partition 2005 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2005` |
| raw sync baostock__query_history_k_data_plus_daily partition 2006 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2006` |
| raw sync baostock__query_history_k_data_plus_daily partition 2007 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2007` |
| raw sync baostock__query_history_k_data_plus_daily partition 2008 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2008` |
| raw sync baostock__query_history_k_data_plus_daily partition 2009 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2009` |
| raw sync baostock__query_history_k_data_plus_daily partition 2010 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2010` |
| raw sync baostock__query_history_k_data_plus_daily partition 2011 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2011` |
| raw sync baostock__query_history_k_data_plus_daily partition 2012 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2012` |
| raw sync baostock__query_history_k_data_plus_daily partition 2013 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2013` |
| raw sync baostock__query_history_k_data_plus_daily partition 2014 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2014` |
| raw sync baostock__query_history_k_data_plus_daily partition 2015 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2015` |
| raw sync baostock__query_history_k_data_plus_daily partition 2016 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2016` |
| raw sync baostock__query_history_k_data_plus_daily partition 2017 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2017` |
| raw sync baostock__query_history_k_data_plus_daily partition 2018 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2018` |
| raw sync baostock__query_history_k_data_plus_daily partition 2019 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2019` |
| raw sync baostock__query_history_k_data_plus_daily partition 2020 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2020` |
| raw sync baostock__query_history_k_data_plus_daily partition 2021 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2021` |
| raw sync baostock__query_history_k_data_plus_daily partition 2022 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2022` |
| raw sync baostock__query_history_k_data_plus_daily partition 2023 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2023` |
| raw sync baostock__query_history_k_data_plus_daily partition 2024 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2024` |
| raw sync baostock__query_history_k_data_plus_daily partition 2025 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2025` |
| raw sync baostock__query_history_k_data_plus_daily partition 2026 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_history_k_data_plus_daily --partition 2026` |
| raw sync baostock__query_stock_basic snapshot | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/baostock__query_stock_basic` |
| raw sync eastmoney__balance partition 1990 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1990` |
| raw sync eastmoney__balance partition 1991 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1991` |
| raw sync eastmoney__balance partition 1992 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1992` |
| raw sync eastmoney__balance partition 1993 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1993` |
| raw sync eastmoney__balance partition 1994 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1994` |
| raw sync eastmoney__balance partition 1995 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1995` |
| raw sync eastmoney__balance partition 1996 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1996` |
| raw sync eastmoney__balance partition 1997 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1997` |
| raw sync eastmoney__balance partition 1998 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1998` |
| raw sync eastmoney__balance partition 1999 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 1999` |
| raw sync eastmoney__balance partition 2000 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2000` |
| raw sync eastmoney__balance partition 2001 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2001` |
| raw sync eastmoney__balance partition 2002 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2002` |
| raw sync eastmoney__balance partition 2003 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2003` |
| raw sync eastmoney__balance partition 2004 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2004` |
| raw sync eastmoney__balance partition 2005 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2005` |
| raw sync eastmoney__balance partition 2006 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2006` |
| raw sync eastmoney__balance partition 2007 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2007` |
| raw sync eastmoney__balance partition 2008 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2008` |
| raw sync eastmoney__balance partition 2009 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2009` |
| raw sync eastmoney__balance partition 2010 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2010` |
| raw sync eastmoney__balance partition 2011 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2011` |
| raw sync eastmoney__balance partition 2012 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2012` |
| raw sync eastmoney__balance partition 2013 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2013` |
| raw sync eastmoney__balance partition 2014 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2014` |
| raw sync eastmoney__balance partition 2015 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2015` |
| raw sync eastmoney__balance partition 2016 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2016` |
| raw sync eastmoney__balance partition 2017 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2017` |
| raw sync eastmoney__balance partition 2018 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2018` |
| raw sync eastmoney__balance partition 2019 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2019` |
| raw sync eastmoney__balance partition 2020 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2020` |
| raw sync eastmoney__balance partition 2021 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2021` |
| raw sync eastmoney__balance partition 2022 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2022` |
| raw sync eastmoney__balance partition 2023 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2023` |
| raw sync eastmoney__balance partition 2024 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2024` |
| raw sync eastmoney__balance partition 2025 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2025` |
| raw sync eastmoney__balance partition 2026 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__balance --partition 2026` |
| raw sync eastmoney__cashflow_sq partition 1990 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1990` |
| raw sync eastmoney__cashflow_sq partition 1991 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1991` |
| raw sync eastmoney__cashflow_sq partition 1992 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1992` |
| raw sync eastmoney__cashflow_sq partition 1993 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1993` |
| raw sync eastmoney__cashflow_sq partition 1994 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1994` |
| raw sync eastmoney__cashflow_sq partition 1995 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1995` |
| raw sync eastmoney__cashflow_sq partition 1996 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1996` |
| raw sync eastmoney__cashflow_sq partition 1997 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1997` |
| raw sync eastmoney__cashflow_sq partition 1998 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1998` |
| raw sync eastmoney__cashflow_sq partition 1999 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 1999` |
| raw sync eastmoney__cashflow_sq partition 2000 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2000` |
| raw sync eastmoney__cashflow_sq partition 2001 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2001` |
| raw sync eastmoney__cashflow_sq partition 2002 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2002` |
| raw sync eastmoney__cashflow_sq partition 2003 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2003` |
| raw sync eastmoney__cashflow_sq partition 2004 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2004` |
| raw sync eastmoney__cashflow_sq partition 2005 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2005` |
| raw sync eastmoney__cashflow_sq partition 2006 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2006` |
| raw sync eastmoney__cashflow_sq partition 2007 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2007` |
| raw sync eastmoney__cashflow_sq partition 2008 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2008` |
| raw sync eastmoney__cashflow_sq partition 2009 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2009` |
| raw sync eastmoney__cashflow_sq partition 2010 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2010` |
| raw sync eastmoney__cashflow_sq partition 2011 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2011` |
| raw sync eastmoney__cashflow_sq partition 2012 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2012` |
| raw sync eastmoney__cashflow_sq partition 2013 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2013` |
| raw sync eastmoney__cashflow_sq partition 2014 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2014` |
| raw sync eastmoney__cashflow_sq partition 2015 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2015` |
| raw sync eastmoney__cashflow_sq partition 2016 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2016` |
| raw sync eastmoney__cashflow_sq partition 2017 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2017` |
| raw sync eastmoney__cashflow_sq partition 2018 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2018` |
| raw sync eastmoney__cashflow_sq partition 2019 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2019` |
| raw sync eastmoney__cashflow_sq partition 2020 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2020` |
| raw sync eastmoney__cashflow_sq partition 2021 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2021` |
| raw sync eastmoney__cashflow_sq partition 2022 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2022` |
| raw sync eastmoney__cashflow_sq partition 2023 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2023` |
| raw sync eastmoney__cashflow_sq partition 2024 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2024` |
| raw sync eastmoney__cashflow_sq partition 2025 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2025` |
| raw sync eastmoney__cashflow_sq partition 2026 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_sq --partition 2026` |
| raw sync eastmoney__cashflow_ytd partition 1990 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1990` |
| raw sync eastmoney__cashflow_ytd partition 1991 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1991` |
| raw sync eastmoney__cashflow_ytd partition 1992 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1992` |
| raw sync eastmoney__cashflow_ytd partition 1993 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1993` |
| raw sync eastmoney__cashflow_ytd partition 1994 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1994` |
| raw sync eastmoney__cashflow_ytd partition 1995 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1995` |
| raw sync eastmoney__cashflow_ytd partition 1996 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1996` |
| raw sync eastmoney__cashflow_ytd partition 1997 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1997` |
| raw sync eastmoney__cashflow_ytd partition 1998 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1998` |
| raw sync eastmoney__cashflow_ytd partition 1999 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 1999` |
| raw sync eastmoney__cashflow_ytd partition 2000 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2000` |
| raw sync eastmoney__cashflow_ytd partition 2001 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2001` |
| raw sync eastmoney__cashflow_ytd partition 2002 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2002` |
| raw sync eastmoney__cashflow_ytd partition 2003 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2003` |
| raw sync eastmoney__cashflow_ytd partition 2004 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2004` |
| raw sync eastmoney__cashflow_ytd partition 2005 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2005` |
| raw sync eastmoney__cashflow_ytd partition 2006 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2006` |
| raw sync eastmoney__cashflow_ytd partition 2007 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2007` |
| raw sync eastmoney__cashflow_ytd partition 2008 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2008` |
| raw sync eastmoney__cashflow_ytd partition 2009 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2009` |
| raw sync eastmoney__cashflow_ytd partition 2010 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2010` |
| raw sync eastmoney__cashflow_ytd partition 2011 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2011` |
| raw sync eastmoney__cashflow_ytd partition 2012 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2012` |
| raw sync eastmoney__cashflow_ytd partition 2013 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2013` |
| raw sync eastmoney__cashflow_ytd partition 2014 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2014` |
| raw sync eastmoney__cashflow_ytd partition 2015 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2015` |
| raw sync eastmoney__cashflow_ytd partition 2016 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2016` |
| raw sync eastmoney__cashflow_ytd partition 2017 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2017` |
| raw sync eastmoney__cashflow_ytd partition 2018 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2018` |
| raw sync eastmoney__cashflow_ytd partition 2019 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2019` |
| raw sync eastmoney__cashflow_ytd partition 2020 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2020` |
| raw sync eastmoney__cashflow_ytd partition 2021 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2021` |
| raw sync eastmoney__cashflow_ytd partition 2022 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2022` |
| raw sync eastmoney__cashflow_ytd partition 2023 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2023` |
| raw sync eastmoney__cashflow_ytd partition 2024 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2024` |
| raw sync eastmoney__cashflow_ytd partition 2025 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2025` |
| raw sync eastmoney__cashflow_ytd partition 2026 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__cashflow_ytd --partition 2026` |
| raw sync eastmoney__dividend_allotment partition 1990 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1990` |
| raw sync eastmoney__dividend_allotment partition 1991 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1991` |
| raw sync eastmoney__dividend_allotment partition 1992 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1992` |
| raw sync eastmoney__dividend_allotment partition 1993 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1993` |
| raw sync eastmoney__dividend_allotment partition 1994 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1994` |
| raw sync eastmoney__dividend_allotment partition 1995 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1995` |
| raw sync eastmoney__dividend_allotment partition 1996 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1996` |
| raw sync eastmoney__dividend_allotment partition 1997 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1997` |
| raw sync eastmoney__dividend_allotment partition 1998 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1998` |
| raw sync eastmoney__dividend_allotment partition 1999 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 1999` |
| raw sync eastmoney__dividend_allotment partition 2000 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2000` |
| raw sync eastmoney__dividend_allotment partition 2001 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2001` |
| raw sync eastmoney__dividend_allotment partition 2002 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2002` |
| raw sync eastmoney__dividend_allotment partition 2003 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2003` |
| raw sync eastmoney__dividend_allotment partition 2004 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2004` |
| raw sync eastmoney__dividend_allotment partition 2005 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2005` |
| raw sync eastmoney__dividend_allotment partition 2006 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2006` |
| raw sync eastmoney__dividend_allotment partition 2007 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2007` |
| raw sync eastmoney__dividend_allotment partition 2008 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2008` |
| raw sync eastmoney__dividend_allotment partition 2009 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2009` |
| raw sync eastmoney__dividend_allotment partition 2010 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2010` |
| raw sync eastmoney__dividend_allotment partition 2011 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2011` |
| raw sync eastmoney__dividend_allotment partition 2012 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2012` |
| raw sync eastmoney__dividend_allotment partition 2013 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2013` |
| raw sync eastmoney__dividend_allotment partition 2014 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2014` |
| raw sync eastmoney__dividend_allotment partition 2015 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2015` |
| raw sync eastmoney__dividend_allotment partition 2016 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2016` |
| raw sync eastmoney__dividend_allotment partition 2017 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2017` |
| raw sync eastmoney__dividend_allotment partition 2018 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2018` |
| raw sync eastmoney__dividend_allotment partition 2019 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2019` |
| raw sync eastmoney__dividend_allotment partition 2020 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2020` |
| raw sync eastmoney__dividend_allotment partition 2021 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2021` |
| raw sync eastmoney__dividend_allotment partition 2022 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2022` |
| raw sync eastmoney__dividend_allotment partition 2023 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2023` |
| raw sync eastmoney__dividend_allotment partition 2024 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2024` |
| raw sync eastmoney__dividend_allotment partition 2025 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2025` |
| raw sync eastmoney__dividend_allotment partition 2026 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_allotment --partition 2026` |
| raw sync eastmoney__dividend_main partition 1990 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1990` |
| raw sync eastmoney__dividend_main partition 1991 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1991` |
| raw sync eastmoney__dividend_main partition 1992 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1992` |
| raw sync eastmoney__dividend_main partition 1993 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1993` |
| raw sync eastmoney__dividend_main partition 1994 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1994` |
| raw sync eastmoney__dividend_main partition 1995 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1995` |
| raw sync eastmoney__dividend_main partition 1996 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1996` |
| raw sync eastmoney__dividend_main partition 1997 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1997` |
| raw sync eastmoney__dividend_main partition 1998 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1998` |
| raw sync eastmoney__dividend_main partition 1999 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 1999` |
| raw sync eastmoney__dividend_main partition 2000 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 2000` |
| raw sync eastmoney__dividend_main partition 2001 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 2001` |
| raw sync eastmoney__dividend_main partition 2002 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 2002` |
| raw sync eastmoney__dividend_main partition 2003 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 2003` |
| raw sync eastmoney__dividend_main partition 2004 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 2004` |
| raw sync eastmoney__dividend_main partition 2005 | success | `uv run dg launch --target-path scheduler --assets key:clickhouse/raw/eastmoney__dividend_main --partition 2005` |

## 5. Raw Table Validation

- Initial runner report stopped after `eastmoney__dividend_main` partition `2005`; the final
  reconciliation resumed the remaining manifest scope without another reset.
- Resume run log: `docs/jobs/reports/2026-06-02-clickhouse-layered-database-resume-runs.tsv`
  records 138 successful `dg launch` runs from `eastmoney__dividend_main 2006` through
  `ths__limit_up_pool_compacted 2026`.
- Cleanup run log: `docs/jobs/reports/2026-06-02-clickhouse-layered-database-cleanup-runs.tsv`
  records 6 successful targeted reruns used to remove residual staging tables through the
  raw sync service itself.
- Final command: `uv run fleur-contracts clickhouse-layer validate-raw --manifest ../docs/jobs/reports/2026-06-02-clickhouse-layered-database-partitions.json`
- Final result: `fleur_raw raw table validation passed.`
- The partition manifest now includes per-object `row_count`; `allow_empty` year datasets use
  that metadata so zero-row years are accepted without requiring active ClickHouse parts.

## 6. dbt Layer Validation

- Final dbt build command: `uv run dbt build --project-dir elt --profiles-dir elt --select staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'`
- Final validation commands:
  - `uv run fleur-contracts clickhouse-layer validate-dbt`
  - `uv run dbt parse --project-dir elt --profiles-dir elt`
  - `uv run python elt/scripts/validate_layer_routing.py`
  - `uv run python elt/scripts/validate_field_glossary.py`
  - `uv run python elt/scripts/validate_staging_readiness.py`
- Final results:
  - `dbt layer database validation passed.`
  - `Layer routing validation passed.`
  - `Field glossary lint passed.`
  - `Staging readiness passed.`
- `dbt parse` still reports unused `intermediate` and `marts` model config paths because those
  directories intentionally contain no business models yet; the databases are created by the
  project hook and validated by `validate-dbt`.

## 7. Failures / Exceptions

- Initial runner artifact did not include the full manifest scope after `eastmoney__dividend_main`
  partition `2005`; this was corrected by the resume run log above.
- Residual staging tables were detected by `validate-raw` after resume; they were cleared by
  targeted raw sync reruns recorded in the cleanup run log.
- No missing raw tables, schema mismatches, partition mismatches, LowCardinality issues, or residual
  staging tables remained after the final `validate-raw`.

## 8. Acceptance Checklist

- All rows in the resume and cleanup TSV logs have status `success`.
- Raw validation has no issues.
- dbt layer validation has no issues.
- The report and TSV logs contain run ids, dataset/partition labels, commands, return codes, and
  timestamps; no secrets are recorded.

## 9. Follow-ups

- No residual staging tables remained after cleanup validation.
- Keep `clickhouse__raw_sync_all_job` as selection coverage only; full historical migration remains
  driven by the partition manifest and explicit per-partition runs.
