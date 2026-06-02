# Contract-driven Parquet schema adapter small-batch failure

UTC time: 2026-06-01T18:37:03Z

## Failure 1: BaoStock stock basic nullable outDate

### Scope

- Plan: `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- Failed asset/job: `source/baostock__query_stock_basic`
- Partition or config: latest snapshot, no partition
- Run ID: `783a2440-a310-4212-a258-08503a410d47`
- Environment reset: yes

### Command

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_stock_basic"
```

### Failure

- Error summary: Parquet write failed because `outDate` was declared non-nullable but contained nulls.
- First failing step: `source__baostock__query_stock_basic`
- Relevant validation message: `pyarrow.lib.ArrowInvalid: Column 'outDate' is declared non-nullable but contains nulls`

### Diagnosis

- Expected schema: `baostock__query_stock_basic.outDate` was declared `date32[day]`, `nullable: false`.
- Actual schema/data: BaoStock returns an empty `outDate` for non-delisted securities; conversion maps that empty string to null.
- Suspected code path: `scheduler.defs.baostock.schemas.response_to_table()` normalizes empty `outDate` to `None`, matching the contract source description but conflicting with the Parquet and ClickHouse raw nullable flags.

### Resolution

- Fix PR or commit: local fix changed `baostock__query_stock_basic.outDate` to nullable in the Parquet and ClickHouse raw contract fields.
- Rerun command:

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_stock_basic"
```

- Rerun Run ID: `c7b42044-6d90-46be-9909-1c2209792b18`
- Final result: rerun succeeded; `source__baostock__query_stock_basic` materialized through `s3_io_manager` without schema/write errors.

## Failure 2: JiuYan industry list nullable author and delete_time

### Scope

- Plan: `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- Failed asset/job: `source/jiuyan__industry_list`
- Partition or config: latest snapshot, no partition
- Run ID: `639adb14-4845-4524-a4ab-abd64a69e5ed`
- Environment reset: yes

### Command

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_list"
```

### Failure

- Error summary: Parquet write failed because `author` was declared non-nullable but contained nulls. After fixing `author`, rerun exposed the same contract issue for `delete_time`.
- First failing step: `source__jiuyan__industry_list`
- Relevant validation messages:
  - `pyarrow.lib.ArrowInvalid: Column 'author' is declared non-nullable but contains nulls`
  - `pyarrow.lib.ArrowInvalid: Column 'delete_time' is declared non-nullable but contains nulls`

### Diagnosis

- Expected schema: `jiuyan__industry_list.author` and `jiuyan__industry_list.delete_time` were declared non-nullable.
- Actual schema/data: the current JiuYan industry-list response includes 748 null `author` values and 956 null `delete_time` values across 956 rows.
- Suspected code path: `scheduler.defs.http.schemas.jiuyan_industry_list_to_table()` preserves nulls from the source response, matching observed payload semantics but conflicting with the contract nullable flags.

### Resolution

- Fix PR or commit: local fix changed `jiuyan__industry_list.author` and `jiuyan__industry_list.delete_time` to nullable in the Parquet and ClickHouse raw contract fields.
- Rerun command:

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_list"
```

- Rerun Run ID: `c793f6f3-3675-4de1-9136-dbd0e712db4f`
- Final result: rerun succeeded; `source__jiuyan__industry_list` materialized through `s3_io_manager` without schema/write errors.

## Failure 3: BaoStock daily K-line nullable market fields

### Scope

- Plan: `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- Failed asset/job: `source/baostock__query_history_k_data_plus_daily`
- Partition or config: year partition `2020`
- Run ID: `bf9e1e52-7de8-4795-93a2-448119f89e13`
- Environment reset: yes

### Command

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition 2020
```

### Failure

- Error summary: Parquet write failed because `open` was declared non-nullable but contained nulls.
- First failing step: `source__baostock__query_history_k_data_plus_daily`
- Relevant validation message: `pyarrow.lib.ArrowInvalid: Column 'open' is declared non-nullable but contains nulls`

### Diagnosis

- Expected schema: BaoStock daily K-line market fields were declared non-nullable.
- Actual schema/data: current 2020 BaoStock K-line response has 1,084,680 rows and nulls in `open`, `high`, `low`, `close`, `preclose`, `volume`, `amount`, `turn`, and `pctChg`.
- Suspected code path: BaoStock returns empty numeric market fields for some inactive/suspended rows; conversion preserves those values as nulls, conflicting with contract nullable flags.

### Resolution

- Fix PR or commit: local fix changed the nullable BaoStock K-line market fields observed in the 2020 payload to nullable in the Parquet and ClickHouse raw contract fields.
- Rerun command:

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition 2020
```

- Rerun Run ID: `58f21a47-71d9-421e-9ca8-d33ecec6379d`
- Final result: rerun succeeded; `source__baostock__query_history_k_data_plus_daily` partition `2020` materialized through `s3_io_manager` without schema/write errors.

## Failure 4: EastMoney nullable financial statement fields

### Scope

- Plan: `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- Failed asset/job: `eastmoney__daily_job`
- Partition or config: year partition `2020`
- Run ID: `34b531b4-0da5-45fa-aaf4-7d1a3ae8801e`
- Environment reset: yes

### Command

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 2020
```

### Failure

- Error summary: Parquet writes failed because several EastMoney financial statement fields were declared non-nullable but contained nulls in 2020 responses.
- First failing steps:
  - `source__eastmoney__balance`: `ACCEPT_DEPOSIT_INTERBANK`
  - `source__eastmoney__cashflow_sq`: `SALES_SERVICES`
  - `source__eastmoney__cashflow_ytd`: `SALES_SERVICES`
  - `source__eastmoney__equity_history`: `LIMITED_SHARES`
  - `source__eastmoney__income_sq`: `TOTAL_OPERATE_INCOME`
  - `source__eastmoney__income_ytd`: `TOTAL_OPERATE_INCOME`
- Relevant validation message pattern: `pyarrow.lib.ArrowInvalid: Column '<FIELD>' is declared non-nullable but contains nulls`

### Diagnosis

- Expected schema: the listed EastMoney numeric fields were declared non-nullable in the Parquet and ClickHouse raw contract fields.
- Actual schema/data: the current 2020 EastMoney responses include nulls for those fields on some securities.
- Suspected code path: `scheduler.defs.sources.eastmoney.schema.eastmoney_rows_to_table()` preserves null values from EastMoney payloads, conflicting with contract nullable flags.

### Resolution

- Fix PR or commit: local fix first changed the initially observed EastMoney nullable fields to nullable in the Parquet and ClickHouse raw contract fields.
- Rerun command 1:

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 2020
```

- Rerun Run ID 1: `8034e625-e0ec-40f1-8055-620c99200280`
- Final result 1: failed again, exposing the same nullable-contract issue on additional EastMoney financial statement numeric fields:
  - `source__eastmoney__balance`: `ACCOUNTS_PAYABLE`
  - `source__eastmoney__cashflow_sq`: `DEPOSIT_INTERBANK_ADD`
  - `source__eastmoney__cashflow_ytd`: `DEPOSIT_INTERBANK_ADD`
  - `source__eastmoney__equity_history`: `UNLIMITED_SHARES`
  - `source__eastmoney__income_sq`: `TOTAL_OPERATE_INCOME_QOQ`
  - `source__eastmoney__income_ytd`: `TOTAL_OPERATE_INCOME_YOY`
- Follow-up fix: broaden EastMoney financial statement contract handling so source fields declared as `type: number` are nullable in Parquet and ClickHouse raw, while keeping identifier, date, currency and categorical fields unchanged.
- Rerun command 2:

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 2020
```

- Rerun Run ID 2: `82a75e3f-3eee-42d2-821a-c7d8324c42a7`
- Final result 2: rerun succeeded; all 8 EastMoney source assets for partition `2020` materialized through `s3_io_manager` without schema/write errors.

## Failure 5: JiuYan action field nullable market-event fields

### Scope

- Plan: `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- Failed asset/job: `source/jiuyan__action_field`
- Partition or config: daily partition `2026-06-01`
- Run ID: `853e712f-5433-47c0-91d7-88aca23901f1`
- Environment reset: yes

### Command

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__action_field" --partition 2026-06-01
```

### Failure

- Error summary: Dagster run surfaced as a bounded task failure; direct fetch-and-write reproduction showed the underlying Parquet write failed because several market-event fields were declared non-nullable but contained nulls.
- First failing step: `source__jiuyan__action_field`
- Relevant validation messages:
  - `RuntimeError: JiuYan action-field materialization failed at async boundary: All 1 bounded tasks failed`
  - `pyarrow.lib.ArrowInvalid: Column 'delete_time' is declared non-nullable but contains nulls`

### Diagnosis

- Expected schema: `jiuyan__action_field.delete_time`, `update_time`, `time`, `num`, `day` and `edition` were declared non-nullable.
- Actual schema/data: a direct follow-up call to the same JiuYan endpoint for `2026-06-01` returned `errCode=0`, 185 rows and 18 columns, with null counts in `delete_time` (185), `update_time` (185), `time` (19), `num` (142), `day` (142) and `edition` (142).
- Suspected code path: `scheduler.defs.http.schemas.jiuyan_action_field_to_table()` preserves source nulls; the daily source and yearly compacted contracts were stricter than the observed payload.

### Resolution

- Fix PR or commit: local fix changed the observed nullable JiuYan action-field fields to nullable in the daily source contract and corresponding compacted/raw contract fields.
- Rerun command 1:

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__action_field" --partition 2026-06-01
```

- Rerun Run ID 1: `ec396721-ad5b-4f0f-ae72-cca6c90a8109`
- Final result 1: failed again with the same bounded-task wrapper error before the underlying schema issue was fixed.
- Rerun command 2:

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__action_field" --partition 2026-06-01
```

- Rerun Run ID 2: `13dbb27a-2243-466f-bf9c-c7e16c70fe60`
- Final result 2: rerun succeeded; partition `2026-06-01` materialized without schema/write errors.

## Failure 6: THS limit-up pool small-batch failure

### Scope

- Plan: `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- Failed asset/job: `source/ths__limit_up_pool`
- Partition or config: daily partition `2026-06-01`
- Run ID: `bec8cd4b-9f05-4205-9161-9591779c67ea`
- Environment reset: yes

### Command

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition 2026-06-01
```

### Failure

- Error summary: Dagster run surfaced as a bounded task failure; direct fetch-and-write reproduction showed the underlying Parquet write failed because `open_num` was declared non-nullable but contained nulls.
- First failing step: `source__ths__limit_up_pool`
- Relevant validation message: `RuntimeError: THS limit-up pool materialization failed at async boundary: All 1 bounded tasks failed`

### Diagnosis

- Expected schema: `ths__limit_up_pool.open_num` and `limit_up_suc_rate` were declared non-nullable.
- Actual schema/data: a direct follow-up call to the same THS endpoint for `2026-06-01` returned 118 rows and 22 columns, with null counts in `open_num` (69) and `limit_up_suc_rate` (13).
- Suspected code path: `scheduler.defs.http.schemas.ths_limit_up_pool_to_table()` preserves source nulls; the daily source and yearly compacted contracts were stricter than the observed payload.

### Resolution

- Fix PR or commit: local fix changed `open_num` and `limit_up_suc_rate` to nullable in the daily source contract and corresponding compacted/raw contract fields.
- Rerun command:

```bash
set -a
. ./.env
set +a
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition 2026-06-01
```

- Rerun Run ID: `0fe82c1f-146e-490a-8174-ce56aa575dfa`
- Final result: rerun succeeded; partition `2026-06-01` materialized without schema/write errors.
