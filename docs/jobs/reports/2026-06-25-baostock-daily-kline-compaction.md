# BaoStock daily K-line compaction migration

UTC time: 2026-06-25T20:42:51Z

## Scope

- Plan: `docs/plans/archive/0054-baostock-daily-kline-daily-source-compaction-plan.md`
- Daily source asset: `source/baostock__query_history_k_data_plus_daily`
- Compacted source asset: `source/baostock__query_history_k_data_plus_daily_compacted`
- Raw asset: `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`
- Old S3 object pattern: `source/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet`
- New S3 object pattern: `source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet`
- ClickHouse raw table: `fleur_raw.baostock__query_history_k_data_plus_daily_compacted`
- Environment file: `/storage/program/fleur/.env`

## S3 Migration

Existing dev S3 year partitions were copied from the old daily source prefix to the new compacted prefix for `1990` through `2026`.

Validation found one physical Parquet schema difference in old objects for `1990` through `2014`: `isST` was encoded as `bool not null`, while the current compacted contract requires nullable `bool`. The source payload field is not required in the contract, and ClickHouse raw is `Nullable(Bool)`, so the compacted copies for `1990` through `2014` were rewritten from the copied data to normalize Parquet schema metadata. No business fields or row values were transformed.

After validation passed, old `year=*` objects were deleted from `source/baostock__query_history_k_data_plus_daily`. The daily source prefix no longer contains historical year partitions.

## S3 Row Counts

| Year | Old rows | New rows | New schema | Notes |
| --- | ---: | ---: | --- | --- |
| 1990 | 72 | 72 | contract match | `isST` nullability normalized |
| 1991 | 2,977 | 2,977 | contract match | `isST` nullability normalized |
| 1992 | 7,878 | 7,878 | contract match | `isST` nullability normalized |
| 1993 | 26,404 | 26,404 | contract match | `isST` nullability normalized |
| 1994 | 66,282 | 66,282 | contract match | `isST` nullability normalized |
| 1995 | 75,156 | 75,156 | contract match | `isST` nullability normalized |
| 1996 | 95,794 | 95,794 | contract match | `isST` nullability normalized |
| 1997 | 155,054 | 155,054 | contract match | `isST` nullability normalized |
| 1998 | 191,753 | 191,753 | contract match | `isST` nullability normalized |
| 1999 | 209,625 | 209,625 | contract match | `isST` nullability normalized |
| 2000 | 234,492 | 234,492 | contract match | `isST` nullability normalized |
| 2001 | 266,683 | 266,683 | contract match | `isST` nullability normalized |
| 2002 | 277,146 | 277,146 | contract match | `isST` nullability normalized |
| 2003 | 297,217 | 297,217 | contract match | `isST` nullability normalized |
| 2004 | 320,846 | 320,846 | contract match | `isST` nullability normalized |
| 2005 | 329,542 | 329,542 | contract match | `isST` nullability normalized |
| 2006 | 346,887 | 346,887 | contract match | `isST` nullability normalized |
| 2007 | 377,146 | 377,146 | contract match | `isST` nullability normalized |
| 2008 | 415,760 | 415,760 | contract match | `isST` nullability normalized |
| 2009 | 434,231 | 434,231 | contract match | `isST` nullability normalized |
| 2010 | 513,561 | 513,561 | contract match | `isST` nullability normalized |
| 2011 | 614,219 | 614,219 | contract match | `isST` nullability normalized |
| 2012 | 688,240 | 688,240 | contract match | `isST` nullability normalized |
| 2013 | 701,192 | 701,192 | contract match | `isST` nullability normalized |
| 2014 | 744,937 | 744,937 | contract match | `isST` nullability normalized |
| 2015 | 802,478 | 802,478 | contract match | copied object already matched |
| 2016 | 844,934 | 844,934 | contract match | copied object already matched |
| 2017 | 939,565 | 939,565 | contract match | copied object already matched |
| 2018 | 995,754 | 995,754 | contract match | copied object already matched |
| 2019 | 1,024,115 | 1,024,115 | contract match | copied object already matched |
| 2020 | 1,084,680 | 1,084,680 | contract match | copied object already matched |
| 2021 | 1,194,894 | 1,194,894 | contract match | copied object already matched |
| 2022 | 1,280,813 | 1,280,813 | contract match | copied object already matched |
| 2023 | 1,343,433 | 1,343,433 | contract match | copied object already matched |
| 2024 | 1,367,325 | 1,367,325 | contract match | copied object already matched |
| 2025 | 1,377,046 | 1,377,046 | contract match | copied object already matched |
| 2026 | 113 | 113 | contract match | incomplete dev snapshot; one code only |
| Total | 19,648,244 | 19,648,244 | contract match | 37 year objects |

## Raw Sync

Commands:

```bash
cd pipeline
set -a
. /storage/program/fleur/.env
set +a
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026
for year in $(seq 1990 2025); do
  uv run dg launch --target-path scheduler \
    --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted" \
    --partition "$year"
done
```

`make dagster-home` could not run in this worktree because the Makefile requires `DAGSTER_HOME` to originate from a local `.env` file, and this isolated worktree intentionally does not contain `.env`. The equivalent target actions were run manually after loading `/storage/program/fleur/.env`: create `DAGSTER_HOME`, then create the default `dagster.yaml` if missing.

Raw sync run IDs:

| Year | Run ID |
| --- | --- |
| 1990 | `35931f2c-ef54-4069-9d64-ed84ed2d3d2c` |
| 1991 | `1e2a9104-a364-4b85-aa8a-a1d3dceb575d` |
| 1992 | `522dcee0-4bd8-4058-961b-ca586c90711c` |
| 1993 | `bdeb51a3-cc2f-41b8-b306-4c646bb945e9` |
| 1994 | `b3ffb14f-f083-4466-8ec7-00080ad61474` |
| 1995 | `e960f427-bdd9-4dde-8814-afd5b88da78f` |
| 1996 | `b049cb78-f558-4bf4-ac26-75933f51ef00` |
| 1997 | `1f68406a-178b-4118-a239-7773d0ec8c5d` |
| 1998 | `37e076ef-1d85-4c12-b076-7f6be2ec4ded` |
| 1999 | `674347ba-9e79-4f88-a2d0-10769853ec3c` |
| 2000 | `19d51f1d-b3da-4393-80c0-4a1d4eecfdaa` |
| 2001 | `74ab7568-9475-4ab1-b59f-70bb8abb2c4a` |
| 2002 | `8587e67b-2829-4f1b-88f4-874341a9d935` |
| 2003 | `e4330964-ead7-4e56-a398-2722cba581f0` |
| 2004 | `d1cf1522-cfaa-4516-9e37-89fc6290eb4e` |
| 2005 | `927fb1ba-d057-41d8-be3f-f3a7d21fdba7` |
| 2006 | `13c99f33-7f19-48f8-9ffa-288f6014b0d3` |
| 2007 | `46243647-63f5-423e-9c6c-f1081b1328c3` |
| 2008 | `39745f16-3db6-40dc-bb93-07f6f4892393` |
| 2009 | `e06e9fdf-9f23-4612-9583-ee6d5a3c0856` |
| 2010 | `f53f55b6-bd4b-4112-b11f-a426052fe1ae` |
| 2011 | `aa8e7b41-88c8-4950-9776-25c6552daede` |
| 2012 | `94ff2670-c953-4f5a-b519-49f4dfd42223` |
| 2013 | `f71c35cf-7ee7-4daa-9ceb-73ee07f9fd5d` |
| 2014 | `315abf97-af4c-4671-8c16-de1d8a1c49d2` |
| 2015 | `ab70f59e-c114-4833-98ce-831db362d762` |
| 2016 | `a4bca7d5-3869-46f3-926d-d3ee0721684a` |
| 2017 | `0a990ed2-43c6-49df-a5b6-ace9c0950212` |
| 2018 | `2e236cee-3f54-49c7-9bd5-73628439aba4` |
| 2019 | `8496b620-1875-409d-9c9d-c0e900b79d99` |
| 2020 | `e5f3102c-c012-4a82-b6b6-8018f1b26f9b` |
| 2021 | `afdbebc0-5e37-4338-9a46-15f02bf5db8a` |
| 2022 | `25e2715d-dcc6-46a9-a362-e3cd826827a6` |
| 2023 | `5e73e914-454c-435e-b965-69b30d1ef220` |
| 2024 | `045805bf-356b-49d1-bff9-eece6ba60cee` |
| 2025 | `ead59db6-85ac-403c-bad3-00814d1395e0` |
| 2026 | `c060dabd-1055-4e5d-80b6-aa2a93ac7c33` |

## ClickHouse Validation

| Year | S3 rows | Raw rows | Max date | Unique codes |
| --- | ---: | ---: | --- | ---: |
| 1990 | 72 | 72 | 1990-12-31 | 8 |
| 1991 | 2,977 | 2,977 | 1991-12-31 | 13 |
| 1992 | 7,878 | 7,878 | 1992-12-31 | 53 |
| 1993 | 26,404 | 26,404 | 1993-12-31 | 178 |
| 1994 | 66,282 | 66,282 | 1994-12-30 | 290 |
| 1995 | 75,156 | 75,156 | 1995-12-29 | 314 |
| 1996 | 95,794 | 95,794 | 1996-12-31 | 517 |
| 1997 | 155,054 | 155,054 | 1997-12-31 | 723 |
| 1998 | 191,753 | 191,753 | 1998-12-31 | 829 |
| 1999 | 209,625 | 209,625 | 1999-12-30 | 927 |
| 2000 | 234,492 | 234,492 | 2000-12-29 | 1,062 |
| 2001 | 266,683 | 266,683 | 2001-12-31 | 1,141 |
| 2002 | 277,146 | 277,146 | 2002-12-31 | 1,209 |
| 2003 | 297,217 | 297,217 | 2003-12-31 | 1,269 |
| 2004 | 320,846 | 320,846 | 2004-12-31 | 1,365 |
| 2005 | 329,542 | 329,542 | 2005-12-30 | 1,370 |
| 2006 | 346,887 | 346,887 | 2006-12-29 | 1,500 |
| 2007 | 377,146 | 377,146 | 2007-12-28 | 1,643 |
| 2008 | 415,760 | 415,760 | 2008-12-31 | 1,726 |
| 2009 | 434,231 | 434,231 | 2009-12-31 | 1,904 |
| 2010 | 513,561 | 513,561 | 2010-12-31 | 2,320 |
| 2011 | 614,219 | 614,219 | 2011-12-30 | 2,704 |
| 2012 | 688,240 | 688,240 | 2012-12-31 | 2,940 |
| 2013 | 701,192 | 701,192 | 2013-12-31 | 2,993 |
| 2014 | 744,937 | 744,937 | 2014-12-31 | 3,129 |
| 2015 | 802,478 | 802,478 | 2015-12-31 | 3,392 |
| 2016 | 844,934 | 844,934 | 2016-12-30 | 3,612 |
| 2017 | 939,565 | 939,565 | 2017-12-29 | 4,049 |
| 2018 | 995,754 | 995,754 | 2018-12-28 | 4,147 |
| 2019 | 1,024,115 | 1,024,115 | 2019-12-31 | 4,339 |
| 2020 | 1,084,680 | 1,084,680 | 2020-12-31 | 4,702 |
| 2021 | 1,194,894 | 1,194,894 | 2021-12-31 | 5,169 |
| 2022 | 1,280,813 | 1,280,813 | 2022-12-30 | 5,492 |
| 2023 | 1,343,433 | 1,343,433 | 2023-12-29 | 5,686 |
| 2024 | 1,367,325 | 1,367,325 | 2024-12-31 | 5,715 |
| 2025 | 1,377,046 | 1,377,046 | 2025-12-31 | 5,755 |
| 2026 | 113 | 113 | 2026-06-25 | 1 |
| Total | 19,648,244 | 19,648,244 | 2026-06-25 |  |

Additional validation:

- S3 compacted year object count: 37.
- Old daily source year object count after cleanup: 0.
- ClickHouse raw year count: 37.
- `type = 5` rows after joining `fleur_raw.baostock__query_stock_basic`: 0.
- 2026 is an incomplete dev snapshot with 113 rows and one code (`sh.000001`); it was migrated for path consistency but is not a complete 2026 historical data validation.

## dbt Validation

Targeted build command:

```bash
cd pipeline
set -a
. /storage/program/fleur/.env
set +a
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily+
```

Result:

- Completed successfully.
- PASS=72, WARN=0, ERROR=0, SKIP=0, TOTAL=72.
- Built 1 staging view, 8 downstream table models, and 60 data tests.

## Outcome

Plan 0054 migration is complete in dev:

- Daily BaoStock source prefix is now reserved for `trade_date=*` partitions.
- Historical year partitions live under `source/baostock__query_history_k_data_plus_daily_compacted/year=*`.
- New ClickHouse raw table `fleur_raw.baostock__query_history_k_data_plus_daily_compacted` is populated for `1990` through `2026`.
- dbt staging keeps the model name `stg_baostock__query_history_k_data_plus_daily` and reads the compacted raw table.
