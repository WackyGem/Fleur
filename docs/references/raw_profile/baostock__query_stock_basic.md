# Raw 数据画像：baostock__query_stock_basic

日期：2026-06-03

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/baostock__query_stock_basic.yml`
- dbt source：`source('raw', 'baostock__query_stock_basic')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/baostock/stg_baostock__query_stock_basic.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`baostock__query_stock_basic`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table baostock__query_stock_basic --execute --status Accepted --output ../docs/references/raw_profile/baostock__query_stock_basic.md`
- 行数：8,769
- 数据范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`outDate`: 1995-12-31 至 2026-06-24，NULL 7,644 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`baostock__query_stock_basic`
- ClickHouse raw 表：`fleur_raw.baostock__query_stock_basic`
- 表说明：BaoStock security basic-information snapshot.

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：8,769。
  - 覆盖主体数：`code` 8,769 个
  - 日期 / 分区范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`outDate`: 1995-12-31 至 2026-06-24，NULL 7,644 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `code`。
  - 候选自然键去重结果：未发现重复。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`code` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：`code`: canonical 后缀 0/8,769，供应商前缀 8,769/8,769，纯数字 0/8,769，空值 0/8,769
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`type`: `1`(5,532), `5`(1,544), `4`(1,097), `2`(596)；`status`: `1`(7,644), `0`(1,125)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`outDate`: 1995-12-31 至 2026-06-24，NULL 7,644 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 2 个数值字段执行 min/max、NULL、零值和负值检查；其中 0 个字段出现负值，1 个字段出现零值，0 个字段 NULL 数不低于 80%。
  - 单位判断：本报告保留 raw 字段单位；金额、股数、比例和价格单位必须在具体 staging YAML metadata 中记录。
- 其他观察
  - 对 staging 有影响的事实只限确定性格式、类型、NULL/占位和候选键；跨源主数据修正、业务口径和去重优先级不进入 staging。

## 3. 粒度与键

- 观察到的粒度：`code`。
- 候选自然键：`code`。
- 重复检查：未发现重复。
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；候选键重复时保留 source-local 行并把版本选择延后。

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| code | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 8,769 | BaoStock 基础信息接口返回的证券代码。 |
| code_name | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 8,704 | BaoStock 基础信息接口返回的证券简称。 |
| ipoDate | Date | 0 | `1970-01-01` 0 | 1990-12-10 至 2026-06-01; distinct 3,731 | 证券上市日期。 |
| outDate | Nullable(Date) | 7,644 | `1970-01-01` 0 | 1995-12-31 至 2026-06-24; distinct 821 | 证券退市日期；未退市时通常为空。 |
| type | Int8 | 0 | 零值 0；负值 0 | min=1, max=5, distinct 4 | 证券类型代码。 |
| status | Int8 | 0 | 零值 1,125；负值 0 | min=0, max=1, distinct 2 | 证券上市状态。 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`code`
- 观察到的格式：`code`: canonical 后缀 0/8,769，供应商前缀 8,769/8,769，纯数字 0/8,769，空值 0/8,769
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`ipoDate`, `outDate`
- 范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`outDate`: 1995-12-31 至 2026-06-24，NULL 7,644 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`type`, `status`
- 取值：`type`: `1`(5,532), `5`(1,544), `4`(1,097), `2`(596)；`status`: `1`(7,644), `0`(1,125)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 2 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 2 个数值字段执行 min/max、NULL、零值和负值检查；其中 0 个字段出现负值，1 个字段出现零值，0 个字段 NULL 数不低于 80%。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `code` 使用供应商前缀格式 | 中 | 8,769/8,769 行为 `sh.000001` 类格式 | staging 可确定性转为 canonical 后缀格式并拆出交易所 | 跨源主数据修正延后 |

## 7. Staging 设计决策

- 重命名：按 `pipeline/elt/metadata/field_glossary.yml` 选择 canonical 字段；不要仅凭 raw 字段名自动扩展全部宽表字段。
- 类型转换：Date/DateTime/Bool/Float/Int 保持或显式 cast；字符串日期、报告期和供应商布尔/状态字段需在 staging SQL 中记录转换。
- 标准化：证券代码、交易所、本地代码使用项目 macro；文本清洗限于 trim/nullif 等 source-local 规则。
- NULL 处理：空字符串、`1970-01-01` 和明确缺失值可转 NULL，但必须在 YAML `config.meta.normalization` 记录来源字段和规则。
- 测试：候选键字段、日期字段和 canonical security code 优先加 `not_null`/格式 tests；宽表指标不加低价值全字段 `not_null`。
- YAML 元数据：每个 staging 输出字段必须记录 `config.meta.source_columns`；派生字段记录 `derived_from` 和 normalization metadata。

## 8. 延后到 Intermediate/Mart

- 跨源 join：证券主数据、行业/题材实体匹配、财务 statement 合并均延后。
- 需要优先级判断的去重：候选键重复或多公告版本选择不在 staging 静默处理。
- 主数据修正：证券代码历史、上市/退市状态、交易所归属修正延后。
- 粒度变化：财报宽表拆长表、事件合并、题材归并和行情事实组装延后。
- 业务指标逻辑：财务科目重算、同比/环比口径、分红状态解释和复杂文本归一化延后。

## 待确认问题

- [ ] 具体 staging model 落地时，针对实际暴露字段补充更细的字段级 tests 和单位 metadata。
- [ ] 如候选键重复或事件日期顺序需要业务解释，在 intermediate/mart 设计中确认去重优先级和时间线规则。

## 关键 SQL 证据摘要

- 行数：8,769。
- 日期 / 分区范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`outDate`: 1995-12-31 至 2026-06-24，NULL 7,644 行，`1970-01-01` 占位 0 行
- 候选键重复：未发现重复。
- 关键 NULL / 占位值：`code` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`type`: `1`(5,532), `5`(1,544), `4`(1,097), `2`(596)；`status`: `1`(7,644), `0`(1,125)
- 数值范围：已对 2 个数值字段执行 min/max、NULL、零值和负值检查；其中 0 个字段出现负值，1 个字段出现零值，0 个字段 NULL 数不低于 80%。

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
select `code`, `ipoDate`, `outDate`, `type`, `status` from fleur_raw.baostock__query_stock_basic limit 5
```

结果：

```text
[{'code': 'sh.000001', 'ipoDate': datetime.date(1991, 7, 15), 'outDate': None, 'type': 2, 'status': 1}, {'code': 'sh.000002', 'ipoDate': datetime.date(1992, 2, 21), 'outDate': None, 'type': 2, 'status': 1}, {'code': 'sh.000003', 'ipoDate': datetime.date(1992, 8, 17), 'outDate': None, 'type': 2, 'status': 1}, {'code': 'sh.000004', 'ipoDate': datetime.date(1993, 5, 3), 'outDate': None, 'type': 2, 'status': 1}, {'code': 'sh.000005', 'ipoDate': datetime.date(1993, 5, 3), 'outDate': None, 'type': 2, 'status': 1}]
```

### 行数统计

```sql
select count() from fleur_raw.baostock__query_stock_basic
```

结果：

```text
[[8769]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `code`, count() as row_count from fleur_raw.baostock__query_stock_basic group by `code` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 0, 'max_rows_per_key': 0}
```

### 证券代码格式：code

```sql
select countIf(match(toString(`code`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`code`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`code`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`code`) or toString(`code`) = '') as empty_or_null, count() as row_count from fleur_raw.baostock__query_stock_basic
```

结果：

```text
{'canonical_suffix': 0, 'vendor_prefix': 8769, 'numeric_only': 0, 'empty_or_null': 0, 'row_count': 8769}
```
