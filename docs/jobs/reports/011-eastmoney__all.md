# 011 - eastmoney__all

## 状态

- 结果：成功
- 覆盖资产：8 个 Eastmoney 年分区资产
- 覆盖年份：1990-2026
- 重跑时间：2026-05-31 07:30:47 UTC - 2026-05-31 07:43:18 UTC
- Dagster home：`/storage/program/mono-fleur/.dagster`
- 重跑日志目录：`/tmp/eastmoney_full_rerun_20260531073047`

## 资产

- `source/eastmoney__balance`
- `source/eastmoney__cashflow_sq`
- `source/eastmoney__cashflow_ytd`
- `source/eastmoney__dividend_allotment`
- `source/eastmoney__dividend_main`
- `source/eastmoney__equity_history`
- `source/eastmoney__income_sq`
- `source/eastmoney__income_ytd`

## 命令

```bash
cd pipeline
selection="key:source/eastmoney__balance or key:source/eastmoney__cashflow_sq or key:source/eastmoney__cashflow_ytd or key:source/eastmoney__dividend_allotment or key:source/eastmoney__dividend_main or key:source/eastmoney__equity_history or key:source/eastmoney__income_sq or key:source/eastmoney__income_ytd"
for year in $(seq 1990 2026); do
  uv run dg launch --target-path scheduler --assets "$selection" --partition "$year"
done
```

## 本轮 Run ID

| 年份 | Run ID | 物化数 |
| --- | --- | ---: |
| 1990 | `5ac68800-cc1a-4626-80e9-206c94055227` | 8 |
| 1991 | `7911ce2a-2a18-4273-8f43-137b7e050063` | 8 |
| 1992 | `1097b78b-db02-4078-8539-bbd43c66071b` | 8 |
| 1993 | `9fa92876-a05e-45f7-ba46-087429e6f217` | 8 |
| 1994 | `7ccaca57-f93f-4987-a016-33dbd44220af` | 8 |
| 1995 | `34e36445-883f-4ad9-9a4a-23149aee7978` | 8 |
| 1996 | `2b82b1e3-2608-495f-9025-0c749f9d327f` | 8 |
| 1997 | `44a1f52b-314b-4f43-842f-fcff70397d74` | 8 |
| 1998 | `1cca6153-2c00-4abe-aab1-b619fccb8004` | 8 |
| 1999 | `7892bdf6-dfb1-47d9-b925-e228234fe237` | 8 |
| 2000 | `b8fde624-7663-48ce-9cf3-898f6785bcbb` | 8 |
| 2001 | `2d277c68-1597-4bcf-9cf2-d1463168b331` | 8 |
| 2002 | `4e9c2b1a-2932-4dff-9c4b-7e05d0e91c61` | 8 |
| 2003 | `b7a21a6b-d6bb-4d38-a91c-10f4960f417b` | 8 |
| 2004 | `93bf7e58-d120-4c43-9cc1-d0063c4acfcb` | 8 |
| 2005 | `e9a683e4-2073-4b82-bdaa-8ad0fd8bb5ea` | 8 |
| 2006 | `516d66b2-7362-4217-9a79-344b9b137b3b` | 8 |
| 2007 | `d2f6c6bd-98bb-46ed-84f0-c4ef08b82acb` | 8 |
| 2008 | `a0015310-1b71-4acf-b621-9623574518e9` | 8 |
| 2009 | `09cfb158-d144-4055-be67-071ae9e04cdf` | 8 |
| 2010 | `56db66ab-b97c-47eb-82aa-b3217eb5795a` | 8 |
| 2011 | `0844518c-6ddc-402a-8ddc-38c700cb30fc` | 8 |
| 2012 | `d3e5a9b2-19b2-4eee-9e88-9b618d20a70a` | 8 |
| 2013 | `7f0209a0-9857-4f72-afe2-9f178996180e` | 8 |
| 2014 | `9188610b-d306-4bef-84bd-2a4cc1f90d30` | 8 |
| 2015 | `8505f6ce-5ec3-4d10-ab28-d0b119d8a466` | 8 |
| 2016 | `efa781e4-1f7d-4266-8748-5c7f35f241b4` | 8 |
| 2017 | `5c4a0db7-3d52-461b-acf6-6cb425b721c1` | 8 |
| 2018 | `f2df00ac-6304-4609-8b4d-3239fa46c140` | 8 |
| 2019 | `402b4238-6e5a-49e1-874a-1a74e8224c44` | 8 |
| 2020 | `3f92f69d-b645-453d-9649-1ebc2ccd66fc` | 8 |
| 2021 | `1d8fd70a-ec16-4a1a-8141-8c8fe4e9b469` | 8 |
| 2022 | `08f26ac0-8371-42f1-bb7a-3bd8e9d0dc92` | 8 |
| 2023 | `9f9e537e-12a8-4571-bf33-05e0d9dbe218` | 8 |
| 2024 | `171dbde2-e07b-4789-bd63-6fc51733bddd` | 8 |
| 2025 | `bfc07537-4925-4070-8b58-f854b2e1a307` | 8 |
| 2026 | `d195c163-d68a-4623-ab85-8f03a519d977` | 8 |

## 核验

使用本轮日志目录解析出的 37 个 Run ID 查询 Dagster 事件库 `.dagster/history/runs/index.db`：

- `run_count=37`
- 每个年份对应 1 个本轮 run
- 每个本轮 run 在对应年份都有 8 个 `ASSET_MATERIALIZATION`
- `all_ok=True`

## 备注

- 本次按用户要求全量重跑 8 个 Eastmoney 资产，不复用历史 Dagster 物化记录作为完成依据。
- `eastmoney__dividend_main.REPORT_TIME` 历史值可能为 `1991年报`，已将 schema 修正为 string 并通过单元测试。
