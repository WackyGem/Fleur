# Dagster 回填执行计划

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

- `docs/skills/dg-backfill-runbook` 维护通用回填命令模板、选择规则、分区规则
- `docs/jobs/dagster-backfill-2026.md` 记录这一次具体执行计划、顺序、范围和操作记录
- `pipeline/scheduler` 负责实际资产、job、schedule 定义
- Eastmoney 并行度由 Dagster pool 控制，当前按 `eastmoney_run_pool = 3` 执行

## 前置检查

在 `pipeline/` 下执行：

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
  --config-json '{"ops":{"jiuyan__industry_ocr":{"config":{"limit":50}}}}'
```

说明：

- `limit: 50` 会传给 `IndustryOcrConfig.limit`
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
- 以 `2026-05-31` 执行计算，最近 90 个自然日范围是 `2026-03-03...2026-05-31`
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

- [ ] `sina__trade_calendar`
- [ ] `baostock__query_stock_basic`
- [ ] `jiuyan__industry_list`
- [ ] `jiuyan__industry_images`
- [ ] `jiuyan__industry_ocr`，限制 50 张图片
- [ ] `jiuyan__action_field`，最近 90 个自然日
- [ ] `jiuyan__action_field_compacted`
- [ ] `ths__limit_up_pool`，全部日分区
- [ ] `ths__limit_up_pool_compacted`
- [ ] `eastmoney__*` 1990 - 今年
- [ ] `baostock__query_history_k_data_plus_daily` 1995 - 今年

### 异常记录

- 
