# Raw 数据画像：eastmoney__equity_history

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__equity_history.yml`
- dbt source：`source('raw', 'eastmoney__equity_history')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__equity_history.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`eastmoney__equity_history`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__equity_history --execute --output ../docs/references/raw_profile/eastmoney__equity_history.md`
- 行数：146,365
- 数据范围：`1990-12-19` 至 `2026-06-10`
- 分区范围：按自然年生成的 raw 分区已汇总到同一张 ClickHouse raw 表
- 契约数据集：`eastmoney__equity_history`
- ClickHouse raw 表：`raw.eastmoney__equity_history`
- 表说明：东方财富股本变动 F10 数据，raw 侧按自然年分区采集。

## 2. 粒度与键

- 观察到的粒度：每个证券、每个股本变动截止日一行
- 候选自然键：`SECUCODE`, `END_DATE`
- 重复检查：profiling 查询未返回重复的 `(SECUCODE, END_DATE)`
- 粒度注意事项：`END_DATE` 可能晚于 profiling 当日；不要把它解释为入库时间

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| SECUCODE | LowCardinality(String) | 0 | 0 个空字符串/NULL | 所有行均匹配 `000001.SZ` / `600000.SH` 这类规范后缀格式 | 来自 `eastmoney` 原始字段 `SECUCODE`。原始字段说明：证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 0 | 0 个空字符串/NULL | 所有行均匹配纯 6 位本地代码 | 来自 `eastmoney` 原始字段 `SECURITY_CODE`。原始字段说明：证券代码（纯数字） |
| ORG_CODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ORG_CODE`。原始字段说明：机构代码 |
| END_DATE | Date | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_DATE`。原始字段说明：股本变动截止日 |
| CHANGE_REASON | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CHANGE_REASON`。原始字段说明：变动原因 |
| LIMITED_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_SHARES`。原始字段说明：有限售条件股份 |
| UNLIMITED_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNLIMITED_SHARES`。原始字段说明：无限售条件股份（已流通） |
| TOTAL_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_SHARES`。原始字段说明：总股本 |
| LIMITED_SHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_SHARES_RATIO`。原始字段说明：限售股比例（%） |
| LISTED_SHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_SHARES_RATIO`。原始字段说明：已流通股比例（%） |
| TOTAL_SHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_SHARES_RATIO`。原始字段说明：总股本比例（%） |
| LISTED_A_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_A_SHARES`。原始字段说明：已上市流通 A 股 |
| LIMITED_A_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_A_SHARES`。原始字段说明：限售 A 股 |
| LISTED_A_SHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_A_SHARES_RATIO`。原始字段说明：A 股流通比例（%） |
| LIMITED_A_SHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_A_SHARES_RATIO`。原始字段说明：限售A股比例（%） |
| B_FREE_SHARE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `B_FREE_SHARE`。原始字段说明：已上市流通 B 股 |
| H_FREE_SHARE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `H_FREE_SHARE`。原始字段说明：已上市流通 H 股 |
| B_FREE_SHARE_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `B_FREE_SHARE_RATIO`。原始字段说明：B股流通比例（%） |
| H_FREE_SHARE_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `H_FREE_SHARE_RATIO`。原始字段说明：H 股流通比例（%） |
| SECURITY_TYPE_CODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECURITY_TYPE_CODE`。原始字段说明：证券类型代码 |
| NON_FREE_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NON_FREE_SHARES`。原始字段说明：非自由流通股 |
| NON_FREESHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NON_FREESHARES_RATIO`。原始字段说明：非流通股比例（%） |
| LIMITED_B_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_B_SHARES`。原始字段说明：限售 B 股 |
| LIMITED_BSHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_BSHARES_RATIO`。原始字段说明：限售B股比例（%） |
| OTHER_FREE_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_FREE_SHARES`。原始字段说明：其他已上市流通股 |
| OTHER_FREESHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_FREESHARES_RATIO`。原始字段说明：其他流通股比例（%） |
| LIMITED_STATE_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_STATE_SHARES`。原始字段说明：国家持股（限售） |
| LIMITED_STATE_LEGAL | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_STATE_LEGAL`。原始字段说明：国有法人持股（限售） |
| LIMITED_OTHARS | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_OTHARS`。原始字段说明：其他限售股份 |
| LIMITED_DOMESTIC_NOSTATE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_DOMESTIC_NOSTATE`。原始字段说明：境内非国有法人持股（限售） |
| LIMITED_DOMESTIC_NATURAL | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_DOMESTIC_NATURAL`。原始字段说明：境内自然人持股（限售） |
| LOCK_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOCK_SHARES`。原始字段说明：锁定股份 |
| LIMITED_FOREIGN_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_FOREIGN_SHARES`。原始字段说明：外资持股（限售） |
| LIMITED_OVERSEAS_NOSTATE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_OVERSEAS_NOSTATE`。原始字段说明：境外非国有法人持股（限售） |
| LIMITED_OVERSEAS_NATURAL | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_OVERSEAS_NATURAL`。原始字段说明：境外自然人持股（限售） |
| LIMITED_H_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_H_SHARES`。原始字段说明：限售 H 股 |
| SPONSOR_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SPONSOR_SHARES`。原始字段说明：发起人股份 |
| STATE_SPONSOR_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `STATE_SPONSOR_SHARES`。原始字段说明：国家发起人股份 |
| SPONSOR_SOCIAL_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SPONSOR_SOCIAL_SHARES`。原始字段说明：社会发起人股份 |
| RAISE_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RAISE_SHARES`。原始字段说明：募集法人股份 |
| RAISE_STATE_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RAISE_STATE_SHARES`。原始字段说明：国家募集法人股份 |
| RAISE_DOMESTIC_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RAISE_DOMESTIC_SHARES`。原始字段说明：境内募集法人股份 |
| RAISE_OVERSEAS_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RAISE_OVERSEAS_SHARES`。原始字段说明：境外募集法人股份 |
| NOTICE_DATE | Date | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTICE_DATE`。原始字段说明：公告披露日 |
| LISTING_DATE | Date | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTING_DATE`。原始字段说明：上市流通日期 |
| LIMITED_SHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_SHARES_CHANGE`。原始字段说明：限售股变动量 |
| UNLIMITED_SHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNLIMITED_SHARES_CHANGE`。原始字段说明：流通股变动量 |
| TOTAL_SHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_SHARES_CHANGE`。原始字段说明：总股本变动量 |
| LISTED_ASHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_ASHARES_CHANGE`。原始字段说明：已上市流通A股变动量 |
| LIMITED_ASHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_ASHARES_CHANGE`。原始字段说明：限售A股变动量 |
| B_FREESHARE_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `B_FREESHARE_CHANGE`。原始字段说明：B股流通变动量 |
| H_FREESHARE_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `H_FREESHARE_CHANGE`。原始字段说明：H股流通变动量 |
| LIMITED_BSHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_BSHARES_CHANGE`。原始字段说明：限售B股变动量 |
| NONFREE_SHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONFREE_SHARES_CHANGE`。原始字段说明：非流通股变动量 |
| OTHERFREE_SHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHERFREE_SHARES_CHANGE`。原始字段说明：其他流通股变动量 |
| FREE_SHARES | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FREE_SHARES`。原始字段说明：流通股（通常 = TOTAL_SHARES） |
| CHANGE_REASON_EXPLAIN | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CHANGE_REASON_EXPLAIN`。原始字段说明：变动原因详细说明 |
| LIMITED_H_SHARES_RATIO | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_H_SHARES_RATIO`。原始字段说明：限售H股比例（%） |
| LIMITED_H_SHARES_CHANGE | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIMITED_H_SHARES_CHANGE`。原始字段说明：限售H股变动量 |
| IS_FREE_WINDOW | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IS_FREE_WINDOW`。原始字段说明：是否为自由流通窗口 |
| IS_LIMITED_WINDOW | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IS_LIMITED_WINDOW`。原始字段说明：是否限售窗口 |
| LISTED_A_RATIOPC | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_A_RATIOPC`。原始字段说明：A 股占已流通比例（%） |
| LISTED_B_RATIOPC | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_B_RATIOPC`。原始字段说明：B股占已流通比例（%） |
| LISTED_H_RATIOPC | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_H_RATIOPC`。原始字段说明：H 股占已流通比例（%） |
| LISTED_OTHER_RATIOPC | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_OTHER_RATIOPC`。原始字段说明：其他占已流通比例（%） |
| LISTED_SUM_RATIOPC | Float64 | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTED_SUM_RATIOPC`。原始字段说明：合计占已流通比例（%） |
| MARKET_CODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MARKET_CODE`。原始字段说明：市场代码 |
| IS_USE | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IS_USE`。原始字段说明：是否有效 |
| SECURITY_NAME_ABBR | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECURITY_NAME_ABBR`。原始字段说明：证券简称 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：146,365 行 `SECUCODE` 全部已经是规范后缀格式；`SECURITY_CODE` 全部是纯数字本地代码
- 无效样例：正则分布查询未发现无效样例
- 建议 staging 处理：使用 `SECUCODE` 作为 `security_code`、`security_local_code` 和 `exchange_code` 的来源，`input_format` 使用 `eastmoney_suffix`

### 日期与时间字段

- 已画像字段：`END_DATE`
- 范围：`1990-12-19` 至 `2026-06-10`
- 无效值或占位值：NULL 计数 profiling 未发现无效值
- 建议 staging 处理：将 `END_DATE` 直接映射为规范字段 `report_date`

### 枚举字段

- 已画像字段：`CHANGE_REASON`
- 取值：`CHANGE_REASON` 是高基数业务文本；高频值包括“高管股份变动”“债转股上市”“回购”“首发限售股份上市”“自主行权”
- 未知或异常取值：未按封闭枚举评估；不要为 `CHANGE_REASON` 添加 accepted-values 测试
- 建议 staging 处理：当前 baseline staging 不暴露变动原因；后续如暴露，除非先设计治理后的枚举域，否则按描述性文本处理

### 数值字段

- 已画像字段：`LIMITED_SHARES`, `UNLIMITED_SHARES`, `TOTAL_SHARES`
- 最小/最大值：股数类字段存在较大的正数范围；已画像字段未观察到负值
- 负数/零值/极端值：存在零值，尤其是限售股相关字段；已画像数据中 `TOTAL_SHARES` 没有零值
- 单位假设：保留 raw 股数单位；没有明确单位元数据时不要重缩放
- 建议 staging 处理：当前 baseline staging 仅暴露证券和日期字段；股数字段需要单独设计 staging 逻辑和测试

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECUCODE` 和 `SECURITY_CODE` 表示不同代码格式 | 中 | 正则分布显示 `SECUCODE` 为规范后缀格式，`SECURITY_CODE` 为纯数字 | 使用 `SECUCODE` 生成规范字段 `security_code`，并补充 normalization macro metadata（标准化宏元数据） | 无 |
| `END_DATE` 延伸至 2026-06-10 | 低 | 日期范围查询 | 保留为 `report_date`；不要当作入库新鲜度 | 下游模型可能需要 as-of-date 过滤 |
| 股数字段包含零值 | 中 | 数值范围查询 | 在单位和零值策略明确前，不在 baseline staging 暴露股数字段 | 股本事实建模放到后续 staging/intermediate 工作 |

## 6. 建议的 Staging 转换

- 重命名：`SECUCODE -> security_code`, derived `security_local_code`, derived `exchange_code`, `END_DATE -> report_date`
- 类型转换：当前暴露字段无需转换；raw `END_DATE` 已经是 `Date`
- 标准化：使用 `normalize_cn_security_code`、`cn_security_local_code` 和 `cn_exchange_code`，`input_format` 为 `eastmoney_suffix`
- NULL 处理：当前暴露字段未观察到 NULL
- 测试：`security_code` 添加 `not_null` + `cn_security_code_format`；`report_date` 添加 `not_null`；`exchange_code` 添加 accepted values（允许值）`SH`、`SZ`、`BJ`
- YAML 元数据：source lineage（源字段血缘）指向 `raw.eastmoney__equity_history.SECUCODE` 和 `.END_DATE`；代码派生字段补充 normalization metadata（标准化元数据）

## 7. 延后到 Intermediate/Mart

- 跨源 join：staging 不做；与证券主数据的 join 放到 intermediate/mart
- 需要优先级判断的去重：未观察到 `(SECUCODE, END_DATE)` 重复
- 主数据修正：staging 不做证券主数据修正
- 粒度变化：股本事实建模和 equity-history 宽表设计延后处理
- 业务指标逻辑：股数和比例的业务解释延后处理

## 8. 待确认问题

- [ ] 后续确认 equity-history 股数字段是否应扩展到独立 staging 设计，还是继续延后到 intermediate 股本事实模型。

## 9. 验收清单

- [x] 已抽样 raw source。
- [x] 已记录行数和日期/分区范围。
- [x] 已评估粒度和候选键。
- [x] 已完成关键字段画像。
- [x] 已列出 staging 转换建议。
- [x] 已列出延后处理事项。
- [x] 已提出测试或明确豁免。

## Profiling SQL 与结果

### 样例行

```sql
select *
from {{ source('raw', 'eastmoney__equity_history') }}
```


结果（成功）：

```text
15:36:32  Running with dbt=1.11.11
15:36:33  Registered adapter: clickhouse=1.10.0
15:36:33  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:33  
15:36:33  Concurrency: 1 threads (target='dev')
15:36:33  
Previewing inline node:
| SECUCODE  | SECURITY_CODE | ORG_CODE |   END_DATE | CHANGE_REASON | LIMITED_SHARES | ... |
| --------- | ------------- | -------- | ---------- | ------------- | -------------- | --- |
| 600651.SH | 600651        | 10003961 | 1990-12-19 | 首发A股上市        |              0 | ... |
| 600653.SH | 600653        | 10003963 | 1990-12-19 | 首发A股上市        |              0 | ... |
| 600656.SH | 600656        | 10003966 | 1990-12-19 | 首发A股上市        |              0 | ... |
| 000001.SZ | 000001        | 10004085 | 1991-08-17 | 送股上市          |              0 | ... |
| 000001.SZ | 000001        | 10004085 | 1991-12-31 | 股份性质变更        |              0 | ... |
| 000001.SZ | 000001        | 10004085 | 1992-03-23 | 送股上市          |              0 | ... |
| 000002.SZ | 000002        | 10004086 | 1991-06-08 | 送股上市          |              0 | ... |
| 000004.SZ | 000004        | 10004088 | 1991-01-14 | 首发A股上市        |              0 | ... |
| 000004.SZ | 000004        | 10004088 | 1991-06-28 | 送股上市          |              0 | ... |
| 600601.SH | 600601        | 10002659 | 1991-03-11 | 拆细            |              0 | ... |
| 600602.SH | 600602        | 10002660 | 1992-02-21 | 首发B股上市        |              0 | ... |
| 600651.SH | 600651        | 10003961 | 1991-08-26 | 拆细            |              0 | ... |
| 600651.SH | 600651        | 10003961 | 1991-11-29 | 增发A股上市        |              0 | ... |
| 600652.SH | 600652        | 10003962 | 1991-08-26 | 增发A股上市,拆细     |              0 | ... |
| 600653.SH | 600653        | 10003963 | 1991-02-26 | 拆细            |              0 | ... |
| 000001.SZ | 000001        | 10004085 | 1992-12-31 | 股份性质变更        |              0 | ... |
| 000002.SZ | 000002        | 10004086 | 1992-03-30 | 送股上市          |              0 | ... |
| 000004.SZ | 000004        | 10004088 | 1992-04-27 | 送股上市          |              0 | ... |
| 000006.SZ | 000006        | 10004090 | 1992-12-31 | 定期报告          |              0 | ... |
| 000007.SZ | 000007        | 10004091 | 1992-11-09 | 送股上市          |              0 | ... |
| 000009.SZ | 000009        | 10004093 | 1992-12-31 | 定期报告          |              0 | ... |
| 000013.SZ | 000013        | 10004097 | 1992-05-06 | 首发A股上市,首发B股上市 |              0 | ... |
| 000015.SZ | 000015        | 10004099 | 1992-06-25 | 首发A股上市,首发B股上市 |              0 | ... |
| 000017.SZ | 000017        | 10004101 | 1992-03-31 | 首发A股上市,首发B股上市 |              0 | ... |
| 000019.SZ | 000019        | 10004103 | 1992-10-12 | 首发A股上市,首发B股上市 |              0 | ... |
| 600601.SH | 600601        | 10002659 | 1992-06-04 | 配售A股上市        |              0 | ... |
| 600601.SH | 600601        | 10002659 | 1992-12-10 | 拆细            |              0 | ... |
| 600602.SH | 600602        | 10002660 | 1992-12-01 | 拆细            |              0 | ... |
| 600603.SH | 600603        | 10002661 | 1992-12-10 | 拆细            |              0 | ... |
| 600604.SH | 600604        | 10002662 | 1992-07-01 | 首发B股上市        |              0 | ... |
| 600604.SH | 600604        | 10002662 | 1992-12-10 | 拆细            |              0 | ... |
| 600605.SH | 600605        | 10002663 | 1992-12-10 | 拆细            |              0 | ... |
| 600606.SH | 600606        | 10002664 | 1992-12-10 | 拆细            |              0 | ... |
| 600607.SH | 600607        | 10002665 | 1992-12-10 | 拆细            |              0 | ... |
| 600608.SH | 600608        | 10002666 | 1992-12-10 | 拆细            |              0 | ... |
| 600609.SH | 600609        | 10002667 | 1992-12-10 | 拆细            |              0 | ... |
| 600609.SH | 600609        | 10002667 | 1992-12-28 | 送股上市          |              0 | ... |
| 600609.SH | 600609        | 10002667 | 1993-01-29 | 配售A股上市        |              0 | ... |
| 600610.SH | 600610        | 10002668 | 1992-08-05 | 首发A股上市        |              0 | ... |
| 600610.SH | 600610        | 10002668 | 1992-12-10 | 拆细            |              0 | ... |
| 600611.SH | 600611        | 10002669 | 1992-12-10 | 拆细            |              0 | ... |
| 600612.SH | 600612        | 10002670 | 1992-12-10 | 拆细            |              0 | ... |
| 600613.SH | 600613        | 10002671 | 1992-12-10 | 拆细            |              0 | ... |
| 600614.SH | 600614        | 10003924 | 1992-12-10 | 拆细            |              0 | ... |
| 600615.SH | 600615        | 10003925 | 1992-12-10 | 拆细            |              0 | ... |
| 600616.SH | 600616        | 10003926 | 1992-12-10 | 拆细            |              0 | ... |
| 600617.SH | 600617        | 10003927 | 1992-12-10 | 拆细            |              0 | ... |
| 600618.SH | 600618        | 10003928 | 1992-12-10 | 拆细            |              0 | ... |
| 600620.SH | 600620        | 10003930 | 1992-12-10 | 拆细            |              0 | ... |
| 600621.SH | 600621        | 10003931 | 1992-12-10 | 拆细            |              0 | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'eastmoney__equity_history') }}
```


结果（成功）：

```text
15:36:37  Running with dbt=1.11.11
15:36:37  Registered adapter: clickhouse=1.10.0
15:36:38  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:38  
15:36:38  Concurrency: 1 threads (target='dev')
15:36:38  
Previewing inline node:
| row_count |
| --------- |
|    146365 |
```

### 日期范围

```sql
select
    min(`END_DATE`) as min_end_date,
    max(`END_DATE`) as max_end_date,
    countIf(isNull(`END_DATE`)) as null_end_date
from {{ source('raw', 'eastmoney__equity_history') }}
```


结果（成功）：

```text
15:36:42  Running with dbt=1.11.11
15:36:43  Registered adapter: clickhouse=1.10.0
15:36:43  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:43  
15:36:43  Concurrency: 1 threads (target='dev')
15:36:43  
Previewing inline node:
| min_end_date | max_end_date | null_end_date |
| ------------ | ------------ | ------------- |
|   1990-12-19 |   2026-06-10 |             0 |
```

### 候选键重复检查

```sql
select
    `SECUCODE`, `END_DATE`,
    count(*) as row_count
from {{ source('raw', 'eastmoney__equity_history') }}
group by `SECUCODE`, `END_DATE`
having row_count > 1
order by row_count desc
```


结果（成功）：

```text
15:36:47  Running with dbt=1.11.11
15:36:47  Registered adapter: clickhouse=1.10.0
15:36:48  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:48  
15:36:48  Concurrency: 1 threads (target='dev')
15:36:48  
Previewing inline node:
||
|  |
```

### 格式分布：SECUCODE

```sql
select
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__equity_history') }}
```


结果（成功）：

```text
15:36:52  Running with dbt=1.11.11
15:36:52  Registered adapter: clickhouse=1.10.0
15:36:53  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:53  
15:36:53  Concurrency: 1 threads (target='dev')
15:36:53  
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|           146365 |             0 |            0 |             0 |    146365 |
```

### 格式分布：SECURITY_CODE

```sql
select
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__equity_history') }}
```


结果（成功）：

```text
15:37:00  Running with dbt=1.11.11
15:37:01  Registered adapter: clickhouse=1.10.0
15:37:01  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:01  
15:37:01  Concurrency: 1 threads (target='dev')
15:37:01  
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |       146365 |             0 |    146365 |
```

### 高频取值：CHANGE_REASON

```sql
select
    `CHANGE_REASON` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__equity_history') }}
group by `CHANGE_REASON`
order by row_count desc
```


结果（成功）：

```text
15:37:05  Running with dbt=1.11.11
15:37:05  Registered adapter: clickhouse=1.10.0
15:37:06  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:06  
15:37:06  Concurrency: 1 threads (target='dev')
15:37:06  
Previewing inline node:
| value        | row_count |
| ------------ | --------- |
| 高管股份变动       |     33611 |
| 债转股上市        |     15171 |
| 回购           |      9840 |
| 首发限售股份上市     |      9650 |
| 自主行权         |      9266 |
| 网下配售股份上市     |      8951 |
| 转增股上市        |      8920 |
| 股份性质变更       |      7986 |
| 限制性股票        |      6232 |
| 增发A股上市       |      5980 |
| 股权激励限售流通股上市  |      5951 |
| 股改限售流通股上市    |      4708 |
| 定期报告         |      3919 |
| 送股上市         |      2734 |
| 债转股上市,高管股份变动 |      1949 |
| 送股上市,转增股上市   |      1242 |
| 配售A股上市       |      1174 |
| 其他限售股上市      |      1068 |
| 期权行权         |      1063 |
| 职工股上市        |       765 |
```

### 数值范围：LIMITED_SHARES

```sql
select
    min(`LIMITED_SHARES`) as min_value,
    max(`LIMITED_SHARES`) as max_value,
    countIf(`LIMITED_SHARES` = 0) as zero_count,
    countIf(`LIMITED_SHARES` < 0) as negative_count,
    countIf(isNull(`LIMITED_SHARES`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__equity_history') }}
```


结果（成功）：

```text
15:37:12  Running with dbt=1.11.11
15:37:12  Registered adapter: clickhouse=1.10.0
15:37:13  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:13  
15:37:13  Concurrency: 1 threads (target='dev')
15:37:13  
Previewing inline node:
| min_value |       max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 283,744,968,904 |      18271 |              0 |          0 |    146365 |
```

### 数值范围：UNLIMITED_SHARES

```sql
select
    min(`UNLIMITED_SHARES`) as min_value,
    max(`UNLIMITED_SHARES`) as max_value,
    countIf(`UNLIMITED_SHARES` = 0) as zero_count,
    countIf(`UNLIMITED_SHARES` < 0) as negative_count,
    countIf(isNull(`UNLIMITED_SHARES`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__equity_history') }}
```


结果（成功）：

```text
15:37:20  Running with dbt=1.11.11
15:37:20  Registered adapter: clickhouse=1.10.0
15:37:21  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:21  
15:37:21  Concurrency: 1 threads (target='dev')
15:37:21  
Previewing inline node:
| min_value |       max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 356,406,257,089 |         44 |              0 |          0 |    146365 |
```

### 数值范围：TOTAL_SHARES

```sql
select
    min(`TOTAL_SHARES`) as min_value,
    max(`TOTAL_SHARES`) as max_value,
    countIf(`TOTAL_SHARES` = 0) as zero_count,
    countIf(`TOTAL_SHARES` < 0) as negative_count,
    countIf(isNull(`TOTAL_SHARES`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__equity_history') }}
```


结果（成功）：

```text
15:37:25  Running with dbt=1.11.11
15:37:25  Registered adapter: clickhouse=1.10.0
15:37:26  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:26  
15:37:26  Concurrency: 1 threads (target='dev')
15:37:26  
Previewing inline node:
| min_value |       max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------------- | ---------- | -------------- | ---------- | --------- |
|    33,000 | 356,406,257,089 |          0 |              0 |          0 |    146365 |
```
