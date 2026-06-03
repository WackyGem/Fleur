# Raw 数据画像：eastmoney__cashflow_ytd

日期：2026-06-03

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__cashflow_ytd.yml`
- dbt source：`source('raw', 'eastmoney__cashflow_ytd')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__cashflow_ytd.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__cashflow_ytd`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__cashflow_ytd --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__cashflow_ytd.md`
- 行数：283,613
- 数据范围：`REPORT_DATE`: 1998-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1998-07-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1998-07-14 至 2026-06-02，NULL 686 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`eastmoney__cashflow_ytd`
- ClickHouse raw 表：`fleur_raw.eastmoney__cashflow_ytd`
- 表说明：EastMoney year-to-date cashflow F10 rows by natural-year raw partition.

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：283,613。
  - 覆盖主体数：`secucode` 5,420 个；`security_code` 5,420 个
  - 日期 / 分区范围：`REPORT_DATE`: 1998-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1998-07-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1998-07-14 至 2026-06-02，NULL 686 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `SECUCODE`, `REPORT_DATE`。
  - 候选自然键去重结果：未发现重复。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：`SECUCODE`: canonical 后缀 283,613/283,613，供应商前缀 0/283,613，纯数字 0/283,613，空值 0/283,613；`SECURITY_CODE`: canonical 后缀 0/283,613，供应商前缀 0/283,613，纯数字 283,613/283,613，空值 0/283,613
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`SECURITY_CODE`: `000766`(104), `000678`(104), `600795`(104), `600798`(103), `000796`(103), `000722`(103), `000758`(103), `600699`(103)；`SECURITY_NAME_ABBR`: `东方明珠`(159), `百联股份`(145), `通化金马`(104), `国电电力`(104), `襄阳轴承`(104), `郑州煤电`(103), `东百集团`(103), `湖南发展`(103)；`ORG_CODE`: `10004127`(176), `10004106`(174), `10004293`(132), `10116535`(128), `10005673`(104), `10005602`(104), `10634823`(104), `10004008`(103)；`ORG_TYPE`: `通用`(283,613)；`REPORT_TYPE`: `年报`(73,138), `一季报`(71,797), `中报`(70,167), `三季报`(68,511)；`REPORT_DATE_NAME`: `2026一季报`(5,099), `2025一季报`(5,088), `2025年报`(5,084), `2024一季报`(5,073), `2025三季报`(5,060), `2025中报`(5,051), `2024三季报`(5,034), `2024年报`(5,020)；`SECURITY_TYPE_CODE`: `058001001`(283,590), `058001008`(23)；`CURRENCY`: `CNY`(282,968), `NULL`(645)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`REPORT_DATE`: 1998-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1998-07-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1998-07-14 至 2026-06-02，NULL 686 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 240 个数值字段执行 min/max、NULL、零值和负值检查；其中 227 个字段出现负值，212 个字段出现零值，121 个字段 NULL 数不低于 80%。 负值字段样例：`NETCASH_INVEST` 231,278 行(min=-332,948,000,000)，`CCE_ADD` 156,640 行(min=-91,197,500,000)，`NETCASH_FINANCE` 143,270 行(min=-178,876,000,000)，`NETCASH_INVEST_YOY` 140,760 行(min=-288,839,000)，`NETCASH_OPERATE_YOY` 132,660 行(min=-2,362,750,000)，`CCE_ADD_YOY` 130,449 行(min=-2,362,750,000)，`CONSTRUCT_LONG_ASSET_YOY` 129,487 行(min=-1,022,600)，`NETCASH_FINANCE_YOY` 127,894 行(min=-1,195,570,000)。 高 NULL 字段样例：`OPERATE_NETCASH_OTHER_YOY` 283,613 行，`FINANCE_NETCASH_OTHER_YOY` 283,613 行，`INVEST_NETCASH_OTHER_YOY` 283,612 行，`SUBSIDIARY_REDUCE_CASH_YOY` 283,606 行，`END_CCE_OTHER_YOY` 283,605 行，`CCE_ADD_OTHER_YOY` 283,593 行，`INSURED_INVEST_ADD_YOY` 283,583 行，`END_CCE_OTHER` 283,576 行。
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
| SECUCODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,420 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,420 | 证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,418 | 证券简称 |
| ORG_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,413 | 机构代码 |
| ORG_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 机构类型 |
| REPORT_DATE | Date | 0 | `1970-01-01` 0 | 1998-06-30 至 2026-03-31; distinct 106 | 报告期 |
| REPORT_TYPE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 4 | 报告类型 |
| REPORT_DATE_NAME | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 106 | 报告期名称 |
| SECURITY_TYPE_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 2 | 证券类型代码 |
| NOTICE_DATE | Date | 0 | `1970-01-01` 0 | 1998-07-14 至 2026-05-15; distinct 3,697 | 公告日期 |
| UPDATE_DATE | Nullable(Date) | 686 | `1970-01-01` 0 | 1998-07-14 至 2026-06-02; distinct 4,563 | 更新日期 |
| CURRENCY | LowCardinality(Nullable(String)) | 645 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 现金流量表年初至报告期末金额使用的币种。 |
| SALES_SERVICES | Nullable(Float64) | 600 | 零值 18；负值 9 | min=-29,150,600, max=3,577,810,000,000, distinct 282,610 | 销售商品、提供劳务收到的现金 |
| DEPOSIT_INTERBANK_ADD | Nullable(Float64) | 278,191 | 零值 3,668；负值 628 | min=-75,888,800,000, max=78,286,000,000, distinct 1,735 | 同业存放净增加额 |
| LOAN_PBC_ADD | Nullable(Float64) | 279,617 | 零值 3,685；负值 117 | min=-5,080,000,000, max=5,227,670,000, distinct 274 | 向央行借款净增加额 |
| OFI_BF_ADD | Nullable(Float64) | 279,395 | 零值 3,692；负值 177 | min=-16,138,000,000, max=28,859,700,000, distinct 384 | 向其他金融机构拆入资金净增加额 |
| RECEIVE_ORIGIC_PREMIUM | Nullable(Float64) | 279,773 | 零值 3,497；负值 0 | min=0, max=20,983,200,000, distinct 338 | 收到原保险合同保费现金 |
| RECEIVE_REINSURE_NET | Nullable(Float64) | 280,027 | 零值 3,504；负值 31 | min=-314,030,000, max=807,229,000, distinct 80 | 收到再保险业务现金净额 |
| INSURED_INVEST_ADD | Nullable(Float64) | 279,976 | 零值 3,590；负值 2 | min=-39,673,700, max=5,016,100,000, distinct 48 | 保户储金及投资款净增加额 |
| DISPOSAL_TFA_ADD | Nullable(Float64) | 281,739 | 零值 1,579；负值 67 | min=-5,953,790,000, max=24,533,300,000, distinct 286 | 处置交易性金融资产净增加额 |
| RECEIVE_INTEREST_COMMISSION | Nullable(Float64) | 274,004 | 零值 3,631；负值 3 | min=-45,945,000, max=32,481,400,000, distinct 5,895 | 收取利息和手续费现金 |
| BORROW_FUND_ADD | Nullable(Float64) | 279,298 | 零值 3,770；负值 191 | min=-36,889,400,000, max=19,361,700,000, distinct 381 | 拆入资金净增加额 |
| LOAN_ADVANCE_REDUCE | Nullable(Float64) | 262,562 | 零值 20,946；负值 0 | min=0, max=70,142,100,000, distinct 103 | 发放贷款及垫款净减少额 |
| REPO_BUSINESS_ADD | Nullable(Float64) | 278,990 | 零值 3,747；负值 233 | min=-11,374,200,000, max=46,311,600,000, distinct 824 | 回购业务资金净增加额 |
| RECEIVE_TAX_REFUND | Nullable(Float64) | 81,463 | 零值 3,086；负值 37 | min=-391,157,000, max=23,966,600,000, distinct 186,691 | 收到的税费返还 |
| RECEIVE_OTHER_OPERATE | Nullable(Float64) | 1,603 | 零值 59；负值 198 | min=-3,673,130,000, max=301,973,000,000, distinct 281,304 | 收到其他与经营活动有关的现金 |
| OPERATE_INFLOW_OTHER | Nullable(Float64) | 230,483 | 零值 51,594；负值 51 | min=-9,983,900,000, max=57,640,300,000, distinct 1,520 | 经营活动现金流入其他 |
| OPERATE_INFLOW_BALANCE | Nullable(Float64) | 43,128 | 零值 237,658；负值 188 | min=-3,314,250,000, max=9,802,000,000, distinct 2,432 | 经营活动现金流入平衡项 |
| TOTAL_OPERATE_INFLOW | Nullable(Float64) | 228 | 零值 9；负值 44 | min=-42,058,800,000, max=3,832,040,000,000, distinct 283,067 | 经营活动现金流入小计 |
| BUY_SERVICES | Nullable(Float64) | 916 | 零值 28；负值 27 | min=-296,596,000, max=2,919,750,000,000, distinct 282,269 | 购买商品、接受劳务支付的现金 |
| LOAN_ADVANCE_ADD | Nullable(Float64) | 275,025 | 零值 3,836；负值 1,687 | min=-53,617,900,000, max=51,675,100,000, distinct 4,523 | 发放贷款及垫款净增加额 |
| PBC_INTERBANK_ADD | Nullable(Float64) | 278,402 | 零值 3,810；负值 554 | min=-11,235,700,000, max=48,933,100,000, distinct 1,391 | 向央行借款净增加额 |
| PAY_ORIGIC_COMPENSATE | Nullable(Float64) | 279,796 | 零值 3,631；负值 0 | min=0, max=3,490,750,000, distinct 184 | 支付原保险合同赔付款项现金 |
| PAY_INTEREST_COMMISSION | Nullable(Float64) | 276,202 | 零值 3,810；负值 9 | min=-8,685,800, max=20,758,200,000, distinct 3,533 | 支付利息和手续费现金 |
| PAY_POLICY_BONUS | Nullable(Float64) | 279,732 | 零值 3,791；负值 1 | min=-62,965,600, max=4,005,050,000, distinct 91 | 保单红利支出 |
| PAY_STAFF_CASH | Nullable(Float64) | 229 | 零值 8；负值 1 | min=-6,051,810, max=185,223,000,000, distinct 283,043 | 支付给职工以及为职工支付的现金 |
| PAY_ALL_TAX | Nullable(Float64) | 493 | 零值 23；负值 60 | min=-105,026,000, max=449,034,000,000, distinct 282,684 | 支付的各项税费 |
| PAY_OTHER_OPERATE | Nullable(Float64) | 367 | 零值 22；负值 98 | min=-3,549,820,000, max=312,819,000,000, distinct 282,868 | 支付其他与经营活动有关的现金 |
| OPERATE_OUTFLOW_OTHER | Nullable(Float64) | 230,634 | 零值 51,577；负值 123 | min=-17,288,400,000, max=85,692,000,000, distinct 1,347 | 经营活动现金流出其他 |
| OPERATE_OUTFLOW_BALANCE | Nullable(Float64) | 43,073 | 零值 237,462；负值 357 | min=-3,264,310,000, max=6,871,000,000, distinct 2,383 | 经营活动现金流出平衡项 |
| TOTAL_OPERATE_OUTFLOW | Nullable(Float64) | 194 | 零值 8；负值 59 | min=-5,103,120,000, max=3,715,770,000,000, distinct 283,104 | 经营活动现金流出小计 |
| OPERATE_NETCASH_OTHER | Nullable(Float64) | 262,974 | 零值 20,639；负值 0 | min=0, max=0, distinct 1 | 经营活动净现金流量其他 |
| OPERATE_NETCASH_BALANCE | Nullable(Float64) | 43,083 | 零值 239,785；负值 347 | min=-692,800,000, max=2,014,120,000, distinct 271 | 经营活动净现金流量平衡项 |
| NETCASH_OPERATE | Nullable(Float64) | 2 | 零值 7；负值 105,496 | min=-122,530,000,000, max=456,847,000,000, distinct 283,294 | 经营活动产生的现金流量净额 |
| WITHDRAW_INVEST | Nullable(Float64) | 131,897 | 零值 5,403；负值 117 | min=-602,709,000, max=485,318,000,000, distinct 93,658 | 收回投资收到的现金 |
| RECEIVE_INVEST_INCOME | Nullable(Float64) | 107,210 | 零值 4,541；负值 416 | min=-173,671,000, max=33,300,800,000, distinct 147,185 | 取得投资收益收到的现金 |
| DISPOSAL_LONG_ASSET | Nullable(Float64) | 65,289 | 零值 3,116；负值 1,059 | min=-435,937,000, max=38,828,000,000, distinct 161,847 | 处置固定资产等收回的现金净额 |
| DISPOSAL_SUBSIDIARY_OTHER | Nullable(Float64) | 245,647 | 零值 8,457；负值 1,727 | min=-20,231,800,000, max=82,767,000,000, distinct 20,149 | 处置子公司及其他营业单位收到的现金净额 |
| REDUCE_PLEDGE_TIMEDEPOSITS | Nullable(Float64) | 283,300 | 零值 202；负值 0 | min=0, max=67,221,000,000, distinct 110 | 减少质押定期存款 |
| RECEIVE_OTHER_INVEST | Nullable(Float64) | 169,636 | 零值 7,676；负值 126 | min=-231,491,000, max=187,643,000,000, distinct 82,373 | 收到其他与投资活动有关的现金 |
| INVEST_INFLOW_OTHER | Nullable(Float64) | 233,183 | 零值 49,653；负值 3 | min=-1,489,220,000, max=21,741,500,000, distinct 724 | 投资活动现金流入其他 |
| INVEST_INFLOW_BALANCE | Nullable(Float64) | 57,695 | 零值 222,853；负值 86 | min=-1,513,680,000, max=4,618,000,000, distinct 2,781 | 投资活动现金流入平衡项 |
| TOTAL_INVEST_INFLOW | Nullable(Float64) | 20,844 | 零值 4,876；负值 791 | min=-1,483,350,000, max=507,625,000,000, distinct 237,828 | 投资活动现金流入小计 |
| CONSTRUCT_LONG_ASSET | Nullable(Float64) | 2,340 | 零值 108；负值 86 | min=-211,345,000, max=330,861,000,000, distinct 279,588 | 购建固定资产等支付的现金 |
| INVEST_PAY_CASH | Nullable(Float64) | 108,190 | 零值 5,023；负值 176 | min=-640,442,000, max=505,994,000,000, distinct 100,628 | 投资支付的现金 |
| PLEDGE_LOAN_ADD | Nullable(Float64) | 278,674 | 零值 4,746；负值 13 | min=-130,047,000, max=6,716,000,000, distinct 187 | 质押贷款净增加额 |
| OBTAIN_SUBSIDIARY_OTHER | Nullable(Float64) | 241,684 | 零值 7,495；负值 1,346 | min=-22,204,700,000, max=90,942,000,000, distinct 24,517 | 取得子公司及其他营业单位支付的现金净额 |
| ADD_PLEDGE_TIMEDEPOSITS | Nullable(Float64) | 283,448 | 零值 78；负值 0 | min=0, max=11,008,700,000, distinct 86 | 增加质押定期存款 |
| PAY_OTHER_INVEST | Nullable(Float64) | 182,944 | 零值 7,850；负值 210 | min=-2,508,810,000, max=157,556,000,000, distinct 65,385 | 支付其他与投资活动有关的现金 |
| INVEST_OUTFLOW_OTHER | Nullable(Float64) | 233,181 | 零值 49,666；负值 14 | min=-164,896,000, max=111,724,000,000, distinct 677 | 投资活动现金流出其他 |
| INVEST_OUTFLOW_BALANCE | Nullable(Float64) | 44,074 | 零值 239,050；负值 83 | min=-9,351,000,000, max=26,776,400,000, distinct 408 | 投资活动现金流出平衡项 |
| TOTAL_INVEST_OUTFLOW | Nullable(Float64) | 1,624 | 零值 235；负值 189 | min=-16,026,100,000, max=534,645,000,000, distinct 280,487 | 投资活动现金流出小计 |
| INVEST_NETCASH_OTHER | Nullable(Float64) | 262,973 | 零值 20,637；负值 1 | min=-5,039,200,000, max=5,121,120,000, distinct 4 | 投资活动净现金流量其他 |
| INVEST_NETCASH_BALANCE | Nullable(Float64) | 43,689 | 零值 239,738；负值 91 | min=-9,351,000,000, max=29,499,600,000, distinct 70 | 投资活动净现金流量平衡项 |
| NETCASH_INVEST | Nullable(Float64) | 1,095 | 零值 159；负值 231,278 | min=-332,948,000,000, max=52,943,000,000, distinct 281,446 | 投资活动产生的现金流量净额 |
| ACCEPT_INVEST_CASH | Nullable(Float64) | 176,207 | 零值 7,089；负值 402 | min=-2,864,720,000, max=105,733,000,000, distinct 50,148 | 吸收投资收到的现金 |
| SUBSIDIARY_ACCEPT_INVEST | Nullable(Float64) | 225,405 | 零值 6,038；负值 58 | min=-31,323,400,000, max=105,733,000,000, distinct 21,105 | 子公司吸收少数股东投资收到的现金 |
| RECEIVE_LOAN_CASH | Nullable(Float64) | 58,560 | 零值 2,431；负值 30 | min=-219,589,000, max=1,182,910,000,000, distinct 136,329 | 取得借款收到的现金 |
| ISSUE_BOND | Nullable(Float64) | 272,541 | 零值 2,759；负值 0 | min=0, max=150,984,000,000, distinct 3,434 | 发行债券收到的现金 |
| RECEIVE_OTHER_FINANCE | Nullable(Float64) | 166,162 | 零值 7,451；负值 154 | min=-373,251,000, max=206,377,000,000, distinct 83,611 | 收到其他与筹资活动有关的现金 |
| FINANCE_INFLOW_OTHER | Nullable(Float64) | 262,445 | 零值 20,611；负值 2 | min=-110,158,000, max=69,851,300,000, distinct 473 | 筹资活动现金流入其他 |
| FINANCE_INFLOW_BALANCE | Nullable(Float64) | 70,046 | 零值 211,667；负值 39 | min=-5,987,860,000, max=25,417,700,000, distinct 1,527 | 筹资活动现金流入平衡项 |
| TOTAL_FINANCE_INFLOW | Nullable(Float64) | 31,632 | 零值 5,764；负值 93 | min=-518,946,000, max=1,228,900,000,000, distinct 188,209 | 筹资活动现金流入小计 |
| PAY_DEBT_CASH | Nullable(Float64) | 51,260 | 零值 2,309；负值 18 | min=-6,816,740,000, max=1,159,320,000,000, distinct 156,020 | 偿还债务支付的现金 |
| ASSIGN_DIVIDEND_PORFIT | Nullable(Float64) | 18,462 | 零值 798；负值 156 | min=-747,035,000, max=111,199,000,000, distinct 251,756 | 分配股利、利润或偿付利息支付的现金 |
| SUBSIDIARY_PAY_DIVIDEND | Nullable(Float64) | 227,745 | 零值 6,806；负值 53 | min=-4,119,000,000, max=21,945,600,000, distinct 32,199 | 子公司向少数股东支付的现金股利 |
| BUY_SUBSIDIARY_EQUITY | Nullable(Float64) | 283,080 | 零值 408；负值 0 | min=0, max=10,004,200,000, distinct 99 | 子公司减少现金 |
| PAY_OTHER_FINANCE | Nullable(Float64) | 95,373 | 零值 4,377；负值 319 | min=-766,800,000, max=123,648,000,000, distinct 161,459 | 支付其他与筹资活动有关的现金 |
| SUBSIDIARY_REDUCE_CASH | Nullable(Float64) | 262,557 | 零值 21,032；负值 0 | min=0, max=3,800,000,000, distinct 23 | 子公司减少现金 |
| FINANCE_OUTFLOW_OTHER | Nullable(Float64) | 232,989 | 零值 49,644；负值 4 | min=-1,082,180,000, max=783,111,000,000, distinct 777 | 筹资活动现金流出其他 |
| FINANCE_OUTFLOW_BALANCE | Nullable(Float64) | 49,567 | 零值 232,416；负值 105 | min=-2,133,920,000, max=3,936,610,000, distinct 1,446 | 筹资活动现金流出平衡项 |
| TOTAL_FINANCE_OUTFLOW | Nullable(Float64) | 8,381 | 零值 1,692；负值 108 | min=-7,406,990,000, max=1,197,740,000,000, distinct 266,280 | 筹资活动现金流出小计 |
| FINANCE_NETCASH_OTHER | Nullable(Float64) | 262,974 | 零值 20,639；负值 0 | min=0, max=0, distinct 1 | 筹资活动净现金流量其他 |
| FINANCE_NETCASH_BALANCE | Nullable(Float64) | 48,128 | 零值 235,351；负值 52 | min=-14,814,000,000, max=2,480,000,000, distinct 64 | 筹资活动净现金流量平衡项 |
| NETCASH_FINANCE | Nullable(Float64) | 6,740 | 零值 1,421；负值 143,270 | min=-178,876,000,000, max=146,629,000,000, distinct 269,172 | 筹资活动产生的现金流量净额 |
| RATE_CHANGE_EFFECT | Nullable(Float64) | 95,382 | 零值 2,930；负值 97,609 | min=-4,967,000,000, max=12,992,200,000, distinct 181,407 | 汇率变动对现金及现金等价物的影响 |
| CCE_ADD_OTHER | Nullable(Float64) | 262,916 | 零值 20,639；负值 28 | min=-65,248,000, max=143,992,000, distinct 48 | 现金及现金等价物净增加额其他 |
| CCE_ADD_BALANCE | Nullable(Float64) | 43,063 | 零值 239,521；负值 528 | min=-2,480,000,000, max=1,004,620,000, distinct 459 | 现金及现金等价物净增加额平衡项 |
| CCE_ADD | Nullable(Float64) | 191 | 零值 7；负值 156,640 | min=-91,197,500,000, max=125,763,000,000, distinct 283,091 | 现金及现金等价物净增加额 |
| BEGIN_CCE | Nullable(Float64) | 25,466 | 零值 16；负值 6 | min=-145,281,000, max=359,373,000,000, distinct 90,203 | 期初现金及现金等价物余额 |
| END_CCE_OTHER | Nullable(Float64) | 283,576 | 零值 10；负值 18 | min=-535,000,000, max=395,206,000, distinct 25 | 期末现金及现金等价物余额其他 |
| END_CCE_BALANCE | Nullable(Float64) | 67,045 | 零值 216,407；负值 80 | min=-41,536,000,000, max=51,489,000,000, distinct 80 | 期末现金及现金等价物余额平衡项 |
| END_CCE | Nullable(Float64) | 25,146 | 零值 10；负值 233 | min=-3,616,490,000, max=359,373,000,000, distinct 258,189 | 期末现金及现金等价物余额 |
| NETPROFIT | Nullable(Float64) | 132,479 | 零值 14；负值 23,429 | min=-91,810,100,000, max=183,747,000,000, distinct 150,955 | 净利润（间接法起点） |
| ASSET_IMPAIRMENT | Nullable(Float64) | 147,704 | 零值 356；负值 22,932 | min=-5,030,050,000, max=37,233,000,000, distinct 134,370 | 资产减值准备 |
| FA_IR_DEPR | Nullable(Float64) | 132,386 | 零值 229；负值 83 | min=-177,723,000, max=215,818,000,000, distinct 150,793 | 固定资产折旧、油气资产折耗、生产性生物资产折旧 |
| OILGAS_BIOLOGY_DEPR | Nullable(Float64) | 133,428 | 零值 17；负值 83 | min=-177,723,000, max=215,818,000,000, distinct 149,964 | 油气资产折耗、生产性生物资产折旧 |
| IR_DEPR | Nullable(Float64) | 281,291 | 零值 21；负值 2 | min=-893,131, max=4,507,300,000, distinct 2,021 | 折旧与摊销 |
| IA_AMORTIZE | Nullable(Float64) | 137,607 | 零值 119；负值 189 | min=-232,920,000, max=9,689,220,000, distinct 140,932 | 无形资产摊销 |
| LPE_AMORTIZE | Nullable(Float64) | 164,348 | 零值 996；负值 994 | min=-1,603,240,000, max=8,948,160,000, distinct 113,240 | 长期待摊费用摊销 |
| DEFER_INCOME_AMORTIZE | Nullable(Float64) | 282,304 | 零值 82；负值 1,027 | min=-1,973,350,000, max=2,253,420,000, distinct 1,180 | 待摊费用减少（减：增加） |
| PREPAID_EXPENSE_REDUCE | Nullable(Float64) | 263,691 | 零值 23；负值 11,475 | min=-3,109,570,000, max=1,006,140,000, distinct 19,577 | 预提费用增加（减：减少） |
| ACCRUED_EXPENSE_ADD | Nullable(Float64) | 264,085 | 零值 23；负值 5,902 | min=-519,328,000, max=1,301,780,000, distinct 19,028 | 预提费用变动 |
| DISPOSAL_LONGASSET_LOSS | Nullable(Float64) | 157,851 | 零值 1,284；负值 63,703 | min=-9,046,460,000, max=13,273,000,000, distinct 120,980 | 处置固定资产等的损失 |
| FA_SCRAP_LOSS | Nullable(Float64) | 208,547 | 零值 3,186；负值 6,216 | min=-766,000,000, max=21,152,000,000, distinct 70,059 | 固定资产报废损失 |
| FAIRVALUE_CHANGE_LOSS | Nullable(Float64) | 228,351 | 零值 3,826；负值 31,156 | min=-13,170,000,000, max=25,594,000,000, distinct 50,454 | 公允价值变动损失 |
| FINANCE_EXPENSE | Nullable(Float64) | 137,789 | 零值 320；负值 16,515 | min=-8,731,000,000, max=26,778,000,000, distinct 145,014 | 财务费用 |
| INVEST_LOSS | Nullable(Float64) | 153,056 | 零值 1,110；负值 100,482 | min=-65,896,200,000, max=11,686,600,000, distinct 126,675 | 投资损失 |
| DEFER_TAX | Nullable(Float64) | 116,273 | 零值 49,328；负值 77,522 | min=-16,325,000,000, max=15,758,800,000, distinct 117,767 | 递延所得税资产减少（增加以"-"号填列） |
| DT_ASSET_REDUCE | Nullable(Float64) | 166,545 | 零值 533；负值 78,222 | min=-16,325,000,000, max=16,409,700,000, distinct 116,318 | 递延所得税资产减少 |
| DT_LIAB_ADD | Nullable(Float64) | 207,560 | 零值 2,969；负值 42,504 | min=-6,490,960,000, max=13,274,600,000, distinct 70,696 | 递延所得税负债增加 |
| PREDICT_LIAB_ADD | Nullable(Float64) | 283,114 | 零值 341；负值 52 | min=-333,612,000, max=409,180,000, distinct 157 | 预计负债增加 |
| INVENTORY_REDUCE | Nullable(Float64) | 134,369 | 零值 97；负值 98,401 | min=-122,709,000,000, max=234,158,000,000, distinct 148,890 | 存货的减少（增加以"-"号填列） |
| OPERATE_RECE_REDUCE | Nullable(Float64) | 132,519 | 零值 16；负值 104,593 | min=-384,620,000,000, max=74,768,900,000, distinct 150,885 | 经营性应收项目的减少 |
| OPERATE_PAYABLE_ADD | Nullable(Float64) | 132,529 | 零值 17；负值 66,986 | min=-284,959,000,000, max=458,079,000,000, distinct 150,896 | 经营性应付项目的增加 |
| OTHER | Nullable(Float64) | 225,665 | 零值 3,865；负值 19,594 | min=-12,174,100,000, max=17,091,700,000, distinct 52,728 | 现金流量表年初至报告期末补充资料中的其他项目。 |
| OPERATE_NETCASH_OTHERNOTE | Nullable(Float64) | 253,035 | 零值 26,109；负值 1,416 | min=-71,283,700,000, max=139,917,000,000, distinct 4,415 | 经营活动产生的现金流量净额（附注） |
| OPERATE_NETCASH_BALANCENOTE | Nullable(Float64) | 153,931 | 零值 121,414；负值 3,247 | min=-6,235,610,000, max=4,280,000,000, distinct 6,811 | 经营活动净现金流量（附注）平衡项 |
| NETCASH_OPERATENOTE | Nullable(Float64) | 111,437 | 零值 4；负值 56,237 | min=-108,769,000,000, max=456,847,000,000, distinct 171,956 | 经营活动产生的现金流量净额（附注） |
| DEBT_TRANSFER_CAPITAL | Nullable(Float64) | 280,799 | 零值 1,891；负值 17 | min=-298,576,000, max=31,751,800,000, distinct 853 | 债务转为资本 |
| CONVERT_BOND_1YEAR | Nullable(Float64) | 281,569 | 零值 1,826；负值 6 | min=-1,077,080,000, max=3,950,000,000, distinct 215 | 一年内到期的可转换公司债券 |
| FINLEASE_OBTAIN_FA | Nullable(Float64) | 279,811 | 零值 1,846；负值 16 | min=-327,504,000, max=26,120,400,000, distinct 1,871 | 融资租入固定资产 |
| UNINVOLVE_INVESTFIN_OTHER | Nullable(Float64) | 268,662 | 零值 11,479；负值 33 | min=-1,536,530,000, max=85,987,900,000, distinct 3,425 | 不涉及现金收支的投资和筹资活动其他 |
| END_CASH | Nullable(Float64) | 133,223 | 零值 2；负值 5 | min=-161,083,000, max=359,373,000,000, distinct 150,165 | 现金期末余额 |
| BEGIN_CASH | Nullable(Float64) | 133,265 | 零值 5；负值 5 | min=-29,953,700,000, max=359,373,000,000, distinct 82,770 | 现金期初余额 |
| END_CASH_EQUIVALENTS | Nullable(Float64) | 275,888 | 零值 3,560；负值 59 | min=-2,048,780,000, max=94,809,800,000, distinct 3,579 | 现金等价物期末余额 |
| BEGIN_CASH_EQUIVALENTS | Nullable(Float64) | 275,798 | 零值 3,560；负值 58 | min=-1,069,710,000, max=94,809,800,000, distinct 2,782 | 现金等价物期初余额 |
| CCE_ADD_OTHERNOTE | Nullable(Float64) | 283,492 | 零值 6；负值 54 | min=-271,678,000, max=14,864,300,000, distinct 116 | 现金及现金等价物净增加额（附注） |
| CCE_ADD_BALANCENOTE | Nullable(Float64) | 259,642 | 零值 23,436；负值 270 | min=-3,000,000,000, max=1,879,760,000, distinct 457 | 现金及现金等价物净增加额（附注）平衡项 |
| CCE_ADDNOTE | Nullable(Float64) | 111,423 | 零值 1；负值 90,191 | min=-86,403,600,000, max=125,763,000,000, distinct 171,934 | 现金及现金等价物净增加额（附注） |
| SALES_SERVICES_YOY | Nullable(Float64) | 12,413 | 零值 27；负值 97,237 | min=-229.85, max=11,814,500,000, distinct 270,934 | 销售商品、提供劳务收到的现金同比增长率（%） |
| DEPOSIT_INTERBANK_ADD_YOY | Nullable(Float64) | 282,293 | 零值 0；负值 631 | min=-55,417.2, max=347,779, distinct 1,302 | 同业存放净增加额同比增长率（%） |
| LOAN_PBC_ADD_YOY | Nullable(Float64) | 283,401 | 零值 6；负值 113 | min=-7,593.75, max=5,376.78, distinct 190 | 向央行借款净增加额同比增长率（%） |
| OFI_BF_ADD_YOY | Nullable(Float64) | 283,286 | 零值 2；负值 176 | min=-297,879, max=188,459, distinct 289 | 向其他金融机构拆入资金净增加额同比增长率（%） |
| RECEIVE_ORIGIC_PREMIUM_YOY | Nullable(Float64) | 283,328 | 零值 0；负值 122 | min=-100, max=6,763.24, distinct 281 | 收到原保险合同保费现金同比增长率（%） |
| RECEIVE_REINSURE_NET_YOY | Nullable(Float64) | 283,568 | 零值 0；负值 22 | min=-2,353.39, max=5,974.04, distinct 45 | 收到再保险业务现金净额同比增长率（%） |
| INSURED_INVEST_ADD_YOY | Nullable(Float64) | 283,583 | 零值 0；负值 18 | min=-94.6432, max=43,694.8, distinct 30 | 保户储金及投资款净增加额同比增长率（%） |
| DISPOSAL_TFA_ADD_YOY | Nullable(Float64) | 283,456 | 零值 0；负值 88 | min=-187,666, max=605,782, distinct 151 | 处置交易性金融资产净增加额同比增长率（%） |
| RECEIVE_INTEREST_COMMISSION_YOY | Nullable(Float64) | 278,488 | 零值 2；负值 2,516 | min=-127.62, max=3,708,020, distinct 5,101 | 收取利息和手续费现金同比增长率（%） |
| BORROW_FUND_ADD_YOY | Nullable(Float64) | 283,251 | 零值 4；负值 188 | min=-43,456.7, max=24,222,400, distinct 323 | 拆入资金净增加额同比增长率（%） |
| LOAN_ADVANCE_REDUCE_YOY | Nullable(Float64) | 283,569 | 零值 0；负值 24 | min=-100, max=2,072.38, distinct 43 | 发放贷款及垫款净减少额同比增长率（%） |
| REPO_BUSINESS_ADD_YOY | Nullable(Float64) | 282,953 | 零值 0；负值 340 | min=-4,617.9, max=5,385,380, distinct 637 | 回购业务资金净增加额同比增长率（%） |
| RECEIVE_TAX_REFUND_YOY | Nullable(Float64) | 111,053 | 零值 45；负值 81,073 | min=-5,725.61, max=5,251,040,000, distinct 169,260 | 收到的税费返还同比增长率（%） |
| RECEIVE_OTHER_OPERATE_YOY | Nullable(Float64) | 13,863 | 零值 30；负值 120,988 | min=-11,368, max=147,739,000, distinct 269,440 | 收到其他与经营活动有关的现金同比增长率（%） |
| OPERATE_INFLOW_OTHER_YOY | Nullable(Float64) | 282,493 | 零值 0；负值 617 | min=-6,479.19, max=108,070, distinct 1,048 | 经营活动现金流入其他同比增长率（%） |
| OPERATE_INFLOW_BALANCE_YOY | Nullable(Float64) | 280,877 | 零值 19；负值 1,970 | min=-200, max=115,398,000, distinct 1,077 | 经营活动现金流入平衡项同比增长率（%） |
| TOTAL_OPERATE_INFLOW_YOY | Nullable(Float64) | 11,864 | 零值 23；负值 97,625 | min=-75,145,500, max=638,650,000, distinct 271,496 | 经营活动现金流入小计同比增长率（%） |
| BUY_SERVICES_YOY | Nullable(Float64) | 12,831 | 零值 24；负值 103,525 | min=-15,176.1, max=34,370,400, distinct 270,510 | 购买商品、接受劳务支付的现金同比增长率（%） |
| LOAN_ADVANCE_ADD_YOY | Nullable(Float64) | 279,805 | 零值 3；负值 1,969 | min=-809,489, max=6,839,440, distinct 3,765 | 发放贷款及垫款净增加额同比增长率（%） |
| PBC_INTERBANK_ADD_YOY | Nullable(Float64) | 282,511 | 零值 0；负值 527 | min=-116,382, max=46,998.1, distinct 1,086 | 向央行借款净增加额同比增长率（%） |
| PAY_ORIGIC_COMPENSATE_YOY | Nullable(Float64) | 283,468 | 零值 0；负值 43 | min=-92.6515, max=2,268.86, distinct 145 | 支付原保险合同赔付款项现金同比增长率（%） |
| PAY_INTEREST_COMMISSION_YOY | Nullable(Float64) | 280,623 | 零值 0；负值 1,387 | min=-4,267.55, max=2,892,360, distinct 2,969 | 支付利息和手续费现金同比增长率（%） |
| PAY_POLICY_BONUS_YOY | Nullable(Float64) | 283,546 | 零值 0；负值 33 | min=-174.833, max=6,355.7, distinct 66 | 保单红利支出同比增长率（%） |
| PAY_STAFF_CASH_YOY | Nullable(Float64) | 11,875 | 零值 26；负值 69,802 | min=-118.455, max=807,263, distinct 271,477 | 支付给职工以及为职工支付的现金同比增长率（%） |
| PAY_ALL_TAX_YOY | Nullable(Float64) | 12,265 | 零值 26；负值 118,251 | min=-8,287.47, max=37,786,100, distinct 271,073 | 支付的各项税费同比增长率（%） |
| PAY_OTHER_OPERATE_YOY | Nullable(Float64) | 12,068 | 零值 25；负值 113,534 | min=-1,213.01, max=10,272,700, distinct 271,282 | 支付其他与经营活动有关的现金同比增长率（%） |
| OPERATE_OUTFLOW_OTHER_YOY | Nullable(Float64) | 282,568 | 零值 2；负值 562 | min=-459,041, max=2,442,620, distinct 980 | 经营活动现金流出其他同比增长率（%） |
| OPERATE_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 280,670 | 零值 32；负值 2,115 | min=-1,598.5, max=550,095,000, distinct 986 | 经营活动现金流出平衡项同比增长率（%） |
| TOTAL_OPERATE_OUTFLOW_YOY | Nullable(Float64) | 11,816 | 零值 23；负值 97,010 | min=-12,518.3, max=182,975,000, distinct 271,545 | 经营活动现金流出小计同比增长率（%） |
| OPERATE_NETCASH_OTHER_YOY | Nullable(Float64) | 283,613 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 经营活动净现金流量其他同比增长率（%） |
| OPERATE_NETCASH_BALANCE_YOY | Nullable(Float64) | 283,010 | 零值 9；负值 317 | min=-3,598.78, max=8,316.9, distinct 31 | 经营活动净现金流量平衡项同比增长率（%） |
| NETCASH_OPERATE_YOY | Nullable(Float64) | 10,796 | 零值 23；负值 132,660 | min=-2,362,750,000, max=4,775,340, distinct 272,564 | 经营活动产生的现金流量净额同比增长率（%） |
| WITHDRAW_INVEST_YOY | Nullable(Float64) | 173,746 | 零值 459；负值 53,423 | min=-74,243, max=471,313,000,000, distinct 98,818 | 收回投资收到的现金同比增长率（%） |
| RECEIVE_INVEST_INCOME_YOY | Nullable(Float64) | 143,571 | 零值 1,222；负值 66,637 | min=-1,638,490, max=24,676,500,000, distinct 131,366 | 取得投资收益收到的现金同比增长率（%） |
| DISPOSAL_LONG_ASSET_YOY | Nullable(Float64) | 103,868 | 零值 82；负值 87,612 | min=-999,805, max=73,727,500,000, distinct 171,185 | 处置固定资产等收回的现金净额同比增长率（%） |
| DISPOSAL_SUBSIDIARY_OTHER_YOY | Nullable(Float64) | 271,798 | 零值 30；负值 6,608 | min=-836,727,000, max=122,875,000,000, distinct 9,717 | 处置子公司及其他营业单位收到的现金净额同比增长率（%） |
| REDUCE_PLEDGE_TIMEDEPOSITS_YOY | Nullable(Float64) | 283,539 | 零值 0；负值 43 | min=-99.6308, max=71,908.4, distinct 74 | 减少质押定期存款同比增长率（%） |
| RECEIVE_OTHER_INVEST_YOY | Nullable(Float64) | 208,212 | 零值 257；负值 37,724 | min=-17,666.1, max=100,019,000,000, distinct 70,264 | 收到其他与投资活动有关的现金同比增长率（%） |
| INVEST_INFLOW_OTHER_YOY | Nullable(Float64) | 283,072 | 零值 0；负值 295 | min=-131.012, max=7,259,940, distinct 464 | 投资活动现金流入其他同比增长率（%） |
| INVEST_INFLOW_BALANCE_YOY | Nullable(Float64) | 280,734 | 零值 7；负值 2,194 | min=-400, max=24,257,100, distinct 1,223 | 投资活动现金流入平衡项同比增长率（%） |
| TOTAL_INVEST_INFLOW_YOY | Nullable(Float64) | 47,137 | 零值 108；负值 110,963 | min=-1,094,730, max=25,824,100,000, distinct 231,560 | 投资活动现金流入小计同比增长率（%） |
| CONSTRUCT_LONG_ASSET_YOY | Nullable(Float64) | 15,120 | 零值 25；负值 129,487 | min=-1,022,600, max=34,598,300, distinct 268,049 | 购建固定资产等支付的现金同比增长率（%） |
| INVEST_PAY_CASH_YOY | Nullable(Float64) | 152,177 | 零值 631；负值 64,260 | min=-2,701,310, max=324,660,000,000, distinct 116,303 | 投资支付的现金同比增长率（%） |
| PLEDGE_LOAN_ADD_YOY | Nullable(Float64) | 283,551 | 零值 0；负值 30 | min=-100, max=3,862.23, distinct 59 | 质押贷款净增加额同比增长率（%） |
| OBTAIN_SUBSIDIARY_OTHER_YOY | Nullable(Float64) | 268,013 | 零值 125；负值 8,904 | min=-467,251, max=207,319,000,000, distinct 13,174 | 取得子公司及其他营业单位支付的现金净额同比增长率（%） |
| ADD_PLEDGE_TIMEDEPOSITS_YOY | Nullable(Float64) | 283,550 | 零值 0；负值 32 | min=-99.8789, max=79,019.7, distinct 63 | 增加质押定期存款同比增长率（%） |
| PAY_OTHER_INVEST_YOY | Nullable(Float64) | 223,217 | 零值 277；负值 31,085 | min=-1,035,770, max=5,094,000,000,000, distinct 54,802 | 支付其他与投资活动有关的现金同比增长率（%） |
| INVEST_OUTFLOW_OTHER_YOY | Nullable(Float64) | 283,135 | 零值 2；负值 274 | min=-100, max=32,576,300, distinct 381 | 投资活动现金流出其他同比增长率（%） |
| INVEST_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 283,234 | 零值 1；负值 286 | min=-632.111, max=36,799,900, distinct 83 | 投资活动现金流出平衡项同比增长率（%） |
| TOTAL_INVEST_OUTFLOW_YOY | Nullable(Float64) | 14,176 | 零值 23；负值 125,686 | min=-247,613, max=1,270,700,000, distinct 268,969 | 投资活动现金流出小计同比增长率（%） |
| INVEST_NETCASH_OTHER_YOY | Nullable(Float64) | 283,612 | 零值 0；负值 0 | min=201.626, max=201.626, distinct 1 | 投资活动净现金流量其他同比增长率（%） |
| INVEST_NETCASH_BALANCE_YOY | Nullable(Float64) | 283,430 | 零值 3；负值 65 | min=-41,161.6, max=18,902.1, distinct 13 | 投资活动净现金流量平衡项同比增长率（%） |
| NETCASH_INVEST_YOY | Nullable(Float64) | 13,324 | 零值 23；负值 140,760 | min=-288,839,000, max=24,002,200, distinct 269,904 | 投资活动产生的现金流量净额同比增长率（%） |
| ACCEPT_INVEST_CASH_YOY | Nullable(Float64) | 226,513 | 零值 406；负值 31,195 | min=-54,795, max=517,000,000,000, distinct 45,760 | 吸收投资收到的现金同比增长率（%） |
| SUBSIDIARY_ACCEPT_INVEST_YOY | Nullable(Float64) | 253,739 | 零值 417；负值 15,446 | min=-118,302, max=429,515,000, distinct 22,873 | 子公司吸收少数股东投资收到的现金同比增长率（%） |
| RECEIVE_LOAN_CASH_YOY | Nullable(Float64) | 85,946 | 零值 2,158；负值 87,104 | min=-946.48, max=183,322,000, distinct 170,934 | 取得借款收到的现金同比增长率（%） |
| ISSUE_BOND_YOY | Nullable(Float64) | 279,835 | 零值 265；负值 1,677 | min=-100, max=4,421,380, distinct 2,519 | 发行债券收到的现金同比增长率（%） |
| RECEIVE_OTHER_FINANCE_YOY | Nullable(Float64) | 207,438 | 零值 353；负值 38,135 | min=-202,228, max=595,000,000,000, distinct 70,233 | 收到其他与筹资活动有关的现金同比增长率（%） |
| FINANCE_INFLOW_OTHER_YOY | Nullable(Float64) | 283,283 | 零值 3；负值 163 | min=-102.313, max=1,146,750, distinct 303 | 筹资活动现金流入其他同比增长率（%） |
| FINANCE_INFLOW_BALANCE_YOY | Nullable(Float64) | 281,818 | 零值 3；负值 1,546 | min=-647.407, max=34,513,200, distinct 436 | 筹资活动现金流入平衡项同比增长率（%） |
| TOTAL_FINANCE_INFLOW_YOY | Nullable(Float64) | 60,529 | 零值 1,088；负值 100,833 | min=-45,165.5, max=9,030,920,000, distinct 208,231 | 筹资活动现金流入小计同比增长率（%） |
| PAY_DEBT_CASH_YOY | Nullable(Float64) | 77,303 | 零值 2,163；负值 89,048 | min=-1,135.03, max=1,624,680,000, distinct 184,481 | 偿还债务支付的现金同比增长率（%） |
| ASSIGN_DIVIDEND_PORFIT_YOY | Nullable(Float64) | 36,959 | 零值 846；负值 108,141 | min=-167,450, max=2,035,270,000, distinct 240,123 | 分配股利、利润或偿付利息支付的现金同比增长率（%） |
| SUBSIDIARY_PAY_DIVIDEND_YOY | Nullable(Float64) | 247,977 | 零值 1,024；负值 15,993 | min=-2,032.66, max=7,111,210,000, distinct 28,803 | 子公司向少数股东支付的现金股利同比增长率（%） |
| BUY_SUBSIDIARY_EQUITY_YOY | Nullable(Float64) | 283,547 | 零值 1；负值 39 | min=-100, max=67,729.3, distinct 57 | 子公司减少现金同比增长率（%） |
| PAY_OTHER_FINANCE_YOY | Nullable(Float64) | 136,494 | 零值 709；负值 68,990 | min=-395,308, max=51,816,800,000, distinct 141,921 | 支付其他与筹资活动有关的现金同比增长率（%） |
| SUBSIDIARY_REDUCE_CASH_YOY | Nullable(Float64) | 283,606 | 零值 0；负值 4 | min=-100, max=3,700, distinct 6 | 子公司减少现金同比增长率（%） |
| FINANCE_OUTFLOW_OTHER_YOY | Nullable(Float64) | 282,954 | 零值 10；负值 335 | min=-270.174, max=6,357,070,000, distinct 551 | 筹资活动现金流出其他同比增长率（%） |
| FINANCE_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 282,028 | 零值 4；负值 1,334 | min=-101,680,000, max=13,507,100, distinct 333 | 筹资活动现金流出平衡项同比增长率（%） |
| TOTAL_FINANCE_OUTFLOW_YOY | Nullable(Float64) | 25,304 | 零值 389；负值 109,912 | min=-21,953.4, max=2,000,560,000, distinct 254,842 | 筹资活动现金流出小计同比增长率（%） |
| FINANCE_NETCASH_OTHER_YOY | Nullable(Float64) | 283,613 | 零值 0；负值 0 | min=NULL, max=NULL, distinct 0 | 筹资活动净现金流量其他同比增长率（%） |
| FINANCE_NETCASH_BALANCE_YOY | Nullable(Float64) | 283,485 | 零值 1；负值 71 | min=-200, max=275, distinct 8 | 筹资活动净现金流量平衡项同比增长率（%） |
| NETCASH_FINANCE_YOY | Nullable(Float64) | 22,787 | 零值 274；负值 127,894 | min=-1,195,570,000, max=9,023,010,000, distinct 258,116 | 筹资活动产生的现金流量净额同比增长率（%） |
| RATE_CHANGE_EFFECT_YOY | Nullable(Float64) | 114,717 | 零值 49；负值 88,730 | min=-9,550,040,000, max=14,237,500,000, distinct 167,849 | 汇率变动对现金及现金等价物的影响同比增长率（%） |
| CCE_ADD_OTHER_YOY | Nullable(Float64) | 283,593 | 零值 0；负值 4 | min=-652.424, max=27,512.5, distinct 18 | 现金及现金等价物净增加额其他同比增长率（%） |
| CCE_ADD_BALANCE_YOY | Nullable(Float64) | 282,821 | 零值 22；负值 387 | min=-18,257.5, max=1,843,690,000, distinct 67 | 现金及现金等价物净增加额平衡项同比增长率（%） |
| CCE_ADD_YOY | Nullable(Float64) | 11,783 | 零值 24；负值 130,449 | min=-2,362,750,000, max=314,054,000,000, distinct 271,579 | 现金及现金等价物净增加额同比增长率（%） |
| BEGIN_CCE_YOY | Nullable(Float64) | 37,573 | 零值 154；负值 109,476 | min=-497.341, max=1,279,600,000,000, distinct 99,628 | 期初现金及现金等价物余额同比增长率（%） |
| END_CCE_OTHER_YOY | Nullable(Float64) | 283,605 | 零值 0；负值 6 | min=-1,621.61, max=131.081, distinct 8 | 期末现金及现金等价物余额其他同比增长率（%） |
| END_CCE_BALANCE_YOY | Nullable(Float64) | 283,481 | 零值 0；负值 62 | min=-246.744, max=3,872.92, distinct 18 | 期末现金及现金等价物余额平衡项同比增长率（%） |
| END_CCE_YOY | Nullable(Float64) | 37,220 | 零值 22；负值 112,657 | min=-6,947.67, max=5,209,810, distinct 246,200 | 期末现金及现金等价物余额同比增长率（%） |
| NETPROFIT_YOY | Nullable(Float64) | 139,377 | 零值 7；负值 61,752 | min=-11,355,100, max=707,233, distinct 144,096 | 净利润同比增长率（%） |
| ASSET_IMPAIRMENT_YOY | Nullable(Float64) | 157,045 | 零值 12；负值 55,417 | min=-1,897,270,000, max=88,558,800, distinct 126,090 | 资产减值准备同比增长率（%） |
| FA_IR_DEPR_YOY | Nullable(Float64) | 139,488 | 零值 25；负值 40,387 | min=-17,972.9, max=101,750,000, distinct 143,911 | 固定资产折旧、油气资产折耗、生产性生物资产折旧同比增长率（%） |
| OILGAS_BIOLOGY_DEPR_YOY | Nullable(Float64) | 140,557 | 零值 25；负值 40,138 | min=-17,972.9, max=101,750,000, distinct 142,901 | 油气资产折耗、生产性生物资产折旧同比增长率（%） |
| IR_DEPR_YOY | Nullable(Float64) | 281,758 | 零值 248；负值 661 | min=-350, max=21,366.7, distinct 1,575 | 折旧与摊销同比增长率（%） |
| IA_AMORTIZE_YOY | Nullable(Float64) | 145,302 | 零值 4,445；负值 41,737 | min=-14,161.8, max=2,712,690, distinct 133,199 | 无形资产摊销同比增长率（%） |
| LPE_AMORTIZE_YOY | Nullable(Float64) | 176,067 | 零值 3,495；负值 41,122 | min=-182,774, max=2,154,980, distinct 103,048 | 长期待摊费用摊销同比增长率（%） |
| DEFER_INCOME_AMORTIZE_YOY | Nullable(Float64) | 282,583 | 零值 46；负值 535 | min=-12,029.9, max=50,451.3, distinct 979 | 待摊费用减少（减：增加）同比增长率（%） |
| PREPAID_EXPENSE_REDUCE_YOY | Nullable(Float64) | 269,237 | 零值 17；负值 7,637 | min=-419,438,000, max=2,680,440,000, distinct 14,240 | 预提费用增加（减：减少）同比增长率（%） |
| ACCRUED_EXPENSE_ADD_YOY | Nullable(Float64) | 269,735 | 零值 25；负值 6,525 | min=-1,366,100, max=3,450,400, distinct 13,718 | 预提费用变动同比增长率（%） |
| DISPOSAL_LONGASSET_LOSS_YOY | Nullable(Float64) | 176,031 | 零值 40；负值 54,204 | min=-32,185,400,000, max=638,214,000, distinct 106,437 | 处置固定资产等的损失同比增长率（%） |
| FA_SCRAP_LOSS_YOY | Nullable(Float64) | 229,778 | 零值 34；负值 26,602 | min=-43,552,400, max=1,124,550,000, distinct 52,895 | 固定资产报废损失同比增长率（%） |
| FAIRVALUE_CHANGE_LOSS_YOY | Nullable(Float64) | 242,607 | 零值 24；负值 20,988 | min=-26,953,900, max=71,267,900, distinct 39,659 | 公允价值变动损失同比增长率（%） |
| FINANCE_EXPENSE_YOY | Nullable(Float64) | 146,975 | 零值 56；负值 61,466 | min=-21,983,500, max=9,764,140,000, distinct 136,295 | 财务费用同比增长率（%） |
| INVEST_LOSS_YOY | Nullable(Float64) | 167,149 | 零值 434；负值 60,526 | min=-10,250,200,000, max=133,194,000, distinct 115,078 | 投资损失同比增长率（%） |
| DEFER_TAX_YOY | Nullable(Float64) | 170,502 | 零值 69；负值 60,335 | min=-396,525,000, max=2,230,870,000, distinct 111,949 | 递延所得税资产减少（增加以"-"号填列）同比增长率（%） |
| DT_ASSET_REDUCE_YOY | Nullable(Float64) | 172,997 | 零值 37；负值 59,779 | min=-396,525,000, max=145,434,000,000, distinct 110,336 | 递延所得税资产减少同比增长率（%） |
| DT_LIAB_ADD_YOY | Nullable(Float64) | 221,319 | 零值 1,534；负值 30,405 | min=-5,850,400,000, max=3,023,930,000, distinct 59,432 | 递延所得税负债增加同比增长率（%） |
| PREDICT_LIAB_ADD_YOY | Nullable(Float64) | 283,488 | 零值 0；负值 53 | min=-26,689, max=9,675.98, distinct 123 | 预计负债增加同比增长率（%） |
| INVENTORY_REDUCE_YOY | Nullable(Float64) | 141,785 | 零值 15；负值 75,529 | min=-25,687,800, max=5,451,900, distinct 141,619 | 存货的减少（增加以"-"号填列）同比增长率（%） |
| OPERATE_RECE_REDUCE_YOY | Nullable(Float64) | 139,436 | 零值 14；负值 76,491 | min=-12,934,100, max=1,230,780, distinct 144,028 | 经营性应收项目的减少同比增长率（%） |
| OPERATE_PAYABLE_ADD_YOY | Nullable(Float64) | 139,456 | 零值 4；负值 69,047 | min=-4,103,880, max=5,985,490, distinct 144,018 | 经营性应付项目的增加同比增长率（%） |
| OTHER_YOY | Nullable(Float64) | 243,600 | 零值 561；负值 20,448 | min=-3,386,410,000, max=46,631,800, distinct 38,645 | 其他同比增长率（%） |
| OPERATE_NETCASH_OTHERNOTE_YOY | Nullable(Float64) | 280,117 | 零值 19；负值 1,783 | min=-1,642,340, max=1,415,960, distinct 3,296 | 经营活动产生的现金流量净额（附注）同比增长率（%） |
| OPERATE_NETCASH_BALANCENOTE_YOY | Nullable(Float64) | 276,474 | 零值 135；负值 3,795 | min=-3,244,480,000,000, max=580,011,000,000, distinct 3,134 | 经营活动净现金流量（附注）平衡项同比增长率（%） |
| NETCASH_OPERATENOTE_YOY | Nullable(Float64) | 122,137 | 零值 13；负值 77,790 | min=-2,362,750,000, max=3,041,960, distinct 161,306 | 经营活动产生的现金流量净额（附注）同比增长率（%） |
| DEBT_TRANSFER_CAPITAL_YOY | Nullable(Float64) | 283,215 | 零值 0；负值 196 | min=-2,836.33, max=133,876,000, distinct 386 | 债务转为资本同比增长率（%） |
| CONVERT_BOND_1YEAR_YOY | Nullable(Float64) | 283,556 | 零值 0；负值 8 | min=-99.9971, max=673,936, distinct 57 | 一年内到期的可转换公司债券同比增长率（%） |
| FINLEASE_OBTAIN_FA_YOY | Nullable(Float64) | 282,538 | 零值 17；负值 574 | min=-241.554, max=178,697, distinct 1,034 | 融资租入固定资产同比增长率（%） |
| UNINVOLVE_INVESTFIN_OTHER_YOY | Nullable(Float64) | 281,190 | 零值 7；负值 1,295 | min=-5,333.98, max=159,485, distinct 2,266 | 不涉及现金收支的投资和筹资活动其他同比增长率（%） |
| END_CASH_YOY | Nullable(Float64) | 140,615 | 零值 11；负值 65,488 | min=-238.137, max=20,345,500, distinct 142,848 | 现金期末余额同比增长率（%） |
| BEGIN_CASH_YOY | Nullable(Float64) | 140,689 | 零值 18；负值 63,475 | min=-243.844, max=20,345,500, distinct 85,962 | 现金期初余额同比增长率（%） |
| END_CASH_EQUIVALENTS_YOY | Nullable(Float64) | 281,365 | 零值 45；负值 1,127 | min=-955.956, max=12,733,400,000, distinct 2,100 | 现金等价物期末余额同比增长率（%） |
| BEGIN_CASH_EQUIVALENTS_YOY | Nullable(Float64) | 281,308 | 零值 46；负值 1,136 | min=-292.569, max=12,733,400,000, distinct 1,592 | 现金等价物期初余额同比增长率（%） |
| CCE_ADD_OTHERNOTE_YOY | Nullable(Float64) | 283,567 | 零值 0；负值 25 | min=-199,800, max=44,605.8, distinct 46 | 现金及现金等价物净增加额（附注）同比增长率（%） |
| CCE_ADD_BALANCENOTE_YOY | Nullable(Float64) | 283,224 | 零值 0；负值 194 | min=-3,668,890,000, max=12,987.1, distinct 74 | 现金及现金等价物净增加额（附注）平衡项同比增长率（%） |
| CCE_ADDNOTE_YOY | Nullable(Float64) | 122,133 | 零值 19；负值 76,765 | min=-2,362,750,000, max=314,054,000,000, distinct 161,299 | 现金及现金等价物净增加额（附注）同比增长率（%） |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 204,324 | 空字符串 0；`1970-01-01` 0 | distinct 7 | 审计意见类型 |
| OSOPINION_TYPE | LowCardinality(Nullable(String)) | 283,598 | 空字符串 0；`1970-01-01` 0 | distinct 1 | 内控审计意见类型 |
| MINORITY_INTEREST | Nullable(Float64) | 246,362 | 零值 48；负值 12,015 | min=-3,574,000,000, max=12,277,000,000, distinct 37,034 | 少数股东损益 |
| MINORITY_INTEREST_YOY | Nullable(Float64) | 253,259 | 零值 6；负值 14,106 | min=-4,676,660, max=2,274,550,000, distinct 30,269 | 少数股东损益同比增长率（%） |
| USERIGHT_ASSET_AMORTIZE | Nullable(Float64) | 241,320 | 零值 440；负值 41 | min=-47,227,400, max=17,780,000,000, distinct 40,943 | 使用权资产摊销 |
| USERIGHT_ASSET_AMORTIZE_YOY | Nullable(Float64) | 250,536 | 零值 815；负值 13,671 | min=-11,289.2, max=63,908.7, distinct 32,082 | 使用权资产摊销同比增长率（%） |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 283,613/283,613，供应商前缀 0/283,613，纯数字 0/283,613，空值 0/283,613；`SECURITY_CODE`: canonical 后缀 0/283,613，供应商前缀 0/283,613，纯数字 283,613/283,613，空值 0/283,613
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：`REPORT_DATE`: 1998-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1998-07-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1998-07-14 至 2026-06-02，NULL 686 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME`, `SECURITY_TYPE_CODE`, `CURRENCY`
- 取值：`SECURITY_CODE`: `000766`(104), `000678`(104), `600795`(104), `600798`(103), `000796`(103), `000722`(103), `000758`(103), `600699`(103)；`SECURITY_NAME_ABBR`: `东方明珠`(159), `百联股份`(145), `通化金马`(104), `国电电力`(104), `襄阳轴承`(104), `郑州煤电`(103), `东百集团`(103), `湖南发展`(103)；`ORG_CODE`: `10004127`(176), `10004106`(174), `10004293`(132), `10116535`(128), `10005673`(104), `10005602`(104), `10634823`(104), `10004008`(103)；`ORG_TYPE`: `通用`(283,613)；`REPORT_TYPE`: `年报`(73,138), `一季报`(71,797), `中报`(70,167), `三季报`(68,511)；`REPORT_DATE_NAME`: `2026一季报`(5,099), `2025一季报`(5,088), `2025年报`(5,084), `2024一季报`(5,073), `2025三季报`(5,060), `2025中报`(5,051), `2024三季报`(5,034), `2024年报`(5,020)；`SECURITY_TYPE_CODE`: `058001001`(283,590), `058001008`(23)；`CURRENCY`: `CNY`(282,968), `NULL`(645)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 240 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 240 个数值字段执行 min/max、NULL、零值和负值检查；其中 227 个字段出现负值，212 个字段出现零值，121 个字段 NULL 数不低于 80%。 负值字段样例：`NETCASH_INVEST` 231,278 行(min=-332,948,000,000)，`CCE_ADD` 156,640 行(min=-91,197,500,000)，`NETCASH_FINANCE` 143,270 行(min=-178,876,000,000)，`NETCASH_INVEST_YOY` 140,760 行(min=-288,839,000)，`NETCASH_OPERATE_YOY` 132,660 行(min=-2,362,750,000)，`CCE_ADD_YOY` 130,449 行(min=-2,362,750,000)，`CONSTRUCT_LONG_ASSET_YOY` 129,487 行(min=-1,022,600)，`NETCASH_FINANCE_YOY` 127,894 行(min=-1,195,570,000)。 高 NULL 字段样例：`OPERATE_NETCASH_OTHER_YOY` 283,613 行，`FINANCE_NETCASH_OTHER_YOY` 283,613 行，`INVEST_NETCASH_OTHER_YOY` 283,612 行，`SUBSIDIARY_REDUCE_CASH_YOY` 283,606 行，`END_CCE_OTHER_YOY` 283,605 行，`CCE_ADD_OTHER_YOY` 283,593 行，`INSURED_INVEST_ADD_YOY` 283,583 行，`END_CCE_OTHER` 283,576 行。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECURITY_CODE` 为 6 位本地代码 | 中 | 283,613/283,613 行为纯数字 | 只作为 `security_local_code`，不可单独推出交易所 | 交易所归属或证券主数据修正延后 |
| 财务数值存在负值 | 低 | 227 个数值字段出现负值 | 负数符合财务科目/调整项可能性，staging 不过滤 | 口径解释和异常阈值延后 |

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

- 行数：283,613。
- 日期 / 分区范围：`REPORT_DATE`: 1998-06-30 至 2026-03-31，NULL 0 行，`1970-01-01` 占位 0 行；`NOTICE_DATE`: 1998-07-14 至 2026-05-15，NULL 0 行，`1970-01-01` 占位 0 行；`UPDATE_DATE`: 1998-07-14 至 2026-06-02，NULL 686 行，`1970-01-01` 占位 0 行
- 候选键重复：未发现重复。
- 关键 NULL / 占位值：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`SECURITY_CODE`: `000766`(104), `000678`(104), `600795`(104), `600798`(103), `000796`(103), `000722`(103), `000758`(103), `600699`(103)；`SECURITY_NAME_ABBR`: `东方明珠`(159), `百联股份`(145), `通化金马`(104), `国电电力`(104), `襄阳轴承`(104), `郑州煤电`(103), `东百集团`(103), `湖南发展`(103)；`ORG_CODE`: `10004127`(176), `10004106`(174), `10004293`(132), `10116535`(128), `10005673`(104), `10005602`(104), `10634823`(104), `10004008`(103)；`ORG_TYPE`: `通用`(283,613)；`REPORT_TYPE`: `年报`(73,138), `一季报`(71,797), `中报`(70,167), `三季报`(68,511)；`REPORT_DATE_NAME`: `2026一季报`(5,099), `2025一季报`(5,088), `2025年报`(5,084), `2024一季报`(5,073), `2025三季报`(5,060), `2025中报`(5,051), `2024三季报`(5,034), `2024年报`(5,020)；`SECURITY_TYPE_CODE`: `058001001`(283,590), `058001008`(23)；`CURRENCY`: `CNY`(282,968), `NULL`(645)
- 数值范围：已对 240 个数值字段执行 min/max、NULL、零值和负值检查；其中 227 个字段出现负值，212 个字段出现零值，121 个字段 NULL 数不低于 80%。 负值字段样例：`NETCASH_INVEST` 231,278 行(min=-332,948,000,000)，`CCE_ADD` 156,640 行(min=-91,197,500,000)，`NETCASH_FINANCE` 143,270 行(min=-178,876,000,000)，`NETCASH_INVEST_YOY` 140,760 行(min=-288,839,000)，`NETCASH_OPERATE_YOY` 132,660 行(min=-2,362,750,000)，`CCE_ADD_YOY` 130,449 行(min=-2,362,750,000)，`CONSTRUCT_LONG_ASSET_YOY` 129,487 行(min=-1,022,600)，`NETCASH_FINANCE_YOY` 127,894 行(min=-1,195,570,000)。 高 NULL 字段样例：`OPERATE_NETCASH_OTHER_YOY` 283,613 行，`FINANCE_NETCASH_OTHER_YOY` 283,613 行，`INVEST_NETCASH_OTHER_YOY` 283,612 行，`SUBSIDIARY_REDUCE_CASH_YOY` 283,606 行，`END_CCE_OTHER_YOY` 283,605 行，`CCE_ADD_OTHER_YOY` 283,593 行，`INSURED_INVEST_ADD_YOY` 283,583 行，`END_CCE_OTHER` 283,576 行。

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
select `SECUCODE`, `REPORT_DATE`, `SECURITY_CODE`, `NOTICE_DATE`, `UPDATE_DATE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME` from fleur_raw.eastmoney__cashflow_ytd limit 5
```

结果：

```text
[{'SECUCODE': '000005.SZ', 'REPORT_DATE': datetime.date(1998, 6, 30), 'SECURITY_CODE': '000005', 'NOTICE_DATE': datetime.date(1998, 8, 19), 'UPDATE_DATE': datetime.date(1998, 8, 19), 'SECURITY_NAME_ABBR': 'ST星源', 'ORG_CODE': '10004089', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1998中报'}, {'SECUCODE': '000014.SZ', 'REPORT_DATE': datetime.date(1998, 6, 30), 'SECURITY_CODE': '000014', 'NOTICE_DATE': datetime.date(1998, 8, 27), 'UPDATE_DATE': datetime.date(1998, 8, 27), 'SECURITY_NAME_ABBR': '沙河股份', 'ORG_CODE': '10004098', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1998中报'}, {'SECUCODE': '000015.SZ', 'REPORT_DATE': datetime.date(1998, 6, 30), 'SECURITY_CODE': '000015', 'NOTICE_DATE': datetime.date(1998, 8, 29), 'UPDATE_DATE': datetime.date(1998, 8, 29), 'SECURITY_NAME_ABBR': 'PT中浩A', 'ORG_CODE': '10004099', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1998中报'}, {'SECUCODE': '000021.SZ', 'REPORT_DATE': datetime.date(1998, 6, 30), 'SECURITY_CODE': '000021', 'NOTICE_DATE': datetime.date(1998, 8, 10), 'UPDATE_DATE': datetime.date(1998, 8, 10), 'SECURITY_NAME_ABBR': '深科技', 'ORG_CODE': '10004105', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1998中报'}, {'SECUCODE': '000027.SZ', 'REPORT_DATE': datetime.date(1998, 6, 30), 'SECURITY_CODE': '000027', 'NOTICE_DATE': datetime.date(1998, 8, 21), 'UPDATE_DATE': datetime.date(1998, 8, 21), 'SECURITY_NAME_ABBR': '深圳能源', 'ORG_CODE': '10634778', 'ORG_TYPE': '通用', 'REPORT_TYPE': '中报', 'REPORT_DATE_NAME': '1998中报'}]
```

### 行数统计

```sql
select count() from fleur_raw.eastmoney__cashflow_ytd
```

结果：

```text
[[283613]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `SECUCODE`, `REPORT_DATE`, count() as row_count from fleur_raw.eastmoney__cashflow_ytd group by `SECUCODE`, `REPORT_DATE` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 0, 'max_rows_per_key': 0}
```

### 证券代码格式：SECUCODE

```sql
select countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__cashflow_ytd
```

结果：

```text
{'canonical_suffix': 283613, 'vendor_prefix': 0, 'numeric_only': 0, 'empty_or_null': 0, 'row_count': 283613}
```

### 证券代码格式：SECURITY_CODE

```sql
select countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__cashflow_ytd
```

结果：

```text
{'canonical_suffix': 0, 'vendor_prefix': 0, 'numeric_only': 283613, 'empty_or_null': 0, 'row_count': 283613}
```
