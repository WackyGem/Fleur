# Raw 数据画像：eastmoney__balance

日期：2026-06-03

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__balance.yml`
- dbt source：`source('raw', 'eastmoney__balance')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__balance.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__balance`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__balance --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__balance.md`
- 行数：284,265
- 数据范围：`REPORT_DATE`: 1989-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 976 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`eastmoney__balance`
- ClickHouse raw 表：`fleur_raw.eastmoney__balance`
- 表说明：EastMoney balance sheet F10 rows by natural-year raw partition.

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：284,265。
  - 覆盖主体数：`secucode` 5,421 个；`security_code` 5,421 个
  - 日期 / 分区范围：`REPORT_DATE`: 1989-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 976 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `SECUCODE`, `REPORT_DATE`。
  - 候选自然键去重结果：未发现重复。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：`SECUCODE`: canonical 后缀 284,265/284,265，供应商前缀 0/284,265，纯数字 0/284,265，空值 0/284,265；`SECURITY_CODE`: canonical 后缀 0/284,265，供应商前缀 0/284,265，纯数字 284,265/284,265，空值 0/284,265
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`SECURITY_CODE`: `600654`(122), `600653`(122), `600651`(121), `600601`(121), `600610`(120), `600602`(120), `000501`(118), `600605`(118)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(122), `申华控股`(122), `飞乐音响`(121), `方正科技`(121), `云赛智联`(120), `中毅达`(120)；`ORG_CODE`: `10004106`(198), `10004127`(198), `10004293`(157), `10003964`(122), `10003963`(122), `10116535`(122), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(284,265)；`REPORT_TYPE`: `年报`(75,980), `中报`(71,407), `一季报`(69,943), `三季报`(66,935)；`REPORT_DATE_NAME`: `2026一季报`(5,099), `2025年报`(5,084), `2025三季报`(5,060), `2025中报`(5,051), `2025一季报`(5,042), `2024年报`(5,020), `2024一季报`(5,009), `2024三季报`(5,005)；`SECURITY_TYPE_CODE`: `058001001`(284,243), `058001008`(22)；`CURRENCY`: `CNY`(283,335), `NULL`(930)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`REPORT_DATE`: 1989-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 976 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 304 个数值字段执行 min/max、NULL、零值和负值检查；其中 233 个字段出现负值，270 个字段出现零值，179 个字段 NULL 数不低于 80%。 负值字段样例：`INTANGIBLE_ASSET_YOY` 140,706 行(min=-369.667)，`MONETARYFUNDS_YOY` 125,171 行(min=-100)，`TOTAL_OTHER_RECE_YOY` 124,568 行(min=-904.73)，`PREPAYMENT_YOY` 124,546 行(min=-8,849.35)，`TAX_PAYABLE_YOY` 122,687 行(min=-2,542,390)，`TOTAL_OTHER_PAYABLE_YOY` 118,538 行(min=-2,083.8)，`TOTAL_NONCURRENT_LIAB_YOY` 112,391 行(min=-88,422.9)，`FIXED_ASSET_YOY` 110,256 行(min=-100)。 高 NULL 字段样例：`AMORTIZE_COST_FINASSET` 284,265 行，`AMORTIZE_COST_FINLIAB` 284,265 行，`AMORTIZE_COST_NCFINLIAB` 284,265 行，`APPOINT_FVTPL_FINLIAB` 284,265 行，`AMORTIZE_COST_FINASSET_YOY` 284,265 行，`AMORTIZE_COST_FINLIAB_YOY` 284,265 行，`AMORTIZE_COST_NCFINASSET_YOY` 284,265 行，`AMORTIZE_COST_NCFINLIAB_YOY` 284,265 行。
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
| SECUCODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,421 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,421 | 证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,419 | 证券简称 |
| ORG_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,414 | 机构代码 |
| ORG_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 机构类型 |
| REPORT_DATE | Date | 0 | `1970-01-01` 0 | 1989-12-31 至 2026-03-31; distinct 126 | 报告期 |
| REPORT_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 4 | 报告类型 |
| REPORT_DATE_NAME | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 126 | 报告期名称 |
| SECURITY_TYPE_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 2 | 证券类型代码 |
| NOTICE_DATE | Date | 0 | `1970-01-01` 0 | 1991-06-10 至 2026-05-15; distinct 4,334 | 公告日期 |
| UPDATE_DATE | Nullable(Date) | 976 | `1970-01-01` 0 | 1991-06-10 至 2026-06-02; distinct 5,223 | 更新日期 |
| CURRENCY | LowCardinality(Nullable(String)) | 930 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 资产负债表披露金额使用的币种。 |
| ACCEPT_DEPOSIT_INTERBANK | Nullable(Float64) | 262,378 | 零值 20,147；负值 0 | min=0, max=777,003,000,000, distinct 1,737 | 同业存放 |
| ACCOUNTS_PAYABLE | Nullable(Float64) | 9,731 | 零值 99；负值 20 | min=-44,288,800, max=997,478,000,000, distinct 273,513 | 应付账款 |
| ACCOUNTS_RECE | Nullable(Float64) | 7,279 | 零值 177；负值 24 | min=-1,878,640,000, max=442,287,000,000, distinct 276,023 | 应收账款 |
| ACCRUED_EXPENSE | Nullable(Float64) | 283,529 | 零值 3；负值 0 | min=0, max=3,512,820,000, distinct 727 | 预提费用 |
| ADVANCE_RECEIVABLES | Nullable(Float64) | 82,991 | 零值 2,088；负值 44 | min=-36,425,000, max=407,882,000,000, distinct 194,526 | 预收款项 |
| AGENT_TRADE_SECURITY | Nullable(Float64) | 281,012 | 零值 2,729；负值 0 | min=0, max=181,415,000,000, distinct 487 | 代理买卖证券款 |
| AGENT_UNDERWRITE_SECURITY | Nullable(Float64) | 281,482 | 零值 2,734；负值 0 | min=0, max=1,703,950,000, distinct 50 | 代理承销证券款 |
| AMORTIZE_COST_FINASSET | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 以摊余成本计量的金融资产 |
| AMORTIZE_COST_FINLIAB | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 以摊余成本计量的金融负债 |
| AMORTIZE_COST_NCFINASSET | Nullable(Float64) | 284,261 | 零值 0；负值 0 | min=49,984,200, max=2,144,660,000, distinct 4 | 非流动金融资产（摊余成本） |
| AMORTIZE_COST_NCFINLIAB | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 非流动金融负债（摊余成本） |
| APPOINT_FVTPL_FINASSET | Nullable(Float64) | 284,264 | 零值 0；负值 0 | min=13,410,800, max=13,410,800, distinct 1 | 指定为FVTPL的金融资产 |
| APPOINT_FVTPL_FINLIAB | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 指定为FVTPL的金融负债 |
| ASSET_BALANCE | Nullable(Float64) | 44,759 | 零值 237,785；负值 465 | min=-4,078,230,000, max=26,949,900,000, distinct 1,257 | 资产平衡项 |
| ASSET_OTHER | Nullable(Float64) | 266,451 | 零值 17,786；负值 0 | min=0, max=12,133,100,000, distinct 29 | 资产其他项 |
| ASSIGN_CASH_DIVIDEND | Nullable(Float64) | 282,915 | 零值 1,307；负值 0 | min=0, max=687,508,000, distinct 33 | 应付现金股利 |
| AVAILABLE_SALE_FINASSET | Nullable(Float64) | 231,660 | 零值 2,276；负值 0 | min=0, max=69,957,700,000, distinct 29,367 | 可供出售金融资产 |
| BOND_PAYABLE | Nullable(Float64) | 241,016 | 零值 4,223；负值 10 | min=-3,680,330, max=141,620,000,000, distinct 34,211 | 应付债券 |
| BORROW_FUND | Nullable(Float64) | 280,563 | 零值 2,829；负值 0 | min=0, max=82,579,600,000, distinct 617 | 拆入资金 |
| BUY_RESALE_FINASSET | Nullable(Float64) | 280,146 | 零值 2,952；负值 0 | min=0, max=71,757,300,000, distinct 1,116 | 买入返售金融资产 |
| CAPITAL_RESERVE | Nullable(Float64) | 1,774 | 零值 72；负值 750 | min=-308,692,000,000, max=157,127,000,000, distinct 143,535 | 资本公积 |
| CIP | Nullable(Float64) | 41,659 | 零值 885；负值 14 | min=-146,930,000, max=323,105,000,000, distinct 235,473 | 在建工程 |
| CONSUMPTIVE_BIOLOGICAL_ASSET | Nullable(Float64) | 284,239 | 零值 0；负值 0 | min=19,514,400, max=1,376,030,000, distinct 26 | 消耗性生物资产 |
| CONTRACT_ASSET | Nullable(Float64) | 242,140 | 零值 1,417；负值 0 | min=0, max=622,519,000,000, distinct 38,642 | 合同资产 |
| CONTRACT_LIAB | Nullable(Float64) | 170,202 | 零值 60；负值 3 | min=-1,258,860, max=697,730,000,000, distinct 113,679 | 合同负债 |
| CONVERT_DIFF | Nullable(Float64) | 265,644 | 零值 1,726；负值 12,410 | min=-15,829,000,000, max=1,398,780,000, distinct 15,863 | 外币报表折算差额 |
| CREDITOR_INVEST | Nullable(Float64) | 275,773 | 零值 1,545；负值 1 | min=-205,555, max=65,959,100,000, distinct 5,556 | 债权投资 |
| CURRENT_ASSET_BALANCE | Nullable(Float64) | 44,997 | 零值 223,643；负值 386 | min=-2,195,670,000, max=12,548,800,000, distinct 14,300 | 流动资产平衡项 |
| CURRENT_ASSET_OTHER | Nullable(Float64) | 186,726 | 零值 67,076；负值 39 | min=-4,046,300, max=82,437,500,000, distinct 29,860 | 流动资产其他项 |
| CURRENT_LIAB_BALANCE | Nullable(Float64) | 44,888 | 零值 235,685；负值 388 | min=-45,559,800,000, max=8,163,310,000, distinct 3,210 | 流动负债平衡项 |
| CURRENT_LIAB_OTHER | Nullable(Float64) | 174,418 | 零值 79,915；负值 59 | min=-29,905,100, max=26,441,400,000, distinct 29,062 | 流动负债其他项 |
| DEFER_INCOME | Nullable(Float64) | 123,590 | 零值 1,346；负值 70 | min=-1,555,220,000, max=27,652,300,000, distinct 147,959 | 递延收益 |
| DEFER_INCOME_1YEAR | Nullable(Float64) | 283,293 | 零值 671；负值 0 | min=0, max=1,761,900,000, distinct 266 | 一年内到期递延收益 |
| DEFER_TAX_ASSET | Nullable(Float64) | 45,049 | 零值 844；负值 29 | min=-11,555,200, max=54,333,000,000, distinct 218,953 | 递延所得税资产 |
| DEFER_TAX_LIAB | Nullable(Float64) | 122,721 | 零值 2,620；负值 63 | min=-360,902,000, max=39,839,100,000, distinct 140,025 | 递延所得税负债 |
| DERIVE_FINASSET | Nullable(Float64) | 275,103 | 零值 2,004；负值 5 | min=-2,530,000, max=43,768,000,000, distinct 6,964 | 衍生金融资产 |
| DERIVE_FINLIAB | Nullable(Float64) | 275,212 | 零值 1,988；负值 0 | min=0, max=39,213,000,000, distinct 6,895 | 衍生金融负债 |
| DEVELOP_EXPENSE | Nullable(Float64) | 232,998 | 零值 3,470；负值 0 | min=0, max=12,953,400,000, distinct 43,861 | 开发支出 |
| DIV_HOLDSALE_ASSET | Nullable(Float64) | 283,015 | 零值 306；负值 1 | min=-46,054,600, max=37,098,100,000, distinct 541 | 持有待售资产（除） |
| DIV_HOLDSALE_LIAB | Nullable(Float64) | 283,891 | 零值 267；负值 0 | min=0, max=28,454,500,000, distinct 88 | 持有待售负债（除） |
| DIVIDEND_PAYABLE | Nullable(Float64) | 172,870 | 零值 3,470；负值 101 | min=-32,000,000, max=60,660,000,000, distinct 56,007 | 应付股利 |
| DIVIDEND_RECE | Nullable(Float64) | 250,448 | 零值 4,873；负值 17 | min=-37,811,300, max=19,040,000,000, distinct 15,589 | 应收股利 |
| EQUITY_BALANCE | Nullable(Float64) | 81,355 | 零值 202,708；负值 95 | min=-30,172,500, max=414,268,000, distinct 38 | 所有者权益平衡项 |
| EQUITY_OTHER | Nullable(Float64) | 270,176 | 零值 14,078；负值 1 | min=-3,276,280,000, max=9,321,330,000, distinct 11 | 所有者权益其他项 |
| EXPORT_REFUND_RECE | Nullable(Float64) | 283,540 | 零值 716；负值 0 | min=0, max=24,593,100, distinct 10 | 应收出口退税 |
| FEE_COMMISSION_PAYABLE | Nullable(Float64) | 281,119 | 零值 2,803；负值 0 | min=0, max=521,634,000, distinct 334 | 应付手续费及佣金 |
| FIN_FUND | Nullable(Float64) | 284,018 | 零值 0；负值 0 | min=611,054,000, max=81,271,500,000, distinct 246 | 金融往来资金 |
| FINANCE_RECE | Nullable(Float64) | 206,148 | 零值 745；负值 0 | min=0, max=56,261,900,000, distinct 74,948 | 金融应收款 |
| FIXED_ASSET | Nullable(Float64) | 8,923 | 零值 29；负值 0 | min=0, max=770,264,000,000, distinct 275,046 | 固定资产 |
| FIXED_ASSET_DISPOSAL | Nullable(Float64) | 262,005 | 零值 3,880；负值 2,859 | min=-96,559,200, max=2,026,600,000, distinct 15,110 | 固定资产清理 |
| FVTOCI_FINASSET | Nullable(Float64) | 284,263 | 零值 0；负值 0 | min=667,920,000, max=43,096,700,000, distinct 2 | 以公允价值计量且其变动计入其他综合收益的金融资产 |
| FVTOCI_NCFINASSET | Nullable(Float64) | 284,227 | 零值 0；负值 0 | min=17,936.4, max=25,769,800,000, distinct 37 | 其他非流动金融资产（FVTOCI） |
| FVTPL_FINASSET | Nullable(Float64) | 272,790 | 零值 1,103；负值 12 | min=-10,076,800, max=54,908,900,000, distinct 9,653 | 以公允价值计量且其变动计入当期损益的金融资产 |
| FVTPL_FINLIAB | Nullable(Float64) | 277,763 | 零值 409；负值 11 | min=-26,371,600, max=9,137,340,000, distinct 5,653 | 以公允价值计量且其变动计入当期损益的金融负债 |
| GENERAL_RISK_RESERVE | Nullable(Float64) | 276,595 | 零值 3,362；负值 6 | min=-370,451,000, max=8,358,040,000, distinct 1,697 | 一般风险准备 |
| GOODWILL | Nullable(Float64) | 163,908 | 零值 2,309；负值 4 | min=-1,367,020, max=46,097,000,000, distinct 34,023 | 资产负债表披露的商誉金额。 |
| HOLD_MATURITY_INVEST | Nullable(Float64) | 277,149 | 零值 2,457；负值 0 | min=0, max=14,683,900,000, distinct 1,814 | 持有至到期投资 |
| HOLDSALE_ASSET | Nullable(Float64) | 276,693 | 零值 1,911；负值 1 | min=-24,274, max=83,367,000,000, distinct 3,455 | 持有待售资产 |
| HOLDSALE_LIAB | Nullable(Float64) | 281,531 | 零值 1,738；负值 0 | min=0, max=63,423,300,000, distinct 866 | 持有待售负债 |
| INSURANCE_CONTRACT_RESERVE | Nullable(Float64) | 281,565 | 零值 2,422；负值 0 | min=0, max=9,805,490,000, distinct 266 | 保险合同准备金 |
| INTANGIBLE_ASSET | Nullable(Float64) | 10,013 | 零值 338；负值 22 | min=-1,553,460,000, max=298,731,000,000, distinct 272,870 | 无形资产 |
| INTEREST_PAYABLE | Nullable(Float64) | 194,600 | 零值 3,696；负值 79 | min=-9,459,630, max=24,408,300,000, distinct 79,913 | 应付利息 |
| INTEREST_RECE | Nullable(Float64) | 235,111 | 零值 4,677；负值 12 | min=-2,903,800, max=6,571,820,000, distinct 41,440 | 应收利息 |
| INTERNAL_PAYABLE | Nullable(Float64) | 283,503 | 零值 758；负值 0 | min=0, max=3,573,070,000, distinct 5 | 内部应付款 |
| INTERNAL_RECE | Nullable(Float64) | 283,262 | 零值 997；负值 0 | min=0, max=651,999,000, distinct 7 | 内部应收款 |
| INVENTORY | Nullable(Float64) | 5,662 | 零值 221；负值 1 | min=-3,922,930, max=1,112,920,000,000, distinct 277,448 | 资产负债表披露的存货金额。 |
| INVEST_REALESTATE | Nullable(Float64) | 167,097 | 零值 2,543；负值 0 | min=0, max=159,810,000,000, distinct 110,185 | 投资性房地产 |
| LEASE_LIAB | Nullable(Float64) | 199,900 | 零值 391；负值 3 | min=-2,004,760, max=182,765,000,000, distinct 82,642 | 租赁负债 |
| LEND_FUND | Nullable(Float64) | 280,956 | 零值 2,875；负值 0 | min=0, max=298,704,000,000, distinct 362 | 拆出资金 |
| LIAB_BALANCE | Nullable(Float64) | 45,041 | 零值 238,768；负值 161 | min=-1,103,060,000, max=2,561,270,000, distinct 279 | 负债平衡项 |
| LIAB_EQUITY_BALANCE | Nullable(Float64) | 246,371 | 零值 36,878；负值 378 | min=-12,359,400,000, max=21,824,400,000, distinct 675 | 负债和所有者权益平衡项 |
| LIAB_EQUITY_OTHER | Nullable(Float64) | 284,171 | 零值 91；负值 1 | min=-3,689,310, max=179,260,000, distinct 4 | 负债和所有者权益其他项 |
| LIAB_OTHER | Nullable(Float64) | 266,450 | 零值 17,785；负值 0 | min=0, max=7,129,020,000, distinct 31 | 负债其他项 |
| LOAN_ADVANCE | Nullable(Float64) | 276,654 | 零值 3,520；负值 0 | min=0, max=230,254,000,000, distinct 3,789 | 发放贷款及垫款 |
| LOAN_PBC | Nullable(Float64) | 281,180 | 零值 2,819；负值 0 | min=0, max=9,594,390,000, distinct 249 | 向央行借款 |
| LONG_EQUITY_INVEST | Nullable(Float64) | 79,730 | 零值 2,125；负值 481 | min=-254,044,000, max=307,149,000,000, distinct 177,748 | 长期股权投资 |
| LONG_LOAN | Nullable(Float64) | 120,059 | 零值 3,137；负值 2 | min=-30,000,000, max=547,242,000,000, distinct 106,261 | 长期借款 |
| LONG_PAYABLE | Nullable(Float64) | 191,070 | 零值 4,209；负值 303 | min=-9,115,720,000, max=99,762,700,000, distinct 64,452 | 长期应付款 |
| LONG_PREPAID_EXPENSE | Nullable(Float64) | 55,422 | 零值 1,572；负值 75 | min=-24,444,500, max=28,727,000,000, distinct 224,531 | 长期待摊费用 |
| LONG_RECE | Nullable(Float64) | 235,707 | 零值 5,008；负值 1 | min=-970,744, max=292,760,000,000, distinct 35,516 | 长期应收款 |
| LONG_STAFFSALARY_PAYABLE | Nullable(Float64) | 251,474 | 零值 9,665；负值 4 | min=-31,059.6, max=14,200,700,000, distinct 18,581 | 长期应付职工薪酬 |
| MINORITY_EQUITY | Nullable(Float64) | 54,934 | 零值 1,356；负值 15,676 | min=-5,983,760,000, max=333,301,000,000, distinct 226,049 | 少数股东权益 |
| MONETARYFUNDS | Nullable(Float64) | 1,996 | 零值 84；负值 0 | min=0, max=424,426,000,000, distinct 281,889 | 货币资金 |
| NONCURRENT_ASSET_1YEAR | Nullable(Float64) | 238,552 | 零值 5,254；负值 9 | min=-261,000,000, max=298,827,000,000, distinct 32,817 | 一年内到期的非流动资产 |
| NONCURRENT_ASSET_BALANCE | Nullable(Float64) | 45,081 | 零值 223,044；负值 317 | min=-2,000,000,000, max=33,503,200,000, distinct 12,417 | 非流动资产平衡项 |
| NONCURRENT_ASSET_OTHER | Nullable(Float64) | 251,148 | 零值 32,250；负值 3 | min=-80,973,300, max=55,098,700,000, distinct 668 | 非流动资产其他项 |
| NONCURRENT_LIAB_1YEAR | Nullable(Float64) | 105,793 | 零值 2,718；负值 10 | min=-310,000,000, max=146,046,000,000, distinct 137,777 | 一年内到期的非流动负债 |
| NONCURRENT_LIAB_BALANCE | Nullable(Float64) | 53,275 | 零值 226,723；负值 779 | min=-1,182,570,000, max=14,700,000,000, distinct 3,920 | 非流动负债平衡项 |
| NONCURRENT_LIAB_OTHER | Nullable(Float64) | 251,377 | 零值 32,058；负值 1 | min=-6,781,540, max=108,051,000,000, distinct 819 | 非流动负债其他项 |
| NOTE_ACCOUNTS_PAYABLE | Nullable(Float64) | 3,104 | 零值 5；负值 14 | min=-14,103,400, max=1,009,140,000,000, distinct 280,226 | 应付票据及应付账款 |
| NOTE_ACCOUNTS_RECE | Nullable(Float64) | 5,835 | 零值 13；负值 20 | min=-274,977,000, max=446,957,000,000, distinct 277,274 | 应收票据及应收账款 |
| NOTE_PAYABLE | Nullable(Float64) | 102,509 | 零值 2,295；负值 5 | min=-600,000, max=126,229,000,000, distinct 152,797 | 应付票据 |
| NOTE_RECE | Nullable(Float64) | 77,827 | 零值 1,909；负值 8 | min=-1,500,000, max=51,604,400,000, distinct 185,922 | 应收票据 |
| OIL_GAS_ASSET | Nullable(Float64) | 278,404 | 零值 5,008；负值 0 | min=0, max=880,482,000,000, distinct 841 | 油气资产 |
| OTHER_COMPRE_INCOME | Nullable(Float64) | 157,528 | 零值 5,325；负值 55,976 | min=-39,247,000,000, max=20,167,000,000, distinct 108,526 | 其他综合收益 |
| OTHER_CREDITOR_INVEST | Nullable(Float64) | 280,273 | 零值 1,631；负值 0 | min=0, max=74,969,300,000, distinct 2,067 | 其他债权投资 |
| OTHER_CURRENT_ASSET | Nullable(Float64) | 77,606 | 零值 2,888；负值 219 | min=-269,730,000, max=215,348,000,000, distinct 198,475 | 其他流动资产 |
| OTHER_CURRENT_LIAB | Nullable(Float64) | 135,900 | 零值 4,277；负值 126 | min=-120,087,000, max=156,725,000,000, distinct 134,509 | 其他流动负债 |
| OTHER_EQUITY_INVEST | Nullable(Float64) | 219,823 | 零值 970；负值 3 | min=-5,167,170, max=33,849,500,000, distinct 33,048 | 其他权益工具投资 |
| OTHER_EQUITY_OTHER | Nullable(Float64) | 275,303 | 零值 8,955；负值 0 | min=0, max=292,088,000, distinct 8 | 其他权益其他项 |
| OTHER_EQUITY_TOOL | Nullable(Float64) | 258,526 | 零值 10,819；负值 338 | min=-1,367,620,000, max=80,170,700,000, distinct 10,576 | 其他权益工具 |
| OTHER_NONCURRENT_ASSET | Nullable(Float64) | 119,116 | 零值 4,347；负值 21 | min=-3,599,680, max=368,737,000,000, distinct 145,001 | 其他非流动资产 |
| OTHER_NONCURRENT_FINASSET | Nullable(Float64) | 238,590 | 零值 1,090；负值 0 | min=0, max=257,299,000,000, distinct 25,709 | 其他非流动金融资产 |
| OTHER_NONCURRENT_LIAB | Nullable(Float64) | 213,649 | 零值 5,664；负值 334 | min=-560,794,000, max=80,798,100,000, distinct 47,929 | 其他非流动负债 |
| OTHER_PAYABLE | Nullable(Float64) | 137,314 | 零值 90；负值 45 | min=-114,160,000, max=182,887,000,000, distinct 146,504 | 其他应付款 |
| OTHER_RECE | Nullable(Float64) | 144,134 | 零值 102；负值 17 | min=-149,813,000, max=240,938,000,000, distinct 139,688 | 其他应收款 |
| PARENT_EQUITY_BALANCE | Nullable(Float64) | 44,784 | 零值 237,038；负值 494 | min=-3,072,020,000, max=14,301,300,000, distinct 1,696 | 归母权益平衡项 |
| PARENT_EQUITY_OTHER | Nullable(Float64) | 252,177 | 零值 31,530；负值 216 | min=-2,884,180,000, max=6,159,150,000, distinct 543 | 归母权益其他项 |
| PERPETUAL_BOND | Nullable(Float64) | 270,602 | 零值 10,522；负值 0 | min=0, max=80,170,700,000, distinct 1,217 | 永续债 |
| PERPETUAL_BOND_PAYBALE | Nullable(Float64) | 273,695 | 零值 10,478；负值 1 | min=-2,029,740, max=14,808,200,000, distinct 70 | 永续债（负债端） |
| PREDICT_CURRENT_LIAB | Nullable(Float64) | 282,761 | 零值 953；负值 0 | min=0, max=5,776,670,000, distinct 473 | 预计流动负债 |
| PREDICT_LIAB | Nullable(Float64) | 205,267 | 零值 2,890；负值 13 | min=-218,717,000, max=174,066,000,000, distinct 55,929 | 预计负债 |
| PREFERRED_SHARES | Nullable(Float64) | 273,368 | 零值 10,638；负值 0 | min=0, max=15,972,200,000, distinct 32 | 优先股 |
| PREFERRED_SHARES_PAYBALE | Nullable(Float64) | 273,665 | 零值 10,510；负值 0 | min=0, max=2,630,570,000, distinct 64 | 应付优先股 |
| PREMIUM_RECE | Nullable(Float64) | 280,152 | 零值 3,850；负值 1 | min=-225,823,000, max=8,970,770,000, distinct 263 | 预收保费 |
| PREPAYMENT | Nullable(Float64) | 4,210 | 零值 165；负值 16 | min=-116,605,000, max=121,723,000,000, distinct 278,728 | 预付款项 |
| PRODUCTIVE_BIOLOGY_ASSET | Nullable(Float64) | 271,116 | 零值 4,753；负值 0 | min=0, max=11,797,600,000, distinct 7,841 | 生产性生物资产 |
| PROJECT_MATERIAL | Nullable(Float64) | 249,764 | 零值 2,690；负值 36 | min=-276,129,000, max=18,382,600,000, distinct 27,627 | 工程物资 |
| RC_RESERVE_RECE | Nullable(Float64) | 280,326 | 零值 3,797；负值 1 | min=-175,084,000, max=6,507,770,000, distinct 143 | 再保合同应收准备金 |
| REINSURE_PAYABLE | Nullable(Float64) | 280,310 | 零值 3,753；负值 0 | min=0, max=6,283,930,000, distinct 201 | 应付再保款 |
| REINSURE_RECE | Nullable(Float64) | 280,253 | 零值 3,846；负值 1 | min=-10,454,200, max=3,460,700,000, distinct 167 | 应收再保款 |
| SELL_REPO_FINASSET | Nullable(Float64) | 279,282 | 零值 3,933；负值 0 | min=0, max=61,115,100,000, distinct 996 | 卖出回购金融资产款 |
| SETTLE_EXCESS_RESERVE | Nullable(Float64) | 279,608 | 零值 3,925；负值 0 | min=0, max=51,034,300,000, distinct 720 | 清算备付金 |
| SHARE_CAPITAL | Nullable(Float64) | 34 | 零值 17；负值 0 | min=0, max=469,079,000,000, distinct 45,118 | 实收资本（股本） |
| SHORT_BOND_PAYABLE | Nullable(Float64) | 282,128 | 零值 1,642；负值 0 | min=0, max=38,000,000,000, distinct 288 | 短期应付债券 |
| SHORT_FIN_PAYABLE | Nullable(Float64) | 283,962 | 零值 4；负值 0 | min=0, max=34,058,800,000, distinct 250 | 短期金融负债 |
| SHORT_LOAN | Nullable(Float64) | 56,121 | 零值 1,760；负值 0 | min=0, max=223,507,000,000, distinct 154,630 | 短期借款 |
| SPECIAL_PAYABLE | Nullable(Float64) | 248,513 | 零值 3,239；负值 194 | min=-6,992,490,000, max=6,248,240,000, distinct 15,292 | 专项应付款 |
| SPECIAL_RESERVE | Nullable(Float64) | 223,712 | 零值 3,615；负值 111 | min=-74,208,100, max=26,225,000,000, distinct 53,617 | 专项储备 |
| STAFF_SALARY_PAYABLE | Nullable(Float64) | 3,792 | 零值 376；负值 1,094 | min=-70,739,900, max=35,988,000,000, distinct 279,138 | 应付职工薪酬 |
| SUBSIDY_RECE | Nullable(Float64) | 282,000 | 零值 1,966；负值 0 | min=0, max=590,079,000, distinct 286 | 应收补贴 |
| SURPLUS_RESERVE | Nullable(Float64) | 4,170 | 零值 124；负值 6 | min=-1,092,440,000, max=266,528,000,000, distinct 75,744 | 盈余公积 |
| TAX_PAYABLE | Nullable(Float64) | 2,143 | 零值 108；负值 24,460 | min=-18,410,400,000, max=119,740,000,000, distinct 281,679 | 应交税费 |
| TOTAL_ASSETS | Nullable(Float64) | 1 | 零值 5；负值 0 | min=0, max=3,613,670,000,000, distinct 283,965 | 资产总计 |
| TOTAL_CURRENT_ASSETS | Nullable(Float64) | 250 | 零值 22；负值 0 | min=0, max=2,652,780,000,000, distinct 283,696 | 流动资产合计 |
| TOTAL_CURRENT_LIAB | Nullable(Float64) | 250 | 零值 22；负值 14 | min=-111,474,000, max=2,078,120,000,000, distinct 283,702 | 流动负债合计 |
| TOTAL_EQUITY | Nullable(Float64) | 5 | 零值 17；负值 3,327 | min=-31,171,700,000, max=1,835,770,000,000, distinct 283,957 | 所有者权益合计 |
| TOTAL_LIAB_EQUITY | Nullable(Float64) | 8 | 零值 5；负值 0 | min=0, max=3,613,670,000,000, distinct 283,958 | 负债和所有者权益总计 |
| TOTAL_LIABILITIES | Nullable(Float64) | 241 | 零值 30；负值 18 | min=-444,401,000, max=2,775,300,000,000, distinct 283,703 | 负债合计 |
| TOTAL_NONCURRENT_ASSETS | Nullable(Float64) | 301 | 零值 34；负值 2 | min=-2,274,580, max=2,287,700,000,000, distinct 283,607 | 非流动资产合计 |
| TOTAL_NONCURRENT_LIAB | Nullable(Float64) | 12,934 | 零值 1,642；负值 285 | min=-6,135,950,000, max=697,180,000,000, distinct 249,018 | 非流动负债合计 |
| TOTAL_OTHER_PAYABLE | Nullable(Float64) | 2,626 | 零值 2；负值 41 | min=-113,986,000, max=254,053,000,000, distinct 281,140 | 其他应付款合计 |
| TOTAL_OTHER_RECE | Nullable(Float64) | 8,742 | 零值 1；负值 22 | min=-149,813,000, max=280,308,000,000, distinct 274,953 | 其他应收款合计 |
| TOTAL_PARENT_EQUITY | Nullable(Float64) | 5 | 零值 17；负值 3,517 | min=-36,884,800,000, max=1,624,530,000,000, distinct 283,956 | 归属于母公司股东权益合计 |
| TRADE_FINASSET | Nullable(Float64) | 251,792 | 零值 5,780；负值 32 | min=-49,529,400, max=4,589,740,000, distinct 20,405 | 交易性金融资产 |
| TRADE_FINASSET_NOTFVTPL | Nullable(Float64) | 206,690 | 零值 992；负值 4 | min=-800,425, max=176,336,000,000, distinct 65,013 | 非FVTPL交易性金融资产 |
| TRADE_FINLIAB | Nullable(Float64) | 278,124 | 零值 3,672；负值 3 | min=-26,371,600, max=8,957,340,000, distinct 2,277 | 交易性金融负债 |
| TRADE_FINLIAB_NOTFVTPL | Nullable(Float64) | 271,999 | 零值 1,746；负值 2 | min=-2,544,510, max=18,544,400,000, distinct 9,644 | 非FVTPL交易性金融负债 |
| TREASURY_SHARES | Nullable(Float64) | 225,402 | 零值 3,770；负值 5 | min=-25,394,100, max=27,001,800,000, distinct 26,235 | 库存股 |
| UNASSIGN_RPOFIT | Nullable(Float64) | 915 | 零值 40；负值 44,763 | min=-77,505,600,000, max=1,251,560,000,000, distinct 283,006 | 未分配利润 |
| UNCONFIRM_INVEST_LOSS | Nullable(Float64) | 278,760 | 零值 1,768；负值 116 | min=-555,325,000, max=1,292,000,000, distinct 3,466 | 未确认投资损失 |
| USERIGHT_ASSET | Nullable(Float64) | 195,377 | 零值 235；负值 0 | min=0, max=206,743,000,000, distinct 88,075 | 使用权资产 |
| ACCEPT_DEPOSIT_INTERBANK_YOY | Nullable(Float64) | 282,708 | 零值 0；负值 659 | min=-100, max=313,877, distinct 1,551 | 同业存放同比增长率（%） |
| ACCOUNTS_PAYABLE_YOY | Nullable(Float64) | 27,449 | 零值 225；负值 94,341 | min=-11,584,900, max=34,540,300,000, distinct 256,235 | 应付账款同比增长率（%） |
| ACCOUNTS_RECE_YOY | Nullable(Float64) | 20,893 | 零值 50；负值 95,251 | min=-3,048.76, max=90,991,800, distinct 262,919 | 应收账款同比增长率（%） |
| ACCRUED_EXPENSE_YOY | Nullable(Float64) | 284,059 | 零值 2；负值 69 | min=-96.2798, max=13,704.5, distinct 205 | 预提费用同比增长率（%） |
| ADVANCE_RECEIVABLES_YOY | Nullable(Float64) | 100,831 | 零值 1,048；负值 79,413 | min=-16,394.2, max=2,718,080,000, distinct 181,409 | 预收款项同比增长率（%） |
| AGENT_TRADE_SECURITY_YOY | Nullable(Float64) | 283,813 | 零值 35；负值 157 | min=-88.7709, max=3,185,070, distinct 418 | 代理买卖证券款同比增长率（%） |
| AGENT_UNDERWRITE_SECURITY_YOY | Nullable(Float64) | 284,250 | 零值 0；负值 10 | min=-99.8237, max=1,868.31, distinct 15 | 代理承销证券款同比增长率（%） |
| AMORTIZE_COST_FINASSET_YOY | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 以摊余成本计量的金融资产同比增长率（%） |
| AMORTIZE_COST_FINLIAB_YOY | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 以摊余成本计量的金融负债同比增长率（%） |
| AMORTIZE_COST_NCFINASSET_YOY | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 非流动金融资产（摊余成本）同比增长率（%） |
| AMORTIZE_COST_NCFINLIAB_YOY | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 非流动金融负债（摊余成本）同比增长率（%） |
| APPOINT_FVTPL_FINASSET_YOY | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 指定为FVTPL的金融资产同比增长率（%） |
| APPOINT_FVTPL_FINLIAB_YOY | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 指定为FVTPL的金融负债同比增长率（%） |
| ASSET_BALANCE_YOY | Nullable(Float64) | 282,540 | 零值 21；负值 1,161 | min=-400,100, max=114,467,000,000,000, distinct 354 | 资产平衡项同比增长率（%） |
| ASSET_OTHER_YOY | Nullable(Float64) | 284,248 | 零值 0；负值 5 | min=-22.4723, max=68.7258, distinct 17 | 资产其他项同比增长率（%） |
| ASSIGN_CASH_DIVIDEND_YOY | Nullable(Float64) | 284,241 | 零值 2；负值 15 | min=-100, max=700, distinct 9 | 应付现金股利同比增长率（%） |
| AVAILABLE_SALE_FINASSET_YOY | Nullable(Float64) | 244,751 | 零值 8,784；负值 13,100 | min=-100, max=1,000,000,000, distinct 26,225 | 可供出售金融资产同比增长率（%） |
| BOND_PAYABLE_YOY | Nullable(Float64) | 254,247 | 零值 1,441；负值 9,407 | min=-101.383, max=476,058,000, distinct 27,302 | 应付债券同比增长率（%） |
| BORROW_FUND_YOY | Nullable(Float64) | 283,644 | 零值 17；负值 281 | min=-100, max=3,693,940, distinct 541 | 拆入资金同比增长率（%） |
| BUY_RESALE_FINASSET_YOY | Nullable(Float64) | 283,406 | 零值 1；负值 441 | min=-100, max=1,779,920, distinct 838 | 买入返售金融资产同比增长率（%） |
| CAPITAL_RESERVE_YOY | Nullable(Float64) | 12,995 | 零值 64,442；负值 84,617 | min=-50,913.8, max=274,110,000,000, distinct 163,981 | 资本公积同比增长率（%） |
| CIP_YOY | Nullable(Float64) | 61,093 | 零值 871；负值 94,297 | min=-159.721, max=21,088,900, distinct 221,017 | 在建工程同比增长率（%） |
| CONSUMPTIVE_BIOLOGICAL_ASSET_YOY | Nullable(Float64) | 284,247 | 零值 0；负值 0 | min=7.29134, max=372.265, distinct 18 | 消耗性生物资产同比增长率（%） |
| CONTRACT_ASSET_YOY | Nullable(Float64) | 251,665 | 零值 127；负值 15,676 | min=-100, max=1,111,240, distinct 31,750 | 合同资产同比增长率（%） |
| CONTRACT_LIAB_YOY | Nullable(Float64) | 187,898 | 零值 45；负值 44,247 | min=-199.843, max=8,045,210, distinct 96,275 | 合同负债同比增长率（%） |
| CONVERT_DIFF_YOY | Nullable(Float64) | 270,747 | 零值 112；负值 8,985 | min=-1,335,970,000, max=75,829,100, distinct 12,973 | 外币报表折算差额同比增长率（%） |
| CREDITOR_INVEST_YOY | Nullable(Float64) | 279,651 | 零值 412；负值 2,078 | min=-100, max=27,936,000, distinct 3,961 | 债权投资同比增长率（%） |
| CURRENT_ASSET_BALANCE_YOY | Nullable(Float64) | 268,848 | 零值 244；负值 8,442 | min=-14,090,100,000, max=568,131,000,000, distinct 11,497 | 流动资产平衡项同比增长率（%） |
| CURRENT_ASSET_OTHER_YOY | Nullable(Float64) | 253,123 | 零值 69；负值 19,029 | min=-7,893.5, max=2,848,650,000, distinct 24,537 | 流动资产其他项同比增长率（%） |
| CURRENT_LIAB_BALANCE_YOY | Nullable(Float64) | 280,603 | 零值 25；负值 2,240 | min=-4,822.67, max=487,964,000,000, distinct 1,596 | 流动负债平衡项同比增长率（%） |
| CURRENT_LIAB_OTHER_YOY | Nullable(Float64) | 253,698 | 零值 66；负值 16,605 | min=-1,392.32, max=17,308,500, distinct 23,868 | 流动负债其他项同比增长率（%） |
| DEFER_INCOME_1YEAR_YOY | Nullable(Float64) | 284,091 | 零值 1；负值 69 | min=-100, max=1,529.63, distinct 160 | 一年内到期递延收益同比增长率（%） |
| DEFER_INCOME_YOY | Nullable(Float64) | 140,978 | 零值 1,735；负值 77,642 | min=-1,658.03, max=26,397,700, distinct 137,109 | 递延收益同比增长率（%） |
| DEFER_TAX_ASSET_YOY | Nullable(Float64) | 59,045 | 零值 439；负值 65,467 | min=-132.16, max=819,433,000, distinct 213,474 | 递延所得税资产同比增长率（%） |
| DEFER_TAX_LIAB_YOY | Nullable(Float64) | 148,090 | 零值 3,259；负值 68,079 | min=-1,764.26, max=42,156,300, distinct 124,913 | 递延所得税负债同比增长率（%） |
| DERIVE_FINASSET_YOY | Nullable(Float64) | 279,910 | 零值 11；负值 2,206 | min=-196.946, max=14,864,400, distinct 4,221 | 衍生金融资产同比增长率（%） |
| DERIVE_FINLIAB_YOY | Nullable(Float64) | 280,052 | 零值 6；负值 2,175 | min=-100, max=36,588,100, distinct 4,102 | 衍生金融负债同比增长率（%） |
| DEVELOP_EXPENSE_YOY | Nullable(Float64) | 245,231 | 零值 769；负值 13,917 | min=-100, max=6,427,830, distinct 37,272 | 开发支出同比增长率（%） |
| DIV_HOLDSALE_ASSET_YOY | Nullable(Float64) | 284,044 | 零值 72；负值 99 | min=-100, max=29,227.7, distinct 113 | 持有待售资产（除）同比增长率（%） |
| DIV_HOLDSALE_LIAB_YOY | Nullable(Float64) | 284,243 | 零值 8；负值 9 | min=-100, max=852.544, distinct 13 | 持有待售负债（除）同比增长率（%） |
| DIVIDEND_PAYABLE_YOY | Nullable(Float64) | 197,399 | 零值 23,898；负值 31,717 | min=-20,807.3, max=3,750,000,000, distinct 52,430 | 应付股利同比增长率（%） |
| DIVIDEND_RECE_YOY | Nullable(Float64) | 266,171 | 零值 3,639；负值 7,396 | min=-244.801, max=3,600,000,000, distinct 11,598 | 应收股利同比增长率（%） |
| EQUITY_BALANCE_YOY | Nullable(Float64) | 284,057 | 零值 8；负值 120 | min=-200, max=969.799, distinct 7 | 所有者权益平衡项同比增长率（%） |
| EQUITY_OTHER_YOY | Nullable(Float64) | 284,261 | 零值 1；负值 0 | min=0, max=100, distinct 4 | 所有者权益其他项同比增长率（%） |
| EXPORT_REFUND_RECE_YOY | Nullable(Float64) | 284,264 | 零值 0；负值 1 | min=-67.9149, max=-67.9149, distinct 1 | 应收出口退税同比增长率（%） |
| FEE_COMMISSION_PAYABLE_YOY | Nullable(Float64) | 284,005 | 零值 2；负值 100 | min=-99.8205, max=63,992.5, distinct 257 | 应付手续费及佣金同比增长率（%） |
| FIN_FUND_YOY | Nullable(Float64) | 284,056 | 零值 0；负值 69 | min=-87.2482, max=336.755, distinct 209 | 金融往来资金同比增长率（%） |
| FINANCE_RECE_YOY | Nullable(Float64) | 221,496 | 零值 17；负值 30,534 | min=-100, max=619,195,000, distinct 62,447 | 金融应收款同比增长率（%） |
| FIXED_ASSET_DISPOSAL_YOY | Nullable(Float64) | 274,353 | 零值 473；负值 4,791 | min=-761,307,000, max=20,242,700, distinct 8,860 | 固定资产清理同比增长率（%） |
| FIXED_ASSET_YOY | Nullable(Float64) | 20,435 | 零值 1；负值 110,256 | min=-100, max=15,775,200, distinct 263,599 | 固定资产同比增长率（%） |
| FVTOCI_FINASSET_YOY | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 以公允价值计量且其变动计入其他综合收益的金融资产同比增长率（%） |
| FVTOCI_NCFINASSET_YOY | Nullable(Float64) | 284,265 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他非流动金融资产（FVTOCI）同比增长率（%） |
| FVTPL_FINASSET_YOY | Nullable(Float64) | 278,686 | 零值 37；负值 2,793 | min=-125.191, max=80,543,200, distinct 5,348 | 以公允价值计量且其变动计入当期损益的金融资产同比增长率（%） |
| FVTPL_FINLIAB_YOY | Nullable(Float64) | 281,159 | 零值 14；负值 1,514 | min=-239.215, max=3,308,560, distinct 2,978 | 以公允价值计量且其变动计入当期损益的金融负债同比增长率（%） |
| GENERAL_RISK_RESERVE_YOY | Nullable(Float64) | 280,666 | 零值 837；负值 374 | min=-100, max=693,897, distinct 1,471 | 一般风险准备同比增长率（%） |
| GOODWILL_YOY | Nullable(Float64) | 180,260 | 零值 47,409；负值 31,393 | min=-106.577, max=493,392,000, distinct 34,272 | 商誉同比增长率（%） |
| HOLD_MATURITY_INVEST_YOY | Nullable(Float64) | 281,570 | 零值 1,030；负值 964 | min=-100, max=1,438,800, distinct 1,264 | 持有至到期投资同比增长率（%） |
| HOLDSALE_ASSET_YOY | Nullable(Float64) | 282,237 | 零值 573；负值 946 | min=-100, max=1,234,420, distinct 1,121 | 持有待售资产同比增长率（%） |
| HOLDSALE_LIAB_YOY | Nullable(Float64) | 284,038 | 零值 39；负值 88 | min=-100, max=108,684, distinct 159 | 持有待售负债同比增长率（%） |
| INSURANCE_CONTRACT_RESERVE_YOY | Nullable(Float64) | 284,063 | 零值 0；负值 23 | min=-57.5022, max=1,727.83, distinct 198 | 保险合同准备金同比增长率（%） |
| INTANGIBLE_ASSET_YOY | Nullable(Float64) | 22,583 | 零值 95；负值 140,706 | min=-369.667, max=369,877,000, distinct 260,673 | 无形资产同比增长率（%） |
| INTEREST_PAYABLE_YOY | Nullable(Float64) | 215,573 | 零值 1,636；负值 28,699 | min=-4,864.86, max=2,055,310,000, distinct 65,882 | 应付利息同比增长率（%） |
| INTEREST_RECE_YOY | Nullable(Float64) | 253,412 | 零值 489；负值 15,530 | min=-156.312, max=4,361,620,000, distinct 29,635 | 应收利息同比增长率（%） |
| INTERNAL_PAYABLE_YOY | Nullable(Float64) | 284,262 | 零值 0；负值 3 | min=-100, max=-100, distinct 1 | 内部应付款同比增长率（%） |
| INTERNAL_RECE_YOY | Nullable(Float64) | 284,258 | 零值 0；负值 5 | min=-100, max=68.2646, distinct 3 | 内部应收款同比增长率（%） |
| INVENTORY_YOY | Nullable(Float64) | 17,110 | 零值 146；负值 97,892 | min=-321.964, max=377,715,000, distinct 266,598 | 存货同比增长率（%） |
| INVEST_REALESTATE_YOY | Nullable(Float64) | 182,046 | 零值 610；负值 69,648 | min=-100, max=1,005,690, distinct 99,156 | 投资性房地产同比增长率（%） |
| LEASE_LIAB_YOY | Nullable(Float64) | 219,394 | 零值 10；负值 39,070 | min=-121.383, max=6,239,190, distinct 64,371 | 租赁负债同比增长率（%） |
| LEND_FUND_YOY | Nullable(Float64) | 283,975 | 零值 7；负值 135 | min=-100, max=25,615.4, distinct 274 | 拆出资金同比增长率（%） |
| LIAB_BALANCE_YOY | Nullable(Float64) | 283,773 | 零值 10；负值 279 | min=-74,703.3, max=101,291,000,000, distinct 75 | 负债平衡项同比增长率（%） |
| LIAB_EQUITY_BALANCE_YOY | Nullable(Float64) | 283,330 | 零值 20；负值 547 | min=-24,144,100, max=8,906,180,000,000, distinct 163 | 负债和所有者权益平衡项同比增长率（%） |
| LIAB_EQUITY_OTHER_YOY | Nullable(Float64) | 284,264 | 零值 0；负值 0 | min=100, max=100, distinct 1 | 负债和所有者权益其他项同比增长率（%） |
| LIAB_OTHER_YOY | Nullable(Float64) | 284,248 | 零值 0；负值 6 | min=-79.1093, max=195.666, distinct 17 | 负债其他项同比增长率（%） |
| LOAN_ADVANCE_YOY | Nullable(Float64) | 280,882 | 零值 47；负值 1,683 | min=-100, max=7,312,950, distinct 3,236 | 发放贷款及垫款同比增长率（%） |
| LOAN_PBC_YOY | Nullable(Float64) | 284,083 | 零值 1；负值 94 | min=-100, max=21,128.6, distinct 173 | 向央行借款同比增长率（%） |
| LONG_EQUITY_INVEST_YOY | Nullable(Float64) | 101,016 | 零值 6,650；负值 74,665 | min=-118,408, max=13,003,200,000, distinct 169,171 | 长期股权投资同比增长率（%） |
| LONG_LOAN_YOY | Nullable(Float64) | 147,581 | 零值 4,447；负值 65,442 | min=-106, max=2,909,590, distinct 115,786 | 长期借款同比增长率（%） |
| LONG_PAYABLE_YOY | Nullable(Float64) | 212,287 | 零值 10,273；负值 37,056 | min=-455,284, max=70,676,400, distinct 56,252 | 长期应付款同比增长率（%） |
| LONG_PREPAID_EXPENSE_YOY | Nullable(Float64) | 76,745 | 零值 155；负值 108,383 | min=-29,260.8, max=270,514,000,000, distinct 205,621 | 长期待摊费用同比增长率（%） |
| LONG_RECE_YOY | Nullable(Float64) | 249,326 | 零值 1,875；负值 18,805 | min=-157.744, max=3,840,900, distinct 30,541 | 长期应收款同比增长率（%） |
| LONG_STAFFSALARY_PAYABLE_YOY | Nullable(Float64) | 264,505 | 零值 665；负值 11,641 | min=-101.948, max=937,783, distinct 16,668 | 长期应付职工薪酬同比增长率（%） |
| MINORITY_EQUITY_YOY | Nullable(Float64) | 73,819 | 零值 505；负值 86,224 | min=-37,558,200, max=105,404,000,000, distinct 209,311 | 少数股东权益同比增长率（%） |
| MONETARYFUNDS_YOY | Nullable(Float64) | 12,491 | 零值 2；负值 125,171 | min=-100, max=5,209,810, distinct 271,504 | 货币资金同比增长率（%） |
| NONCURRENT_ASSET_1YEAR_YOY | Nullable(Float64) | 256,826 | 零值 991；负值 12,202 | min=-122.414, max=3,031,720,000, distinct 24,631 | 一年内到期的非流动资产同比增长率（%） |
| NONCURRENT_ASSET_BALANCE_YOY | Nullable(Float64) | 268,203 | 零值 1,169；负值 8,167 | min=-56,311.5, max=100,000,000,000, distinct 10,869 | 非流动资产平衡项同比增长率（%） |
| NONCURRENT_ASSET_OTHER_YOY | Nullable(Float64) | 283,577 | 零值 125；负值 286 | min=-100, max=23,485.2, distinct 520 | 非流动资产其他项同比增长率（%） |
| NONCURRENT_LIAB_1YEAR_YOY | Nullable(Float64) | 138,555 | 零值 3,140；负值 60,252 | min=-231.186, max=12,338,400, distinct 131,994 | 一年内到期的非流动负债同比增长率（%） |
| NONCURRENT_LIAB_BALANCE_YOY | Nullable(Float64) | 280,099 | 零值 63；负值 2,674 | min=-681,209, max=265,739,000, distinct 2,270 | 非流动负债平衡项同比增长率（%） |
| NONCURRENT_LIAB_OTHER_YOY | Nullable(Float64) | 283,611 | 零值 0；负值 291 | min=-100, max=535,093,000, distinct 640 | 非流动负债其他项同比增长率（%） |
| NOTE_ACCOUNTS_PAYABLE_YOY | Nullable(Float64) | 14,047 | 零值 218；负值 99,883 | min=-11,584,900, max=34,540,300,000, distinct 269,657 | 应付票据及应付账款同比增长率（%） |
| NOTE_ACCOUNTS_RECE_YOY | Nullable(Float64) | 18,379 | 零值 54；负值 96,966 | min=-3,048.76, max=1,488,710,000, distinct 265,485 | 应收票据及应收账款同比增长率（%） |
| NOTE_PAYABLE_YOY | Nullable(Float64) | 131,708 | 零值 463；负值 66,037 | min=-160, max=4,025,450,000, distinct 147,485 | 应付票据同比增长率（%） |
| NOTE_RECE_YOY | Nullable(Float64) | 102,898 | 零值 167；负值 81,345 | min=-412.25, max=630,449,000, distinct 178,200 | 应收票据同比增长率（%） |
| OIL_GAS_ASSET_YOY | Nullable(Float64) | 283,525 | 零值 1；负值 358 | min=-100, max=4,714.11, distinct 738 | 油气资产同比增长率（%） |
| OTHER_COMPRE_INCOME_YOY | Nullable(Float64) | 177,882 | 零值 3,314；负值 51,828 | min=-11,351,100,000, max=197,158,000,000, distinct 97,494 | 其他综合收益总额同比增长率（%） |
| OTHER_CREDITOR_INVEST_YOY | Nullable(Float64) | 282,829 | 零值 108；负值 655 | min=-100, max=55,358.4, distinct 1,273 | 其他债权投资同比增长率（%） |
| OTHER_CURRENT_ASSET_YOY | Nullable(Float64) | 100,358 | 零值 547；负值 80,346 | min=-81,277.4, max=2,155,900,000, distinct 182,426 | 其他流动资产同比增长率（%） |
| OTHER_CURRENT_LIAB_YOY | Nullable(Float64) | 165,444 | 零值 1,964；负值 52,355 | min=-7,172.85, max=248,448,000, distinct 114,561 | 其他流动负债同比增长率（%） |
| OTHER_EQUITY_INVEST_YOY | Nullable(Float64) | 232,616 | 零值 10,612；负值 20,235 | min=-118.503, max=1,200,000,000, distinct 30,927 | 其他权益工具投资同比增长率（%） |
| OTHER_EQUITY_OTHER_YOY | Nullable(Float64) | 284,264 | 零值 0；负值 1 | min=-0.00772563, max=-0.00772563, distinct 1 | 其他权益其他项同比增长率（%） |
| OTHER_EQUITY_TOOL_YOY | Nullable(Float64) | 273,428 | 零值 1,120；负值 8,185 | min=-989.346, max=141,602,000, distinct 8,792 | 其他权益工具同比增长率（%） |
| OTHER_NONCURRENT_ASSET_YOY | Nullable(Float64) | 144,268 | 零值 3,553；负值 65,950 | min=-323.298, max=109,617,000, distinct 133,122 | 其他非流动资产同比增长率（%） |
| OTHER_NONCURRENT_FINASSET_YOY | Nullable(Float64) | 249,440 | 零值 5,550；负值 12,936 | min=-100, max=1,397,060, distinct 23,528 | 其他非流动金融资产同比增长率（%） |
| OTHER_NONCURRENT_LIAB_YOY | Nullable(Float64) | 233,693 | 零值 4,441；负值 22,123 | min=-199,414, max=11,313,200,000, distinct 40,810 | 其他非流动负债同比增长率（%） |
| OTHER_PAYABLE_YOY | Nullable(Float64) | 145,592 | 零值 12；负值 56,166 | min=-2,083.8, max=56,311,600, distinct 138,412 | 其他应付款同比增长率（%） |
| OTHER_RECE_YOY | Nullable(Float64) | 153,482 | 零值 6；负值 58,635 | min=-904.73, max=46,955,900, distinct 130,537 | 其他应收款同比增长率（%） |
| PARENT_EQUITY_BALANCE_YOY | Nullable(Float64) | 281,897 | 零值 78；负值 1,619 | min=-4,370,720,000, max=20,856,300,000,000, distinct 452 | 归母权益平衡项同比增长率（%） |
| PARENT_EQUITY_OTHER_YOY | Nullable(Float64) | 284,195 | 零值 4；负值 26 | min=-210.977, max=1,149.12, distinct 46 | 归母权益其他项同比增长率（%） |
| PERPETUAL_BOND_PAYBALE_YOY | Nullable(Float64) | 284,223 | 零值 9；负值 24 | min=-100, max=435.985, distinct 30 | 永续债（负债端）同比增长率（%） |
| PERPETUAL_BOND_YOY | Nullable(Float64) | 281,763 | 零值 792；负值 655 | min=-100, max=900, distinct 1,256 | 永续债同比增长率（%） |
| PREDICT_CURRENT_LIAB_YOY | Nullable(Float64) | 283,886 | 零值 28；负值 140 | min=-100, max=11,925.2, distinct 325 | 预计流动负债同比增长率（%） |
| PREDICT_LIAB_YOY | Nullable(Float64) | 223,739 | 零值 5,193；负值 23,012 | min=-2,815.38, max=1,873,780, distinct 49,495 | 预计负债同比增长率（%） |
| PREFERRED_SHARES_PAYBALE_YOY | Nullable(Float64) | 284,210 | 零值 3；负值 35 | min=-30.9859, max=206.956, distinct 39 | 应付优先股同比增长率（%） |
| PREFERRED_SHARES_YOY | Nullable(Float64) | 284,066 | 零值 172；负值 19 | min=-100, max=100, distinct 16 | 优先股同比增长率（%） |
| PREMIUM_RECE_YOY | Nullable(Float64) | 284,097 | 零值 0；负值 66 | min=-100, max=482,226, distinct 164 | 预收保费同比增长率（%） |
| PREPAYMENT_YOY | Nullable(Float64) | 15,777 | 零值 109；负值 124,546 | min=-8,849.35, max=33,423,500, distinct 267,976 | 预付款项同比增长率（%） |
| PRODUCTIVE_BIOLOGY_ASSET_YOY | Nullable(Float64) | 276,807 | 零值 219；负值 3,702 | min=-100, max=78,107.1, distinct 7,080 | 生产性生物资产同比增长率（%） |
| PROJECT_MATERIAL_YOY | Nullable(Float64) | 258,928 | 零值 1,113；负值 13,378 | min=-546.714, max=29,447,600, distinct 23,296 | 工程物资同比增长率（%） |
| RC_RESERVE_RECE_YOY | Nullable(Float64) | 284,151 | 零值 0；负值 54 | min=-100, max=1,630,570, distinct 113 | 再保合同应收准备金同比增长率（%） |
| REINSURE_PAYABLE_YOY | Nullable(Float64) | 284,120 | 零值 1；负值 54 | min=-100, max=2,067,350, distinct 143 | 应付再保款同比增长率（%） |
| REINSURE_RECE_YOY | Nullable(Float64) | 284,131 | 零值 0；负值 50 | min=-100, max=86,207.3, distinct 133 | 应收再保款同比增长率（%） |
| SELL_REPO_FINASSET_YOY | Nullable(Float64) | 283,485 | 零值 2；负值 358 | min=-100, max=6,437.86, distinct 762 | 卖出回购金融资产款同比增长率（%） |
| SETTLE_EXCESS_RESERVE_YOY | Nullable(Float64) | 283,659 | 零值 1；负值 240 | min=-100, max=137,229, distinct 598 | 清算备付金同比增长率（%） |
| SHARE_CAPITAL_YOY | Nullable(Float64) | 10,180 | 零值 161,293；负值 18,589 | min=-100, max=553,444, distinct 50,394 | 实收资本（股本）同比增长率（%） |
| SHORT_BOND_PAYABLE_YOY | Nullable(Float64) | 284,040 | 零值 23；负值 89 | min=-100, max=3,000, distinct 159 | 短期应付债券同比增长率（%） |
| SHORT_FIN_PAYABLE_YOY | Nullable(Float64) | 284,031 | 零值 5；负值 94 | min=-100, max=74,956.4, distinct 220 | 短期金融负债同比增长率（%） |
| SHORT_LOAN_YOY | Nullable(Float64) | 79,629 | 零值 3,059；负值 87,963 | min=-100, max=2,000,000,000, distinct 180,957 | 短期借款同比增长率（%） |
| SPECIAL_PAYABLE_YOY | Nullable(Float64) | 258,870 | 零值 6,027；负值 8,944 | min=-20,872.5, max=2,952,920, distinct 15,678 | 专项应付款同比增长率（%） |
| SPECIAL_RESERVE_YOY | Nullable(Float64) | 234,505 | 零值 1,090；负值 15,043 | min=-5,729.64, max=9,208,250, distinct 47,606 | 专项储备同比增长率（%） |
| STAFF_SALARY_PAYABLE_YOY | Nullable(Float64) | 15,385 | 零值 189；负值 93,933 | min=-87,901.9, max=31,842,600, distinct 268,296 | 应付职工薪酬同比增长率（%） |
| SUBSIDY_RECE_YOY | Nullable(Float64) | 284,102 | 零值 3；负值 102 | min=-100, max=1,192.79, distinct 114 | 应收补贴同比增长率（%） |
| SURPLUS_RESERVE_YOY | Nullable(Float64) | 15,410 | 零值 65,051；负值 10,566 | min=-134.316, max=1,226,590, distinct 83,802 | 盈余公积同比增长率（%） |
| TAX_PAYABLE_YOY | Nullable(Float64) | 12,781 | 零值 5；负值 122,687 | min=-2,542,390, max=1,378,760,000, distinct 271,201 | 应交税费同比增长率（%） |
| TOTAL_ASSETS_YOY | Nullable(Float64) | 9,146 | 零值 1；负值 72,154 | min=-100, max=152,060,000, distinct 274,854 | 资产总计同比增长率（%） |
| TOTAL_CURRENT_ASSETS_YOY | Nullable(Float64) | 10,326 | 零值 1；负值 97,677 | min=-100, max=3,630,810, distinct 273,662 | 流动资产合计同比增长率（%） |
| TOTAL_CURRENT_LIAB_YOY | Nullable(Float64) | 10,325 | 零值 1；负值 94,950 | min=-880.652, max=183,016,000, distinct 273,663 | 流动负债合计同比增长率（%） |
| TOTAL_EQUITY_YOY | Nullable(Float64) | 9,954 | 零值 1；负值 65,755 | min=-406,993, max=16,902,700, distinct 274,040 | 所有者权益合计同比增长率（%） |
| TOTAL_LIAB_EQUITY_YOY | Nullable(Float64) | 9,970 | 零值 1；负值 72,147 | min=-100, max=2,530,510, distinct 274,031 | 负债和所有者权益总计同比增长率（%） |
| TOTAL_LIABILITIES_YOY | Nullable(Float64) | 9,750 | 零值 1；负值 92,167 | min=-702.993, max=230,180,000, distinct 274,239 | 负债合计同比增长率（%） |
| TOTAL_NONCURRENT_ASSETS_YOY | Nullable(Float64) | 10,439 | 零值 8；负值 77,887 | min=-116.937, max=893,469,000, distinct 273,539 | 非流动资产合计同比增长率（%） |
| TOTAL_NONCURRENT_LIAB_YOY | Nullable(Float64) | 29,477 | 零值 2,902；负值 112,391 | min=-88,422.9, max=613,595,000, distinct 245,594 | 非流动负债合计同比增长率（%） |
| TOTAL_OTHER_PAYABLE_YOY | Nullable(Float64) | 13,501 | 零值 37；负值 118,538 | min=-2,083.8, max=763,487,000, distinct 270,471 | 其他应付款合计同比增长率（%） |
| TOTAL_OTHER_RECE_YOY | Nullable(Float64) | 20,699 | 零值 37；负值 124,568 | min=-904.73, max=46,955,900, distinct 263,277 | 其他应收款合计同比增长率（%） |
| TOTAL_PARENT_EQUITY_YOY | Nullable(Float64) | 9,191 | 零值 2；负值 65,007 | min=-406,993, max=15,071,400, distinct 274,802 | 归属于母公司股东权益合计同比增长率（%） |
| TRADE_FINASSET_NOTFVTPL_YOY | Nullable(Float64) | 227,557 | 零值 594；负值 28,543 | min=-104.478, max=516,941,000,000, distinct 53,588 | 非FVTPL交易性金融资产同比增长率（%） |
| TRADE_FINASSET_YOY | Nullable(Float64) | 264,455 | 零值 696；负值 11,188 | min=-32,824.7, max=17,110,800, distinct 16,755 | 交易性金融资产同比增长率（%） |
| TRADE_FINLIAB_NOTFVTPL_YOY | Nullable(Float64) | 278,643 | 零值 168；负值 2,874 | min=-100, max=5,597,630, distinct 5,153 | 非FVTPL交易性金融负债同比增长率（%） |
| TRADE_FINLIAB_YOY | Nullable(Float64) | 282,891 | 零值 3；负值 787 | min=-100, max=3,308,560, distinct 1,126 | 交易性金融负债同比增长率（%） |
| TREASURY_SHARES_YOY | Nullable(Float64) | 244,894 | 零值 7,456；负值 19,731 | min=-316.283, max=135,549,000, distinct 23,628 | 库存股同比增长率（%） |
| UNASSIGN_RPOFIT_YOY | Nullable(Float64) | 11,719 | 零值 13；负值 72,611 | min=-2,446,290, max=9,750,190, distinct 272,268 | 未分配利润同比增长率（%） |
| UNCONFIRM_INVEST_LOSS_YOY | Nullable(Float64) | 282,127 | 零值 24；负值 536 | min=-6,786,630, max=42,411.1, distinct 2,052 | 未确认投资损失同比增长率（%） |
| USERIGHT_ASSET_YOY | Nullable(Float64) | 213,838 | 零值 9；负值 42,113 | min=-100, max=1,055,220, distinct 70,097 | 使用权资产同比增长率（%） |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 205,273 | 空字符串 0；`1970-01-01` 0 | distinct 7 | 审计意见类型 |
| OSOPINION_TYPE | LowCardinality(Nullable(String)) | 284,250 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 内控审计意见类型 |
| LISTING_STATE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 3 | 上市状态 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 284,265/284,265，供应商前缀 0/284,265，纯数字 0/284,265，空值 0/284,265；`SECURITY_CODE`: canonical 后缀 0/284,265，供应商前缀 0/284,265，纯数字 284,265/284,265，空值 0/284,265
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：`REPORT_DATE`: 1989-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 976 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME`, `SECURITY_TYPE_CODE`, `CURRENCY`
- 取值：`SECURITY_CODE`: `600654`(122), `600653`(122), `600651`(121), `600601`(121), `600610`(120), `600602`(120), `000501`(118), `600605`(118)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(122), `申华控股`(122), `飞乐音响`(121), `方正科技`(121), `云赛智联`(120), `中毅达`(120)；`ORG_CODE`: `10004106`(198), `10004127`(198), `10004293`(157), `10003964`(122), `10003963`(122), `10116535`(122), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(284,265)；`REPORT_TYPE`: `年报`(75,980), `中报`(71,407), `一季报`(69,943), `三季报`(66,935)；`REPORT_DATE_NAME`: `2026一季报`(5,099), `2025年报`(5,084), `2025三季报`(5,060), `2025中报`(5,051), `2025一季报`(5,042), `2024年报`(5,020), `2024一季报`(5,009), `2024三季报`(5,005)；`SECURITY_TYPE_CODE`: `058001001`(284,243), `058001008`(22)；`CURRENCY`: `CNY`(283,335), `NULL`(930)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 304 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 304 个数值字段执行 min/max、NULL、零值和负值检查；其中 233 个字段出现负值，270 个字段出现零值，179 个字段 NULL 数不低于 80%。 负值字段样例：`INTANGIBLE_ASSET_YOY` 140,706 行(min=-369.667)，`MONETARYFUNDS_YOY` 125,171 行(min=-100)，`TOTAL_OTHER_RECE_YOY` 124,568 行(min=-904.73)，`PREPAYMENT_YOY` 124,546 行(min=-8,849.35)，`TAX_PAYABLE_YOY` 122,687 行(min=-2,542,390)，`TOTAL_OTHER_PAYABLE_YOY` 118,538 行(min=-2,083.8)，`TOTAL_NONCURRENT_LIAB_YOY` 112,391 行(min=-88,422.9)，`FIXED_ASSET_YOY` 110,256 行(min=-100)。 高 NULL 字段样例：`AMORTIZE_COST_FINASSET` 284,265 行，`AMORTIZE_COST_FINLIAB` 284,265 行，`AMORTIZE_COST_NCFINLIAB` 284,265 行，`APPOINT_FVTPL_FINLIAB` 284,265 行，`AMORTIZE_COST_FINASSET_YOY` 284,265 行，`AMORTIZE_COST_FINLIAB_YOY` 284,265 行，`AMORTIZE_COST_NCFINASSET_YOY` 284,265 行，`AMORTIZE_COST_NCFINLIAB_YOY` 284,265 行。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECURITY_CODE` 为 6 位本地代码 | 中 | 284,265/284,265 行为纯数字 | 只作为 `security_local_code`，不可单独推出交易所 | 交易所归属或证券主数据修正延后 |
| 财务数值存在负值 | 低 | 233 个数值字段出现负值 | 负数符合财务科目/调整项可能性，staging 不过滤 | 口径解释和异常阈值延后 |

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

- 行数：284,265。
- 日期 / 分区范围：`REPORT_DATE`: 1989-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1991-06-10 至 2026-06-02，NULL 976 行，`1970-01-01` 占位 0 行
- 候选键重复：未发现重复。
- 关键 NULL / 占位值：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`SECURITY_CODE`: `600654`(122), `600653`(122), `600651`(121), `600601`(121), `600610`(120), `600602`(120), `000501`(118), `600605`(118)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(122), `申华控股`(122), `飞乐音响`(121), `方正科技`(121), `云赛智联`(120), `中毅达`(120)；`ORG_CODE`: `10004106`(198), `10004127`(198), `10004293`(157), `10003964`(122), `10003963`(122), `10116535`(122), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(284,265)；`REPORT_TYPE`: `年报`(75,980), `中报`(71,407), `一季报`(69,943), `三季报`(66,935)；`REPORT_DATE_NAME`: `2026一季报`(5,099), `2025年报`(5,084), `2025三季报`(5,060), `2025中报`(5,051), `2025一季报`(5,042), `2024年报`(5,020), `2024一季报`(5,009), `2024三季报`(5,005)；`SECURITY_TYPE_CODE`: `058001001`(284,243), `058001008`(22)；`CURRENCY`: `CNY`(283,335), `NULL`(930)
- 数值范围：已对 304 个数值字段执行 min/max、NULL、零值和负值检查；其中 233 个字段出现负值，270 个字段出现零值，179 个字段 NULL 数不低于 80%。 负值字段样例：`INTANGIBLE_ASSET_YOY` 140,706 行(min=-369.667)，`MONETARYFUNDS_YOY` 125,171 行(min=-100)，`TOTAL_OTHER_RECE_YOY` 124,568 行(min=-904.73)，`PREPAYMENT_YOY` 124,546 行(min=-8,849.35)，`TAX_PAYABLE_YOY` 122,687 行(min=-2,542,390)，`TOTAL_OTHER_PAYABLE_YOY` 118,538 行(min=-2,083.8)，`TOTAL_NONCURRENT_LIAB_YOY` 112,391 行(min=-88,422.9)，`FIXED_ASSET_YOY` 110,256 行(min=-100)。 高 NULL 字段样例：`AMORTIZE_COST_FINASSET` 284,265 行，`AMORTIZE_COST_FINLIAB` 284,265 行，`AMORTIZE_COST_NCFINLIAB` 284,265 行，`APPOINT_FVTPL_FINLIAB` 284,265 行，`AMORTIZE_COST_FINASSET_YOY` 284,265 行，`AMORTIZE_COST_FINLIAB_YOY` 284,265 行，`AMORTIZE_COST_NCFINASSET_YOY` 284,265 行，`AMORTIZE_COST_NCFINLIAB_YOY` 284,265 行。

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
select `SECUCODE`, `REPORT_DATE`, `SECURITY_CODE`, `NOTICE_DATE`, `UPDATE_DATE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME` from fleur_raw.eastmoney__balance limit 5
```

结果：

```text
[{'SECUCODE': '600601.SH', 'REPORT_DATE': datetime.date(1990, 6, 30), 'SECURITY_CODE': '600601', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '方正科技', 'ORG_CODE': '10002659', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1990中报'}, {'SECUCODE': '600601.SH', 'REPORT_DATE': datetime.date(1991, 6, 30), 'SECURITY_CODE': '600601', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '方正科技', 'ORG_CODE': '10002659', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1991中报'}, {'SECUCODE': '600602.SH', 'REPORT_DATE': datetime.date(1990, 6, 30), 'SECURITY_CODE': '600602', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '云赛智联', 'ORG_CODE': '10002660', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1990中报'}, {'SECUCODE': '600602.SH', 'REPORT_DATE': datetime.date(1991, 6, 30), 'SECURITY_CODE': '600602', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '云赛智联', 'ORG_CODE': '10002660', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1991中报'}, {'SECUCODE': '600651.SH', 'REPORT_DATE': datetime.date(1990, 6, 30), 'SECURITY_CODE': '600651', 'NOTICE_DATE': datetime.date(1991, 7, 29), 'UPDATE_DATE': datetime.date(1991, 7, 29), 'SECURITY_NAME_ABBR': '飞乐音响', 'ORG_CODE': '10003961', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1990中报'}]
```

### 行数统计

```sql
select count() from fleur_raw.eastmoney__balance
```

结果：

```text
[[284265]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `SECUCODE`, `REPORT_DATE`, count() as row_count from fleur_raw.eastmoney__balance group by `SECUCODE`, `REPORT_DATE` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 0, 'max_rows_per_key': 0}
```

### 证券代码格式：SECUCODE

```sql
select countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__balance
```

结果：

```text
{'canonical_suffix': 284265, 'vendor_prefix': 0, 'numeric_only': 0, 'empty_or_null': 0, 'row_count': 284265}
```

### 证券代码格式：SECURITY_CODE

```sql
select countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__balance
```

结果：

```text
{'canonical_suffix': 0, 'vendor_prefix': 0, 'numeric_only': 284265, 'empty_or_null': 0, 'row_count': 284265}
```
