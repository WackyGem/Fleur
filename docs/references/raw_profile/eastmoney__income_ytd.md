# Raw 数据画像：eastmoney__income_ytd

日期：2026-06-03

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__income_ytd.yml`
- dbt source：`source('raw', 'eastmoney__income_ytd')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__income_ytd.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__income_ytd`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__income_ytd --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__income_ytd.md`
- 行数：298,396
- 数据范围：`REPORT_DATE`: 1988-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 523 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`eastmoney__income_ytd`
- ClickHouse raw 表：`fleur_raw.eastmoney__income_ytd`
- 表说明：EastMoney year-to-date income F10 rows by natural-year raw partition.

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：298,396。
  - 覆盖主体数：`secucode` 5,524 个；`security_code` 5,524 个
  - 日期 / 分区范围：`REPORT_DATE`: 1988-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 523 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `SECUCODE`, `REPORT_DATE`。
  - 候选自然键去重结果：未发现重复。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：`SECUCODE`: canonical 后缀 298,396/298,396，供应商前缀 0/298,396，纯数字 0/298,396，空值 0/298,396；`SECURITY_CODE`: canonical 后缀 0/298,396，供应商前缀 0/298,396，纯数字 298,396/298,396，空值 0/298,396
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`SECURITY_CODE`: `600654`(123), `600653`(123), `600601`(121), `600651`(121), `600610`(120), `000030`(120), `600602`(119), `000501`(119)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(123), `申华控股`(123), `方正科技`(121), `飞乐音响`(121), `中毅达`(120), `富奥股份`(120)；`ORG_CODE`: `10004106`(200), `10004127`(198), `10004293`(154), `10116535`(128), `10003963`(123), `10003964`(123), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(292,603), `证券`(2,826), `银行`(2,449), `保险`(518)；`REPORT_TYPE`: `年报`(77,446), `中报`(74,897), `一季报`(74,512), `三季报`(71,541)；`REPORT_DATE_NAME`: `2026一季报`(5,198), `2025年报`(5,183), `2024一季报`(5,174), `2025一季报`(5,171), `2025三季报`(5,159), `2025中报`(5,150), `2024三季报`(5,127), `2024年报`(5,118)；`SECURITY_TYPE_CODE`: `058001001`(298,373), `058001008`(23)；`CURRENCY`: `CNY`(297,518), `NULL`(878)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`REPORT_DATE`: 1988-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 523 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 190 个数值字段执行 min/max、NULL、零值和负值检查；其中 174 个字段出现负值，162 个字段出现零值，112 个字段 NULL 数不低于 80%。 负值字段样例：`NONBUSINESS_INCOME_YOY` 131,209 行(min=-22,094,300)，`BASIC_EPS_YOY` 130,232 行(min=-138,467)，`FINANCE_EXPENSE_YOY` 129,792 行(min=-4,745,010)，`NONBUSINESS_EXPENSE_YOY` 125,739 行(min=-381,496,000)，`INCOME_TAX_YOY` 125,154 行(min=-963,525,000)，`OPERATE_PROFIT_YOY` 124,133 行(min=-2,095,560)，`TOTAL_PROFIT_YOY` 123,232 行(min=-12,062,300)，`NETPROFIT_YOY` 122,314 行(min=-11,355,100)。 高 NULL 字段样例：`ME_RESEARCH_EXPENSE_YOY` 298,396 行，`PARENT_OCI_BALANCE_YOY` 298,396 行，`UNABLE_OCI_BALANCE_YOY` 298,396 行，`ABLE_OCI_BALANCE_YOY` 298,396 行，`OCI_BALANCE_YOY` 298,396 行，`PRECOMBINE_TCI_YOY` 298,396 行，`EFFECT_TCI_BALANCE_YOY` 298,396 行，`TCI_BALANCE_YOY` 298,396 行。
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
| SECUCODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,524 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,524 | 证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,522 | 证券简称 |
| ORG_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,517 | 机构代码 |
| ORG_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 4 | 机构类型 |
| REPORT_DATE | Date | 0 | `1970-01-01` 0 | 1988-12-31 至 2026-03-31; distinct 128 | 报告期 |
| REPORT_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 4 | 报告类型 |
| REPORT_DATE_NAME | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 128 | 报告期名称 |
| SECURITY_TYPE_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 2 | 证券类型代码 |
| NOTICE_DATE | Date | 0 | `1970-01-01` 0 | 1991-06-10 至 2026-05-15; distinct 4,356 | 公告日期 |
| UPDATE_DATE | Nullable(Date) | 523 | `1970-01-01` 0 | 1991-06-10 至 2026-06-02; distinct 5,171 | 更新日期 |
| CURRENCY | LowCardinality(Nullable(String)) | 878 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 利润表年初至报告期末金额使用的币种。 |
| TOTAL_OPERATE_INCOME | Nullable(Float64) | 1,065 | 零值 223；负值 34 | min=-673,496,000, max=3,318,170,000,000, distinct 296,702 | 营业总收入 |
| TOTAL_OPERATE_INCOME_YOY | Nullable(Float64) | 11,609 | 零值 11；负值 95,980 | min=-306.469, max=6,998,080, distinct 286,380 | 营业总收入同比增长率（%） |
| OPERATE_INCOME | Nullable(Float64) | 1,173 | 零值 163；负值 34 | min=-673,496,000, max=3,318,170,000,000, distinct 296,647 | 营业收入 |
| OPERATE_INCOME_YOY | Nullable(Float64) | 12,034 | 零值 14；负值 95,914 | min=-306.469, max=6,998,080, distinct 285,996 | 营业收入同比增长率（%） |
| INTEREST_INCOME | Nullable(Float64) | 286,898 | 零值 2,537；负值 8 | min=-116,121,000, max=1,427,950,000,000, distinct 8,881 | 利息收入 |
| INTEREST_INCOME_YOY | Nullable(Float64) | 290,367 | 零值 4；负值 3,219 | min=-1,156.92, max=180,269,000, distinct 8,020 | 利息收入同比增长率（%） |
| EARNED_PREMIUM | Nullable(Float64) | 294,915 | 零值 2,675；负值 2 | min=-4,289,320, max=757,599,000,000, distinct 796 | 已赚保费 |
| EARNED_PREMIUM_YOY | Nullable(Float64) | 297,691 | 零值 0；负值 217 | min=-101.315, max=35,873, distinct 705 | 已赚保费同比增长率（%） |
| FEE_COMMISSION_INCOME | Nullable(Float64) | 288,436 | 零值 2,675；负值 12 | min=-854,403, max=164,714,000,000, distinct 7,190 | 手续费及佣金收入 |
| FEE_COMMISSION_INCOME_YOY | Nullable(Float64) | 291,736 | 零值 3；负值 2,699 | min=-205.904, max=640,245, distinct 6,650 | 手续费及佣金收入同比增长率（%） |
| OTHER_BUSINESS_INCOME | Nullable(Float64) | 293,337 | 零值 37；负值 52 | min=-534,557,000, max=77,782,000,000, distinct 4,712 | 其他业务收入 |
| OTHER_BUSINESS_INCOME_YOY | Nullable(Float64) | 293,980 | 零值 12；负值 1,759 | min=-19,654.7, max=119,900, distinct 4,379 | 其他业务收入同比增长率（%） |
| TOI_OTHER | Nullable(Float64) | 297,507 | 零值 614；负值 11 | min=-2,829,110,000, max=159,626,000,000, distinct 273 | 营业总收入其他 |
| TOI_OTHER_YOY | Nullable(Float64) | 298,182 | 零值 0；负值 74 | min=-174.113, max=5,757.35, distinct 210 | 营业总收入其他同比增长率（%） |
| TOTAL_OPERATE_COST | Nullable(Float64) | 2,635 | 零值 298；负值 69 | min=-683,986,000, max=3,232,470,000,000, distinct 295,172 | 营业总成本 |
| TOTAL_OPERATE_COST_YOY | Nullable(Float64) | 14,073 | 零值 2；负值 91,324 | min=-10,525.2, max=721,477, distinct 284,042 | 营业总成本同比增长率（%） |
| OPERATE_COST | Nullable(Float64) | 7,529 | 零值 233；负值 20 | min=-89,903,700, max=2,819,360,000,000, distinct 290,233 | 营业成本 |
| OPERATE_COST_YOY | Nullable(Float64) | 18,846 | 零值 24；负值 93,256 | min=-529.846, max=46,354,000, distinct 279,171 | 营业成本同比增长率（%） |
| INTEREST_EXPENSE | Nullable(Float64) | 289,137 | 零值 2,595；负值 14 | min=-5,143,930,000, max=790,543,000,000, distinct 6,616 | 利息支出 |
| INTEREST_EXPENSE_YOY | Nullable(Float64) | 292,431 | 零值 1；负值 2,368 | min=-1,843.56, max=19,135,500, distinct 5,960 | 利息支出同比增长率（%） |
| FEE_COMMISSION_EXPENSE | Nullable(Float64) | 290,582 | 零值 2,565；负值 4 | min=-299,524, max=130,383,000,000, distinct 5,105 | 手续费及佣金支出 |
| FEE_COMMISSION_EXPENSE_YOY | Nullable(Float64) | 293,634 | 零值 0；负值 1,733 | min=-189.535, max=691,518, distinct 4,754 | 手续费及佣金支出同比增长率（%） |
| RESEARCH_EXPENSE | Nullable(Float64) | 156,709 | 零值 377；负值 11 | min=-16,028,800, max=57,978,100,000, distinct 141,002 | 研发费用 |
| RESEARCH_EXPENSE_YOY | Nullable(Float64) | 172,433 | 零值 22；负值 43,686 | min=-128.847, max=3,277,810, distinct 125,806 | 研发费用同比增长率（%） |
| SURRENDER_VALUE | Nullable(Float64) | 295,480 | 零值 2,582；负值 4 | min=-3,077,220,000, max=116,229,000,000, distinct 335 | 退保金 |
| SURRENDER_VALUE_YOY | Nullable(Float64) | 298,093 | 零值 0；负值 121 | min=-616.035, max=4,172.41, distinct 303 | 退保金同比增长率（%） |
| NET_COMPENSATE_EXPENSE | Nullable(Float64) | 294,928 | 零值 3,007；负值 1 | min=-11,071,200, max=309,798,000,000, distinct 456 | 分保费用 |
| NET_COMPENSATE_EXPENSE_YOY | Nullable(Float64) | 297,988 | 零值 0；负值 110 | min=-91.346, max=3,449.62, distinct 408 | 分保费用同比增长率（%） |
| NET_CONTRACT_RESERVE | Nullable(Float64) | 279,407 | 零值 17,857；负值 130 | min=-9,813,000,000, max=446,411,000,000, distinct 1,101 | 提取保险合同准备金 |
| NET_CONTRACT_RESERVE_YOY | Nullable(Float64) | 297,402 | 零值 1；负值 427 | min=-38,693.8, max=35,224.3, distinct 985 | 提取保险合同准备金同比增长率（%） |
| POLICY_BONUS_EXPENSE | Nullable(Float64) | 295,516 | 零值 2,573；负值 0 | min=0, max=33,491,000,000, distinct 308 | 保单红利支出 |
| POLICY_BONUS_EXPENSE_YOY | Nullable(Float64) | 298,122 | 零值 0；负值 88 | min=-97.3193, max=10,500, distinct 274 | 保单红利支出同比增长率（%） |
| REINSURE_EXPENSE | Nullable(Float64) | 295,509 | 零值 2,616；负值 14 | min=-89,041,000, max=1,336,000,000, distinct 194 | 分保费用支出 |
| REINSURE_EXPENSE_YOY | Nullable(Float64) | 298,159 | 零值 2；负值 76 | min=-98.8963, max=106,598, distinct 214 | 分保费用支出同比增长率（%） |
| OTHER_BUSINESS_COST | Nullable(Float64) | 294,154 | 零值 25；负值 3 | min=-141,000,000, max=83,243,000,000, distinct 3,826 | 其他业务成本 |
| OTHER_BUSINESS_COST_YOY | Nullable(Float64) | 294,471 | 零值 152；负值 1,489 | min=-316.923, max=1,523,600, distinct 3,702 | 其他业务成本同比增长率（%） |
| OPERATE_TAX_ADD | Nullable(Float64) | 4,070 | 零值 147；负值 569 | min=-441,111,000, max=295,106,000,000, distinct 293,117 | 营业税金及附加 |
| OPERATE_TAX_ADD_YOY | Nullable(Float64) | 16,015 | 零值 17；负值 109,398 | min=-14,554.9, max=4,506,070,000, distinct 282,000 | 营业税金及附加同比增长率（%） |
| SALE_EXPENSE | Nullable(Float64) | 14,199 | 零值 267；负值 28 | min=-63,358,300, max=74,108,000,000, distinct 283,330 | 销售费用 |
| SALE_EXPENSE_YOY | Nullable(Float64) | 26,237 | 零值 20；负值 100,584 | min=-320.09, max=66,921,200, distinct 271,875 | 销售费用同比增长率（%） |
| MANAGE_EXPENSE | Nullable(Float64) | 2,263 | 零值 97；负值 177 | min=-355,563,000, max=255,131,000,000, distinct 295,697 | 管理费用 |
| MANAGE_EXPENSE_YOY | Nullable(Float64) | 13,461 | 零值 1；负值 95,230 | min=-2,609.62, max=209,762, distinct 284,664 | 管理费用同比增长率（%） |
| ME_RESEARCH_EXPENSE | Nullable(Float64) | 298,367 | 零值 0；负值 0 | min=124,988, max=241,547,000, distinct 29 | 管理费用中的研发费用 |
| ME_RESEARCH_EXPENSE_YOY | Nullable(Float64) | 298,396 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 管理费用中的研发费用同比增长率（%） |
| FINANCE_EXPENSE | Nullable(Float64) | 7,132 | 零值 86；负值 83,527 | min=-8,605,000,000, max=27,816,000,000, distinct 290,843 | 财务费用 |
| FINANCE_EXPENSE_YOY | Nullable(Float64) | 18,206 | 零值 6；负值 129,792 | min=-4,745,010, max=30,936,300, distinct 279,916 | 财务费用同比增长率（%） |
| FE_INTEREST_EXPENSE | Nullable(Float64) | 157,996 | 零值 396；负值 420 | min=-22,647,000,000, max=30,409,000,000, distinct 138,695 | 财务费用之利息费用 |
| FE_INTEREST_EXPENSE_YOY | Nullable(Float64) | 176,494 | 零值 78；负值 58,657 | min=-15,410.5, max=29,341,900, distinct 121,595 | 财务费用之利息费用同比增长率（%） |
| FE_INTEREST_INCOME | Nullable(Float64) | 147,737 | 零值 32；负值 7,280 | min=-8,183,000,000, max=24,241,000,000, distinct 150,318 | 财务费用之利息收入 |
| FE_INTEREST_INCOME_YOY | Nullable(Float64) | 163,991 | 零值 26；负值 63,377 | min=-461,084, max=33,037,700, distinct 134,328 | 财务费用之利息收入同比增长率（%） |
| ASSET_IMPAIRMENT_LOSS | Nullable(Float64) | 193,338 | 零值 1,147；负值 22,551 | min=-2,332,000,000, max=202,668,000,000, distinct 99,582 | 资产减值损失 |
| ASSET_IMPAIRMENT_LOSS_YOY | Nullable(Float64) | 206,394 | 零值 22；负值 39,872 | min=-30,826,500, max=313,773,000, distinct 90,064 | 资产减值损失同比增长率（%） |
| CREDIT_IMPAIRMENT_LOSS | Nullable(Float64) | 295,368 | 零值 44；负值 574 | min=-1,754,000,000, max=193,491,000,000, distinct 2,975 | 信用减值损失 |
| CREDIT_IMPAIRMENT_LOSS_YOY | Nullable(Float64) | 295,970 | 零值 1；负值 1,181 | min=-251,566, max=399,621, distinct 2,426 | 信用减值损失同比增长率（%） |
| TOC_OTHER | Nullable(Float64) | 281,947 | 零值 15,761；负值 238 | min=-14,610,700,000, max=97,082,000,000, distinct 666 | 营业总成本其他 |
| TOC_OTHER_YOY | Nullable(Float64) | 297,824 | 零值 0；负值 293 | min=-379,387, max=37,186.8, distinct 560 | 营业总成本其他同比增长率（%） |
| FAIRVALUE_CHANGE_INCOME | Nullable(Float64) | 190,422 | 零值 3,732；负值 41,503 | min=-49,952,000,000, max=147,655,000,000, distinct 98,848 | 公允价值变动收益 |
| FAIRVALUE_CHANGE_INCOME_YOY | Nullable(Float64) | 216,162 | 零值 45；负值 40,207 | min=-71,267,900, max=1,453,530,000, distinct 79,906 | 公允价值变动收益同比增长率（%） |
| INVEST_INCOME | Nullable(Float64) | 46,081 | 零值 1,622；负值 56,989 | min=-11,686,600,000, max=241,814,000,000, distinct 240,498 | 投资收益 |
| INVEST_INCOME_YOY | Nullable(Float64) | 71,794 | 零值 776；负值 107,746 | min=-142,870,000, max=10,250,200,000, distinct 223,055 | 投资收益同比增长率（%） |
| INVEST_JOINT_INCOME | Nullable(Float64) | 157,676 | 零值 2,102；负值 59,128 | min=-14,033,100,000, max=28,303,600,000, distinct 135,361 | 对联营企业和合营企业的投资收益 |
| INVEST_JOINT_INCOME_YOY | Nullable(Float64) | 176,973 | 零值 101；负值 57,898 | min=-1,235,150,000, max=5,500,440,000, distinct 120,318 | 对联营企业和合营企业的投资收益同比增长率（%） |
| NET_EXPOSURE_INCOME | Nullable(Float64) | 296,193 | 零值 2,059；负值 62 | min=-1,108,440,000, max=135,856,000, distinct 137 | 净敞口收益 |
| NET_EXPOSURE_INCOME_YOY | Nullable(Float64) | 298,309 | 零值 0；负值 44 | min=-2,165.41, max=2,365.58, distinct 86 | 净敞口收益同比增长率（%） |
| EXCHANGE_INCOME | Nullable(Float64) | 288,433 | 零值 3,100；负值 2,856 | min=-32,365,000,000, max=22,517,000,000, distinct 6,542 | 汇兑收益 |
| EXCHANGE_INCOME_YOY | Nullable(Float64) | 292,313 | 零值 4；负值 3,032 | min=-107,479,000, max=33,478,500, distinct 6,049 | 汇兑收益同比增长率（%） |
| ASSET_DISPOSAL_INCOME | Nullable(Float64) | 171,644 | 零值 1,697；负值 49,056 | min=-5,009,480,000, max=8,982,580,000, distinct 114,192 | 资产处置收益 |
| ASSET_DISPOSAL_INCOME_YOY | Nullable(Float64) | 198,974 | 零值 18；负值 48,858 | min=-26,393,400,000, max=470,215,000,000, distinct 96,787 | 资产处置收益同比增长率（%） |
| ASSET_IMPAIRMENT_INCOME | Nullable(Float64) | 185,526 | 零值 1,146；负值 92,527 | min=-37,233,000,000, max=1,205,640,000, distinct 108,320 | 资产减值收益 |
| ASSET_IMPAIRMENT_INCOME_YOY | Nullable(Float64) | 208,521 | 零值 16；负值 48,657 | min=-3,137,920,000, max=15,741,100, distinct 88,595 | 资产减值收益同比增长率（%） |
| CREDIT_IMPAIRMENT_INCOME | Nullable(Float64) | 168,734 | 零值 355；负值 90,332 | min=-39,713,900,000, max=2,205,680,000, distinct 128,046 | 信用减值收益 |
| CREDIT_IMPAIRMENT_INCOME_YOY | Nullable(Float64) | 185,888 | 零值 4；负值 59,674 | min=-35,126,900, max=26,454,300, distinct 111,998 | 信用减值收益同比增长率（%） |
| OTHER_INCOME | Nullable(Float64) | 140,679 | 零值 420；负值 179 | min=-4,415,680,000, max=21,704,000,000, distinct 155,180 | 其他收益 |
| OTHER_INCOME_YOY | Nullable(Float64) | 157,535 | 零值 258；负值 63,615 | min=-130,353, max=12,114,300, distinct 140,264 | 其他收益同比增长率（%） |
| OPERATE_PROFIT_OTHER | Nullable(Float64) | 297,532 | 零值 774；负值 40 | min=-19,935,000,000, max=6,516,860,000, distinct 90 | 营业利润其他 |
| OPERATE_PROFIT_OTHER_YOY | Nullable(Float64) | 298,378 | 零值 0；负值 14 | min=-11,592.6, max=100, distinct 18 | 营业利润其他同比增长率（%） |
| OPERATE_PROFIT_BALANCE | Nullable(Float64) | 48,894 | 零值 218,012；负值 5,235 | min=-9,464,260,000, max=5,216,150,000, distinct 30,806 | 营业利润平衡项 |
| OPERATE_PROFIT_BALANCE_YOY | Nullable(Float64) | 270,962 | 零值 16；负值 12,035 | min=-13,755,300,000, max=28,263,700,000, distinct 26,134 | 营业利润平衡项同比增长率（%） |
| OPERATE_PROFIT | Nullable(Float64) | 356 | 零值 80；负值 52,767 | min=-71,550,900,000, max=424,111,000,000, distinct 297,622 | 营业利润 |
| OPERATE_PROFIT_YOY | Nullable(Float64) | 10,865 | 零值 3；负值 124,133 | min=-2,095,560, max=20,625,100, distinct 287,242 | 营业利润同比增长率（%） |
| NONBUSINESS_INCOME | Nullable(Float64) | 11,406 | 零值 489；负值 322 | min=-56,694,300, max=51,911,000,000, distinct 274,978 | 营业外收入 |
| NONBUSINESS_INCOME_YOY | Nullable(Float64) | 27,891 | 零值 113；负值 131,209 | min=-22,094,300, max=92,053,100,000, distinct 268,742 | 营业外收入同比增长率（%） |
| NONCURRENT_DISPOSAL_INCOME | Nullable(Float64) | 279,306 | 零值 328；负值 24 | min=-22,661,700, max=9,815,820,000, distinct 16,765 | 非流动资产处置净收益 |
| NONCURRENT_DISPOSAL_INCOME_YOY | Nullable(Float64) | 288,098 | 零值 13；负值 5,121 | min=-210,976, max=2,417,550,000, distinct 9,985 | 非流动资产处置净收益同比增长率（%） |
| NONBUSINESS_EXPENSE | Nullable(Float64) | 11,840 | 零值 612；负值 1,568 | min=-3,122,000,000, max=32,807,000,000, distinct 273,521 | 营业外支出 |
| NONBUSINESS_EXPENSE_YOY | Nullable(Float64) | 29,285 | 零值 138；负值 125,739 | min=-381,496,000, max=741,125,000,000, distinct 266,899 | 营业外支出同比增长率（%） |
| NONCURRENT_DISPOSAL_LOSS | Nullable(Float64) | 243,743 | 零值 1,062；负值 610 | min=-131,113,000, max=2,535,970,000, distinct 49,200 | 非流动资产处置净损失 |
| NONCURRENT_DISPOSAL_LOSS_YOY | Nullable(Float64) | 257,632 | 零值 58；负值 19,311 | min=-16,864,300, max=640,047,000, distinct 39,694 | 非流动资产处置净损失同比增长率（%） |
| EFFECT_TP_OTHER | Nullable(Float64) | 282,780 | 零值 15,397；负值 25 | min=-3,955,770,000, max=1,381,000,000, distinct 209 | 影响利润总额其他 |
| EFFECT_TP_OTHER_YOY | Nullable(Float64) | 298,377 | 零值 0；负值 13 | min=-5,429,420, max=542.613, distinct 19 | 影响利润总额其他同比增长率（%） |
| TOTAL_PROFIT_BALANCE | Nullable(Float64) | 47,020 | 零值 235,353；负值 989 | min=-5,885,130,000, max=29,429,000,000, distinct 13,164 | 利润总额平衡项 |
| TOTAL_PROFIT_BALANCE_YOY | Nullable(Float64) | 284,472 | 零值 57；负值 8,226 | min=-76,681,700,000, max=337,572,000,000, distinct 9,806 | 利润总额平衡项同比增长率（%） |
| TOTAL_PROFIT | Nullable(Float64) | 25 | 零值 16；负值 47,596 | min=-73,923,800,000, max=424,899,000,000, distinct 298,035 | 利润总额 |
| TOTAL_PROFIT_YOY | Nullable(Float64) | 10,341 | 零值 2；负值 123,232 | min=-12,062,300, max=1,793,210, distinct 287,780 | 利润总额同比增长率（%） |
| INCOME_TAX | Nullable(Float64) | 10,196 | 零值 343；负值 28,971 | min=-7,894,050,000, max=85,515,000,000, distinct 286,136 | 所得税费用 |
| INCOME_TAX_YOY | Nullable(Float64) | 24,424 | 零值 58；负值 125,154 | min=-963,525,000, max=55,332,200,000, distinct 273,284 | 所得税费用同比增长率（%） |
| EFFECT_NETPROFIT_OTHER | Nullable(Float64) | 298,334 | 零值 37；负值 12 | min=-58,000,000, max=27,947,400,000, distinct 24 | 影响净利润其他 |
| EFFECT_NETPROFIT_OTHER_YOY | Nullable(Float64) | 298,392 | 零值 0；负值 1 | min=-101.463, max=2,113.23, distinct 4 | 影响净利润其他同比增长率（%） |
| EFFECT_NETPROFIT_BALANCE | Nullable(Float64) | 265,516 | 零值 31,564；负值 715 | min=-2,124,670,000, max=290,545,000, distinct 1,103 | 净利润平衡项 |
| EFFECT_NETPROFIT_BALANCE_YOY | Nullable(Float64) | 297,012 | 零值 6；负值 621 | min=-16,257,900,000, max=5,575,910,000, distinct 350 | 净利润平衡项同比增长率（%） |
| UNCONFIRM_INVEST_LOSS | Nullable(Float64) | 295,169 | 零值 245；负值 551 | min=-160,000,000, max=507,000,000, distinct 2,903 | 未确认投资损失 |
| UNCONFIRM_INVEST_LOSS_YOY | Nullable(Float64) | 296,738 | 零值 6；负值 842 | min=-127,906, max=6,472,760, distinct 1,637 | 未确认投资损失同比增长率（%） |
| NETPROFIT | Nullable(Float64) | 84 | 零值 3；负值 48,643 | min=-91,810,100,000, max=370,766,000,000, distinct 297,983 | 净利润 |
| NETPROFIT_YOY | Nullable(Float64) | 10,716 | 零值 2；负值 122,314 | min=-11,355,100, max=7,179,670, distinct 287,416 | 净利润同比增长率（%） |
| PRECOMBINE_PROFIT | Nullable(Float64) | 296,706 | 零值 983；负值 237 | min=-1,356,000,000, max=6,595,790,000, distinct 667 | 合并前净损益 |
| PRECOMBINE_PROFIT_YOY | Nullable(Float64) | 298,238 | 零值 0；负值 88 | min=-5,521.69, max=4,846,410, distinct 153 | 合并前净损益同比增长率（%） |
| CONTINUED_NETPROFIT | Nullable(Float64) | 134,968 | 零值 12；负值 30,608 | min=-91,810,100,000, max=183,747,000,000, distinct 163,346 | 持续经营净利润 |
| CONTINUED_NETPROFIT_YOY | Nullable(Float64) | 151,439 | 零值 6；负值 66,747 | min=-11,355,100, max=1,350,060, distinct 146,909 | 持续经营净利润同比增长率（%） |
| DISCONTINUED_NETPROFIT | Nullable(Float64) | 291,053 | 零值 2,364；负值 2,812 | min=-9,567,140,000, max=11,066,100,000, distinct 4,508 | 终止经营净利润 |
| DISCONTINUED_NETPROFIT_YOY | Nullable(Float64) | 295,906 | 零值 5；负值 996 | min=-62,043,400, max=179,555,000, distinct 2,380 | 终止经营净利润同比增长率（%） |
| PARENT_NETPROFIT | Nullable(Float64) | 11 | 零值 3；负值 47,616 | min=-88,556,500,000, max=368,562,000,000, distinct 298,064 | 归属于母公司股东的净利润 |
| PARENT_NETPROFIT_YOY | Nullable(Float64) | 10,275 | 零值 4；负值 120,949 | min=-195,862, max=1,043,860, distinct 287,855 | 归属于母公司股东的净利润同比增长率（%） |
| MINORITY_INTEREST | Nullable(Float64) | 60,428 | 零值 1,287；负值 96,649 | min=-31,772,300,000, max=26,322,800,000, distinct 234,529 | 少数股东损益 |
| MINORITY_INTEREST_YOY | Nullable(Float64) | 79,749 | 零值 94；负值 104,964 | min=-51,176,700,000, max=4,556,350,000, distinct 217,986 | 少数股东损益同比增长率（%） |
| DEDUCT_PARENT_NETPROFIT | Nullable(Float64) | 13,068 | 零值 1；负值 61,469 | min=-85,917,400,000, max=368,126,000,000, distinct 284,912 | 扣除非经常性损益后归属于母公司股东的净利润 |
| DEDUCT_PARENT_NETPROFIT_YOY | Nullable(Float64) | 25,181 | 零值 94；负值 120,115 | min=-371,333,000,000, max=1,429,250,000, distinct 272,903 | 扣非归母净利润同比增长率（%） |
| NETPROFIT_OTHER | Nullable(Float64) | 298,162 | 零值 20；负值 42 | min=-995,427,000, max=790,623,000, distinct 194 | 净利润其他 |
| NETPROFIT_OTHER_YOY | Nullable(Float64) | 298,306 | 零值 4；负值 42 | min=-1,770.34, max=1,883.12, distinct 87 | 净利润其他同比增长率（%） |
| NETPROFIT_BALANCE | Nullable(Float64) | 298,376 | 零值 14；负值 1 | min=-282,719, max=162,569, distinct 6 | 净利润平衡项 |
| NETPROFIT_BALANCE_YOY | Nullable(Float64) | 298,395 | 零值 0；负值 1 | min=-100, max=-100, distinct 1 | 净利润平衡项同比增长率（%） |
| BASIC_EPS | Nullable(Float64) | 19,814 | 零值 445；负值 44,772 | min=-24.7834, max=68.64, distinct 18,118 | 基本每股收益（元/股） |
| BASIC_EPS_YOY | Nullable(Float64) | 34,137 | 零值 7,410；负值 130,232 | min=-138,467, max=639,900, distinct 106,487 | 基本每股收益（元/股）同比增长率（%） |
| DILUTED_EPS | Nullable(Float64) | 40,466 | 零值 484；负值 42,379 | min=-24.7834, max=68.64, distinct 17,620 | 稀释每股收益（元/股） |
| DILUTED_EPS_YOY | Nullable(Float64) | 54,530 | 零值 6,977；负值 120,066 | min=-138,467, max=639,900, distinct 98,944 | 稀释每股收益（元/股）同比增长率（%） |
| OTHER_COMPRE_INCOME | Nullable(Float64) | 161,561 | 零值 2,523；负值 69,736 | min=-76,069,000,000, max=61,293,000,000, distinct 131,545 | 其他综合收益总额 |
| OTHER_COMPRE_INCOME_YOY | Nullable(Float64) | 179,714 | 零值 87；负值 60,334 | min=-13,970,700,000, max=197,158,000,000, distinct 117,532 | 其他综合收益总额同比增长率（%） |
| PARENT_OCI | Nullable(Float64) | 179,917 | 零值 1,109；负值 60,150 | min=-75,951,000,000, max=61,293,000,000, distinct 115,399 | 归属于母公司股东的其他综合收益 |
| PARENT_OCI_YOY | Nullable(Float64) | 195,758 | 零值 59；负值 52,451 | min=-13,970,700,000, max=197,158,000,000, distinct 101,911 | 归母其他综合收益同比增长率（%） |
| MINORITY_OCI | Nullable(Float64) | 256,270 | 零值 1,934；负值 20,565 | min=-6,758,000,000, max=7,947,000,000, distinct 38,853 | 归属于少数股东的其他综合收益 |
| MINORITY_OCI_YOY | Nullable(Float64) | 265,352 | 零值 27；负值 16,744 | min=-190,589,000, max=213,470,000, distinct 32,599 | 少数股东其他综合收益同比增长率（%） |
| PARENT_OCI_OTHER | Nullable(Float64) | 298,352 | 零值 40；负值 2 | min=-7,041,000,000, max=1,399,000,000, distinct 5 | 归母其他综合收益其他 |
| PARENT_OCI_OTHER_YOY | Nullable(Float64) | 298,395 | 零值 0；负值 1 | min=-1,201.12, max=-1,201.12, distinct 1 | 归母其他综合收益其他同比增长率（%） |
| PARENT_OCI_BALANCE | Nullable(Float64) | 298,357 | 零值 39；负值 0 | min=0, max=0, distinct 1 | 归母其他综合收益平衡项 |
| PARENT_OCI_BALANCE_YOY | Nullable(Float64) | 298,396 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 归母其他综合收益平衡项同比增长率（%） |
| UNABLE_OCI | Nullable(Float64) | 262,036 | 零值 2,707；负值 17,877 | min=-23,813,000,000, max=33,244,000,000, distinct 31,269 | 以后将重分类进损益的其他综合收益 |
| UNABLE_OCI_YOY | Nullable(Float64) | 272,984 | 零值 47；负值 12,232 | min=-1,419,320,000, max=68,777,300, distinct 24,500 | 以后将重分类进损益的其他综合收益同比增长率（%） |
| CREDITRISK_FAIRVALUE_CHANGE | Nullable(Float64) | 296,847 | 零值 1,502；负值 25 | min=-415,000,000, max=458,000,000, distinct 47 | 信用风险引起的公允价值变动 |
| CREDITRISK_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 298,361 | 零值 0；负值 20 | min=-1,050, max=1,533.33, distinct 35 | 信用风险引起的公允价值变动同比增长率（%） |
| OTHERRIGHT_FAIRVALUE_CHANGE | Nullable(Float64) | 268,159 | 零值 1,367；负值 15,385 | min=-67,454,000,000, max=67,947,000,000, distinct 27,022 | 其他权益工具公允价值变动 |
| OTHERRIGHT_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 276,782 | 零值 32；负值 10,364 | min=-1,419,320,000, max=19,383,900, distinct 20,927 | 其他权益工具公允价值变动同比增长率（%） |
| SETUP_PROFIT_CHANGE | Nullable(Float64) | 291,751 | 零值 1,954；负值 2,620 | min=-1,143,750,000, max=1,874,000,000, distinct 4,208 | 重分类调整变动 |
| SETUP_PROFIT_CHANGE_YOY | Nullable(Float64) | 294,713 | 零值 12；负值 1,780 | min=-2,189,100, max=2,238,300, distinct 3,469 | 重分类调整变动同比增长率（%） |
| RIGHTLAW_UNABLE_OCI | Nullable(Float64) | 292,320 | 零值 1,916；负值 2,026 | min=-5,106,660,000, max=4,784,430,000, distinct 3,674 | 权益法下不能重分类的其他综合收益 |
| RIGHTLAW_UNABLE_OCI_YOY | Nullable(Float64) | 295,836 | 零值 8；负值 1,237 | min=-82,571,000, max=2,724,890, distinct 2,377 | 权益法下不能重分类的其他综合收益同比增长率（%） |
| UNABLE_OCI_OTHER | Nullable(Float64) | 296,497 | 零值 1,158；负值 352 | min=-34,412,000,000, max=40,685,000,000, distinct 612 | 不能重分类其他综合收益其他 |
| UNABLE_OCI_OTHER_YOY | Nullable(Float64) | 297,981 | 零值 8；负值 220 | min=-177,204, max=53,233.3, distinct 391 | 不能重分类其他综合收益其他同比增长率（%） |
| UNABLE_OCI_BALANCE | Nullable(Float64) | 298,395 | 零值 0；负值 0 | min=1,666.68, max=1,666.68, distinct 1 | 不能重分类其他综合收益平衡项 |
| UNABLE_OCI_BALANCE_YOY | Nullable(Float64) | 298,396 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 不能重分类其他综合收益平衡项同比增长率（%） |
| ABLE_OCI | Nullable(Float64) | 190,031 | 零值 1,093；负值 54,499 | min=-76,703,000,000, max=56,080,000,000, distinct 105,810 | 以后将重分类进损益的其他综合收益（可重分类） |
| ABLE_OCI_YOY | Nullable(Float64) | 204,386 | 零值 47；负值 48,863 | min=-13,970,700,000, max=197,158,000,000, distinct 93,516 | 以后将重分类进损益的其他综合收益（可重分类）同比增长率（%） |
| RIGHTLAW_ABLE_OCI | Nullable(Float64) | 279,294 | 零值 1,784；负值 8,520 | min=-5,030,000,000, max=7,626,000,000, distinct 16,012 | 权益法下可重分类的其他综合收益 |
| RIGHTLAW_ABLE_OCI_YOY | Nullable(Float64) | 285,064 | 零值 8；负值 6,838 | min=-30,333,800, max=49,960,500, distinct 12,828 | 权益法下可重分类的其他综合收益同比增长率（%） |
| AFA_FAIRVALUE_CHANGE | Nullable(Float64) | 285,786 | 零值 269；负值 7,007 | min=-62,849,000,000, max=52,099,000,000, distinct 11,931 | 可供出售金融资产公允价值变动 |
| AFA_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 289,704 | 零值 5；负值 4,573 | min=-7,199,390, max=14,666,600, distinct 8,467 | 可供出售金融资产公允价值变动同比增长率（%） |
| HMI_AFA | Nullable(Float64) | 297,986 | 零值 358；负值 28 | min=-28,723,000,000, max=17,719,000,000, distinct 50 | 持有有待售资产公允价值变动 |
| HMI_AFA_YOY | Nullable(Float64) | 298,375 | 零值 0；负值 12 | min=-60,904.2, max=2,342, distinct 21 | 持有有待售资产公允价值变动同比增长率（%） |
| CASHFLOW_HEDGE_VALID | Nullable(Float64) | 289,636 | 零值 1,793；负值 3,424 | min=-11,545,000,000, max=19,018,000,000, distinct 6,602 | 现金流量套期有效部分 |
| CASHFLOW_HEDGE_VALID_YOY | Nullable(Float64) | 293,066 | 零值 3；负值 2,603 | min=-1,727,550, max=570,142, distinct 5,199 | 现金流量套期有效部分同比增长率（%） |
| CREDITOR_FAIRVALUE_CHANGE | Nullable(Float64) | 292,112 | 零值 1,539；负值 2,181 | min=-122,252,000,000, max=240,577,000,000, distinct 4,658 | 债权投资公允价值变动 |
| CREDITOR_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 294,654 | 零值 1；负值 1,894 | min=-855,391, max=32,823.8, distinct 3,712 | 债权投资公允价值变动同比增长率（%） |
| CREDITOR_IMPAIRMENT_RESERVE | Nullable(Float64) | 293,488 | 零值 1,521；负值 1,526 | min=-13,711,000,000, max=20,603,700,000, distinct 3,261 | 债权投资减值准备 |
| CREDITOR_IMPAIRMENT_RESERVE_YOY | Nullable(Float64) | 295,719 | 零值 2；负值 1,323 | min=-903,939, max=289,119, distinct 2,649 | 债权投资减值准备同比增长率（%） |
| FINANCE_OCI_AMT | Nullable(Float64) | 296,065 | 零值 1,551；负值 388 | min=-18,955,000,000, max=40,789,000,000, distinct 735 | 金融资产重分类金额 |
| FINANCE_OCI_AMT_YOY | Nullable(Float64) | 297,951 | 零值 0；负值 240 | min=-20,169.6, max=45,859.4, distinct 437 | 金融资产重分类金额同比增长率（%） |
| CONVERT_DIFF | Nullable(Float64) | 204,217 | 零值 863；负值 47,101 | min=-21,549,000,000, max=36,012,000,000, distinct 92,256 | 外币报表折算差额 |
| CONVERT_DIFF_YOY | Nullable(Float64) | 216,013 | 零值 44；负值 43,472 | min=-11,351,100,000, max=61,191,700,000, distinct 82,122 | 外币报表折算差额同比增长率（%） |
| ABLE_OCI_OTHER | Nullable(Float64) | 293,041 | 零值 1,361；负值 1,716 | min=-296,563,000,000, max=86,143,000,000, distinct 3,640 | 可重分类其他综合收益其他 |
| ABLE_OCI_OTHER_YOY | Nullable(Float64) | 295,928 | 零值 16；负值 1,322 | min=-325,964, max=513,723,000, distinct 2,343 | 可重分类其他综合收益其他同比增长率（%） |
| ABLE_OCI_BALANCE | Nullable(Float64) | 298,355 | 零值 40；负值 0 | min=0, max=3,253,760, distinct 2 | 可重分类其他综合收益平衡项 |
| ABLE_OCI_BALANCE_YOY | Nullable(Float64) | 298,396 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 可重分类其他综合收益平衡项同比增长率（%） |
| OCI_OTHER | Nullable(Float64) | 298,342 | 零值 49；负值 2 | min=-1,164,000,000, max=46,000,000, distinct 6 | 其他综合收益其他 |
| OCI_OTHER_YOY | Nullable(Float64) | 298,395 | 零值 0；负值 1 | min=-389.437, max=-389.437, distinct 1 | 其他综合收益其他同比增长率（%） |
| OCI_BALANCE | Nullable(Float64) | 298,357 | 零值 39；负值 0 | min=0, max=0, distinct 1 | 其他综合收益平衡项 |
| OCI_BALANCE_YOY | Nullable(Float64) | 298,396 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他综合收益平衡项同比增长率（%） |
| TOTAL_COMPRE_INCOME | Nullable(Float64) | 53,623 | 零值 18；负值 43,430 | min=-91,215,200,000, max=418,135,000,000, distinct 244,579 | 综合收益总额 |
| TOTAL_COMPRE_INCOME_YOY | Nullable(Float64) | 67,184 | 零值 6；负值 101,139 | min=-11,355,100, max=52,646,600,000, distinct 231,070 | 综合收益总额同比增长率（%） |
| PARENT_TCI | Nullable(Float64) | 55,294 | 零值 23；负值 42,649 | min=-87,939,900,000, max=418,252,000,000, distinct 242,899 | 归属于母公司股东的综合收益总额 |
| PARENT_TCI_YOY | Nullable(Float64) | 69,469 | 零值 10；负值 99,445 | min=-1,013,660, max=981,274, distinct 228,778 | 归母综合收益总额同比增长率（%） |
| MINORITY_TCI | Nullable(Float64) | 101,766 | 零值 1,325；负值 83,426 | min=-32,056,600,000, max=33,772,000,000, distinct 193,577 | 归属于少数股东的综合收益总额 |
| MINORITY_TCI_YOY | Nullable(Float64) | 121,150 | 零值 95；负值 85,904 | min=-51,176,700,000, max=4,556,350,000, distinct 176,721 | 少数股东综合收益总额同比增长率（%） |
| PRECOMBINE_TCI | Nullable(Float64) | 298,329 | 零值 67；负值 0 | min=0, max=0, distinct 1 | 合并前综合收益总额 |
| PRECOMBINE_TCI_YOY | Nullable(Float64) | 298,396 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 合并前综合收益总额同比增长率（%） |
| EFFECT_TCI_BALANCE | Nullable(Float64) | 269,505 | 零值 28,883；负值 3 | min=-27,351,400, max=7,090,080, distinct 8 | 综合收益总额平衡项 |
| EFFECT_TCI_BALANCE_YOY | Nullable(Float64) | 298,396 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 综合收益总额平衡项同比增长率（%） |
| TCI_OTHER | Nullable(Float64) | 298,268 | 零值 67；负值 0 | min=0, max=631,864,000, distinct 56 | 综合收益总额其他 |
| TCI_OTHER_YOY | Nullable(Float64) | 298,347 | 零值 3；负值 28 | min=-95.3425, max=256.945, distinct 47 | 综合收益总额其他同比增长率（%） |
| TCI_BALANCE | Nullable(Float64) | 269,510 | 零值 28,882；负值 2 | min=-18, max=264,235,000, distinct 5 | 综合收益总额平衡项 |
| TCI_BALANCE_YOY | Nullable(Float64) | 298,396 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 综合收益总额平衡项同比增长率（%） |
| ACF_END_INCOME | Nullable(Float64) | 289,772 | 零值 2,043；负值 4,560 | min=-5,911,100,000, max=22,908,000,000, distinct 6,101 | 持续经营终止经营净损益 |
| ACF_END_INCOME_YOY | Nullable(Float64) | 294,197 | 零值 4；负值 1,922 | min=-654,855, max=5,301,800, distinct 4,084 | 持续经营终止经营净损益同比增长率（%） |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 217,079 | 空字符串 0；`1970-01-01` 0 | distinct 7 | 审计意见类型 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 298,396/298,396，供应商前缀 0/298,396，纯数字 0/298,396，空值 0/298,396；`SECURITY_CODE`: canonical 后缀 0/298,396，供应商前缀 0/298,396，纯数字 298,396/298,396，空值 0/298,396
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：`REPORT_DATE`: 1988-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 523 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME`, `SECURITY_TYPE_CODE`, `CURRENCY`
- 取值：`SECURITY_CODE`: `600654`(123), `600653`(123), `600601`(121), `600651`(121), `600610`(120), `000030`(120), `600602`(119), `000501`(119)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(123), `申华控股`(123), `方正科技`(121), `飞乐音响`(121), `中毅达`(120), `富奥股份`(120)；`ORG_CODE`: `10004106`(200), `10004127`(198), `10004293`(154), `10116535`(128), `10003963`(123), `10003964`(123), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(292,603), `证券`(2,826), `银行`(2,449), `保险`(518)；`REPORT_TYPE`: `年报`(77,446), `中报`(74,897), `一季报`(74,512), `三季报`(71,541)；`REPORT_DATE_NAME`: `2026一季报`(5,198), `2025年报`(5,183), `2024一季报`(5,174), `2025一季报`(5,171), `2025三季报`(5,159), `2025中报`(5,150), `2024三季报`(5,127), `2024年报`(5,118)；`SECURITY_TYPE_CODE`: `058001001`(298,373), `058001008`(23)；`CURRENCY`: `CNY`(297,518), `NULL`(878)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 190 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 190 个数值字段执行 min/max、NULL、零值和负值检查；其中 174 个字段出现负值，162 个字段出现零值，112 个字段 NULL 数不低于 80%。 负值字段样例：`NONBUSINESS_INCOME_YOY` 131,209 行(min=-22,094,300)，`BASIC_EPS_YOY` 130,232 行(min=-138,467)，`FINANCE_EXPENSE_YOY` 129,792 行(min=-4,745,010)，`NONBUSINESS_EXPENSE_YOY` 125,739 行(min=-381,496,000)，`INCOME_TAX_YOY` 125,154 行(min=-963,525,000)，`OPERATE_PROFIT_YOY` 124,133 行(min=-2,095,560)，`TOTAL_PROFIT_YOY` 123,232 行(min=-12,062,300)，`NETPROFIT_YOY` 122,314 行(min=-11,355,100)。 高 NULL 字段样例：`ME_RESEARCH_EXPENSE_YOY` 298,396 行，`PARENT_OCI_BALANCE_YOY` 298,396 行，`UNABLE_OCI_BALANCE_YOY` 298,396 行，`ABLE_OCI_BALANCE_YOY` 298,396 行，`OCI_BALANCE_YOY` 298,396 行，`PRECOMBINE_TCI_YOY` 298,396 行，`EFFECT_TCI_BALANCE_YOY` 298,396 行，`TCI_BALANCE_YOY` 298,396 行。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECURITY_CODE` 为 6 位本地代码 | 中 | 298,396/298,396 行为纯数字 | 只作为 `security_local_code`，不可单独推出交易所 | 交易所归属或证券主数据修正延后 |
| 财务数值存在负值 | 低 | 174 个数值字段出现负值 | 负数符合财务科目/调整项可能性，staging 不过滤 | 口径解释和异常阈值延后 |

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

- 行数：298,396。
- 日期 / 分区范围：`REPORT_DATE`: 1988-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 523 行，`1970-01-01` 占位 0 行
- 候选键重复：未发现重复。
- 关键 NULL / 占位值：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`SECURITY_CODE`: `600654`(123), `600653`(123), `600601`(121), `600651`(121), `600610`(120), `000030`(120), `600602`(119), `000501`(119)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(123), `申华控股`(123), `方正科技`(121), `飞乐音响`(121), `中毅达`(120), `富奥股份`(120)；`ORG_CODE`: `10004106`(200), `10004127`(198), `10004293`(154), `10116535`(128), `10003963`(123), `10003964`(123), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(292,603), `证券`(2,826), `银行`(2,449), `保险`(518)；`REPORT_TYPE`: `年报`(77,446), `中报`(74,897), `一季报`(74,512), `三季报`(71,541)；`REPORT_DATE_NAME`: `2026一季报`(5,198), `2025年报`(5,183), `2024一季报`(5,174), `2025一季报`(5,171), `2025三季报`(5,159), `2025中报`(5,150), `2024三季报`(5,127), `2024年报`(5,118)；`SECURITY_TYPE_CODE`: `058001001`(298,373), `058001008`(23)；`CURRENCY`: `CNY`(297,518), `NULL`(878)
- 数值范围：已对 190 个数值字段执行 min/max、NULL、零值和负值检查；其中 174 个字段出现负值，162 个字段出现零值，112 个字段 NULL 数不低于 80%。 负值字段样例：`NONBUSINESS_INCOME_YOY` 131,209 行(min=-22,094,300)，`BASIC_EPS_YOY` 130,232 行(min=-138,467)，`FINANCE_EXPENSE_YOY` 129,792 行(min=-4,745,010)，`NONBUSINESS_EXPENSE_YOY` 125,739 行(min=-381,496,000)，`INCOME_TAX_YOY` 125,154 行(min=-963,525,000)，`OPERATE_PROFIT_YOY` 124,133 行(min=-2,095,560)，`TOTAL_PROFIT_YOY` 123,232 行(min=-12,062,300)，`NETPROFIT_YOY` 122,314 行(min=-11,355,100)。 高 NULL 字段样例：`ME_RESEARCH_EXPENSE_YOY` 298,396 行，`PARENT_OCI_BALANCE_YOY` 298,396 行，`UNABLE_OCI_BALANCE_YOY` 298,396 行，`ABLE_OCI_BALANCE_YOY` 298,396 行，`OCI_BALANCE_YOY` 298,396 行，`PRECOMBINE_TCI_YOY` 298,396 行，`EFFECT_TCI_BALANCE_YOY` 298,396 行，`TCI_BALANCE_YOY` 298,396 行。

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
select `SECUCODE`, `REPORT_DATE`, `SECURITY_CODE`, `NOTICE_DATE`, `UPDATE_DATE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME` from fleur_raw.eastmoney__income_ytd limit 5
```

结果：

```text
[{'SECUCODE': '600601.SH', 'REPORT_DATE': datetime.date(1990, 6, 30), 'SECURITY_CODE': '600601', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '方正科技', 'ORG_CODE': '10002659', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1990中报'}, {'SECUCODE': '600601.SH', 'REPORT_DATE': datetime.date(1991, 6, 30), 'SECURITY_CODE': '600601', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '方正科技', 'ORG_CODE': '10002659', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1991中报'}, {'SECUCODE': '600602.SH', 'REPORT_DATE': datetime.date(1990, 6, 30), 'SECURITY_CODE': '600602', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '云赛智联', 'ORG_CODE': '10002660', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1990中报'}, {'SECUCODE': '600602.SH', 'REPORT_DATE': datetime.date(1991, 6, 30), 'SECURITY_CODE': '600602', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '云赛智联', 'ORG_CODE': '10002660', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1991中报'}, {'SECUCODE': '600651.SH', 'REPORT_DATE': datetime.date(1990, 6, 30), 'SECURITY_CODE': '600651', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '飞乐音响', 'ORG_CODE': '10003961', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1990中报'}]
```

### 行数统计

```sql
select count() from fleur_raw.eastmoney__income_ytd
```

结果：

```text
[[298396]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `SECUCODE`, `REPORT_DATE`, count() as row_count from fleur_raw.eastmoney__income_ytd group by `SECUCODE`, `REPORT_DATE` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 0, 'max_rows_per_key': 0}
```

### 证券代码格式：SECUCODE

```sql
select countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__income_ytd
```

结果：

```text
{'canonical_suffix': 298396, 'vendor_prefix': 0, 'numeric_only': 0, 'empty_or_null': 0, 'row_count': 298396}
```

### 证券代码格式：SECURITY_CODE

```sql
select countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__income_ytd
```

结果：

```text
{'canonical_suffix': 0, 'vendor_prefix': 0, 'numeric_only': 298396, 'empty_or_null': 0, 'row_count': 298396}
```
