# 010 - baostock__query_history_k_data_plus_daily

## 状态

- 结果：成功
- 覆盖年份：1995-2026
- 第一阶段时间：2026-05-31 06:27:14 UTC - 2026-05-31 06:36:32 UTC
- 第二阶段时间：2026-05-31 06:43:59 UTC - 2026-05-31 07:06:55 UTC
- Dagster home：`/storage/program/mono-fleur/.dagster`
- 本轮日志：`/tmp/baostock_history_2010_2025_20260531064359.log`

## Run ID

| 年份 | Run ID |
| --- | --- |
| 1995 | `cbdf0adf-c9da-4148-9b8d-a3b71c573618` |
| 1996 | `7b4be34e-573e-4521-8834-4fdda1fb80de` |
| 1997 | `a3f01fed-1cf7-48c6-ac1a-aaef09881fd7` |
| 1998 | `bd5b384b-25dd-4d9e-90c3-10369d6522c6` |
| 1999 | `0a0968fa-e701-4336-820c-60c0d48cd6e6` |
| 2000 | `42fd9f0e-c4b1-4fd6-80a8-098695e897ec` |
| 2001 | `985b1d0e-1f0d-431c-a440-0e372bc9593f` |
| 2002 | `f96fab86-f07b-403a-9cda-bd89f428152a` |
| 2003 | `0ab9b4ba-7daa-47e2-8416-09b620939097` |
| 2004 | `72ddafa7-0854-4aa8-8981-46d2f02df095` |
| 2005 | `3a0c68e5-4021-4d9e-afcf-6596ba06a537` |
| 2006 | `16bd8f2b-a2d5-4691-957a-c23aea257d91` |
| 2007 | `b3373433-24d8-41ae-b348-aaf868a0c6dc` |
| 2008 | `0127f594-f7b3-4cd2-a3fa-91af5b93f7b8` |
| 2009 | `bed50a25-0fba-425d-ac25-d7acdd99baac` |
| 2010 | `cb5918a1-75d3-469b-a470-e03b5abb8a5d` |
| 2011 | `0c8e7109-50e4-4d36-853d-4311911fef3e` |
| 2012 | `39f69fc6-dc25-4d09-bb2c-600cba209c54` |
| 2013 | `121bc9cf-8126-4368-abb0-d9503f08c827` |
| 2014 | `4696659f-1ea9-40a2-8efd-ff2788ac115f` |
| 2015 | `53130d2c-5885-4a23-bafa-3293915dfb10` |
| 2016 | `8b66ad64-c7d5-43c7-a513-0fc865a9d9e8` |
| 2017 | `7ead1bd4-800d-4cdb-85ce-b7e07b010c12` |
| 2018 | `0cfcaf9d-0cc8-4d52-a939-b6132ae82afd` |
| 2019 | `722805f1-d1b2-499d-9959-8e301575ace4` |
| 2020 | `c9afbeea-19ac-4b61-bede-b36de1f56a1a` |
| 2021 | `2be01378-2467-4fcd-852c-f8ace045f1ef` |
| 2022 | `0467894b-7f2b-4998-8789-5cebc6b21b77` |
| 2023 | `bfe5c7ec-ff45-428e-aa59-8d4f8ecb1ae5` |
| 2024 | `70f3403e-4e8f-4096-a68e-838233133c1c` |
| 2025 | `a3969e83-509a-4baf-935b-78a2751a63b5` |
| 2026 | `afc67081-ba03-46c8-af05-c391ff80c7ee` |

## 命令

```bash
cd pipeline
for year in $(seq 1995 2009); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/baostock__query_history_k_data_plus_daily" \
    --partition "$year"
done
```

```bash
cd pipeline
for year in $(seq 2010 2025); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/baostock__query_history_k_data_plus_daily" \
    --partition "$year"
done
```

说明：2026 年在本轮继续执行前已有成功物化记录，最终核验时纳入完整范围。

## 关键输出

- 2010-2025 每个年份都 `RUN_SUCCESS`
- 2010-2025 每个年份都产生 1 个 `ASSET_MATERIALIZATION`
- 2010-2025 单年耗时约 52s 到 115s

## 核验

使用 Dagster 事件库 `.dagster/history/runs/index.db` 查询 `source/baostock__query_history_k_data_plus_daily` 的成功物化事件：

- `materialized_count=32`
- `missing=none`
- 覆盖分区：1995-2026

## 备注

- 该资产是年分区资产，回填必须按 `--partition YYYY` 循环。
- 2010 年曾有一次取消记录，已在本轮重跑成功。
- 2026 年曾有一次失败记录，已有后续成功重跑记录，最终核验以成功物化为准。
