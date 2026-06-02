# Raw 数据画像：eastmoney__income_ytd

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__income_ytd.yml`
- dbt source：`source('raw', 'eastmoney__income_ytd')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__income_ytd.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`eastmoney__income_ytd`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__income_ytd --execute --output ../docs/references/raw_profile/eastmoney__income_ytd.md`，并补充 ClickHouse 结构化汇总查询
- 行数：298,396
- 数据范围：`REPORT_DATE`: 1988-12-31 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 523 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`eastmoney__income_ytd`
- ClickHouse raw 表：`fleur_raw.eastmoney__income_ytd`
- 表说明：EastMoney year-to-date income F10 rows by natural-year raw partition.

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
| CURRENCY | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利润表年初至报告期末金额使用的币种。 |
| TOTAL_OPERATE_INCOME | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业总收入 |
| TOTAL_OPERATE_INCOME_YOY | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业总收入同比增长率（%） |
| OPERATE_INCOME | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业收入 |
| OPERATE_INCOME_YOY | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业收入同比增长率（%） |
| INTEREST_INCOME | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利息收入 |
| INTEREST_INCOME_YOY | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利息收入同比增长率（%） |
| EARNED_PREMIUM | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 已赚保费 |
| EARNED_PREMIUM_YOY | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 已赚保费同比增长率（%） |
| FEE_COMMISSION_INCOME | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 手续费及佣金收入 |
| FEE_COMMISSION_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 手续费及佣金收入同比增长率（%） |
| OTHER_BUSINESS_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他业务收入 |
| OTHER_BUSINESS_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他业务收入同比增长率（%） |
| TOI_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业总收入其他 |
| TOI_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业总收入其他同比增长率（%） |
| TOTAL_OPERATE_COST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业总成本 |
| TOTAL_OPERATE_COST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业总成本同比增长率（%） |
| OPERATE_COST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业成本 |
| OPERATE_COST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业成本同比增长率（%） |
| INTEREST_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利息支出 |
| INTEREST_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利息支出同比增长率（%） |
| FEE_COMMISSION_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 手续费及佣金支出 |
| FEE_COMMISSION_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 手续费及佣金支出同比增长率（%） |
| RESEARCH_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 研发费用 |
| RESEARCH_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 研发费用同比增长率（%） |
| SURRENDER_VALUE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 退保金 |
| SURRENDER_VALUE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 退保金同比增长率（%） |
| NET_COMPENSATE_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 分保费用 |
| NET_COMPENSATE_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 分保费用同比增长率（%） |
| NET_CONTRACT_RESERVE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 提取保险合同准备金 |
| NET_CONTRACT_RESERVE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 提取保险合同准备金同比增长率（%） |
| POLICY_BONUS_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 保单红利支出 |
| POLICY_BONUS_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 保单红利支出同比增长率（%） |
| REINSURE_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 分保费用支出 |
| REINSURE_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 分保费用支出同比增长率（%） |
| OTHER_BUSINESS_COST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他业务成本 |
| OTHER_BUSINESS_COST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他业务成本同比增长率（%） |
| OPERATE_TAX_ADD | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业税金及附加 |
| OPERATE_TAX_ADD_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业税金及附加同比增长率（%） |
| SALE_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 销售费用 |
| SALE_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 销售费用同比增长率（%） |
| MANAGE_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 管理费用 |
| MANAGE_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 管理费用同比增长率（%） |
| ME_RESEARCH_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 管理费用中的研发费用 |
| ME_RESEARCH_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 管理费用中的研发费用同比增长率（%） |
| FINANCE_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 财务费用 |
| FINANCE_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 财务费用同比增长率（%） |
| FE_INTEREST_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 财务费用之利息费用 |
| FE_INTEREST_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 财务费用之利息费用同比增长率（%） |
| FE_INTEREST_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 财务费用之利息收入 |
| FE_INTEREST_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 财务费用之利息收入同比增长率（%） |
| ASSET_IMPAIRMENT_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产减值损失 |
| ASSET_IMPAIRMENT_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产减值损失同比增长率（%） |
| CREDIT_IMPAIRMENT_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 信用减值损失 |
| CREDIT_IMPAIRMENT_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 信用减值损失同比增长率（%） |
| TOC_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业总成本其他 |
| TOC_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业总成本其他同比增长率（%） |
| FAIRVALUE_CHANGE_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 公允价值变动收益 |
| FAIRVALUE_CHANGE_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 公允价值变动收益同比增长率（%） |
| INVEST_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资收益 |
| INVEST_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 投资收益同比增长率（%） |
| INVEST_JOINT_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 对联营企业和合营企业的投资收益 |
| INVEST_JOINT_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 对联营企业和合营企业的投资收益同比增长率（%） |
| NET_EXPOSURE_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净敞口收益 |
| NET_EXPOSURE_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净敞口收益同比增长率（%） |
| EXCHANGE_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 汇兑收益 |
| EXCHANGE_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 汇兑收益同比增长率（%） |
| ASSET_DISPOSAL_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产处置收益 |
| ASSET_DISPOSAL_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产处置收益同比增长率（%） |
| ASSET_IMPAIRMENT_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产减值收益 |
| ASSET_IMPAIRMENT_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 资产减值收益同比增长率（%） |
| CREDIT_IMPAIRMENT_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 信用减值收益 |
| CREDIT_IMPAIRMENT_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 信用减值收益同比增长率（%） |
| OTHER_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他收益 |
| OTHER_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他收益同比增长率（%） |
| OPERATE_PROFIT_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业利润其他 |
| OPERATE_PROFIT_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业利润其他同比增长率（%） |
| OPERATE_PROFIT_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业利润平衡项 |
| OPERATE_PROFIT_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业利润平衡项同比增长率（%） |
| OPERATE_PROFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业利润 |
| OPERATE_PROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业利润同比增长率（%） |
| NONBUSINESS_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业外收入 |
| NONBUSINESS_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业外收入同比增长率（%） |
| NONCURRENT_DISPOSAL_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产处置净收益 |
| NONCURRENT_DISPOSAL_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产处置净收益同比增长率（%） |
| NONBUSINESS_EXPENSE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业外支出 |
| NONBUSINESS_EXPENSE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 营业外支出同比增长率（%） |
| NONCURRENT_DISPOSAL_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产处置净损失 |
| NONCURRENT_DISPOSAL_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流动资产处置净损失同比增长率（%） |
| EFFECT_TP_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 影响利润总额其他 |
| EFFECT_TP_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 影响利润总额其他同比增长率（%） |
| TOTAL_PROFIT_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利润总额平衡项 |
| TOTAL_PROFIT_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利润总额平衡项同比增长率（%） |
| TOTAL_PROFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利润总额 |
| TOTAL_PROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 利润总额同比增长率（%） |
| INCOME_TAX | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 所得税费用 |
| INCOME_TAX_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 所得税费用同比增长率（%） |
| EFFECT_NETPROFIT_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 影响净利润其他 |
| EFFECT_NETPROFIT_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 影响净利润其他同比增长率（%） |
| EFFECT_NETPROFIT_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润平衡项 |
| EFFECT_NETPROFIT_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润平衡项同比增长率（%） |
| UNCONFIRM_INVEST_LOSS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 未确认投资损失 |
| UNCONFIRM_INVEST_LOSS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 未确认投资损失同比增长率（%） |
| NETPROFIT | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润 |
| NETPROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润同比增长率（%） |
| PRECOMBINE_PROFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合并前净损益 |
| PRECOMBINE_PROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合并前净损益同比增长率（%） |
| CONTINUED_NETPROFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持续经营净利润 |
| CONTINUED_NETPROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持续经营净利润同比增长率（%） |
| DISCONTINUED_NETPROFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 终止经营净利润 |
| DISCONTINUED_NETPROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 终止经营净利润同比增长率（%） |
| PARENT_NETPROFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归属于母公司股东的净利润 |
| PARENT_NETPROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归属于母公司股东的净利润同比增长率（%） |
| MINORITY_INTEREST | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 少数股东损益 |
| MINORITY_INTEREST_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 少数股东损益同比增长率（%） |
| DEDUCT_PARENT_NETPROFIT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 扣除非经常性损益后归属于母公司股东的净利润 |
| DEDUCT_PARENT_NETPROFIT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 扣非归母净利润同比增长率（%） |
| NETPROFIT_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润其他 |
| NETPROFIT_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润其他同比增长率（%） |
| NETPROFIT_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润平衡项 |
| NETPROFIT_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 净利润平衡项同比增长率（%） |
| BASIC_EPS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 基本每股收益（元/股） |
| BASIC_EPS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 基本每股收益（元/股）同比增长率（%） |
| DILUTED_EPS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 稀释每股收益（元/股） |
| DILUTED_EPS_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 稀释每股收益（元/股）同比增长率（%） |
| OTHER_COMPRE_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他综合收益总额 |
| OTHER_COMPRE_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他综合收益总额同比增长率（%） |
| PARENT_OCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归属于母公司股东的其他综合收益 |
| PARENT_OCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母其他综合收益同比增长率（%） |
| MINORITY_OCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归属于少数股东的其他综合收益 |
| MINORITY_OCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 少数股东其他综合收益同比增长率（%） |
| PARENT_OCI_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母其他综合收益其他 |
| PARENT_OCI_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母其他综合收益其他同比增长率（%） |
| PARENT_OCI_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母其他综合收益平衡项 |
| PARENT_OCI_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母其他综合收益平衡项同比增长率（%） |
| UNABLE_OCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以后将重分类进损益的其他综合收益 |
| UNABLE_OCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以后将重分类进损益的其他综合收益同比增长率（%） |
| CREDITRISK_FAIRVALUE_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 信用风险引起的公允价值变动 |
| CREDITRISK_FAIRVALUE_CHANGE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 信用风险引起的公允价值变动同比增长率（%） |
| OTHERRIGHT_FAIRVALUE_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他权益工具公允价值变动 |
| OTHERRIGHT_FAIRVALUE_CHANGE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他权益工具公允价值变动同比增长率（%） |
| SETUP_PROFIT_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 重分类调整变动 |
| SETUP_PROFIT_CHANGE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 重分类调整变动同比增长率（%） |
| RIGHTLAW_UNABLE_OCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 权益法下不能重分类的其他综合收益 |
| RIGHTLAW_UNABLE_OCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 权益法下不能重分类的其他综合收益同比增长率（%） |
| UNABLE_OCI_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 不能重分类其他综合收益其他 |
| UNABLE_OCI_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 不能重分类其他综合收益其他同比增长率（%） |
| UNABLE_OCI_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 不能重分类其他综合收益平衡项 |
| UNABLE_OCI_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 不能重分类其他综合收益平衡项同比增长率（%） |
| ABLE_OCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以后将重分类进损益的其他综合收益（可重分类） |
| ABLE_OCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 以后将重分类进损益的其他综合收益（可重分类）同比增长率（%） |
| RIGHTLAW_ABLE_OCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 权益法下可重分类的其他综合收益 |
| RIGHTLAW_ABLE_OCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 权益法下可重分类的其他综合收益同比增长率（%） |
| AFA_FAIRVALUE_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 可供出售金融资产公允价值变动 |
| AFA_FAIRVALUE_CHANGE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 可供出售金融资产公允价值变动同比增长率（%） |
| HMI_AFA | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有有待售资产公允价值变动 |
| HMI_AFA_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持有有待售资产公允价值变动同比增长率（%） |
| CASHFLOW_HEDGE_VALID | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金流量套期有效部分 |
| CASHFLOW_HEDGE_VALID_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 现金流量套期有效部分同比增长率（%） |
| CREDITOR_FAIRVALUE_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 债权投资公允价值变动 |
| CREDITOR_FAIRVALUE_CHANGE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 债权投资公允价值变动同比增长率（%） |
| CREDITOR_IMPAIRMENT_RESERVE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 债权投资减值准备 |
| CREDITOR_IMPAIRMENT_RESERVE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 债权投资减值准备同比增长率（%） |
| FINANCE_OCI_AMT | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 金融资产重分类金额 |
| FINANCE_OCI_AMT_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 金融资产重分类金额同比增长率（%） |
| CONVERT_DIFF | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 外币报表折算差额 |
| CONVERT_DIFF_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 外币报表折算差额同比增长率（%） |
| ABLE_OCI_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 可重分类其他综合收益其他 |
| ABLE_OCI_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 可重分类其他综合收益其他同比增长率（%） |
| ABLE_OCI_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 可重分类其他综合收益平衡项 |
| ABLE_OCI_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 可重分类其他综合收益平衡项同比增长率（%） |
| OCI_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他综合收益其他 |
| OCI_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他综合收益其他同比增长率（%） |
| OCI_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他综合收益平衡项 |
| OCI_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他综合收益平衡项同比增长率（%） |
| TOTAL_COMPRE_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 综合收益总额 |
| TOTAL_COMPRE_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 综合收益总额同比增长率（%） |
| PARENT_TCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归属于母公司股东的综合收益总额 |
| PARENT_TCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归母综合收益总额同比增长率（%） |
| MINORITY_TCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 归属于少数股东的综合收益总额 |
| MINORITY_TCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 少数股东综合收益总额同比增长率（%） |
| PRECOMBINE_TCI | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合并前综合收益总额 |
| PRECOMBINE_TCI_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合并前综合收益总额同比增长率（%） |
| EFFECT_TCI_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 综合收益总额平衡项 |
| EFFECT_TCI_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 综合收益总额平衡项同比增长率（%） |
| TCI_OTHER | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 综合收益总额其他 |
| TCI_OTHER_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 综合收益总额其他同比增长率（%） |
| TCI_BALANCE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 综合收益总额平衡项 |
| TCI_BALANCE_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 综合收益总额平衡项同比增长率（%） |
| ACF_END_INCOME | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持续经营终止经营净损益 |
| ACF_END_INCOME_YOY | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 持续经营终止经营净损益同比增长率（%） |
| OPINION_TYPE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 审计意见类型 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 298396/298396，供应商前缀 0/298396，纯数字 0/298396，空值 0/298396；`SECURITY_CODE`: canonical 后缀 0/298396，供应商前缀 0/298396，纯数字 298396/298396，空值 0/298396
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：EastMoney 后缀格式可直接作为 canonical security_code；本地代码必须仅作为 local code 使用。

### 日期与时间字段

- 已画像字段：`REPORT_DATE`, `NOTICE_DATE`, `UPDATE_DATE`
- 范围：`REPORT_DATE`: 1988-12-31 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 523 行
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `SECURITY_NAME_ABBR`, `ORG_CODE`, `ORG_TYPE`, `REPORT_TYPE`, `REPORT_DATE_NAME`, `SECURITY_TYPE_CODE`
- 取值：`SECUCODE`: `600653.SH`(123), `600654.SH`(123), `600651.SH`(121), `600601.SH`(121), `000030.SZ`(120), `600610.SH`(120), `000029.SZ`(119), `000501.SZ`(119)；`SECURITY_CODE`: `600654`(123), `600653`(123), `600601`(121), `600651`(121), `600610`(120), `000030`(120), `000501`(119), `600602`(119)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(123), `申华控股`(123), `方正科技`(121), `飞乐音响`(121), `中毅达`(120), `富奥股份`(120)；`ORG_CODE`: `10004106`(200), `10004127`(198), `10004293`(154), `10116535`(128), `10003963`(123), `10003964`(123), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(292603), `证券`(2826), `银行`(2449), `保险`(518)；`REPORT_TYPE`: `年报`(77446), `中报`(74897), `一季报`(74512), `三季报`(71541)；`REPORT_DATE_NAME`: `2026一季报`(5198), `2025年报`(5183), `2024一季报`(5174), `2025一季报`(5171), `2025三季报`(5159), `2025中报`(5150), `2024三季报`(5127), `2024年报`(5118)；`SECURITY_TYPE_CODE`: `058001001`(298373), `058001008`(23)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`TOTAL_OPERATE_INCOME`, `NETPROFIT`, `TOTAL_OPERATE_INCOME_YOY`, `OPERATE_INCOME`, `OPERATE_INCOME_YOY`, `INTEREST_INCOME`, `INTEREST_INCOME_YOY`, `EARNED_PREMIUM`, `EARNED_PREMIUM_YOY`, `FEE_COMMISSION_INCOME`
- 最小/最大值：`TOTAL_OPERATE_INCOME` min=-673495950.57, max=3318168000000.0, zero=1288, negative=34, NULL=0；`NETPROFIT` min=-91810091101.49, max=370766000000.0, zero=87, negative=48643, NULL=0；`TOTAL_OPERATE_INCOME_YOY` min=-306.468910397, max=6998082.27742629, zero=11620, negative=95980, NULL=0；`OPERATE_INCOME` min=-673495950.57, max=3318168000000.0, zero=1336, negative=34, NULL=0；`OPERATE_INCOME_YOY` min=-306.468910397, max=6998082.27742629, zero=12048, negative=95914, NULL=0；`INTEREST_INCOME` min=-116120513.42, max=1427948000000.0, zero=289435, negative=8, NULL=0；`INTEREST_INCOME_YOY` min=-1156.9209958447, max=180269313.135593, zero=290371, negative=3219, NULL=0；`EARNED_PREMIUM` min=-4289316.07, max=757599000000.0, zero=297590, negative=2, NULL=0；`EARNED_PREMIUM_YOY` min=-101.3152770417, max=35872.9713665051, zero=297691, negative=217, NULL=0；`FEE_COMMISSION_INCOME` min=-854402.51, max=164714000000.0, zero=291111, negative=12, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `UPDATE_DATE` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 523 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `SECURITY_CODE` 只有 6 位本地代码 | 中 | 298396/298396 行 | 仅作为 `security_local_code`；不可单独推出交易所 | 需要其他字段或主数据补齐交易所 |
| `TOTAL_OPERATE_INCOME` 存在负值 | 低 | 34 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `NETPROFIT` 存在负值 | 低 | 48643 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `TOTAL_OPERATE_INCOME_YOY` 存在负值 | 低 | 95980 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `OPERATE_INCOME` 存在负值 | 低 | 34 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `OPERATE_INCOME_YOY` 存在负值 | 低 | 95914 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `INTEREST_INCOME` 存在负值 | 低 | 8 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `INTEREST_INCOME_YOY` 存在负值 | 低 | 3219 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `EARNED_PREMIUM` 存在负值 | 低 | 2 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `EARNED_PREMIUM_YOY` 存在负值 | 低 | 217 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |
| `FEE_COMMISSION_INCOME` 存在负值 | 低 | 12 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |

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

- `select count() from fleur_raw.eastmoney__income_ytd`：298,396
- 日期字段范围：`REPORT_DATE`: 1988-12-31 至 2026-03-31，NULL 0 行；`NOTICE_DATE`: 1991-06-10 至 2026-05-15，NULL 0 行；`UPDATE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 523 行
- 证券代码格式：`SECUCODE`: canonical 后缀 298396/298396，供应商前缀 0/298396，纯数字 0/298396，空值 0/298396；`SECURITY_CODE`: canonical 后缀 0/298396，供应商前缀 0/298396，纯数字 298396/298396，空值 0/298396
- 候选键重复：未发现重复
- 枚举 top values：`SECUCODE`: `600653.SH`(123), `600654.SH`(123), `600651.SH`(121), `600601.SH`(121), `000030.SZ`(120), `600610.SH`(120), `000029.SZ`(119), `000501.SZ`(119)；`SECURITY_CODE`: `600654`(123), `600653`(123), `600601`(121), `600651`(121), `600610`(120), `000030`(120), `000501`(119), `600602`(119)；`SECURITY_NAME_ABBR`: `东方明珠`(184), `百联股份`(171), `中安科`(123), `申华控股`(123), `方正科技`(121), `飞乐音响`(121), `中毅达`(120), `富奥股份`(120)；`ORG_CODE`: `10004106`(200), `10004127`(198), `10004293`(154), `10116535`(128), `10003963`(123), `10003964`(123), `10002659`(121), `10003961`(121)；`ORG_TYPE`: `通用`(292603), `证券`(2826), `银行`(2449), `保险`(518)；`REPORT_TYPE`: `年报`(77446), `中报`(74897), `一季报`(74512), `三季报`(71541)；`REPORT_DATE_NAME`: `2026一季报`(5198), `2025年报`(5183), `2024一季报`(5174), `2025一季报`(5171), `2025三季报`(5159), `2025中报`(5150), `2024三季报`(5127), `2024年报`(5118)；`SECURITY_TYPE_CODE`: `058001001`(298373), `058001008`(23)
- 数值范围摘要：`TOTAL_OPERATE_INCOME` min=-673495950.57, max=3318168000000.0, zero=1288, negative=34, NULL=0；`NETPROFIT` min=-91810091101.49, max=370766000000.0, zero=87, negative=48643, NULL=0；`TOTAL_OPERATE_INCOME_YOY` min=-306.468910397, max=6998082.27742629, zero=11620, negative=95980, NULL=0；`OPERATE_INCOME` min=-673495950.57, max=3318168000000.0, zero=1336, negative=34, NULL=0；`OPERATE_INCOME_YOY` min=-306.468910397, max=6998082.27742629, zero=12048, negative=95914, NULL=0；`INTEREST_INCOME` min=-116120513.42, max=1427948000000.0, zero=289435, negative=8, NULL=0；`INTEREST_INCOME_YOY` min=-1156.9209958447, max=180269313.135593, zero=290371, negative=3219, NULL=0；`EARNED_PREMIUM` min=-4289316.07, max=757599000000.0, zero=297590, negative=2, NULL=0；`EARNED_PREMIUM_YOY` min=-101.3152770417, max=35872.9713665051, zero=297691, negative=217, NULL=0；`FEE_COMMISSION_INCOME` min=-854402.51, max=164714000000.0, zero=291111, negative=12, NULL=0
