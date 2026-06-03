# Raw 数据画像：eastmoney__income_sq

日期：2026-06-03

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__income_sq.yml`
- dbt source：`source('raw', 'eastmoney__income_sq')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__income_sq.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__income_sq`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__income_sq --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__income_sq.md`
- 行数：279,918
- 数据范围：`REPORT_DATE`: 1993-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1993-08-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1993-08-14 至 2026-06-02，NULL 193 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`eastmoney__income_sq`
- ClickHouse raw 表：`fleur_raw.eastmoney__income_sq`
- 表说明：EastMoney single-quarter income F10 rows by natural-year raw partition.

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：279,918。
  - 覆盖主体数：`secucode` 5,418 个；`security_code` 5,418 个
  - 日期 / 分区范围：`REPORT_DATE`: 1993-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1993-08-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1993-08-14 至 2026-06-02，NULL 193 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `SECUCODE`, `REPORT_DATE`。
  - 候选自然键去重结果：未发现重复。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：`SECUCODE`: canonical 后缀 279,918/279,918，供应商前缀 0/279,918，纯数字 0/279,918，空值 0/279,918；`SECURITY_CODE`: canonical 后缀 0/279,918，供应商前缀 0/279,918，纯数字 279,918/279,918，空值 0/279,918
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`SECURITY_CODE`: `000553`(103), `000025`(103), `000411`(102), `000592`(102), `600081`(101), `600717`(101), `000869`(101), `600703`(101)；`SECURITY_NAME_ABBR`: `东方明珠`(150), `百联股份`(135), `特力A`(103), `安道麦A`(103), `英特集团`(102), `平潭发展`(102), `诚志股份`(101), `百花医药`(101)；`ORG_CODE`: `10004127`(168), `10004106`(164), `10116535`(126), `10004293`(124), `10004109`(103), `10005499`(103), `10564780`(102), `10005533`(102)；`ORG_TYPE`: `通用`(279,918)；`REPORT_TYPE`: `一季度`(73,061), `三季度`(69,893), `四季度`(69,543), `二季度`(67,421)；`REPORT_DATE_NAME`: `2026一季度`(5,099), `2025四季度`(5,084), `2024一季度`(5,073), `2025一季度`(5,072), `2025三季度`(5,060), `2025二季度`(5,051), `2024三季度`(5,026), `2024四季度`(5,019)；`SECURITY_TYPE_CODE`: `058001001`(279,895), `058001008`(23)；`CURRENCY`: `CNY`(279,424), `NULL`(494)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`REPORT_DATE`: 1993-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1993-08-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1993-08-14 至 2026-06-02，NULL 193 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 285 个数值字段执行 min/max、NULL、零值和负值检查；其中 247 个字段出现负值，195 个字段出现零值，169 个字段 NULL 数不低于 80%。 负值字段样例：`PARENT_NETPROFIT_QOQ` 136,516 行(min=-11,225,600,000)，`OPERATE_PROFIT_QOQ` 136,299 行(min=-19,177,700)，`NETPROFIT_QOQ` 136,277 行(min=-14,944,200)，`TOTAL_PROFIT_QOQ` 136,235 行(min=-30,705,900)，`FINANCE_EXPENSE_QOQ` 132,272 行(min=-25,720,300)，`DEDUCT_PARENT_NETPROFIT_QOQ` 131,193 行(min=-13,312,900)，`OPERATE_TAX_ADD_QOQ` 129,390 行(min=-2,203,570,000)，`INCOME_TAX_QOQ` 128,374 行(min=-21,887,500,000)。 高 NULL 字段样例：`ME_RESEARCH_EXPENSE` 279,918 行，`ME_RESEARCH_EXPENSE_QOQ` 279,918 行，`OPERATE_PROFIT_BALANCE_QOQ` 279,918 行，`TOTAL_PROFIT_BALANCE_QOQ` 279,918 行，`NETPROFIT_BALANCE_QOQ` 279,918 行，`EFFECT_NETPROFIT_BALANCE_QOQ` 279,918 行，`CREDITRISK_FAIRVALUE_CHANGE_QOQ` 279,918 行，`UNABLE_OCI_BALANCE` 279,918 行。
  - 单位判断：本报告保留 raw 字段单位；金额、股数、比例和价格单位必须在具体 staging YAML metadata 中记录。
- 其他观察
  - 对 staging 有影响的事实只限确定性格式、类型、NULL/占位和候选键；跨源主数据修正、业务口径和去重优先级不进入 staging。

## 3. 粒度与键

- 观察到的粒度：`SECUCODE`, `REPORT_DATE`。
- 候选自然键：`SECUCODE`, `REPORT_DATE`。
- 重复检查：未发现重复。
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；候选键重复时保留 source-local 行并把版本选择延后。

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| SECUCODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,418 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,418 | 证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,416 | 证券简称 |
| ORG_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,411 | 机构代码 |
| ORG_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 机构类型 |
| REPORT_DATE | Date | 0 | `1970-01-01` 0 | 1993-06-30 至 2026-03-31; distinct 115 | 报告期 |
| REPORT_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 4 | 报告类型 |
| REPORT_DATE_NAME | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 115 | 报告期名称 |
| SECURITY_TYPE_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 2 | 证券类型代码 |
| NOTICE_DATE | Date | 0 | `1970-01-01` 0 | 1993-08-14 至 2026-05-15; distinct 3,401 | 公告日期 |
| UPDATE_DATE | Nullable(Date) | 193 | `1970-01-01` 0 | 1993-08-14 至 2026-06-02; distinct 4,236 | 更新日期 |
| CURRENCY | LowCardinality(Nullable(String)) | 494 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 利润表单季度金额使用的币种。 |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 204,587 | 空字符串 0；`1970-01-01` 0 | distinct 7 | 审计意见类型 |
| OSOPINION_TYPE | LowCardinality(Nullable(String)) | 279,903 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 内控审计意见类型 |
| TOTAL_OPERATE_INCOME | Nullable(Float64) | 736 | 零值 282；负值 526 | min=-38,213,600,000, max=876,259,000,000, distinct 278,612 | 营业总收入 |
| TOTAL_OPERATE_INCOME_QOQ | Nullable(Float64) | 3,910 | 零值 44；负值 124,346 | min=-300,982, max=8,782,390, distinct 275,604 | 营业总收入环比增长率（%） |
| OPERATE_INCOME | Nullable(Float64) | 848 | 零值 215；负值 527 | min=-38,213,600,000, max=876,259,000,000, distinct 278,567 | 营业收入 |
| OPERATE_INCOME_QOQ | Nullable(Float64) | 4,332 | 零值 44；负值 124,117 | min=-300,982, max=8,782,390, distinct 275,206 | 营业收入环比增长率（%） |
| INTEREST_INCOME | Nullable(Float64) | 273,279 | 零值 1,820；负值 75 | min=-530,424,000, max=9,685,390,000, distinct 4,804 | 利息收入 |
| INTEREST_INCOME_QOQ | Nullable(Float64) | 275,329 | 零值 9；负值 2,286 | min=-653.824, max=254,402, distinct 4,534 | 利息收入环比增长率（%） |
| EARNED_PREMIUM | Nullable(Float64) | 277,565 | 零值 1,868；负值 13 | min=-1,333,880,000, max=8,426,690,000, distinct 486 | 已赚保费 |
| EARNED_PREMIUM_QOQ | Nullable(Float64) | 279,456 | 零值 0；负值 217 | min=-295.3, max=7,315.87, distinct 456 | 已赚保费环比增长率（%） |
| FEE_COMMISSION_INCOME | Nullable(Float64) | 275,535 | 零值 1,917；负值 66 | min=-948,081,000, max=3,972,880,000, distinct 2,452 | 手续费及佣金收入 |
| FEE_COMMISSION_INCOME_QOQ | Nullable(Float64) | 277,597 | 零值 4；负值 1,140 | min=-2,433.33, max=8,344,900,000, distinct 2,279 | 手续费及佣金收入环比增长率（%） |
| OTHER_BUSINESS_INCOME | Nullable(Float64) | 279,732 | 零值 14；负值 18 | min=-11,075,100, max=561,708,000, distinct 173 | 其他业务收入 |
| OTHER_BUSINESS_INCOME_QOQ | Nullable(Float64) | 279,880 | 零值 0；负值 20 | min=-91.2101, max=6,641.02, distinct 38 | 其他业务收入环比增长率（%） |
| TOI_OTHER | Nullable(Float64) | 279,403 | 零值 397；负值 12 | min=-4,527,980,000, max=1,575,990,000, distinct 119 | 营业总收入其他 |
| TOI_OTHER_QOQ | Nullable(Float64) | 279,811 | 零值 0；负值 49 | min=-1,944.27, max=44,359, distinct 107 | 营业总收入其他环比增长率（%） |
| TOTAL_OPERATE_COST | Nullable(Float64) | 35 | 零值 185；负值 427 | min=-31,214,800,000, max=858,308,000,000, distinct 279,469 | 营业总成本 |
| TOTAL_OPERATE_COST_QOQ | Nullable(Float64) | 3,490 | 零值 0；负值 121,623 | min=-4,615.81, max=1,161,590, distinct 276,125 | 营业总成本环比增长率（%） |
| OPERATE_COST | Nullable(Float64) | 919 | 零值 228；负值 543 | min=-30,122,400,000, max=752,738,000,000, distinct 278,459 | 营业成本 |
| OPERATE_COST_QOQ | Nullable(Float64) | 4,532 | 零值 49；负值 123,225 | min=-134,925, max=8,837,790, distinct 275,000 | 营业成本环比增长率（%） |
| INTEREST_EXPENSE | Nullable(Float64) | 275,507 | 零值 1,884；负值 91 | min=-273,721,000, max=5,248,700,000, distinct 2,527 | 利息支出 |
| INTEREST_EXPENSE_QOQ | Nullable(Float64) | 277,552 | 零值 0；负值 1,204 | min=-4,755.83, max=34,020.2, distinct 2,343 | 利息支出环比增长率（%） |
| FEE_COMMISSION_EXPENSE | Nullable(Float64) | 275,744 | 零值 1,858；负值 64 | min=-943,149,000, max=910,542,000, distinct 2,305 | 手续费及佣金支出 |
| FEE_COMMISSION_EXPENSE_QOQ | Nullable(Float64) | 277,729 | 零值 3；负值 1,057 | min=-2,122.75, max=1,239,940, distinct 2,167 | 手续费及佣金支出环比增长率（%） |
| RESEARCH_EXPENSE | Nullable(Float64) | 141,970 | 零值 532；负值 1,036 | min=-5,929,320,000, max=21,749,900,000, distinct 137,230 | 研发费用 |
| RESEARCH_EXPENSE_QOQ | Nullable(Float64) | 146,633 | 零值 51；负值 60,832 | min=-10,029.3, max=213,542,000,000, distinct 133,007 | 研发费用环比增长率（%） |
| SURRENDER_VALUE | Nullable(Float64) | 278,026 | 零值 1,834；负值 2 | min=-3,670,120,000, max=1,819,800,000, distinct 59 | 退保金 |
| SURRENDER_VALUE_QOQ | Nullable(Float64) | 279,865 | 零值 0；负值 30 | min=-11,596.8, max=515.118, distinct 53 | 退保金环比增长率（%） |
| NET_COMPENSATE_EXPENSE | Nullable(Float64) | 277,821 | 零值 1,950；负值 1 | min=-2,516,160, max=1,040,090,000, distinct 148 | 分保费用 |
| NET_COMPENSATE_EXPENSE_QOQ | Nullable(Float64) | 279,780 | 零值 0；负值 60 | min=-153.595, max=27,452.1, distinct 138 | 分保费用环比增长率（%） |
| NET_CONTRACT_RESERVE | Nullable(Float64) | 255,000 | 零值 24,192；负值 130 | min=-1,834,260,000, max=8,170,360,000, distinct 722 | 提取保险合同准备金 |
| NET_CONTRACT_RESERVE_QOQ | Nullable(Float64) | 279,223 | 零值 1；负值 338 | min=-77,383.9, max=31,930, distinct 682 | 提取保险合同准备金环比增长率（%） |
| POLICY_BONUS_EXPENSE | Nullable(Float64) | 278,028 | 零值 1,833；负值 0 | min=0, max=435,475,000, distinct 58 | 保单红利支出 |
| POLICY_BONUS_EXPENSE_QOQ | Nullable(Float64) | 279,865 | 零值 0；负值 29 | min=-74.7965, max=471.604, distinct 53 | 保单红利支出环比增长率（%） |
| REINSURE_EXPENSE | Nullable(Float64) | 277,983 | 零值 1,856；负值 12 | min=-52,760,800, max=141,953,000, distinct 80 | 分保费用支出 |
| REINSURE_EXPENSE_QOQ | Nullable(Float64) | 279,844 | 零值 0；负值 39 | min=-1,322.32, max=147,522, distinct 74 | 分保费用支出环比增长率（%） |
| OTHER_BUSINESS_COST | Nullable(Float64) | 279,872 | 零值 5；负值 2 | min=-4,282,510, max=349,197,000, distinct 42 | 其他业务成本 |
| OTHER_BUSINESS_COST_QOQ | Nullable(Float64) | 279,888 | 零值 0；负值 16 | min=-818.565, max=35,431.1, distinct 30 | 其他业务成本环比增长率（%） |
| OPERATE_TAX_ADD | Nullable(Float64) | 2,623 | 零值 484；负值 2,231 | min=-1,364,580,000, max=93,024,000,000, distinct 276,366 | 营业税金及附加 |
| OPERATE_TAX_ADD_QOQ | Nullable(Float64) | 6,739 | 零值 48；负值 129,390 | min=-2,203,570,000, max=73,244,200,000, distinct 272,580 | 营业税金及附加环比增长率（%） |
| SALE_EXPENSE | Nullable(Float64) | 7,263 | 零值 466；负值 2,639 | min=-8,819,640,000, max=23,021,000,000, distinct 271,846 | 销售费用 |
| SALE_EXPENSE_QOQ | Nullable(Float64) | 11,245 | 零值 55；负值 121,232 | min=-52,389.7, max=2,908,100,000, distinct 268,256 | 销售费用环比增长率（%） |
| MANAGE_EXPENSE | Nullable(Float64) | 470 | 零值 5；负值 2,208 | min=-3,015,190,000, max=24,497,000,000, distinct 279,170 | 管理费用 |
| MANAGE_EXPENSE_QOQ | Nullable(Float64) | 3,811 | 零值 3；负值 124,579 | min=-23,564.2, max=8,321,940, distinct 275,878 | 管理费用环比增长率（%） |
| ME_RESEARCH_EXPENSE | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 管理费用中的研发费用 |
| ME_RESEARCH_EXPENSE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 管理费用中的研发费用环比增长率（%） |
| FINANCE_EXPENSE | Nullable(Float64) | 518 | 零值 13；负值 81,996 | min=-4,219,730,000, max=8,810,960,000, distinct 279,102 | 财务费用 |
| FINANCE_EXPENSE_QOQ | Nullable(Float64) | 3,882 | 零值 4；负值 132,272 | min=-25,720,300, max=13,948,400, distinct 275,801 | 财务费用环比增长率（%） |
| FE_INTEREST_EXPENSE | Nullable(Float64) | 145,907 | 零值 1,376；负值 3,032 | min=-37,844,000,000, max=53,056,000,000, distinct 132,161 | 财务费用之利息费用 |
| FE_INTEREST_EXPENSE_QOQ | Nullable(Float64) | 152,990 | 零值 246；负值 64,221 | min=-339,073, max=2,619,440,000, distinct 125,876 | 财务费用之利息费用环比增长率（%） |
| FE_INTEREST_INCOME | Nullable(Float64) | 134,192 | 零值 164；负值 10,100 | min=-13,726,000,000, max=18,958,000,000, distinct 145,454 | 财务费用之利息收入 |
| FE_INTEREST_INCOME_QOQ | Nullable(Float64) | 139,329 | 零值 6；负值 71,696 | min=-166,619, max=26,093,600, distinct 140,386 | 财务费用之利息收入环比增长率（%） |
| ASSET_IMPAIRMENT_LOSS | Nullable(Float64) | 172,403 | 零值 5,871；负值 29,352 | min=-4,127,000,000, max=34,427,000,000, distinct 100,981 | 资产减值损失 |
| ASSET_IMPAIRMENT_LOSS_QOQ | Nullable(Float64) | 186,650 | 零值 7；负值 45,082 | min=-55,308,100,000, max=1,727,230,000,000, distinct 91,721 | 资产减值损失环比增长率（%） |
| CREDIT_IMPAIRMENT_LOSS | Nullable(Float64) | 278,677 | 零值 47；负值 390 | min=-9,857,000,000, max=7,109,000,000, distinct 1,192 | 信用减值损失 |
| CREDIT_IMPAIRMENT_LOSS_QOQ | Nullable(Float64) | 279,704 | 零值 0；负值 110 | min=-642,954, max=31,892.3, distinct 214 | 信用减值损失环比增长率（%） |
| OTHER_INCOME | Nullable(Float64) | 128,603 | 零值 1,279；负值 2,384 | min=-4,444,380,000, max=17,545,600,000, distinct 148,235 | 其他收益 |
| OTHER_INCOME_QOQ | Nullable(Float64) | 134,438 | 零值 547；负值 72,301 | min=-33,216,300, max=47,185,600,000, distinct 144,201 | 其他收益环比增长率（%） |
| TOC_OTHER | Nullable(Float64) | 256,945 | 零值 22,778；负值 19 | min=-1,980,000,000, max=7,931,000,000, distinct 195 | 营业总成本其他 |
| TOC_OTHER_QOQ | Nullable(Float64) | 279,758 | 零值 0；负值 88 | min=-562.916, max=69,550, distinct 149 | 营业总成本其他环比增长率（%） |
| INVEST_INCOME | Nullable(Float64) | 53,124 | 零值 9,036；负值 59,367 | min=-14,086,600,000, max=67,303,900,000, distinct 215,283 | 投资收益 |
| INVEST_INCOME_QOQ | Nullable(Float64) | 71,646 | 零值 593；负值 103,802 | min=-364,483,000,000, max=233,489,000,000, distinct 203,632 | 投资收益环比增长率（%） |
| INVEST_JOINT_INCOME | Nullable(Float64) | 155,266 | 零值 3,826；负值 51,197 | min=-13,842,500,000, max=7,876,070,000, distinct 120,329 | 对联营企业和合营企业的投资收益 |
| INVEST_JOINT_INCOME_QOQ | Nullable(Float64) | 166,599 | 零值 51；负值 55,610 | min=-11,934,900,000, max=408,325,000,000, distinct 111,909 | 对联营企业和合营企业的投资收益环比增长率（%） |
| ACF_END_INCOME | Nullable(Float64) | 274,643 | 零值 1,706；负值 2,788 | min=-2,544,500,000, max=4,486,970,000, distinct 3,557 | 持续经营终止经营净损益 |
| ACF_END_INCOME_QOQ | Nullable(Float64) | 277,030 | 零值 1；负值 1,385 | min=-20,464,500, max=408,820, distinct 2,723 | 持续经营终止经营净损益环比增长率（%） |
| EXCHANGE_INCOME | Nullable(Float64) | 275,994 | 零值 2,267；负值 825 | min=-246,751,000, max=211,612,000, distinct 1,653 | 汇兑收益 |
| EXCHANGE_INCOME_QOQ | Nullable(Float64) | 278,606 | 零值 0；负值 681 | min=-6,081,640,000, max=196,330, distinct 1,297 | 汇兑收益环比增长率（%） |
| NET_EXPOSURE_INCOME | Nullable(Float64) | 278,330 | 零值 1,462；负值 63 | min=-677,665,000, max=202,171,000, distinct 127 | 净敞口收益 |
| NET_EXPOSURE_INCOME_QOQ | Nullable(Float64) | 279,817 | 零值 0；负值 50 | min=-3,078.46, max=142,316, distinct 99 | 净敞口收益环比增长率（%） |
| FAIRVALUE_CHANGE_INCOME | Nullable(Float64) | 187,143 | 零值 6,715；负值 36,456 | min=-22,274,700,000, max=11,923,200,000, distinct 84,978 | 公允价值变动收益 |
| FAIRVALUE_CHANGE_INCOME_QOQ | Nullable(Float64) | 200,740 | 零值 65；负值 39,451 | min=-14,032,900,000, max=102,352,000,000, distinct 75,624 | 公允价值变动收益环比增长率（%） |
| ASSET_DISPOSAL_INCOME | Nullable(Float64) | 170,562 | 零值 11,647；负值 41,809 | min=-2,553,150,000, max=8,934,740,000, distinct 96,808 | 资产处置收益 |
| ASSET_DISPOSAL_INCOME_QOQ | Nullable(Float64) | 193,189 | 零值 9；负值 44,058 | min=-108,509,000,000, max=372,447,000,000, distinct 80,561 | 资产处置收益环比增长率（%） |
| CREDIT_IMPAIRMENT_INCOME | Nullable(Float64) | 154,535 | 零值 1,314；负值 80,347 | min=-39,713,900,000, max=7,214,350,000, distinct 123,844 | 信用减值收益 |
| CREDIT_IMPAIRMENT_INCOME_QOQ | Nullable(Float64) | 161,867 | 零值 6；负值 60,819 | min=-423,564,000,000, max=13,211,300,000, distinct 117,591 | 信用减值收益环比增长率（%） |
| ASSET_IMPAIRMENT_INCOME | Nullable(Float64) | 180,139 | 零值 4,361；负值 73,430 | min=-36,654,000,000, max=1,465,820,000, distinct 95,157 | 资产减值收益 |
| ASSET_IMPAIRMENT_INCOME_QOQ | Nullable(Float64) | 194,642 | 零值 28；负值 44,098 | min=-18,231,400,000,000, max=7,587,380,000, distinct 83,552 | 资产减值收益环比增长率（%） |
| OPERATE_PROFIT | Nullable(Float64) | 166 | 零值 6；负值 60,486 | min=-51,232,000,000, max=70,371,000,000, distinct 279,513 | 营业利润 |
| OPERATE_PROFIT_QOQ | Nullable(Float64) | 3,117 | 零值 0；负值 136,299 | min=-19,177,700, max=10,034,400, distinct 276,574 | 营业利润环比增长率（%） |
| NONBUSINESS_INCOME | Nullable(Float64) | 12,581 | 零值 4,702；负值 15,758 | min=-2,443,630,000, max=27,387,000,000, distinct 253,234 | 营业外收入 |
| NONBUSINESS_INCOME_QOQ | Nullable(Float64) | 23,627 | 零值 148；负值 124,930 | min=-30,006,800,000, max=139,930,000,000, distinct 252,463 | 营业外收入环比增长率（%） |
| NONCURRENT_DISPOSAL_INCOME | Nullable(Float64) | 261,133 | 零值 2,892；负值 1,253 | min=-695,362,000, max=9,593,500,000, distinct 15,534 | 非流动资产处置净收益 |
| NONCURRENT_DISPOSAL_INCOME_QOQ | Nullable(Float64) | 267,207 | 零值 2；负值 6,756 | min=-196,117,000, max=33,995,200,000, distinct 11,417 | 非流动资产处置净收益环比增长率（%） |
| NONBUSINESS_EXPENSE | Nullable(Float64) | 14,768 | 零值 6,270；负值 10,219 | min=-12,317,800,000, max=21,402,000,000, distinct 250,000 | 营业外支出 |
| NONBUSINESS_EXPENSE_QOQ | Nullable(Float64) | 28,919 | 零值 65；负值 122,062 | min=-1,084,260,000, max=171,629,000,000, distinct 245,981 | 营业外支出环比增长率（%） |
| NONCURRENT_DISPOSAL_LOSS | Nullable(Float64) | 230,227 | 零值 5,446；负值 2,624 | min=-1,182,050,000, max=2,006,850,000, distinct 43,687 | 非流动资产处置净损失 |
| NONCURRENT_DISPOSAL_LOSS_QOQ | Nullable(Float64) | 242,532 | 零值 3；负值 18,935 | min=-8,650,040, max=10,821,500,000, distinct 34,879 | 非流动资产处置净损失环比增长率（%） |
| OPERATE_PROFIT_OTHER | Nullable(Float64) | 279,343 | 零值 519；负值 17 | min=-2,454,000,000, max=22,770,300, distinct 57 | 营业利润其他 |
| OPERATE_PROFIT_OTHER_QOQ | Nullable(Float64) | 279,899 | 零值 0；负值 10 | min=-489.665, max=3,190.23, distinct 19 | 营业利润其他环比增长率（%） |
| OPERATE_PROFIT_BALANCE | Nullable(Float64) | 219,758 | 零值 55,396；负值 714 | min=-3,612,400,000, max=200,000,000, distinct 4,598 | 营业利润平衡项 |
| OPERATE_PROFIT_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 营业利润平衡项环比增长率（%） |
| TOTAL_PROFIT | Nullable(Float64) | 39 | 零值 4；负值 55,961 | min=-51,143,900,000, max=68,939,000,000, distinct 279,643 | 利润总额 |
| TOTAL_PROFIT_QOQ | Nullable(Float64) | 2,968 | 零值 0；负值 136,235 | min=-30,705,900, max=4,257,840,000, distinct 276,725 | 利润总额环比增长率（%） |
| EFFECT_TP_OTHER | Nullable(Float64) | 257,410 | 零值 22,450；负值 5 | min=-2,723,460, max=23,380,500, distinct 59 | 影响利润总额其他 |
| EFFECT_TP_OTHER_QOQ | Nullable(Float64) | 279,909 | 零值 0；负值 5 | min=-100, max=1,119.09, distinct 7 | 影响利润总额其他环比增长率（%） |
| TOTAL_PROFIT_BALANCE | Nullable(Float64) | 219,758 | 零值 58,412；负值 111 | min=-60,833,700, max=510,223,000, distinct 1,586 | 利润总额平衡项 |
| TOTAL_PROFIT_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 利润总额平衡项环比增长率（%） |
| INCOME_TAX | Nullable(Float64) | 11,593 | 零值 1,503；负值 43,791 | min=-7,529,000,000, max=17,474,000,000, distinct 266,393 | 所得税费用 |
| INCOME_TAX_QOQ | Nullable(Float64) | 18,294 | 零值 44；负值 128,374 | min=-21,887,500,000, max=304,518,000,000, distinct 260,600 | 所得税费用环比增长率（%） |
| NETPROFIT | Nullable(Float64) | 38 | 零值 3；负值 57,197 | min=-63,573,400,000, max=53,643,000,000, distinct 279,635 | 净利润 |
| NETPROFIT_QOQ | Nullable(Float64) | 3,284 | 零值 0；负值 136,277 | min=-14,944,200, max=3,718,570,000, distinct 276,409 | 净利润环比增长率（%） |
| CONTINUED_NETPROFIT | Nullable(Float64) | 123,136 | 零值 9；负值 35,789 | min=-63,573,400,000, max=53,643,000,000, distinct 156,721 | 持续经营净利润 |
| CONTINUED_NETPROFIT_QOQ | Nullable(Float64) | 128,171 | 零值 0；负值 75,447 | min=-2,740,000, max=3,718,570,000, distinct 151,696 | 持续经营净利润环比增长率（%） |
| DISCONTINUED_NETPROFIT | Nullable(Float64) | 275,335 | 零值 2,230；负值 1,422 | min=-3,526,220,000, max=9,471,890,000, distinct 2,346 | 终止经营净利润 |
| DISCONTINUED_NETPROFIT_QOQ | Nullable(Float64) | 278,178 | 零值 3；负值 844 | min=-66,190,700, max=52,238,500, distinct 1,548 | 终止经营净利润环比增长率（%） |
| NETPROFIT_OTHER | Nullable(Float64) | 279,742 | 零值 31；负值 31 | min=-101,840,000, max=257,156,000, distinct 138 | 净利润其他 |
| NETPROFIT_OTHER_QOQ | Nullable(Float64) | 279,806 | 零值 2；负值 59 | min=-9,323.27, max=4,450.9, distinct 104 | 净利润其他环比增长率（%） |
| NETPROFIT_BALANCE | Nullable(Float64) | 279,913 | 零值 2；负值 0 | min=0, max=162,569, distinct 4 | 净利润平衡项 |
| NETPROFIT_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 净利润平衡项环比增长率（%） |
| EFFECT_NETPROFIT_OTHER | Nullable(Float64) | 279,893 | 零值 14；负值 2 | min=-2,163,420, max=26,344,300,000, distinct 12 | 影响净利润其他 |
| EFFECT_NETPROFIT_OTHER_QOQ | Nullable(Float64) | 279,916 | 零值 0；负值 0 | min=105.835, max=2,622.04, distinct 2 | 影响净利润其他环比增长率（%） |
| EFFECT_NETPROFIT_BALANCE | Nullable(Float64) | 274,915 | 零值 4,723；负值 144 | min=-68,279,500, max=83,354,200, distinct 196 | 净利润平衡项 |
| EFFECT_NETPROFIT_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 净利润平衡项环比增长率（%） |
| UNCONFIRM_INVEST_LOSS | Nullable(Float64) | 277,625 | 零值 120；负值 500 | min=-192,824,000, max=440,000,000, distinct 2,162 | 未确认投资损失 |
| UNCONFIRM_INVEST_LOSS_QOQ | Nullable(Float64) | 278,101 | 零值 8；负值 881 | min=-2,208,100, max=787,897, distinct 1,784 | 未确认投资损失环比增长率（%） |
| MINORITY_INTEREST | Nullable(Float64) | 57,033 | 零值 1,841；负值 93,699 | min=-16,341,400,000, max=13,529,000,000, distinct 220,495 | 少数股东损益 |
| MINORITY_INTEREST_QOQ | Nullable(Float64) | 63,566 | 零值 49；负值 106,634 | min=-195,968,000,000, max=77,681,500,000, distinct 215,328 | 少数股东损益环比增长率（%） |
| PARENT_NETPROFIT | Nullable(Float64) | 3 | 零值 3；负值 56,699 | min=-60,540,600,000, max=53,604,000,000, distinct 279,682 | 归属于母公司股东的净利润 |
| PARENT_NETPROFIT_QOQ | Nullable(Float64) | 2,883 | 零值 0；负值 136,516 | min=-11,225,600,000, max=6,598,360, distinct 276,813 | 归属于母公司股东的净利润环比增长率（%） |
| BASIC_EPS | Nullable(Float64) | 21,115 | 零值 2,780；负值 54,214 | min=-19.047, max=31.392, distinct 12,771 | 基本每股收益（元/股） |
| BASIC_EPS_QOQ | Nullable(Float64) | 27,660 | 零值 6,553；负值 123,236 | min=-2,349,800, max=1,850,900, distinct 103,340 | 基本每股收益（元/股）环比增长率（%） |
| DILUTED_EPS | Nullable(Float64) | 27,336 | 零值 2,794；负值 53,012 | min=-19.047, max=30.4452, distinct 12,682 | 稀释每股收益（元/股） |
| DILUTED_EPS_QOQ | Nullable(Float64) | 34,364 | 零值 6,429；负值 119,987 | min=-2,349,800, max=1,850,900, distinct 101,089 | 稀释每股收益（元/股）环比增长率（%） |
| UNABLE_OCI | Nullable(Float64) | 254,175 | 零值 3,512；负值 11,867 | min=-14,454,600,000, max=8,699,550,000, distinct 22,016 | 以后将重分类进损益的其他综合收益 |
| UNABLE_OCI_QOQ | Nullable(Float64) | 260,526 | 零值 21；负值 9,516 | min=-614,618,000,000, max=518,656,000,000, distinct 18,578 | 以后将重分类进损益的其他综合收益环比增长率（%） |
| CREDITRISK_FAIRVALUE_CHANGE | Nullable(Float64) | 278,886 | 零值 1,026；负值 3 | min=-26,144,800, max=14,026,000, distinct 7 | 信用风险引起的公允价值变动 |
| CREDITRISK_FAIRVALUE_CHANGE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 信用风险引起的公允价值变动环比增长率（%） |
| OTHERRIGHT_FAIRVALUE_CHANGE | Nullable(Float64) | 258,052 | 零值 2,195；负值 10,529 | min=-14,454,600,000, max=8,699,550,000, distinct 19,457 | 其他权益工具公允价值变动 |
| OTHERRIGHT_FAIRVALUE_CHANGE_QOQ | Nullable(Float64) | 262,601 | 零值 19；负值 8,490 | min=-631,829,000,000, max=763,467,000,000, distinct 16,654 | 其他权益工具公允价值变动环比增长率（%） |
| SETUP_PROFIT_CHANGE | Nullable(Float64) | 276,390 | 零值 1,658；负值 969 | min=-1,143,750,000, max=848,917,000, distinct 1,851 | 重分类调整变动 |
| SETUP_PROFIT_CHANGE_QOQ | Nullable(Float64) | 278,558 | 零值 6；负值 683 | min=-100,968,000, max=1,536,840,000, distinct 1,250 | 重分类调整变动环比增长率（%） |
| RIGHTLAW_UNABLE_OCI | Nullable(Float64) | 276,119 | 零值 1,729；负值 1,029 | min=-4,703,150,000, max=8,699,550,000, distinct 2,043 | 权益法下不能重分类的其他综合收益 |
| RIGHTLAW_UNABLE_OCI_QOQ | Nullable(Float64) | 278,399 | 零值 2；负值 745 | min=-7,012,910,000, max=6,945,750, distinct 1,404 | 权益法下不能重分类的其他综合收益环比增长率（%） |
| UNABLE_OCI_OTHER | Nullable(Float64) | 278,882 | 零值 826；负值 111 | min=-1,332,200,000, max=1,320,160,000, distinct 211 | 不能重分类其他综合收益其他 |
| UNABLE_OCI_OTHER_QOQ | Nullable(Float64) | 279,769 | 零值 0；负值 73 | min=-2,569.45, max=42,738.2, distinct 140 | 不能重分类其他综合收益其他环比增长率（%） |
| UNABLE_OCI_BALANCE | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 不能重分类其他综合收益平衡项 |
| UNABLE_OCI_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 不能重分类其他综合收益平衡项环比增长率（%） |
| ABLE_OCI | Nullable(Float64) | 180,577 | 零值 1,857；负值 52,168 | min=-13,915,000,000, max=13,546,000,000, distinct 97,276 | 以后将重分类进损益的其他综合收益（可重分类） |
| ABLE_OCI_QOQ | Nullable(Float64) | 186,406 | 零值 11；负值 47,710 | min=-101,796,000,000, max=8,770,500,000, distinct 92,676 | 以后将重分类进损益的其他综合收益（可重分类）环比增长率（%） |
| RIGHTLAW_ABLE_OCI | Nullable(Float64) | 266,289 | 零值 2,228；负值 5,818 | min=-4,217,610,000, max=4,421,000,000, distinct 11,325 | 权益法下可重分类的其他综合收益 |
| RIGHTLAW_ABLE_OCI_QOQ | Nullable(Float64) | 270,090 | 零值 3；负值 4,882 | min=-680,471,000, max=1,569,880,000, distinct 9,429 | 权益法下可重分类的其他综合收益环比增长率（%） |
| AFA_FAIRVALUE_CHANGE | Nullable(Float64) | 269,189 | 零值 589；负值 5,527 | min=-8,769,480,000, max=5,963,790,000, distinct 10,046 | 可供出售金融资产公允价值变动 |
| AFA_FAIRVALUE_CHANGE_QOQ | Nullable(Float64) | 270,737 | 零值 7；负值 4,345 | min=-1,569,230, max=195,615,000, distinct 8,713 | 可供出售金融资产公允价值变动环比增长率（%） |
| HMI_AFA | Nullable(Float64) | 279,611 | 零值 294；负值 8 | min=-11,881,600, max=75,583,600, distinct 14 | 持有有待售资产公允价值变动 |
| HMI_AFA_QOQ | Nullable(Float64) | 279,917 | 零值 0；负值 0 | min=100, max=100, distinct 1 | 持有有待售资产公允价值变动环比增长率（%） |
| CASHFLOW_HEDGE_VALID | Nullable(Float64) | 272,636 | 零值 1,490；负值 2,721 | min=-11,545,000,000, max=7,312,000,000, distinct 5,770 | 现金流量套期有效部分 |
| CASHFLOW_HEDGE_VALID_QOQ | Nullable(Float64) | 274,640 | 零值 1；负值 2,674 | min=-1,829,200, max=386,323, distinct 5,121 | 现金流量套期有效部分环比增长率（%） |
| CREDITOR_FAIRVALUE_CHANGE | Nullable(Float64) | 276,626 | 零值 1,117；负值 988 | min=-4,577,870,000, max=1,034,860,000, distinct 2,172 | 债权投资公允价值变动 |
| CREDITOR_FAIRVALUE_CHANGE_QOQ | Nullable(Float64) | 277,978 | 零值 1；负值 986 | min=-334,295, max=128,509, distinct 1,898 | 债权投资公允价值变动环比增长率（%） |
| CREDITOR_IMPAIRMENT_RESERVE | Nullable(Float64) | 277,887 | 零值 1,061；负值 493 | min=-174,008,000, max=210,874,000, distinct 969 | 债权投资减值准备 |
| CREDITOR_IMPAIRMENT_RESERVE_QOQ | Nullable(Float64) | 279,075 | 零值 0；负值 418 | min=-63,697.5, max=14,634,900, distinct 821 | 债权投资减值准备环比增长率（%） |
| FINANCE_OCI_AMT | Nullable(Float64) | 278,309 | 零值 1,092；负值 267 | min=-138,336,000, max=1,122,610,000, distinct 517 | 金融资产重分类金额 |
| FINANCE_OCI_AMT_QOQ | Nullable(Float64) | 279,562 | 零值 0；负值 191 | min=-28,151.8, max=13,296,800, distinct 348 | 金融资产重分类金额环比增长率（%） |
| CONVERT_DIFF | Nullable(Float64) | 191,726 | 零值 1,196；负值 46,955 | min=-7,751,000,000, max=11,482,000,000, distinct 86,872 | 外币报表折算差额 |
| CONVERT_DIFF_QOQ | Nullable(Float64) | 196,182 | 零值 4；负值 43,396 | min=-101,796,000,000, max=30,147,100,000, distinct 83,321 | 外币报表折算差额环比增长率（%） |
| ABLE_OCI_OTHER | Nullable(Float64) | 276,876 | 零值 1,158；负值 914 | min=-7,680,000,000, max=2,059,120,000, distinct 1,850 | 可重分类其他综合收益其他 |
| ABLE_OCI_OTHER_QOQ | Nullable(Float64) | 278,520 | 零值 18；负值 728 | min=-95,678,500,000, max=8,770,500,000, distinct 1,294 | 可重分类其他综合收益其他环比增长率（%） |
| ABLE_OCI_BALANCE | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 可重分类其他综合收益平衡项 |
| ABLE_OCI_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 可重分类其他综合收益平衡项环比增长率（%） |
| OCI_OTHER | Nullable(Float64) | 279,907 | 零值 9；负值 0 | min=0, max=343,721,000, distinct 3 | 其他综合收益其他 |
| OCI_OTHER_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他综合收益其他环比增长率（%） |
| OCI_BALANCE | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他综合收益平衡项 |
| OCI_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他综合收益平衡项环比增长率（%） |
| OTHER_COMPRE_INCOME | Nullable(Float64) | 156,895 | 零值 3,701；负值 63,988 | min=-14,697,700,000, max=14,207,000,000, distinct 118,838 | 其他综合收益总额 |
| OTHER_COMPRE_INCOME_QOQ | Nullable(Float64) | 165,597 | 零值 28；负值 57,941 | min=-487,680,000,000, max=51,000,000,000, distinct 112,635 | 其他综合收益总额环比增长率（%） |
| PARENT_OCI | Nullable(Float64) | 172,785 | 零值 2,377；负值 55,804 | min=-14,793,800,000, max=13,533,000,000, distinct 104,473 | 归属于母公司股东的其他综合收益 |
| PARENT_OCI_QOQ | Nullable(Float64) | 179,715 | 零值 19；负值 50,681 | min=-379,428,000,000, max=15,686,700,000, distinct 99,137 | 归母其他综合收益环比增长率（%） |
| MINORITY_OCI | Nullable(Float64) | 243,820 | 零值 2,169；负值 17,782 | min=-4,223,000,000, max=4,828,000,000, distinct 33,745 | 归属于少数股东的其他综合收益 |
| MINORITY_OCI_QOQ | Nullable(Float64) | 248,287 | 零值 5；负值 15,990 | min=-19,305,800,000, max=63,630,100,000, distinct 31,102 | 少数股东其他综合收益环比增长率（%） |
| PARENT_OCI_OTHER | Nullable(Float64) | 279,915 | 零值 2；负值 0 | min=0, max=1,399,000,000, distinct 2 | 归母其他综合收益其他 |
| PARENT_OCI_OTHER_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 归母其他综合收益其他环比增长率（%） |
| PARENT_OCI_BALANCE | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 归母其他综合收益平衡项 |
| PARENT_OCI_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 归母其他综合收益平衡项环比增长率（%） |
| TOTAL_COMPRE_INCOME | Nullable(Float64) | 42,942 | 零值 12；负值 51,076 | min=-63,071,200,000, max=60,297,000,000, distinct 236,811 | 综合收益总额 |
| TOTAL_COMPRE_INCOME_QOQ | Nullable(Float64) | 46,936 | 零值 0；负值 114,996 | min=-120,104,000,000, max=95,721,000,000, distinct 232,831 | 综合收益总额环比增长率（%） |
| PARENT_TCI | Nullable(Float64) | 44,389 | 零值 12；负值 50,714 | min=-58,908,400,000, max=53,093,000,000, distinct 235,363 | 归属于母公司股东的综合收益总额 |
| PARENT_TCI_QOQ | Nullable(Float64) | 48,721 | 零值 1；负值 113,950 | min=-120,138,000,000, max=8,195,780, distinct 231,044 | 归母综合收益总额环比增长率（%） |
| MINORITY_TCI | Nullable(Float64) | 92,394 | 零值 1,755；负值 81,497 | min=-16,435,300,000, max=15,291,000,000, distinct 185,366 | 归属于少数股东的综合收益总额 |
| MINORITY_TCI_QOQ | Nullable(Float64) | 99,067 | 零值 44；负值 89,309 | min=-195,968,000,000, max=77,681,500,000, distinct 179,972 | 少数股东综合收益总额环比增长率（%） |
| EFFECT_TCI_BALANCE | Nullable(Float64) | 270,131 | 零值 9,772；负值 6 | min=-65,287,100, max=7,090,080, distinct 15 | 综合收益总额平衡项 |
| EFFECT_TCI_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 综合收益总额平衡项环比增长率（%） |
| TCI_OTHER | Nullable(Float64) | 279,822 | 零值 43；负值 0 | min=0, max=190,587,000, distinct 49 | 综合收益总额其他 |
| TCI_OTHER_QOQ | Nullable(Float64) | 279,870 | 零值 1；负值 25 | min=-100, max=1,135.17, distinct 47 | 综合收益总额其他环比增长率（%） |
| TCI_BALANCE | Nullable(Float64) | 270,137 | 零值 9,774；负值 2 | min=-18, max=6,499,130, distinct 8 | 综合收益总额平衡项 |
| TCI_BALANCE_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 综合收益总额平衡项环比增长率（%） |
| PRECOMBINE_PROFIT | Nullable(Float64) | 278,881 | 零值 687；负值 147 | min=-783,000,000, max=1,873,870,000, distinct 351 | 合并前净损益 |
| PRECOMBINE_PROFIT_QOQ | Nullable(Float64) | 279,733 | 零值 0；负值 101 | min=-9,446.84, max=113,997, distinct 164 | 合并前净损益环比增长率（%） |
| PRECOMBINE_TCI | Nullable(Float64) | 279,878 | 零值 40；负值 0 | min=0, max=0, distinct 1 | 合并前综合收益总额 |
| PRECOMBINE_TCI_QOQ | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 合并前综合收益总额环比增长率（%） |
| DEDUCT_PARENT_NETPROFIT | Nullable(Float64) | 9,960 | 零值 19；负值 69,827 | min=-59,431,200,000, max=78,026,000,000, distinct 269,668 | 扣除非经常性损益后归属于母公司股东的净利润 |
| DEDUCT_PARENT_NETPROFIT_QOQ | Nullable(Float64) | 14,408 | 零值 0；负值 131,193 | min=-13,312,900, max=242,711,000,000, distinct 265,276 | 扣非归母净利润环比增长率（%） |
| TOTAL_OPERATE_INCOME_YOY | Nullable(Float64) | 14,751 | 零值 14；负值 95,852 | min=-70,118, max=85,891,000,000, distinct 264,771 | 营业总收入同比增长率（%） |
| OPERATE_INCOME_YOY | Nullable(Float64) | 14,921 | 零值 15；负值 95,857 | min=-70,118, max=85,891,000,000, distinct 264,627 | 营业收入同比增长率（%） |
| INTEREST_INCOME_YOY | Nullable(Float64) | 275,696 | 零值 6；负值 2,090 | min=-6,663.27, max=220,761, distinct 4,151 | 利息收入同比增长率（%） |
| EARNED_PREMIUM_YOY | Nullable(Float64) | 279,500 | 零值 0；负值 162 | min=-936.365, max=11,502.9, distinct 415 | 已赚保费同比增长率（%） |
| FEE_COMMISSION_INCOME_YOY | Nullable(Float64) | 277,794 | 零值 0；负值 1,021 | min=-17,275.8, max=195,222, distinct 2,081 | 手续费及佣金收入同比增长率（%） |
| OTHER_BUSINESS_INCOME_YOY | Nullable(Float64) | 279,829 | 零值 0；负值 40 | min=-17,469.6, max=74,209.7, distinct 88 | 其他业务收入同比增长率（%） |
| TOI_OTHER_YOY | Nullable(Float64) | 279,822 | 零值 0；负值 44 | min=-703.676, max=59,439, distinct 96 | 营业总收入其他同比增长率（%） |
| TOTAL_OPERATE_COST_YOY | Nullable(Float64) | 14,212 | 零值 0；负值 92,608 | min=-142,663, max=1,066,180, distinct 265,492 | 营业总成本同比增长率（%） |
| OPERATE_COST_YOY | Nullable(Float64) | 15,145 | 零值 24；负值 95,556 | min=-47,056.4, max=49,811,600, distinct 264,399 | 营业成本同比增长率（%） |
| INTEREST_EXPENSE_YOY | Nullable(Float64) | 277,746 | 零值 0；负值 1,081 | min=-1,262.26, max=1,124,640, distinct 2,143 | 利息支出同比增长率（%） |
| FEE_COMMISSION_EXPENSE_YOY | Nullable(Float64) | 277,898 | 零值 0；负值 926 | min=-14,598.7, max=14,718,000, distinct 1,985 | 手续费及佣金支出同比增长率（%） |
| RESEARCH_EXPENSE_YOY | Nullable(Float64) | 157,871 | 零值 23；负值 47,816 | min=-78,284.1, max=42,556,800,000, distinct 121,746 | 研发费用同比增长率（%） |
| SURRENDER_VALUE_YOY | Nullable(Float64) | 279,873 | 零值 0；负值 12 | min=-6,065.55, max=1,077.07, distinct 45 | 退保金同比增长率（%） |
| NET_COMPENSATE_EXPENSE_YOY | Nullable(Float64) | 279,798 | 零值 0；负值 39 | min=-100, max=163,270, distinct 118 | 分保费用同比增长率（%） |
| NET_CONTRACT_RESERVE_YOY | Nullable(Float64) | 279,288 | 零值 1；负值 280 | min=-34,153.9, max=14,074.3, distinct 615 | 提取保险合同准备金同比增长率（%） |
| POLICY_BONUS_EXPENSE_YOY | Nullable(Float64) | 279,874 | 零值 0；负值 13 | min=-57.2543, max=782.187, distinct 44 | 保单红利支出同比增长率（%） |
| REINSURE_EXPENSE_YOY | Nullable(Float64) | 279,853 | 零值 0；负值 28 | min=-359.934, max=47,630.2, distinct 65 | 分保费用支出同比增长率（%） |
| OTHER_BUSINESS_COST_YOY | Nullable(Float64) | 279,893 | 零值 0；负值 13 | min=-96.6232, max=54,905.1, distinct 25 | 其他业务成本同比增长率（%） |
| OPERATE_TAX_ADD_YOY | Nullable(Float64) | 17,561 | 零值 22；负值 109,036 | min=-20,559,100, max=27,895,000,000, distinct 261,814 | 营业税金及附加同比增长率（%） |
| SALE_EXPENSE_YOY | Nullable(Float64) | 22,012 | 零值 24；负值 103,039 | min=-491,851, max=69,989,400, distinct 257,459 | 销售费用同比增长率（%） |
| MANAGE_EXPENSE_YOY | Nullable(Float64) | 14,315 | 零值 0；负值 99,977 | min=-11,939.8, max=11,554,700, distinct 265,392 | 管理费用同比增长率（%） |
| ME_RESEARCH_EXPENSE_YOY | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 管理费用中的研发费用同比增长率（%） |
| FINANCE_EXPENSE_YOY | Nullable(Float64) | 14,395 | 零值 2；负值 124,266 | min=-20,603,800, max=30,936,300, distinct 265,305 | 财务费用同比增长率（%） |
| FE_INTEREST_EXPENSE_YOY | Nullable(Float64) | 165,179 | 零值 78；负值 57,602 | min=-1,812,200, max=10,356,700,000, distinct 113,868 | 财务费用之利息费用同比增长率（%） |
| FE_INTEREST_INCOME_YOY | Nullable(Float64) | 150,740 | 零值 11；负值 63,396 | min=-461,084, max=965,686,000, distinct 129,008 | 财务费用之利息收入同比增长率（%） |
| ASSET_IMPAIRMENT_LOSS_YOY | Nullable(Float64) | 191,989 | 零值 26；负值 39,787 | min=-8,313,960,000, max=37,793,200,000, distinct 86,420 | 资产减值损失同比增长率（%） |
| CREDIT_IMPAIRMENT_LOSS_YOY | Nullable(Float64) | 279,666 | 零值 1；负值 122 | min=-71,478.4, max=7,383.06, distinct 252 | 信用减值损失同比增长率（%） |
| OTHER_INCOME_YOY | Nullable(Float64) | 146,022 | 零值 311；负值 63,838 | min=-1,087,610,000, max=60,764,000,000, distinct 133,029 | 其他收益同比增长率（%） |
| TOC_OTHER_YOY | Nullable(Float64) | 279,777 | 零值 0；负值 76 | min=-239.46, max=265,645, distinct 130 | 营业总成本其他同比增长率（%） |
| INVEST_INCOME_YOY | Nullable(Float64) | 86,052 | 零值 464；负值 94,969 | min=-415,709,000,000, max=262,385,000,000, distinct 189,653 | 投资收益同比增长率（%） |
| INVEST_JOINT_INCOME_YOY | Nullable(Float64) | 174,491 | 零值 66；负值 51,064 | min=-721,261,000, max=201,895,000,000, distinct 104,018 | 对联营企业和合营企业的投资收益同比增长率（%） |
| ACF_END_INCOME_YOY | Nullable(Float64) | 277,624 | 零值 1；负值 1,072 | min=-340,819, max=59,228,500, distinct 2,156 | 持续经营终止经营净损益同比增长率（%） |
| EXCHANGE_INCOME_YOY | Nullable(Float64) | 278,646 | 零值 0；负值 656 | min=-69,756.8, max=4,336,980, distinct 1,261 | 汇兑收益同比增长率（%） |
| NET_EXPOSURE_INCOME_YOY | Nullable(Float64) | 279,839 | 零值 0；负值 40 | min=-110,584, max=6,944.82, distinct 74 | 净敞口收益同比增长率（%） |
| FAIRVALUE_CHANGE_INCOME_YOY | Nullable(Float64) | 212,467 | 零值 31；负值 33,264 | min=-2,417,380,000, max=6,312,440,000,000, distinct 64,627 | 公允价值变动收益同比增长率（%） |
| ASSET_DISPOSAL_INCOME_YOY | Nullable(Float64) | 205,252 | 零值 12；负值 37,284 | min=-14,931,900,000, max=472,185,000,000, distinct 70,055 | 资产处置收益同比增长率（%） |
| CREDIT_IMPAIRMENT_INCOME_YOY | Nullable(Float64) | 172,786 | 零值 3；负值 55,859 | min=-3,167,170,000, max=40,391,200,000, distinct 106,693 | 信用减值收益同比增长率（%） |
| ASSET_IMPAIRMENT_INCOME_YOY | Nullable(Float64) | 204,662 | 零值 19；负值 40,288 | min=-35,030,600,000, max=34,453,600,000, distinct 73,753 | 资产减值收益同比增长率（%） |
| OPERATE_PROFIT_YOY | Nullable(Float64) | 13,902 | 零值 0；负值 121,611 | min=-4,195,760, max=16,208,600, distinct 265,804 | 营业利润同比增长率（%） |
| NONBUSINESS_INCOME_YOY | Nullable(Float64) | 35,855 | 零值 104；负值 119,691 | min=-8,219,800,000, max=562,558,000,000, distinct 240,882 | 营业外收入同比增长率（%） |
| NONCURRENT_DISPOSAL_INCOME_YOY | Nullable(Float64) | 270,801 | 零值 7；负值 4,713 | min=-315,631,000, max=24,912,600,000, distinct 8,309 | 非流动资产处置净收益同比增长率（%） |
| NONBUSINESS_EXPENSE_YOY | Nullable(Float64) | 41,970 | 零值 108；负值 114,228 | min=-485,388,000,000, max=741,125,000,000, distinct 233,762 | 营业外支出同比增长率（%） |
| NONCURRENT_DISPOSAL_LOSS_YOY | Nullable(Float64) | 247,384 | 零值 26；负值 16,223 | min=-208,677,000, max=27,797,000,000, distinct 30,766 | 非流动资产处置净损失同比增长率（%） |
| OPERATE_PROFIT_OTHER_YOY | Nullable(Float64) | 279,901 | 零值 0；负值 11 | min=-11,592.6, max=15,336.1, distinct 17 | 营业利润其他同比增长率（%） |
| OPERATE_PROFIT_BALANCE_YOY | Nullable(Float64) | 276,413 | 零值 6；负值 1,520 | min=-622,398, max=2,000,000,000,000, distinct 3,221 | 营业利润平衡项同比增长率（%） |
| TOTAL_PROFIT_YOY | Nullable(Float64) | 13,782 | 零值 0；负值 121,137 | min=-30,158,100, max=2,139,920,000,000, distinct 265,925 | 利润总额同比增长率（%） |
| EFFECT_TP_OTHER_YOY | Nullable(Float64) | 279,896 | 零值 0；负值 14 | min=-5,429,420, max=936.08, distinct 18 | 影响利润总额其他同比增长率（%） |
| TOTAL_PROFIT_BALANCE_YOY | Nullable(Float64) | 278,600 | 零值 9；负值 818 | min=-58,357.7, max=6,212,010,000, distinct 756 | 利润总额平衡项同比增长率（%） |
| INCOME_TAX_YOY | Nullable(Float64) | 29,592 | 零值 40；负值 118,757 | min=-15,966,500,000, max=66,117,600,000, distinct 249,263 | 所得税费用同比增长率（%） |
| NETPROFIT_YOY | Nullable(Float64) | 13,859 | 零值 0；负值 120,558 | min=-5,618,230, max=2,089,850,000,000, distinct 265,848 | 净利润同比增长率（%） |
| CONTINUED_NETPROFIT_YOY | Nullable(Float64) | 139,404 | 零值 3；负值 67,273 | min=-2,612,500, max=2,089,850,000,000, distinct 140,471 | 持续经营净利润同比增长率（%） |
| DISCONTINUED_NETPROFIT_YOY | Nullable(Float64) | 278,658 | 零值 1；负值 541 | min=-1,484,700, max=436,832,000, distinct 1,052 | 终止经营净利润同比增长率（%） |
| NETPROFIT_OTHER_YOY | Nullable(Float64) | 279,831 | 零值 4；负值 45 | min=-6,435.73, max=4,148.71, distinct 77 | 净利润其他同比增长率（%） |
| NETPROFIT_BALANCE_YOY | Nullable(Float64) | 279,917 | 零值 0；负值 1 | min=-100, max=-100, distinct 1 | 净利润平衡项同比增长率（%） |
| EFFECT_NETPROFIT_OTHER_YOY | Nullable(Float64) | 279,915 | 零值 0；负值 0 | min=94.6548, max=4,234.15, distinct 3 | 影响净利润其他同比增长率（%） |
| EFFECT_NETPROFIT_BALANCE_YOY | Nullable(Float64) | 279,686 | 零值 0；负值 111 | min=-200, max=608.054, distinct 22 | 净利润平衡项同比增长率（%） |
| UNCONFIRM_INVEST_LOSS_YOY | Nullable(Float64) | 278,798 | 零值 5；负值 566 | min=-170,443, max=373,104, distinct 1,101 | 未确认投资损失同比增长率（%） |
| MINORITY_INTEREST_YOY | Nullable(Float64) | 77,029 | 零值 39；负值 98,909 | min=-128,431,000,000, max=102,429,000,000, distinct 201,654 | 少数股东损益同比增长率（%） |
| PARENT_NETPROFIT_YOY | Nullable(Float64) | 13,719 | 零值 2；负值 120,133 | min=-3,638,700, max=2,011,230,000,000, distinct 265,987 | 归属于母公司股东的净利润同比增长率（%） |
| BASIC_EPS_YOY | Nullable(Float64) | 38,169 | 零值 8,089；负值 120,207 | min=-1,054,600, max=322,400, distinct 98,680 | 基本每股收益（元/股）同比增长率（%） |
| DILUTED_EPS_YOY | Nullable(Float64) | 44,525 | 零值 7,917；负值 117,240 | min=-1,054,600, max=324,900, distinct 96,545 | 稀释每股收益（元/股）同比增长率（%） |
| UNABLE_OCI_YOY | Nullable(Float64) | 263,145 | 零值 19；负值 8,020 | min=-35,809,500,000, max=1,387,350,000,000, distinct 16,186 | 以后将重分类进损益的其他综合收益同比增长率（%） |
| CREDITRISK_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 279,914 | 零值 0；负值 2 | min=-164.138, max=187.972, distinct 4 | 信用风险引起的公允价值变动同比增长率（%） |
| OTHERRIGHT_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 264,970 | 零值 18；负值 7,135 | min=-35,809,500,000, max=1,387,350,000,000, distinct 14,444 | 其他权益工具公允价值变动同比增长率（%） |
| SETUP_PROFIT_CHANGE_YOY | Nullable(Float64) | 278,616 | 零值 1；负值 615 | min=-299,946, max=2,066,170, distinct 1,230 | 重分类调整变动同比增长率（%） |
| RIGHTLAW_UNABLE_OCI_YOY | Nullable(Float64) | 278,641 | 零值 3；负值 621 | min=-189,344, max=449,668, distinct 1,194 | 权益法下不能重分类的其他综合收益同比增长率（%） |
| UNABLE_OCI_OTHER_YOY | Nullable(Float64) | 279,790 | 零值 0；负值 64 | min=-177,204, max=10,053.3, distinct 118 | 不能重分类其他综合收益其他同比增长率（%） |
| UNABLE_OCI_BALANCE_YOY | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 不能重分类其他综合收益平衡项同比增长率（%） |
| ABLE_OCI_YOY | Nullable(Float64) | 194,667 | 零值 20；负值 44,410 | min=-19,266,700,000, max=11,694,000,000, distinct 84,508 | 以后将重分类进损益的其他综合收益（可重分类）同比增长率（%） |
| RIGHTLAW_ABLE_OCI_YOY | Nullable(Float64) | 271,260 | 零值 5；负值 4,419 | min=-28,834,800, max=20,175,000,000, distinct 8,322 | 权益法下可重分类的其他综合收益同比增长率（%） |
| AFA_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 272,752 | 零值 5；负值 3,788 | min=-226,877,000, max=8,755,600, distinct 6,838 | 可供出售金融资产公允价值变动同比增长率（%） |
| HMI_AFA_YOY | Nullable(Float64) | 279,912 | 零值 0；负值 3 | min=-518.084, max=430.373, distinct 6 | 持有有待售资产公允价值变动同比增长率（%） |
| CASHFLOW_HEDGE_VALID_YOY | Nullable(Float64) | 275,432 | 零值 2；负值 2,228 | min=-1,386,900, max=175,901, distinct 4,362 | 现金流量套期有效部分同比增长率（%） |
| CREDITOR_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 278,268 | 零值 0；负值 839 | min=-346,003, max=65,136.1, distinct 1,600 | 债权投资公允价值变动同比增长率（%） |
| CREDITOR_IMPAIRMENT_RESERVE_YOY | Nullable(Float64) | 279,239 | 零值 0；负值 333 | min=-903,939, max=4,798,400, distinct 663 | 债权投资减值准备同比增长率（%） |
| FINANCE_OCI_AMT_YOY | Nullable(Float64) | 279,605 | 零值 0；负值 154 | min=-20,264, max=1,521,140, distinct 305 | 金融资产重分类金额同比增长率（%） |
| CONVERT_DIFF_YOY | Nullable(Float64) | 203,230 | 零值 10；负值 40,337 | min=-120,417,000,000, max=132,777,000,000, distinct 76,287 | 外币报表折算差额同比增长率（%） |
| ABLE_OCI_OTHER_YOY | Nullable(Float64) | 278,777 | 零值 12；负值 579 | min=-24,767,000,000, max=11,694,000,000, distinct 1,075 | 可重分类其他综合收益其他同比增长率（%） |
| ABLE_OCI_BALANCE_YOY | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 可重分类其他综合收益平衡项同比增长率（%） |
| OCI_OTHER_YOY | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他综合收益其他同比增长率（%） |
| OCI_BALANCE_YOY | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他综合收益平衡项同比增长率（%） |
| OTHER_COMPRE_INCOME_YOY | Nullable(Float64) | 174,345 | 零值 41；负值 53,945 | min=-43,781,600,000, max=31,103,700,000, distinct 104,275 | 其他综合收益总额同比增长率（%） |
| PARENT_OCI_YOY | Nullable(Float64) | 188,321 | 零值 26；负值 47,185 | min=-125,145,000,000, max=11,694,000,000, distinct 90,722 | 归母其他综合收益同比增长率（%） |
| MINORITY_OCI_YOY | Nullable(Float64) | 251,935 | 零值 6；负值 14,301 | min=-13,328,400,000, max=14,092,400,000, distinct 27,520 | 少数股东其他综合收益同比增长率（%） |
| PARENT_OCI_OTHER_YOY | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 归母其他综合收益其他同比增长率（%） |
| PARENT_OCI_BALANCE_YOY | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 归母其他综合收益平衡项同比增长率（%） |
| TOTAL_COMPRE_INCOME_YOY | Nullable(Float64) | 57,001 | 零值 3；负值 102,987 | min=-6,618,670, max=2,087,250,000,000, distinct 222,779 | 综合收益总额同比增长率（%） |
| PARENT_TCI_YOY | Nullable(Float64) | 58,867 | 零值 4；负值 101,894 | min=-10,489,500,000, max=2,009,900,000,000, distinct 220,912 | 归母综合收益总额同比增长率（%） |
| MINORITY_TCI_YOY | Nullable(Float64) | 111,524 | 零值 40；负值 82,800 | min=-128,431,000,000, max=102,429,000,000, distinct 167,322 | 少数股东综合收益总额同比增长率（%） |
| EFFECT_TCI_BALANCE_YOY | Nullable(Float64) | 279,917 | 零值 0；负值 1 | min=-26.6208, max=-26.6208, distinct 1 | 综合收益总额平衡项同比增长率（%） |
| TCI_OTHER_YOY | Nullable(Float64) | 279,871 | 零值 3；负值 28 | min=-100, max=1,937.79, distinct 43 | 综合收益总额其他同比增长率（%） |
| TCI_BALANCE_YOY | Nullable(Float64) | 279,917 | 零值 0；负值 0 | min=200, max=200, distinct 1 | 综合收益总额平衡项同比增长率（%） |
| PRECOMBINE_PROFIT_YOY | Nullable(Float64) | 279,771 | 零值 0；负值 79 | min=-17,788.1, max=7,607.46, distinct 114 | 合并前净损益同比增长率（%） |
| PRECOMBINE_TCI_YOY | Nullable(Float64) | 279,918 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 合并前综合收益总额同比增长率（%） |
| DEDUCT_PARENT_NETPROFIT_YOY | Nullable(Float64) | 24,887 | 零值 47；负值 118,640 | min=-371,333,000,000, max=252,781,000, distinct 253,529 | 扣非归母净利润同比增长率（%） |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 279,918/279,918，供应商前缀 0/279,918，纯数字 0/279,918，空值 0/279,918；`SECURITY_CODE`: canonical 后缀 0/279,918，供应商前缀 0/279,918，纯数字 279,918/279,918，空值 0/279,918
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：`REPORT_DATE`: 1993-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1993-08-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1993-08-14 至 2026-06-02，NULL 193 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME`, `SECURITY_TYPE_CODE`, `CURRENCY`
- 取值：`SECURITY_CODE`: `000553`(103), `000025`(103), `000411`(102), `000592`(102), `600081`(101), `600717`(101), `000869`(101), `600703`(101)；`SECURITY_NAME_ABBR`: `东方明珠`(150), `百联股份`(135), `特力A`(103), `安道麦A`(103), `英特集团`(102), `平潭发展`(102), `诚志股份`(101), `百花医药`(101)；`ORG_CODE`: `10004127`(168), `10004106`(164), `10116535`(126), `10004293`(124), `10004109`(103), `10005499`(103), `10564780`(102), `10005533`(102)；`ORG_TYPE`: `通用`(279,918)；`REPORT_TYPE`: `一季度`(73,061), `三季度`(69,893), `四季度`(69,543), `二季度`(67,421)；`REPORT_DATE_NAME`: `2026一季度`(5,099), `2025四季度`(5,084), `2024一季度`(5,073), `2025一季度`(5,072), `2025三季度`(5,060), `2025二季度`(5,051), `2024三季度`(5,026), `2024四季度`(5,019)；`SECURITY_TYPE_CODE`: `058001001`(279,895), `058001008`(23)；`CURRENCY`: `CNY`(279,424), `NULL`(494)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 285 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 285 个数值字段执行 min/max、NULL、零值和负值检查；其中 247 个字段出现负值，195 个字段出现零值，169 个字段 NULL 数不低于 80%。 负值字段样例：`PARENT_NETPROFIT_QOQ` 136,516 行(min=-11,225,600,000)，`OPERATE_PROFIT_QOQ` 136,299 行(min=-19,177,700)，`NETPROFIT_QOQ` 136,277 行(min=-14,944,200)，`TOTAL_PROFIT_QOQ` 136,235 行(min=-30,705,900)，`FINANCE_EXPENSE_QOQ` 132,272 行(min=-25,720,300)，`DEDUCT_PARENT_NETPROFIT_QOQ` 131,193 行(min=-13,312,900)，`OPERATE_TAX_ADD_QOQ` 129,390 行(min=-2,203,570,000)，`INCOME_TAX_QOQ` 128,374 行(min=-21,887,500,000)。 高 NULL 字段样例：`ME_RESEARCH_EXPENSE` 279,918 行，`ME_RESEARCH_EXPENSE_QOQ` 279,918 行，`OPERATE_PROFIT_BALANCE_QOQ` 279,918 行，`TOTAL_PROFIT_BALANCE_QOQ` 279,918 行，`NETPROFIT_BALANCE_QOQ` 279,918 行，`EFFECT_NETPROFIT_BALANCE_QOQ` 279,918 行，`CREDITRISK_FAIRVALUE_CHANGE_QOQ` 279,918 行，`UNABLE_OCI_BALANCE` 279,918 行。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECURITY_CODE` 为 6 位本地代码 | 中 | 279,918/279,918 行为纯数字 | 只作为 `security_local_code`，不可单独推出交易所 | 交易所归属或证券主数据修正延后 |
| 财务数值存在负值 | 低 | 247 个数值字段出现负值 | 负数符合财务科目/调整项可能性，staging 不过滤 | 口径解释和异常阈值延后 |

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

- 行数：279,918。
- 日期 / 分区范围：`REPORT_DATE`: 1993-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1993-08-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1993-08-14 至 2026-06-02，NULL 193 行，`1970-01-01` 占位 0 行
- 候选键重复：未发现重复。
- 关键 NULL / 占位值：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`SECURITY_CODE`: `000553`(103), `000025`(103), `000411`(102), `000592`(102), `600081`(101), `600717`(101), `000869`(101), `600703`(101)；`SECURITY_NAME_ABBR`: `东方明珠`(150), `百联股份`(135), `特力A`(103), `安道麦A`(103), `英特集团`(102), `平潭发展`(102), `诚志股份`(101), `百花医药`(101)；`ORG_CODE`: `10004127`(168), `10004106`(164), `10116535`(126), `10004293`(124), `10004109`(103), `10005499`(103), `10564780`(102), `10005533`(102)；`ORG_TYPE`: `通用`(279,918)；`REPORT_TYPE`: `一季度`(73,061), `三季度`(69,893), `四季度`(69,543), `二季度`(67,421)；`REPORT_DATE_NAME`: `2026一季度`(5,099), `2025四季度`(5,084), `2024一季度`(5,073), `2025一季度`(5,072), `2025三季度`(5,060), `2025二季度`(5,051), `2024三季度`(5,026), `2024四季度`(5,019)；`SECURITY_TYPE_CODE`: `058001001`(279,895), `058001008`(23)；`CURRENCY`: `CNY`(279,424), `NULL`(494)
- 数值范围：已对 285 个数值字段执行 min/max、NULL、零值和负值检查；其中 247 个字段出现负值，195 个字段出现零值，169 个字段 NULL 数不低于 80%。 负值字段样例：`PARENT_NETPROFIT_QOQ` 136,516 行(min=-11,225,600,000)，`OPERATE_PROFIT_QOQ` 136,299 行(min=-19,177,700)，`NETPROFIT_QOQ` 136,277 行(min=-14,944,200)，`TOTAL_PROFIT_QOQ` 136,235 行(min=-30,705,900)，`FINANCE_EXPENSE_QOQ` 132,272 行(min=-25,720,300)，`DEDUCT_PARENT_NETPROFIT_QOQ` 131,193 行(min=-13,312,900)，`OPERATE_TAX_ADD_QOQ` 129,390 行(min=-2,203,570,000)，`INCOME_TAX_QOQ` 128,374 行(min=-21,887,500,000)。 高 NULL 字段样例：`ME_RESEARCH_EXPENSE` 279,918 行，`ME_RESEARCH_EXPENSE_QOQ` 279,918 行，`OPERATE_PROFIT_BALANCE_QOQ` 279,918 行，`TOTAL_PROFIT_BALANCE_QOQ` 279,918 行，`NETPROFIT_BALANCE_QOQ` 279,918 行，`EFFECT_NETPROFIT_BALANCE_QOQ` 279,918 行，`CREDITRISK_FAIRVALUE_CHANGE_QOQ` 279,918 行，`UNABLE_OCI_BALANCE` 279,918 行。

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
select `SECUCODE`, `REPORT_DATE`, `SECURITY_CODE`, `NOTICE_DATE`, `UPDATE_DATE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME` from fleur_raw.eastmoney__income_sq limit 5
```

结果：

```text
[{'SECUCODE': '600665.SH', 'REPORT_DATE': datetime.date(1993, 6, 30), 'SECURITY_CODE': '600665', 'NOTICE_DATE': datetime.date(1993, 8, 14), 'UPDATE_DATE': datetime.date(1993, 8, 14), 'SECURITY_NAME_ABBR': '天地源', 'ORG_CODE': '10003975', 'ORG_TYPE': '通用', 'REPORT_TYPE': '二季度', 'REPORT_DATE_NAME': '1993二季度'}, {'SECUCODE': '600668.SH', 'REPORT_DATE': datetime.date(1993, 6, 30), 'SECURITY_CODE': '600668', 'NOTICE_DATE': datetime.date(1993, 8, 28), 'UPDATE_DATE': datetime.date(1993, 8, 28), 'SECURITY_NAME_ABBR': '尖峰集团', 'ORG_CODE': '10003978', 'ORG_TYPE': '通用', 'REPORT_TYPE': '二季度', 'REPORT_DATE_NAME': '1993二季度'}, {'SECUCODE': '000550.SZ', 'REPORT_DATE': datetime.date(1993, 12, 31), 'SECURITY_CODE': '000550', 'NOTICE_DATE': datetime.date(1994, 3, 30), 'UPDATE_DATE': datetime.date(1994, 3, 30), 'SECURITY_NAME_ABBR': '江铃汽车', 'ORG_CODE': '10005496', 'ORG_TYPE': '通用', 'REPORT_TYPE': '四季度', 'REPORT_DATE_NAME': '1993四季度'}, {'SECUCODE': '600861.SH', 'REPORT_DATE': datetime.date(1994, 6, 30), 'SECURITY_CODE': '600861', 'NOTICE_DATE': datetime.date(1994, 8, 19), 'UPDATE_DATE': datetime.date(1994, 8, 19), 'SECURITY_NAME_ABBR': '北京人力', 'ORG_CODE': '10004305', 'ORG_TYPE': '通用', 'REPORT_TYPE': '二季度', 'REPORT_DATE_NAME': '1994二季度'}, {'SECUCODE': '000541.SZ', 'REPORT_DATE': datetime.date(1993, 12, 31), 'SECURITY_CODE': '000541', 'NOTICE_DATE': datetime.date(1995, 8, 5), 'UPDATE_DATE': datetime.date(1995, 8, 5), 'SECURITY_NAME_ABBR': '佛山照明', 'ORG_CODE': '10005487', 'ORG_TYPE': '通用', 'REPORT_TYPE': '四季度', 'REPORT_DATE_NAME': '1993四季度'}]
```

### 行数统计

```sql
select count() from fleur_raw.eastmoney__income_sq
```

结果：

```text
[[279918]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `SECUCODE`, `REPORT_DATE`, count() as row_count from fleur_raw.eastmoney__income_sq group by `SECUCODE`, `REPORT_DATE` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 0, 'max_rows_per_key': 0}
```

### 证券代码格式：SECUCODE

```sql
select countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__income_sq
```

结果：

```text
{'canonical_suffix': 279918, 'vendor_prefix': 0, 'numeric_only': 0, 'empty_or_null': 0, 'row_count': 279918}
```

### 证券代码格式：SECURITY_CODE

```sql
select countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__income_sq
```

结果：

```text
{'canonical_suffix': 0, 'vendor_prefix': 0, 'numeric_only': 279918, 'empty_or_null': 0, 'row_count': 279918}
```
