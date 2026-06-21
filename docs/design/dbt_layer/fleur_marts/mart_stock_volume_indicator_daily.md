# mart_stock_volume_indicator_daily 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_ma_daily')`
- 目标 SQL：`pipeline/elt/models/marts/mart_stock_volume_indicator_daily.sql`
- 目标 YAML：`pipeline/elt/models/marts/mart_stock_volume_indicator_daily.yml`

## 1. 模型定位

A 股股票日频成交量形指标 mart。模型只暴露均量字段，作为成交量形指标的稳定消费接口。

均量当前由 Furnace MA calculation 表和 `int_stock_ma_daily` wrapper 提供。本 mart 按语义把这些字段从价格趋势指标中拆出，避免把成交量指标混入 `mart_stock_trend_indicator_daily`。

非目标：

- 不输出价格 MA、BOLL、MACD、RSI、KDJ。
- 不读取行情、估值、财务、raw、staging 或 `fleur_calculation` 物理表。
- 不在 mart 层重算均量。

## 2. 数据粒度与依赖

- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 唯一上游：`int_stock_ma_daily`。
- Join 策略：无 join。

## 3. 字段分组

| 字段组 | 来源 | 字段 |
|---|---|---|
| 主键 | `int_stock_ma_daily` | `security_code`, `trade_date` |
| 均量 | `int_stock_ma_daily` | `volume_ma_5`, `volume_ma_10`, `volume_ma_20`, `volume_ma_60` |

## 4. NULL 语义

均量 NULL 语义完全沿用 `int_stock_ma_daily`：

- 窗口不足时允许为 NULL。
- source 缺口导致无法形成窗口时允许为 NULL。
- 0 成交量是有效输入。
- mart 层不填 0，不使用上一日值，不重算公式。

## 5. 验证

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_volume_indicator_daily
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_volume_indicator_daily --limit 20
```

补充质量检查：

- `(security_code, trade_date)` 唯一。
- `security_code`、`trade_date` 非空。
- `security_code` 符合 A 股标准代码格式。
- 字段集合只包含四个 `volume_ma_*` 均量字段和主键。
