# mart_stock_momentum_indicator_daily 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_rsi_daily')`
- Intermediate model：`ref('int_stock_kdj_daily')`
- 目标 SQL：`pipeline/elt/models/marts/mart_stock_momentum_indicator_daily.sql`
- 目标 YAML：`pipeline/elt/models/marts/mart_stock_momentum_indicator_daily.yml`

## 1. 模型定位

A 股股票日频动量指标 mart。模型整合 RSI 和 KDJ，作为动量类技术指标的稳定消费接口。

本模型不承载 RSI 或 KDJ 公式实现。RSI/KDJ 均由 Furnace calculation 层完成，并经 dbt intermediate wrapper 暴露。本 mart 只负责按 `security_code`, `trade_date` 对齐和消费层字段命名。

非目标：

- 不从 `mart_stock_quotes_daily` 读取 KDJ，避免 mart 依赖 mart。
- 不读取行情、估值、财务、raw、staging 或 `fleur_calculation` 物理表。
- 不引入趋势指标、均量字段或其他未纳入本计划的指标。

## 2. 数据粒度与依赖

- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 左表：`int_stock_rsi_daily`。
- Join 策略：左连接 `int_stock_kdj_daily`。KDJ 缺口不会导致 RSI 行丢失。

## 3. 字段分组

| 字段组 | 来源 | 字段 |
|---|---|---|
| 主键 | `int_stock_rsi_daily` | `security_code`, `trade_date` |
| RSI | `int_stock_rsi_daily` | `rsi_6`, `rsi_12`, `rsi_14`, `rsi_24`, `rsi_25`, `rsi_50` |
| KDJ 指标 | `int_stock_kdj_daily` | `kdj_rsv`, `kdj_k_value`, `kdj_d_value`, `kdj_j_value` |

KDJ 字段在 mart 层统一加 `kdj_` 前缀，避免 `rsv`、`k_value`、`d_value`、`j_value` 在消费侧语义不清。

## 4. NULL 语义

指标 NULL 语义完全沿用 upstream wrapper：

- RSI/KDJ warm-up 阶段允许为 NULL。
- 当前价格缺失或状态无法推进时允许为 NULL。
- mart 层不填 0，不使用上一日值，不重算公式。

KDJ 口径：

- 上游 `int_stock_kdj_daily` 第一版固定 canonical `KDJ(9,3,3)`，并保留参数字段用于 wrapper 自描述。
- 本 mart 只输出消费侧指标值 `kdj_rsv`, `kdj_k_value`, `kdj_d_value`, `kdj_j_value`，不输出 KDJ 参数字段。

## 5. 验证

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_momentum_indicator_daily
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_momentum_indicator_daily --limit 20
```

补充质量检查：

- `(security_code, trade_date)` 唯一。
- `security_code`、`trade_date` 非空。
- `security_code` 符合 A 股标准代码格式。
- RSI 非空值在 `[0, 100]`。
- KDJ 指标字段与 `int_stock_kdj_daily` 对应值一致。
