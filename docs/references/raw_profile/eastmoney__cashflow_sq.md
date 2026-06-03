# Raw 数据画像：eastmoney__cashflow_sq

日期：2026-06-03

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__cashflow_sq.yml`
- dbt source：`source('raw', 'eastmoney__cashflow_sq')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__cashflow_sq.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__cashflow_sq`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__cashflow_sq --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__cashflow_sq.md`
- 行数：274,016
- 数据范围：`REPORT_DATE`: 2000-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 2001-03-15 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 2001-03-15 至 2026-06-02，NULL 449 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`eastmoney__cashflow_sq`
- ClickHouse raw 表：`fleur_raw.eastmoney__cashflow_sq`
- 表说明：EastMoney single-quarter cashflow F10 rows by natural-year raw partition.

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：274,016。
  - 覆盖主体数：`secucode` 5,408 个；`security_code` 5,408 个
  - 日期 / 分区范围：`REPORT_DATE`: 2000-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 2001-03-15 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 2001-03-15 至 2026-06-02，NULL 449 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `SECUCODE`, `REPORT_DATE`。
  - 候选自然键去重结果：未发现重复。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：`SECUCODE`: canonical 后缀 274,016/274,016，供应商前缀 0/274,016，纯数字 0/274,016，空值 0/274,016；`SECURITY_CODE`: canonical 后缀 0/274,016，供应商前缀 0/274,016，纯数字 274,016/274,016，空值 0/274,016
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`SECURITY_CODE`: `600019`(97), `000402`(97), `000951`(95), `600302`(95), `000025`(95), `000410`(95), `000798`(95), `000619`(95)；`SECURITY_NAME_ABBR`: `东方明珠`(142), `百联股份`(127), `宝钢股份`(97), `金融街`(97), `*ST广糖`(95), `云维股份`(95), `国电电力`(95), `中国重汽`(95)；`ORG_CODE`: `10004127`(160), `10004106`(156), `10116535`(126), `10004293`(116), `10002266`(97), `10633808`(97), `10005578`(95), `10002608`(95)；`ORG_TYPE`: `通用`(274,016)；`REPORT_TYPE`: `一季度`(71,797), `三季度`(68,255), `四季度`(67,850), `二季度`(66,114)；`REPORT_DATE_NAME`: `2026一季度`(5,099), `2025一季度`(5,088), `2025四季度`(5,084), `2024一季度`(5,073), `2025三季度`(5,060), `2025二季度`(5,051), `2024三季度`(5,034), `2024四季度`(5,020)；`SECURITY_TYPE_CODE`: `058001001`(273,993), `058001008`(23)；`CURRENCY`: `CNY`(273,610), `NULL`(406)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`REPORT_DATE`: 2000-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 2001-03-15 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 2001-03-15 至 2026-06-02，NULL 449 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 357 个数值字段执行 min/max、NULL、零值和负值检查；其中 220 个字段出现负值，178 个字段出现零值，254 个字段 NULL 数不低于 80%。 负值字段样例：`NETCASH_INVEST` 209,389 行(min=-125,557,000,000)，`NETCASH_FINANCE` 144,970 行(min=-104,203,000,000)，`CCE_ADD` 140,834 行(min=-105,279,000,000)，`NETCASH_INVEST_QOQ` 135,735 行(min=-927,434,000)，`NETCASH_INVEST_YOY` 132,079 行(min=-1,432,960,000)，`PAY_ALL_TAX_QOQ` 131,816 行(min=-70,182.5)，`TOTAL_INVEST_OUTFLOW_QOQ` 131,493 行(min=-1,212,750)，`CONSTRUCT_LONG_ASSET_QOQ` 130,642 行(min=-952,765)。 高 NULL 字段样例：`OPERATE_INFLOW_BALANCE_QOQ` 274,016 行，`OPERATE_OUTFLOW_BALANCE_QOQ` 274,016 行，`OPERATE_NETCASH_OTHER_QOQ` 274,016 行，`OPERATE_NETCASH_BALANCE_QOQ` 274,016 行，`INVEST_INFLOW_BALANCE_QOQ` 274,016 行，`INVEST_OUTFLOW_BALANCE_QOQ` 274,016 行，`INVEST_NETCASH_OTHER_QOQ` 274,016 行，`INVEST_NETCASH_BALANCE_QOQ` 274,016 行。
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
| SECUCODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,408 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,408 | 证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,406 | 证券简称 |
| ORG_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,401 | 机构代码 |
| ORG_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 机构类型 |
| REPORT_DATE | Date | 0 | `1970-01-01` 0 | 2000-12-31 至 2026-03-31; distinct 100 | 报告期 |
| REPORT_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 4 | 报告类型 |
| REPORT_DATE_NAME | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 100 | 报告期名称 |
| SECURITY_TYPE_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 2 | 证券类型代码 |
| NOTICE_DATE | Date | 0 | `1970-01-01` 0 | 2001-03-15 至 2026-05-15; distinct 3,143 | 公告日期 |
| UPDATE_DATE | Nullable(Date) | 449 | `1970-01-01` 0 | 2001-03-15 至 2026-06-02; distinct 4,011 | 更新日期 |
| CURRENCY | LowCardinality(Nullable(String)) | 406 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 现金流量表单季度金额使用的币种。 |
| SALES_SERVICES | Nullable(Float64) | 908 | 零值 105；负值 1,420 | min=-42,322,200,000, max=959,495,000,000, distinct 272,745 | 销售商品、提供劳务收到的现金 |
| DEPOSIT_INTERBANK_ADD | Nullable(Float64) | 270,594 | 零值 1,962；负值 622 | min=-87,592,400,000, max=102,878,000,000, distinct 1,460 | 同业存放净增加额 |
| LOAN_PBC_ADD | Nullable(Float64) | 271,785 | 零值 1,988；负值 110 | min=-5,080,000,000, max=3,140,000,000, distinct 239 | 向央行借款净增加额 |
| OFI_BF_ADD | Nullable(Float64) | 271,615 | 零值 2,006；负值 173 | min=-19,095,000,000, max=12,402,400,000, distinct 315 | 向其他金融机构拆入资金净增加额 |
| RECEIVE_ORIGIC_PREMIUM | Nullable(Float64) | 271,881 | 零值 1,818；负值 12 | min=-283,792,000, max=8,374,330,000, distinct 318 | 收到原保险合同保费现金 |
| RECEIVE_REINSURE_NET | Nullable(Float64) | 272,097 | 零值 1,854；负值 32 | min=-269,841,000, max=459,644,000, distinct 66 | 收到再保险业务现金净额 |
| INSURED_INVEST_ADD | Nullable(Float64) | 272,096 | 零值 1,876；负值 10 | min=-83,857,700, max=4,082,700,000, distinct 45 | 保户储金及投资款净增加额 |
| DISPOSAL_TFA_ADD | Nullable(Float64) | 273,043 | 零值 765；负值 80 | min=-6,298,430,000, max=23,209,900,000, distinct 209 | 处置交易性金融资产净增加额 |
| RECEIVE_INTEREST_COMMISSION | Nullable(Float64) | 266,497 | 零值 1,991；负值 127 | min=-1,531,850,000, max=9,739,200,000, distinct 5,521 | 收取利息和手续费现金 |
| BORROW_FUND_ADD | Nullable(Float64) | 271,571 | 零值 2,038；负值 188 | min=-18,430,500,000, max=20,261,300,000, distinct 324 | 拆入资金净增加额 |
| LOAN_ADVANCE_REDUCE | Nullable(Float64) | 257,733 | 零值 16,206；负值 16 | min=-795,635,000, max=4,164,420,000, distinct 78 | 发放贷款及垫款净减少额 |
| REPO_BUSINESS_ADD | Nullable(Float64) | 271,237 | 零值 2,024；负值 337 | min=-36,772,700,000, max=38,087,000,000, distinct 746 | 回购业务资金净增加额 |
| RECEIVE_TAX_REFUND | Nullable(Float64) | 92,048 | 零值 12,696；负值 8,523 | min=-3,578,150,000, max=18,906,100,000, distinct 168,226 | 收到的税费返还 |
| RECEIVE_OTHER_OPERATE | Nullable(Float64) | 2,004 | 零值 229；负值 31,500 | min=-65,519,400,000, max=249,712,000,000, distinct 271,388 | 收到其他与经营活动有关的现金 |
| OPERATE_INFLOW_OTHER | Nullable(Float64) | 235,335 | 零值 37,343；负值 311 | min=-49,803,800,000, max=48,393,300,000, distinct 1,337 | 经营活动现金流入其他 |
| OPERATE_INFLOW_BALANCE | Nullable(Float64) | 214,415 | 零值 59,437；负值 54 | min=-1,000,000,000, max=3,124,000,000, distinct 91 | 经营活动现金流入平衡项 |
| TOTAL_OPERATE_INFLOW | Nullable(Float64) | 464 | 零值 32；负值 2,221 | min=-68,258,900,000, max=1,017,790,000,000, distinct 273,288 | 经营活动现金流入小计 |
| BUY_SERVICES | Nullable(Float64) | 1,257 | 零值 120；负值 4,187 | min=-28,744,100,000, max=765,727,000,000, distinct 272,390 | 购买商品、接受劳务支付的现金 |
| LOAN_ADVANCE_ADD | Nullable(Float64) | 267,695 | 零值 2,096；负值 1,874 | min=-25,264,100,000, max=31,738,200,000, distinct 4,096 | 发放贷款及垫款净增加额 |
| PBC_INTERBANK_ADD | Nullable(Float64) | 270,777 | 零值 1,984；负值 574 | min=-12,903,500,000, max=16,460,800,000, distinct 1,254 | 向央行借款净增加额 |
| PAY_ORIGIC_COMPENSATE | Nullable(Float64) | 271,987 | 零值 1,860；负值 0 | min=0, max=1,123,980,000, distinct 170 | 支付原保险合同赔付款项现金 |
| PAY_INTEREST_COMMISSION | Nullable(Float64) | 268,701 | 零值 2,036；负值 175 | min=-1,733,110,000, max=6,383,010,000, distinct 3,273 | 支付利息和手续费现金 |
| PAY_POLICY_BONUS | Nullable(Float64) | 271,993 | 零值 1,948；负值 2 | min=-253,085,000, max=816,197,000, distinct 76 | 保单红利支出 |
| PAY_STAFF_CASH | Nullable(Float64) | 454 | 零值 47；负值 711 | min=-13,314,900,000, max=68,955,000,000, distinct 273,261 | 支付给职工以及为职工支付的现金 |
| PAY_ALL_TAX | Nullable(Float64) | 784 | 零值 123；负值 4,875 | min=-11,766,000,000, max=138,610,000,000, distinct 272,851 | 支付的各项税费 |
| PAY_OTHER_OPERATE | Nullable(Float64) | 598 | 零值 63；负值 21,389 | min=-58,559,900,000, max=249,401,000,000, distinct 273,083 | 支付其他与经营活动有关的现金 |
| OPERATE_OUTFLOW_OTHER | Nullable(Float64) | 235,462 | 零值 37,360；负值 335 | min=-57,217,200,000, max=72,015,300,000, distinct 1,175 | 经营活动现金流出其他 |
| OPERATE_OUTFLOW_BALANCE | Nullable(Float64) | 214,400 | 零值 59,369；负值 106 | min=-2,107,460,000, max=1,457,000,000, distinct 100 | 经营活动现金流出平衡项 |
| TOTAL_OPERATE_OUTFLOW | Nullable(Float64) | 417 | 零值 31；负值 2,731 | min=-54,168,300,000, max=956,817,000,000, distinct 273,339 | 经营活动现金流出小计 |
| OPERATE_NETCASH_OTHER | Nullable(Float64) | 258,165 | 零值 15,851；负值 0 | min=0, max=0, distinct 1 | 经营活动净现金流量其他 |
| OPERATE_NETCASH_BALANCE | Nullable(Float64) | 214,401 | 零值 59,318；负值 134 | min=-692,800,000, max=673,164,000, distinct 165 | 经营活动净现金流量平衡项 |
| NETCASH_OPERATE | Nullable(Float64) | 2 | 零值 31；负值 94,764 | min=-122,530,000,000, max=155,272,000,000, distinct 273,748 | 经营活动产生的现金流量净额 |
| WITHDRAW_INVEST | Nullable(Float64) | 146,879 | 零值 18,648；负值 6,340 | min=-155,249,000,000, max=208,464,000,000, distinct 75,089 | 收回投资收到的现金 |
| RECEIVE_INVEST_INCOME | Nullable(Float64) | 121,619 | 零值 20,381；负值 9,124 | min=-3,023,090,000, max=18,824,100,000, distinct 126,666 | 取得投资收益收到的现金 |
| DISPOSAL_LONG_ASSET | Nullable(Float64) | 86,394 | 零值 22,167；负值 13,066 | min=-3,135,520,000, max=38,578,000,000, distinct 133,423 | 处置固定资产等收回的现金净额 |
| DISPOSAL_SUBSIDIARY_OTHER | Nullable(Float64) | 250,708 | 零值 11,089；负值 2,533 | min=-20,231,800,000, max=49,821,000,000, distinct 10,709 | 处置子公司及其他营业单位收到的现金净额 |
| REDUCE_PLEDGE_TIMEDEPOSITS | Nullable(Float64) | 273,904 | 零值 55；负值 3 | min=-1,373,000,000, max=4,057,000,000, distinct 57 | 减少质押定期存款 |
| RECEIVE_OTHER_INVEST | Nullable(Float64) | 184,542 | 零值 13,482；负值 7,718 | min=-33,893,000,000, max=70,665,000,000, distinct 65,523 | 收到其他与投资活动有关的现金 |
| INVEST_INFLOW_OTHER | Nullable(Float64) | 237,291 | 零值 36,280；负值 67 | min=-3,119,850,000, max=9,706,000,000, distinct 442 | 投资活动现金流入其他 |
| INVEST_INFLOW_BALANCE | Nullable(Float64) | 221,908 | 零值 52,031；负值 17 | min=-20,000,000, max=580,000,000, distinct 67 | 投资活动现金流入平衡项 |
| TOTAL_INVEST_INFLOW | Nullable(Float64) | 30,720 | 零值 12,900；负值 13,260 | min=-152,490,000,000, max=240,716,000,000, distinct 218,033 | 投资活动现金流入小计 |
| CONSTRUCT_LONG_ASSET | Nullable(Float64) | 3,366 | 零值 1,021；负值 7,037 | min=-20,970,400,000, max=139,643,000,000, distinct 268,832 | 购建固定资产等支付的现金 |
| INVEST_PAY_CASH | Nullable(Float64) | 125,133 | 零值 21,232；负值 10,305 | min=-149,230,000,000, max=222,124,000,000, distinct 81,513 | 投资支付的现金 |
| PLEDGE_LOAN_ADD | Nullable(Float64) | 271,532 | 零值 2,404；负值 5 | min=-131,522,000, max=510,000,000, distinct 80 | 质押贷款净增加额 |
| OBTAIN_SUBSIDIARY_OTHER | Nullable(Float64) | 247,622 | 零值 11,078；负值 2,986 | min=-63,785,100,000, max=90,942,000,000, distinct 13,598 | 取得子公司及其他营业单位支付的现金净额 |
| ADD_PLEDGE_TIMEDEPOSITS | Nullable(Float64) | 273,968 | 零值 17；负值 5 | min=-90,974,700, max=461,673,000, distinct 32 | 增加质押定期存款 |
| PAY_OTHER_INVEST | Nullable(Float64) | 197,561 | 零值 12,228；负值 8,443 | min=-53,376,000,000, max=98,599,000,000, distinct 50,435 | 支付其他与投资活动有关的现金 |
| INVEST_OUTFLOW_OTHER | Nullable(Float64) | 237,327 | 零值 36,309；负值 83 | min=-21,889,000,000, max=31,969,000,000, distinct 363 | 投资活动现金流出其他 |
| INVEST_OUTFLOW_BALANCE | Nullable(Float64) | 214,895 | 零值 59,066；负值 12 | min=-383,291,000, max=3,609,000,000, distinct 49 | 投资活动现金流出平衡项 |
| TOTAL_INVEST_OUTFLOW | Nullable(Float64) | 2,457 | 零值 850；负值 9,394 | min=-145,526,000,000, max=226,809,000,000, distinct 270,027 | 投资活动现金流出小计 |
| INVEST_NETCASH_OTHER | Nullable(Float64) | 258,179 | 零值 15,836；负值 0 | min=0, max=5,120,730,000, distinct 2 | 投资活动净现金流量其他 |
| INVEST_NETCASH_BALANCE | Nullable(Float64) | 214,743 | 零值 59,230；负值 21 | min=-34,704,500, max=20,000,000, distinct 19 | 投资活动净现金流量平衡项 |
| NETCASH_INVEST | Nullable(Float64) | 1,752 | 零值 594；负值 209,389 | min=-125,557,000,000, max=60,737,900,000, distinct 271,169 | 投资活动产生的现金流量净额 |
| ACCEPT_INVEST_CASH | Nullable(Float64) | 193,639 | 零值 32,234；负值 4,991 | min=-50,965,300,000, max=105,089,000,000, distinct 31,964 | 吸收投资收到的现金 |
| SUBSIDIARY_ACCEPT_INVEST | Nullable(Float64) | 234,417 | 零值 15,974；负值 1,529 | min=-35,465,300,000, max=105,089,000,000, distinct 13,321 | 子公司吸收少数股东投资收到的现金 |
| RECEIVE_LOAN_CASH | Nullable(Float64) | 67,809 | 零值 18,066；负值 4,740 | min=-91,209,300,000, max=378,981,000,000, distinct 116,025 | 取得借款收到的现金 |
| ISSUE_BOND | Nullable(Float64) | 267,069 | 零值 3,640；负值 336 | min=-24,423,700,000, max=39,365,800,000, distinct 2,061 | 发行债券收到的现金 |
| RECEIVE_OTHER_FINANCE | Nullable(Float64) | 182,959 | 零值 14,876；负值 10,147 | min=-69,904,000,000, max=69,900,000,000, distinct 67,051 | 收到其他与筹资活动有关的现金 |
| FINANCE_INFLOW_OTHER | Nullable(Float64) | 257,881 | 零值 15,866；负值 36 | min=-5,840,590,000, max=65,988,000,000, distinct 266 | 筹资活动现金流入其他 |
| FINANCE_INFLOW_BALANCE | Nullable(Float64) | 225,684 | 零值 48,288；负值 13 | min=-19,500,000, max=2,841,000,000, distinct 39 | 筹资活动现金流入平衡项 |
| TOTAL_FINANCE_INFLOW | Nullable(Float64) | 41,560 | 零值 18,510；负值 6,563 | min=-92,294,300,000, max=431,723,000,000, distinct 164,992 | 筹资活动现金流入小计 |
| PAY_DEBT_CASH | Nullable(Float64) | 59,924 | 零值 15,361；负值 4,228 | min=-83,243,200,000, max=423,759,000,000, distinct 136,077 | 偿还债务支付的现金 |
| ASSIGN_DIVIDEND_PORFIT | Nullable(Float64) | 27,131 | 零值 10,831；负值 6,342 | min=-4,945,460,000, max=66,670,000,000, distinct 233,897 | 分配股利、利润或偿付利息支付的现金 |
| SUBSIDIARY_PAY_DIVIDEND | Nullable(Float64) | 237,006 | 零值 14,030；负值 1,790 | min=-9,323,810,000, max=15,528,000,000, distinct 19,421 | 子公司向少数股东支付的现金股利 |
| BUY_SUBSIDIARY_EQUITY | Nullable(Float64) | 273,569 | 零值 417；负值 1 | min=-6,800,000, max=2,314,200,000, distinct 31 | 子公司减少现金 |
| PAY_OTHER_FINANCE | Nullable(Float64) | 111,468 | 零值 13,468；负值 13,784 | min=-37,079,000,000, max=64,588,500,000, distinct 140,064 | 支付其他与筹资活动有关的现金 |
| SUBSIDIARY_REDUCE_CASH | Nullable(Float64) | 257,769 | 零值 16,239；负值 1 | min=-3,000,000, max=650,000,000, distinct 8 | 子公司减少现金 |
| FINANCE_OUTFLOW_OTHER | Nullable(Float64) | 237,126 | 零值 36,371；负值 46 | min=-55,325,100,000, max=773,111,000,000, distinct 483 | 筹资活动现金流出其他 |
| FINANCE_OUTFLOW_BALANCE | Nullable(Float64) | 218,718 | 零值 55,224；负值 27 | min=-93,300,000, max=408,702,000, distinct 56 | 筹资活动现金流出平衡项 |
| TOTAL_FINANCE_OUTFLOW | Nullable(Float64) | 13,394 | 零值 6,717；负值 5,805 | min=-82,581,900,000, max=441,309,000,000, distinct 251,819 | 筹资活动现金流出小计 |
| FINANCE_NETCASH_OTHER | Nullable(Float64) | 258,169 | 零值 15,847；负值 0 | min=0, max=0, distinct 1 | 筹资活动净现金流量其他 |
| FINANCE_NETCASH_BALANCE | Nullable(Float64) | 217,899 | 零值 56,086；负值 12 | min=-92,520,000, max=1,258,200,000, distinct 21 | 筹资活动净现金流量平衡项 |
| NETCASH_FINANCE | Nullable(Float64) | 10,604 | 零值 5,531；负值 144,970 | min=-104,203,000,000, max=119,768,000,000, distinct 255,472 | 筹资活动产生的现金流量净额 |
| RATE_CHANGE_EFFECT | Nullable(Float64) | 98,760 | 零值 3,816；负值 94,569 | min=-4,251,140,000, max=8,354,230,000, distinct 170,003 | 汇率变动对现金及现金等价物的影响 |
| CCE_ADD_OTHER | Nullable(Float64) | 258,140 | 零值 15,855；负值 12 | min=-105,762,000, max=105,762,000, distinct 22 | 现金及现金等价物净增加额其他 |
| CCE_ADD_BALANCE | Nullable(Float64) | 214,392 | 零值 59,269；负值 191 | min=-673,164,000, max=692,800,000, distinct 208 | 现金及现金等价物净增加额平衡项 |
| CCE_ADD | Nullable(Float64) | 405 | 零值 36；负值 140,834 | min=-105,279,000,000, max=87,813,300,000, distinct 273,338 | 现金及现金等价物净增加额 |
| BEGIN_CCE | Nullable(Float64) | 16,450 | 零值 16；负值 226 | min=-625,769,000, max=359,373,000,000, distinct 257,119 | 期初现金及现金等价物余额 |
| END_CCE_OTHER | Nullable(Float64) | 274,009 | 零值 3；负值 3 | min=-1,482,680, max=395,206,000, distinct 5 | 期末现金及现金等价物余额其他 |
| END_CCE_BALANCE | Nullable(Float64) | 218,170 | 零值 55,784；负值 32 | min=-232,420,000, max=2,144,580,000, distinct 34 | 期末现金及现金等价物余额平衡项 |
| END_CCE | Nullable(Float64) | 16,085 | 零值 10；负值 232 | min=-3,616,490,000, max=359,373,000,000, distinct 257,654 | 期末现金及现金等价物余额 |
| SALES_SERVICES_QOQ | Nullable(Float64) | 4,481 | 零值 9；负值 122,036 | min=-7,725.92, max=11,307,600, distinct 269,253 | 销售商品、提供劳务收到的现金环比增长率（%） |
| DEPOSIT_INTERBANK_ADD_QOQ | Nullable(Float64) | 272,766 | 零值 0；负值 572 | min=-158,375, max=43,758,000, distinct 1,236 | 同业存放净增加额环比增长率（%） |
| LOAN_PBC_ADD_QOQ | Nullable(Float64) | 273,819 | 零值 0；负值 98 | min=-112,074, max=13,257.5, distinct 178 | 向央行借款净增加额环比增长率（%） |
| OFI_BF_ADD_QOQ | Nullable(Float64) | 273,710 | 零值 2；负值 154 | min=-281,085, max=854,108, distinct 263 | 向其他金融机构拆入资金净增加额环比增长率（%） |
| RECEIVE_ORIGIC_PREMIUM_QOQ | Nullable(Float64) | 273,718 | 零值 0；负值 139 | min=-207.939, max=24,493.9, distinct 296 | 收到原保险合同保费现金环比增长率（%） |
| RECEIVE_REINSURE_NET_QOQ | Nullable(Float64) | 273,967 | 零值 0；负值 22 | min=-8,356.57, max=1,248.11, distinct 49 | 收到再保险业务现金净额环比增长率（%） |
| INSURED_INVEST_ADD_QOQ | Nullable(Float64) | 273,982 | 零值 0；负值 16 | min=-197.165, max=2,178.42, distinct 34 | 保户储金及投资款净增加额环比增长率（%） |
| DISPOSAL_TFA_ADD_QOQ | Nullable(Float64) | 273,866 | 零值 0；负值 84 | min=-5,512.79, max=592,843, distinct 150 | 处置交易性金融资产净增加额环比增长率（%） |
| RECEIVE_INTEREST_COMMISSION_QOQ | Nullable(Float64) | 268,826 | 零值 4；负值 2,605 | min=-115,936, max=15,118,700,000, distinct 5,135 | 收取利息和手续费现金环比增长率（%） |
| BORROW_FUND_ADD_QOQ | Nullable(Float64) | 273,671 | 零值 3；负值 176 | min=-1,931.87, max=29,235,000, distinct 301 | 拆入资金净增加额环比增长率（%） |
| LOAN_ADVANCE_REDUCE_QOQ | Nullable(Float64) | 273,967 | 零值 0；负值 29 | min=-374.912, max=2,345.73, distinct 49 | 发放贷款及垫款净减少额环比增长率（%） |
| REPO_BUSINESS_ADD_QOQ | Nullable(Float64) | 273,368 | 零值 0；负值 330 | min=-6,600, max=486,817, distinct 623 | 回购业务资金净增加额环比增长率（%） |
| RECEIVE_TAX_REFUND_QOQ | Nullable(Float64) | 115,362 | 零值 34；负值 81,620 | min=-4,471,200,000, max=104,215,000,000, distinct 151,991 | 收到的税费返还环比增长率（%） |
| RECEIVE_OTHER_OPERATE_QOQ | Nullable(Float64) | 5,982 | 零值 4；负值 127,932 | min=-3,310,180, max=138,854,000, distinct 267,658 | 收到其他与经营活动有关的现金环比增长率（%） |
| OPERATE_INFLOW_OTHER_QOQ | Nullable(Float64) | 272,888 | 零值 0；负值 580 | min=-6,470.89, max=246,988, distinct 1,076 | 经营活动现金流入其他环比增长率（%） |
| OPERATE_INFLOW_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动现金流入平衡项环比增长率（%） |
| TOTAL_OPERATE_INFLOW_QOQ | Nullable(Float64) | 3,875 | 零值 2；负值 122,298 | min=-89,969.5, max=426,295,000, distinct 269,904 | 经营活动现金流入小计环比增长率（%） |
| BUY_SERVICES_QOQ | Nullable(Float64) | 4,865 | 零值 2；负值 125,233 | min=-270,000,000,000, max=6,719,210, distinct 268,861 | 购买商品、接受劳务支付的现金环比增长率（%） |
| LOAN_ADVANCE_ADD_QOQ | Nullable(Float64) | 270,197 | 零值 6；负值 1,925 | min=-1,248,910,000, max=73,575,600, distinct 3,725 | 发放贷款及垫款净增加额环比增长率（%） |
| PBC_INTERBANK_ADD_QOQ | Nullable(Float64) | 272,906 | 零值 0；负值 531 | min=-37,832, max=499,638, distinct 1,099 | 向央行借款净增加额环比增长率（%） |
| PAY_ORIGIC_COMPENSATE_QOQ | Nullable(Float64) | 273,860 | 零值 0；负值 65 | min=-100, max=2,631.64, distinct 156 | 支付原保险合同赔付款项现金环比增长率（%） |
| PAY_INTEREST_COMMISSION_QOQ | Nullable(Float64) | 270,985 | 零值 2；负值 1,532 | min=-176,751, max=79,972,700, distinct 2,983 | 支付利息和手续费现金环比增长率（%） |
| PAY_POLICY_BONUS_QOQ | Nullable(Float64) | 273,956 | 零值 0；负值 36 | min=-659.078, max=1,797.3, distinct 60 | 保单红利支出环比增长率（%） |
| PAY_STAFF_CASH_QOQ | Nullable(Float64) | 3,881 | 零值 4；负值 121,206 | min=-23,452.6, max=2,165,330,000,000, distinct 269,887 | 支付给职工以及为职工支付的现金环比增长率（%） |
| PAY_ALL_TAX_QOQ | Nullable(Float64) | 4,351 | 零值 4；负值 131,816 | min=-70,182.5, max=28,716,800,000, distinct 269,363 | 支付的各项税费环比增长率（%） |
| PAY_OTHER_OPERATE_QOQ | Nullable(Float64) | 4,086 | 零值 1；负值 127,800 | min=-552,810, max=708,124,000,000, distinct 269,664 | 支付其他与经营活动有关的现金环比增长率（%） |
| OPERATE_OUTFLOW_OTHER_QOQ | Nullable(Float64) | 272,971 | 零值 5；负值 527 | min=-547,009, max=19,999,500, distinct 991 | 经营活动现金流出其他环比增长率（%） |
| OPERATE_OUTFLOW_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动现金流出平衡项环比增长率（%） |
| TOTAL_OPERATE_OUTFLOW_QOQ | Nullable(Float64) | 3,817 | 零值 0；负值 124,251 | min=-73,990.1, max=17,651,200,000,000, distinct 269,968 | 经营活动现金流出小计环比增长率（%） |
| OPERATE_NETCASH_OTHER_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动净现金流量其他环比增长率（%） |
| OPERATE_NETCASH_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动净现金流量平衡项环比增长率（%） |
| NETCASH_OPERATE_QOQ | Nullable(Float64) | 2,957 | 零值 0；负值 130,431 | min=-106,206,000, max=3,278,050,000,000, distinct 270,823 | 经营活动产生的现金流量净额环比增长率（%） |
| WITHDRAW_INVEST_QOQ | Nullable(Float64) | 177,205 | 零值 717；负值 51,603 | min=-421,606,000,000, max=2,039,000,000,000, distinct 79,770 | 收回投资收到的现金环比增长率（%） |
| RECEIVE_INVEST_INCOME_QOQ | Nullable(Float64) | 155,147 | 零值 99；负值 62,950 | min=-117,744,000,000, max=394,397,000,000, distinct 110,201 | 取得投资收益收到的现金环比增长率（%） |
| DISPOSAL_LONG_ASSET_QOQ | Nullable(Float64) | 126,915 | 零值 94；负值 77,216 | min=-318,833,000, max=1,453,510,000,000, distinct 133,025 | 处置固定资产等收回的现金净额环比增长率（%） |
| DISPOSAL_SUBSIDIARY_OTHER_QOQ | Nullable(Float64) | 266,343 | 零值 39；负值 4,958 | min=-5,401,630,000,000, max=5,878,300,000,000, distinct 4,900 | 处置子公司及其他营业单位收到的现金净额环比增长率（%） |
| REDUCE_PLEDGE_TIMEDEPOSITS_QOQ | Nullable(Float64) | 273,973 | 零值 0；负值 23 | min=-169.378, max=35,100, distinct 43 | 减少质押定期存款环比增长率（%） |
| RECEIVE_OTHER_INVEST_QOQ | Nullable(Float64) | 209,013 | 零值 265；负值 34,257 | min=-32,883,500,000, max=1,100,000,000,000, distinct 57,954 | 收到其他与投资活动有关的现金环比增长率（%） |
| INVEST_INFLOW_OTHER_QOQ | Nullable(Float64) | 273,658 | 零值 1；负值 181 | min=-2,527.92, max=1,729,200, distinct 304 | 投资活动现金流入其他环比增长率（%） |
| INVEST_INFLOW_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 投资活动现金流入平衡项环比增长率（%） |
| TOTAL_INVEST_INFLOW_QOQ | Nullable(Float64) | 54,235 | 零值 58；负值 110,640 | min=-263,294,000,000, max=2,646,230,000,000, distinct 211,367 | 投资活动现金流入小计环比增长率（%） |
| CONSTRUCT_LONG_ASSET_QOQ | Nullable(Float64) | 8,452 | 零值 4；负值 130,642 | min=-952,765, max=17,768,400,000, distinct 264,654 | 购建固定资产等支付的现金环比增长率（%） |
| INVEST_PAY_CASH_QOQ | Nullable(Float64) | 159,541 | 零值 739；负值 61,371 | min=-398,701,000,000, max=12,224,100,000,000, distinct 93,459 | 投资支付的现金环比增长率（%） |
| PLEDGE_LOAN_ADD_QOQ | Nullable(Float64) | 273,961 | 零值 0；负值 30 | min=-9,019.43, max=1,462.53, distinct 55 | 质押贷款净增加额环比增长率（%） |
| OBTAIN_SUBSIDIARY_OTHER_QOQ | Nullable(Float64) | 263,718 | 零值 24；负值 6,500 | min=-45,661,100,000, max=53,444,800,000,000, distinct 7,337 | 取得子公司及其他营业单位支付的现金净额环比增长率（%） |
| ADD_PLEDGE_TIMEDEPOSITS_QOQ | Nullable(Float64) | 273,998 | 零值 0；负值 10 | min=-3,041.38, max=917,900, distinct 18 | 增加质押定期存款环比增长率（%） |
| PAY_OTHER_INVEST_QOQ | Nullable(Float64) | 221,566 | 零值 257；负值 28,170 | min=-6,965,010,000,000, max=7,281,690,000,000, distinct 44,931 | 支付其他与投资活动有关的现金环比增长率（%） |
| INVEST_OUTFLOW_OTHER_QOQ | Nullable(Float64) | 273,723 | 零值 1；负值 166 | min=-30,143.8, max=799,880,000,000, distinct 212 | 投资活动现金流出其他环比增长率（%） |
| INVEST_OUTFLOW_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 投资活动现金流出平衡项环比增长率（%） |
| TOTAL_INVEST_OUTFLOW_QOQ | Nullable(Float64) | 7,146 | 零值 4；负值 131,493 | min=-1,212,750, max=1,237,770,000,000, distinct 266,061 | 投资活动现金流出小计环比增长率（%） |
| INVEST_NETCASH_OTHER_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 投资活动净现金流量其他环比增长率（%） |
| INVEST_NETCASH_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 投资活动净现金流量平衡项环比增长率（%） |
| NETCASH_INVEST_QOQ | Nullable(Float64) | 6,072 | 零值 3；负值 135,735 | min=-927,434,000, max=24,107,100,000, distinct 267,315 | 投资活动产生的现金流量净额环比增长率（%） |
| ACCEPT_INVEST_CASH_QOQ | Nullable(Float64) | 236,571 | 零值 239；负值 24,031 | min=-254,299,000,000,000, max=108,620,000,000,000, distinct 24,329 | 吸收投资收到的现金环比增长率（%） |
| SUBSIDIARY_ACCEPT_INVEST_QOQ | Nullable(Float64) | 255,506 | 零值 175；负值 11,632 | min=-7,934,560,000, max=2,204,520,000,000, distinct 11,996 | 子公司吸收少数股东投资收到的现金环比增长率（%） |
| RECEIVE_LOAN_CASH_QOQ | Nullable(Float64) | 94,268 | 零值 1,057；负值 92,410 | min=-132,834,000, max=431,031,000,000, distinct 146,225 | 取得借款收到的现金环比增长率（%） |
| ISSUE_BOND_QOQ | Nullable(Float64) | 271,528 | 零值 62；负值 1,616 | min=-1,379.98, max=498,000,000,000, distinct 1,421 | 发行债券收到的现金环比增长率（%） |
| RECEIVE_OTHER_FINANCE_QOQ | Nullable(Float64) | 210,561 | 零值 128；负值 34,105 | min=-999,254,000,000, max=1,452,190,000,000, distinct 56,442 | 收到其他与筹资活动有关的现金环比增长率（%） |
| FINANCE_INFLOW_OTHER_QOQ | Nullable(Float64) | 273,798 | 零值 1；负值 113 | min=-699.654, max=7,999,900, distinct 192 | 筹资活动现金流入其他环比增长率（%） |
| FINANCE_INFLOW_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 筹资活动现金流入平衡项环比增长率（%） |
| TOTAL_FINANCE_INFLOW_QOQ | Nullable(Float64) | 69,092 | 零值 590；负值 104,115 | min=-238,611,000,000, max=9,126,790,000,000, distinct 183,751 | 筹资活动现金流入小计环比增长率（%） |
| PAY_DEBT_CASH_QOQ | Nullable(Float64) | 83,733 | 零值 1,390；负值 95,906 | min=-1,605,320,000,000, max=1,005,200,000,000, distinct 163,095 | 偿还债务支付的现金环比增长率（%） |
| ASSIGN_DIVIDEND_PORFIT_QOQ | Nullable(Float64) | 45,997 | 零值 185；负值 117,401 | min=-12,934,400, max=420,484,000,000, distinct 223,837 | 分配股利、利润或偿付利息支付的现金环比增长率（%） |
| SUBSIDIARY_PAY_DIVIDEND_QOQ | Nullable(Float64) | 256,819 | 零值 72；负值 10,338 | min=-50,211,300,000, max=3,041,830,000,000, distinct 13,093 | 子公司向少数股东支付的现金股利环比增长率（%） |
| BUY_SUBSIDIARY_EQUITY_QOQ | Nullable(Float64) | 273,996 | 零值 0；负值 18 | min=-200, max=202.382, distinct 6 | 子公司减少现金环比增长率（%） |
| PAY_OTHER_FINANCE_QOQ | Nullable(Float64) | 140,503 | 零值 787；负值 68,406 | min=-7,177,400,000, max=10,415,100,000,000, distinct 125,756 | 支付其他与筹资活动有关的现金环比增长率（%） |
| SUBSIDIARY_REDUCE_CASH_QOQ | Nullable(Float64) | 274,010 | 零值 1；负值 2 | min=-86.8293, max=1,344.44, distinct 6 | 子公司减少现金环比增长率（%） |
| FINANCE_OUTFLOW_OTHER_QOQ | Nullable(Float64) | 273,597 | 零值 2；负值 227 | min=-1,389.18, max=126,741,000, distinct 323 | 筹资活动现金流出其他环比增长率（%） |
| FINANCE_OUTFLOW_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 筹资活动现金流出平衡项环比增长率（%） |
| TOTAL_FINANCE_OUTFLOW_QOQ | Nullable(Float64) | 26,823 | 零值 131；负值 121,930 | min=-454,662,000, max=2,086,600,000,000, distinct 243,723 | 筹资活动现金流出小计环比增长率（%） |
| FINANCE_NETCASH_OTHER_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 筹资活动净现金流量其他环比增长率（%） |
| FINANCE_NETCASH_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 筹资活动净现金流量平衡项环比增长率（%） |
| NETCASH_FINANCE_QOQ | Nullable(Float64) | 22,385 | 零值 93；负值 121,952 | min=-302,501,000,000, max=500,000,000,000, distinct 248,442 | 筹资活动产生的现金流量净额环比增长率（%） |
| RATE_CHANGE_EFFECT_QOQ | Nullable(Float64) | 108,713 | 零值 15；负值 86,659 | min=-48,024,600,000, max=56,577,400,000, distinct 163,851 | 汇率变动对现金及现金等价物的影响环比增长率（%） |
| CCE_ADD_OTHER_QOQ | Nullable(Float64) | 274,010 | 零值 0；负值 3 | min=-207.625, max=176.648, distinct 5 | 现金及现金等价物净增加额其他环比增长率（%） |
| CCE_ADD_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额平衡项环比增长率（%） |
| CCE_ADD_QOQ | Nullable(Float64) | 3,791 | 零值 0；负值 127,065 | min=-336,227,000,000, max=24,585,800,000, distinct 269,982 | 现金及现金等价物净增加额环比增长率（%） |
| BEGIN_CCE_QOQ | Nullable(Float64) | 19,943 | 零值 34；负值 129,011 | min=-273,404,000,000, max=2,901,840, distinct 253,846 | 期初现金及现金等价物余额环比增长率（%） |
| END_CCE_OTHER_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 期末现金及现金等价物余额其他环比增长率（%） |
| END_CCE_BALANCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 期末现金及现金等价物余额平衡项环比增长率（%） |
| END_CCE_QOQ | Nullable(Float64) | 19,227 | 零值 37；负值 130,460 | min=-5,036.03, max=2,901,840, distinct 254,575 | 期末现金及现金等价物余额环比增长率（%） |
| NETPROFIT | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 净利润（间接法起点） |
| ASSET_IMPAIRMENT | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 资产减值准备 |
| FA_IR_DEPR | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 固定资产折旧、油气资产折耗、生产性生物资产折旧 |
| OILGAS_BIOLOGY_DEPR | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 油气资产折耗、生产性生物资产折旧 |
| IR_DEPR | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 折旧与摊销 |
| IA_AMORTIZE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 无形资产摊销 |
| LPE_AMORTIZE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 长期待摊费用摊销 |
| DEFER_INCOME_AMORTIZE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 待摊费用减少（减：增加） |
| PREPAID_EXPENSE_REDUCE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预提费用增加（减：减少） |
| ACCRUED_EXPENSE_ADD | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预提费用变动 |
| DISPOSAL_LONGASSET_LOSS | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 处置固定资产等的损失 |
| FA_SCRAP_LOSS | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 固定资产报废损失 |
| FAIRVALUE_CHANGE_LOSS | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 公允价值变动损失 |
| FINANCE_EXPENSE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 财务费用 |
| INVEST_LOSS | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 投资损失 |
| DEFER_TAX | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税资产减少（增加以"-"号填列） |
| DT_ASSET_REDUCE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税资产减少 |
| DT_LIAB_ADD | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税负债增加 |
| PREDICT_LIAB_ADD | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预计负债增加 |
| INVENTORY_REDUCE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 存货的减少（增加以"-"号填列） |
| OPERATE_RECE_REDUCE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营性应收项目的减少 |
| OPERATE_PAYABLE_ADD | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营性应付项目的增加 |
| OTHER | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金流量表单季度补充资料中的其他项目。 |
| OPERATE_NETCASH_OTHERNOTE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动产生的现金流量净额（附注） |
| OPERATE_NETCASH_BALANCENOTE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动净现金流量（附注）平衡项 |
| NETCASH_OPERATENOTE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动产生的现金流量净额（附注） |
| DEBT_TRANSFER_CAPITAL | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 债务转为资本 |
| CONVERT_BOND_1YEAR | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 一年内到期的可转换公司债券 |
| FINLEASE_OBTAIN_FA | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 融资租入固定资产 |
| UNINVOLVE_INVESTFIN_OTHER | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 不涉及现金收支的投资和筹资活动其他 |
| END_CASH | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金期末余额 |
| BEGIN_CASH | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金期初余额 |
| END_CASH_EQUIVALENTS | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金等价物期末余额 |
| BEGIN_CASH_EQUIVALENTS | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金等价物期初余额 |
| CCE_ADD_OTHERNOTE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注） |
| CCE_ADD_BALANCENOTE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注）平衡项 |
| CCE_ADDNOTE | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注） |
| MINORITY_INTEREST | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 少数股东损益 |
| NETPROFIT_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 净利润环比增长率（%） |
| ASSET_IMPAIRMENT_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 资产减值准备环比增长率（%） |
| FA_IR_DEPR_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 固定资产折旧、油气资产折耗、生产性生物资产折旧环比增长率（%） |
| OILGAS_BIOLOGY_DEPR_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 油气资产折耗、生产性生物资产折旧环比增长率（%） |
| IR_DEPR_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 折旧与摊销环比增长率（%） |
| IA_AMORTIZE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 无形资产摊销环比增长率（%） |
| LPE_AMORTIZE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 长期待摊费用摊销环比增长率（%） |
| DEFER_INCOME_AMORTIZE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 待摊费用减少（减：增加）环比增长率（%） |
| PREPAID_EXPENSE_REDUCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预提费用增加（减：减少）环比增长率（%） |
| ACCRUED_EXPENSE_ADD_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预提费用变动环比增长率（%） |
| DISPOSAL_LONGASSET_LOSS_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 处置固定资产等的损失环比增长率（%） |
| FA_SCRAP_LOSS_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 固定资产报废损失环比增长率（%） |
| FAIRVALUE_CHANGE_LOSS_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 公允价值变动损失环比增长率（%） |
| FINANCE_EXPENSE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 财务费用环比增长率（%） |
| INVEST_LOSS_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 投资损失环比增长率（%） |
| DEFER_TAX_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税资产减少（增加以"-"号填列）环比增长率（%） |
| DT_ASSET_REDUCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税资产减少环比增长率（%） |
| DT_LIAB_ADD_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税负债增加环比增长率（%） |
| PREDICT_LIAB_ADD_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预计负债增加环比增长率（%） |
| INVENTORY_REDUCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 存货的减少（增加以"-"号填列）环比增长率（%） |
| OPERATE_RECE_REDUCE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营性应收项目的减少环比增长率（%） |
| OPERATE_PAYABLE_ADD_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营性应付项目的增加环比增长率（%） |
| OTHER_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他环比增长率（%） |
| OPERATE_NETCASH_OTHERNOTE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动产生的现金流量净额（附注）环比增长率（%） |
| OPERATE_NETCASH_BALANCENOTE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动净现金流量（附注）平衡项环比增长率（%） |
| NETCASH_OPERATENOTE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动产生的现金流量净额（附注）环比增长率（%） |
| DEBT_TRANSFER_CAPITAL_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 债务转为资本环比增长率（%） |
| CONVERT_BOND_1YEAR_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 一年内到期的可转换公司债券环比增长率（%） |
| FINLEASE_OBTAIN_FA_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 融资租入固定资产环比增长率（%） |
| UNINVOLVE_INVESTFIN_OTHER_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 不涉及现金收支的投资和筹资活动其他环比增长率（%） |
| END_CASH_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金期末余额环比增长率（%） |
| BEGIN_CASH_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金期初余额环比增长率（%） |
| END_CASH_EQUIVALENTS_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金等价物期末余额环比增长率（%） |
| BEGIN_CASH_EQUIVALENTS_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金等价物期初余额环比增长率（%） |
| CCE_ADD_OTHERNOTE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注）环比增长率（%） |
| CCE_ADD_BALANCENOTE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注）平衡项环比增长率（%） |
| CCE_ADDNOTE_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注）环比增长率（%） |
| MINORITY_INTEREST_QOQ | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 少数股东损益环比增长率（%） |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 201,199 | 空字符串 0；`1970-01-01` 0 | distinct 7 | 审计意见类型 |
| OSOPINION_TYPE | LowCardinality(Nullable(String)) | 274,001 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 内控审计意见类型 |
| LISTING_STATE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 3 | 上市状态 |
| SALES_SERVICES_YOY | Nullable(Float64) | 15,140 | 零值 22；负值 101,341 | min=-690,184, max=11,814,500,000, distinct 258,580 | 销售商品、提供劳务收到的现金同比增长率（%） |
| DEPOSIT_INTERBANK_ADD_YOY | Nullable(Float64) | 272,894 | 零值 0；负值 567 | min=-1,054,580, max=11,642,300, distinct 1,104 | 同业存放净增加额同比增长率（%） |
| LOAN_PBC_ADD_YOY | Nullable(Float64) | 273,838 | 零值 2；负值 92 | min=-20,761.3, max=47,629.4, distinct 156 | 向央行借款净增加额同比增长率（%） |
| OFI_BF_ADD_YOY | Nullable(Float64) | 273,768 | 零值 1；负值 134 | min=-297,879, max=224,643, distinct 214 | 向其他金融机构拆入资金净增加额同比增长率（%） |
| RECEIVE_ORIGIC_PREMIUM_YOY | Nullable(Float64) | 273,753 | 零值 0；负值 113 | min=-525.1, max=9,509.7, distinct 260 | 收到原保险合同保费现金同比增长率（%） |
| RECEIVE_REINSURE_NET_YOY | Nullable(Float64) | 273,983 | 零值 0；负值 13 | min=-6,568.25, max=1,347.09, distinct 32 | 收到再保险业务现金净额同比增长率（%） |
| INSURED_INVEST_ADD_YOY | Nullable(Float64) | 273,988 | 零值 0；负值 16 | min=-103.565, max=43,694.8, distinct 28 | 保户储金及投资款净增加额同比增长率（%） |
| DISPOSAL_TFA_ADD_YOY | Nullable(Float64) | 273,896 | 零值 0；负值 64 | min=-187,666, max=9,183.6, distinct 118 | 处置交易性金融资产净增加额同比增长率（%） |
| RECEIVE_INTEREST_COMMISSION_YOY | Nullable(Float64) | 269,242 | 零值 1；负值 2,406 | min=-1,515.19, max=1,982,570, distinct 4,695 | 收取利息和手续费现金同比增长率（%） |
| BORROW_FUND_ADD_YOY | Nullable(Float64) | 273,734 | 零值 1；负值 139 | min=-552,574, max=24,222,400, distinct 248 | 拆入资金净增加额同比增长率（%） |
| LOAN_ADVANCE_REDUCE_YOY | Nullable(Float64) | 273,985 | 零值 0；负值 15 | min=-203.556, max=648.488, distinct 30 | 发放贷款及垫款净减少额同比增长率（%） |
| REPO_BUSINESS_ADD_YOY | Nullable(Float64) | 273,446 | 零值 0；负值 282 | min=-8,980.87, max=1,070,080, distinct 542 | 回购业务资金净增加额同比增长率（%） |
| RECEIVE_TAX_REFUND_YOY | Nullable(Float64) | 127,070 | 零值 24；负值 72,348 | min=-1,404,460,000, max=532,723,000,000, distinct 141,663 | 收到的税费返还同比增长率（%） |
| RECEIVE_OTHER_OPERATE_YOY | Nullable(Float64) | 16,644 | 零值 21；负值 121,126 | min=-138,254,000, max=147,739,000, distinct 257,029 | 收到其他与经营活动有关的现金同比增长率（%） |
| OPERATE_INFLOW_OTHER_YOY | Nullable(Float64) | 273,072 | 零值 0；负值 482 | min=-28,915.3, max=437,791, distinct 905 | 经营活动现金流入其他同比增长率（%） |
| OPERATE_INFLOW_BALANCE_YOY | Nullable(Float64) | 273,886 | 零值 3；负值 71 | min=-200, max=273.251, distinct 25 | 经营活动现金流入平衡项同比增长率（%） |
| TOTAL_OPERATE_INFLOW_YOY | Nullable(Float64) | 14,441 | 零值 16；负值 101,827 | min=-31,189,400, max=638,650,000, distinct 259,341 | 经营活动现金流入小计同比增长率（%） |
| BUY_SERVICES_YOY | Nullable(Float64) | 15,559 | 零值 17；负值 106,935 | min=-408,922, max=404,240,000, distinct 258,150 | 购买商品、接受劳务支付的现金同比增长率（%） |
| LOAN_ADVANCE_ADD_YOY | Nullable(Float64) | 270,631 | 零值 2；负值 1,694 | min=-2,175,480, max=14,967,600, distinct 3,284 | 发放贷款及垫款净增加额同比增长率（%） |
| PBC_INTERBANK_ADD_YOY | Nullable(Float64) | 273,030 | 零值 0；负值 477 | min=-12,906.7, max=156,233, distinct 970 | 向央行借款净增加额同比增长率（%） |
| PAY_ORIGIC_COMPENSATE_YOY | Nullable(Float64) | 273,880 | 零值 0；负值 53 | min=-100, max=6,825.2, distinct 134 | 支付原保险合同赔付款项现金同比增长率（%） |
| PAY_INTEREST_COMMISSION_YOY | Nullable(Float64) | 271,255 | 零值 0；负值 1,364 | min=-18,425.8, max=53,023,000, distinct 2,713 | 支付利息和手续费现金同比增长率（%） |
| PAY_POLICY_BONUS_YOY | Nullable(Float64) | 273,961 | 零值 0；负值 28 | min=-1,058.98, max=526.324, distinct 55 | 保单红利支出同比增长率（%） |
| PAY_STAFF_CASH_YOY | Nullable(Float64) | 14,445 | 零值 18；负值 79,897 | min=-42,334.6, max=933,889,000,000, distinct 259,319 | 支付给职工以及为职工支付的现金同比增长率（%） |
| PAY_ALL_TAX_YOY | Nullable(Float64) | 14,965 | 零值 16；负值 117,830 | min=-4,774,600,000, max=271,037,000, distinct 258,755 | 支付的各项税费同比增长率（%） |
| PAY_OTHER_OPERATE_YOY | Nullable(Float64) | 14,669 | 零值 17；负值 117,082 | min=-12,439,200, max=147,416,000,000, distinct 259,087 | 支付其他与经营活动有关的现金同比增长率（%） |
| OPERATE_OUTFLOW_OTHER_YOY | Nullable(Float64) | 273,139 | 零值 0；负值 451 | min=-652,093, max=7,921,620, distinct 834 | 经营活动现金流出其他同比增长率（%） |
| OPERATE_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 273,833 | 零值 4；负值 90 | min=-1,598.5, max=200, distinct 24 | 经营活动现金流出平衡项同比增长率（%） |
| TOTAL_OPERATE_OUTFLOW_YOY | Nullable(Float64) | 14,376 | 零值 16；负值 101,728 | min=-79,912.4, max=13,594,200,000,000, distinct 259,406 | 经营活动现金流出小计同比增长率（%） |
| OPERATE_NETCASH_OTHER_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动净现金流量其他同比增长率（%） |
| OPERATE_NETCASH_BALANCE_YOY | Nullable(Float64) | 273,790 | 零值 6；负值 118 | min=-3,598.78, max=8,316.9, distinct 23 | 经营活动净现金流量平衡项同比增长率（%） |
| NETCASH_OPERATE_YOY | Nullable(Float64) | 13,241 | 零值 17；负值 127,327 | min=-3,828,320,000,000, max=18,672,100,000, distinct 260,540 | 经营活动产生的现金流量净额同比增长率（%） |
| WITHDRAW_INVEST_YOY | Nullable(Float64) | 191,512 | 零值 333；负值 42,926 | min=-13,269,300,000, max=10,177,200,000,000, distinct 70,751 | 收回投资收到的现金同比增长率（%） |
| RECEIVE_INVEST_INCOME_YOY | Nullable(Float64) | 168,302 | 零值 280；负值 54,741 | min=-34,902,300,000, max=357,366,000,000, distinct 98,248 | 取得投资收益收到的现金同比增长率（%） |
| DISPOSAL_LONG_ASSET_YOY | Nullable(Float64) | 141,727 | 零值 51；负值 68,374 | min=-12,917,900,000, max=470,509,000,000, distinct 121,573 | 处置固定资产等收回的现金净额同比增长率（%） |
| DISPOSAL_SUBSIDIARY_OTHER_YOY | Nullable(Float64) | 269,414 | 零值 9；负值 2,791 | min=-2,816,900,000, max=200,931,000,000, distinct 3,214 | 处置子公司及其他营业单位收到的现金净额同比增长率（%） |
| REDUCE_PLEDGE_TIMEDEPOSITS_YOY | Nullable(Float64) | 273,977 | 零值 0；负值 23 | min=-155.722, max=28,171.4, distinct 39 | 减少质押定期存款同比增长率（%） |
| RECEIVE_OTHER_INVEST_YOY | Nullable(Float64) | 218,612 | 零值 162；负值 28,637 | min=-70,821,700,000, max=534,550,000,000, distinct 50,833 | 收到其他与投资活动有关的现金同比增长率（%） |
| INVEST_INFLOW_OTHER_YOY | Nullable(Float64) | 273,707 | 零值 1；负值 164 | min=-4,178.47, max=2,562,040, distinct 276 | 投资活动现金流入其他同比增长率（%） |
| INVEST_INFLOW_BALANCE_YOY | Nullable(Float64) | 273,963 | 零值 0；负值 38 | min=-200, max=532.083, distinct 16 | 投资活动现金流入平衡项同比增长率（%） |
| TOTAL_INVEST_INFLOW_YOY | Nullable(Float64) | 68,366 | 零值 69；负值 100,344 | min=-65,321,800,000, max=7,337,190,000,000, distinct 198,890 | 投资活动现金流入小计同比增长率（%） |
| CONSTRUCT_LONG_ASSET_YOY | Nullable(Float64) | 19,439 | 零值 18；负值 123,761 | min=-30,613,600, max=858,876,000, distinct 253,682 | 购建固定资产等支付的现金同比增长率（%） |
| INVEST_PAY_CASH_YOY | Nullable(Float64) | 175,776 | 零值 377；负值 50,914 | min=-8,304,010,000, max=708,278,000,000, distinct 83,208 | 投资支付的现金同比增长率（%） |
| PLEDGE_LOAN_ADD_YOY | Nullable(Float64) | 273,968 | 零值 0；负值 22 | min=-107.426, max=3,862.23, distinct 47 | 质押贷款净增加额同比增长率（%） |
| OBTAIN_SUBSIDIARY_OTHER_YOY | Nullable(Float64) | 267,293 | 零值 13；负值 4,044 | min=-80,700,500,000, max=17,602,400,000,000, distinct 5,001 | 取得子公司及其他营业单位支付的现金净额同比增长率（%） |
| ADD_PLEDGE_TIMEDEPOSITS_YOY | Nullable(Float64) | 273,995 | 零值 0；负值 8 | min=-3,044.96, max=331,450, distinct 21 | 增加质押定期存款同比增长率（%） |
| PAY_OTHER_INVEST_YOY | Nullable(Float64) | 230,935 | 零值 116；负值 22,652 | min=-3,333,330,000, max=6,081,250,000,000, distinct 38,242 | 支付其他与投资活动有关的现金同比增长率（%） |
| INVEST_OUTFLOW_OTHER_YOY | Nullable(Float64) | 273,798 | 零值 5；负值 106 | min=-37,000,000,000, max=1,071,130, distinct 182 | 投资活动现金流出其他同比增长率（%） |
| INVEST_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 273,978 | 零值 0；负值 25 | min=-100, max=1,548.69, distinct 8 | 投资活动现金流出平衡项同比增长率（%） |
| TOTAL_INVEST_OUTFLOW_YOY | Nullable(Float64) | 18,003 | 零值 17；负值 121,392 | min=-4,104,510, max=184,095,000,000, distinct 255,234 | 投资活动现金流出小计同比增长率（%） |
| INVEST_NETCASH_OTHER_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 投资活动净现金流量其他同比增长率（%） |
| INVEST_NETCASH_BALANCE_YOY | Nullable(Float64) | 273,972 | 零值 1；负值 17 | min=-200, max=200, distinct 5 | 投资活动净现金流量平衡项同比增长率（%） |
| NETCASH_INVEST_YOY | Nullable(Float64) | 16,830 | 零值 17；负值 132,079 | min=-1,432,960,000, max=18,644,800,000, distinct 256,573 | 投资活动产生的现金流量净额同比增长率（%） |
| ACCEPT_INVEST_CASH_YOY | Nullable(Float64) | 246,920 | 零值 82；负值 16,325 | min=-47,989,000,000,000, max=12,562,600,000,000, distinct 19,444 | 吸收投资收到的现金同比增长率（%） |
| SUBSIDIARY_ACCEPT_INVEST_YOY | Nullable(Float64) | 260,295 | 零值 65；负值 8,257 | min=-4,558,160,000, max=442,567,000,000, distinct 9,852 | 子公司吸收少数股东投资收到的现金同比增长率（%） |
| RECEIVE_LOAN_CASH_YOY | Nullable(Float64) | 108,388 | 零值 1,773；负值 79,958 | min=-3,340,100, max=342,520,000,000, distinct 137,184 | 取得借款收到的现金同比增长率（%） |
| ISSUE_BOND_YOY | Nullable(Float64) | 272,332 | 零值 52；负值 915 | min=-479,704, max=555,530,000, distinct 1,151 | 发行债券收到的现金同比增长率（%） |
| RECEIVE_OTHER_FINANCE_YOY | Nullable(Float64) | 221,158 | 零值 90；负值 27,817 | min=-1,431,390,000,000, max=9,120,050,000,000, distinct 48,255 | 收到其他与筹资活动有关的现金同比增长率（%） |
| FINANCE_INFLOW_OTHER_YOY | Nullable(Float64) | 273,851 | 零值 0；负值 91 | min=-5,794.83, max=1,146,750, distinct 158 | 筹资活动现金流入其他同比增长率（%） |
| FINANCE_INFLOW_BALANCE_YOY | Nullable(Float64) | 273,985 | 零值 0；负值 21 | min=-647.407, max=4,483.01, distinct 7 | 筹资活动现金流入平衡项同比增长率（%） |
| TOTAL_FINANCE_INFLOW_YOY | Nullable(Float64) | 83,383 | 零值 1,005；负值 92,077 | min=-930,484,000, max=12,623,300,000,000, distinct 172,607 | 筹资活动现金流入小计同比增长率（%） |
| PAY_DEBT_CASH_YOY | Nullable(Float64) | 97,560 | 零值 1,950；负值 83,040 | min=-643,929, max=1,101,280,000,000, distinct 153,250 | 偿还债务支付的现金同比增长率（%） |
| ASSIGN_DIVIDEND_PORFIT_YOY | Nullable(Float64) | 57,731 | 零值 156；负值 103,387 | min=-296,282,000,000, max=3,719,590,000,000, distinct 211,390 | 分配股利、利润或偿付利息支付的现金同比增长率（%） |
| SUBSIDIARY_PAY_DIVIDEND_YOY | Nullable(Float64) | 258,181 | 零值 183；负值 8,409 | min=-389,385,000,000, max=4,311,140,000,000, distinct 12,755 | 子公司向少数股东支付的现金股利同比增长率（%） |
| BUY_SUBSIDIARY_EQUITY_YOY | Nullable(Float64) | 273,999 | 零值 0；负值 10 | min=-100, max=3,012.24, distinct 15 | 子公司减少现金同比增长率（%） |
| PAY_OTHER_FINANCE_YOY | Nullable(Float64) | 157,247 | 零值 457；负值 58,135 | min=-981,740,000,000, max=2,046,000,000,000, distinct 111,244 | 支付其他与筹资活动有关的现金同比增长率（%） |
| SUBSIDIARY_REDUCE_CASH_YOY | Nullable(Float64) | 274,013 | 零值 0；负值 2 | min=-100, max=355.556, distinct 3 | 子公司减少现金同比增长率（%） |
| FINANCE_OUTFLOW_OTHER_YOY | Nullable(Float64) | 273,675 | 零值 2；负值 172 | min=-4,465.22, max=6,357,070,000, distinct 290 | 筹资活动现金流出其他同比增长率（%） |
| FINANCE_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 273,961 | 零值 0；负值 33 | min=-101,680,000, max=237.346, distinct 11 | 筹资活动现金流出平衡项同比增长率（%） |
| TOTAL_FINANCE_OUTFLOW_YOY | Nullable(Float64) | 39,078 | 零值 119；负值 107,787 | min=-6,601,720,000, max=497,708,000,000, distinct 231,312 | 筹资活动现金流出小计同比增长率（%） |
| FINANCE_NETCASH_OTHER_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 筹资活动净现金流量其他同比增长率（%） |
| FINANCE_NETCASH_BALANCE_YOY | Nullable(Float64) | 273,992 | 零值 0；负值 13 | min=-100, max=100, distinct 2 | 筹资活动净现金流量平衡项同比增长率（%） |
| NETCASH_FINANCE_YOY | Nullable(Float64) | 34,285 | 零值 106；负值 118,442 | min=-497,708,000,000, max=766,184,000,000, distinct 236,496 | 筹资活动产生的现金流量净额同比增长率（%） |
| RATE_CHANGE_EFFECT_YOY | Nullable(Float64) | 118,453 | 零值 28；负值 82,243 | min=-129,940,000,000, max=8,053,210,000, distinct 154,332 | 汇率变动对现金及现金等价物的影响同比增长率（%） |
| CCE_ADD_OTHER_YOY | Nullable(Float64) | 274,010 | 零值 0；负值 2 | min=-117.675, max=1,018.98, distinct 6 | 现金及现金等价物净增加额其他同比增长率（%） |
| CCE_ADD_BALANCE_YOY | Nullable(Float64) | 273,749 | 零值 8；负值 120 | min=-8,316.9, max=3,598.78, distinct 28 | 现金及现金等价物净增加额平衡项同比增长率（%） |
| CCE_ADD_YOY | Nullable(Float64) | 14,347 | 零值 17；负值 128,884 | min=-8,362,480,000,000, max=2,014,360,000, distinct 259,430 | 现金及现金等价物净增加额同比增长率（%） |
| BEGIN_CCE_YOY | Nullable(Float64) | 30,065 | 零值 133；负值 110,958 | min=-6,947.67, max=1,279,600,000,000, distinct 243,646 | 期初现金及现金等价物余额同比增长率（%） |
| END_CCE_OTHER_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 期末现金及现金等价物余额其他同比增长率（%） |
| END_CCE_BALANCE_YOY | Nullable(Float64) | 273,966 | 零值 0；负值 24 | min=-159.7, max=244.416, distinct 6 | 期末现金及现金等价物余额平衡项同比增长率（%） |
| END_CCE_YOY | Nullable(Float64) | 29,715 | 零值 22；负值 112,158 | min=-6,947.67, max=5,209,810, distinct 244,110 | 期末现金及现金等价物余额同比增长率（%） |
| NETPROFIT_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 净利润同比增长率（%） |
| ASSET_IMPAIRMENT_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 资产减值准备同比增长率（%） |
| FA_IR_DEPR_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 固定资产折旧、油气资产折耗、生产性生物资产折旧同比增长率（%） |
| OILGAS_BIOLOGY_DEPR_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 油气资产折耗、生产性生物资产折旧同比增长率（%） |
| IR_DEPR_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 折旧与摊销同比增长率（%） |
| IA_AMORTIZE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 无形资产摊销同比增长率（%） |
| LPE_AMORTIZE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 长期待摊费用摊销同比增长率（%） |
| DEFER_INCOME_AMORTIZE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 待摊费用减少（减：增加）同比增长率（%） |
| PREPAID_EXPENSE_REDUCE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预提费用增加（减：减少）同比增长率（%） |
| ACCRUED_EXPENSE_ADD_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预提费用变动同比增长率（%） |
| DISPOSAL_LONGASSET_LOSS_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 处置固定资产等的损失同比增长率（%） |
| FA_SCRAP_LOSS_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 固定资产报废损失同比增长率（%） |
| FAIRVALUE_CHANGE_LOSS_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 公允价值变动损失同比增长率（%） |
| FINANCE_EXPENSE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 财务费用同比增长率（%） |
| INVEST_LOSS_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 投资损失同比增长率（%） |
| DEFER_TAX_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税资产减少（增加以"-"号填列）同比增长率（%） |
| DT_ASSET_REDUCE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税资产减少同比增长率（%） |
| DT_LIAB_ADD_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 递延所得税负债增加同比增长率（%） |
| PREDICT_LIAB_ADD_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 预计负债增加同比增长率（%） |
| INVENTORY_REDUCE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 存货的减少（增加以"-"号填列）同比增长率（%） |
| OPERATE_RECE_REDUCE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营性应收项目的减少同比增长率（%） |
| OPERATE_PAYABLE_ADD_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营性应付项目的增加同比增长率（%） |
| OTHER_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 其他同比增长率（%） |
| OPERATE_NETCASH_OTHERNOTE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动产生的现金流量净额（附注）同比增长率（%） |
| OPERATE_NETCASH_BALANCENOTE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动净现金流量（附注）平衡项同比增长率（%） |
| NETCASH_OPERATENOTE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动产生的现金流量净额（附注）同比增长率（%） |
| DEBT_TRANSFER_CAPITAL_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 债务转为资本同比增长率（%） |
| CONVERT_BOND_1YEAR_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 一年内到期的可转换公司债券同比增长率（%） |
| FINLEASE_OBTAIN_FA_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 融资租入固定资产同比增长率（%） |
| UNINVOLVE_INVESTFIN_OTHER_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 不涉及现金收支的投资和筹资活动其他同比增长率（%） |
| END_CASH_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金期末余额同比增长率（%） |
| BEGIN_CASH_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金期初余额同比增长率（%） |
| END_CASH_EQUIVALENTS_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金等价物期末余额同比增长率（%） |
| BEGIN_CASH_EQUIVALENTS_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金等价物期初余额同比增长率（%） |
| CCE_ADD_OTHERNOTE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注）同比增长率（%） |
| CCE_ADD_BALANCENOTE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注）平衡项同比增长率（%） |
| CCE_ADDNOTE_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 现金及现金等价物净增加额（附注）同比增长率（%） |
| MINORITY_INTEREST_YOY | Nullable(Float64) | 274,016 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 少数股东损益同比增长率（%） |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 274,016/274,016，供应商前缀 0/274,016，纯数字 0/274,016，空值 0/274,016；`SECURITY_CODE`: canonical 后缀 0/274,016，供应商前缀 0/274,016，纯数字 274,016/274,016，空值 0/274,016
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：`REPORT_DATE`: 2000-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 2001-03-15 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 2001-03-15 至 2026-06-02，NULL 449 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME`, `SECURITY_TYPE_CODE`, `CURRENCY`
- 取值：`SECURITY_CODE`: `600019`(97), `000402`(97), `000951`(95), `600302`(95), `000025`(95), `000410`(95), `000798`(95), `000619`(95)；`SECURITY_NAME_ABBR`: `东方明珠`(142), `百联股份`(127), `宝钢股份`(97), `金融街`(97), `*ST广糖`(95), `云维股份`(95), `国电电力`(95), `中国重汽`(95)；`ORG_CODE`: `10004127`(160), `10004106`(156), `10116535`(126), `10004293`(116), `10002266`(97), `10633808`(97), `10005578`(95), `10002608`(95)；`ORG_TYPE`: `通用`(274,016)；`REPORT_TYPE`: `一季度`(71,797), `三季度`(68,255), `四季度`(67,850), `二季度`(66,114)；`REPORT_DATE_NAME`: `2026一季度`(5,099), `2025一季度`(5,088), `2025四季度`(5,084), `2024一季度`(5,073), `2025三季度`(5,060), `2025二季度`(5,051), `2024三季度`(5,034), `2024四季度`(5,020)；`SECURITY_TYPE_CODE`: `058001001`(273,993), `058001008`(23)；`CURRENCY`: `CNY`(273,610), `NULL`(406)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 357 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 357 个数值字段执行 min/max、NULL、零值和负值检查；其中 220 个字段出现负值，178 个字段出现零值，254 个字段 NULL 数不低于 80%。 负值字段样例：`NETCASH_INVEST` 209,389 行(min=-125,557,000,000)，`NETCASH_FINANCE` 144,970 行(min=-104,203,000,000)，`CCE_ADD` 140,834 行(min=-105,279,000,000)，`NETCASH_INVEST_QOQ` 135,735 行(min=-927,434,000)，`NETCASH_INVEST_YOY` 132,079 行(min=-1,432,960,000)，`PAY_ALL_TAX_QOQ` 131,816 行(min=-70,182.5)，`TOTAL_INVEST_OUTFLOW_QOQ` 131,493 行(min=-1,212,750)，`CONSTRUCT_LONG_ASSET_QOQ` 130,642 行(min=-952,765)。 高 NULL 字段样例：`OPERATE_INFLOW_BALANCE_QOQ` 274,016 行，`OPERATE_OUTFLOW_BALANCE_QOQ` 274,016 行，`OPERATE_NETCASH_OTHER_QOQ` 274,016 行，`OPERATE_NETCASH_BALANCE_QOQ` 274,016 行，`INVEST_INFLOW_BALANCE_QOQ` 274,016 行，`INVEST_OUTFLOW_BALANCE_QOQ` 274,016 行，`INVEST_NETCASH_OTHER_QOQ` 274,016 行，`INVEST_NETCASH_BALANCE_QOQ` 274,016 行。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECURITY_CODE` 为 6 位本地代码 | 中 | 274,016/274,016 行为纯数字 | 只作为 `security_local_code`，不可单独推出交易所 | 交易所归属或证券主数据修正延后 |
| 财务数值存在负值 | 低 | 220 个数值字段出现负值 | 负数符合财务科目/调整项可能性，staging 不过滤 | 口径解释和异常阈值延后 |

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

- 行数：274,016。
- 日期 / 分区范围：`REPORT_DATE`: 2000-12-31 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 2001-03-15 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 2001-03-15 至 2026-06-02，NULL 449 行，`1970-01-01` 占位 0 行
- 候选键重复：未发现重复。
- 关键 NULL / 占位值：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`SECURITY_CODE`: `600019`(97), `000402`(97), `000951`(95), `600302`(95), `000025`(95), `000410`(95), `000798`(95), `000619`(95)；`SECURITY_NAME_ABBR`: `东方明珠`(142), `百联股份`(127), `宝钢股份`(97), `金融街`(97), `*ST广糖`(95), `云维股份`(95), `国电电力`(95), `中国重汽`(95)；`ORG_CODE`: `10004127`(160), `10004106`(156), `10116535`(126), `10004293`(116), `10002266`(97), `10633808`(97), `10005578`(95), `10002608`(95)；`ORG_TYPE`: `通用`(274,016)；`REPORT_TYPE`: `一季度`(71,797), `三季度`(68,255), `四季度`(67,850), `二季度`(66,114)；`REPORT_DATE_NAME`: `2026一季度`(5,099), `2025一季度`(5,088), `2025四季度`(5,084), `2024一季度`(5,073), `2025三季度`(5,060), `2025二季度`(5,051), `2024三季度`(5,034), `2024四季度`(5,020)；`SECURITY_TYPE_CODE`: `058001001`(273,993), `058001008`(23)；`CURRENCY`: `CNY`(273,610), `NULL`(406)
- 数值范围：已对 357 个数值字段执行 min/max、NULL、零值和负值检查；其中 220 个字段出现负值，178 个字段出现零值，254 个字段 NULL 数不低于 80%。 负值字段样例：`NETCASH_INVEST` 209,389 行(min=-125,557,000,000)，`NETCASH_FINANCE` 144,970 行(min=-104,203,000,000)，`CCE_ADD` 140,834 行(min=-105,279,000,000)，`NETCASH_INVEST_QOQ` 135,735 行(min=-927,434,000)，`NETCASH_INVEST_YOY` 132,079 行(min=-1,432,960,000)，`PAY_ALL_TAX_QOQ` 131,816 行(min=-70,182.5)，`TOTAL_INVEST_OUTFLOW_QOQ` 131,493 行(min=-1,212,750)，`CONSTRUCT_LONG_ASSET_QOQ` 130,642 行(min=-952,765)。 高 NULL 字段样例：`OPERATE_INFLOW_BALANCE_QOQ` 274,016 行，`OPERATE_OUTFLOW_BALANCE_QOQ` 274,016 行，`OPERATE_NETCASH_OTHER_QOQ` 274,016 行，`OPERATE_NETCASH_BALANCE_QOQ` 274,016 行，`INVEST_INFLOW_BALANCE_QOQ` 274,016 行，`INVEST_OUTFLOW_BALANCE_QOQ` 274,016 行，`INVEST_NETCASH_OTHER_QOQ` 274,016 行，`INVEST_NETCASH_BALANCE_QOQ` 274,016 行。

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
select `SECUCODE`, `REPORT_DATE`, `SECURITY_CODE`, `NOTICE_DATE`, `UPDATE_DATE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME` from fleur_raw.eastmoney__cashflow_sq limit 5
```

结果：

```text
[{'SECUCODE': '000619.SZ', 'REPORT_DATE': datetime.date(2001, 9, 30), 'SECURITY_CODE': '000619', 'NOTICE_DATE': datetime.date(2001, 10, 25), 'UPDATE_DATE': datetime.date(2001, 10, 25), 'SECURITY_NAME_ABBR': '海螺新材', 'ORG_CODE': '10005558', 'ORG_TYPE': '通用', 'REPORT_TYPE': '三季度', 'REPORT_DATE_NAME': '2001三季度'}, {'SECUCODE': '000629.SZ', 'REPORT_DATE': datetime.date(2001, 9, 30), 'SECURITY_CODE': '000629', 'NOTICE_DATE': datetime.date(2001, 12, 18), 'UPDATE_DATE': datetime.date(2001, 12, 18), 'SECURITY_NAME_ABBR': '钒钛股份', 'ORG_CODE': '10005567', 'ORG_TYPE': '通用', 'REPORT_TYPE': '三季度', 'REPORT_DATE_NAME': '2001三季度'}, {'SECUCODE': '600247.SH', 'REPORT_DATE': datetime.date(2001, 9, 30), 'SECURITY_CODE': '600247', 'NOTICE_DATE': datetime.date(2001, 10, 26), 'UPDATE_DATE': datetime.date(2001, 10, 26), 'SECURITY_NAME_ABBR': '*ST成城', 'ORG_CODE': '10002453', 'ORG_TYPE': '通用', 'REPORT_TYPE': '三季度', 'REPORT_DATE_NAME': '2001三季度'}, {'SECUCODE': '600250.SH', 'REPORT_DATE': datetime.date(2000, 12, 31), 'SECURITY_CODE': '600250', 'NOTICE_DATE': datetime.date(2001, 4, 18), 'UPDATE_DATE': datetime.date(2001, 4, 18), 'SECURITY_NAME_ABBR': '南京商旅', 'ORG_CODE': '10002455', 'ORG_TYPE': '通用', 'REPORT_TYPE': '四季度', 'REPORT_DATE_NAME': '2000四季度'}, {'SECUCODE': '600306.SH', 'REPORT_DATE': datetime.date(2000, 12, 31), 'SECURITY_CODE': '600306', 'NOTICE_DATE': datetime.date(2001, 3, 22), 'UPDATE_DATE': datetime.date(2001, 3, 22), 'SECURITY_NAME_ABBR': '退市商城', 'ORG_CODE': '10002500', 'ORG_TYPE': '通用', 'REPORT_TYPE': '四季度', 'REPORT_DATE_NAME': '2000四季度'}]
```

### 行数统计

```sql
select count() from fleur_raw.eastmoney__cashflow_sq
```

结果：

```text
[[274016]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `SECUCODE`, `REPORT_DATE`, count() as row_count from fleur_raw.eastmoney__cashflow_sq group by `SECUCODE`, `REPORT_DATE` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 0, 'max_rows_per_key': 0}
```

### 证券代码格式：SECUCODE

```sql
select countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__cashflow_sq
```

结果：

```text
{'canonical_suffix': 274016, 'vendor_prefix': 0, 'numeric_only': 0, 'empty_or_null': 0, 'row_count': 274016}
```

### 证券代码格式：SECURITY_CODE

```sql
select countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__cashflow_sq
```

结果：

```text
{'canonical_suffix': 0, 'vendor_prefix': 0, 'numeric_only': 274016, 'empty_or_null': 0, 'row_count': 274016}
```
