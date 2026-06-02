# ClickHouse Layered Database Baseline

日期：2026-06-02T16:52:31.059494+00:00
Git working tree：M docs/references/data_dict/baostock__query_history_k_data_plus_daily.md
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
?? pipeline/contract_tools/src/fleur_contracts/clickhouse_layer_migration.py
?? pipeline/contract_tools/tests/test_clickhouse_layer_migration.py
?? pipeline/elt/macros/generate_schema_name.sql
?? pipeline/elt/scripts/validate_layer_routing.py

## 1. Database Scope

| Database | Exists |
| --- | --- |
| `raw` | True |
| `analytics` | True |
| `fleur_raw` | False |
| `fleur_staging` | False |
| `fleur_intermediate` | False |
| `fleur_marts` | False |

## 2. Raw Dataset Baseline

- Raw-enabled datasets：15
- 注：本报告由迁移工具早期版本生成，表格 `Target` 列显示迁移目标
  `fleur_raw.<table>`；同一行的 row count、schema fingerprint、active parts 和
  partition 清单均是在清库前从历史 `raw.<table>` 采集。

| Dataset | Target | Historical raw rows | Schema fingerprint | Active parts | Partitions |
| --- | --- | ---: | --- | ---: | --- |
| `baostock__query_history_k_data_plus_daily` | `fleur_raw.baostock__query_history_k_data_plus_daily` | 20335243 | `71717fba7ad34e9049d669f01968cc8c868774e954a7ec4101e2c83bb4363cf9` | 43 | 1990, 1991, 1992, 1993, 1994, 1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| `baostock__query_stock_basic` | `fleur_raw.baostock__query_stock_basic` | 8769 | `0edb1e612a76505f2d257d94b1d3011c3e53432902a0ad056eb506218fc4eb5a` | 1 | tuple() |
| `eastmoney__balance` | `fleur_raw.eastmoney__balance` | 284265 | `9639c8d086b0da03df3018094d4126c1e061a18468136198dbb3d5eafb21c0bf` | 36 | 1991, 1992, 1993, 1994, 1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| `eastmoney__cashflow_sq` | `fleur_raw.eastmoney__cashflow_sq` | 274016 | `36e78f0dc0dce2ab4e4bc0113b95290615e009d07458c948aa2f2d32c81d3e57` | 26 | 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| `eastmoney__cashflow_ytd` | `fleur_raw.eastmoney__cashflow_ytd` | 283613 | `d4665c5ada56a1452591dcfe51b751e3304a9a0ce8d41f2984b33feb632147f7` | 29 | 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| `eastmoney__dividend_allotment` | `fleur_raw.eastmoney__dividend_allotment` | 1156 | `a8c6a0bf07061e61fed8d63beec805b501d92d8a5a5cd950f372ca273a82fedd` | 31 | 1992, 1993, 1994, 1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023 |
| `eastmoney__dividend_main` | `fleur_raw.eastmoney__dividend_main` | 151606 | `4d0ef4f19425dbb60d1b900abca64efb71d60d4bae778534362b79c81a5af2b9` | 36 | 1991, 1992, 1993, 1994, 1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| `eastmoney__equity_history` | `fleur_raw.eastmoney__equity_history` | 146365 | `ea3739aafe6278d3a7dd51e24725ea26e5789964ca18ce38e0fa570ae3ba8f07` | 37 | 1990, 1991, 1992, 1993, 1994, 1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| `eastmoney__income_sq` | `fleur_raw.eastmoney__income_sq` | 279918 | `1a5d2ab8611ad4bb37f4f17d02c607da51875cbe6944c89c22bdffb4eaf98675` | 33 | 1993, 1994, 1995, 1996, 1997, 1998, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| `eastmoney__income_ytd` | `fleur_raw.eastmoney__income_ytd` | 298396 | `2c62d542c521a9ccca0fab405b3c751b58d937adf2e9fe91a5d1b2b7f05924ce` | 36 | 1991, 1992, 1993, 1994, 1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| `jiuyan__action_field_compacted` | `fleur_raw.jiuyan__action_field_compacted` | 5853 | `4727f364fa8898d31e5e9701f6e34bd08bc0d9a0efabac8d517eedcd9833f1c5` | 1 | 2026 |
| `jiuyan__industry_list` | `fleur_raw.jiuyan__industry_list` | 956 | `3f3cbb395c18f5d172e37d0ee42b606b71b3c29dbd9a1b02ce2fd9afa0e41b17` | 1 | tuple() |
| `jiuyan__industry_ocr_snapshot` | `fleur_raw.jiuyan__industry_ocr_snapshot` | 1069 | `bd3404708e995c854327df0c5a025b20e62c164ed8f78c7b3c21f68e0b5ac835` | 1 | tuple() |
| `sina__trade_calendar` | `fleur_raw.sina__trade_calendar` | 8797 | `4668ed72498a2ee3fcb851ca3d599a0c3028396b795a7c9295fa723bcfc9c47c` | 1 | tuple() |
| `ths__limit_up_pool_compacted` | `fleur_raw.ths__limit_up_pool_compacted` | 15664 | `a3ed53afb1cc924bf687b37a5b0654c0d3cd04590b347537c022f2d5fc32ed62` | 2 | 2025, 2026 |

## 3. S3 Coverage

- Manifest datasets：15
- Datasets with missing objects：0
- Partition manifest：`docs/jobs/reports/2026-06-02-clickhouse-layered-database-partitions.json`
- Manifest 已在最终验收前补充每个 S3 Parquet object 的 `row_count`、`size` 和 `mtime`。

## 4. ClickHouse Privilege Evidence

- Required privileges: `DROP DATABASE`, `CREATE DATABASE`, `CREATE TABLE`, `INSERT`,
  `ALTER TABLE REPLACE PARTITION`, `EXCHANGE TABLES`, `SELECT`.
- `SHOW GRANTS` for migration user `mono_fleur` included `SELECT`, `INSERT`, `ALTER`,
  `CREATE`, `DROP`, `S3`, and related table/database privileges on `*.*`.

## 5. Safety Gate

- Confirmation token：`e9f3ad906e01eae0`
- Reset command：`uv run fleur-contracts clickhouse-layer reset --manifest <partition-manifest.json> --confirm <token>`
