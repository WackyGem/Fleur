# Plan 0010: S3 Parquet Schema 类型优化实施计划

状态：草案

计划日期：2026-05-30

关联文档：

- `docs/RFC/0007-dbt-raw-layer-and-dagster-dbt-integration.md`（阶段 1 前置改造）
- `docs/references/data_dict/*.md`（字段校对文档，逐字段类型定义）

参考资料：

- `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/http/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/fields.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py`
- `pipeline/scheduler/src/scheduler/defs/sources/sina/trade_calendar.py`
- `pipeline/scheduler/src/scheduler/defs/common/types.py`
- `pipeline/scheduler/src/scheduler/defs/common/schema.py`
- `pipeline/scheduler/src/scheduler/defs/storage/parquet.py`

## 目标

将 S3 parquet 文件的列类型从全 `pa.string()` 改为真实数据类型，为后续 ClickHouse raw 层和 dbt 集成奠定基础。

**核心原则：逐字段显式映射，不使用字段名模式推断。**

每个数据源的 schema 由 `docs/references/data_dict/` 中校对过的文档逐字段定义，确保每个字段的 PyArrow 类型准确无误。

## 非目标

本计划不包含：

- ClickHouse raw 表建表和 `REPLACE PARTITION` 逻辑（RFC 0007 阶段 2）。
- dbt 模型创建和 Declarative Automation 配置（RFC 0007 阶段 3）。
- 修改 S3 路径结构、asset key 或分区策略。
- 修改 Dagster schedule、sensor 或 job 定义。
- 修改 BaoStock TCP 协议、HTTP 客户端或 API 请求逻辑。

## 设计决策

### 决策 1：逐字段显式映射（替代模式推断）

**选择：为每个数据源定义显式 `pa.Schema`，逐字段声明类型，不使用正则模式推断。**

理由：

1. **准确性**：API 返回的字段语义需要逐个确认，正则推断容易遗漏特例（如 `REPORT_DATE` 在 `dividend_main` 返回中文文本，其他端点返回日期）。
2. **可维护性**：显式 schema 与 `docs/references/data_dict/` 一一对应，修改时有明确依据。
3. **可审计性**：每个字段的类型选择都有对应的 API 验证记录。
4. **性能**：省去正则匹配开销，schema 构建为纯声明式。

### 决策 2：类型转换策略

**选择：统一的 None/异常 双层处理。**

| 输入 | 行为 | 说明 |
|------|------|------|
| `None` | 返回 `None` | 缺失值，PyArrow 自动处理为 null |
| 有效值 | 返回转换后的值 | 正常路径 |
| 空字符串 | 抛 `SchemaTypeError` | 数据质量问题，应立即发现 |
| 无效格式 | 抛 `SchemaTypeError` | 数据源返回意外格式，应立即发现 |

理由：

1. **None 安全**：API 字段值经常为 null（如 `update_time`、`day`、`edition`），转换函数直接支持 None 输入，调用方无需额外检查。
2. **格式严格**：空字符串和无效格式抛异常，便于开发调试时发现问题。
3. **fail-fast 原则**：在数据写入阶段发现问题，比在下游查询时发现更高效。
4. **与 ClickHouse 对齐**：None → null 与 ClickHouse `Nullable()` 类型一致。

### 决策 3：时间字段处理策略

根据 API 验证结果，各数据源的时间字段格式如下：

| 数据源 | 字段 | API 格式 | PyArrow 类型 |
|--------|------|---------|-------------|
| BaoStock | `date` | `'2026-05-25'` | `pa.date32()` |
| BaoStock | `ipoDate`, `outDate` | `'1999-11-10'` 或 `''` | `pa.date32()` |
| EastMoney 财务报表 | `REPORT_DATE` | `'2026-03-31 00:00:00'` | `pa.date32()` |
| EastMoney 财务报表 | `NOTICE_DATE`, `UPDATE_DATE` | `'2026-04-29 00:00:00'` | `pa.date32()` |
| EastMoney dividend_main | `REPORT_DATE` | `'2025年报'`（中文文本） | `pa.string()` |
| EastMoney dividend_main | `DAT_YAGGR`, `REPORT_TIME` | `'2026-03-31 00:00:00'` | `pa.date32()` |
| EastMoney equity_history | `END_DATE`, `NOTICE_DATE`, `LISTING_DATE` | `'2026-04-07 00:00:00'` | `pa.date32()` |
| JiuYan action_field | `date` | `'2026-05-28'` | `pa.date32()` |
| JiuYan action_field | `create_time`, `update_time`, `delete_time` | `'2026-05-28 12:03:00'` | `pa.timestamp("ns")` |
| JiuYan action_field | `time`（action_info） | `'09:37:51'` | `pa.time32("ms")` |
| JiuYan industry_list | `create_time`, `update_time`, `delete_time` | `'2026-05-29 11:57:31'` | `pa.timestamp("ns")` |
| THS limit_up_pool | `date` | `'20260529'` | `pa.date32()` |
| THS limit_up_pool | `first_limit_up_time`, `last_limit_up_time` | `'1780032963'`（Unix 秒） | `pa.timestamp("ns", tz="UTC")` |
| Sina trade_calendar | `trade_date` | `date` 对象 | `pa.date32()` |

### 决策 4：数值字段统一为 `pa.float64()`

EastMoney 财务报表的金额字段（如 `TOTAL_ASSETS`、`NETPROFIT`）和比率字段（如 `*_RATIO`、`*_YOY`）统一使用 `pa.float64()`。

理由：

1. ClickHouse `Float64` 与 PyArrow `float64` 一一对应，无需类型转换。
2. `float64` 有效精度 ~15-17 位，对财务数据足够（单笔金额 < 万亿不会丢失分位精度）。
3. 保持与 ClickHouse 分析场景的一致性。

### 决策 5：布尔字段统一为 `pa.bool_()`

无论 API 返回 `'0'`/`'1'` 字符串还是 `0`/`1` 整数，都转为 `pa.bool_()`。

验证的布尔字段：

| 数据源 | 字段 | API 返回值 |
|--------|------|-----------|
| EastMoney equity_history | `IS_FREE_WINDOW`, `IS_LIMITED_WINDOW`, `IS_USE` | `'0'`/`'1'` |
| EastMoney dividend_main | `IS_UNASSIGN`, `IS_PAYCASH`, `IS_EQUITY_RECENT` | `'0'`/`'1'` |
| EastMoney dividend_main | `*_TODAY`（7 个字段） | `'0'`/`'1'` |
| THS limit_up_pool | `is_new`, `is_again_limit` | `0`/`1` |
| JiuYan action_field | `is_delete` | `'0'`/`'1'` |
| JiuYan industry_list | `is_top`, `is_delete` | `0`/`1` 或 `'0'`/`'1'` |
| JiuYan industry_list | `title_red`, `title_bold` | `0`/`1` |

## 逐字段 Schema 定义

### 1. BaoStock Stock Basic（6 字段）

```python
STOCK_BASIC_SCHEMA = pa.schema([
    pa.field("code", pa.string()),
    pa.field("code_name", pa.string()),
    pa.field("ipoDate", pa.date32()),
    pa.field("outDate", pa.date32()),
    pa.field("type", pa.int8()),
    pa.field("status", pa.int8()),
])
```

### 2. BaoStock K History Daily（14 字段）

```python
K_HISTORY_DAILY_SCHEMA = pa.schema([
    pa.field("date", pa.date32()),
    pa.field("code", pa.string()),
    pa.field("open", pa.float64()),
    pa.field("high", pa.float64()),
    pa.field("low", pa.float64()),
    pa.field("close", pa.float64()),
    pa.field("preclose", pa.float64()),
    pa.field("volume", pa.int64()),
    pa.field("amount", pa.float64()),
    pa.field("adjustflag", pa.int8()),
    pa.field("turn", pa.float64()),
    pa.field("tradestatus", pa.int8()),
    pa.field("pctChg", pa.float64()),
    pa.field("isST", pa.int8()),
])
```

### 3. EastMoney Balance（319 字段）

```python
EASTMONEY_BALANCE_SCHEMA = pa.schema([
    pa.field("SECUCODE", pa.string()),
    pa.field("SECURITY_CODE", pa.string()),
    pa.field("SECURITY_NAME_ABBR", pa.string()),
    pa.field("ORG_CODE", pa.string()),
    pa.field("ORG_TYPE", pa.string()),
    pa.field("REPORT_DATE", pa.date32()),          # '2026-03-31 00:00:00'
    pa.field("REPORT_TYPE", pa.string()),
    pa.field("REPORT_DATE_NAME", pa.string()),     # '2026一季报' 文本
    pa.field("SECURITY_TYPE_CODE", pa.string()),
    pa.field("NOTICE_DATE", pa.date32()),
    pa.field("UPDATE_DATE", pa.date32()),
    pa.field("CURRENCY", pa.string()),
    pa.field("ACCEPT_DEPOSIT_INTERBANK", pa.float64()),
    pa.field("ACCOUNTS_PAYABLE", pa.float64()),
    pa.field("ACCOUNTS_RECE", pa.float64()),
    pa.field("ACCRUED_EXPENSE", pa.float64()),
    pa.field("ADVANCE_RECEIVABLES", pa.float64()),
    pa.field("AGENT_TRADE_SECURITY", pa.float64()),
    pa.field("AGENT_UNDERWRITE_SECURITY", pa.float64()),
    pa.field("AMORTIZE_COST_FINASSET", pa.float64()),
    pa.field("AMORTIZE_COST_FINLIAB", pa.float64()),
    pa.field("AMORTIZE_COST_NCFINASSET", pa.float64()),
    pa.field("AMORTIZE_COST_NCFINLIAB", pa.float64()),
    pa.field("APPOINT_FVTPL_FINASSET", pa.float64()),
    pa.field("APPOINT_FVTPL_FINLIAB", pa.float64()),
    pa.field("ASSET_BALANCE", pa.float64()),
    pa.field("ASSET_OTHER", pa.float64()),
    pa.field("ASSIGN_CASH_DIVIDEND", pa.float64()),
    pa.field("AVAILABLE_SALE_FINASSET", pa.float64()),
    pa.field("BOND_PAYABLE", pa.float64()),
    pa.field("BORROW_FUND", pa.float64()),
    pa.field("BUY_RESALE_FINASSET", pa.float64()),
    pa.field("CAPITAL_RESERVE", pa.float64()),
    pa.field("CIP", pa.float64()),
    pa.field("CONSUMPTIVE_BIOLOGICAL_ASSET", pa.float64()),
    pa.field("CONTRACT_ASSET", pa.float64()),
    pa.field("CONTRACT_LIAB", pa.float64()),
    pa.field("CONVERT_DIFF", pa.float64()),
    pa.field("CREDITOR_INVEST", pa.float64()),
    pa.field("CURRENT_ASSET_BALANCE", pa.float64()),
    pa.field("CURRENT_ASSET_OTHER", pa.float64()),
    pa.field("CURRENT_LIAB_BALANCE", pa.float64()),
    pa.field("CURRENT_LIAB_OTHER", pa.float64()),
    pa.field("DEFER_INCOME", pa.float64()),
    pa.field("DEFER_INCOME_1YEAR", pa.float64()),
    pa.field("DEFER_TAX_ASSET", pa.float64()),
    pa.field("DEFER_TAX_LIAB", pa.float64()),
    pa.field("DERIVE_FINASSET", pa.float64()),
    pa.field("DERIVE_FINLIAB", pa.float64()),
    pa.field("DEVELOP_EXPENSE", pa.float64()),
    pa.field("DIV_HOLDSALE_ASSET", pa.float64()),
    pa.field("DIV_HOLDSALE_LIAB", pa.float64()),
    pa.field("DIVIDEND_PAYABLE", pa.float64()),
    pa.field("DIVIDEND_RECE", pa.float64()),
    pa.field("EQUITY_BALANCE", pa.float64()),
    pa.field("EQUITY_OTHER", pa.float64()),
    pa.field("EXPORT_REFUND_RECE", pa.float64()),
    pa.field("FEE_COMMISSION_PAYABLE", pa.float64()),
    pa.field("FIN_FUND", pa.float64()),
    pa.field("FINANCE_RECE", pa.float64()),
    pa.field("FIXED_ASSET", pa.float64()),
    pa.field("FIXED_ASSET_DISPOSAL", pa.float64()),
    pa.field("FVTOCI_FINASSET", pa.float64()),
    pa.field("FVTOCI_NCFINASSET", pa.float64()),
    pa.field("FVTPL_FINASSET", pa.float64()),
    pa.field("FVTPL_FINLIAB", pa.float64()),
    pa.field("GENERAL_RISK_RESERVE", pa.float64()),
    pa.field("GOODWILL", pa.float64()),
    pa.field("HOLD_MATURITY_INVEST", pa.float64()),
    pa.field("HOLDSALE_ASSET", pa.float64()),
    pa.field("HOLDSALE_LIAB", pa.float64()),
    pa.field("INSURANCE_CONTRACT_RESERVE", pa.float64()),
    pa.field("INTANGIBLE_ASSET", pa.float64()),
    pa.field("INTEREST_PAYABLE", pa.float64()),
    pa.field("INTEREST_RECE", pa.float64()),
    pa.field("INTERNAL_PAYABLE", pa.float64()),
    pa.field("INTERNAL_RECE", pa.float64()),
    pa.field("INVENTORY", pa.float64()),
    pa.field("INVEST_REALESTATE", pa.float64()),
    pa.field("LEASE_LIAB", pa.float64()),
    pa.field("LEND_FUND", pa.float64()),
    pa.field("LIAB_BALANCE", pa.float64()),
    pa.field("LIAB_EQUITY_BALANCE", pa.float64()),
    pa.field("LIAB_EQUITY_OTHER", pa.float64()),
    pa.field("LIAB_OTHER", pa.float64()),
    pa.field("LOAN_ADVANCE", pa.float64()),
    pa.field("LOAN_PBC", pa.float64()),
    pa.field("LONG_EQUITY_INVEST", pa.float64()),
    pa.field("LONG_LOAN", pa.float64()),
    pa.field("LONG_PAYABLE", pa.float64()),
    pa.field("LONG_PREPAID_EXPENSE", pa.float64()),
    pa.field("LONG_RECE", pa.float64()),
    pa.field("MARGIN_DEPOSIT", pa.float64()),
    pa.field("MONETARYFUNDS", pa.float64()),
    pa.field("NONCURRENT_ASSET_BALANCE", pa.float64()),
    pa.field("NONCURRENT_ASSET_OTHER", pa.float64()),
    pa.field("NONCURRENT_LIAB_BALANCE", pa.float64()),
    pa.field("NONCURRENT_LIAB_OTHER", pa.float64()),
    pa.field("NOTE_PAYABLE", pa.float64()),
    pa.field("NOTE_RECE", pa.float64()),
    pa.field("OTHER_COMPRE_INCOME", pa.float64()),
    pa.field("OTHER_NONCURRENT_LIAB", pa.float64()),
    pa.field("OTHER_PAYABLE", pa.float64()),
    pa.field("OTHER_RECE", pa.float64()),
    pa.field("PARENT_EQUITY", pa.float64()),
    pa.field("PERPETUAL_BOND", pa.float64()),
    pa.field("PREPAYMENT", pa.float64()),
    pa.field("PREDICT_LIAB", pa.float64()),
    pa.field("PURE_DIFF_INCOME", pa.float64()),
    pa.field("RECEIVE_DIVIDEND", pa.float64()),
    pa.field("RECEIVE_INVEST_INCOME", pa.float64()),
    pa.field("REINSURE_DEPOSIT", pa.float64()),
    pa.field("RESERVE", pa.float64()),
    pa.field("RESTRICTED_ASSET", pa.float64()),
    pa.field("SELL_REPU_ASSET", pa.float64()),
    pa.field("SETTLEMENT_PROVISION", pa.float64()),
    pa.field("SHARE_CAPITAL", pa.float64()),
    pa.field("SPECIAL_PAYABLE", pa.float64()),
    pa.field("SPLIT_FEE_RECE", pa.float64()),
    pa.field("STAFF_PAYABLE", pa.float64()),
    pa.field("SUBTOTAL_EQUITY", pa.float64()),
    pa.field("SUBTOTAL_NONCURRENT_ASSET", pa.float64()),
    pa.field("SUBTOTAL_NONCURRENT_LIAB", pa.float64()),
    pa.field("TAX_PAYABLE", pa.float64()),
    pa.field("TOTAL_ASSETS", pa.float64()),
    pa.field("TOTAL_CURRENT_ASSET", pa.float64()),
    pa.field("TOTAL_CURRENT_LIAB", pa.float64()),
    pa.field("TOTAL_EQUITY", pa.float64()),
    pa.field("TOTAL_LIAB", pa.float64()),
    pa.field("TOTAL_LIAB_EQUITY", pa.float64()),
    pa.field("TOTAL_NONCURRENT_ASSET", pa.float64()),
    pa.field("TOTAL_NONCURRENT_LIAB", pa.float64()),
    pa.field("TRADING_ASSET", pa.float64()),
    pa.field("TRADING_FL", pa.float64()),
    pa.field("UNASSIGN_RPOFIT", pa.float64()),
    pa.field("UNREALIZED_LOSS_DEPR", pa.float64()),
    pa.field("USER_RIGHT_FUND", pa.float64()),
])
```

> **注意**：以上仅展示前 120 个字段。完整的 319 字段定义从 `docs/references/data_dict/eastmoney__balance.md` 逐行提取，所有 `number` 类型字段映射为 `pa.float64()`，所有 `string` 类型字段映射为 `pa.string()`，日期字段映射为 `pa.date32()`。完整列表在实施时从数据字典文件生成。

### 4. EastMoney Dividend Main（30 字段）

```python
EASTMONEY_DIVIDEND_MAIN_SCHEMA = pa.schema([
    pa.field("SECUCODE", pa.string()),
    pa.field("SECURITY_CODE", pa.string()),
    pa.field("SECURITY_NAME_ABBR", pa.string()),
    pa.field("NOTICE_DATE", pa.date32()),
    pa.field("IMPL_PLAN_PROFILE", pa.string()),
    pa.field("ASSIGN_PROGRESS", pa.string()),
    pa.field("EQUITY_RECORD_DATE", pa.date32()),    # OpenAPI 声明为 number，实际返回日期
    pa.field("EX_DIVIDEND_DATE", pa.date32()),
    pa.field("PAY_CASH_DATE", pa.date32()),
    pa.field("IS_UNASSIGN", pa.bool_()),
    pa.field("REPORT_DATE", pa.string()),            # 中文文本如 '2025年报'，非日期！
    pa.field("ASSIGN_OBJECT", pa.string()),
    pa.field("IMPL_PLAN_NEWPROFILE", pa.string()),
    pa.field("NEW_PROFILE", pa.string()),
    pa.field("GMDECISION_NOTICE_DATE", pa.date32()),
    pa.field("INFO_CODE", pa.string()),
    pa.field("DAT_YAGGR", pa.date32()),              # '2026-03-31 00:00:00'
    pa.field("TOTAL_DIVIDEND", pa.float64()),
    pa.field("TOTAL_DIVIDEND_A", pa.float64()),
    pa.field("REPORT_TIME", pa.date32()),            # '2025-12-31 00:00:00'
    pa.field("DAT_YAGGR_TODAY", pa.bool_()),         # '0'/'1'
    pa.field("NOTICE_TODAY", pa.bool_()),
    pa.field("GMDECISION_TODAY", pa.bool_()),
    pa.field("DIRECTORSUPERVISOR_TODAY", pa.bool_()),
    pa.field("EQUITY_TODAY", pa.bool_()),
    pa.field("EX_DIVIDEND_TODAY", pa.bool_()),
    pa.field("PAYCASH_TODAY", pa.bool_()),
    pa.field("IS_PAYCASH", pa.bool_()),
    pa.field("IS_EQUITY_RECENT", pa.bool_()),
    pa.field("LAST_TRADE_DATE", pa.date32()),
])
```

### 5. EastMoney Dividend Allotment（10 字段）

```python
EASTMONEY_DIVIDEND_ALLOTMENT_SCHEMA = pa.schema([
    pa.field("SECUCODE", pa.string()),
    pa.field("SECURITY_CODE", pa.string()),
    pa.field("SECURITY_NAME_ABBR", pa.string()),
    pa.field("NOTICE_DATE", pa.date32()),
    pa.field("ISSUE_NUM", pa.float64()),
    pa.field("TOTAL_RAISE_FUNDS", pa.float64()),
    pa.field("ISSUE_PRICE", pa.float64()),
    pa.field("EQUITY_RECORD_DATE", pa.date32()),
    pa.field("EX_DIVIDEND_DATEE", pa.string()),
    pa.field("EVENT_EXPLAIN", pa.string()),
])
```

### 6. EastMoney Equity History（69 字段）

```python
EASTMONEY_EQUITY_HISTORY_SCHEMA = pa.schema([
    pa.field("SECUCODE", pa.string()),
    pa.field("SECURITY_CODE", pa.string()),
    pa.field("ORG_CODE", pa.string()),
    pa.field("END_DATE", pa.date32()),
    pa.field("CHANGE_REASON", pa.string()),
    pa.field("LIMITED_SHARES", pa.float64()),
    pa.field("UNLIMITED_SHARES", pa.float64()),
    pa.field("TOTAL_SHARES", pa.float64()),
    pa.field("LIMITED_SHARES_RATIO", pa.float64()),
    pa.field("LISTED_SHARES_RATIO", pa.float64()),
    pa.field("TOTAL_SHARES_RATIO", pa.float64()),
    pa.field("LISTED_A_SHARES", pa.float64()),
    pa.field("LIMITED_A_SHARES", pa.float64()),
    pa.field("LISTED_A_SHARES_RATIO", pa.float64()),
    pa.field("LIMITED_A_SHARES_RATIO", pa.float64()),
    pa.field("B_FREE_SHARE", pa.float64()),
    pa.field("H_FREE_SHARE", pa.float64()),
    pa.field("B_FREE_SHARE_RATIO", pa.float64()),
    pa.field("H_FREE_SHARE_RATIO", pa.float64()),
    pa.field("SECURITY_TYPE_CODE", pa.string()),
    pa.field("NON_FREE_SHARES", pa.float64()),
    pa.field("NON_FREESHARES_RATIO", pa.float64()),
    pa.field("LIMITED_B_SHARES", pa.float64()),
    pa.field("LIMITED_BSHARES_RATIO", pa.float64()),
    pa.field("OTHER_FREE_SHARES", pa.float64()),
    pa.field("OTHER_FREESHARES_RATIO", pa.float64()),
    pa.field("LIMITED_STATE_SHARES", pa.float64()),
    pa.field("LIMITED_STATE_LEGAL", pa.float64()),
    pa.field("LIMITED_OTHARS", pa.float64()),
    pa.field("LIMITED_DOMESTIC_NOSTATE", pa.float64()),
    pa.field("LIMITED_DOMESTIC_NATURAL", pa.float64()),
    pa.field("LOCK_SHARES", pa.float64()),
    pa.field("LIMITED_FOREIGN_SHARES", pa.float64()),
    pa.field("LIMITED_OVERSEAS_NOSTATE", pa.float64()),
    pa.field("LIMITED_OVERSEAS_NATURAL", pa.float64()),
    pa.field("LIMITED_H_SHARES", pa.float64()),
    pa.field("SPONSOR_SHARES", pa.float64()),
    pa.field("STATE_SPONSOR_SHARES", pa.float64()),
    pa.field("SPONSOR_SOCIAL_SHARES", pa.float64()),
    pa.field("RAISE_SHARES", pa.float64()),
    pa.field("RAISE_STATE_SHARES", pa.float64()),
    pa.field("RAISE_DOMESTIC_SHARES", pa.float64()),
    pa.field("RAISE_OVERSEAS_SHARES", pa.float64()),
    pa.field("NOTICE_DATE", pa.date32()),
    pa.field("LISTING_DATE", pa.date32()),
    pa.field("LIMITED_SHARES_CHANGE", pa.float64()),
    pa.field("UNLIMITED_SHARES_CHANGE", pa.float64()),
    pa.field("TOTAL_SHARES_CHANGE", pa.float64()),
    pa.field("LISTED_ASHARES_CHANGE", pa.float64()),
    pa.field("LIMITED_ASHARES_CHANGE", pa.float64()),
    pa.field("B_FREESHARE_CHANGE", pa.float64()),
    pa.field("H_FREESHARE_CHANGE", pa.float64()),
    pa.field("LIMITED_BSHARES_CHANGE", pa.float64()),
    pa.field("NONFREE_SHARES_CHANGE", pa.float64()),
    pa.field("OTHERFREE_SHARES_CHANGE", pa.float64()),
    pa.field("FREE_SHARES", pa.float64()),
    pa.field("CHANGE_REASON_EXPLAIN", pa.string()),
    pa.field("LIMITED_H_SHARES_RATIO", pa.float64()),
    pa.field("LIMITED_H_SHARES_CHANGE", pa.float64()),
    pa.field("IS_FREE_WINDOW", pa.bool_()),
    pa.field("IS_LIMITED_WINDOW", pa.bool_()),
    pa.field("LISTED_A_RATIOPC", pa.float64()),
    pa.field("LISTED_B_RATIOPC", pa.float64()),
    pa.field("LISTED_H_RATIOPC", pa.float64()),
    pa.field("LISTED_OTHER_RATIOPC", pa.float64()),
    pa.field("LISTED_SUM_RATIOPC", pa.float64()),
    pa.field("MARKET_CODE", pa.string()),
    pa.field("IS_USE", pa.bool_()),
    pa.field("SECURITY_NAME_ABBR", pa.string()),
])
```

### 7. EastMoney Income SQ（299 字段）

> 同 Balance 结构，`REPORT_DATE` 为 `pa.date32()`，所有 `number` 字段为 `pa.float64()`，`IS_*` 为 `pa.bool_()`。完整字段列表从 `docs/references/data_dict/eastmoney__income_sq.md` 逐行提取。

### 8. EastMoney Income YTD（203 字段）

> 同上，从 `docs/references/data_dict/eastmoney__income_ytd.md` 逐行提取。

### 9. EastMoney Cashflow SQ（372 字段）

> 同上，从 `docs/references/data_dict/eastmoney__cashflow_sq.md` 逐行提取。

### 10. EastMoney Cashflow YTD（254 字段）

> 同上，从 `docs/references/data_dict/eastmoney__cashflow_ytd.md` 逐行提取。

### 11. THS Limit Up Pool（22 字段）

```python
THS_LIMIT_UP_POOL_SCHEMA = pa.schema([
    pa.field("date", pa.date32()),                              # '20260529' → date32
    pa.field("open_num", pa.int64()),                           # 开板次数
    pa.field("first_limit_up_time", pa.timestamp("ns", tz="UTC")),  # Unix 时间戳 → UTC
    pa.field("last_limit_up_time", pa.timestamp("ns", tz="UTC")),
    pa.field("code", pa.string()),                              # 股票代码
    pa.field("limit_up_type", pa.string()),                     # 涨停类型
    pa.field("order_volume", pa.float64()),                     # 封单量（手）
    pa.field("is_new", pa.bool_()),                             # 是否新股 0/1
    pa.field("limit_up_suc_rate", pa.float64()),                # 涨停成功率
    pa.field("currency_value", pa.float64()),                   # 流通市值（元）
    pa.field("market_id", pa.int64()),                          # 市场 ID
    pa.field("is_again_limit", pa.bool_()),                     # 是否回封 0/1
    pa.field("change_rate", pa.float64()),                      # 涨跌幅 (%)
    pa.field("turnover_rate", pa.float64()),                    # 换手率 (%)
    pa.field("reason_type", pa.string()),                       # 涨停原因标签
    pa.field("order_amount", pa.float64()),                     # 封单金额（元）
    pa.field("high_days", pa.string()),                         # 连板天数描述（"首板"）
    pa.field("name", pa.string()),                              # 股票名称
    pa.field("high_days_value", pa.int64()),                    # 连板天数数值
    pa.field("change_tag", pa.string()),                        # 变动标签
    pa.field("market_type", pa.string()),                       # 市场类型
    pa.field("latest", pa.float64()),                           # 最新价
])
```

### 12. JiuYan Action Field（18 字段）

```python
JIUYAN_ACTION_FIELD_SCHEMA = pa.schema([
    pa.field("action_field_id", pa.string()),
    pa.field("name", pa.string()),
    pa.field("date", pa.date32()),                  # '2026-05-28'
    pa.field("reason", pa.string()),
    pa.field("sort_no", pa.int64()),
    pa.field("is_delete", pa.bool_()),              # '0'/'1'
    pa.field("delete_time", pa.timestamp("ns")),    # '2026-05-28 15:43:38' 或 null
    pa.field("create_time", pa.timestamp("ns")),    # '2026-05-28 12:03:00'
    pa.field("update_time", pa.timestamp("ns")),    # '2026-05-28 12:03:00' 或 null
    pa.field("count", pa.int64()),
    pa.field("code", pa.string()),                  # 'sz002350'
    pa.field("time", pa.time32("ms")),              # '09:37:51' → time
    pa.field("num", pa.string()),                   # '9天5板' 或 null
    pa.field("price", pa.int64()),                  # 1718（分）
    pa.field("day", pa.int64()),                    # 9（连板天数）或 null
    pa.field("edition", pa.int64()),                # 5（连板板数）或 null
    pa.field("shares_range", pa.float64()),         # 999.0（万股）
    pa.field("expound", pa.string()),               # 分析文本
])
```

### 13. JiuYan Industry List（17 字段）

```python
JIUYAN_INDUSTRY_LIST_SCHEMA = pa.schema([
    pa.field("industry_id", pa.string()),
    pa.field("title_red", pa.bool_()),              # 0/1 格式标记
    pa.field("title_bold", pa.bool_()),             # 0/1 格式标记
    pa.field("title", pa.string()),
    pa.field("author", pa.string()),
    pa.field("imgs", pa.string()),
    pa.field("keyword", pa.string()),
    pa.field("content", pa.string()),
    pa.field("is_top", pa.bool_()),
    pa.field("status", pa.int64()),
    pa.field("sort_no", pa.int64()),
    pa.field("forward_count", pa.int64()),
    pa.field("browsers_count", pa.int64()),
    pa.field("is_delete", pa.bool_()),
    pa.field("delete_time", pa.timestamp("ns")),    # '2026-05-29 11:57:31' 或 null
    pa.field("create_time", pa.timestamp("ns")),
    pa.field("update_time", pa.timestamp("ns")),
])
```

### 14. JiuYan Industry OCR（5 字段）

```python
JIUYAN_INDUSTRY_OCR_SCHEMA = pa.schema([
    pa.field("stock_name", pa.string()),
    pa.field("theme_path", pa.string()),            # 展平后的数组
    pa.field("source", pa.string()),
    pa.field("industry_id", pa.string()),
    pa.field("relation", pa.string()),
])
```

### 15. Sina Trade Calendar（1 字段）

```python
SINA_TRADE_CALENDAR_SCHEMA = pa.schema([
    pa.field("trade_date", pa.date32()),
])
```

## 实施方案

### 阶段 1：通用类型转换基础设施

**目标：** 创建类型感知的值转换函数和 schema 构建工具。

#### 1.1 `common/types.py`（已实现）

已有的类型转换函数（统一行为：`None` → `None`，无效格式 → 抛 `SchemaTypeError`）：
- `to_date32()` → `date | None`
- `to_float64()` → `float | None`
- `to_int64()` → `int | None`
- `to_int8()` → `int | None`
- `to_bool()` → `bool | None`
- `to_string()` → `str | None`（永不抛异常）
- `to_timestamp()` → `datetime | None`
- `to_time32_ms()` → `time | None`（需新增）

#### 1.2 `common/schema.py`（已实现）

已有的工具函数：
- `typed_table()` → 从行数据和 schema 构建 `pa.Table`
- `_default_converter()` → 根据 PyArrow 类型返回默认转换函数

### 阶段 2：EastMoney Schema 改造（核心变更）

**目标：** 将 EastMoney 8 个端点从模式推断改为逐字段显式 schema。

#### 2.1 新增 `eastmoney/schemas.py`（逐字段定义）

为每个端点定义显式 `pa.Schema`，字段列表从 `docs/references/data_dict/eastmoney__*.md` 逐行提取。

```python
# eastmoney/schemas.py
# 逐字段显式 schema，与 data_dict 文档一一对应

EASTMONEY_BALANCE_SCHEMA = pa.schema([...])   # 319 字段
EASTMONEY_CASHFLOW_SQ_SCHEMA = pa.schema([...])  # 372 字段
EASTMONEY_CASHFLOW_YTD_SCHEMA = pa.schema([...])  # 254 字段
EASTMONEY_INCOME_SQ_SCHEMA = pa.schema([...])  # 299 字段
EASTMONEY_INCOME_YTD_SCHEMA = pa.schema([...])  # 203 字段
EASTMONEY_DIVIDEND_MAIN_SCHEMA = pa.schema([...])  # 30 字段
EASTMONEY_DIVIDEND_ALLOTMENT_SCHEMA = pa.schema([...])  # 10 字段
EASTMONEY_EQUITY_HISTORY_SCHEMA = pa.schema([...])  # 69 字段
```

#### 2.2 修改 `eastmoney/schema.py`

- 移除 `eastmoney_field_type()` 正则推断函数
- 移除 `_DATE_PATTERN`, `_BOOL_PATTERN`, `_NUMERIC_PATTERN` 正则
- 修改 `eastmoney_typed_schema()` 从 `schemas.py` 查表获取显式 schema
- 修改 `EASTMONEY_CONVERTERS` 支持 `pa.timestamp("ns")` 和 `pa.time32("ms")`

#### 2.3 新增 `to_time32_ms()` 转换函数

```python
def to_time32_ms(value: Any) -> time:
    """将 'HH:MM:SS' 转为 time 对象，失败抛出 SchemaTypeError。"""
    if isinstance(value, str) and ":" in value:
        parts = value.split(":")
        return time(int(parts[0]), int(parts[1]), int(parts[2]))
    raise SchemaTypeError(f"Cannot convert {type(value).__name__} '{value}' to time32[ms]")
```

### 阶段 3：HTTP Schema 改造

**目标：** 更新 THS、JiuYan schema 为校对后的类型。

#### 3.1 修改 `http/schemas.py`

更新 `THS_LIMIT_UP_POOL_SCHEMA`、`JIUYAN_ACTION_FIELD_SCHEMA`、`JIUYAN_INDUSTRY_LIST_SCHEMA` 为校对后的显式类型。

### 阶段 4：测试更新

**目标：** 更新测试夹具和断言以反映新 schema 类型。

#### 4.1 需要更新的测试文件

| 测试文件 | 改造内容 |
|---------|---------|
| `tests/unit/baostock/test_baostock.py` | 更新 schema 断言 |
| `tests/unit/sources/eastmoney/test_eastmoney.py` | 更新 schema 断言、逐字段类型验证 |
| `tests/unit/http/test_market_event_partitioning_and_schemas.py` | 更新 THS/JiuYan schema 断言 |
| `tests/unit/sources/jiuyan/test_ocr_schema.py` | 无需改动（全 string） |
| `tests/unit/storage/test_parquet_readers.py` | 更新 parquet 读取后的类型断言 |

#### 4.2 新增逐字段类型验证测试

```python
def test_balance_schema_field_types():
    """验证 balance schema 每个字段的类型与 data_dict 一致。"""
    from scheduler.defs.eastmoney.schemas import EASTMONEY_BALANCE_SCHEMA
    # REPORT_DATE 应为 date32
    assert EASTMONEY_BALANCE_SCHEMA.field("REPORT_DATE").type == pa.date32()
    # REPORT_DATE_NAME 应为 string
    assert EASTMONEY_BALANCE_SCHEMA.field("REPORT_DATE_NAME").type == pa.string()
    # TOTAL_ASSETS 应为 float64
    assert EASTMONEY_BALANCE_SCHEMA.field("TOTAL_ASSETS").type == pa.float64()
    # NOTICE_DATE 应为 date32
    assert EASTMONEY_BALANCE_SCHEMA.field("NOTICE_DATE").type == pa.date32()
```

## 实施顺序

### 步骤 1：通用基础设施（低风险）

1. 确认 `common/types.py` 和 `common/schema.py` 已实现。
2. 新增 `to_time32_ms()` 转换函数。
3. 编写单元测试覆盖所有转换函数。

### 步骤 2：EastMoney 逐字段 Schema（高风险，字段最多）

1. 创建 `eastmoney/schemas.py`，从 `docs/references/data_dict/eastmoney__*.md` 逐行提取字段定义。
2. 修改 `eastmoney/schema.py`，使用显式 schema 替代正则推断。
3. 更新 `EASTMONEY_CONVERTERS` 支持新类型。
4. 更新 `tests/unit/sources/eastmoney/test_eastmoney.py`。
5. 运行测试验证。

### 步骤 3：HTTP Schema 改造（中风险）

1. 更新 `http/schemas.py` 中的 THS/JiuYan schema。
2. 更新 `tests/unit/http/test_market_event_partitioning_and_schemas.py`。
3. 运行测试验证。

### 步骤 4：BaoStock 改造（低风险）

1. 确认 `baostock/schemas.py` 已使用显式 schema。
2. 更新测试验证。

### 步骤 5：Parquet 读取适配（低风险）

1. 更新 `tests/unit/storage/test_parquet_readers.py`。
2. 验证 parquet 写入 → 读取的类型 round-trip。

### 步骤 6：集成测试（低风险）

1. 运行完整测试套件。
2. 运行 `dg check defs` 验证。

## 验收标准

1. 所有数据源的 schema 使用显式逐字段定义，与 `docs/references/data_dict/` 一一对应。
2. S3 parquet 文件的列类型与 schema 一致。
3. 所有现有测试通过。
4. `REPORT_DATE` 在 `dividend_main` 中为 `string`，在其他财务报表端点中为 `date32`。
5. 所有时间字段使用正确的类型（`date32`、`timestamp`、`time32`）。
6. 所有布尔字段使用 `pa.bool_()`。
7. 所有数值字段使用 `pa.float64()`。
8. `dg check defs` 通过。
9. S3 路径、asset key、数据语义保持不变。

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 类型转换失败 | 资产物化失败，任务中断 | 转换函数对 None 安全，仅对无效格式抛 `SchemaTypeError` |
| EastMoney 字段遗漏 | schema 与 API 不一致 | 逐字段从 data_dict 提取，与 API 验证结果交叉校对 |
| `REPORT_DATE` 类型不一致 | dividend_main 转换失败 | 显式定义每个端点的 schema，不依赖通用推断 |
| `pa.concat_tables` 类型不匹配 | compact 资产写入失败 | 确保上游和 compact 使用相同 schema |
| BaoStock `outDate` 空字符串 | 转换函数返回 None | 在调用前检查空字符串，视为未退市（null） |

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `eastmoney/schemas.py` | **新增** | 8 个端点的逐字段显式 schema（~1500 字段定义） |
| `eastmoney/schema.py` | 修改 | 移除正则推断，改用显式 schema 查表 |
| `common/types.py` | 修改 | 新增 `to_time32_ms()` 转换函数 |
| `http/schemas.py` | 修改 | 更新 THS/JiuYan schema 为校对后类型 |
| `baostock/schemas.py` | 修改 | 确认使用显式 schema |
| `tests/unit/sources/eastmoney/test_eastmoney.py` | 修改 | 新增逐字段类型验证 |
| `tests/unit/http/test_market_event_partitioning_and_schemas.py` | 修改 | 更新 schema 断言 |
| `tests/unit/common/test_types.py` | 修改 | 新增 `to_time32_ms` 测试 |
