# Raw 数据画像：eastmoney__balance

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__balance.yml`
- dbt source：`source('raw', 'eastmoney__balance')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__balance`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__balance --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__balance.md`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`eastmoney__balance`
- ClickHouse raw 表：`fleur_raw.eastmoney__balance`
- 表说明：EastMoney balance sheet F10 rows by natural-year raw partition.

## 2. 数据分析发现

基于当前 raw 表的现状分析：

- 数据量与覆盖
  - 总记录数：待补充
  - 覆盖主体数：待补充
  - 日期 / 分区范围：待补充
- 粒度与候选键
  - 观察到的粒度：待补充
  - 候选自然键去重结果：待补充
  - 旧候选键或备选键对比：待补充
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：待补充
  - 占位值：待补充
  - 预期缺失：待补充
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：待补充
  - 直接 raw input 参照命中情况：待补充
- 分布与相关性
  - 枚举 top values：待补充
  - 少量值 / 长尾文本：待补充
  - 字段间强相关：待补充
- 时间字段合理性
  - 日期范围：待补充
  - 日期先后关系异常：待补充
  - 批次时间范围：待补充
- 数值字段合理性
  - 负数 / 零值 / 极端值：待补充
  - 单位判断：待补充
- 其他观察
  - 对 staging 设计有影响、但不应在 staging 静默修正的事实：待补充

## 3. 粒度与键

- 观察到的粒度：待补充
- 候选自然键：待补充
- 重复检查：待补充
- 粒度注意事项：待补充

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| SECUCODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECUCODE`。 原始字段说明：证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECURITY_CODE`。 原始字段说明：证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECURITY_NAME_ABBR`。 原始字段说明：证券简称 |
| ORG_CODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ORG_CODE`。 原始字段说明：机构代码 |
| ORG_TYPE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ORG_TYPE`。 原始字段说明：机构类型 |
| REPORT_DATE | Date | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REPORT_DATE`。 原始字段说明：报告期 |
| REPORT_TYPE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REPORT_TYPE`。 原始字段说明：报告类型 |
| REPORT_DATE_NAME | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REPORT_DATE_NAME`。 原始字段说明：报告期名称 |
| SECURITY_TYPE_CODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECURITY_TYPE_CODE`。 原始字段说明：证券类型代码 |
| NOTICE_DATE | Date | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTICE_DATE`。 原始字段说明：公告日期 |
| UPDATE_DATE | Nullable(Date) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UPDATE_DATE`。 原始字段说明：更新日期 |
| CURRENCY | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENCY`。 原始字段说明：资产负债表披露金额使用的币种。 |
| ACCEPT_DEPOSIT_INTERBANK | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCEPT_DEPOSIT_INTERBANK`。 原始字段说明：同业存放 |
| ACCOUNTS_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCOUNTS_PAYABLE`。 原始字段说明：应付账款 |
| ACCOUNTS_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCOUNTS_RECE`。 原始字段说明：应收账款 |
| ACCRUED_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCRUED_EXPENSE`。 原始字段说明：预提费用 |
| ADVANCE_RECEIVABLES | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ADVANCE_RECEIVABLES`。 原始字段说明：预收款项 |
| AGENT_TRADE_SECURITY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AGENT_TRADE_SECURITY`。 原始字段说明：代理买卖证券款 |
| AGENT_UNDERWRITE_SECURITY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AGENT_UNDERWRITE_SECURITY`。 原始字段说明：代理承销证券款 |
| AMORTIZE_COST_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AMORTIZE_COST_FINASSET`。 原始字段说明：以摊余成本计量的金融资产 |
| AMORTIZE_COST_FINLIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AMORTIZE_COST_FINLIAB`。 原始字段说明：以摊余成本计量的金融负债 |
| AMORTIZE_COST_NCFINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AMORTIZE_COST_NCFINASSET`。 原始字段说明：非流动金融资产（摊余成本） |
| AMORTIZE_COST_NCFINLIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AMORTIZE_COST_NCFINLIAB`。 原始字段说明：非流动金融负债（摊余成本） |
| APPOINT_FVTPL_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `APPOINT_FVTPL_FINASSET`。 原始字段说明：指定为FVTPL的金融资产 |
| APPOINT_FVTPL_FINLIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `APPOINT_FVTPL_FINLIAB`。 原始字段说明：指定为FVTPL的金融负债 |
| ASSET_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_BALANCE`。 原始字段说明：资产平衡项 |
| ASSET_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_OTHER`。 原始字段说明：资产其他项 |
| ASSIGN_CASH_DIVIDEND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSIGN_CASH_DIVIDEND`。 原始字段说明：应付现金股利 |
| AVAILABLE_SALE_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AVAILABLE_SALE_FINASSET`。 原始字段说明：可供出售金融资产 |
| BOND_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BOND_PAYABLE`。 原始字段说明：应付债券 |
| BORROW_FUND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BORROW_FUND`。 原始字段说明：拆入资金 |
| BUY_RESALE_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BUY_RESALE_FINASSET`。 原始字段说明：买入返售金融资产 |
| CAPITAL_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CAPITAL_RESERVE`。 原始字段说明：资本公积 |
| CIP | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CIP`。 原始字段说明：在建工程 |
| CONSUMPTIVE_BIOLOGICAL_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONSUMPTIVE_BIOLOGICAL_ASSET`。 原始字段说明：消耗性生物资产 |
| CONTRACT_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTRACT_ASSET`。 原始字段说明：合同资产 |
| CONTRACT_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTRACT_LIAB`。 原始字段说明：合同负债 |
| CONVERT_DIFF | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_DIFF`。 原始字段说明：外币报表折算差额 |
| CREDITOR_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_INVEST`。 原始字段说明：债权投资 |
| CURRENT_ASSET_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENT_ASSET_BALANCE`。 原始字段说明：流动资产平衡项 |
| CURRENT_ASSET_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENT_ASSET_OTHER`。 原始字段说明：流动资产其他项 |
| CURRENT_LIAB_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENT_LIAB_BALANCE`。 原始字段说明：流动负债平衡项 |
| CURRENT_LIAB_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENT_LIAB_OTHER`。 原始字段说明：流动负债其他项 |
| DEFER_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_INCOME`。 原始字段说明：递延收益 |
| DEFER_INCOME_1YEAR | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_INCOME_1YEAR`。 原始字段说明：一年内到期递延收益 |
| DEFER_TAX_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_TAX_ASSET`。 原始字段说明：递延所得税资产 |
| DEFER_TAX_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_TAX_LIAB`。 原始字段说明：递延所得税负债 |
| DERIVE_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DERIVE_FINASSET`。 原始字段说明：衍生金融资产 |
| DERIVE_FINLIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DERIVE_FINLIAB`。 原始字段说明：衍生金融负债 |
| DEVELOP_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEVELOP_EXPENSE`。 原始字段说明：开发支出 |
| DIV_HOLDSALE_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIV_HOLDSALE_ASSET`。 原始字段说明：持有待售资产（除） |
| DIV_HOLDSALE_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIV_HOLDSALE_LIAB`。 原始字段说明：持有待售负债（除） |
| DIVIDEND_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIVIDEND_PAYABLE`。 原始字段说明：应付股利 |
| DIVIDEND_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIVIDEND_RECE`。 原始字段说明：应收股利 |
| EQUITY_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EQUITY_BALANCE`。 原始字段说明：所有者权益平衡项 |
| EQUITY_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EQUITY_OTHER`。 原始字段说明：所有者权益其他项 |
| EXPORT_REFUND_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EXPORT_REFUND_RECE`。 原始字段说明：应收出口退税 |
| FEE_COMMISSION_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_PAYABLE`。 原始字段说明：应付手续费及佣金 |
| FIN_FUND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FIN_FUND`。 原始字段说明：金融往来资金 |
| FINANCE_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_RECE`。 原始字段说明：金融应收款 |
| FIXED_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FIXED_ASSET`。 原始字段说明：固定资产 |
| FIXED_ASSET_DISPOSAL | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FIXED_ASSET_DISPOSAL`。 原始字段说明：固定资产清理 |
| FVTOCI_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FVTOCI_FINASSET`。 原始字段说明：以公允价值计量且其变动计入其他综合收益的金融资产 |
| FVTOCI_NCFINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FVTOCI_NCFINASSET`。 原始字段说明：其他非流动金融资产（FVTOCI） |
| FVTPL_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FVTPL_FINASSET`。 原始字段说明：以公允价值计量且其变动计入当期损益的金融资产 |
| FVTPL_FINLIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FVTPL_FINLIAB`。 原始字段说明：以公允价值计量且其变动计入当期损益的金融负债 |
| GENERAL_RISK_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `GENERAL_RISK_RESERVE`。 原始字段说明：一般风险准备 |
| GOODWILL | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `GOODWILL`。 原始字段说明：资产负债表披露的商誉金额。 |
| HOLD_MATURITY_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HOLD_MATURITY_INVEST`。 原始字段说明：持有至到期投资 |
| HOLDSALE_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HOLDSALE_ASSET`。 原始字段说明：持有待售资产 |
| HOLDSALE_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HOLDSALE_LIAB`。 原始字段说明：持有待售负债 |
| INSURANCE_CONTRACT_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INSURANCE_CONTRACT_RESERVE`。 原始字段说明：保险合同准备金 |
| INTANGIBLE_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTANGIBLE_ASSET`。 原始字段说明：无形资产 |
| INTEREST_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_PAYABLE`。 原始字段说明：应付利息 |
| INTEREST_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_RECE`。 原始字段说明：应收利息 |
| INTERNAL_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTERNAL_PAYABLE`。 原始字段说明：内部应付款 |
| INTERNAL_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTERNAL_RECE`。 原始字段说明：内部应收款 |
| INVENTORY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVENTORY`。 原始字段说明：资产负债表披露的存货金额。 |
| INVEST_REALESTATE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_REALESTATE`。 原始字段说明：投资性房地产 |
| LEASE_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LEASE_LIAB`。 原始字段说明：租赁负债 |
| LEND_FUND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LEND_FUND`。 原始字段说明：拆出资金 |
| LIAB_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIAB_BALANCE`。 原始字段说明：负债平衡项 |
| LIAB_EQUITY_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIAB_EQUITY_BALANCE`。 原始字段说明：负债和所有者权益平衡项 |
| LIAB_EQUITY_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIAB_EQUITY_OTHER`。 原始字段说明：负债和所有者权益其他项 |
| LIAB_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIAB_OTHER`。 原始字段说明：负债其他项 |
| LOAN_ADVANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_ADVANCE`。 原始字段说明：发放贷款及垫款 |
| LOAN_PBC | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_PBC`。 原始字段说明：向央行借款 |
| LONG_EQUITY_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_EQUITY_INVEST`。 原始字段说明：长期股权投资 |
| LONG_LOAN | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_LOAN`。 原始字段说明：长期借款 |
| LONG_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_PAYABLE`。 原始字段说明：长期应付款 |
| LONG_PREPAID_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_PREPAID_EXPENSE`。 原始字段说明：长期待摊费用 |
| LONG_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_RECE`。 原始字段说明：长期应收款 |
| LONG_STAFFSALARY_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_STAFFSALARY_PAYABLE`。 原始字段说明：长期应付职工薪酬 |
| MINORITY_EQUITY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_EQUITY`。 原始字段说明：少数股东权益 |
| MONETARYFUNDS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MONETARYFUNDS`。 原始字段说明：货币资金 |
| NONCURRENT_ASSET_1YEAR | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_ASSET_1YEAR`。 原始字段说明：一年内到期的非流动资产 |
| NONCURRENT_ASSET_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_ASSET_BALANCE`。 原始字段说明：非流动资产平衡项 |
| NONCURRENT_ASSET_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_ASSET_OTHER`。 原始字段说明：非流动资产其他项 |
| NONCURRENT_LIAB_1YEAR | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_LIAB_1YEAR`。 原始字段说明：一年内到期的非流动负债 |
| NONCURRENT_LIAB_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_LIAB_BALANCE`。 原始字段说明：非流动负债平衡项 |
| NONCURRENT_LIAB_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_LIAB_OTHER`。 原始字段说明：非流动负债其他项 |
| NOTE_ACCOUNTS_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTE_ACCOUNTS_PAYABLE`。 原始字段说明：应付票据及应付账款 |
| NOTE_ACCOUNTS_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTE_ACCOUNTS_RECE`。 原始字段说明：应收票据及应收账款 |
| NOTE_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTE_PAYABLE`。 原始字段说明：应付票据 |
| NOTE_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTE_RECE`。 原始字段说明：应收票据 |
| OIL_GAS_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OIL_GAS_ASSET`。 原始字段说明：油气资产 |
| OTHER_COMPRE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_COMPRE_INCOME`。 原始字段说明：其他综合收益 |
| OTHER_CREDITOR_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_CREDITOR_INVEST`。 原始字段说明：其他债权投资 |
| OTHER_CURRENT_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_CURRENT_ASSET`。 原始字段说明：其他流动资产 |
| OTHER_CURRENT_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_CURRENT_LIAB`。 原始字段说明：其他流动负债 |
| OTHER_EQUITY_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_EQUITY_INVEST`。 原始字段说明：其他权益工具投资 |
| OTHER_EQUITY_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_EQUITY_OTHER`。 原始字段说明：其他权益其他项 |
| OTHER_EQUITY_TOOL | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_EQUITY_TOOL`。 原始字段说明：其他权益工具 |
| OTHER_NONCURRENT_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_NONCURRENT_ASSET`。 原始字段说明：其他非流动资产 |
| OTHER_NONCURRENT_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_NONCURRENT_FINASSET`。 原始字段说明：其他非流动金融资产 |
| OTHER_NONCURRENT_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_NONCURRENT_LIAB`。 原始字段说明：其他非流动负债 |
| OTHER_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_PAYABLE`。 原始字段说明：其他应付款 |
| OTHER_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_RECE`。 原始字段说明：其他应收款 |
| PARENT_EQUITY_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_EQUITY_BALANCE`。 原始字段说明：归母权益平衡项 |
| PARENT_EQUITY_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_EQUITY_OTHER`。 原始字段说明：归母权益其他项 |
| PERPETUAL_BOND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PERPETUAL_BOND`。 原始字段说明：永续债 |
| PERPETUAL_BOND_PAYBALE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PERPETUAL_BOND_PAYBALE`。 原始字段说明：永续债（负债端） |
| PREDICT_CURRENT_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREDICT_CURRENT_LIAB`。 原始字段说明：预计流动负债 |
| PREDICT_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREDICT_LIAB`。 原始字段说明：预计负债 |
| PREFERRED_SHARES | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREFERRED_SHARES`。 原始字段说明：优先股 |
| PREFERRED_SHARES_PAYBALE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREFERRED_SHARES_PAYBALE`。 原始字段说明：应付优先股 |
| PREMIUM_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREMIUM_RECE`。 原始字段说明：预收保费 |
| PREPAYMENT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREPAYMENT`。 原始字段说明：预付款项 |
| PRODUCTIVE_BIOLOGY_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRODUCTIVE_BIOLOGY_ASSET`。 原始字段说明：生产性生物资产 |
| PROJECT_MATERIAL | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PROJECT_MATERIAL`。 原始字段说明：工程物资 |
| RC_RESERVE_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RC_RESERVE_RECE`。 原始字段说明：再保合同应收准备金 |
| REINSURE_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_PAYABLE`。 原始字段说明：应付再保款 |
| REINSURE_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_RECE`。 原始字段说明：应收再保款 |
| SELL_REPO_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SELL_REPO_FINASSET`。 原始字段说明：卖出回购金融资产款 |
| SETTLE_EXCESS_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SETTLE_EXCESS_RESERVE`。 原始字段说明：清算备付金 |
| SHARE_CAPITAL | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SHARE_CAPITAL`。 原始字段说明：实收资本（股本） |
| SHORT_BOND_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SHORT_BOND_PAYABLE`。 原始字段说明：短期应付债券 |
| SHORT_FIN_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SHORT_FIN_PAYABLE`。 原始字段说明：短期金融负债 |
| SHORT_LOAN | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SHORT_LOAN`。 原始字段说明：短期借款 |
| SPECIAL_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SPECIAL_PAYABLE`。 原始字段说明：专项应付款 |
| SPECIAL_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SPECIAL_RESERVE`。 原始字段说明：专项储备 |
| STAFF_SALARY_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `STAFF_SALARY_PAYABLE`。 原始字段说明：应付职工薪酬 |
| SUBSIDY_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SUBSIDY_RECE`。 原始字段说明：应收补贴 |
| SURPLUS_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SURPLUS_RESERVE`。 原始字段说明：盈余公积 |
| TAX_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TAX_PAYABLE`。 原始字段说明：应交税费 |
| TOTAL_ASSETS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_ASSETS`。 原始字段说明：资产总计 |
| TOTAL_CURRENT_ASSETS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_CURRENT_ASSETS`。 原始字段说明：流动资产合计 |
| TOTAL_CURRENT_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_CURRENT_LIAB`。 原始字段说明：流动负债合计 |
| TOTAL_EQUITY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_EQUITY`。 原始字段说明：所有者权益合计 |
| TOTAL_LIAB_EQUITY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_LIAB_EQUITY`。 原始字段说明：负债和所有者权益总计 |
| TOTAL_LIABILITIES | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_LIABILITIES`。 原始字段说明：负债合计 |
| TOTAL_NONCURRENT_ASSETS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_NONCURRENT_ASSETS`。 原始字段说明：非流动资产合计 |
| TOTAL_NONCURRENT_LIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_NONCURRENT_LIAB`。 原始字段说明：非流动负债合计 |
| TOTAL_OTHER_PAYABLE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OTHER_PAYABLE`。 原始字段说明：其他应付款合计 |
| TOTAL_OTHER_RECE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OTHER_RECE`。 原始字段说明：其他应收款合计 |
| TOTAL_PARENT_EQUITY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PARENT_EQUITY`。 原始字段说明：归属于母公司股东权益合计 |
| TRADE_FINASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TRADE_FINASSET`。 原始字段说明：交易性金融资产 |
| TRADE_FINASSET_NOTFVTPL | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TRADE_FINASSET_NOTFVTPL`。 原始字段说明：非FVTPL交易性金融资产 |
| TRADE_FINLIAB | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TRADE_FINLIAB`。 原始字段说明：交易性金融负债 |
| TRADE_FINLIAB_NOTFVTPL | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TRADE_FINLIAB_NOTFVTPL`。 原始字段说明：非FVTPL交易性金融负债 |
| TREASURY_SHARES | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TREASURY_SHARES`。 原始字段说明：库存股 |
| UNASSIGN_RPOFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNASSIGN_RPOFIT`。 原始字段说明：未分配利润 |
| UNCONFIRM_INVEST_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNCONFIRM_INVEST_LOSS`。 原始字段说明：未确认投资损失 |
| USERIGHT_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `USERIGHT_ASSET`。 原始字段说明：使用权资产 |
| ACCEPT_DEPOSIT_INTERBANK_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCEPT_DEPOSIT_INTERBANK_YOY`。 原始字段说明：同业存放同比增长率（%） |
| ACCOUNTS_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCOUNTS_PAYABLE_YOY`。 原始字段说明：应付账款同比增长率（%） |
| ACCOUNTS_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCOUNTS_RECE_YOY`。 原始字段说明：应收账款同比增长率（%） |
| ACCRUED_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCRUED_EXPENSE_YOY`。 原始字段说明：预提费用同比增长率（%） |
| ADVANCE_RECEIVABLES_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ADVANCE_RECEIVABLES_YOY`。 原始字段说明：预收款项同比增长率（%） |
| AGENT_TRADE_SECURITY_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AGENT_TRADE_SECURITY_YOY`。 原始字段说明：代理买卖证券款同比增长率（%） |
| AGENT_UNDERWRITE_SECURITY_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AGENT_UNDERWRITE_SECURITY_YOY`。 原始字段说明：代理承销证券款同比增长率（%） |
| AMORTIZE_COST_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AMORTIZE_COST_FINASSET_YOY`。 原始字段说明：以摊余成本计量的金融资产同比增长率（%） |
| AMORTIZE_COST_FINLIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AMORTIZE_COST_FINLIAB_YOY`。 原始字段说明：以摊余成本计量的金融负债同比增长率（%） |
| AMORTIZE_COST_NCFINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AMORTIZE_COST_NCFINASSET_YOY`。 原始字段说明：非流动金融资产（摊余成本）同比增长率（%） |
| AMORTIZE_COST_NCFINLIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AMORTIZE_COST_NCFINLIAB_YOY`。 原始字段说明：非流动金融负债（摊余成本）同比增长率（%） |
| APPOINT_FVTPL_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `APPOINT_FVTPL_FINASSET_YOY`。 原始字段说明：指定为FVTPL的金融资产同比增长率（%） |
| APPOINT_FVTPL_FINLIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `APPOINT_FVTPL_FINLIAB_YOY`。 原始字段说明：指定为FVTPL的金融负债同比增长率（%） |
| ASSET_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_BALANCE_YOY`。 原始字段说明：资产平衡项同比增长率（%） |
| ASSET_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_OTHER_YOY`。 原始字段说明：资产其他项同比增长率（%） |
| ASSIGN_CASH_DIVIDEND_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSIGN_CASH_DIVIDEND_YOY`。 原始字段说明：应付现金股利同比增长率（%） |
| AVAILABLE_SALE_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AVAILABLE_SALE_FINASSET_YOY`。 原始字段说明：可供出售金融资产同比增长率（%） |
| BOND_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BOND_PAYABLE_YOY`。 原始字段说明：应付债券同比增长率（%） |
| BORROW_FUND_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BORROW_FUND_YOY`。 原始字段说明：拆入资金同比增长率（%） |
| BUY_RESALE_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BUY_RESALE_FINASSET_YOY`。 原始字段说明：买入返售金融资产同比增长率（%） |
| CAPITAL_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CAPITAL_RESERVE_YOY`。 原始字段说明：资本公积同比增长率（%） |
| CIP_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CIP_YOY`。 原始字段说明：在建工程同比增长率（%） |
| CONSUMPTIVE_BIOLOGICAL_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONSUMPTIVE_BIOLOGICAL_ASSET_YOY`。 原始字段说明：消耗性生物资产同比增长率（%） |
| CONTRACT_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTRACT_ASSET_YOY`。 原始字段说明：合同资产同比增长率（%） |
| CONTRACT_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTRACT_LIAB_YOY`。 原始字段说明：合同负债同比增长率（%） |
| CONVERT_DIFF_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_DIFF_YOY`。 原始字段说明：外币报表折算差额同比增长率（%） |
| CREDITOR_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_INVEST_YOY`。 原始字段说明：债权投资同比增长率（%） |
| CURRENT_ASSET_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENT_ASSET_BALANCE_YOY`。 原始字段说明：流动资产平衡项同比增长率（%） |
| CURRENT_ASSET_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENT_ASSET_OTHER_YOY`。 原始字段说明：流动资产其他项同比增长率（%） |
| CURRENT_LIAB_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENT_LIAB_BALANCE_YOY`。 原始字段说明：流动负债平衡项同比增长率（%） |
| CURRENT_LIAB_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENT_LIAB_OTHER_YOY`。 原始字段说明：流动负债其他项同比增长率（%） |
| DEFER_INCOME_1YEAR_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_INCOME_1YEAR_YOY`。 原始字段说明：一年内到期递延收益同比增长率（%） |
| DEFER_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_INCOME_YOY`。 原始字段说明：递延收益同比增长率（%） |
| DEFER_TAX_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_TAX_ASSET_YOY`。 原始字段说明：递延所得税资产同比增长率（%） |
| DEFER_TAX_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_TAX_LIAB_YOY`。 原始字段说明：递延所得税负债同比增长率（%） |
| DERIVE_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DERIVE_FINASSET_YOY`。 原始字段说明：衍生金融资产同比增长率（%） |
| DERIVE_FINLIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DERIVE_FINLIAB_YOY`。 原始字段说明：衍生金融负债同比增长率（%） |
| DEVELOP_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEVELOP_EXPENSE_YOY`。 原始字段说明：开发支出同比增长率（%） |
| DIV_HOLDSALE_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIV_HOLDSALE_ASSET_YOY`。 原始字段说明：持有待售资产（除）同比增长率（%） |
| DIV_HOLDSALE_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIV_HOLDSALE_LIAB_YOY`。 原始字段说明：持有待售负债（除）同比增长率（%） |
| DIVIDEND_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIVIDEND_PAYABLE_YOY`。 原始字段说明：应付股利同比增长率（%） |
| DIVIDEND_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIVIDEND_RECE_YOY`。 原始字段说明：应收股利同比增长率（%） |
| EQUITY_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EQUITY_BALANCE_YOY`。 原始字段说明：所有者权益平衡项同比增长率（%） |
| EQUITY_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EQUITY_OTHER_YOY`。 原始字段说明：所有者权益其他项同比增长率（%） |
| EXPORT_REFUND_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EXPORT_REFUND_RECE_YOY`。 原始字段说明：应收出口退税同比增长率（%） |
| FEE_COMMISSION_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_PAYABLE_YOY`。 原始字段说明：应付手续费及佣金同比增长率（%） |
| FIN_FUND_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FIN_FUND_YOY`。 原始字段说明：金融往来资金同比增长率（%） |
| FINANCE_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_RECE_YOY`。 原始字段说明：金融应收款同比增长率（%） |
| FIXED_ASSET_DISPOSAL_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FIXED_ASSET_DISPOSAL_YOY`。 原始字段说明：固定资产清理同比增长率（%） |
| FIXED_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FIXED_ASSET_YOY`。 原始字段说明：固定资产同比增长率（%） |
| FVTOCI_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FVTOCI_FINASSET_YOY`。 原始字段说明：以公允价值计量且其变动计入其他综合收益的金融资产同比增长率（%） |
| FVTOCI_NCFINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FVTOCI_NCFINASSET_YOY`。 原始字段说明：其他非流动金融资产（FVTOCI）同比增长率（%） |
| FVTPL_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FVTPL_FINASSET_YOY`。 原始字段说明：以公允价值计量且其变动计入当期损益的金融资产同比增长率（%） |
| FVTPL_FINLIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FVTPL_FINLIAB_YOY`。 原始字段说明：以公允价值计量且其变动计入当期损益的金融负债同比增长率（%） |
| GENERAL_RISK_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `GENERAL_RISK_RESERVE_YOY`。 原始字段说明：一般风险准备同比增长率（%） |
| GOODWILL_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `GOODWILL_YOY`。 原始字段说明：商誉同比增长率（%） |
| HOLD_MATURITY_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HOLD_MATURITY_INVEST_YOY`。 原始字段说明：持有至到期投资同比增长率（%） |
| HOLDSALE_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HOLDSALE_ASSET_YOY`。 原始字段说明：持有待售资产同比增长率（%） |
| HOLDSALE_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HOLDSALE_LIAB_YOY`。 原始字段说明：持有待售负债同比增长率（%） |
| INSURANCE_CONTRACT_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INSURANCE_CONTRACT_RESERVE_YOY`。 原始字段说明：保险合同准备金同比增长率（%） |
| INTANGIBLE_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTANGIBLE_ASSET_YOY`。 原始字段说明：无形资产同比增长率（%） |
| INTEREST_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_PAYABLE_YOY`。 原始字段说明：应付利息同比增长率（%） |
| INTEREST_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_RECE_YOY`。 原始字段说明：应收利息同比增长率（%） |
| INTERNAL_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTERNAL_PAYABLE_YOY`。 原始字段说明：内部应付款同比增长率（%） |
| INTERNAL_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTERNAL_RECE_YOY`。 原始字段说明：内部应收款同比增长率（%） |
| INVENTORY_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVENTORY_YOY`。 原始字段说明：存货同比增长率（%） |
| INVEST_REALESTATE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_REALESTATE_YOY`。 原始字段说明：投资性房地产同比增长率（%） |
| LEASE_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LEASE_LIAB_YOY`。 原始字段说明：租赁负债同比增长率（%） |
| LEND_FUND_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LEND_FUND_YOY`。 原始字段说明：拆出资金同比增长率（%） |
| LIAB_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIAB_BALANCE_YOY`。 原始字段说明：负债平衡项同比增长率（%） |
| LIAB_EQUITY_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIAB_EQUITY_BALANCE_YOY`。 原始字段说明：负债和所有者权益平衡项同比增长率（%） |
| LIAB_EQUITY_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIAB_EQUITY_OTHER_YOY`。 原始字段说明：负债和所有者权益其他项同比增长率（%） |
| LIAB_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LIAB_OTHER_YOY`。 原始字段说明：负债其他项同比增长率（%） |
| LOAN_ADVANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_ADVANCE_YOY`。 原始字段说明：发放贷款及垫款同比增长率（%） |
| LOAN_PBC_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_PBC_YOY`。 原始字段说明：向央行借款同比增长率（%） |
| LONG_EQUITY_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_EQUITY_INVEST_YOY`。 原始字段说明：长期股权投资同比增长率（%） |
| LONG_LOAN_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_LOAN_YOY`。 原始字段说明：长期借款同比增长率（%） |
| LONG_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_PAYABLE_YOY`。 原始字段说明：长期应付款同比增长率（%） |
| LONG_PREPAID_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_PREPAID_EXPENSE_YOY`。 原始字段说明：长期待摊费用同比增长率（%） |
| LONG_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_RECE_YOY`。 原始字段说明：长期应收款同比增长率（%） |
| LONG_STAFFSALARY_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LONG_STAFFSALARY_PAYABLE_YOY`。 原始字段说明：长期应付职工薪酬同比增长率（%） |
| MINORITY_EQUITY_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_EQUITY_YOY`。 原始字段说明：少数股东权益同比增长率（%） |
| MONETARYFUNDS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MONETARYFUNDS_YOY`。 原始字段说明：货币资金同比增长率（%） |
| NONCURRENT_ASSET_1YEAR_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_ASSET_1YEAR_YOY`。 原始字段说明：一年内到期的非流动资产同比增长率（%） |
| NONCURRENT_ASSET_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_ASSET_BALANCE_YOY`。 原始字段说明：非流动资产平衡项同比增长率（%） |
| NONCURRENT_ASSET_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_ASSET_OTHER_YOY`。 原始字段说明：非流动资产其他项同比增长率（%） |
| NONCURRENT_LIAB_1YEAR_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_LIAB_1YEAR_YOY`。 原始字段说明：一年内到期的非流动负债同比增长率（%） |
| NONCURRENT_LIAB_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_LIAB_BALANCE_YOY`。 原始字段说明：非流动负债平衡项同比增长率（%） |
| NONCURRENT_LIAB_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_LIAB_OTHER_YOY`。 原始字段说明：非流动负债其他项同比增长率（%） |
| NOTE_ACCOUNTS_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTE_ACCOUNTS_PAYABLE_YOY`。 原始字段说明：应付票据及应付账款同比增长率（%） |
| NOTE_ACCOUNTS_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTE_ACCOUNTS_RECE_YOY`。 原始字段说明：应收票据及应收账款同比增长率（%） |
| NOTE_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTE_PAYABLE_YOY`。 原始字段说明：应付票据同比增长率（%） |
| NOTE_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTE_RECE_YOY`。 原始字段说明：应收票据同比增长率（%） |
| OIL_GAS_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OIL_GAS_ASSET_YOY`。 原始字段说明：油气资产同比增长率（%） |
| OTHER_COMPRE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_COMPRE_INCOME_YOY`。 原始字段说明：其他综合收益总额同比增长率（%） |
| OTHER_CREDITOR_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_CREDITOR_INVEST_YOY`。 原始字段说明：其他债权投资同比增长率（%） |
| OTHER_CURRENT_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_CURRENT_ASSET_YOY`。 原始字段说明：其他流动资产同比增长率（%） |
| OTHER_CURRENT_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_CURRENT_LIAB_YOY`。 原始字段说明：其他流动负债同比增长率（%） |
| OTHER_EQUITY_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_EQUITY_INVEST_YOY`。 原始字段说明：其他权益工具投资同比增长率（%） |
| OTHER_EQUITY_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_EQUITY_OTHER_YOY`。 原始字段说明：其他权益其他项同比增长率（%） |
| OTHER_EQUITY_TOOL_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_EQUITY_TOOL_YOY`。 原始字段说明：其他权益工具同比增长率（%） |
| OTHER_NONCURRENT_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_NONCURRENT_ASSET_YOY`。 原始字段说明：其他非流动资产同比增长率（%） |
| OTHER_NONCURRENT_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_NONCURRENT_FINASSET_YOY`。 原始字段说明：其他非流动金融资产同比增长率（%） |
| OTHER_NONCURRENT_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_NONCURRENT_LIAB_YOY`。 原始字段说明：其他非流动负债同比增长率（%） |
| OTHER_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_PAYABLE_YOY`。 原始字段说明：其他应付款同比增长率（%） |
| OTHER_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_RECE_YOY`。 原始字段说明：其他应收款同比增长率（%） |
| PARENT_EQUITY_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_EQUITY_BALANCE_YOY`。 原始字段说明：归母权益平衡项同比增长率（%） |
| PARENT_EQUITY_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_EQUITY_OTHER_YOY`。 原始字段说明：归母权益其他项同比增长率（%） |
| PERPETUAL_BOND_PAYBALE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PERPETUAL_BOND_PAYBALE_YOY`。 原始字段说明：永续债（负债端）同比增长率（%） |
| PERPETUAL_BOND_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PERPETUAL_BOND_YOY`。 原始字段说明：永续债同比增长率（%） |
| PREDICT_CURRENT_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREDICT_CURRENT_LIAB_YOY`。 原始字段说明：预计流动负债同比增长率（%） |
| PREDICT_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREDICT_LIAB_YOY`。 原始字段说明：预计负债同比增长率（%） |
| PREFERRED_SHARES_PAYBALE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREFERRED_SHARES_PAYBALE_YOY`。 原始字段说明：应付优先股同比增长率（%） |
| PREFERRED_SHARES_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREFERRED_SHARES_YOY`。 原始字段说明：优先股同比增长率（%） |
| PREMIUM_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREMIUM_RECE_YOY`。 原始字段说明：预收保费同比增长率（%） |
| PREPAYMENT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREPAYMENT_YOY`。 原始字段说明：预付款项同比增长率（%） |
| PRODUCTIVE_BIOLOGY_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRODUCTIVE_BIOLOGY_ASSET_YOY`。 原始字段说明：生产性生物资产同比增长率（%） |
| PROJECT_MATERIAL_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PROJECT_MATERIAL_YOY`。 原始字段说明：工程物资同比增长率（%） |
| RC_RESERVE_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RC_RESERVE_RECE_YOY`。 原始字段说明：再保合同应收准备金同比增长率（%） |
| REINSURE_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_PAYABLE_YOY`。 原始字段说明：应付再保款同比增长率（%） |
| REINSURE_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_RECE_YOY`。 原始字段说明：应收再保款同比增长率（%） |
| SELL_REPO_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SELL_REPO_FINASSET_YOY`。 原始字段说明：卖出回购金融资产款同比增长率（%） |
| SETTLE_EXCESS_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SETTLE_EXCESS_RESERVE_YOY`。 原始字段说明：清算备付金同比增长率（%） |
| SHARE_CAPITAL_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SHARE_CAPITAL_YOY`。 原始字段说明：实收资本（股本）同比增长率（%） |
| SHORT_BOND_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SHORT_BOND_PAYABLE_YOY`。 原始字段说明：短期应付债券同比增长率（%） |
| SHORT_FIN_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SHORT_FIN_PAYABLE_YOY`。 原始字段说明：短期金融负债同比增长率（%） |
| SHORT_LOAN_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SHORT_LOAN_YOY`。 原始字段说明：短期借款同比增长率（%） |
| SPECIAL_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SPECIAL_PAYABLE_YOY`。 原始字段说明：专项应付款同比增长率（%） |
| SPECIAL_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SPECIAL_RESERVE_YOY`。 原始字段说明：专项储备同比增长率（%） |
| STAFF_SALARY_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `STAFF_SALARY_PAYABLE_YOY`。 原始字段说明：应付职工薪酬同比增长率（%） |
| SUBSIDY_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SUBSIDY_RECE_YOY`。 原始字段说明：应收补贴同比增长率（%） |
| SURPLUS_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SURPLUS_RESERVE_YOY`。 原始字段说明：盈余公积同比增长率（%） |
| TAX_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TAX_PAYABLE_YOY`。 原始字段说明：应交税费同比增长率（%） |
| TOTAL_ASSETS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_ASSETS_YOY`。 原始字段说明：资产总计同比增长率（%） |
| TOTAL_CURRENT_ASSETS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_CURRENT_ASSETS_YOY`。 原始字段说明：流动资产合计同比增长率（%） |
| TOTAL_CURRENT_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_CURRENT_LIAB_YOY`。 原始字段说明：流动负债合计同比增长率（%） |
| TOTAL_EQUITY_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_EQUITY_YOY`。 原始字段说明：所有者权益合计同比增长率（%） |
| TOTAL_LIAB_EQUITY_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_LIAB_EQUITY_YOY`。 原始字段说明：负债和所有者权益总计同比增长率（%） |
| TOTAL_LIABILITIES_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_LIABILITIES_YOY`。 原始字段说明：负债合计同比增长率（%） |
| TOTAL_NONCURRENT_ASSETS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_NONCURRENT_ASSETS_YOY`。 原始字段说明：非流动资产合计同比增长率（%） |
| TOTAL_NONCURRENT_LIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_NONCURRENT_LIAB_YOY`。 原始字段说明：非流动负债合计同比增长率（%） |
| TOTAL_OTHER_PAYABLE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OTHER_PAYABLE_YOY`。 原始字段说明：其他应付款合计同比增长率（%） |
| TOTAL_OTHER_RECE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OTHER_RECE_YOY`。 原始字段说明：其他应收款合计同比增长率（%） |
| TOTAL_PARENT_EQUITY_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PARENT_EQUITY_YOY`。 原始字段说明：归属于母公司股东权益合计同比增长率（%） |
| TRADE_FINASSET_NOTFVTPL_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TRADE_FINASSET_NOTFVTPL_YOY`。 原始字段说明：非FVTPL交易性金融资产同比增长率（%） |
| TRADE_FINASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TRADE_FINASSET_YOY`。 原始字段说明：交易性金融资产同比增长率（%） |
| TRADE_FINLIAB_NOTFVTPL_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TRADE_FINLIAB_NOTFVTPL_YOY`。 原始字段说明：非FVTPL交易性金融负债同比增长率（%） |
| TRADE_FINLIAB_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TRADE_FINLIAB_YOY`。 原始字段说明：交易性金融负债同比增长率（%） |
| TREASURY_SHARES_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TREASURY_SHARES_YOY`。 原始字段说明：库存股同比增长率（%） |
| UNASSIGN_RPOFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNASSIGN_RPOFIT_YOY`。 原始字段说明：未分配利润同比增长率（%） |
| UNCONFIRM_INVEST_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNCONFIRM_INVEST_LOSS_YOY`。 原始字段说明：未确认投资损失同比增长率（%） |
| USERIGHT_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `USERIGHT_ASSET_YOY`。 原始字段说明：使用权资产同比增长率（%） |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPINION_TYPE`。 原始字段说明：审计意见类型 |
| OSOPINION_TYPE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OSOPINION_TYPE`。 原始字段说明：内控审计意见类型 |
| LISTING_STATE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LISTING_STATE`。 原始字段说明：上市状态 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `ORG_CODE`, `SECURITY_TYPE_CODE`
- 观察到的格式：待补充
- 无效样例：待补充
- 建议 staging 处理：待补充

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `REPORT_DATE_NAME`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：待补充
- 无效值或占位值：待补充
- 建议 staging 处理：待补充

### 枚举字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`
- 取值：待补充
- 未知或异常取值：待补充
- 建议 staging 处理：待补充

### 数值字段

- 已画像字段：`ACCEPT_DEPOSIT_INTERBANK`, `ACCOUNTS_PAYABLE`, `ACCOUNTS_RECE`, `ACCRUED_EXPENSE`, `ADVANCE_RECEIVABLES`, `AGENT_TRADE_SECURITY`, `AGENT_UNDERWRITE_SECURITY`, `AMORTIZE_COST_FINASSET`
- 最小/最大值：待补充
- 负数/零值/极端值：待补充
- 单位假设：待补充
- 建议 staging 处理：待补充

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 待补充 | 待补充 | 待补充 | 待补充 | 待补充 |

## 7. Staging 设计决策

- 重命名：待补充
- 类型转换：待补充
- 标准化：待补充
- NULL 处理：待补充
- 测试：待补充
- YAML 元数据：待补充

## 8. 延后到 Intermediate/Mart

- 跨源 join：待补充
- 需要优先级判断的去重：待补充
- 主数据修正：待补充
- 粒度变化：待补充
- 业务指标逻辑：待补充

## 待确认问题

- [ ] 确认画像发现，并在依赖该报告开展新 staging 工作前更新报告状态。

## 关键 SQL 证据摘要

- 行数：待补充
- 日期 / 分区范围：待补充
- 候选键重复：待补充
- 关键 NULL / 占位值：待补充
- 枚举 / 文本分布：待补充
- 数值范围：待补充

## 9. 验收清单

- [ ] 已抽样 raw source。
- [ ] 已记录行数和日期/分区范围。
- [ ] 已评估粒度和候选键。
- [ ] 已完成关键字段画像。
- [ ] 已列出 staging 转换建议。
- [ ] 已列出延后处理事项。
- [ ] 已提出测试或明确豁免。

## Profiling SQL 与结果

### 样例行

```sql
select *
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:36:11  Running with dbt=1.11.11
21:36:12  Registered adapter: clickhouse=1.10.0
21:36:12  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:36:13  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:13
21:36:13  Concurrency: 1 threads (target='dev')
21:36:13
Previewing inline node:
| SECUCODE  | SECURITY_CODE | SECURITY_NAME_ABBR | ORG_CODE | ORG_TYPE | REPORT_DATE | ... |
| --------- | ------------- | ------------------ | -------- | -------- | ----------- | --- |
| 600601.SH | 600601        | 方正科技               | 10002659 | 通用       |  1990-06-30 | ... |
| 600601.SH | 600601        | 方正科技               | 10002659 | 通用       |  1991-06-30 | ... |
| 600602.SH | 600602        | 云赛智联               | 10002660 | 通用       |  1990-06-30 | ... |
| 600602.SH | 600602        | 云赛智联               | 10002660 | 通用       |  1991-06-30 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1990-06-30 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1991-06-30 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1989-12-31 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1990-06-30 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1991-06-30 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1989-12-31 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1990-06-30 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1991-06-30 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1989-12-31 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1990-06-30 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1991-06-30 | ... |
| 600656.SH | 600656        | 退市博元               | 10003966 | 通用       |  1989-12-31 | ... |
| 600656.SH | 600656        | 退市博元               | 10003966 | 通用       |  1990-06-30 | ... |
| 600656.SH | 600656        | 退市博元               | 10003966 | 通用       |  1991-06-30 | ... |
| 000501.SZ | 000501        | 武商集团               | 10004338 | 通用       |  1989-12-31 | ... |
| 000501.SZ | 000501        | 武商集团               | 10004338 | 通用       |  1990-12-31 | ... |
| 600601.SH | 600601        | 方正科技               | 10002659 | 通用       |  1990-12-31 | ... |
| 600601.SH | 600601        | 方正科技               | 10002659 | 通用       |  1991-12-31 | ... |
| 600601.SH | 600601        | 方正科技               | 10002659 | 通用       |  1992-06-30 | ... |
| 600602.SH | 600602        | 云赛智联               | 10002660 | 通用       |  1992-06-30 | ... |
| 600603.SH | 600603        | 广汇物流               | 10002661 | 通用       |  1990-12-31 | ... |
| 600603.SH | 600603        | 广汇物流               | 10002661 | 通用       |  1991-12-31 | ... |
| 600604.SH | 600604        | 市北高新               | 10002662 | 通用       |  1992-06-30 | ... |
| 600605.SH | 600605        | 汇通能源               | 10002663 | 通用       |  1992-06-30 | ... |
| 600606.SH | 600606        | 绿地控股               | 10002664 | 通用       |  1992-06-30 | ... |
| 600608.SH | 600608        | *ST沪科              | 10002666 | 通用       |  1992-06-30 | ... |
| 600614.SH | 600614        | 退市鹏起               | 10003924 | 通用       |  1990-12-31 | ... |
| 600614.SH | 600614        | 退市鹏起               | 10003924 | 通用       |  1991-12-31 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1990-12-31 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1991-12-31 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1992-06-30 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1990-12-31 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1991-12-31 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1992-06-30 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1990-12-31 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1991-12-31 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1992-06-30 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1990-12-31 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1991-12-31 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1992-06-30 | ... |
| 600656.SH | 600656        | 退市博元               | 10003966 | 通用       |  1990-12-31 | ... |
| 600656.SH | 600656        | 退市博元               | 10003966 | 通用       |  1991-12-31 | ... |
| 600656.SH | 600656        | 退市博元               | 10003966 | 通用       |  1992-06-30 | ... |
| 000003.SZ | 000003        | PT金田A              | 10004087 | 通用       |  1991-12-31 | ... |
| 000003.SZ | 000003        | PT金田A              | 10004087 | 通用       |  1992-12-31 | ... |
| 000004.SZ | 000004        | *ST国华              | 10004088 | 通用       |  1992-12-31 | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:36:16  Running with dbt=1.11.11
21:36:16  Registered adapter: clickhouse=1.10.0
21:36:17  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:36:17  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:17
21:36:17  Concurrency: 1 threads (target='dev')
21:36:17
Previewing inline node:
| row_count |
| --------- |
|    284265 |
```

### 日期范围

```sql
select
    min(`REPORT_DATE`) as min_report_date,
    max(`REPORT_DATE`) as max_report_date,
    countIf(isNull(`REPORT_DATE`)) as null_report_date,
    countIf(`REPORT_DATE` = toDate('1970-01-01')) as placeholder_report_date,
    min(`REPORT_DATE_NAME`) as min_report_date_name,
    max(`REPORT_DATE_NAME`) as max_report_date_name,
    countIf(isNull(`REPORT_DATE_NAME`)) as null_report_date_name,
    countIf(toString(`REPORT_DATE_NAME`) = '1970-01-01') as placeholder_report_date_name,
    min(`NOTICE_DATE`) as min_notice_date,
    max(`NOTICE_DATE`) as max_notice_date,
    countIf(isNull(`NOTICE_DATE`)) as null_notice_date,
    countIf(`NOTICE_DATE` = toDate('1970-01-01')) as placeholder_notice_date,
    min(`UPDATE_DATE`) as min_update_date,
    max(`UPDATE_DATE`) as max_update_date,
    countIf(isNull(`UPDATE_DATE`)) as null_update_date,
    countIf(`UPDATE_DATE` = toDate('1970-01-01')) as placeholder_update_date
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:36:20  Running with dbt=1.11.11
21:36:21  Registered adapter: clickhouse=1.10.0
21:36:21  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:36:21  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:21
21:36:21  Concurrency: 1 threads (target='dev')
21:36:21
Previewing inline node:
| min_report_date | max_report_date | null_report_date | placeholder_repor... | min_report_date_name | max_report_date_name | ... |
| --------------- | --------------- | ---------------- | -------------------- | -------------------- | -------------------- | --- |
|      1989-12-31 |      2026-03-31 |                0 |                    0 | 1989年报               | 2026一季报              | ... |
```

### 格式分布：SECUCODE

```sql
select
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:36:25  Running with dbt=1.11.11
21:36:25  Registered adapter: clickhouse=1.10.0
21:36:26  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:36:26  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:26
21:36:26  Concurrency: 1 threads (target='dev')
21:36:26
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|           284265 |             0 |            0 |             0 |    284265 |
```

### 格式分布：SECURITY_CODE

```sql
select
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:36:30  Running with dbt=1.11.11
21:36:30  Registered adapter: clickhouse=1.10.0
21:36:30  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:36:31  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:31
21:36:31  Concurrency: 1 threads (target='dev')
21:36:31
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |       284265 |             0 |    284265 |
```

### 格式分布：ORG_CODE

```sql
select
    countIf(match(toString(`ORG_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`ORG_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`ORG_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`ORG_CODE`) or toString(`ORG_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:36:34  Running with dbt=1.11.11
21:36:34  Registered adapter: clickhouse=1.10.0
21:36:35  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:36:35  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:35
21:36:35  Concurrency: 1 threads (target='dev')
21:36:35
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |    284265 |
```

### 格式分布：SECURITY_TYPE_CODE

```sql
select
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_TYPE_CODE`) or toString(`SECURITY_TYPE_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:36:39  Running with dbt=1.11.11
21:36:39  Registered adapter: clickhouse=1.10.0
21:36:39  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:36:40  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:40
21:36:40  Concurrency: 1 threads (target='dev')
21:36:40
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |    284265 |
```

### 高频取值：SECUCODE

```sql
select
    `SECUCODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
group by `SECUCODE`
order by row_count desc
```


结果（成功）：

```text
21:36:43  Running with dbt=1.11.11
21:36:43  Registered adapter: clickhouse=1.10.0
21:36:44  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:36:44  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:44
21:36:44  Concurrency: 1 threads (target='dev')
21:36:44
Previewing inline node:
| value     | row_count |
| --------- | --------- |
| 600654.SH |       122 |
| 600653.SH |       122 |
| 600651.SH |       121 |
| 600601.SH |       121 |
| 600602.SH |       120 |
| 600610.SH |       120 |
| 600608.SH |       118 |
| 000030.SZ |       118 |
| 600603.SH |       118 |
| 000007.SZ |       118 |
| 600605.SH |       118 |
| 000501.SZ |       118 |
| 600633.SH |       117 |
| 600604.SH |       117 |
| 000009.SZ |       117 |
| 600661.SH |       117 |
| 600629.SH |       117 |
| 600606.SH |       117 |
| 600822.SH |       117 |
| 000025.SZ |       117 |
```

### 高频取值：SECURITY_CODE

```sql
select
    `SECURITY_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
group by `SECURITY_CODE`
order by row_count desc
```


结果（成功）：

```text
21:36:48  Running with dbt=1.11.11
21:36:48  Registered adapter: clickhouse=1.10.0
21:36:48  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:36:49  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:49
21:36:49  Concurrency: 1 threads (target='dev')
21:36:49
Previewing inline node:
| value  | row_count |
| ------ | --------- |
| 600654 |       122 |
| 600653 |       122 |
| 600601 |       121 |
| 600651 |       121 |
| 600610 |       120 |
| 600602 |       120 |
| 000007 |       118 |
| 600605 |       118 |
| 600603 |       118 |
| 000030 |       118 |
| 600608 |       118 |
| 000501 |       118 |
| 600642 |       117 |
| 000553 |       117 |
| 600822 |       117 |
| 600629 |       117 |
| 000008 |       117 |
| 000025 |       117 |
| 600606 |       117 |
| 000006 |       117 |
```

### 高频取值：SECURITY_NAME_ABBR

```sql
select
    `SECURITY_NAME_ABBR` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
group by `SECURITY_NAME_ABBR`
order by row_count desc
```


结果（成功）：

```text
21:36:52  Running with dbt=1.11.11
21:36:52  Registered adapter: clickhouse=1.10.0
21:36:52  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:36:53  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:53
21:36:53  Concurrency: 1 threads (target='dev')
21:36:53
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 东方明珠  |       184 |
| 百联股份  |       171 |
| 申华控股  |       122 |
| 中安科   |       122 |
| 方正科技  |       121 |
| 飞乐音响  |       121 |
| 中毅达   |       120 |
| 云赛智联  |       120 |
| *ST沪科 |       118 |
| 武商集团  |       118 |
| 富奥股份  |       118 |
| 汇通能源  |       118 |
| 广汇物流  |       118 |
| 全新好   |       118 |
| 新金路   |       117 |
| 安道麦A  |       117 |
| 浙数文化  |       117 |
| 深物业A  |       117 |
| ST复华  |       117 |
| 特力A   |       117 |
```

### 高频取值：ORG_CODE

```sql
select
    `ORG_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
group by `ORG_CODE`
order by row_count desc
```


结果（成功）：

```text
21:36:56  Running with dbt=1.11.11
21:36:57  Registered adapter: clickhouse=1.10.0
21:36:57  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:36:57  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:36:57
21:36:57  Concurrency: 1 threads (target='dev')
21:36:57
Previewing inline node:
| value    | row_count |
| -------- | --------- |
| 10004127 |       198 |
| 10004106 |       198 |
| 10004293 |       157 |
| 10003964 |       122 |
| 10116535 |       122 |
| 10003963 |       122 |
| 10002659 |       121 |
| 10003961 |       121 |
| 10002668 |       120 |
| 10002660 |       120 |
| 10002663 |       118 |
| 10002666 |       118 |
| 10002661 |       118 |
| 10004338 |       118 |
| 10004091 |       118 |
| 10634796 |       118 |
| 10002662 |       117 |
| 10004090 |       117 |
| 10005456 |       117 |
| 10004093 |       117 |
```

### 高频取值：ORG_TYPE

```sql
select
    `ORG_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
group by `ORG_TYPE`
order by row_count desc
```


结果（成功）：

```text
21:37:01  Running with dbt=1.11.11
21:37:01  Registered adapter: clickhouse=1.10.0
21:37:02  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:37:02  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:02
21:37:02  Concurrency: 1 threads (target='dev')
21:37:02
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 通用    |    284265 |
```

### 高频取值：REPORT_TYPE

```sql
select
    `REPORT_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
group by `REPORT_TYPE`
order by row_count desc
```


结果（成功）：

```text
21:37:06  Running with dbt=1.11.11
21:37:06  Registered adapter: clickhouse=1.10.0
21:37:06  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:37:07  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:07
21:37:07  Concurrency: 1 threads (target='dev')
21:37:07
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 年报    |     75980 |
| 中报    |     71407 |
| 一季报   |     69943 |
| 三季报   |     66935 |
```

### 数值范围：ACCEPT_DEPOSIT_INTERBANK

```sql
select
    min(`ACCEPT_DEPOSIT_INTERBANK`) as min_value,
    max(`ACCEPT_DEPOSIT_INTERBANK`) as max_value,
    countIf(`ACCEPT_DEPOSIT_INTERBANK` = 0) as zero_count,
    countIf(`ACCEPT_DEPOSIT_INTERBANK` < 0) as negative_count,
    countIf(isNull(`ACCEPT_DEPOSIT_INTERBANK`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:37:10  Running with dbt=1.11.11
21:37:10  Registered adapter: clickhouse=1.10.0
21:37:11  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:37:11  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:11
21:37:11  Concurrency: 1 threads (target='dev')
21:37:11
Previewing inline node:
| min_value |          max_value | zero_count | negative_count | null_count | row_count |
| --------- | ------------------ | ---------- | -------------- | ---------- | --------- |
|         0 | 777,003,282,181.39 |      20147 |              0 |     262378 |    284265 |
```

### 数值范围：ACCOUNTS_PAYABLE

```sql
select
    min(`ACCOUNTS_PAYABLE`) as min_value,
    max(`ACCOUNTS_PAYABLE`) as max_value,
    countIf(`ACCOUNTS_PAYABLE` = 0) as zero_count,
    countIf(`ACCOUNTS_PAYABLE` < 0) as negative_count,
    countIf(isNull(`ACCOUNTS_PAYABLE`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:37:14  Running with dbt=1.11.11
21:37:15  Registered adapter: clickhouse=1.10.0
21:37:15  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:37:16  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:16
21:37:16  Concurrency: 1 threads (target='dev')
21:37:16
Previewing inline node:
|      min_value |       max_value | zero_count | negative_count | null_count | row_count |
| -------------- | --------------- | ---------- | -------------- | ---------- | --------- |
| -44,288,812.27 | 997,477,873,000 |         99 |             20 |       9731 |    284265 |
```

### 数值范围：ACCOUNTS_RECE

```sql
select
    min(`ACCOUNTS_RECE`) as min_value,
    max(`ACCOUNTS_RECE`) as max_value,
    countIf(`ACCOUNTS_RECE` = 0) as zero_count,
    countIf(`ACCOUNTS_RECE` < 0) as negative_count,
    countIf(isNull(`ACCOUNTS_RECE`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:37:19  Running with dbt=1.11.11
21:37:19  Registered adapter: clickhouse=1.10.0
21:37:20  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:37:20  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:20
21:37:20  Concurrency: 1 threads (target='dev')
21:37:20
Previewing inline node:
|         min_value |       max_value | zero_count | negative_count | null_count | row_count |
| ----------------- | --------------- | ---------- | -------------- | ---------- | --------- |
| -1,878,640,045.72 | 442,286,697,000 |        177 |             24 |       7279 |    284265 |
```

### 数值范围：ACCRUED_EXPENSE

```sql
select
    min(`ACCRUED_EXPENSE`) as min_value,
    max(`ACCRUED_EXPENSE`) as max_value,
    countIf(`ACCRUED_EXPENSE` = 0) as zero_count,
    countIf(`ACCRUED_EXPENSE` < 0) as negative_count,
    countIf(isNull(`ACCRUED_EXPENSE`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:37:24  Running with dbt=1.11.11
21:37:24  Registered adapter: clickhouse=1.10.0
21:37:24  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:37:25  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:25
21:37:25  Concurrency: 1 threads (target='dev')
21:37:25
Previewing inline node:
| min_value |        max_value | zero_count | negative_count | null_count | row_count |
| --------- | ---------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 3,512,824,010.83 |          3 |              0 |     283529 |    284265 |
```

### 数值范围：ADVANCE_RECEIVABLES

```sql
select
    min(`ADVANCE_RECEIVABLES`) as min_value,
    max(`ADVANCE_RECEIVABLES`) as max_value,
    countIf(`ADVANCE_RECEIVABLES` = 0) as zero_count,
    countIf(`ADVANCE_RECEIVABLES` < 0) as negative_count,
    countIf(isNull(`ADVANCE_RECEIVABLES`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:37:28  Running with dbt=1.11.11
21:37:28  Registered adapter: clickhouse=1.10.0
21:37:29  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:37:29  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:29
21:37:29  Concurrency: 1 threads (target='dev')
21:37:29
Previewing inline node:
|      min_value |         max_value | zero_count | negative_count | null_count | row_count |
| -------------- | ----------------- | ---------- | -------------- | ---------- | --------- |
| -36,425,028.35 | 407,882,270,730.4 |       2088 |             44 |      82991 |    284265 |
```

### 数值范围：AGENT_TRADE_SECURITY

```sql
select
    min(`AGENT_TRADE_SECURITY`) as min_value,
    max(`AGENT_TRADE_SECURITY`) as max_value,
    countIf(`AGENT_TRADE_SECURITY` = 0) as zero_count,
    countIf(`AGENT_TRADE_SECURITY` < 0) as negative_count,
    countIf(isNull(`AGENT_TRADE_SECURITY`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:37:33  Running with dbt=1.11.11
21:37:33  Registered adapter: clickhouse=1.10.0
21:37:33  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:37:34  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:34
21:37:34  Concurrency: 1 threads (target='dev')
21:37:34
Previewing inline node:
| min_value |          max_value | zero_count | negative_count | null_count | row_count |
| --------- | ------------------ | ---------- | -------------- | ---------- | --------- |
|         0 | 181,414,551,134.91 |       2729 |              0 |     281012 |    284265 |
```

### 数值范围：AGENT_UNDERWRITE_SECURITY

```sql
select
    min(`AGENT_UNDERWRITE_SECURITY`) as min_value,
    max(`AGENT_UNDERWRITE_SECURITY`) as max_value,
    countIf(`AGENT_UNDERWRITE_SECURITY` = 0) as zero_count,
    countIf(`AGENT_UNDERWRITE_SECURITY` < 0) as negative_count,
    countIf(isNull(`AGENT_UNDERWRITE_SECURITY`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:37:37  Running with dbt=1.11.11
21:37:37  Registered adapter: clickhouse=1.10.0
21:37:38  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:37:38  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:38
21:37:38  Concurrency: 1 threads (target='dev')
21:37:38
Previewing inline node:
| min_value |     max_value | zero_count | negative_count | null_count | row_count |
| --------- | ------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 1,703,950,000 |       2734 |              0 |     281482 |    284265 |
```

### 数值范围：AMORTIZE_COST_FINASSET

```sql
select
    min(`AMORTIZE_COST_FINASSET`) as min_value,
    max(`AMORTIZE_COST_FINASSET`) as max_value,
    countIf(`AMORTIZE_COST_FINASSET` = 0) as zero_count,
    countIf(`AMORTIZE_COST_FINASSET` < 0) as negative_count,
    countIf(isNull(`AMORTIZE_COST_FINASSET`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__balance') }}
```


结果（成功）：

```text
21:37:42  Running with dbt=1.11.11
21:37:42  Registered adapter: clickhouse=1.10.0
21:37:42  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:37:43  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:37:43
21:37:43  Concurrency: 1 threads (target='dev')
21:37:43
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|           |           |          0 |              0 |     284265 |    284265 |
```
