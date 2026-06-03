# Raw 数据画像：eastmoney__income_ytd

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__income_ytd.yml`
- dbt source：`source('raw', 'eastmoney__income_ytd')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__income_ytd`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__income_ytd --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__income_ytd.md`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`eastmoney__income_ytd`
- ClickHouse raw 表：`fleur_raw.eastmoney__income_ytd`
- 表说明：EastMoney year-to-date income F10 rows by natural-year raw partition.

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
| CURRENCY | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENCY`。 原始字段说明：利润表年初至报告期末金额使用的币种。 |
| TOTAL_OPERATE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_INCOME`。 原始字段说明：营业总收入 |
| TOTAL_OPERATE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_INCOME_YOY`。 原始字段说明：营业总收入同比增长率（%） |
| OPERATE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INCOME`。 原始字段说明：营业收入 |
| OPERATE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INCOME_YOY`。 原始字段说明：营业收入同比增长率（%） |
| INTEREST_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_INCOME`。 原始字段说明：利息收入 |
| INTEREST_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_INCOME_YOY`。 原始字段说明：利息收入同比增长率（%） |
| EARNED_PREMIUM | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EARNED_PREMIUM`。 原始字段说明：已赚保费 |
| EARNED_PREMIUM_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EARNED_PREMIUM_YOY`。 原始字段说明：已赚保费同比增长率（%） |
| FEE_COMMISSION_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_INCOME`。 原始字段说明：手续费及佣金收入 |
| FEE_COMMISSION_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_INCOME_YOY`。 原始字段说明：手续费及佣金收入同比增长率（%） |
| OTHER_BUSINESS_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_INCOME`。 原始字段说明：其他业务收入 |
| OTHER_BUSINESS_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_INCOME_YOY`。 原始字段说明：其他业务收入同比增长率（%） |
| TOI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOI_OTHER`。 原始字段说明：营业总收入其他 |
| TOI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOI_OTHER_YOY`。 原始字段说明：营业总收入其他同比增长率（%） |
| TOTAL_OPERATE_COST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_COST`。 原始字段说明：营业总成本 |
| TOTAL_OPERATE_COST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_COST_YOY`。 原始字段说明：营业总成本同比增长率（%） |
| OPERATE_COST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_COST`。 原始字段说明：营业成本 |
| OPERATE_COST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_COST_YOY`。 原始字段说明：营业成本同比增长率（%） |
| INTEREST_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_EXPENSE`。 原始字段说明：利息支出 |
| INTEREST_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INTEREST_EXPENSE_YOY`。 原始字段说明：利息支出同比增长率（%） |
| FEE_COMMISSION_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_EXPENSE`。 原始字段说明：手续费及佣金支出 |
| FEE_COMMISSION_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FEE_COMMISSION_EXPENSE_YOY`。 原始字段说明：手续费及佣金支出同比增长率（%） |
| RESEARCH_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RESEARCH_EXPENSE`。 原始字段说明：研发费用 |
| RESEARCH_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RESEARCH_EXPENSE_YOY`。 原始字段说明：研发费用同比增长率（%） |
| SURRENDER_VALUE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SURRENDER_VALUE`。 原始字段说明：退保金 |
| SURRENDER_VALUE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SURRENDER_VALUE_YOY`。 原始字段说明：退保金同比增长率（%） |
| NET_COMPENSATE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_COMPENSATE_EXPENSE`。 原始字段说明：分保费用 |
| NET_COMPENSATE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_COMPENSATE_EXPENSE_YOY`。 原始字段说明：分保费用同比增长率（%） |
| NET_CONTRACT_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_CONTRACT_RESERVE`。 原始字段说明：提取保险合同准备金 |
| NET_CONTRACT_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_CONTRACT_RESERVE_YOY`。 原始字段说明：提取保险合同准备金同比增长率（%） |
| POLICY_BONUS_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `POLICY_BONUS_EXPENSE`。 原始字段说明：保单红利支出 |
| POLICY_BONUS_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `POLICY_BONUS_EXPENSE_YOY`。 原始字段说明：保单红利支出同比增长率（%） |
| REINSURE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_EXPENSE`。 原始字段说明：分保费用支出 |
| REINSURE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REINSURE_EXPENSE_YOY`。 原始字段说明：分保费用支出同比增长率（%） |
| OTHER_BUSINESS_COST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_COST`。 原始字段说明：其他业务成本 |
| OTHER_BUSINESS_COST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_BUSINESS_COST_YOY`。 原始字段说明：其他业务成本同比增长率（%） |
| OPERATE_TAX_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_TAX_ADD`。 原始字段说明：营业税金及附加 |
| OPERATE_TAX_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_TAX_ADD_YOY`。 原始字段说明：营业税金及附加同比增长率（%） |
| SALE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SALE_EXPENSE`。 原始字段说明：销售费用 |
| SALE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SALE_EXPENSE_YOY`。 原始字段说明：销售费用同比增长率（%） |
| MANAGE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MANAGE_EXPENSE`。 原始字段说明：管理费用 |
| MANAGE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MANAGE_EXPENSE_YOY`。 原始字段说明：管理费用同比增长率（%） |
| ME_RESEARCH_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ME_RESEARCH_EXPENSE`。 原始字段说明：管理费用中的研发费用 |
| ME_RESEARCH_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ME_RESEARCH_EXPENSE_YOY`。 原始字段说明：管理费用中的研发费用同比增长率（%） |
| FINANCE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_EXPENSE`。 原始字段说明：财务费用 |
| FINANCE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_EXPENSE_YOY`。 原始字段说明：财务费用同比增长率（%） |
| FE_INTEREST_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_EXPENSE`。 原始字段说明：财务费用之利息费用 |
| FE_INTEREST_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_EXPENSE_YOY`。 原始字段说明：财务费用之利息费用同比增长率（%） |
| FE_INTEREST_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_INCOME`。 原始字段说明：财务费用之利息收入 |
| FE_INTEREST_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FE_INTEREST_INCOME_YOY`。 原始字段说明：财务费用之利息收入同比增长率（%） |
| ASSET_IMPAIRMENT_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_LOSS`。 原始字段说明：资产减值损失 |
| ASSET_IMPAIRMENT_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_LOSS_YOY`。 原始字段说明：资产减值损失同比增长率（%） |
| CREDIT_IMPAIRMENT_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_LOSS`。 原始字段说明：信用减值损失 |
| CREDIT_IMPAIRMENT_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_LOSS_YOY`。 原始字段说明：信用减值损失同比增长率（%） |
| TOC_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOC_OTHER`。 原始字段说明：营业总成本其他 |
| TOC_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOC_OTHER_YOY`。 原始字段说明：营业总成本其他同比增长率（%） |
| FAIRVALUE_CHANGE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FAIRVALUE_CHANGE_INCOME`。 原始字段说明：公允价值变动收益 |
| FAIRVALUE_CHANGE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FAIRVALUE_CHANGE_INCOME_YOY`。 原始字段说明：公允价值变动收益同比增长率（%） |
| INVEST_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INCOME`。 原始字段说明：投资收益 |
| INVEST_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INCOME_YOY`。 原始字段说明：投资收益同比增长率（%） |
| INVEST_JOINT_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_JOINT_INCOME`。 原始字段说明：对联营企业和合营企业的投资收益 |
| INVEST_JOINT_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_JOINT_INCOME_YOY`。 原始字段说明：对联营企业和合营企业的投资收益同比增长率（%） |
| NET_EXPOSURE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_EXPOSURE_INCOME`。 原始字段说明：净敞口收益 |
| NET_EXPOSURE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NET_EXPOSURE_INCOME_YOY`。 原始字段说明：净敞口收益同比增长率（%） |
| EXCHANGE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EXCHANGE_INCOME`。 原始字段说明：汇兑收益 |
| EXCHANGE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EXCHANGE_INCOME_YOY`。 原始字段说明：汇兑收益同比增长率（%） |
| ASSET_DISPOSAL_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_DISPOSAL_INCOME`。 原始字段说明：资产处置收益 |
| ASSET_DISPOSAL_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_DISPOSAL_INCOME_YOY`。 原始字段说明：资产处置收益同比增长率（%） |
| ASSET_IMPAIRMENT_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_INCOME`。 原始字段说明：资产减值收益 |
| ASSET_IMPAIRMENT_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_INCOME_YOY`。 原始字段说明：资产减值收益同比增长率（%） |
| CREDIT_IMPAIRMENT_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_INCOME`。 原始字段说明：信用减值收益 |
| CREDIT_IMPAIRMENT_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDIT_IMPAIRMENT_INCOME_YOY`。 原始字段说明：信用减值收益同比增长率（%） |
| OTHER_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_INCOME`。 原始字段说明：其他收益 |
| OTHER_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_INCOME_YOY`。 原始字段说明：其他收益同比增长率（%） |
| OPERATE_PROFIT_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_OTHER`。 原始字段说明：营业利润其他 |
| OPERATE_PROFIT_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_OTHER_YOY`。 原始字段说明：营业利润其他同比增长率（%） |
| OPERATE_PROFIT_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_BALANCE`。 原始字段说明：营业利润平衡项 |
| OPERATE_PROFIT_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_BALANCE_YOY`。 原始字段说明：营业利润平衡项同比增长率（%） |
| OPERATE_PROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT`。 原始字段说明：营业利润 |
| OPERATE_PROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PROFIT_YOY`。 原始字段说明：营业利润同比增长率（%） |
| NONBUSINESS_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_INCOME`。 原始字段说明：营业外收入 |
| NONBUSINESS_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_INCOME_YOY`。 原始字段说明：营业外收入同比增长率（%） |
| NONCURRENT_DISPOSAL_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_INCOME`。 原始字段说明：非流动资产处置净收益 |
| NONCURRENT_DISPOSAL_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_INCOME_YOY`。 原始字段说明：非流动资产处置净收益同比增长率（%） |
| NONBUSINESS_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_EXPENSE`。 原始字段说明：营业外支出 |
| NONBUSINESS_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONBUSINESS_EXPENSE_YOY`。 原始字段说明：营业外支出同比增长率（%） |
| NONCURRENT_DISPOSAL_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_LOSS`。 原始字段说明：非流动资产处置净损失 |
| NONCURRENT_DISPOSAL_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NONCURRENT_DISPOSAL_LOSS_YOY`。 原始字段说明：非流动资产处置净损失同比增长率（%） |
| EFFECT_TP_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TP_OTHER`。 原始字段说明：影响利润总额其他 |
| EFFECT_TP_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TP_OTHER_YOY`。 原始字段说明：影响利润总额其他同比增长率（%） |
| TOTAL_PROFIT_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT_BALANCE`。 原始字段说明：利润总额平衡项 |
| TOTAL_PROFIT_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT_BALANCE_YOY`。 原始字段说明：利润总额平衡项同比增长率（%） |
| TOTAL_PROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT`。 原始字段说明：利润总额 |
| TOTAL_PROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_PROFIT_YOY`。 原始字段说明：利润总额同比增长率（%） |
| INCOME_TAX | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INCOME_TAX`。 原始字段说明：所得税费用 |
| INCOME_TAX_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INCOME_TAX_YOY`。 原始字段说明：所得税费用同比增长率（%） |
| EFFECT_NETPROFIT_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_OTHER`。 原始字段说明：影响净利润其他 |
| EFFECT_NETPROFIT_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_OTHER_YOY`。 原始字段说明：影响净利润其他同比增长率（%） |
| EFFECT_NETPROFIT_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_BALANCE`。 原始字段说明：净利润平衡项 |
| EFFECT_NETPROFIT_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_NETPROFIT_BALANCE_YOY`。 原始字段说明：净利润平衡项同比增长率（%） |
| UNCONFIRM_INVEST_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNCONFIRM_INVEST_LOSS`。 原始字段说明：未确认投资损失 |
| UNCONFIRM_INVEST_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNCONFIRM_INVEST_LOSS_YOY`。 原始字段说明：未确认投资损失同比增长率（%） |
| NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT`。 原始字段说明：净利润 |
| NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_YOY`。 原始字段说明：净利润同比增长率（%） |
| PRECOMBINE_PROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_PROFIT`。 原始字段说明：合并前净损益 |
| PRECOMBINE_PROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_PROFIT_YOY`。 原始字段说明：合并前净损益同比增长率（%） |
| CONTINUED_NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTINUED_NETPROFIT`。 原始字段说明：持续经营净利润 |
| CONTINUED_NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONTINUED_NETPROFIT_YOY`。 原始字段说明：持续经营净利润同比增长率（%） |
| DISCONTINUED_NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISCONTINUED_NETPROFIT`。 原始字段说明：终止经营净利润 |
| DISCONTINUED_NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISCONTINUED_NETPROFIT_YOY`。 原始字段说明：终止经营净利润同比增长率（%） |
| PARENT_NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_NETPROFIT`。 原始字段说明：归属于母公司股东的净利润 |
| PARENT_NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_NETPROFIT_YOY`。 原始字段说明：归属于母公司股东的净利润同比增长率（%） |
| MINORITY_INTEREST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_INTEREST`。 原始字段说明：少数股东损益 |
| MINORITY_INTEREST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_INTEREST_YOY`。 原始字段说明：少数股东损益同比增长率（%） |
| DEDUCT_PARENT_NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEDUCT_PARENT_NETPROFIT`。 原始字段说明：扣除非经常性损益后归属于母公司股东的净利润 |
| DEDUCT_PARENT_NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEDUCT_PARENT_NETPROFIT_YOY`。 原始字段说明：扣非归母净利润同比增长率（%） |
| NETPROFIT_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_OTHER`。 原始字段说明：净利润其他 |
| NETPROFIT_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_OTHER_YOY`。 原始字段说明：净利润其他同比增长率（%） |
| NETPROFIT_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_BALANCE`。 原始字段说明：净利润平衡项 |
| NETPROFIT_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_BALANCE_YOY`。 原始字段说明：净利润平衡项同比增长率（%） |
| BASIC_EPS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BASIC_EPS`。 原始字段说明：基本每股收益（元/股） |
| BASIC_EPS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BASIC_EPS_YOY`。 原始字段说明：基本每股收益（元/股）同比增长率（%） |
| DILUTED_EPS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DILUTED_EPS`。 原始字段说明：稀释每股收益（元/股） |
| DILUTED_EPS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DILUTED_EPS_YOY`。 原始字段说明：稀释每股收益（元/股）同比增长率（%） |
| OTHER_COMPRE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_COMPRE_INCOME`。 原始字段说明：其他综合收益总额 |
| OTHER_COMPRE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_COMPRE_INCOME_YOY`。 原始字段说明：其他综合收益总额同比增长率（%） |
| PARENT_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI`。 原始字段说明：归属于母公司股东的其他综合收益 |
| PARENT_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_YOY`。 原始字段说明：归母其他综合收益同比增长率（%） |
| MINORITY_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_OCI`。 原始字段说明：归属于少数股东的其他综合收益 |
| MINORITY_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_OCI_YOY`。 原始字段说明：少数股东其他综合收益同比增长率（%） |
| PARENT_OCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_OTHER`。 原始字段说明：归母其他综合收益其他 |
| PARENT_OCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_OTHER_YOY`。 原始字段说明：归母其他综合收益其他同比增长率（%） |
| PARENT_OCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_BALANCE`。 原始字段说明：归母其他综合收益平衡项 |
| PARENT_OCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_OCI_BALANCE_YOY`。 原始字段说明：归母其他综合收益平衡项同比增长率（%） |
| UNABLE_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI`。 原始字段说明：以后将重分类进损益的其他综合收益 |
| UNABLE_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_YOY`。 原始字段说明：以后将重分类进损益的其他综合收益同比增长率（%） |
| CREDITRISK_FAIRVALUE_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITRISK_FAIRVALUE_CHANGE`。 原始字段说明：信用风险引起的公允价值变动 |
| CREDITRISK_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITRISK_FAIRVALUE_CHANGE_YOY`。 原始字段说明：信用风险引起的公允价值变动同比增长率（%） |
| OTHERRIGHT_FAIRVALUE_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHERRIGHT_FAIRVALUE_CHANGE`。 原始字段说明：其他权益工具公允价值变动 |
| OTHERRIGHT_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHERRIGHT_FAIRVALUE_CHANGE_YOY`。 原始字段说明：其他权益工具公允价值变动同比增长率（%） |
| SETUP_PROFIT_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SETUP_PROFIT_CHANGE`。 原始字段说明：重分类调整变动 |
| SETUP_PROFIT_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SETUP_PROFIT_CHANGE_YOY`。 原始字段说明：重分类调整变动同比增长率（%） |
| RIGHTLAW_UNABLE_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_UNABLE_OCI`。 原始字段说明：权益法下不能重分类的其他综合收益 |
| RIGHTLAW_UNABLE_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_UNABLE_OCI_YOY`。 原始字段说明：权益法下不能重分类的其他综合收益同比增长率（%） |
| UNABLE_OCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_OTHER`。 原始字段说明：不能重分类其他综合收益其他 |
| UNABLE_OCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_OTHER_YOY`。 原始字段说明：不能重分类其他综合收益其他同比增长率（%） |
| UNABLE_OCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_BALANCE`。 原始字段说明：不能重分类其他综合收益平衡项 |
| UNABLE_OCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNABLE_OCI_BALANCE_YOY`。 原始字段说明：不能重分类其他综合收益平衡项同比增长率（%） |
| ABLE_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI`。 原始字段说明：以后将重分类进损益的其他综合收益（可重分类） |
| ABLE_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_YOY`。 原始字段说明：以后将重分类进损益的其他综合收益（可重分类）同比增长率（%） |
| RIGHTLAW_ABLE_OCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_ABLE_OCI`。 原始字段说明：权益法下可重分类的其他综合收益 |
| RIGHTLAW_ABLE_OCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RIGHTLAW_ABLE_OCI_YOY`。 原始字段说明：权益法下可重分类的其他综合收益同比增长率（%） |
| AFA_FAIRVALUE_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AFA_FAIRVALUE_CHANGE`。 原始字段说明：可供出售金融资产公允价值变动 |
| AFA_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `AFA_FAIRVALUE_CHANGE_YOY`。 原始字段说明：可供出售金融资产公允价值变动同比增长率（%） |
| HMI_AFA | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HMI_AFA`。 原始字段说明：持有有待售资产公允价值变动 |
| HMI_AFA_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `HMI_AFA_YOY`。 原始字段说明：持有有待售资产公允价值变动同比增长率（%） |
| CASHFLOW_HEDGE_VALID | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CASHFLOW_HEDGE_VALID`。 原始字段说明：现金流量套期有效部分 |
| CASHFLOW_HEDGE_VALID_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CASHFLOW_HEDGE_VALID_YOY`。 原始字段说明：现金流量套期有效部分同比增长率（%） |
| CREDITOR_FAIRVALUE_CHANGE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_FAIRVALUE_CHANGE`。 原始字段说明：债权投资公允价值变动 |
| CREDITOR_FAIRVALUE_CHANGE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_FAIRVALUE_CHANGE_YOY`。 原始字段说明：债权投资公允价值变动同比增长率（%） |
| CREDITOR_IMPAIRMENT_RESERVE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_IMPAIRMENT_RESERVE`。 原始字段说明：债权投资减值准备 |
| CREDITOR_IMPAIRMENT_RESERVE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CREDITOR_IMPAIRMENT_RESERVE_YOY`。 原始字段说明：债权投资减值准备同比增长率（%） |
| FINANCE_OCI_AMT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OCI_AMT`。 原始字段说明：金融资产重分类金额 |
| FINANCE_OCI_AMT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OCI_AMT_YOY`。 原始字段说明：金融资产重分类金额同比增长率（%） |
| CONVERT_DIFF | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_DIFF`。 原始字段说明：外币报表折算差额 |
| CONVERT_DIFF_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_DIFF_YOY`。 原始字段说明：外币报表折算差额同比增长率（%） |
| ABLE_OCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_OTHER`。 原始字段说明：可重分类其他综合收益其他 |
| ABLE_OCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_OTHER_YOY`。 原始字段说明：可重分类其他综合收益其他同比增长率（%） |
| ABLE_OCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_BALANCE`。 原始字段说明：可重分类其他综合收益平衡项 |
| ABLE_OCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ABLE_OCI_BALANCE_YOY`。 原始字段说明：可重分类其他综合收益平衡项同比增长率（%） |
| OCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_OTHER`。 原始字段说明：其他综合收益其他 |
| OCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_OTHER_YOY`。 原始字段说明：其他综合收益其他同比增长率（%） |
| OCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_BALANCE`。 原始字段说明：其他综合收益平衡项 |
| OCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OCI_BALANCE_YOY`。 原始字段说明：其他综合收益平衡项同比增长率（%） |
| TOTAL_COMPRE_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_COMPRE_INCOME`。 原始字段说明：综合收益总额 |
| TOTAL_COMPRE_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_COMPRE_INCOME_YOY`。 原始字段说明：综合收益总额同比增长率（%） |
| PARENT_TCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_TCI`。 原始字段说明：归属于母公司股东的综合收益总额 |
| PARENT_TCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PARENT_TCI_YOY`。 原始字段说明：归母综合收益总额同比增长率（%） |
| MINORITY_TCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_TCI`。 原始字段说明：归属于少数股东的综合收益总额 |
| MINORITY_TCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_TCI_YOY`。 原始字段说明：少数股东综合收益总额同比增长率（%） |
| PRECOMBINE_TCI | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_TCI`。 原始字段说明：合并前综合收益总额 |
| PRECOMBINE_TCI_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PRECOMBINE_TCI_YOY`。 原始字段说明：合并前综合收益总额同比增长率（%） |
| EFFECT_TCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TCI_BALANCE`。 原始字段说明：综合收益总额平衡项 |
| EFFECT_TCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EFFECT_TCI_BALANCE_YOY`。 原始字段说明：综合收益总额平衡项同比增长率（%） |
| TCI_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_OTHER`。 原始字段说明：综合收益总额其他 |
| TCI_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_OTHER_YOY`。 原始字段说明：综合收益总额其他同比增长率（%） |
| TCI_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_BALANCE`。 原始字段说明：综合收益总额平衡项 |
| TCI_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TCI_BALANCE_YOY`。 原始字段说明：综合收益总额平衡项同比增长率（%） |
| ACF_END_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACF_END_INCOME`。 原始字段说明：持续经营终止经营净损益 |
| ACF_END_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACF_END_INCOME_YOY`。 原始字段说明：持续经营终止经营净损益同比增长率（%） |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPINION_TYPE`。 原始字段说明：审计意见类型 |

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

- 已画像字段：`TOTAL_OPERATE_INCOME`, `TOTAL_OPERATE_INCOME_YOY`, `OPERATE_INCOME`, `OPERATE_INCOME_YOY`, `INTEREST_INCOME`, `INTEREST_INCOME_YOY`, `EARNED_PREMIUM`, `EARNED_PREMIUM_YOY`
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
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:42:35  Running with dbt=1.11.11
21:42:35  Registered adapter: clickhouse=1.10.0
21:42:35  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:42:36  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:36
21:42:36  Concurrency: 1 threads (target='dev')
21:42:36
Previewing inline node:
| SECUCODE  | SECURITY_CODE | SECURITY_NAME_ABBR | ORG_CODE | ORG_TYPE | REPORT_DATE | ... |
| --------- | ------------- | ------------------ | -------- | -------- | ----------- | --- |
| 600601.SH | 600601        | 方正科技               | 10002659 | 通用       |  1990-06-30 | ... |
| 600601.SH | 600601        | 方正科技               | 10002659 | 通用       |  1991-06-30 | ... |
| 600602.SH | 600602        | 云赛智联               | 10002660 | 通用       |  1990-06-30 | ... |
| 600602.SH | 600602        | 云赛智联               | 10002660 | 通用       |  1991-06-30 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1990-06-30 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1991-06-30 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1988-12-31 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1989-12-31 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1990-06-30 | ... |
| 600652.SH | 600652        | 退市游久               | 10003962 | 通用       |  1991-06-30 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1988-12-31 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1989-12-31 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1990-06-30 | ... |
| 600653.SH | 600653        | 申华控股               | 10003963 | 通用       |  1991-06-30 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1988-12-31 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1989-12-31 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1990-06-30 | ... |
| 600654.SH | 600654        | 中安科                | 10003964 | 通用       |  1991-06-30 | ... |
| 600656.SH | 600656        | 退市博元               | 10003966 | 通用       |  1988-12-31 | ... |
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
| 600603.SH | 600603        | 广汇物流               | 10002661 | 通用       |  1992-06-30 | ... |
| 600605.SH | 600605        | 汇通能源               | 10002663 | 通用       |  1992-06-30 | ... |
| 600606.SH | 600606        | 绿地控股               | 10002664 | 通用       |  1992-06-30 | ... |
| 600608.SH | 600608        | *ST沪科              | 10002666 | 通用       |  1992-06-30 | ... |
| 600614.SH | 600614        | 退市鹏起               | 10003924 | 通用       |  1990-12-31 | ... |
| 600614.SH | 600614        | 退市鹏起               | 10003924 | 通用       |  1991-12-31 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1990-12-31 | ... |
| 600651.SH | 600651        | 飞乐音响               | 10003961 | 通用       |  1991-12-31 | ... |
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
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:42:39  Running with dbt=1.11.11
21:42:40  Registered adapter: clickhouse=1.10.0
21:42:40  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:42:40  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:40
21:42:40  Concurrency: 1 threads (target='dev')
21:42:40
Previewing inline node:
| row_count |
| --------- |
|    298396 |
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
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:42:44  Running with dbt=1.11.11
21:42:44  Registered adapter: clickhouse=1.10.0
21:42:44  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:42:45  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:45
21:42:45  Concurrency: 1 threads (target='dev')
21:42:45
Previewing inline node:
| min_report_date | max_report_date | null_report_date | placeholder_repor... | min_report_date_name | max_report_date_name | ... |
| --------------- | --------------- | ---------------- | -------------------- | -------------------- | -------------------- | --- |
|      1988-12-31 |      2026-03-31 |                0 |                    0 | 1988年报               | 2026一季报              | ... |
```

### 格式分布：SECUCODE

```sql
select
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:42:48  Running with dbt=1.11.11
21:42:48  Registered adapter: clickhouse=1.10.0
21:42:49  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:42:49  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:49
21:42:49  Concurrency: 1 threads (target='dev')
21:42:49
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|           298396 |             0 |            0 |             0 |    298396 |
```

### 格式分布：SECURITY_CODE

```sql
select
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:42:53  Running with dbt=1.11.11
21:42:53  Registered adapter: clickhouse=1.10.0
21:42:53  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:42:54  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:54
21:42:54  Concurrency: 1 threads (target='dev')
21:42:54
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |       298396 |             0 |    298396 |
```

### 格式分布：ORG_CODE

```sql
select
    countIf(match(toString(`ORG_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`ORG_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`ORG_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`ORG_CODE`) or toString(`ORG_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:42:57  Running with dbt=1.11.11
21:42:57  Registered adapter: clickhouse=1.10.0
21:42:58  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:42:58  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:42:58
21:42:58  Concurrency: 1 threads (target='dev')
21:42:58
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |    298396 |
```

### 格式分布：SECURITY_TYPE_CODE

```sql
select
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_TYPE_CODE`) or toString(`SECURITY_TYPE_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:43:02  Running with dbt=1.11.11
21:43:02  Registered adapter: clickhouse=1.10.0
21:43:02  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:43:03  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:03
21:43:03  Concurrency: 1 threads (target='dev')
21:43:03
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |    298396 |
```

### 高频取值：SECUCODE

```sql
select
    `SECUCODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
group by `SECUCODE`
order by row_count desc
```


结果（成功）：

```text
21:43:06  Running with dbt=1.11.11
21:43:06  Registered adapter: clickhouse=1.10.0
21:43:07  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:43:07  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:07
21:43:07  Concurrency: 1 threads (target='dev')
21:43:07
Previewing inline node:
| value     | row_count |
| --------- | --------- |
| 600653.SH |       123 |
| 600654.SH |       123 |
| 600601.SH |       121 |
| 600651.SH |       121 |
| 000030.SZ |       120 |
| 600610.SH |       120 |
| 000028.SZ |       119 |
| 000501.SZ |       119 |
| 600603.SH |       119 |
| 000029.SZ |       119 |
| 600602.SH |       119 |
| 000021.SZ |       118 |
| 000510.SZ |       118 |
| 000006.SZ |       118 |
| 000563.SZ |       118 |
| 600608.SH |       118 |
| 000553.SZ |       118 |
| 000007.SZ |       118 |
| 000025.SZ |       118 |
| 000014.SZ |       117 |
```

### 高频取值：SECURITY_CODE

```sql
select
    `SECURITY_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
group by `SECURITY_CODE`
order by row_count desc
```


结果（成功）：

```text
21:43:11  Running with dbt=1.11.11
21:43:11  Registered adapter: clickhouse=1.10.0
21:43:11  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:43:12  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:12
21:43:12  Concurrency: 1 threads (target='dev')
21:43:12
Previewing inline node:
| value  | row_count |
| ------ | --------- |
| 600653 |       123 |
| 600654 |       123 |
| 600601 |       121 |
| 600651 |       121 |
| 600610 |       120 |
| 000030 |       120 |
| 000501 |       119 |
| 000029 |       119 |
| 600602 |       119 |
| 000028 |       119 |
| 600603 |       119 |
| 600608 |       118 |
| 000510 |       118 |
| 000025 |       118 |
| 000563 |       118 |
| 000007 |       118 |
| 000553 |       118 |
| 000021 |       118 |
| 000006 |       118 |
| 000505 |       117 |
```

### 高频取值：SECURITY_NAME_ABBR

```sql
select
    `SECURITY_NAME_ABBR` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
group by `SECURITY_NAME_ABBR`
order by row_count desc
```


结果（成功）：

```text
21:43:15  Running with dbt=1.11.11
21:43:15  Registered adapter: clickhouse=1.10.0
21:43:16  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:43:16  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:16
21:43:16  Concurrency: 1 threads (target='dev')
21:43:16
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 东方明珠  |       184 |
| 百联股份  |       171 |
| 中安科   |       123 |
| 申华控股  |       123 |
| 飞乐音响  |       121 |
| 方正科技  |       121 |
| 中毅达   |       120 |
| 富奥股份  |       120 |
| 武商集团  |       119 |
| 国药一致  |       119 |
| 深深房A  |       119 |
| 广汇物流  |       119 |
| 云赛智联  |       119 |
| 陕国投A  |       118 |
| 新金路   |       118 |
| 特力A   |       118 |
| 深振业A  |       118 |
| 深科技   |       118 |
| 全新好   |       118 |
| 安道麦A  |       118 |
```

### 高频取值：ORG_CODE

```sql
select
    `ORG_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
group by `ORG_CODE`
order by row_count desc
```


结果（成功）：

```text
21:43:20  Running with dbt=1.11.11
21:43:20  Registered adapter: clickhouse=1.10.0
21:43:20  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:43:21  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:21
21:43:21  Concurrency: 1 threads (target='dev')
21:43:21
Previewing inline node:
| value    | row_count |
| -------- | --------- |
| 10004106 |       200 |
| 10004127 |       198 |
| 10004293 |       154 |
| 10116535 |       128 |
| 10003963 |       123 |
| 10003964 |       123 |
| 10002659 |       121 |
| 10003961 |       121 |
| 10002668 |       120 |
| 10634796 |       120 |
| 10002660 |       119 |
| 10004338 |       119 |
| 10634825 |       119 |
| 10634826 |       119 |
| 10002661 |       119 |
| 10004109 |       118 |
| 10002666 |       118 |
| 10004090 |       118 |
| 10004105 |       118 |
| 10005509 |       118 |
```

### 高频取值：ORG_TYPE

```sql
select
    `ORG_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
group by `ORG_TYPE`
order by row_count desc
```


结果（成功）：

```text
21:43:24  Running with dbt=1.11.11
21:43:24  Registered adapter: clickhouse=1.10.0
21:43:25  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:43:25  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:25
21:43:25  Concurrency: 1 threads (target='dev')
21:43:25
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 通用    |    292603 |
| 证券    |      2826 |
| 银行    |      2449 |
| 保险    |       518 |
```

### 高频取值：REPORT_TYPE

```sql
select
    `REPORT_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
group by `REPORT_TYPE`
order by row_count desc
```


结果（成功）：

```text
21:43:29  Running with dbt=1.11.11
21:43:29  Registered adapter: clickhouse=1.10.0
21:43:29  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:43:30  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:30
21:43:30  Concurrency: 1 threads (target='dev')
21:43:30
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 年报    |     77446 |
| 中报    |     74897 |
| 一季报   |     74512 |
| 三季报   |     71541 |
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
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:43:33  Running with dbt=1.11.11
21:43:33  Registered adapter: clickhouse=1.10.0
21:43:34  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:43:34  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:34
21:43:34  Concurrency: 1 threads (target='dev')
21:43:34
Previewing inline node:
|       min_value |         max_value | zero_count | negative_count | null_count | row_count |
| --------------- | ----------------- | ---------- | -------------- | ---------- | --------- |
| -673,495,950.57 | 3,318,168,000,000 |        223 |             34 |       1065 |    298396 |
```

### 数值范围：TOTAL_OPERATE_INCOME_YOY

```sql
select
    min(`TOTAL_OPERATE_INCOME_YOY`) as min_value,
    max(`TOTAL_OPERATE_INCOME_YOY`) as max_value,
    countIf(`TOTAL_OPERATE_INCOME_YOY` = 0) as zero_count,
    countIf(`TOTAL_OPERATE_INCOME_YOY` < 0) as negative_count,
    countIf(isNull(`TOTAL_OPERATE_INCOME_YOY`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:43:38  Running with dbt=1.11.11
21:43:38  Registered adapter: clickhouse=1.10.0
21:43:38  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:43:39  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:39
21:43:39  Concurrency: 1 threads (target='dev')
21:43:39
Previewing inline node:
| min_value |      max_value | zero_count | negative_count | null_count | row_count |
| --------- | -------------- | ---------- | -------------- | ---------- | --------- |
| -306.469… | 6,998,082.277… |         11 |          95980 |      11609 |    298396 |
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
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:43:42  Running with dbt=1.11.11
21:43:42  Registered adapter: clickhouse=1.10.0
21:43:43  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:43:43  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:43
21:43:43  Concurrency: 1 threads (target='dev')
21:43:43
Previewing inline node:
|       min_value |         max_value | zero_count | negative_count | null_count | row_count |
| --------------- | ----------------- | ---------- | -------------- | ---------- | --------- |
| -673,495,950.57 | 3,318,168,000,000 |        163 |             34 |       1173 |    298396 |
```

### 数值范围：OPERATE_INCOME_YOY

```sql
select
    min(`OPERATE_INCOME_YOY`) as min_value,
    max(`OPERATE_INCOME_YOY`) as max_value,
    countIf(`OPERATE_INCOME_YOY` = 0) as zero_count,
    countIf(`OPERATE_INCOME_YOY` < 0) as negative_count,
    countIf(isNull(`OPERATE_INCOME_YOY`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:43:47  Running with dbt=1.11.11
21:43:47  Registered adapter: clickhouse=1.10.0
21:43:47  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:43:48  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:48
21:43:48  Concurrency: 1 threads (target='dev')
21:43:48
Previewing inline node:
| min_value |      max_value | zero_count | negative_count | null_count | row_count |
| --------- | -------------- | ---------- | -------------- | ---------- | --------- |
| -306.469… | 6,998,082.277… |         14 |          95914 |      12034 |    298396 |
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
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:43:51  Running with dbt=1.11.11
21:43:51  Registered adapter: clickhouse=1.10.0
21:43:52  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:43:52  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:52
21:43:52  Concurrency: 1 threads (target='dev')
21:43:52
Previewing inline node:
|       min_value |         max_value | zero_count | negative_count | null_count | row_count |
| --------------- | ----------------- | ---------- | -------------- | ---------- | --------- |
| -116,120,513.42 | 1,427,948,000,000 |       2537 |              8 |     286898 |    298396 |
```

### 数值范围：INTEREST_INCOME_YOY

```sql
select
    min(`INTEREST_INCOME_YOY`) as min_value,
    max(`INTEREST_INCOME_YOY`) as max_value,
    countIf(`INTEREST_INCOME_YOY` = 0) as zero_count,
    countIf(`INTEREST_INCOME_YOY` < 0) as negative_count,
    countIf(isNull(`INTEREST_INCOME_YOY`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:43:56  Running with dbt=1.11.11
21:43:56  Registered adapter: clickhouse=1.10.0
21:43:56  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:43:57  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:43:57
21:43:57  Concurrency: 1 threads (target='dev')
21:43:57
Previewing inline node:
|   min_value |        max_value | zero_count | negative_count | null_count | row_count |
| ----------- | ---------------- | ---------- | -------------- | ---------- | --------- |
| -1,156.921… | 180,269,313.136… |          4 |           3219 |     290367 |    298396 |
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
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:44:00  Running with dbt=1.11.11
21:44:00  Registered adapter: clickhouse=1.10.0
21:44:01  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:44:01  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:44:01
21:44:01  Concurrency: 1 threads (target='dev')
21:44:01
Previewing inline node:
|     min_value |       max_value | zero_count | negative_count | null_count | row_count |
| ------------- | --------------- | ---------- | -------------- | ---------- | --------- |
| -4,289,316.07 | 757,599,000,000 |       2675 |              2 |     294915 |    298396 |
```

### 数值范围：EARNED_PREMIUM_YOY

```sql
select
    min(`EARNED_PREMIUM_YOY`) as min_value,
    max(`EARNED_PREMIUM_YOY`) as max_value,
    countIf(`EARNED_PREMIUM_YOY` = 0) as zero_count,
    countIf(`EARNED_PREMIUM_YOY` < 0) as negative_count,
    countIf(isNull(`EARNED_PREMIUM_YOY`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__income_ytd') }}
```


结果（成功）：

```text
21:44:05  Running with dbt=1.11.11
21:44:05  Registered adapter: clickhouse=1.10.0
21:44:05  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:44:06  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:44:06
21:44:06  Concurrency: 1 threads (target='dev')
21:44:06
Previewing inline node:
| min_value |   max_value | zero_count | negative_count | null_count | row_count |
| --------- | ----------- | ---------- | -------------- | ---------- | --------- |
| -101.315… | 35,872.971… |          0 |            217 |     297691 |    298396 |
```
