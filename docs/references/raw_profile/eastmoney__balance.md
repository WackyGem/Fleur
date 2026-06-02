# Raw 数据画像：eastmoney__balance

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__balance.yml`
- dbt source：`source('raw', 'eastmoney__balance')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__balance.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`eastmoney__balance`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__balance --execute --output ../docs/references/raw_profile/eastmoney__balance.md`，并补充 ClickHouse 结构化汇总查询
- 行数：284,265
- 数据范围：`REPORT_DATE`: 1989-12-31 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 976 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`eastmoney__balance`
- ClickHouse raw 表：`fleur_raw.eastmoney__balance`
- 表说明：EastMoney balance sheet F10 rows by natural-year raw partition.

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
| CURRENCY | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产负债表披露金额使用的币种。 |
| ACCEPT_DEPOSIT_INTERBANK | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 同业存放 |
| ACCOUNTS_PAYABLE | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付账款 |
| ACCOUNTS_RECE | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收账款 |
| ACCRUED_EXPENSE | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预提费用 |
| ADVANCE_RECEIVABLES | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预收款项 |
| AGENT_TRADE_SECURITY | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 代理买卖证券款 |
| AGENT_UNDERWRITE_SECURITY | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 代理承销证券款 |
| AMORTIZE_COST_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以摊余成本计量的金融资产 |
| AMORTIZE_COST_FINLIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以摊余成本计量的金融负债 |
| AMORTIZE_COST_NCFINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动金融资产（摊余成本） |
| AMORTIZE_COST_NCFINLIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动金融负债（摊余成本） |
| APPOINT_FVTPL_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 指定为FVTPL的金融资产 |
| APPOINT_FVTPL_FINLIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 指定为FVTPL的金融负债 |
| ASSET_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产平衡项 |
| ASSET_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产其他项 |
| ASSIGN_CASH_DIVIDEND | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付现金股利 |
| AVAILABLE_SALE_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 可供出售金融资产 |
| BOND_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付债券 |
| BORROW_FUND | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 拆入资金 |
| BUY_RESALE_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 买入返售金融资产 |
| CAPITAL_RESERVE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资本公积 |
| CIP | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 在建工程 |
| CONSUMPTIVE_BIOLOGICAL_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 消耗性生物资产 |
| CONTRACT_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合同资产 |
| CONTRACT_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合同负债 |
| CONVERT_DIFF | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 外币报表折算差额 |
| CREDITOR_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 债权投资 |
| CURRENT_ASSET_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动资产平衡项 |
| CURRENT_ASSET_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动资产其他项 |
| CURRENT_LIAB_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动负债平衡项 |
| CURRENT_LIAB_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动负债其他项 |
| DEFER_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延收益 |
| DEFER_INCOME_1YEAR | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一年内到期递延收益 |
| DEFER_TAX_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税资产 |
| DEFER_TAX_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税负债 |
| DERIVE_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 衍生金融资产 |
| DERIVE_FINLIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 衍生金融负债 |
| DEVELOP_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 开发支出 |
| DIV_HOLDSALE_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有待售资产（除） |
| DIV_HOLDSALE_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有待售负债（除） |
| DIVIDEND_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付股利 |
| DIVIDEND_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收股利 |
| EQUITY_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 所有者权益平衡项 |
| EQUITY_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 所有者权益其他项 |
| EXPORT_REFUND_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收出口退税 |
| FEE_COMMISSION_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付手续费及佣金 |
| FIN_FUND | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 金融往来资金 |
| FINANCE_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 金融应收款 |
| FIXED_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 固定资产 |
| FIXED_ASSET_DISPOSAL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 固定资产清理 |
| FVTOCI_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以公允价值计量且其变动计入其他综合收益的金融资产 |
| FVTOCI_NCFINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他非流动金融资产（FVTOCI） |
| FVTPL_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以公允价值计量且其变动计入当期损益的金融资产 |
| FVTPL_FINLIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以公允价值计量且其变动计入当期损益的金融负债 |
| GENERAL_RISK_RESERVE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一般风险准备 |
| GOODWILL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产负债表披露的商誉金额。 |
| HOLD_MATURITY_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有至到期投资 |
| HOLDSALE_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有待售资产 |
| HOLDSALE_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有待售负债 |
| INSURANCE_CONTRACT_RESERVE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 保险合同准备金 |
| INTANGIBLE_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 无形资产 |
| INTEREST_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付利息 |
| INTEREST_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收利息 |
| INTERNAL_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 内部应付款 |
| INTERNAL_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 内部应收款 |
| INVENTORY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产负债表披露的存货金额。 |
| INVEST_REALESTATE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资性房地产 |
| LEASE_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 租赁负债 |
| LEND_FUND | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 拆出资金 |
| LIAB_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债平衡项 |
| LIAB_EQUITY_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债和所有者权益平衡项 |
| LIAB_EQUITY_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债和所有者权益其他项 |
| LIAB_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债其他项 |
| LOAN_ADVANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发放贷款及垫款 |
| LOAN_PBC | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 向央行借款 |
| LONG_EQUITY_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期股权投资 |
| LONG_LOAN | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期借款 |
| LONG_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期应付款 |
| LONG_PREPAID_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期待摊费用 |
| LONG_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期应收款 |
| LONG_STAFFSALARY_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期应付职工薪酬 |
| MINORITY_EQUITY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 少数股东权益 |
| MONETARYFUNDS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 货币资金 |
| NONCURRENT_ASSET_1YEAR | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一年内到期的非流动资产 |
| NONCURRENT_ASSET_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产平衡项 |
| NONCURRENT_ASSET_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产其他项 |
| NONCURRENT_LIAB_1YEAR | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一年内到期的非流动负债 |
| NONCURRENT_LIAB_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动负债平衡项 |
| NONCURRENT_LIAB_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动负债其他项 |
| NOTE_ACCOUNTS_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付票据及应付账款 |
| NOTE_ACCOUNTS_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收票据及应收账款 |
| NOTE_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付票据 |
| NOTE_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收票据 |
| OIL_GAS_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 油气资产 |
| OTHER_COMPRE_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他综合收益 |
| OTHER_CREDITOR_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他债权投资 |
| OTHER_CURRENT_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他流动资产 |
| OTHER_CURRENT_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他流动负债 |
| OTHER_EQUITY_INVEST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他权益工具投资 |
| OTHER_EQUITY_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他权益其他项 |
| OTHER_EQUITY_TOOL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他权益工具 |
| OTHER_NONCURRENT_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他非流动资产 |
| OTHER_NONCURRENT_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他非流动金融资产 |
| OTHER_NONCURRENT_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他非流动负债 |
| OTHER_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他应付款 |
| OTHER_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他应收款 |
| PARENT_EQUITY_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母权益平衡项 |
| PARENT_EQUITY_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母权益其他项 |
| PERPETUAL_BOND | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 永续债 |
| PERPETUAL_BOND_PAYBALE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 永续债（负债端） |
| PREDICT_CURRENT_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预计流动负债 |
| PREDICT_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预计负债 |
| PREFERRED_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 优先股 |
| PREFERRED_SHARES_PAYBALE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付优先股 |
| PREMIUM_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预收保费 |
| PREPAYMENT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预付款项 |
| PRODUCTIVE_BIOLOGY_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 生产性生物资产 |
| PROJECT_MATERIAL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 工程物资 |
| RC_RESERVE_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 再保合同应收准备金 |
| REINSURE_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付再保款 |
| REINSURE_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收再保款 |
| SELL_REPO_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 卖出回购金融资产款 |
| SETTLE_EXCESS_RESERVE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 清算备付金 |
| SHARE_CAPITAL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 实收资本（股本） |
| SHORT_BOND_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 短期应付债券 |
| SHORT_FIN_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 短期金融负债 |
| SHORT_LOAN | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 短期借款 |
| SPECIAL_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 专项应付款 |
| SPECIAL_RESERVE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 专项储备 |
| STAFF_SALARY_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付职工薪酬 |
| SUBSIDY_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收补贴 |
| SURPLUS_RESERVE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 盈余公积 |
| TAX_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应交税费 |
| TOTAL_ASSETS | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产总计 |
| TOTAL_CURRENT_ASSETS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动资产合计 |
| TOTAL_CURRENT_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动负债合计 |
| TOTAL_EQUITY | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 所有者权益合计 |
| TOTAL_LIAB_EQUITY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债和所有者权益总计 |
| TOTAL_LIABILITIES | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债合计 |
| TOTAL_NONCURRENT_ASSETS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产合计 |
| TOTAL_NONCURRENT_LIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动负债合计 |
| TOTAL_OTHER_PAYABLE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他应付款合计 |
| TOTAL_OTHER_RECE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他应收款合计 |
| TOTAL_PARENT_EQUITY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归属于母公司股东权益合计 |
| TRADE_FINASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 交易性金融资产 |
| TRADE_FINASSET_NOTFVTPL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非FVTPL交易性金融资产 |
| TRADE_FINLIAB | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 交易性金融负债 |
| TRADE_FINLIAB_NOTFVTPL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非FVTPL交易性金融负债 |
| TREASURY_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 库存股 |
| UNASSIGN_RPOFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 未分配利润 |
| UNCONFIRM_INVEST_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 未确认投资损失 |
| USERIGHT_ASSET | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 使用权资产 |
| ACCEPT_DEPOSIT_INTERBANK_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 同业存放同比增长率（%） |
| ACCOUNTS_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付账款同比增长率（%） |
| ACCOUNTS_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收账款同比增长率（%） |
| ACCRUED_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预提费用同比增长率（%） |
| ADVANCE_RECEIVABLES_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预收款项同比增长率（%） |
| AGENT_TRADE_SECURITY_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 代理买卖证券款同比增长率（%） |
| AGENT_UNDERWRITE_SECURITY_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 代理承销证券款同比增长率（%） |
| AMORTIZE_COST_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以摊余成本计量的金融资产同比增长率（%） |
| AMORTIZE_COST_FINLIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以摊余成本计量的金融负债同比增长率（%） |
| AMORTIZE_COST_NCFINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动金融资产（摊余成本）同比增长率（%） |
| AMORTIZE_COST_NCFINLIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动金融负债（摊余成本）同比增长率（%） |
| APPOINT_FVTPL_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 指定为FVTPL的金融资产同比增长率（%） |
| APPOINT_FVTPL_FINLIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 指定为FVTPL的金融负债同比增长率（%） |
| ASSET_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产平衡项同比增长率（%） |
| ASSET_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产其他项同比增长率（%） |
| ASSIGN_CASH_DIVIDEND_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付现金股利同比增长率（%） |
| AVAILABLE_SALE_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 可供出售金融资产同比增长率（%） |
| BOND_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付债券同比增长率（%） |
| BORROW_FUND_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 拆入资金同比增长率（%） |
| BUY_RESALE_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 买入返售金融资产同比增长率（%） |
| CAPITAL_RESERVE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资本公积同比增长率（%） |
| CIP_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 在建工程同比增长率（%） |
| CONSUMPTIVE_BIOLOGICAL_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 消耗性生物资产同比增长率（%） |
| CONTRACT_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合同资产同比增长率（%） |
| CONTRACT_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合同负债同比增长率（%） |
| CONVERT_DIFF_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 外币报表折算差额同比增长率（%） |
| CREDITOR_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 债权投资同比增长率（%） |
| CURRENT_ASSET_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动资产平衡项同比增长率（%） |
| CURRENT_ASSET_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动资产其他项同比增长率（%） |
| CURRENT_LIAB_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动负债平衡项同比增长率（%） |
| CURRENT_LIAB_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动负债其他项同比增长率（%） |
| DEFER_INCOME_1YEAR_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一年内到期递延收益同比增长率（%） |
| DEFER_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延收益同比增长率（%） |
| DEFER_TAX_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税资产同比增长率（%） |
| DEFER_TAX_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 递延所得税负债同比增长率（%） |
| DERIVE_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 衍生金融资产同比增长率（%） |
| DERIVE_FINLIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 衍生金融负债同比增长率（%） |
| DEVELOP_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 开发支出同比增长率（%） |
| DIV_HOLDSALE_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有待售资产（除）同比增长率（%） |
| DIV_HOLDSALE_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有待售负债（除）同比增长率（%） |
| DIVIDEND_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付股利同比增长率（%） |
| DIVIDEND_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收股利同比增长率（%） |
| EQUITY_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 所有者权益平衡项同比增长率（%） |
| EQUITY_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 所有者权益其他项同比增长率（%） |
| EXPORT_REFUND_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收出口退税同比增长率（%） |
| FEE_COMMISSION_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付手续费及佣金同比增长率（%） |
| FIN_FUND_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 金融往来资金同比增长率（%） |
| FINANCE_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 金融应收款同比增长率（%） |
| FIXED_ASSET_DISPOSAL_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 固定资产清理同比增长率（%） |
| FIXED_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 固定资产同比增长率（%） |
| FVTOCI_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以公允价值计量且其变动计入其他综合收益的金融资产同比增长率（%） |
| FVTOCI_NCFINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他非流动金融资产（FVTOCI）同比增长率（%） |
| FVTPL_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以公允价值计量且其变动计入当期损益的金融资产同比增长率（%） |
| FVTPL_FINLIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以公允价值计量且其变动计入当期损益的金融负债同比增长率（%） |
| GENERAL_RISK_RESERVE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一般风险准备同比增长率（%） |
| GOODWILL_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 商誉同比增长率（%） |
| HOLD_MATURITY_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有至到期投资同比增长率（%） |
| HOLDSALE_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有待售资产同比增长率（%） |
| HOLDSALE_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有待售负债同比增长率（%） |
| INSURANCE_CONTRACT_RESERVE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 保险合同准备金同比增长率（%） |
| INTANGIBLE_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 无形资产同比增长率（%） |
| INTEREST_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付利息同比增长率（%） |
| INTEREST_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收利息同比增长率（%） |
| INTERNAL_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 内部应付款同比增长率（%） |
| INTERNAL_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 内部应收款同比增长率（%） |
| INVENTORY_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 存货同比增长率（%） |
| INVEST_REALESTATE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资性房地产同比增长率（%） |
| LEASE_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 租赁负债同比增长率（%） |
| LEND_FUND_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 拆出资金同比增长率（%） |
| LIAB_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债平衡项同比增长率（%） |
| LIAB_EQUITY_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债和所有者权益平衡项同比增长率（%） |
| LIAB_EQUITY_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债和所有者权益其他项同比增长率（%） |
| LIAB_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债其他项同比增长率（%） |
| LOAN_ADVANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发放贷款及垫款同比增长率（%） |
| LOAN_PBC_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 向央行借款同比增长率（%） |
| LONG_EQUITY_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期股权投资同比增长率（%） |
| LONG_LOAN_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期借款同比增长率（%） |
| LONG_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期应付款同比增长率（%） |
| LONG_PREPAID_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期待摊费用同比增长率（%） |
| LONG_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期应收款同比增长率（%） |
| LONG_STAFFSALARY_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 长期应付职工薪酬同比增长率（%） |
| MINORITY_EQUITY_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 少数股东权益同比增长率（%） |
| MONETARYFUNDS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 货币资金同比增长率（%） |
| NONCURRENT_ASSET_1YEAR_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一年内到期的非流动资产同比增长率（%） |
| NONCURRENT_ASSET_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产平衡项同比增长率（%） |
| NONCURRENT_ASSET_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产其他项同比增长率（%） |
| NONCURRENT_LIAB_1YEAR_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 一年内到期的非流动负债同比增长率（%） |
| NONCURRENT_LIAB_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动负债平衡项同比增长率（%） |
| NONCURRENT_LIAB_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动负债其他项同比增长率（%） |
| NOTE_ACCOUNTS_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付票据及应付账款同比增长率（%） |
| NOTE_ACCOUNTS_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收票据及应收账款同比增长率（%） |
| NOTE_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付票据同比增长率（%） |
| NOTE_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收票据同比增长率（%） |
| OIL_GAS_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 油气资产同比增长率（%） |
| OTHER_COMPRE_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他综合收益总额同比增长率（%） |
| OTHER_CREDITOR_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他债权投资同比增长率（%） |
| OTHER_CURRENT_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他流动资产同比增长率（%） |
| OTHER_CURRENT_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他流动负债同比增长率（%） |
| OTHER_EQUITY_INVEST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他权益工具投资同比增长率（%） |
| OTHER_EQUITY_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他权益其他项同比增长率（%） |
| OTHER_EQUITY_TOOL_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他权益工具同比增长率（%） |
| OTHER_NONCURRENT_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他非流动资产同比增长率（%） |
| OTHER_NONCURRENT_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他非流动金融资产同比增长率（%） |
| OTHER_NONCURRENT_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他非流动负债同比增长率（%） |
| OTHER_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他应付款同比增长率（%） |
| OTHER_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他应收款同比增长率（%） |
| PARENT_EQUITY_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母权益平衡项同比增长率（%） |
| PARENT_EQUITY_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母权益其他项同比增长率（%） |
| PERPETUAL_BOND_PAYBALE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 永续债（负债端）同比增长率（%） |
| PERPETUAL_BOND_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 永续债同比增长率（%） |
| PREDICT_CURRENT_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预计流动负债同比增长率（%） |
| PREDICT_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预计负债同比增长率（%） |
| PREFERRED_SHARES_PAYBALE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付优先股同比增长率（%） |
| PREFERRED_SHARES_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 优先股同比增长率（%） |
| PREMIUM_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预收保费同比增长率（%） |
| PREPAYMENT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 预付款项同比增长率（%） |
| PRODUCTIVE_BIOLOGY_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 生产性生物资产同比增长率（%） |
| PROJECT_MATERIAL_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 工程物资同比增长率（%） |
| RC_RESERVE_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 再保合同应收准备金同比增长率（%） |
| REINSURE_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付再保款同比增长率（%） |
| REINSURE_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收再保款同比增长率（%） |
| SELL_REPO_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 卖出回购金融资产款同比增长率（%） |
| SETTLE_EXCESS_RESERVE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 清算备付金同比增长率（%） |
| SHARE_CAPITAL_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 实收资本（股本）同比增长率（%） |
| SHORT_BOND_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 短期应付债券同比增长率（%） |
| SHORT_FIN_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 短期金融负债同比增长率（%） |
| SHORT_LOAN_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 短期借款同比增长率（%） |
| SPECIAL_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 专项应付款同比增长率（%） |
| SPECIAL_RESERVE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 专项储备同比增长率（%） |
| STAFF_SALARY_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应付职工薪酬同比增长率（%） |
| SUBSIDY_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应收补贴同比增长率（%） |
| SURPLUS_RESERVE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 盈余公积同比增长率（%） |
| TAX_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 应交税费同比增长率（%） |
| TOTAL_ASSETS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产总计同比增长率（%） |
| TOTAL_CURRENT_ASSETS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动资产合计同比增长率（%） |
| TOTAL_CURRENT_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流动负债合计同比增长率（%） |
| TOTAL_EQUITY_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 所有者权益合计同比增长率（%） |
| TOTAL_LIAB_EQUITY_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债和所有者权益总计同比增长率（%） |
| TOTAL_LIABILITIES_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 负债合计同比增长率（%） |
| TOTAL_NONCURRENT_ASSETS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产合计同比增长率（%） |
| TOTAL_NONCURRENT_LIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动负债合计同比增长率（%） |
| TOTAL_OTHER_PAYABLE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他应付款合计同比增长率（%） |
| TOTAL_OTHER_RECE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他应收款合计同比增长率（%） |
| TOTAL_PARENT_EQUITY_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归属于母公司股东权益合计同比增长率（%） |
| TRADE_FINASSET_NOTFVTPL_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非FVTPL交易性金融资产同比增长率（%） |
| TRADE_FINASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 交易性金融资产同比增长率（%） |
| TRADE_FINLIAB_NOTFVTPL_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非FVTPL交易性金融负债同比增长率（%） |
| TRADE_FINLIAB_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 交易性金融负债同比增长率（%） |
| TREASURY_SHARES_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 库存股同比增长率（%） |
| UNASSIGN_RPOFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 未分配利润同比增长率（%） |
| UNCONFIRM_INVEST_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 未确认投资损失同比增长率（%） |
| USERIGHT_ASSET_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 使用权资产同比增长率（%） |
| OPINION_TYPE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 审计意见类型 |
| OSOPINION_TYPE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 内控审计意见类型 |
| LISTING_STATE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 上市状态 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 284265/284265，供应商前缀 0/284265，纯数字 0/284265，空值 0/284265；`SECURITY_CODE`: canonical 后缀 0/284265，供应商前缀 0/284265，纯数字 284265/284265，空值 0/284265
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：EastMoney 后缀格式可直接作为 canonical security_code；本地代码必须仅作为 local code 使用。

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：`REPORT_DATE`: 1989-12-31 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 976 行
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME`, `SECURITY_TYPE_CODE`
- 取值：`SECUCODE`: `600653.SH`(122), `600654.SH`(122), `600651.SH`(121), `600601.SH`(121), `600602.SH`(120), `600610.SH`(120), `600608.SH`(118), `600605.SH`(118)；`SECURITY_CODE`: `600654`(122), `600653`(122), `600651`(121), `600601`(121), `600610`(120), `600602`(120), `000501`(118), `600605`(118)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(122), `申华控股`(122), `飞乐音响`(121), `方正科技`(121), `云赛智联`(120), `中毅达`(120)；`ORG_CODE`: `10004106`(198), `10004127`(198), `10004293`(157), `10003964`(122), `10003963`(122), `10116535`(122), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(284265)；`REPORT_TYPE`: `年报`(75980), `中报`(71407), `一季报`(69943), `三季报`(66935)；`REPORT_DATE_NAME`: `2026一季报`(5099), `2025年报`(5084), `2025三季报`(5060), `2025中报`(5051), `2025一季报`(5042), `2024年报`(5020), `2024一季报`(5009), `2024三季报`(5005)；`SECURITY_TYPE_CODE`: `058001001`(284243), `058001008`(22)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`TOTAL_ASSETS`, `TOTAL_EQUITY`, `TOTAL_LIABILITIES`, `ACCEPT_DEPOSIT_INTERBANK`, `ACCOUNTS_PAYABLE`, `ACCOUNTS_RECE`, `ACCRUED_EXPENSE`, `ADVANCE_RECEIVABLES`, `AGENT_TRADE_SECURITY`, `AGENT_UNDERWRITE_SECURITY`
- 最小/最大值：`TOTAL_ASSETS` min=0.0, max=3613674212000.0, zero=6, negative=0, NULL=0；`TOTAL_EQUITY` min=-31171741000.0, max=1835768000000.0, zero=22, negative=3327, NULL=0；`TOTAL_LIABILITIES` min=-444400844.0, max=2775299325000.0, zero=271, negative=18, NULL=0；`ACCEPT_DEPOSIT_INTERBANK` min=0.0, max=777003282181.39, zero=282525, negative=0, NULL=0；`ACCOUNTS_PAYABLE` min=-44288812.27, max=997477873000.0, zero=9830, negative=20, NULL=0；`ACCOUNTS_RECE` min=-1878640045.72, max=442286697000.0, zero=7456, negative=24, NULL=0；`ACCRUED_EXPENSE` min=0.0, max=3512824010.83, zero=283532, negative=0, NULL=0；`ADVANCE_RECEIVABLES` min=-36425028.35, max=407882270730.4, zero=85079, negative=44, NULL=0；`AGENT_TRADE_SECURITY` min=0.0, max=181414551134.91, zero=283741, negative=0, NULL=0；`AGENT_UNDERWRITE_SECURITY` min=0.0, max=1703950000.0, zero=284216, negative=0, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `UPDATE_DATE` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 976 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `SECURITY_CODE` 只有 6 位本地代码 | 中 | 284265/284265 行 | 仅作为 `security_local_code`；不可单独推出交易所 | 需要其他字段或主数据补齐交易所 |
| `TOTAL_EQUITY` 存在负值 | 低 | 3327 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `TOTAL_LIABILITIES` 存在负值 | 低 | 18 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `ACCOUNTS_PAYABLE` 存在负值 | 低 | 20 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `ACCOUNTS_RECE` 存在负值 | 低 | 24 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `ADVANCE_RECEIVABLES` 存在负值 | 低 | 44 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |

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

- `select count() from fleur_raw.eastmoney__balance`：284,265
- 日期字段范围：`REPORT_DATE`: 1989-12-31 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 976 行
- 证券代码格式：`SECUCODE`: canonical 后缀 284265/284265，供应商前缀 0/284265，纯数字 0/284265，空值 0/284265；`SECURITY_CODE`: canonical 后缀 0/284265，供应商前缀 0/284265，纯数字 284265/284265，空值 0/284265
- 候选键重复：未发现重复
- 枚举 top values：`SECUCODE`: `600653.SH`(122), `600654.SH`(122), `600651.SH`(121), `600601.SH`(121), `600602.SH`(120), `600610.SH`(120), `600608.SH`(118), `600605.SH`(118)；`SECURITY_CODE`: `600654`(122), `600653`(122), `600651`(121), `600601`(121), `600610`(120), `600602`(120), `000501`(118), `600605`(118)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(122), `申华控股`(122), `飞乐音响`(121), `方正科技`(121), `云赛智联`(120), `中毅达`(120)；`ORG_CODE`: `10004106`(198), `10004127`(198), `10004293`(157), `10003964`(122), `10003963`(122), `10116535`(122), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(284265)；`REPORT_TYPE`: `年报`(75980), `中报`(71407), `一季报`(69943), `三季报`(66935)；`REPORT_DATE_NAME`: `2026一季报`(5099), `2025年报`(5084), `2025三季报`(5060), `2025中报`(5051), `2025一季报`(5042), `2024年报`(5020), `2024一季报`(5009), `2024三季报`(5005)；`SECURITY_TYPE_CODE`: `058001001`(284243), `058001008`(22)
- 数值范围摘要：`TOTAL_ASSETS` min=0.0, max=3613674212000.0, zero=6, negative=0, NULL=0；`TOTAL_EQUITY` min=-31171741000.0, max=1835768000000.0, zero=22, negative=3327, NULL=0；`TOTAL_LIABILITIES` min=-444400844.0, max=2775299325000.0, zero=271, negative=18, NULL=0；`ACCEPT_DEPOSIT_INTERBANK` min=0.0, max=777003282181.39, zero=282525, negative=0, NULL=0；`ACCOUNTS_PAYABLE` min=-44288812.27, max=997477873000.0, zero=9830, negative=20, NULL=0；`ACCOUNTS_RECE` min=-1878640045.72, max=442286697000.0, zero=7456, negative=24, NULL=0；`ACCRUED_EXPENSE` min=0.0, max=3512824010.83, zero=283532, negative=0, NULL=0；`ADVANCE_RECEIVABLES` min=-36425028.35, max=407882270730.4, zero=85079, negative=44, NULL=0；`AGENT_TRADE_SECURITY` min=0.0, max=181414551134.91, zero=283741, negative=0, NULL=0；`AGENT_UNDERWRITE_SECURITY` min=0.0, max=1703950000.0, zero=284216, negative=0, NULL=0
