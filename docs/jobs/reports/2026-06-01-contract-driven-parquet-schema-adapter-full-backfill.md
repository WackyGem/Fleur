# Contract-driven Parquet schema adapter full backfill

UTC start: 2026-06-01T19:23:45Z

UTC end: 2026-06-01T21:15:33Z

## Scope

- Plan: `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- Preconditions:
  - Phase 2 source/S3 small-batch completed after recorded nullable-contract fixes.
  - Phase 3 ClickHouse raw small-batch completed.
  - `uv run fleur-contracts validate-parquet --all-available` passed: checked 6 available objects, skipped 12 missing objects.
  - `uv run fleur-contracts validate-clickhouse --all-available` passed: checked 15 tables.
- Full backfill date basis: current project date `2026-06-01`.

## Ranges

- Snapshot/no partition source assets: current snapshot.
- BaoStock/EastMoney year partitions: `1990` through `2026`.
- JiuYan `source/jiuyan__action_field`: `2026-03-04...2026-06-01`.
- THS `source/ths__limit_up_pool`: `2025-01-01...2026-01-15`, then `2026-01-16...2026-06-01`.
- JiuYan OCR: one `limit=30` run, then snapshot publish.
- JiuYan compacted year partitions: `2026`.
- THS compacted year partitions: `2025` and `2026`.
- ClickHouse raw year partitions: aligned to completed source partitions above.

## Run IDs

### Source snapshots and OCR

| Step | Run ID | UTC finished |
| --- | --- | --- |
| `source/sina__trade_calendar` | `b4e78f1a-3504-4230-b511-bc0f2918a0fb` | 2026-06-01T19:25:03Z |
| `source/baostock__query_stock_basic` | `1848d33e-6140-4437-bbc4-c29e5223df43` | 2026-06-01T19:25:23Z |
| `source/jiuyan__industry_list` | `1dba0a02-5885-41b6-9d47-74edd5d38d1f` | 2026-06-01T19:25:34Z |
| `source/jiuyan__industry_images`, `limit=30` | `0f0a4e43-3585-43de-a899-4947d41dbea2` | 2026-06-01T19:25:46Z |
| `source/jiuyan__industry_ocr`, `limit=30` | `4b857392-0a67-4317-bab2-163f5f8dc64f` | 2026-06-01T19:28:36Z |
| `source/jiuyan__industry_ocr_snapshot` | `953e54c9-f59e-4d35-98c2-6ec3cd534539` | 2026-06-01T19:28:47Z |

### BaoStock source year partitions

| Partition | Run ID | UTC finished |
| --- | --- | --- |
| `1990` | `417b354c-fc1c-4d43-abb3-7a5654641d35` | 2026-06-01T19:29:42Z |
| `1991` | `112be1c8-2fce-485b-924b-5ac7e08402f4` | 2026-06-01T19:30:08Z |
| `1992` | `5991ff80-1b3c-4272-846b-22698b7cc07e` | 2026-06-01T19:34:11Z |
| `1993` | `7c4fe75a-bfed-430c-a420-eee5bdd180ab` | 2026-06-01T19:34:39Z |
| `1994` | `4185174a-86ce-4bfb-8c2d-7ccb9a1ff9e9` | 2026-06-01T20:05:01Z |
| `1995` | `2dc3d6a0-79fe-4adf-a9ee-4abfca5325c2` | 2026-06-01T20:05:20Z |
| `1996` | `b50cdef2-1a1b-4a31-85c4-ee436c8c1747` | 2026-06-01T20:05:45Z |
| `1997` | `2babf6e0-32c8-4fa0-93dd-c47b38226de7` | 2026-06-01T20:06:14Z |
| `1998` | `fb9637c6-a43e-4e16-8255-9a8370bfbe22` | 2026-06-01T20:06:39Z |
| `1999` | `57eea929-685f-47b6-b5f3-c235516cebb6` | 2026-06-01T20:07:09Z |
| `2000` | `092346d4-1c8f-4625-87c7-9420bea9f41c` | 2026-06-01T20:07:37Z |
| `2001` | `ee2356bf-3ecf-4793-98c1-b30295fe3007` | 2026-06-01T20:08:03Z |
| `2002` | `294dfbc1-dbbf-4537-b46f-4ade924efafc` | 2026-06-01T20:08:32Z |
| `2003` | `cd3cd7a2-b3d5-401c-8983-cd613215f189` | 2026-06-01T20:09:03Z |
| `2004` | `f8a85724-7326-4d7e-88dc-ac2b1c5e2bdd` | 2026-06-01T20:09:30Z |
| `2005` | `c20e321d-a298-490b-90f5-955be1a59780` | 2026-06-01T20:10:00Z |
| `2006` | `2711a21c-c52f-445e-b270-05e15776dcc3` | 2026-06-01T20:10:34Z |
| `2007` | `8ad9a7c6-0a97-42fd-8393-e6f67df53240` | 2026-06-01T20:11:04Z |
| `2008` | `dbc09858-f6ec-45c7-9f5d-bbda4d18a097` | 2026-06-01T20:11:40Z |
| `2009` | `3418f3c8-a0ca-4410-830e-a429c01326c8` | 2026-06-01T20:12:16Z |
| `2010` | `b0916c4b-c2b6-48ca-bfb6-3d2fc141ba83` | 2026-06-01T20:12:56Z |
| `2011` | `0d7f024e-f3a9-4b34-8f1e-09281cb3e1cd` | 2026-06-01T20:13:36Z |
| `2012` | `e5ee1d2b-ab11-440d-aa71-46d29cb45a7f` | 2026-06-01T20:14:24Z |
| `2013` | `b2c25261-9113-436b-bc1d-af449ad0883d` | 2026-06-01T20:15:04Z |
| `2014` | `fdc39473-5668-4374-ac54-ab6fac39aa52` | 2026-06-01T20:15:48Z |
| `2015` | `58d4b465-4727-43ff-95e0-5c215be59a46` | 2026-06-01T20:20:04Z |
| `2016` | `ba56ef08-91b9-4288-af85-2e4f6dd9fd23` | 2026-06-01T20:21:18Z |
| `2017` | `25923472-e212-4280-bcc1-69ee10c352da` | 2026-06-01T20:23:11Z |
| `2018` | `9076ca6c-ce70-449d-9308-e5b24714c7c4` | 2026-06-01T20:24:39Z |
| `2019` | `52816257-2bf4-499a-a7b8-cf5e65cc43a7` | 2026-06-01T20:26:07Z |
| `2020` | `54796ef5-69c9-46e0-91f9-9c4b70aa4542` | 2026-06-01T20:27:44Z |
| `2021` | `96e4af6e-b928-495e-8584-570852497845` | 2026-06-01T20:29:17Z |
| `2022` | `a63145c7-a9a8-4427-9be3-8710cb57301a` | 2026-06-01T20:30:58Z |
| `2023` | `a444139a-9170-4b21-b182-5528d13d0e67` | 2026-06-01T20:32:48Z |
| `2024` | `81e90e42-cc98-4072-bcdb-d3fcbb983c1c` | 2026-06-01T20:34:35Z |
| `2025` | `5def32f5-babe-4c74-a8ca-ea90591022a9` | 2026-06-01T20:36:22Z |
| `2026` | `86ccb293-c44f-4b3a-b07c-635ec3e2fce1` | 2026-06-01T20:37:52Z |

### EastMoney source year partitions

| Partition | Run ID | UTC finished |
| --- | --- | --- |
| `1990` | `b4396752-3364-4997-865a-e8dc38aa64ae` | 2026-06-01T19:29:54Z |
| `1991` | `0d4b83c4-2c27-4d61-8077-2a2e589a5e71` | 2026-06-01T19:33:26Z |
| `1992` | `8dbe6653-eee6-42db-a6d7-ad3fea53e8ba` | 2026-06-01T19:34:23Z |
| `1993` | `c97f8fad-189f-470c-ad9b-a2a413f28716` | 2026-06-01T19:40:27Z |
| `1994` | `a7abf3af-197b-4e71-b1d6-2c9571656e80` | 2026-06-01T19:43:04Z |
| `1995` | `f2f436b5-9fa5-438f-803b-fd9a53a8d174` | 2026-06-01T19:43:17Z |
| `1996` | `f67c92aa-a056-4784-9ff3-0bff046cdb14` | 2026-06-01T19:43:31Z |
| `1997` | `5f2d4f20-a687-4921-a530-2cfc11ea49e5` | 2026-06-01T19:43:47Z |
| `1998` | `59823ad7-c425-4769-93fb-5e3349fc1e6e` | 2026-06-01T19:44:04Z |
| `1999` | `a69abb96-1cce-4833-9d89-bb5dac923db3` | 2026-06-01T19:44:21Z |
| `2000` | `c494f0f9-c386-4942-bdbb-77c2a40662b3` | 2026-06-01T19:44:39Z |
| `2001` | `f1454fc5-250c-46bd-9f5f-a0a2d6eb6b35` | 2026-06-01T19:44:57Z |
| `2002` | `9d542c07-cd7f-41ec-9d84-9c06359439f7` | 2026-06-01T19:45:17Z |
| `2003` | `680a9a1c-7174-4266-a7fb-4e4d219e9ee4` | 2026-06-01T19:45:37Z |
| `2004` | `ac3cef4a-8abb-488c-9839-5bbe3a9db8c2` | 2026-06-01T19:45:57Z |
| `2005` | `a2594182-565d-458f-9a32-2a088cb64cb1` | 2026-06-01T19:46:18Z |
| `2006` | `7c547766-cf02-46c9-95a0-d400dd7bc610` | 2026-06-01T19:46:39Z |
| `2007` | `1d4a572b-a424-4f7b-a2fe-99116103b5ca` | 2026-06-01T19:47:01Z |
| `2008` | `f48443dd-04a3-44ef-8e85-1c406fe31768` | 2026-06-01T19:47:23Z |
| `2009` | `431998b7-1591-4d6f-87dc-a2bb581d586a` | 2026-06-01T19:47:45Z |
| `2010` | `0da43f40-667b-4b59-9ef7-972faf4d60bb` | 2026-06-01T19:48:11Z |
| `2011` | `52a875a1-7f23-4a93-b470-a4adc078371c` | 2026-06-01T19:48:38Z |
| `2012` | `7e0c4059-108e-4811-bb33-6d91863cd355` | 2026-06-01T19:49:07Z |
| `2013` | `300124c7-4caa-4ca8-80cb-dd97f6671c99` | 2026-06-01T19:49:35Z |
| `2014` | `7bcf2917-05fb-4a6b-965f-9bec26dd3295` | 2026-06-01T19:50:03Z |
| `2015` | `05b3a514-3c48-4516-9efb-f70daa46b717` | 2026-06-01T19:50:33Z |
| `2016` | `0cdb642e-27c1-431d-ad54-a8b9cb76b213` | 2026-06-01T19:51:05Z |
| `2017` | `ded82047-601a-4911-afd9-cd45352bec7e` | 2026-06-01T19:51:40Z |
| `2018` | `3701a742-db95-4f96-b891-d01f4edb2ffc` | 2026-06-01T19:52:16Z |
| `2019` | `96a2c0d9-6992-4588-a770-9a7408fe8818` | 2026-06-01T19:52:52Z |
| `2020` | `73e67009-7185-4a98-82f4-75f172e30458` | 2026-06-01T19:53:31Z |
| `2021` | `d21f0840-424e-4e53-bd4d-1e1212a1bfc1` | 2026-06-01T19:54:13Z |
| `2022` | `4699b1f6-0a4c-46e7-beca-fdf87a36dd7b` | 2026-06-01T19:54:57Z |
| `2023` | `20c5eac0-c2e5-43c5-838b-fad3fd3dc5de` | 2026-06-01T19:55:43Z |
| `2024` | `181a536c-083c-40b8-bdcf-abb75b1653b3` | 2026-06-01T19:56:30Z |
| `2025` | `1927a862-7ae5-4a78-870d-fa5b586be9d7` | 2026-06-01T19:57:16Z |
| `2026` | `e26f7571-f42a-4aa0-bb02-8e0dba348857` | 2026-06-01T20:01:11Z |

### Remaining source and raw partitions

- Source JiuYan action-field range `2026-03-04...2026-06-01`: `b47e2f8a-95f6-4966-93dc-625c64b36cc2`, succeeded at 2026-06-01T20:50:23Z.
- Source THS limit-up range `2025-01-01...2026-01-15`: `471837c8-2f17-4151-8d82-1308be1a33c6`, succeeded at 2026-06-01T20:51:23Z.
- Source THS limit-up range `2026-01-16...2026-06-01`: `1ca90db8-c4e8-48ac-9d90-4a1db13f4384`, succeeded at 2026-06-01T20:51:55Z.
- Source JiuYan action-field compacted `2026`: `a96d9494-145e-4696-b36d-088684ee259b`, succeeded at 2026-06-01T20:52:18Z.
- Source THS limit-up compacted `2025`: `c7d6b12f-537a-440f-9367-373a87842ea2`, succeeded at 2026-06-01T20:52:41Z.
- Source THS limit-up compacted `2026`: `b75e5996-e01e-43f8-b5e9-b226c8a4b050`, succeeded at 2026-06-01T20:53:05Z.
- ClickHouse raw snapshot job: `cb31b1bc-bfcd-4038-a18a-6f7919af15ed`, succeeded at 2026-06-01T20:53:26Z.
- ClickHouse raw BaoStock partitions `1990` through `2026`: succeeded; detailed Run ID TSV: `/tmp/mono-fleur-clickhouse-raw-baostock-20260601T205352Z.tsv`.
- ClickHouse raw EastMoney partition `1990`: failed once, fixed and rerun succeeded; see Failure 5 below.
- ClickHouse raw EastMoney partitions `1991` through `2026`: succeeded; detailed Run ID TSV: `/tmp/mono-fleur-clickhouse-raw-eastmoney-1991-2026-20260601T210413Z.tsv`.
- ClickHouse raw JiuYan market-event compacted `2026`: `1d08b55a-1729-4cfc-8891-880a29ce19b3`, succeeded at 2026-06-01T21:12:05Z.
- ClickHouse raw THS market-event compacted `2025`: `298086ea-af39-4c3a-9f98-7a3027ddf1e0`, succeeded at 2026-06-01T21:12:16Z.
- ClickHouse raw THS market-event compacted `2026`: `ec4a2326-da71-4bf4-8e92-8093ec5db3d2`, succeeded at 2026-06-01T21:12:27Z.

## Commands

Commands are executed from `pipeline/` after loading root `.env` and running `make dagster-home`.

Representative full-backfill commands:

```bash
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__action_field" --partition-range "2026-03-04...2026-06-01"
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition-range "2025-01-01...2026-01-15"
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition-range "2026-01-16...2026-06-01"
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__action_field_compacted" --partition 2026
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool_compacted" --partition 2025
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool_compacted" --partition 2026
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_snapshot_job
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_eastmoney_job --partition 1990
```

Partition loops used stop-on-first-failure wrappers around:

```bash
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_baostock_job --partition YYYY
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_eastmoney_job --partition YYYY
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_jiuyan_market_event_job --partition 2026
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_ths_market_event_job --partition YYYY
```

Detailed loop outputs:

- BaoStock raw `1990` through `2026`: `/tmp/mono-fleur-clickhouse-raw-baostock-20260601T205352Z.tsv`
- EastMoney raw `1991` through `2026`: `/tmp/mono-fleur-clickhouse-raw-eastmoney-1991-2026-20260601T210413Z.tsv`
- Market-event raw: `/tmp/mono-fleur-clickhouse-raw-market-events-20260601T211154Z.tsv`

## Validation

Final validation passed:

```bash
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

- Parquet schema validation checked 15 available objects and skipped 3 missing objects:
  - `source/jiuyan__action_field/year=2026/000000_0.parquet`
  - `source/jiuyan__industry_ocr/year=2026/000000_0.parquet`
  - `source/ths__limit_up_pool/year=2026/000000_0.parquet`
- ClickHouse schema validation checked 15 tables and skipped 0 missing tables.
- Final status: full backfill completed with all discovered issues fixed and rerun.

## Issues

### Failure 1: EastMoney dividend_main nullable `IMPL_PLAN_PROFILE`

- UTC time: 2026-06-01T19:30:21Z
- Failed job: `eastmoney__daily_job`
- Failed asset: `source/eastmoney__dividend_main`
- Partition: `1991`
- Run ID: `b827b646-9847-4f4d-91fd-23f1b4c57863`
- Command:

```bash
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 1991
```

- Failure summary: Parquet write failed because `IMPL_PLAN_PROFILE` was declared non-nullable but contained nulls.
- Validation message: `pyarrow.lib.ArrowInvalid: Column 'IMPL_PLAN_PROFILE' is declared non-nullable but contains nulls`
- First fix: changed `eastmoney__dividend_main.IMPL_PLAN_PROFILE` to nullable in source, Parquet, and ClickHouse raw contract fields.
- Rerun command:

```bash
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 1991
```

- Rerun Run ID: `57826556-7a2a-421d-921c-5409e3c527e3`
- Rerun result: failed again in `source/eastmoney__dividend_main`; Parquet write then reported `NEW_PROFILE` as declared non-nullable but containing nulls.
- Second fix: changed `eastmoney__dividend_main.NEW_PROFILE` to nullable in source, Parquet, and ClickHouse raw contract fields.
- Second rerun Run ID: `0d4b83c4-2c27-4d61-8077-2a2e589a5e71`
- Second rerun result: succeeded for `eastmoney__daily_job --partition 1991`.
- Current status: fixed; resume source year backfill from `1992`.

### Failure 2: EastMoney 1993 nullable `UPDATE_DATE`

- UTC time: 2026-06-01T19:34:52Z
- Failed job: `eastmoney__daily_job`
- Failed assets:
  - `source/eastmoney__income_ytd`
  - `source/eastmoney__balance`
- Partition: `1993`
- Run ID: `058b271f-a1ef-4ffa-bad5-6f748e561d39`
- Command:

```bash
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 1993
```

- Failure summary: Parquet writes failed because `UPDATE_DATE` was declared non-nullable but contained nulls in the 1993 payload.
- Validation message: `pyarrow.lib.ArrowInvalid: Column 'UPDATE_DATE' is declared non-nullable but contains nulls`
- Fix: changed `UPDATE_DATE` to nullable in source, Parquet, and ClickHouse raw contract fields for:
  - `eastmoney__balance`
  - `eastmoney__cashflow_sq`
  - `eastmoney__cashflow_ytd`
  - `eastmoney__income_sq`
  - `eastmoney__income_ytd`
- Validation after fix:
  - `uv run fleur-contracts validate`
  - `uv run fleur-contracts generate --check`
  - `uv run pytest contract_tools/tests/test_parquet_adapter.py scheduler/tests/unit/test_contract_schemas.py -q`
- Rerun command:

- Rerun result: failed follow-up on nullable `CURRENCY`; see below.
- Current status: superseded by the follow-up `CURRENCY` fix below.

#### Follow-up: EastMoney 1993 nullable `CURRENCY`

- UTC time: 2026-06-01T19:40Z
- Failed job: `eastmoney__daily_job`
- Failed assets:
  - `source/eastmoney__income_ytd`
  - `source/eastmoney__balance`
- Partition: `1993`
- Failure summary: the 1993 rerun exposed the same nullable-contract issue for `CURRENCY`.
- Validation message: `pyarrow.lib.ArrowInvalid: Column 'CURRENCY' is declared non-nullable but contains nulls`
- Fix: changed `CURRENCY` to nullable in source, Parquet, and ClickHouse raw contract fields for:
  - `eastmoney__balance`
  - `eastmoney__cashflow_sq`
  - `eastmoney__cashflow_ytd`
  - `eastmoney__income_sq`
  - `eastmoney__income_ytd`
- Validation after fix:
  - `uv run fleur-contracts validate`
  - `uv run fleur-contracts generate`
  - `uv run pytest contract_tools/tests/test_parquet_adapter.py scheduler/tests/unit/test_contract_schemas.py -q`
- Rerun command:

```bash
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 1993
```

- Rerun Run ID: `c97f8fad-189f-470c-ad9b-a2a413f28716`
- Rerun result: succeeded; all 8 EastMoney source assets for partition `1993` materialized through `s3_io_manager`.
- Current status: fixed; resume source year backfill from `1994`.

### Failure 3: EastMoney 2026 dividend_main nullable `TOTAL_DIVIDEND`

- UTC time: 2026-06-01T19:57:46Z
- Failed job: `eastmoney__daily_job`
- Failed asset: `source/eastmoney__dividend_main`
- Partition: `2026`
- Run ID: `0118452c-3cc0-47ed-ba3c-fde4c3322cd0`
- Command:

```bash
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 2026
```

- Failure summary: Parquet write failed because `TOTAL_DIVIDEND` was declared non-nullable but contained nulls in the 2026 payload.
- Validation message: `pyarrow.lib.ArrowInvalid: Column 'TOTAL_DIVIDEND' is declared non-nullable but contains nulls`
- Fix: changed `TOTAL_DIVIDEND` and paired `TOTAL_DIVIDEND_A` to nullable in source, Parquet, and ClickHouse raw contract fields for `eastmoney__dividend_main`.
- Validation after fix:
  - `uv run fleur-contracts validate`
  - `uv run fleur-contracts generate --check`
  - `uv run pytest contract_tools/tests/test_parquet_adapter.py scheduler/tests/unit/test_contract_schemas.py -q`
- Rerun command:

```bash
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 2026
```

- Rerun Run ID: `e26f7571-f42a-4aa0-bb02-8e0dba348857`
- Rerun UTC finished: 2026-06-01T20:01:11Z
- Rerun result: succeeded; all 8 EastMoney source assets for partition `2026` materialized through `s3_io_manager`.
- Current status: fixed; EastMoney source year backfill completed through `2026`.

### Failure 4: BaoStock 2015 nullable `isST`

- UTC time: 2026-06-01T20:16:39Z
- Failed asset: `source/baostock__query_history_k_data_plus_daily`
- Partition: `2015`
- Run ID: `29b3a919-6e99-48dd-80f6-e04f100bbc9c`
- Command:

```bash
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition 2015
```

- Failure summary: Parquet write failed because `isST` was declared non-nullable but contained nulls in the 2015 payload.
- Validation message: `pyarrow.lib.ArrowInvalid: Column 'isST' is declared non-nullable but contains nulls`
- Fix: changed `baostock__query_history_k_data_plus_daily.isST` to nullable in source, Parquet, and ClickHouse raw contract fields.
- Validation after fix:
  - `uv run fleur-contracts validate`
  - `uv run fleur-contracts generate --check`
  - `uv run pytest contract_tools/tests/test_parquet_adapter.py scheduler/tests/unit/test_contract_schemas.py -q`
- Rerun command:

```bash
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition 2015
```

- Rerun Run ID: `58d4b465-4727-43ff-95e0-5c215be59a46`
- Rerun UTC finished: 2026-06-01T20:20:04Z
- Rerun result: succeeded; `source/baostock__query_history_k_data_plus_daily` partition `2015` materialized through `s3_io_manager`.
- Current status: fixed; resume BaoStock source year backfill from `2016`.

#### Follow-up: BaoStock 2016 interrupted while stopping runaway loop

- UTC time: 2026-06-01T20:17:37Z
- Asset: `source/baostock__query_history_k_data_plus_daily`
- Partition: `2016`
- Run ID: `e7159e8e-e76d-405d-9d54-887ac239dd13`
- Result: failed due to manual process termination after the parent shell loop continued past the 2015 schema failure.
- Rerun Run ID: `ba56ef08-91b9-4288-af85-2e4f6dd9fd23`
- Rerun UTC finished: 2026-06-01T20:21:18Z
- Rerun result: succeeded; `source/baostock__query_history_k_data_plus_daily` partition `2016` materialized through `s3_io_manager`.
- Current status: fixed; resume BaoStock source year backfill from `2017`.

### Failure 5: EastMoney raw empty 1990 partitions validated as year 0

- UTC time: 2026-06-01T21:02:14Z
- Failed job: `clickhouse__raw_sync_eastmoney_job`
- Partition: `1990`
- Run ID: `e950d8ff-2f3f-4155-943d-204d89a15f5d`
- Command:

```bash
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_eastmoney_job --partition 1990
```

- Failure summary: ClickHouse raw sync failed for EastMoney source partitions that legitimately staged zero rows for 1990. ClickHouse aggregate validation returned `min(year)=0` and `max(year)=0` for empty staging tables, and raw sync rejected that as a partition range mismatch.
- Validation messages:
  - `RuntimeError: Staging table eastmoney__balance__stage contains partition range 0..0, expected 1990`
  - Same pattern for `eastmoney__cashflow_sq`, `eastmoney__cashflow_ytd`, `eastmoney__dividend_allotment`, `eastmoney__dividend_main`, `eastmoney__income_sq`, and `eastmoney__income_ytd`.
- Diagnosis:
  - `eastmoney__equity_history__stage` was non-empty and validated correctly as `1990..1990`.
  - `eastmoney__balance__stage` had `count()=0`, `min(year)=0`, `max(year)=0`.
  - The affected EastMoney raw specs have `allow_empty=True`, so an empty source partition should be allowed and should replace the raw partition with zero rows.
- Fix: patched `RawSyncService._validate_schema_and_year_partition()` to return `0` before min/max comparison when `row_count == 0` and `spec.allow_empty` is true.
- Validation after fix:
  - `uv run pytest scheduler/tests/unit/clickhouse/test_raw_sync.py -q`
  - `uv run ruff check scheduler/src/scheduler/defs/clickhouse/raw_sync.py scheduler/tests/unit/clickhouse/test_raw_sync.py`
- Rerun command:

```bash
uv run dg launch --target-path scheduler --job clickhouse__raw_sync_eastmoney_job --partition 1990
```

- Rerun Run ID: `99207d09-d516-4d70-a0ec-f18dfcf076f3`
- Rerun UTC finished: 2026-06-01T21:03:49Z
- Rerun result: succeeded; empty EastMoney raw year partitions are accepted when the raw spec has `allow_empty=True`, and non-empty sibling tables still validate year `1990`.
