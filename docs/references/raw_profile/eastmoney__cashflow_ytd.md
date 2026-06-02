# Raw 数据画像：eastmoney__cashflow_ytd

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__cashflow_ytd.yml`
- dbt source：`source('raw', 'eastmoney__cashflow_ytd')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__cashflow_ytd.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`eastmoney__cashflow_ytd`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__cashflow_ytd --execute --output ../docs/references/raw_profile/eastmoney__cashflow_ytd.md`，并补充 ClickHouse 结构化汇总查询
- 行数：283,613
- 数据范围：`REPORT_DATE`: 1998-06-30 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1998-07-14 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 686 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`eastmoney__cashflow_ytd`
- ClickHouse raw 表：`fleur_raw.eastmoney__cashflow_ytd`
- 表说明：EastMoney year-to-date cashflow F10 rows by natural-year raw partition.

## 2. 粒度与键

- 观察到的粒度：候选自然键为 `SECUCODE`, `REPORT_DATE`, `REPORT_TYPE`，本次 profiling 未发现重复。
- 候选自然键：`SECUCODE`, `REPORT_DATE`, `REPORT_TYPE`
- 重复检查：未发现重复
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；如果候选键重复，需要在 intermediate/mart 设计中处理。

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| SECUCODE | LowCardinality(String) | 未逐列统计 | 见关键字段画像 | 见关键字段画像 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 未逐列统计 | 见关键字段画像 | 见关键字段画像 | 证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 证券简称 |
| ORG_CODE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 机构代码 |
| ORG_TYPE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 机构类型 |
| REPORT_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 报告期 |
| REPORT_TYPE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 报告类型 |
| REPORT_DATE_NAME | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 报告期名称 |
| SECURITY_TYPE_CODE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 证券类型代码 |
| NOTICE_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 公告日期 |
| UPDATE_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 更新日期 |
| CURRENCY | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金流量表年初至报告期末金额使用的币种。 |
| SALES_SERVICES | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 销售商品、提供劳务收到的现金 |
| DEPOSIT_INTERBANK_ADD | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 同业存放净增加额 |
| LOAN_PBC_ADD | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 向央行借款净增加额 |
| OFI_BF_ADD | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 向其他金融机构拆入资金净增加额 |
| RECEIVE_ORIGIC_PREMIUM | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到原保险合同保费现金 |
| RECEIVE_REINSURE_NET | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到再保险业务现金净额 |
| INSURED_INVEST_ADD | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 保户储金及投资款净增加额 |
| DISPOSAL_TFA_ADD | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 处置交易性金融资产净增加额 |
| RECEIVE_INTEREST_COMMISSION | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收取利息和手续费现金 |
| BORROW_FUND_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 拆入资金净增加额 |
| LOAN_ADVANCE_REDUCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发放贷款及垫款净减少额 |
| REPO_BUSINESS_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 回购业务资金净增加额 |
| RECEIVE_TAX_REFUND | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到的税费返还 |
| RECEIVE_OTHER_OPERATE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到其他与经营活动有关的现金 |
| OPERATE_INFLOW_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流入其他 |
| OPERATE_INFLOW_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流入平衡项 |
| TOTAL_OPERATE_INFLOW | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流入小计 |
| BUY_SERVICES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 购买商品、接受劳务支付的现金 |
| LOAN_ADVANCE_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发放贷款及垫款净增加额 |
| PBC_INTERBANK_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 向央行借款净增加额 |
| PAY_ORIGIC_COMPENSATE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付原保险合同赔付款项现金 |
| PAY_INTEREST_COMMISSION | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付利息和手续费现金 |
| PAY_POLICY_BONUS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 保单红利支出 |
| PAY_STAFF_CASH | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付给职工以及为职工支付的现金 |
| PAY_ALL_TAX | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付的各项税费 |
| PAY_OTHER_OPERATE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付其他与经营活动有关的现金 |
| OPERATE_OUTFLOW_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流出其他 |
| OPERATE_OUTFLOW_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流出平衡项 |
| TOTAL_OPERATE_OUTFLOW | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流出小计 |
| OPERATE_NETCASH_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动净现金流量其他 |
| OPERATE_NETCASH_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动净现金流量平衡项 |
| NETCASH_OPERATE | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动产生的现金流量净额 |
| WITHDRAW_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收回投资收到的现金 |
| RECEIVE_INVEST_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 取得投资收益收到的现金 |
| DISPOSAL_LONG_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 处置固定资产等收回的现金净额 |
| DISPOSAL_SUBSIDIARY_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 处置子公司及其他营业单位收到的现金净额 |
| REDUCE_PLEDGE_TIMEDEPOSITS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 减少质押定期存款 |
| RECEIVE_OTHER_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到其他与投资活动有关的现金 |
| INVEST_INFLOW_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流入其他 |
| INVEST_INFLOW_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流入平衡项 |
| TOTAL_INVEST_INFLOW | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流入小计 |
| CONSTRUCT_LONG_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 购建固定资产等支付的现金 |
| INVEST_PAY_CASH | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资支付的现金 |
| PLEDGE_LOAN_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 质押贷款净增加额 |
| OBTAIN_SUBSIDIARY_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 取得子公司及其他营业单位支付的现金净额 |
| ADD_PLEDGE_TIMEDEPOSITS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 增加质押定期存款 |
| PAY_OTHER_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付其他与投资活动有关的现金 |
| INVEST_OUTFLOW_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流出其他 |
| INVEST_OUTFLOW_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流出平衡项 |
| TOTAL_INVEST_OUTFLOW | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流出小计 |
| INVEST_NETCASH_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动净现金流量其他 |
| INVEST_NETCASH_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动净现金流量平衡项 |
| NETCASH_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动产生的现金流量净额 |
| ACCEPT_INVEST_CASH | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 吸收投资收到的现金 |
| SUBSIDIARY_ACCEPT_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 子公司吸收少数股东投资收到的现金 |
| RECEIVE_LOAN_CASH | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 取得借款收到的现金 |
| ISSUE_BOND | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发行债券收到的现金 |
| RECEIVE_OTHER_FINANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到其他与筹资活动有关的现金 |
| FINANCE_INFLOW_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流入其他 |
| FINANCE_INFLOW_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流入平衡项 |
| TOTAL_FINANCE_INFLOW | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流入小计 |
| PAY_DEBT_CASH | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 偿还债务支付的现金 |
| ASSIGN_DIVIDEND_PORFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 分配股利、利润或偿付利息支付的现金 |
| SUBSIDIARY_PAY_DIVIDEND | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 子公司向少数股东支付的现金股利 |
| BUY_SUBSIDIARY_EQUITY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 子公司减少现金 |
| PAY_OTHER_FINANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付其他与筹资活动有关的现金 |
| SUBSIDIARY_REDUCE_CASH | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 子公司减少现金 |
| FINANCE_OUTFLOW_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流出其他 |
| FINANCE_OUTFLOW_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流出平衡项 |
| TOTAL_FINANCE_OUTFLOW | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流出小计 |
| FINANCE_NETCASH_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动净现金流量其他 |
| FINANCE_NETCASH_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动净现金流量平衡项 |
| NETCASH_FINANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动产生的现金流量净额 |
| RATE_CHANGE_EFFECT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 汇率变动对现金及现金等价物的影响 |
| CCE_ADD_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额其他 |
| CCE_ADD_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额平衡项 |
| CCE_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额 |
| BEGIN_CCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 期初现金及现金等价物余额 |
| END_CCE_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 期末现金及现金等价物余额其他 |
| END_CCE_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 期末现金及现金等价物余额平衡项 |
| END_CCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 期末现金及现金等价物余额 |
| NETPROFIT | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润（间接法起点） |
| ASSET_IMPAIRMENT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产减值准备 |
| FA_IR_DEPR | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 固定资产折旧、油气资产折耗、生产性生物资产折旧 |
| OILGAS_BIOLOGY_DEPR | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 油气资产折耗、生产性生物资产折旧 |
| IR_DEPR | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 折旧与摊销 |
| IA_AMORTIZE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 无形资产摊销 |
| LPE_AMORTIZE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期待摊费用摊销 |
| DEFER_INCOME_AMORTIZE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 待摊费用减少（减：增加） |
| PREPAID_EXPENSE_REDUCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预提费用增加（减：减少） |
| ACCRUED_EXPENSE_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预提费用变动 |
| DISPOSAL_LONGASSET_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 处置固定资产等的损失 |
| FA_SCRAP_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 固定资产报废损失 |
| FAIRVALUE_CHANGE_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 公允价值变动损失 |
| FINANCE_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 财务费用 |
| INVEST_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资损失 |
| DEFER_TAX | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税资产减少（增加以"-"号填列） |
| DT_ASSET_REDUCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税资产减少 |
| DT_LIAB_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税负债增加 |
| PREDICT_LIAB_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预计负债增加 |
| INVENTORY_REDUCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 存货的减少（增加以"-"号填列） |
| OPERATE_RECE_REDUCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营性应收项目的减少 |
| OPERATE_PAYABLE_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营性应付项目的增加 |
| OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金流量表年初至报告期末补充资料中的其他项目。 |
| OPERATE_NETCASH_OTHERNOTE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动产生的现金流量净额（附注） |
| OPERATE_NETCASH_BALANCENOTE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动净现金流量（附注）平衡项 |
| NETCASH_OPERATENOTE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动产生的现金流量净额（附注） |
| DEBT_TRANSFER_CAPITAL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 债务转为资本 |
| CONVERT_BOND_1YEAR | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一年内到期的可转换公司债券 |
| FINLEASE_OBTAIN_FA | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 融资租入固定资产 |
| UNINVOLVE_INVESTFIN_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 不涉及现金收支的投资和筹资活动其他 |
| END_CASH | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金期末余额 |
| BEGIN_CASH | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金期初余额 |
| END_CASH_EQUIVALENTS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金等价物期末余额 |
| BEGIN_CASH_EQUIVALENTS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金等价物期初余额 |
| CCE_ADD_OTHERNOTE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额（附注） |
| CCE_ADD_BALANCENOTE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额（附注）平衡项 |
| CCE_ADDNOTE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额（附注） |
| SALES_SERVICES_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 销售商品、提供劳务收到的现金同比增长率（%） |
| DEPOSIT_INTERBANK_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 同业存放净增加额同比增长率（%） |
| LOAN_PBC_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 向央行借款净增加额同比增长率（%） |
| OFI_BF_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 向其他金融机构拆入资金净增加额同比增长率（%） |
| RECEIVE_ORIGIC_PREMIUM_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到原保险合同保费现金同比增长率（%） |
| RECEIVE_REINSURE_NET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到再保险业务现金净额同比增长率（%） |
| INSURED_INVEST_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 保户储金及投资款净增加额同比增长率（%） |
| DISPOSAL_TFA_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 处置交易性金融资产净增加额同比增长率（%） |
| RECEIVE_INTEREST_COMMISSION_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收取利息和手续费现金同比增长率（%） |
| BORROW_FUND_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 拆入资金净增加额同比增长率（%） |
| LOAN_ADVANCE_REDUCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发放贷款及垫款净减少额同比增长率（%） |
| REPO_BUSINESS_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 回购业务资金净增加额同比增长率（%） |
| RECEIVE_TAX_REFUND_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到的税费返还同比增长率（%） |
| RECEIVE_OTHER_OPERATE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到其他与经营活动有关的现金同比增长率（%） |
| OPERATE_INFLOW_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流入其他同比增长率（%） |
| OPERATE_INFLOW_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流入平衡项同比增长率（%） |
| TOTAL_OPERATE_INFLOW_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流入小计同比增长率（%） |
| BUY_SERVICES_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 购买商品、接受劳务支付的现金同比增长率（%） |
| LOAN_ADVANCE_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发放贷款及垫款净增加额同比增长率（%） |
| PBC_INTERBANK_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 向央行借款净增加额同比增长率（%） |
| PAY_ORIGIC_COMPENSATE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付原保险合同赔付款项现金同比增长率（%） |
| PAY_INTEREST_COMMISSION_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付利息和手续费现金同比增长率（%） |
| PAY_POLICY_BONUS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 保单红利支出同比增长率（%） |
| PAY_STAFF_CASH_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付给职工以及为职工支付的现金同比增长率（%） |
| PAY_ALL_TAX_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付的各项税费同比增长率（%） |
| PAY_OTHER_OPERATE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付其他与经营活动有关的现金同比增长率（%） |
| OPERATE_OUTFLOW_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流出其他同比增长率（%） |
| OPERATE_OUTFLOW_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流出平衡项同比增长率（%） |
| TOTAL_OPERATE_OUTFLOW_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动现金流出小计同比增长率（%） |
| OPERATE_NETCASH_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动净现金流量其他同比增长率（%） |
| OPERATE_NETCASH_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动净现金流量平衡项同比增长率（%） |
| NETCASH_OPERATE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动产生的现金流量净额同比增长率（%） |
| WITHDRAW_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收回投资收到的现金同比增长率（%） |
| RECEIVE_INVEST_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 取得投资收益收到的现金同比增长率（%） |
| DISPOSAL_LONG_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 处置固定资产等收回的现金净额同比增长率（%） |
| DISPOSAL_SUBSIDIARY_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 处置子公司及其他营业单位收到的现金净额同比增长率（%） |
| REDUCE_PLEDGE_TIMEDEPOSITS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 减少质押定期存款同比增长率（%） |
| RECEIVE_OTHER_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到其他与投资活动有关的现金同比增长率（%） |
| INVEST_INFLOW_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流入其他同比增长率（%） |
| INVEST_INFLOW_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流入平衡项同比增长率（%） |
| TOTAL_INVEST_INFLOW_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流入小计同比增长率（%） |
| CONSTRUCT_LONG_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 购建固定资产等支付的现金同比增长率（%） |
| INVEST_PAY_CASH_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资支付的现金同比增长率（%） |
| PLEDGE_LOAN_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 质押贷款净增加额同比增长率（%） |
| OBTAIN_SUBSIDIARY_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 取得子公司及其他营业单位支付的现金净额同比增长率（%） |
| ADD_PLEDGE_TIMEDEPOSITS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 增加质押定期存款同比增长率（%） |
| PAY_OTHER_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付其他与投资活动有关的现金同比增长率（%） |
| INVEST_OUTFLOW_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流出其他同比增长率（%） |
| INVEST_OUTFLOW_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流出平衡项同比增长率（%） |
| TOTAL_INVEST_OUTFLOW_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动现金流出小计同比增长率（%） |
| INVEST_NETCASH_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动净现金流量其他同比增长率（%） |
| INVEST_NETCASH_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动净现金流量平衡项同比增长率（%） |
| NETCASH_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资活动产生的现金流量净额同比增长率（%） |
| ACCEPT_INVEST_CASH_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 吸收投资收到的现金同比增长率（%） |
| SUBSIDIARY_ACCEPT_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 子公司吸收少数股东投资收到的现金同比增长率（%） |
| RECEIVE_LOAN_CASH_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 取得借款收到的现金同比增长率（%） |
| ISSUE_BOND_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发行债券收到的现金同比增长率（%） |
| RECEIVE_OTHER_FINANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 收到其他与筹资活动有关的现金同比增长率（%） |
| FINANCE_INFLOW_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流入其他同比增长率（%） |
| FINANCE_INFLOW_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流入平衡项同比增长率（%） |
| TOTAL_FINANCE_INFLOW_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流入小计同比增长率（%） |
| PAY_DEBT_CASH_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 偿还债务支付的现金同比增长率（%） |
| ASSIGN_DIVIDEND_PORFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 分配股利、利润或偿付利息支付的现金同比增长率（%） |
| SUBSIDIARY_PAY_DIVIDEND_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 子公司向少数股东支付的现金股利同比增长率（%） |
| BUY_SUBSIDIARY_EQUITY_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 子公司减少现金同比增长率（%） |
| PAY_OTHER_FINANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 支付其他与筹资活动有关的现金同比增长率（%） |
| SUBSIDIARY_REDUCE_CASH_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 子公司减少现金同比增长率（%） |
| FINANCE_OUTFLOW_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流出其他同比增长率（%） |
| FINANCE_OUTFLOW_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流出平衡项同比增长率（%） |
| TOTAL_FINANCE_OUTFLOW_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动现金流出小计同比增长率（%） |
| FINANCE_NETCASH_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动净现金流量其他同比增长率（%） |
| FINANCE_NETCASH_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动净现金流量平衡项同比增长率（%） |
| NETCASH_FINANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 筹资活动产生的现金流量净额同比增长率（%） |
| RATE_CHANGE_EFFECT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 汇率变动对现金及现金等价物的影响同比增长率（%） |
| CCE_ADD_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额其他同比增长率（%） |
| CCE_ADD_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额平衡项同比增长率（%） |
| CCE_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额同比增长率（%） |
| BEGIN_CCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 期初现金及现金等价物余额同比增长率（%） |
| END_CCE_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 期末现金及现金等价物余额其他同比增长率（%） |
| END_CCE_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 期末现金及现金等价物余额平衡项同比增长率（%） |
| END_CCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 期末现金及现金等价物余额同比增长率（%） |
| NETPROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润同比增长率（%） |
| ASSET_IMPAIRMENT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产减值准备同比增长率（%） |
| FA_IR_DEPR_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 固定资产折旧、油气资产折耗、生产性生物资产折旧同比增长率（%） |
| OILGAS_BIOLOGY_DEPR_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 油气资产折耗、生产性生物资产折旧同比增长率（%） |
| IR_DEPR_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 折旧与摊销同比增长率（%） |
| IA_AMORTIZE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 无形资产摊销同比增长率（%） |
| LPE_AMORTIZE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期待摊费用摊销同比增长率（%） |
| DEFER_INCOME_AMORTIZE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 待摊费用减少（减：增加）同比增长率（%） |
| PREPAID_EXPENSE_REDUCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预提费用增加（减：减少）同比增长率（%） |
| ACCRUED_EXPENSE_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预提费用变动同比增长率（%） |
| DISPOSAL_LONGASSET_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 处置固定资产等的损失同比增长率（%） |
| FA_SCRAP_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 固定资产报废损失同比增长率（%） |
| FAIRVALUE_CHANGE_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 公允价值变动损失同比增长率（%） |
| FINANCE_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 财务费用同比增长率（%） |
| INVEST_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资损失同比增长率（%） |
| DEFER_TAX_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税资产减少（增加以"-"号填列）同比增长率（%） |
| DT_ASSET_REDUCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税资产减少同比增长率（%） |
| DT_LIAB_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税负债增加同比增长率（%） |
| PREDICT_LIAB_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预计负债增加同比增长率（%） |
| INVENTORY_REDUCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 存货的减少（增加以"-"号填列）同比增长率（%） |
| OPERATE_RECE_REDUCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营性应收项目的减少同比增长率（%） |
| OPERATE_PAYABLE_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营性应付项目的增加同比增长率（%） |
| OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他同比增长率（%） |
| OPERATE_NETCASH_OTHERNOTE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动产生的现金流量净额（附注）同比增长率（%） |
| OPERATE_NETCASH_BALANCENOTE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动净现金流量（附注）平衡项同比增长率（%） |
| NETCASH_OPERATENOTE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 经营活动产生的现金流量净额（附注）同比增长率（%） |
| DEBT_TRANSFER_CAPITAL_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 债务转为资本同比增长率（%） |
| CONVERT_BOND_1YEAR_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一年内到期的可转换公司债券同比增长率（%） |
| FINLEASE_OBTAIN_FA_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 融资租入固定资产同比增长率（%） |
| UNINVOLVE_INVESTFIN_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 不涉及现金收支的投资和筹资活动其他同比增长率（%） |
| END_CASH_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金期末余额同比增长率（%） |
| BEGIN_CASH_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金期初余额同比增长率（%） |
| END_CASH_EQUIVALENTS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金等价物期末余额同比增长率（%） |
| BEGIN_CASH_EQUIVALENTS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金等价物期初余额同比增长率（%） |
| CCE_ADD_OTHERNOTE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额（附注）同比增长率（%） |
| CCE_ADD_BALANCENOTE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额（附注）平衡项同比增长率（%） |
| CCE_ADDNOTE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金及现金等价物净增加额（附注）同比增长率（%） |
| OPINION_TYPE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 审计意见类型 |
| OSOPINION_TYPE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 内控审计意见类型 |
| MINORITY_INTEREST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 少数股东损益 |
| MINORITY_INTEREST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 少数股东损益同比增长率（%） |
| USERIGHT_ASSET_AMORTIZE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 使用权资产摊销 |
| USERIGHT_ASSET_AMORTIZE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 使用权资产摊销同比增长率（%） |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 283613/283613，供应商前缀 0/283613，纯数字 0/283613，空值 0/283613；`SECURITY_CODE`: canonical 后缀 0/283613，供应商前缀 0/283613，纯数字 283613/283613，空值 0/283613
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：EastMoney 后缀格式可直接作为 canonical security_code；本地代码必须仅作为 local code 使用。

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：`REPORT_DATE`: 1998-06-30 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1998-07-14 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 686 行
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME`, `SECURITY_TYPE_CODE`
- 取值：`SECUCODE`: `600795.SH`(104), `000766.SZ`(104), `000678.SZ`(104), `000021.SZ`(103), `600080.SH`(103), `600138.SH`(103), `000656.SZ`(103), `000702.SZ`(103)；`SECURITY_CODE`: `000766`(104), `000678`(104), `600795`(104), `600798`(103), `000796`(103), `000722`(103), `000758`(103), `600699`(103)；`SECURITY_NAME_ABBR`: `东方明珠`(159), `百联股份`(145), `国电电力`(104), `通化金马`(104), `襄阳轴承`(104), `通宝能源`(103), `中安科`(103), `厦门象屿`(103)；`ORG_CODE`: `10004127`(176), `10004106`(174), `10004293`(132), `10116535`(128), `10005673`(104), `10005602`(104), `10634823`(104), `10004008`(103)；`ORG_TYPE`: `通用`(283613)；`REPORT_TYPE`: `年报`(73138), `一季报`(71797), `中报`(70167), `三季报`(68511)；`REPORT_DATE_NAME`: `2026一季报`(5099), `2025一季报`(5088), `2025年报`(5084), `2024一季报`(5073), `2025三季报`(5060), `2025中报`(5051), `2024三季报`(5034), `2024年报`(5020)；`SECURITY_TYPE_CODE`: `058001001`(283590), `058001008`(23)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`NETCASH_OPERATE`, `NETPROFIT`, `SALES_SERVICES`, `DEPOSIT_INTERBANK_ADD`, `LOAN_PBC_ADD`, `OFI_BF_ADD`, `RECEIVE_ORIGIC_PREMIUM`, `RECEIVE_REINSURE_NET`, `INSURED_INVEST_ADD`, `DISPOSAL_TFA_ADD`
- 最小/最大值：`NETCASH_OPERATE` min=-122529899000.0, max=456847000000.0, zero=9, negative=105496, NULL=0；`NETPROFIT` min=-91810091101.49, max=183747000000.0, zero=132493, negative=23429, NULL=0；`SALES_SERVICES` min=-29150641.0, max=3577814000000.0, zero=618, negative=9, NULL=0；`DEPOSIT_INTERBANK_ADD` min=-75888794445.99, max=78286026036.56, zero=281859, negative=628, NULL=0；`LOAN_PBC_ADD` min=-5080000000.0, max=5227672331.67, zero=283302, negative=117, NULL=0；`OFI_BF_ADD` min=-16138000000.0, max=28859653769.21, zero=283087, negative=177, NULL=0；`RECEIVE_ORIGIC_PREMIUM` min=0.0, max=20983152538.41, zero=283270, negative=0, NULL=0；`RECEIVE_REINSURE_NET` min=-314030336.15, max=807228699.97, zero=283531, negative=31, NULL=0；`INSURED_INVEST_ADD` min=-39673665.86, max=5016097925.1, zero=283566, negative=2, NULL=0；`DISPOSAL_TFA_ADD` min=-5953787991.21, max=24533331064.82, zero=283318, negative=67, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `UPDATE_DATE` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 686 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `SECURITY_CODE` 只有 6 位本地代码 | 中 | 283613/283613 行 | 仅作为 `security_local_code`；不可单独推出交易所 | 需要其他字段或主数据补齐交易所 |
| `NETCASH_OPERATE` 存在负值 | 低 | 105496 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `NETPROFIT` 存在负值 | 低 | 23429 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `SALES_SERVICES` 存在负值 | 低 | 9 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `DEPOSIT_INTERBANK_ADD` 存在负值 | 低 | 628 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `LOAN_PBC_ADD` 存在负值 | 低 | 117 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `OFI_BF_ADD` 存在负值 | 低 | 177 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `RECEIVE_REINSURE_NET` 存在负值 | 低 | 31 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `INSURED_INVEST_ADD` 存在负值 | 低 | 2 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `DISPOSAL_TFA_ADD` 存在负值 | 低 | 67 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |

## 6. 建议的 Staging 转换

- 重命名：按 `pipeline/elt/metadata/field_glossary.yml` 选择 canonical 字段；不要仅凭 raw 字段名自动扩展全部宽表字段。
- 类型转换：raw Date/Bool/Float/Int 类型已由 ClickHouse schema 承载；字符串日期或占位日期需要显式处理。
- 标准化：证券代码、交易所、本地代码使用项目 macro；文本清洗限于 trim/nullif 等 source-local 规则。
- NULL 处理：空字符串、`1970-01-01` 和明显缺失值可转 NULL，但必须在 YAML `config.meta.normalization` 记录。
- 测试：候选键字段、日期字段和 canonical security_code 应优先加 `not_null`/format tests；非封闭业务文本不加 accepted-values。
- YAML 元数据：每个 staging 输出字段必须记录 `config.meta.source_columns`；派生字段记录 normalization 来源和输入格式。

## 7. 延后到 Intermediate/Mart

- 跨源 join：证券主数据、行业/题材实体匹配、财务 statement 合并均延后。
- 需要优先级判断的去重：候选键重复或多公告版本选择不在 staging 静默处理。
- 主数据修正：证券代码历史、上市/退市状态、交易所归属修正延后。
- 粒度变化：财报宽表拆长表、事件合并、题材归并和行情事实组装延后。
- 业务指标逻辑：财务科目重算、同比/环比口径、限售/分红状态解释延后。

## 8. 待确认问题

- [ ] 具体 staging model 落地时，针对实际暴露字段补充更细的字段级 tests 和单位 metadata。
- [ ] 对候选键存在重复的表，确认是否需要保留 source-local 行版本字段或推迟到 intermediate 去重。

## 9. 验收清单

- [x] 已抽样 raw source。
- [x] 已记录行数和日期/分区范围。
- [x] 已评估粒度和候选键。
- [x] 已完成关键字段画像。
- [x] 已列出 staging 转换建议。
- [x] 已列出延后处理事项。
- [x] 已提出测试或明确豁免。

## Profiling SQL 与结果摘要

- `select count() from fleur_raw.eastmoney__cashflow_ytd`：283,613
- 日期字段范围：`REPORT_DATE`: 1998-06-30 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1998-07-14 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 686 行
- 证券代码格式：`SECUCODE`: canonical 后缀 283613/283613，供应商前缀 0/283613，纯数字 0/283613，空值 0/283613；`SECURITY_CODE`: canonical 后缀 0/283613，供应商前缀 0/283613，纯数字 283613/283613，空值 0/283613
- 候选键重复：未发现重复
- 枚举 top values：`SECUCODE`: `600795.SH`(104), `000766.SZ`(104), `000678.SZ`(104), `000021.SZ`(103), `600080.SH`(103), `600138.SH`(103), `000656.SZ`(103), `000702.SZ`(103)；`SECURITY_CODE`: `000766`(104), `000678`(104), `600795`(104), `600798`(103), `000796`(103), `000722`(103), `000758`(103), `600699`(103)；`SECURITY_NAME_ABBR`: `东方明珠`(159), `百联股份`(145), `国电电力`(104), `通化金马`(104), `襄阳轴承`(104), `通宝能源`(103), `中安科`(103), `厦门象屿`(103)；`ORG_CODE`: `10004127`(176), `10004106`(174), `10004293`(132), `10116535`(128), `10005673`(104), `10005602`(104), `10634823`(104), `10004008`(103)；`ORG_TYPE`: `通用`(283613)；`REPORT_TYPE`: `年报`(73138), `一季报`(71797), `中报`(70167), `三季报`(68511)；`REPORT_DATE_NAME`: `2026一季报`(5099), `2025一季报`(5088), `2025年报`(5084), `2024一季报`(5073), `2025三季报`(5060), `2025中报`(5051), `2024三季报`(5034), `2024年报`(5020)；`SECURITY_TYPE_CODE`: `058001001`(283590), `058001008`(23)
- 数值范围摘要：`NETCASH_OPERATE` min=-122529899000.0, max=456847000000.0, zero=9, negative=105496, NULL=0；`NETPROFIT` min=-91810091101.49, max=183747000000.0, zero=132493, negative=23429, NULL=0；`SALES_SERVICES` min=-29150641.0, max=3577814000000.0, zero=618, negative=9, NULL=0；`DEPOSIT_INTERBANK_ADD` min=-75888794445.99, max=78286026036.56, zero=281859, negative=628, NULL=0；`LOAN_PBC_ADD` min=-5080000000.0, max=5227672331.67, zero=283302, negative=117, NULL=0；`OFI_BF_ADD` min=-16138000000.0, max=28859653769.21, zero=283087, negative=177, NULL=0；`RECEIVE_ORIGIC_PREMIUM` min=0.0, max=20983152538.41, zero=283270, negative=0, NULL=0；`RECEIVE_REINSURE_NET` min=-314030336.15, max=807228699.97, zero=283531, negative=31, NULL=0；`INSURED_INVEST_ADD` min=-39673665.86, max=5016097925.1, zero=283566, negative=2, NULL=0；`DISPOSAL_TFA_ADD` min=-5953787991.21, max=24533331064.82, zero=283318, negative=67, NULL=0
