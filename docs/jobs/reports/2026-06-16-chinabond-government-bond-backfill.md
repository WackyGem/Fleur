# ChinaBond government bond backfill

UTC time: 2026-06-16T20:22:16Z

## Scope

- Plan: `docs/plans/archive/0042-chinabond-government-bond-s3-raw-implementation-plan.md`
- Dataset: `chinabond__government_bond`
- Source asset: `source/chinabond__government_bond`
- Raw asset: `clickhouse/raw/chinabond__government_bond`
- Source storage: `source/chinabond__government_bond/year=YYYY/000000_0.parquet`
- ClickHouse raw table: `fleur_raw.chinabond__government_bond`
- Partition range: `2006` through `2026`
- Environment file: `/storage/program/fleur/.env`

## Commands

Small-batch source/S3:

```bash
cd pipeline
set -a
. /storage/program/fleur/.env
set +a
uv run dg launch --target-path scheduler --assets "key:source/chinabond__government_bond" --partition 2026
uv run dg launch --target-path scheduler --assets "key:source/chinabond__government_bond" --partition 2006
```

Full source/S3:

```bash
cd pipeline
set -a
. /storage/program/fleur/.env
set +a
for year in 2007 2008 2009 2010 2011 2012 2013 2014 2015 2016 2017 2018 2019 2020 2021 2022 2023 2024 2025; do
  uv run dg launch --target-path scheduler --assets "key:source/chinabond__government_bond" --partition "$year"
done
```

Small-batch and full ClickHouse raw sync:

```bash
cd pipeline
set -a
. /storage/program/fleur/.env
set +a
for year in 2026 2006 2007 2008 2009 2010 2011 2012 2013 2014 2015 2016 2017 2018 2019 2020 2021 2022 2023 2024 2025; do
  uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/chinabond__government_bond" --partition "$year"
done
```

Validation:

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

## Run IDs

| Year | Source/S3 Run ID | ClickHouse raw Run ID |
| --- | --- | --- |
| 2006 | `75995950-5a0a-48be-9df8-f2d2474bafc1` | `8be99342-5a8c-4ade-978e-f4a28611201f` |
| 2007 | `9a9b12e5-4c9c-46ee-9c6a-2b8da3e41b4f` | `f8b6b664-bdbd-4f1f-943a-20b1c8ad9ca6` |
| 2008 | `7622c87d-0204-4c4b-971a-dcef7f063e78` | `9a18d3dd-cb00-4d79-95a8-df51b5370db2` |
| 2009 | `9143899b-cf4a-4c5d-9ab2-5ce8333594b7` | `971334ac-fc0a-4102-a733-149960bf1124` |
| 2010 | `f12873af-d6d9-4b3b-9ad7-3f302b34ed2c` | `bb56e2f6-ade9-42a1-be1b-909b3009cdf1` |
| 2011 | `7b4b559f-6a44-4d3b-992a-a501b051f496` | `c6dd7801-9d36-4e18-b276-61b800fbba4b` |
| 2012 | `322a6fe1-c73a-4ce9-aab0-f069343d7221` | `996e5b21-7e91-4f77-89ea-086e0495d540` |
| 2013 | `2280cf46-b340-423a-8b20-dc63a911925c` | `c254b362-788f-4211-8a6e-a441cb1640e0` |
| 2014 | `1eca6b15-2efc-4fb9-9e3b-3ac72ed7cb56` | `301de36b-3f81-485a-b639-f9cf3002c8dc` |
| 2015 | `186338ca-cb7d-4482-82c8-2a2e4ee55e29` | `87f9b5c1-d155-4641-bc15-30f08d435de5` |
| 2016 | `1174bac7-161d-4c7e-864b-549a8ad990d3` | `10b43888-f180-4f21-8aa7-4c6661f4ac96` |
| 2017 | `ef904211-4797-4bbe-932a-13c4ac89bc96` | `6fbe7b7e-c46a-4055-9f76-9543f4a4174c` |
| 2018 | `24b78596-54b5-468b-910b-dd4efb4b2d40` | `3c710c87-5ca4-425e-b1dc-ad2057d5a70f` |
| 2019 | `27c1828a-de84-4149-874f-cf5ae816f563` | `99575480-91ca-448c-93a7-615a0fcebdb6` |
| 2020 | `fea42a70-3c60-41ff-afdc-719b4862cdf4` | `c11cb2ba-9c36-4216-957f-bb45196e45fb` |
| 2021 | `bb80d4b6-921d-4f7e-b8bc-20deb2401bb6` | `32b2f57b-1eec-450a-a0a4-7e2301f71179` |
| 2022 | `095cb68d-f010-475f-bea9-83c078a64154` | `39945703-c6b1-48d9-96ee-70a24fac598b` |
| 2023 | `e8802fb1-4a51-493a-b386-635c4416a540` | `8da228ae-cefa-4739-869d-7265a2d1a4ef` |
| 2024 | `0a28e9c7-34df-4411-9639-9754fa26e699` | `344a657a-1cdb-4923-94f6-6854e975530d` |
| 2025 | `f7040ad6-7d67-4685-9196-8881b6698544` | `606a01d9-7e46-4292-8514-0a1716aba071` |
| 2026 | `03921fa5-1804-46c3-805e-dc80366f8132` | `11ed0ada-ece9-4a17-ab7f-6271ff460a8b` |

## Row Counts

| Year | S3 rows | Raw rows | Min work date | Max work date |
| --- | ---: | ---: | --- | --- |
| 2006 | 214 | 214 | 2006-03-01 | 2006-12-31 |
| 2007 | 249 | 249 | 2007-01-04 | 2007-12-29 |
| 2008 | 251 | 251 | 2008-01-02 | 2008-12-31 |
| 2009 | 250 | 250 | 2009-01-04 | 2009-12-31 |
| 2010 | 250 | 250 | 2010-01-04 | 2010-12-31 |
| 2011 | 250 | 250 | 2011-01-04 | 2011-12-31 |
| 2012 | 249 | 249 | 2012-01-04 | 2012-12-31 |
| 2013 | 250 | 250 | 2013-01-04 | 2013-12-31 |
| 2014 | 250 | 250 | 2014-01-02 | 2014-12-31 |
| 2015 | 249 | 249 | 2015-01-04 | 2015-12-31 |
| 2016 | 251 | 251 | 2016-01-04 | 2016-12-31 |
| 2017 | 251 | 251 | 2017-01-03 | 2017-12-31 |
| 2018 | 252 | 252 | 2018-01-02 | 2018-12-31 |
| 2019 | 250 | 250 | 2019-01-02 | 2019-12-31 |
| 2020 | 249 | 249 | 2020-01-02 | 2020-12-31 |
| 2021 | 250 | 250 | 2021-01-04 | 2021-12-31 |
| 2022 | 250 | 250 | 2022-01-04 | 2022-12-31 |
| 2023 | 250 | 250 | 2023-01-03 | 2023-12-31 |
| 2024 | 251 | 251 | 2024-01-02 | 2024-12-31 |
| 2025 | 248 | 248 | 2025-01-02 | 2025-12-31 |
| 2026 | 111 | 111 | 2026-01-04 | 2026-06-16 |
| Total | 5,075 | 5,075 | 2006-03-01 | 2026-06-16 |

## Validation Results

- `uv run fleur-contracts validate-parquet --all-available`: passed.
  - Checked 17 available Parquet objects.
  - Skipped 3 unrelated missing sparse daily objects.
- `uv run fleur-contracts validate-clickhouse --all-available`: passed.
  - Checked 17 ClickHouse raw tables.
  - Skipped 0 missing tables.
- Raw row counts match S3 row counts for every ChinaBond year partition.

## Outcome

Backfill completed successfully for all `2006` through `2026` ChinaBond government bond partitions. No failed or accepted partitions remain.
