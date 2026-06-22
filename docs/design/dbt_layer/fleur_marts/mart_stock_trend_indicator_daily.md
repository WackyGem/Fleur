# mart_stock_trend_indicator_daily 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_ma_daily')`
- Intermediate model：`ref('int_stock_boll_daily')`
- Intermediate model：`ref('int_stock_macd_daily')`
- MACD 上游计划：`docs/plans/archive/0034-furnace-macd-technical-indicator-implementation-plan.md`
- 目标 SQL：`pipeline/elt/models/marts/mart_stock_trend_indicator_daily.sql`
- 目标 YAML：`pipeline/elt/models/marts/mart_stock_trend_indicator_daily.yml`

## 1. 模型定位

A 股股票日频趋势指标 mart。模型整合价格 MA、组合 MA、双重 EMA、BOLL 和 MACD，作为趋势类技术指标的稳定消费接口。

本模型不承载公式实现。MA/BOLL/MACD 的计算均由 Furnace calculation 层完成，并经 dbt intermediate wrapper 暴露。本 mart 只负责按 `security_code`, `trade_date` 对齐、字段分组和消费层命名。

非目标：

- 不输出 `volume_ma_*` 均量字段，均量归属 `mart_stock_volume_indicator_daily`。
- 不读取行情、估值、财务、raw、staging 或 `fleur_calculation` 物理表。
- 不暴露 MACD 内部状态列 `ema_fast_state_12`、`ema_slow_state_26`、`macd_dea_state`。

## 2. 数据粒度与依赖

- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 左表：`int_stock_ma_daily`。
- Join 策略：左连接 `int_stock_boll_daily` 和 `int_stock_macd_daily`。某类指标缺口不会导致 MA 行丢失。

## 3. 字段分组

| 字段组 | 来源 | 字段 |
|---|---|---|
| 主键 | `int_stock_ma_daily` | `security_code`, `trade_date` |
| 价格 MA | `int_stock_ma_daily` | `price_ma_3`, `price_ma_5`, `price_ma_6`, `price_ma_10`, `price_ma_12`, `price_ma_14`, `price_ma_20`, `price_ma_24`, `price_ma_28`, `price_ma_30`, `price_ma_57`, `price_ma_60`, `price_ma_114`, `price_ma_250` |
| MA 组合和 EMA | `int_stock_ma_daily` | `price_avg_ma_3_6_12_24`, `price_avg_ma_14_28_57_114`, `price_ema2_10` |
| BOLL | `int_stock_boll_daily` | `boll_mid_10_1p5`, `boll_upper_10_1p5`, `boll_lower_10_1p5`, `boll_mid_20_2`, `boll_upper_20_2`, `boll_lower_20_2`, `boll_mid_50_2p5`, `boll_upper_50_2p5`, `boll_lower_50_2p5` |
| MACD | `int_stock_macd_daily` | `macd_dif`, `macd_dea`, `macd_histogram` |
| Crossing 前值 | mart 内窗口函数 | 所有趋势数值字段对应的 `prev_*` 字段，例如 `prev_price_ma_20`, `prev_boll_upper_20_2`, `prev_macd_dif` |

## 4. NULL 语义

指标 NULL 语义完全沿用 upstream wrapper：

- MA/BOLL/MACD warm-up 阶段允许为 NULL。
- 当前价格缺失或状态无法推进时允许为 NULL。
- mart 层不填 0，不使用上一日值，不重算公式。
- `prev_*` 字段使用 `lag(field) over (partition by security_code order by trade_date)` 生成。
- `prev_*` 的“上一期”是同一 `security_code` 的上一交易行，不是自然日前一日；停牌或非交易日不会被补行。
- 首个交易行、上一交易行不存在、上一交易行对应指标为 NULL 时，`prev_*` 为 NULL，不做 forward fill。
- Rearview crossing 规则任一当前值或前值为 NULL 时应按 `no_match` 处理，mart 层只提供字段事实，不判断 crossing。

MACD 口径：

- 参数固定为 `MACD(12,26,9)`。
- 输入固定为 `close_price_forward_adj`。
- EMA(12)、EMA(26) 和 DEA(9) 均使用 SMA 启动。
- `macd_histogram = macd_dif - macd_dea`，不是 2 倍柱状图。

## 5. 验证

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator_daily
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_trend_indicator_daily --limit 20
```

补充质量检查：

- `(security_code, trade_date)` 唯一。
- `security_code`、`trade_date` 非空。
- `security_code` 符合 A 股标准代码格式。
- BOLL 非空完整三元组满足 `up >= mid >= down`。
- compiled SQL 中 MACD 来源为 `int_stock_macd_daily`。
- `prev_*` 字段仅由同证券上一交易行生成，不跨证券、不按自然日补齐。
