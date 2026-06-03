# Raw 数据画像：eastmoney__income_sq

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__income_sq.yml`
- dbt source：`source('raw', 'eastmoney__income_sq')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__income_sq`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__income_sq --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__income_sq.md`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`eastmoney__income_sq`
- ClickHouse raw 表：`fleur_raw.eastmoney__income_sq`
- 表说明：EastMoney single-quarter income F10 rows by natural-year raw partition.

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
| CURRENCY | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENCY`。 原始字段说明：利润表单季度金额使用的币种。 |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPINION_TYPE`。 原始字段说明：审计意见类型 |
| OSOPINION_TYPE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OSOPINION_TYPE`。 原始字段说明：内控审计意见类型 |
| TOTAL_OPERATE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_INCOME`。 原始字段说明：营业总收入 |
| TOTAL_OPERATE_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_INCOME_QOQ`。 原始字段说明：营业总收入环比增长率（%） |
| OPERATE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INCOME`。 原始字段说明：营业收入 |
| OPERATE_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INCOME_QOQ`。 原始字段说明：营业收入环比增长率（%） |
| INTEREST_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_INCOME`。 原始字段说明：利息收入 |
| INTEREST_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_INCOME_QOQ`。 原始字段说明：利息收入环比增长率（%） |
| EARNED_PREMIUM | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EARNED_PREMIUM`。 原始字段说明：已赚保费 |
| EARNED_PREMIUM_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EARNED_PREMIUM_QOQ`。 原始字段说明：已赚保费环比增长率（%） |
| FEE_COMMISSION_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_INCOME`。 原始字段说明：手续费及佣金收入 |
| FEE_COMMISSION_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_INCOME_QOQ`。 原始字段说明：手续费及佣金收入环比增长率（%） |
| OTHER_BUSINESS_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_INCOME`。 原始字段说明：其他业务收入 |
| OTHER_BUSINESS_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_INCOME_QOQ`。 原始字段说明：其他业务收入环比增长率（%） |
| TOI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOI_OTHER`。 原始字段说明：营业总收入其他 |
| TOI_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOI_OTHER_QOQ`。 原始字段说明：营业总收入其他环比增长率（%） |
| TOTAL_OPERATE_COST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_COST`。 原始字段说明：营业总成本 |
| TOTAL_OPERATE_COST_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_COST_QOQ`。 原始字段说明：营业总成本环比增长率（%） |
| OPERATE_COST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_COST`。 原始字段说明：营业成本 |
| OPERATE_COST_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_COST_QOQ`。 原始字段说明：营业成本环比增长率（%） |
| INTEREST_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_EXPENSE`。 原始字段说明：利息支出 |
| INTEREST_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_EXPENSE_QOQ`。 原始字段说明：利息支出环比增长率（%） |
| FEE_COMMISSION_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_EXPENSE`。 原始字段说明：手续费及佣金支出 |
| FEE_COMMISSION_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_EXPENSE_QOQ`。 原始字段说明：手续费及佣金支出环比增长率（%） |
| RESEARCH_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RESEARCH_EXPENSE`。 原始字段说明：研发费用 |
| RESEARCH_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RESEARCH_EXPENSE_QOQ`。 原始字段说明：研发费用环比增长率（%） |
| SURRENDER_VALUE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SURRENDER_VALUE`。 原始字段说明：退保金 |
| SURRENDER_VALUE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SURRENDER_VALUE_QOQ`。 原始字段说明：退保金环比增长率（%） |
| NET_COMPENSATE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_COMPENSATE_EXPENSE`。 原始字段说明：分保费用 |
| NET_COMPENSATE_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_COMPENSATE_EXPENSE_QOQ`。 原始字段说明：分保费用环比增长率（%） |
| NET_CONTRACT_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_CONTRACT_RESERVE`。 原始字段说明：提取保险合同准备金 |
| NET_CONTRACT_RESERVE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_CONTRACT_RESERVE_QOQ`。 原始字段说明：提取保险合同准备金环比增长率（%） |
| POLICY_BONUS_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `POLICY_BONUS_EXPENSE`。 原始字段说明：保单红利支出 |
| POLICY_BONUS_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `POLICY_BONUS_EXPENSE_QOQ`。 原始字段说明：保单红利支出环比增长率（%） |
| REINSURE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_EXPENSE`。 原始字段说明：分保费用支出 |
| REINSURE_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_EXPENSE_QOQ`。 原始字段说明：分保费用支出环比增长率（%） |
| OTHER_BUSINESS_COST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_COST`。 原始字段说明：其他业务成本 |
| OTHER_BUSINESS_COST_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_COST_QOQ`。 原始字段说明：其他业务成本环比增长率（%） |
| OPERATE_TAX_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_TAX_ADD`。 原始字段说明：营业税金及附加 |
| OPERATE_TAX_ADD_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_TAX_ADD_QOQ`。 原始字段说明：营业税金及附加环比增长率（%） |
| SALE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SALE_EXPENSE`。 原始字段说明：销售费用 |
| SALE_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SALE_EXPENSE_QOQ`。 原始字段说明：销售费用环比增长率（%） |
| MANAGE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MANAGE_EXPENSE`。 原始字段说明：管理费用 |
| MANAGE_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MANAGE_EXPENSE_QOQ`。 原始字段说明：管理费用环比增长率（%） |
| ME_RESEARCH_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ME_RESEARCH_EXPENSE`。 原始字段说明：管理费用中的研发费用 |
| ME_RESEARCH_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ME_RESEARCH_EXPENSE_QOQ`。 原始字段说明：管理费用中的研发费用环比增长率（%） |
| FINANCE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_EXPENSE`。 原始字段说明：财务费用 |
| FINANCE_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_EXPENSE_QOQ`。 原始字段说明：财务费用环比增长率（%） |
| FE_INTEREST_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_EXPENSE`。 原始字段说明：财务费用之利息费用 |
| FE_INTEREST_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_EXPENSE_QOQ`。 原始字段说明：财务费用之利息费用环比增长率（%） |
| FE_INTEREST_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_INCOME`。 原始字段说明：财务费用之利息收入 |
| FE_INTEREST_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_INCOME_QOQ`。 原始字段说明：财务费用之利息收入环比增长率（%） |
| ASSET_IMPAIRMENT_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_LOSS`。 原始字段说明：资产减值损失 |
| ASSET_IMPAIRMENT_LOSS_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_LOSS_QOQ`。 原始字段说明：资产减值损失环比增长率（%） |
| CREDIT_IMPAIRMENT_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_LOSS`。 原始字段说明：信用减值损失 |
| CREDIT_IMPAIRMENT_LOSS_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_LOSS_QOQ`。 原始字段说明：信用减值损失环比增长率（%） |
| OTHER_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_INCOME`。 原始字段说明：其他收益 |
| OTHER_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_INCOME_QOQ`。 原始字段说明：其他收益环比增长率（%） |
| TOC_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOC_OTHER`。 原始字段说明：营业总成本其他 |
| TOC_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOC_OTHER_QOQ`。 原始字段说明：营业总成本其他环比增长率（%） |
| INVEST_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INCOME`。 原始字段说明：投资收益 |
| INVEST_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INCOME_QOQ`。 原始字段说明：投资收益环比增长率（%） |
| INVEST_JOINT_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_JOINT_INCOME`。 原始字段说明：对联营企业和合营企业的投资收益 |
| INVEST_JOINT_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_JOINT_INCOME_QOQ`。 原始字段说明：对联营企业和合营企业的投资收益环比增长率（%） |
| ACF_END_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACF_END_INCOME`。 原始字段说明：持续经营终止经营净损益 |
| ACF_END_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACF_END_INCOME_QOQ`。 原始字段说明：持续经营终止经营净损益环比增长率（%） |
| EXCHANGE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EXCHANGE_INCOME`。 原始字段说明：汇兑收益 |
| EXCHANGE_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EXCHANGE_INCOME_QOQ`。 原始字段说明：汇兑收益环比增长率（%） |
| NET_EXPOSURE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_EXPOSURE_INCOME`。 原始字段说明：净敞口收益 |
| NET_EXPOSURE_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_EXPOSURE_INCOME_QOQ`。 原始字段说明：净敞口收益环比增长率（%） |
| FAIRVALUE_CHANGE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FAIRVALUE_CHANGE_INCOME`。 原始字段说明：公允价值变动收益 |
| FAIRVALUE_CHANGE_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FAIRVALUE_CHANGE_INCOME_QOQ`。 原始字段说明：公允价值变动收益环比增长率（%） |
| ASSET_DISPOSAL_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_DISPOSAL_INCOME`。 原始字段说明：资产处置收益 |
| ASSET_DISPOSAL_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_DISPOSAL_INCOME_QOQ`。 原始字段说明：资产处置收益环比增长率（%） |
| CREDIT_IMPAIRMENT_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_INCOME`。 原始字段说明：信用减值收益 |
| CREDIT_IMPAIRMENT_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_INCOME_QOQ`。 原始字段说明：信用减值收益环比增长率（%） |
| ASSET_IMPAIRMENT_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_INCOME`。 原始字段说明：资产减值收益 |
| ASSET_IMPAIRMENT_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_INCOME_QOQ`。 原始字段说明：资产减值收益环比增长率（%） |
| OPERATE_PROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT`。 原始字段说明：营业利润 |
| OPERATE_PROFIT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_QOQ`。 原始字段说明：营业利润环比增长率（%） |
| NONBUSINESS_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_INCOME`。 原始字段说明：营业外收入 |
| NONBUSINESS_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_INCOME_QOQ`。 原始字段说明：营业外收入环比增长率（%） |
| NONCURRENT_DISPOSAL_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_INCOME`。 原始字段说明：非流动资产处置净收益 |
| NONCURRENT_DISPOSAL_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_INCOME_QOQ`。 原始字段说明：非流动资产处置净收益环比增长率（%） |
| NONBUSINESS_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_EXPENSE`。 原始字段说明：营业外支出 |
| NONBUSINESS_EXPENSE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_EXPENSE_QOQ`。 原始字段说明：营业外支出环比增长率（%） |
| NONCURRENT_DISPOSAL_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_LOSS`。 原始字段说明：非流动资产处置净损失 |
| NONCURRENT_DISPOSAL_LOSS_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_LOSS_QOQ`。 原始字段说明：非流动资产处置净损失环比增长率（%） |
| OPERATE_PROFIT_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_OTHER`。 原始字段说明：营业利润其他 |
| OPERATE_PROFIT_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_OTHER_QOQ`。 原始字段说明：营业利润其他环比增长率（%） |
| OPERATE_PROFIT_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_BALANCE`。 原始字段说明：营业利润平衡项 |
| OPERATE_PROFIT_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_BALANCE_QOQ`。 原始字段说明：营业利润平衡项环比增长率（%） |
| TOTAL_PROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT`。 原始字段说明：利润总额 |
| TOTAL_PROFIT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT_QOQ`。 原始字段说明：利润总额环比增长率（%） |
| EFFECT_TP_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TP_OTHER`。 原始字段说明：影响利润总额其他 |
| EFFECT_TP_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TP_OTHER_QOQ`。 原始字段说明：影响利润总额其他环比增长率（%） |
| TOTAL_PROFIT_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT_BALANCE`。 原始字段说明：利润总额平衡项 |
| TOTAL_PROFIT_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT_BALANCE_QOQ`。 原始字段说明：利润总额平衡项环比增长率（%） |
| INCOME_TAX | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INCOME_TAX`。 原始字段说明：所得税费用 |
| INCOME_TAX_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INCOME_TAX_QOQ`。 原始字段说明：所得税费用环比增长率（%） |
| NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT`。 原始字段说明：净利润 |
| NETPROFIT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_QOQ`。 原始字段说明：净利润环比增长率（%） |
| CONTINUED_NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTINUED_NETPROFIT`。 原始字段说明：持续经营净利润 |
| CONTINUED_NETPROFIT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTINUED_NETPROFIT_QOQ`。 原始字段说明：持续经营净利润环比增长率（%） |
| DISCONTINUED_NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISCONTINUED_NETPROFIT`。 原始字段说明：终止经营净利润 |
| DISCONTINUED_NETPROFIT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISCONTINUED_NETPROFIT_QOQ`。 原始字段说明：终止经营净利润环比增长率（%） |
| NETPROFIT_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_OTHER`。 原始字段说明：净利润其他 |
| NETPROFIT_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_OTHER_QOQ`。 原始字段说明：净利润其他环比增长率（%） |
| NETPROFIT_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_BALANCE`。 原始字段说明：净利润平衡项 |
| NETPROFIT_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_BALANCE_QOQ`。 原始字段说明：净利润平衡项环比增长率（%） |
| EFFECT_NETPROFIT_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_OTHER`。 原始字段说明：影响净利润其他 |
| EFFECT_NETPROFIT_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_OTHER_QOQ`。 原始字段说明：影响净利润其他环比增长率（%） |
| EFFECT_NETPROFIT_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_BALANCE`。 原始字段说明：净利润平衡项 |
| EFFECT_NETPROFIT_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_BALANCE_QOQ`。 原始字段说明：净利润平衡项环比增长率（%） |
| UNCONFIRM_INVEST_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNCONFIRM_INVEST_LOSS`。 原始字段说明：未确认投资损失 |
| UNCONFIRM_INVEST_LOSS_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNCONFIRM_INVEST_LOSS_QOQ`。 原始字段说明：未确认投资损失环比增长率（%） |
| MINORITY_INTEREST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_INTEREST`。 原始字段说明：少数股东损益 |
| MINORITY_INTEREST_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_INTEREST_QOQ`。 原始字段说明：少数股东损益环比增长率（%） |
| PARENT_NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_NETPROFIT`。 原始字段说明：归属于母公司股东的净利润 |
| PARENT_NETPROFIT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_NETPROFIT_QOQ`。 原始字段说明：归属于母公司股东的净利润环比增长率（%） |
| BASIC_EPS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BASIC_EPS`。 原始字段说明：基本每股收益（元/股） |
| BASIC_EPS_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BASIC_EPS_QOQ`。 原始字段说明：基本每股收益（元/股）环比增长率（%） |
| DILUTED_EPS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DILUTED_EPS`。 原始字段说明：稀释每股收益（元/股） |
| DILUTED_EPS_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DILUTED_EPS_QOQ`。 原始字段说明：稀释每股收益（元/股）环比增长率（%） |
| UNABLE_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI`。 原始字段说明：以后将重分类进损益的其他综合收益 |
| UNABLE_OCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_QOQ`。 原始字段说明：以后将重分类进损益的其他综合收益环比增长率（%） |
| CREDITRISK_FAIRVALUE_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITRISK_FAIRVALUE_CHANGE`。 原始字段说明：信用风险引起的公允价值变动 |
| CREDITRISK_FAIRVALUE_CHANGE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITRISK_FAIRVALUE_CHANGE_QOQ`。 原始字段说明：信用风险引起的公允价值变动环比增长率（%） |
| OTHERRIGHT_FAIRVALUE_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHERRIGHT_FAIRVALUE_CHANGE`。 原始字段说明：其他权益工具公允价值变动 |
| OTHERRIGHT_FAIRVALUE_CHANGE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHERRIGHT_FAIRVALUE_CHANGE_QOQ`。 原始字段说明：其他权益工具公允价值变动环比增长率（%） |
| SETUP_PROFIT_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SETUP_PROFIT_CHANGE`。 原始字段说明：重分类调整变动 |
| SETUP_PROFIT_CHANGE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SETUP_PROFIT_CHANGE_QOQ`。 原始字段说明：重分类调整变动环比增长率（%） |
| RIGHTLAW_UNABLE_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_UNABLE_OCI`。 原始字段说明：权益法下不能重分类的其他综合收益 |
| RIGHTLAW_UNABLE_OCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_UNABLE_OCI_QOQ`。 原始字段说明：权益法下不能重分类的其他综合收益环比增长率（%） |
| UNABLE_OCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_OTHER`。 原始字段说明：不能重分类其他综合收益其他 |
| UNABLE_OCI_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_OTHER_QOQ`。 原始字段说明：不能重分类其他综合收益其他环比增长率（%） |
| UNABLE_OCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_BALANCE`。 原始字段说明：不能重分类其他综合收益平衡项 |
| UNABLE_OCI_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_BALANCE_QOQ`。 原始字段说明：不能重分类其他综合收益平衡项环比增长率（%） |
| ABLE_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI`。 原始字段说明：以后将重分类进损益的其他综合收益（可重分类） |
| ABLE_OCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_QOQ`。 原始字段说明：以后将重分类进损益的其他综合收益（可重分类）环比增长率（%） |
| RIGHTLAW_ABLE_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_ABLE_OCI`。 原始字段说明：权益法下可重分类的其他综合收益 |
| RIGHTLAW_ABLE_OCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_ABLE_OCI_QOQ`。 原始字段说明：权益法下可重分类的其他综合收益环比增长率（%） |
| AFA_FAIRVALUE_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AFA_FAIRVALUE_CHANGE`。 原始字段说明：可供出售金融资产公允价值变动 |
| AFA_FAIRVALUE_CHANGE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AFA_FAIRVALUE_CHANGE_QOQ`。 原始字段说明：可供出售金融资产公允价值变动环比增长率（%） |
| HMI_AFA | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HMI_AFA`。 原始字段说明：持有有待售资产公允价值变动 |
| HMI_AFA_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HMI_AFA_QOQ`。 原始字段说明：持有有待售资产公允价值变动环比增长率（%） |
| CASHFLOW_HEDGE_VALID | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CASHFLOW_HEDGE_VALID`。 原始字段说明：现金流量套期有效部分 |
| CASHFLOW_HEDGE_VALID_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CASHFLOW_HEDGE_VALID_QOQ`。 原始字段说明：现金流量套期有效部分环比增长率（%） |
| CREDITOR_FAIRVALUE_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_FAIRVALUE_CHANGE`。 原始字段说明：债权投资公允价值变动 |
| CREDITOR_FAIRVALUE_CHANGE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_FAIRVALUE_CHANGE_QOQ`。 原始字段说明：债权投资公允价值变动环比增长率（%） |
| CREDITOR_IMPAIRMENT_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_IMPAIRMENT_RESERVE`。 原始字段说明：债权投资减值准备 |
| CREDITOR_IMPAIRMENT_RESERVE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_IMPAIRMENT_RESERVE_QOQ`。 原始字段说明：债权投资减值准备环比增长率（%） |
| FINANCE_OCI_AMT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OCI_AMT`。 原始字段说明：金融资产重分类金额 |
| FINANCE_OCI_AMT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OCI_AMT_QOQ`。 原始字段说明：金融资产重分类金额环比增长率（%） |
| CONVERT_DIFF | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_DIFF`。 原始字段说明：外币报表折算差额 |
| CONVERT_DIFF_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_DIFF_QOQ`。 原始字段说明：外币报表折算差额环比增长率（%） |
| ABLE_OCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_OTHER`。 原始字段说明：可重分类其他综合收益其他 |
| ABLE_OCI_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_OTHER_QOQ`。 原始字段说明：可重分类其他综合收益其他环比增长率（%） |
| ABLE_OCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_BALANCE`。 原始字段说明：可重分类其他综合收益平衡项 |
| ABLE_OCI_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_BALANCE_QOQ`。 原始字段说明：可重分类其他综合收益平衡项环比增长率（%） |
| OCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_OTHER`。 原始字段说明：其他综合收益其他 |
| OCI_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_OTHER_QOQ`。 原始字段说明：其他综合收益其他环比增长率（%） |
| OCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_BALANCE`。 原始字段说明：其他综合收益平衡项 |
| OCI_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_BALANCE_QOQ`。 原始字段说明：其他综合收益平衡项环比增长率（%） |
| OTHER_COMPRE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_COMPRE_INCOME`。 原始字段说明：其他综合收益总额 |
| OTHER_COMPRE_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_COMPRE_INCOME_QOQ`。 原始字段说明：其他综合收益总额环比增长率（%） |
| PARENT_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI`。 原始字段说明：归属于母公司股东的其他综合收益 |
| PARENT_OCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_QOQ`。 原始字段说明：归母其他综合收益环比增长率（%） |
| MINORITY_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_OCI`。 原始字段说明：归属于少数股东的其他综合收益 |
| MINORITY_OCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_OCI_QOQ`。 原始字段说明：少数股东其他综合收益环比增长率（%） |
| PARENT_OCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_OTHER`。 原始字段说明：归母其他综合收益其他 |
| PARENT_OCI_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_OTHER_QOQ`。 原始字段说明：归母其他综合收益其他环比增长率（%） |
| PARENT_OCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_BALANCE`。 原始字段说明：归母其他综合收益平衡项 |
| PARENT_OCI_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_BALANCE_QOQ`。 原始字段说明：归母其他综合收益平衡项环比增长率（%） |
| TOTAL_COMPRE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_COMPRE_INCOME`。 原始字段说明：综合收益总额 |
| TOTAL_COMPRE_INCOME_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_COMPRE_INCOME_QOQ`。 原始字段说明：综合收益总额环比增长率（%） |
| PARENT_TCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_TCI`。 原始字段说明：归属于母公司股东的综合收益总额 |
| PARENT_TCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_TCI_QOQ`。 原始字段说明：归母综合收益总额环比增长率（%） |
| MINORITY_TCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_TCI`。 原始字段说明：归属于少数股东的综合收益总额 |
| MINORITY_TCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_TCI_QOQ`。 原始字段说明：少数股东综合收益总额环比增长率（%） |
| EFFECT_TCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TCI_BALANCE`。 原始字段说明：综合收益总额平衡项 |
| EFFECT_TCI_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TCI_BALANCE_QOQ`。 原始字段说明：综合收益总额平衡项环比增长率（%） |
| TCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_OTHER`。 原始字段说明：综合收益总额其他 |
| TCI_OTHER_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_OTHER_QOQ`。 原始字段说明：综合收益总额其他环比增长率（%） |
| TCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_BALANCE`。 原始字段说明：综合收益总额平衡项 |
| TCI_BALANCE_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_BALANCE_QOQ`。 原始字段说明：综合收益总额平衡项环比增长率（%） |
| PRECOMBINE_PROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_PROFIT`。 原始字段说明：合并前净损益 |
| PRECOMBINE_PROFIT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_PROFIT_QOQ`。 原始字段说明：合并前净损益环比增长率（%） |
| PRECOMBINE_TCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_TCI`。 原始字段说明：合并前综合收益总额 |
| PRECOMBINE_TCI_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_TCI_QOQ`。 原始字段说明：合并前综合收益总额环比增长率（%） |
| DEDUCT_PARENT_NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEDUCT_PARENT_NETPROFIT`。 原始字段说明：扣除非经常性损益后归属于母公司股东的净利润 |
| DEDUCT_PARENT_NETPROFIT_QOQ | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEDUCT_PARENT_NETPROFIT_QOQ`。 原始字段说明：扣非归母净利润环比增长率（%） |
| TOTAL_OPERATE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_INCOME_YOY`。 原始字段说明：营业总收入同比增长率（%） |
| OPERATE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INCOME_YOY`。 原始字段说明：营业收入同比增长率（%） |
| INTEREST_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_INCOME_YOY`。 原始字段说明：利息收入同比增长率（%） |
| EARNED_PREMIUM_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EARNED_PREMIUM_YOY`。 原始字段说明：已赚保费同比增长率（%） |
| FEE_COMMISSION_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_INCOME_YOY`。 原始字段说明：手续费及佣金收入同比增长率（%） |
| OTHER_BUSINESS_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_INCOME_YOY`。 原始字段说明：其他业务收入同比增长率（%） |
| TOI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOI_OTHER_YOY`。 原始字段说明：营业总收入其他同比增长率（%） |
| TOTAL_OPERATE_COST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_COST_YOY`。 原始字段说明：营业总成本同比增长率（%） |
| OPERATE_COST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_COST_YOY`。 原始字段说明：营业成本同比增长率（%） |
| INTEREST_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_EXPENSE_YOY`。 原始字段说明：利息支出同比增长率（%） |
| FEE_COMMISSION_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_EXPENSE_YOY`。 原始字段说明：手续费及佣金支出同比增长率（%） |
| RESEARCH_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RESEARCH_EXPENSE_YOY`。 原始字段说明：研发费用同比增长率（%） |
| SURRENDER_VALUE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SURRENDER_VALUE_YOY`。 原始字段说明：退保金同比增长率（%） |
| NET_COMPENSATE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_COMPENSATE_EXPENSE_YOY`。 原始字段说明：分保费用同比增长率（%） |
| NET_CONTRACT_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_CONTRACT_RESERVE_YOY`。 原始字段说明：提取保险合同准备金同比增长率（%） |
| POLICY_BONUS_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `POLICY_BONUS_EXPENSE_YOY`。 原始字段说明：保单红利支出同比增长率（%） |
| REINSURE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_EXPENSE_YOY`。 原始字段说明：分保费用支出同比增长率（%） |
| OTHER_BUSINESS_COST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_COST_YOY`。 原始字段说明：其他业务成本同比增长率（%） |
| OPERATE_TAX_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_TAX_ADD_YOY`。 原始字段说明：营业税金及附加同比增长率（%） |
| SALE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SALE_EXPENSE_YOY`。 原始字段说明：销售费用同比增长率（%） |
| MANAGE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MANAGE_EXPENSE_YOY`。 原始字段说明：管理费用同比增长率（%） |
| ME_RESEARCH_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ME_RESEARCH_EXPENSE_YOY`。 原始字段说明：管理费用中的研发费用同比增长率（%） |
| FINANCE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_EXPENSE_YOY`。 原始字段说明：财务费用同比增长率（%） |
| FE_INTEREST_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_EXPENSE_YOY`。 原始字段说明：财务费用之利息费用同比增长率（%） |
| FE_INTEREST_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_INCOME_YOY`。 原始字段说明：财务费用之利息收入同比增长率（%） |
| ASSET_IMPAIRMENT_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_LOSS_YOY`。 原始字段说明：资产减值损失同比增长率（%） |
| CREDIT_IMPAIRMENT_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_LOSS_YOY`。 原始字段说明：信用减值损失同比增长率（%） |
| OTHER_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_INCOME_YOY`。 原始字段说明：其他收益同比增长率（%） |
| TOC_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOC_OTHER_YOY`。 原始字段说明：营业总成本其他同比增长率（%） |
| INVEST_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INCOME_YOY`。 原始字段说明：投资收益同比增长率（%） |
| INVEST_JOINT_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_JOINT_INCOME_YOY`。 原始字段说明：对联营企业和合营企业的投资收益同比增长率（%） |
| ACF_END_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACF_END_INCOME_YOY`。 原始字段说明：持续经营终止经营净损益同比增长率（%） |
| EXCHANGE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EXCHANGE_INCOME_YOY`。 原始字段说明：汇兑收益同比增长率（%） |
| NET_EXPOSURE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_EXPOSURE_INCOME_YOY`。 原始字段说明：净敞口收益同比增长率（%） |
| FAIRVALUE_CHANGE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FAIRVALUE_CHANGE_INCOME_YOY`。 原始字段说明：公允价值变动收益同比增长率（%） |
| ASSET_DISPOSAL_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_DISPOSAL_INCOME_YOY`。 原始字段说明：资产处置收益同比增长率（%） |
| CREDIT_IMPAIRMENT_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_INCOME_YOY`。 原始字段说明：信用减值收益同比增长率（%） |
| ASSET_IMPAIRMENT_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_INCOME_YOY`。 原始字段说明：资产减值收益同比增长率（%） |
| OPERATE_PROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_YOY`。 原始字段说明：营业利润同比增长率（%） |
| NONBUSINESS_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_INCOME_YOY`。 原始字段说明：营业外收入同比增长率（%） |
| NONCURRENT_DISPOSAL_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_INCOME_YOY`。 原始字段说明：非流动资产处置净收益同比增长率（%） |
| NONBUSINESS_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_EXPENSE_YOY`。 原始字段说明：营业外支出同比增长率（%） |
| NONCURRENT_DISPOSAL_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_LOSS_YOY`。 原始字段说明：非流动资产处置净损失同比增长率（%） |
| OPERATE_PROFIT_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_OTHER_YOY`。 原始字段说明：营业利润其他同比增长率（%） |
| OPERATE_PROFIT_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_BALANCE_YOY`。 原始字段说明：营业利润平衡项同比增长率（%） |
| TOTAL_PROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT_YOY`。 原始字段说明：利润总额同比增长率（%） |
| EFFECT_TP_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TP_OTHER_YOY`。 原始字段说明：影响利润总额其他同比增长率（%） |
| TOTAL_PROFIT_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT_BALANCE_YOY`。 原始字段说明：利润总额平衡项同比增长率（%） |
| INCOME_TAX_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INCOME_TAX_YOY`。 原始字段说明：所得税费用同比增长率（%） |
| NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_YOY`。 原始字段说明：净利润同比增长率（%） |
| CONTINUED_NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTINUED_NETPROFIT_YOY`。 原始字段说明：持续经营净利润同比增长率（%） |
| DISCONTINUED_NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISCONTINUED_NETPROFIT_YOY`。 原始字段说明：终止经营净利润同比增长率（%） |
| NETPROFIT_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_OTHER_YOY`。 原始字段说明：净利润其他同比增长率（%） |
| NETPROFIT_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_BALANCE_YOY`。 原始字段说明：净利润平衡项同比增长率（%） |
| EFFECT_NETPROFIT_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_OTHER_YOY`。 原始字段说明：影响净利润其他同比增长率（%） |
| EFFECT_NETPROFIT_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_BALANCE_YOY`。 原始字段说明：净利润平衡项同比增长率（%） |
| UNCONFIRM_INVEST_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNCONFIRM_INVEST_LOSS_YOY`。 原始字段说明：未确认投资损失同比增长率（%） |
| MINORITY_INTEREST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_INTEREST_YOY`。 原始字段说明：少数股东损益同比增长率（%） |
| PARENT_NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_NETPROFIT_YOY`。 原始字段说明：归属于母公司股东的净利润同比增长率（%） |
| BASIC_EPS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BASIC_EPS_YOY`。 原始字段说明：基本每股收益（元/股）同比增长率（%） |
| DILUTED_EPS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DILUTED_EPS_YOY`。 原始字段说明：稀释每股收益（元/股）同比增长率（%） |
| UNABLE_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_YOY`。 原始字段说明：以后将重分类进损益的其他综合收益同比增长率（%） |
| CREDITRISK_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITRISK_FAIRVALUE_CHANGE_YOY`。 原始字段说明：信用风险引起的公允价值变动同比增长率（%） |
| OTHERRIGHT_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHERRIGHT_FAIRVALUE_CHANGE_YOY`。 原始字段说明：其他权益工具公允价值变动同比增长率（%） |
| SETUP_PROFIT_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SETUP_PROFIT_CHANGE_YOY`。 原始字段说明：重分类调整变动同比增长率（%） |
| RIGHTLAW_UNABLE_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_UNABLE_OCI_YOY`。 原始字段说明：权益法下不能重分类的其他综合收益同比增长率（%） |
| UNABLE_OCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_OTHER_YOY`。 原始字段说明：不能重分类其他综合收益其他同比增长率（%） |
| UNABLE_OCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_BALANCE_YOY`。 原始字段说明：不能重分类其他综合收益平衡项同比增长率（%） |
| ABLE_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_YOY`。 原始字段说明：以后将重分类进损益的其他综合收益（可重分类）同比增长率（%） |
| RIGHTLAW_ABLE_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_ABLE_OCI_YOY`。 原始字段说明：权益法下可重分类的其他综合收益同比增长率（%） |
| AFA_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AFA_FAIRVALUE_CHANGE_YOY`。 原始字段说明：可供出售金融资产公允价值变动同比增长率（%） |
| HMI_AFA_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HMI_AFA_YOY`。 原始字段说明：持有有待售资产公允价值变动同比增长率（%） |
| CASHFLOW_HEDGE_VALID_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CASHFLOW_HEDGE_VALID_YOY`。 原始字段说明：现金流量套期有效部分同比增长率（%） |
| CREDITOR_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_FAIRVALUE_CHANGE_YOY`。 原始字段说明：债权投资公允价值变动同比增长率（%） |
| CREDITOR_IMPAIRMENT_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_IMPAIRMENT_RESERVE_YOY`。 原始字段说明：债权投资减值准备同比增长率（%） |
| FINANCE_OCI_AMT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OCI_AMT_YOY`。 原始字段说明：金融资产重分类金额同比增长率（%） |
| CONVERT_DIFF_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_DIFF_YOY`。 原始字段说明：外币报表折算差额同比增长率（%） |
| ABLE_OCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_OTHER_YOY`。 原始字段说明：可重分类其他综合收益其他同比增长率（%） |
| ABLE_OCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_BALANCE_YOY`。 原始字段说明：可重分类其他综合收益平衡项同比增长率（%） |
| OCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_OTHER_YOY`。 原始字段说明：其他综合收益其他同比增长率（%） |
| OCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_BALANCE_YOY`。 原始字段说明：其他综合收益平衡项同比增长率（%） |
| OTHER_COMPRE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_COMPRE_INCOME_YOY`。 原始字段说明：其他综合收益总额同比增长率（%） |
| PARENT_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_YOY`。 原始字段说明：归母其他综合收益同比增长率（%） |
| MINORITY_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_OCI_YOY`。 原始字段说明：少数股东其他综合收益同比增长率（%） |
| PARENT_OCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_OTHER_YOY`。 原始字段说明：归母其他综合收益其他同比增长率（%） |
| PARENT_OCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_BALANCE_YOY`。 原始字段说明：归母其他综合收益平衡项同比增长率（%） |
| TOTAL_COMPRE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_COMPRE_INCOME_YOY`。 原始字段说明：综合收益总额同比增长率（%） |
| PARENT_TCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_TCI_YOY`。 原始字段说明：归母综合收益总额同比增长率（%） |
| MINORITY_TCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_TCI_YOY`。 原始字段说明：少数股东综合收益总额同比增长率（%） |
| EFFECT_TCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TCI_BALANCE_YOY`。 原始字段说明：综合收益总额平衡项同比增长率（%） |
| TCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_OTHER_YOY`。 原始字段说明：综合收益总额其他同比增长率（%） |
| TCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_BALANCE_YOY`。 原始字段说明：综合收益总额平衡项同比增长率（%） |
| PRECOMBINE_PROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_PROFIT_YOY`。 原始字段说明：合并前净损益同比增长率（%） |
| PRECOMBINE_TCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_TCI_YOY`。 原始字段说明：合并前综合收益总额同比增长率（%） |
| DEDUCT_PARENT_NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEDUCT_PARENT_NETPROFIT_YOY`。 原始字段说明：扣非归母净利润同比增长率（%） |

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

- 已画像字段：`TOTAL_OPERATE_INCOME`, `TOTAL_OPERATE_INCOME_QOQ`, `OPERATE_INCOME`, `OPERATE_INCOME_QOQ`, `INTEREST_INCOME`, `INTEREST_INCOME_QOQ`, `EARNED_PREMIUM`, `EARNED_PREMIUM_QOQ`
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
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:40:59  Running with dbt=1.11.11
21:40:59  Registered adapter: clickhouse=1.10.0
21:40:59  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:41:00  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:00
21:41:00  Concurrency: 1 threads (target='dev')
21:41:00
Previewing inline node:
| SECUCODE  | SECURITY_CODE | SECURITY_NAME_ABBR | ORG_CODE | ORG_TYPE | REPORT_DATE | ... |
| --------- | ------------- | ------------------ | -------- | -------- | ----------- | --- |
| 600665.SH | 600665        | 天地源                | 10003975 | 通用       |  1993-06-30 | ... |
| 600668.SH | 600668        | 尖峰集团               | 10003978 | 通用       |  1993-06-30 | ... |
| 000550.SZ | 000550        | 江铃汽车               | 10005496 | 通用       |  1993-12-31 | ... |
| 600861.SH | 600861        | 北京人力               | 10004305 | 通用       |  1994-06-30 | ... |
| 000541.SZ | 000541        | 佛山照明               | 10005487 | 通用       |  1993-12-31 | ... |
| 000056.SZ | 000056        | *ST皇庭              | 10004137 | 通用       |  1996-06-30 | ... |
| 000410.SZ | 000410        | 沈阳机床               | 10004178 | 通用       |  1996-06-30 | ... |
| 000411.SZ | 000411        | 英特集团               | 10564780 | 通用       |  1996-06-30 | ... |
| 000416.SZ | 000416        | *ST民控              | 10564772 | 通用       |  1996-06-30 | ... |
| 000428.SZ | 000428        | 华天酒店               | 10634779 | 通用       |  1996-06-30 | ... |
| 000592.SZ | 000592        | 平潭发展               | 10005533 | 通用       |  1995-12-31 | ... |
| 000603.SZ | 000603        | 盛达资源               | 10005544 | 通用       |  1996-06-30 | ... |
| 600719.SH | 600719        | 大连热电               | 10004029 | 通用       |  1996-06-30 | ... |
| 600723.SH | 600723        | 首商股份               | 10004033 | 通用       |  1996-06-30 | ... |
| 600745.SH | 600745        | *ST闻泰              | 10004055 | 通用       |  1996-06-30 | ... |
| 000417.SZ | 000417        | 合百集团               | 10564770 | 通用       |  1996-06-30 | ... |
| 000665.SZ | 000665        | 湖北广电               | 10005590 | 通用       |  1996-12-31 | ... |
| 000760.SZ | 000760        | 斯太退                | 10005668 | 通用       |  1997-06-30 | ... |
| 000777.SZ | 000777        | 中核科技               | 10005680 | 通用       |  1997-06-30 | ... |
| 000785.SZ | 000785        | 居然智家               | 10005686 | 通用       |  1997-06-30 | ... |
| 000793.SZ | 000793        | *ST华闻              | 10005694 | 通用       |  1997-06-30 | ... |
| 000799.SZ | 000799        | 酒鬼酒                | 10005699 | 通用       |  1997-06-30 | ... |
| 000816.SZ | 000816        | 智慧农业               | 10005713 | 通用       |  1997-06-30 | ... |
| 000868.SZ | 000868        | 安凯客车               | 10005747 | 通用       |  1997-06-30 | ... |
| 600098.SH | 600098        | 广州发展               | 10002322 | 通用       |  1997-06-30 | ... |
| 600108.SH | 600108        | 亚盛集团               | 10002332 | 通用       |  1997-06-30 | ... |
| 600783.SH | 600783        | 鲁信创投               | 10634820 | 通用       |  1996-12-31 | ... |
| 000629.SZ | 000629        | 钒钛股份               | 10005567 | 通用       |  1998-03-31 | ... |
| 600160.SH | 600160        | 巨化股份               | 10002375 | 通用       |  1998-06-30 | ... |
| 600196.SH | 600196        | 复星医药               | 10002407 | 通用       |  1998-06-30 | ... |
| 000951.SZ | 000951        | 中国重汽               | 10005808 | 通用       |  1999-06-30 | ... |
| 000952.SZ | 000952        | 广济药业               | 10005809 | 通用       |  1999-06-30 | ... |
| 600129.SH | 600129        | 太极集团               | 10002351 | 通用       |  1999-03-31 | ... |
| 600129.SH | 600129        | 太极集团               | 10002351 | 通用       |  1999-06-30 | ... |
| 600220.SH | 600220        | ST阳光               | 10002429 | 通用       |  1999-06-30 | ... |
| 600223.SH | 600223        | 福瑞达                | 10002432 | 通用       |  1999-06-30 | ... |
| 000003.SZ | 000003        | PT金田A              | 10004087 | 通用       |  2001-09-30 | ... |
| 000011.SZ | 000011        | 深物业A               | 10004095 | 通用       |  2001-03-31 | ... |
| 000017.SZ | 000017        | 深中华A               | 10004101 | 通用       |  2001-09-30 | ... |
| 000025.SZ | 000025        | 特力A                | 10004109 | 通用       |  2000-09-30 | ... |
| 000025.SZ | 000025        | 特力A                | 10004109 | 通用       |  2001-03-31 | ... |
| 000030.SZ | 000030        | 富奥股份               | 10634796 | 通用       |  2001-03-31 | ... |
| 000047.SZ | 000047        | ST中侨               | 10004130 | 通用       |  2000-09-30 | ... |
| 000047.SZ | 000047        | ST中侨               | 10004130 | 通用       |  2001-03-31 | ... |
| 000150.SZ | 000150        | *ST宜康              | 10634794 | 通用       |  2000-06-30 | ... |
| 000153.SZ | 000153        | 丰原药业               | 10004159 | 通用       |  2000-06-30 | ... |
| 000406.SZ | 000406        | 石油大明               | 10634762 | 通用       |  2001-09-30 | ... |
| 000411.SZ | 000411        | 英特集团               | 10564780 | 通用       |  2001-03-31 | ... |
| 000411.SZ | 000411        | 英特集团               | 10564780 | 通用       |  2001-09-30 | ... |
| 000515.SZ | 000515        | 攀渝钛业               | 10005461 | 通用       |  2001-03-31 | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:41:03  Running with dbt=1.11.11
21:41:04  Registered adapter: clickhouse=1.10.0
21:41:04  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:41:04  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:04
21:41:04  Concurrency: 1 threads (target='dev')
21:41:04
Previewing inline node:
| row_count |
| --------- |
|    279918 |
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
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:41:08  Running with dbt=1.11.11
21:41:08  Registered adapter: clickhouse=1.10.0
21:41:08  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:41:09  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:09
21:41:09  Concurrency: 1 threads (target='dev')
21:41:09
Previewing inline node:
| min_report_date | max_report_date | null_report_date | placeholder_repor... | min_report_date_name | max_report_date_name | ... |
| --------------- | --------------- | ---------------- | -------------------- | -------------------- | -------------------- | --- |
|      1993-06-30 |      2026-03-31 |                0 |                    0 | 1993二季度              | 2026一季度              | ... |
```

### 格式分布：SECUCODE

```sql
select
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:41:12  Running with dbt=1.11.11
21:41:13  Registered adapter: clickhouse=1.10.0
21:41:13  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:41:14  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:14
21:41:14  Concurrency: 1 threads (target='dev')
21:41:14
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|           279918 |             0 |            0 |             0 |    279918 |
```

### 格式分布：SECURITY_CODE

```sql
select
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:41:17  Running with dbt=1.11.11
21:41:17  Registered adapter: clickhouse=1.10.0
21:41:18  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:41:18  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:18
21:41:18  Concurrency: 1 threads (target='dev')
21:41:18
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |       279918 |             0 |    279918 |
```

### 格式分布：ORG_CODE

```sql
select
    countIf(match(toString(`ORG_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`ORG_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`ORG_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`ORG_CODE`) or toString(`ORG_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:41:22  Running with dbt=1.11.11
21:41:22  Registered adapter: clickhouse=1.10.0
21:41:22  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:41:23  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:23
21:41:23  Concurrency: 1 threads (target='dev')
21:41:23
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |    279918 |
```

### 格式分布：SECURITY_TYPE_CODE

```sql
select
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_TYPE_CODE`) or toString(`SECURITY_TYPE_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:41:26  Running with dbt=1.11.11
21:41:26  Registered adapter: clickhouse=1.10.0
21:41:27  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:41:27  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:27
21:41:27  Concurrency: 1 threads (target='dev')
21:41:27
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |    279918 |
```

### 高频取值：SECUCODE

```sql
select
    `SECUCODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
group by `SECUCODE`
order by row_count desc
```


结果（成功）：

```text
21:41:31  Running with dbt=1.11.11
21:41:31  Registered adapter: clickhouse=1.10.0
21:41:31  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:41:32  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:32
21:41:32  Concurrency: 1 threads (target='dev')
21:41:32
Previewing inline node:
| value     | row_count |
| --------- | --------- |
| 000025.SZ |       103 |
| 000553.SZ |       103 |
| 000411.SZ |       102 |
| 000592.SZ |       102 |
| 600399.SH |       101 |
| 000838.SZ |       101 |
| 600129.SH |       101 |
| 000733.SZ |       101 |
| 600010.SH |       101 |
| 600721.SH |       101 |
| 000990.SZ |       101 |
| 001696.SZ |       101 |
| 000822.SZ |       101 |
| 000011.SZ |       101 |
| 600148.SH |       101 |
| 600743.SH |       101 |
| 600834.SH |       101 |
| 000546.SZ |       101 |
| 000536.SZ |       101 |
| 000869.SZ |       101 |
```

### 高频取值：SECURITY_CODE

```sql
select
    `SECURITY_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
group by `SECURITY_CODE`
order by row_count desc
```


结果（成功）：

```text
21:41:35  Running with dbt=1.11.11
21:41:35  Registered adapter: clickhouse=1.10.0
21:41:36  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:41:36  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:36
21:41:36  Concurrency: 1 threads (target='dev')
21:41:36
Previewing inline node:
| value  | row_count |
| ------ | --------- |
| 000025 |       103 |
| 000553 |       103 |
| 000592 |       102 |
| 000411 |       102 |
| 000869 |       101 |
| 000990 |       101 |
| 600717 |       101 |
| 600081 |       101 |
| 000733 |       101 |
| 000858 |       101 |
| 001696 |       101 |
| 600721 |       101 |
| 600399 |       101 |
| 600834 |       101 |
| 600855 |       101 |
| 600178 |       101 |
| 000822 |       101 |
| 600227 |       101 |
| 000546 |       101 |
| 600743 |       101 |
```

### 高频取值：SECURITY_NAME_ABBR

```sql
select
    `SECURITY_NAME_ABBR` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
group by `SECURITY_NAME_ABBR`
order by row_count desc
```


结果（成功）：

```text
21:41:39  Running with dbt=1.11.11
21:41:40  Registered adapter: clickhouse=1.10.0
21:41:40  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:41:40  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:40
21:41:40  Concurrency: 1 threads (target='dev')
21:41:40
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 东方明珠  |       150 |
| 百联股份  |       135 |
| 特力A   |       103 |
| 安道麦A  |       103 |
| 平潭发展  |       102 |
| 英特集团  |       102 |
| 富奥股份  |       101 |
| 华映科技  |       101 |
| 振华科技  |       101 |
| *ST发展 |       101 |
| 百花医药  |       101 |
| 华远控股  |       101 |
| 诚志股份  |       101 |
| 深物业A  |       101 |
| 五粮液   |       101 |
| 天津港   |       101 |
| 长春一东  |       101 |
| 山东海化  |       101 |
| 东风科技  |       101 |
| 航天长峰  |       101 |
```

### 高频取值：ORG_CODE

```sql
select
    `ORG_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
group by `ORG_CODE`
order by row_count desc
```


结果（成功）：

```text
21:41:44  Running with dbt=1.11.11
21:41:44  Registered adapter: clickhouse=1.10.0
21:41:44  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:41:45  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:45
21:41:45  Concurrency: 1 threads (target='dev')
21:41:45
Previewing inline node:
| value    | row_count |
| -------- | --------- |
| 10004127 |       168 |
| 10004106 |       164 |
| 10116535 |       126 |
| 10004293 |       124 |
| 10004109 |       103 |
| 10005499 |       103 |
| 10005533 |       102 |
| 10564780 |       102 |
| 10005843 |       101 |
| 10002391 |       101 |
| 10004095 |       101 |
| 10005719 |       101 |
| 10004027 |       101 |
| 10002435 |       101 |
| 10005740 |       101 |
| 10002305 |       101 |
| 10005618 |       101 |
| 10005748 |       101 |
| 10002364 |       101 |
| 10002261 |       101 |
```

### 高频取值：ORG_TYPE

```sql
select
    `ORG_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
group by `ORG_TYPE`
order by row_count desc
```


结果（成功）：

```text
21:41:48  Running with dbt=1.11.11
21:41:49  Registered adapter: clickhouse=1.10.0
21:41:49  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:41:50  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:50
21:41:50  Concurrency: 1 threads (target='dev')
21:41:50
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 通用    |    279918 |
```

### 高频取值：REPORT_TYPE

```sql
select
    `REPORT_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
group by `REPORT_TYPE`
order by row_count desc
```


结果（成功）：

```text
21:41:53  Running with dbt=1.11.11
21:41:53  Registered adapter: clickhouse=1.10.0
21:41:54  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:41:54  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:54
21:41:54  Concurrency: 1 threads (target='dev')
21:41:54
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 一季度   |     73061 |
| 三季度   |     69893 |
| 四季度   |     69543 |
| 二季度   |     67421 |
```

### 数值范围：TOTAL_OPERATE_INCOME

```sql
select
    min(`TOTAL_OPERATE_INCOME`) as min_value,
    max(`TOTAL_OPERATE_INCOME`) as max_value,
    countIf(`TOTAL_OPERATE_INCOME` = 0) as zero_count,
    countIf(`TOTAL_OPERATE_INCOME` < 0) as negative_count,
    countIf(isNull(`TOTAL_OPERATE_INCOME`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:41:57  Running with dbt=1.11.11
21:41:58  Registered adapter: clickhouse=1.10.0
21:41:58  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:41:58  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:41:58
21:41:58  Concurrency: 1 threads (target='dev')
21:41:58
Previewing inline node:
|          min_value |       max_value | zero_count | negative_count | null_count | row_count |
| ------------------ | --------------- | ---------- | -------------- | ---------- | --------- |
| -38,213,612,634.31 | 876,259,000,000 |        282 |            526 |        736 |    279918 |
```

### 数值范围：TOTAL_OPERATE_INCOME_QOQ

```sql
select
    min(`TOTAL_OPERATE_INCOME_QOQ`) as min_value,
    max(`TOTAL_OPERATE_INCOME_QOQ`) as max_value,
    countIf(`TOTAL_OPERATE_INCOME_QOQ` = 0) as zero_count,
    countIf(`TOTAL_OPERATE_INCOME_QOQ` < 0) as negative_count,
    countIf(isNull(`TOTAL_OPERATE_INCOME_QOQ`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:42:02  Running with dbt=1.11.11
21:42:02  Registered adapter: clickhouse=1.10.0
21:42:02  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:42:03  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:03
21:42:03  Concurrency: 1 threads (target='dev')
21:42:03
Previewing inline node:
|     min_value |      max_value | zero_count | negative_count | null_count | row_count |
| ------------- | -------------- | ---------- | -------------- | ---------- | --------- |
| -300,982.305… | 8,782,387.708… |         44 |         124346 |       3910 |    279918 |
```

### 数值范围：OPERATE_INCOME

```sql
select
    min(`OPERATE_INCOME`) as min_value,
    max(`OPERATE_INCOME`) as max_value,
    countIf(`OPERATE_INCOME` = 0) as zero_count,
    countIf(`OPERATE_INCOME` < 0) as negative_count,
    countIf(isNull(`OPERATE_INCOME`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:42:06  Running with dbt=1.11.11
21:42:07  Registered adapter: clickhouse=1.10.0
21:42:07  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:42:07  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:07
21:42:07  Concurrency: 1 threads (target='dev')
21:42:07
Previewing inline node:
|          min_value |       max_value | zero_count | negative_count | null_count | row_count |
| ------------------ | --------------- | ---------- | -------------- | ---------- | --------- |
| -38,213,612,634.31 | 876,259,000,000 |        215 |            527 |        848 |    279918 |
```

### 数值范围：OPERATE_INCOME_QOQ

```sql
select
    min(`OPERATE_INCOME_QOQ`) as min_value,
    max(`OPERATE_INCOME_QOQ`) as max_value,
    countIf(`OPERATE_INCOME_QOQ` = 0) as zero_count,
    countIf(`OPERATE_INCOME_QOQ` < 0) as negative_count,
    countIf(isNull(`OPERATE_INCOME_QOQ`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:42:11  Running with dbt=1.11.11
21:42:11  Registered adapter: clickhouse=1.10.0
21:42:11  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:42:12  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:12
21:42:12  Concurrency: 1 threads (target='dev')
21:42:12
Previewing inline node:
|     min_value |      max_value | zero_count | negative_count | null_count | row_count |
| ------------- | -------------- | ---------- | -------------- | ---------- | --------- |
| -300,982.305… | 8,782,387.708… |         44 |         124117 |       4332 |    279918 |
```

### 数值范围：INTEREST_INCOME

```sql
select
    min(`INTEREST_INCOME`) as min_value,
    max(`INTEREST_INCOME`) as max_value,
    countIf(`INTEREST_INCOME` = 0) as zero_count,
    countIf(`INTEREST_INCOME` < 0) as negative_count,
    countIf(isNull(`INTEREST_INCOME`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:42:15  Running with dbt=1.11.11
21:42:16  Registered adapter: clickhouse=1.10.0
21:42:16  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:42:16  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:16
21:42:16  Concurrency: 1 threads (target='dev')
21:42:16
Previewing inline node:
|       min_value |        max_value | zero_count | negative_count | null_count | row_count |
| --------------- | ---------------- | ---------- | -------------- | ---------- | --------- |
| -530,423,733.81 | 9,685,389,891.64 |       1820 |             75 |     273279 |    279918 |
```

### 数值范围：INTEREST_INCOME_QOQ

```sql
select
    min(`INTEREST_INCOME_QOQ`) as min_value,
    max(`INTEREST_INCOME_QOQ`) as max_value,
    countIf(`INTEREST_INCOME_QOQ` = 0) as zero_count,
    countIf(`INTEREST_INCOME_QOQ` < 0) as negative_count,
    countIf(isNull(`INTEREST_INCOME_QOQ`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:42:20  Running with dbt=1.11.11
21:42:20  Registered adapter: clickhouse=1.10.0
21:42:20  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:42:21  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:21
21:42:21  Concurrency: 1 threads (target='dev')
21:42:21
Previewing inline node:
| min_value |    max_value | zero_count | negative_count | null_count | row_count |
| --------- | ------------ | ---------- | -------------- | ---------- | --------- |
| -653.824… | 254,402.011… |          9 |           2286 |     275329 |    279918 |
```

### 数值范围：EARNED_PREMIUM

```sql
select
    min(`EARNED_PREMIUM`) as min_value,
    max(`EARNED_PREMIUM`) as max_value,
    countIf(`EARNED_PREMIUM` = 0) as zero_count,
    countIf(`EARNED_PREMIUM` < 0) as negative_count,
    countIf(isNull(`EARNED_PREMIUM`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:42:24  Running with dbt=1.11.11
21:42:25  Registered adapter: clickhouse=1.10.0
21:42:25  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:42:25  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:25
21:42:25  Concurrency: 1 threads (target='dev')
21:42:25
Previewing inline node:
|         min_value |        max_value | zero_count | negative_count | null_count | row_count |
| ----------------- | ---------------- | ---------- | -------------- | ---------- | --------- |
| -1,333,875,904.48 | 8,426,693,895.32 |       1868 |             13 |     277565 |    279918 |
```

### 数值范围：EARNED_PREMIUM_QOQ

```sql
select
    min(`EARNED_PREMIUM_QOQ`) as min_value,
    max(`EARNED_PREMIUM_QOQ`) as max_value,
    countIf(`EARNED_PREMIUM_QOQ` = 0) as zero_count,
    countIf(`EARNED_PREMIUM_QOQ` < 0) as negative_count,
    countIf(isNull(`EARNED_PREMIUM_QOQ`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_sq') }}
```


结果（成功）：

```text
21:42:29  Running with dbt=1.11.11
21:42:29  Registered adapter: clickhouse=1.10.0
21:42:29  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:42:30  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:30
21:42:30  Concurrency: 1 threads (target='dev')
21:42:30
Previewing inline node:
| min_value |  max_value | zero_count | negative_count | null_count | row_count |
| --------- | ---------- | ---------- | -------------- | ---------- | --------- |
| -295.300… | 7,315.866… |          0 |            217 |     279456 |    279918 |
```
