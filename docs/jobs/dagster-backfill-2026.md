# Dagster 回填执行计划

状态：部分被统一入口替代

说明：Source 到 ClickHouse raw 的新手动入口已实现为 `backfill__fetch_sources_to_raw_job`。本文保留 2026 年早期逐资产命令和执行顺序作为历史/诊断参考；后续 source/raw 回填优先使用统一入口，并在真实运行后补充新的运行报告。

统一入口 dry-run 示例：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_sources_to_raw_controller":{"config":{"target_scope":"baostock_daily_kline","start_date":"2026-01-01","end_date":"2026-06-30","dry_run":true}}}}'
```

## 目标

本计划用于执行以下回填：

1. `sina__trade_calendar`
2. `baostock__query_stock_basic`
3. `jiuyan__industry_list`
4. `jiuyan__industry_images`
5. `jiuyan__industry_ocr`，本次限制回填 50 张图片
6. `jiuyan__action_field`，范围为最近 90 个自然日，内部过滤交易日
7. `jiuyan__action_field_compacted`
8. `ths__limit_up_pool`，范围为全部日分区，内部过滤交易日
9. `ths__limit_up_pool_compacted`
10. `eastmoney__*` 全部东方财富资产，范围为 `1990` 至今年
11. `baostock__query_history_k_data_plus_daily`，范围为 `1995` 至今年

## 职责区分

- `docs/skills/fleur-dagster-backfill-runbook` 维护通用回填命令模板、选择规则、分区规则
- `docs/jobs/dagster-backfill-2026.md` 记录这一次具体执行计划、顺序、范围和操作记录
- `pipeline/scheduler` 负责实际资产、job、schedule 定义
- 所有 `dg` / `dagster` 命令必须使用根目录 `.env` 中的 `DAGSTER_HOME` 作为 Dagster home
- Eastmoney 并行度由 Dagster pool 控制，当前按 `eastmoney_run_pool = 3` 执行

## 前置检查

先确认根目录 `.env` 中已配置 `DAGSTER_HOME`，并让当前 shell 使用该值：

```bash
set -a
. ./.env
set +a
test -n "$DAGSTER_HOME"
```

然后初始化/确认 Dagster home 和 pool 限制：

```bash
make dagster-home
```

再在 `pipeline/` 下执行：

```bash
uv run dg list defs --target-path scheduler --json
```

确认以下对象存在：

- `sina__trade_calendar_job`
- `eastmoney__daily_job`
- `source/jiuyan__industry_list`
- `source/jiuyan__industry_images`
- `source/jiuyan__industry_ocr`
- `source/jiuyan__action_field`
- `source/jiuyan__action_field_compacted`
- `source/ths__limit_up_pool`
- `source/ths__limit_up_pool_compacted`
- `source/baostock__query_stock_basic`
- `source/baostock__query_history_k_data_plus_daily`

确认 Eastmoney 资产集合至少包含：

- `source/eastmoney__balance`
- `source/eastmoney__cashflow_sq`
- `source/eastmoney__cashflow_ytd`
- `source/eastmoney__dividend_allotment`
- `source/eastmoney__dividend_main`
- `source/eastmoney__equity_history`
- `source/eastmoney__income_sq`
- `source/eastmoney__income_ytd`

## 执行顺序

1. 先回填 `sina__trade_calendar`
2. 再回填 `baostock__query_stock_basic`
3. 再回填 `jiuyan__industry_list`
4. 再回填 `jiuyan__industry_images`
5. 再回填 `jiuyan__industry_ocr`，限制 50 张图片
6. 再回填 `jiuyan__action_field`，最近 90 个自然日
7. 再回填 `jiuyan__action_field_compacted`
8. 再回填 `ths__limit_up_pool`，全部日分区
9. 再回填 `ths__limit_up_pool_compacted`
10. 再回填 `eastmoney__*` 全部资产
11. 最后回填 `baostock__query_history_k_data_plus_daily`

理由：

- `sina__trade_calendar` 是交易日事实来源
- `baostock__query_stock_basic` 是 Eastmoney 和 BaoStock 日线的上游依赖
- `jiuyan__industry_images` 依赖 `jiuyan__industry_list`
- `jiuyan__industry_ocr` 依赖 `jiuyan__industry_images`
- `jiuyan__action_field` 依赖 `sina__trade_calendar` 作为交易日过滤事实来源
- `jiuyan__action_field_compacted` 依赖 `jiuyan__action_field` 和 `sina__trade_calendar`
- `ths__limit_up_pool` 依赖 `sina__trade_calendar` 作为交易日过滤事实来源
- `ths__limit_up_pool_compacted` 依赖 `ths__limit_up_pool` 和 `sina__trade_calendar`
- Eastmoney 依赖股票基础信息，应该在基础信息之后执行
- `baostock__query_history_k_data_plus_daily` 依赖 `baostock__query_stock_basic` 和交易日历，放在后面最稳
- `jiuyan__industry_images` 若单独回填，使用精确 asset selection；不要直接用 `jiuyan__industry_ocr_pipeline_job`

## 命令模板

### 1. `sina__trade_calendar`

```bash
cd pipeline
uv run dg launch --target-path scheduler --job sina__trade_calendar_job
```

### 2. `baostock__query_stock_basic`

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_stock_basic"
```

说明：

- 不使用 `baostock__daily_job`，因为该 job 同时包含 `baostock__query_stock_basic` 和 `baostock__query_history_k_data_plus_daily`
- 这里需要先单独刷新股票基础信息，避免提前启动年分区日线回填

### 3. `jiuyan__industry_list`

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_list"
```

### 4. `jiuyan__industry_images`

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_images"
```

说明：

- 该资产依赖 `jiuyan__industry_list`
- 单独回填图片时使用精确 asset selection，不直接跑 `jiuyan__industry_ocr_pipeline_job`

### 5. `jiuyan__industry_ocr`，限制 50 张图片

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__industry_ocr" \
  --config-json '{"ops":{"source__jiuyan__industry_ocr":{"config":{"limit":50}}}}'
```

说明：

- `limit: 50` 会传给 `IndustryOcrConfig.limit`
- 资产 key 带 `source/` 前缀，Dagster step/op config key 是 `source__jiuyan__industry_ocr`
- 该资产依赖 `jiuyan__industry_images`
- 本次只回填 50 张图片的 OCR，不使用 `force_ocr`

### 6. `jiuyan__action_field`，最近 90 个自然日

```bash
cd pipeline
end_date="$(date +%F)"
start_date="$(date -d "$end_date -89 days" +%F)"
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__action_field" \
  --partition-range "${start_date}...${end_date}"
```

说明：

- `--partition-range` 使用包含式范围
- `dg launch` 要求 range 起止值都是资产有效 partition key；如果 `start_date` 或 `end_date` 是周末/未来无效分区，要落到最近的有效日分区再执行
- 以 `2026-05-31` 执行计算，实际成功使用的有效范围是 `2026-03-02...2026-05-29`
- 资产内部会基于 `sina__trade_calendar` 过滤交易日
- 当前代码限制单次最多处理最近 80 个交易日；90 个自然日通常低于该交易日数量上限

### 7. `jiuyan__action_field_compacted`

```bash
cd pipeline
for year in $(seq 2021 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/jiuyan__action_field_compacted" \
    --partition "$year"
done
```

说明：

- 该资产是年分区，分区起点是 `2021`
- 必须在对应年份的 `jiuyan__action_field` 日分区回填之后执行
- 本次 `jiuyan__action_field` 只回填最近 90 个自然日，因此历史年份 compacted 可能只会读取已有日分区产物

### 8. `ths__limit_up_pool`，全部日分区

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/ths__limit_up_pool" \
  --partition-range "2025-01-01...$(date +%F)"
```

说明：

- `ths__limit_up_pool` 的分区起点是 `2025-01-01`
- 资产内部会基于 `sina__trade_calendar` 过滤交易日
- 当前代码会保留最近最多 380 个交易日；从 `2025-01-01` 到 `2026-05-31` 的交易日数量应低于该上限

### 9. `ths__limit_up_pool_compacted`

```bash
cd pipeline
for year in $(seq 2025 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/ths__limit_up_pool_compacted" \
    --partition "$year"
done
```

说明：

- 该资产是年分区
- 必须在对应年份的 `ths__limit_up_pool` 日分区回填之后执行
- 需求里重复提到 `ths__limit_up_pool_compacted`，本计划只执行一次 compacted 回填

### 10. Eastmoney 全资产，1990 到今年

按年循环执行，单年单次 launch，允许最多 3 个 run 并行：

```bash
cd pipeline
for year in $(seq 1990 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__balance" --partition "$year"
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__cashflow_sq" --partition "$year"
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__cashflow_ytd" --partition "$year"
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__dividend_allotment" --partition "$year"
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__dividend_main" --partition "$year"
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__equity_history" --partition "$year"
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__income_sq" --partition "$year"
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__income_ytd" --partition "$year"
done
```

说明：

- 这里按资产逐个跑，避免一次选择过宽
- Eastmoney 使用 `eastmoney_run_pool`，当前并行上限为 3
- 如果实际执行时发现某些资产需要拆分限速，可以单独拆出更细的循环

### 11. `baostock__query_history_k_data_plus_daily`，1995 到今年

```bash
cd pipeline
for year in $(seq 1995 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition "$year"
done
```

说明：

- 该资产是年分区
- 每次只跑一个年分区
- 需要时可先试跑一个年份，例如 `2024`

## 推荐执行方式

1. 先试跑小切片
2. 观察日志和产物
3. 再按年份批量推进
4. 每完成一个大段年份，记录结果和异常

## 记录区

### 已完成

- [x] `sina__trade_calendar`
- [x] `baostock__query_stock_basic`
- [x] `jiuyan__industry_list`
- [x] `jiuyan__industry_images`
- [x] `jiuyan__industry_ocr`，限制 50 张图片
- [x] `jiuyan__action_field`，最近 90 个自然日
- [x] `jiuyan__action_field_compacted`
- [x] `ths__limit_up_pool`，全部日分区
- [x] `ths__limit_up_pool_compacted`
- [x] `eastmoney__*` 1990 - 今年
- [x] `baostock__query_history_k_data_plus_daily` 1995 - 今年

报告记录见 `docs/jobs/reports/`。

### 异常记录

- `jiuyan__industry_images` 首次失败后迁移并重跑成功，见 `004-jiuyan__industry_images.md`。
- `jiuyan__industry_ocr` 本次按计划限制 50 张，结果 48 成功、2 失败，失败项保留状态记录，见 `005-jiuyan__industry_ocr.md`。
- `jiuyan__action_field` 首次使用周末终点失败，改用有效分区范围 `2026-03-02...2026-05-29` 后成功，见 `006-jiuyan__action_field.md`。
- `jiuyan__action_field_compacted` 历史年份无本次 daily 回填数据，2026 成功，见 `007-jiuyan__action_field_compacted.md`。
- `eastmoney__dividend_main.REPORT_TIME` 历史值可能为 `1991年报`，已将 schema 修正为 string；按用户要求已重新全量回填 8 个 Eastmoney 资产 1990-2026，见 `011-eastmoney__all.md`。
- `baostock__query_history_k_data_plus_daily` 2010 曾有取消记录、2026 曾有失败记录，均已有后续成功物化，见 `010-baostock__query_history_k_data_plus_daily.md`。
