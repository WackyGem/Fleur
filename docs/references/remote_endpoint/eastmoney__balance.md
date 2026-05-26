# eastmoney_balance

东方财富 F10 — 资产负债表

## Endpoint

```
GET https://datacenter.eastmoney.com/securities/api/data/get
```

无需认证，直接 GET 请求。

> **注意**：本接口使用 `data/get`（无版本号），参数风格与 `data/v1/get` 不同。
> 参数名为 `type`/`sty`/`p`/`ps`/`sr`/`st`，而非 `reportName`/`columns`/`pageNumber`/`pageSize`/`sortColumns`/`sortTypes`。

## Query Parameters

| 参数 | 必填 | 说明 | 示例值 |
|:-----|:-----|:-----|:-------|
| `type` | 是 | 报表标识，固定值 | `RPT_F10_FINANCE_GBALANCE` |
| `sty` | 是 | 返回字段样式，固定值 | `F10_FINANCE_GBALANCE` |
| `filter` | 是 | DataCenter DSL 过滤条件 | `(SECUCODE="601088.SH")` |
| `p` | 是 | 页码，从 1 开始 | `1` |
| `ps` | 是 | 每页条数 | `5` |
| `sr` | 是 | 排序方向：`-1` 降序，`1` 升序 | `-1` |
| `st` | 是 | 排序字段 | `REPORT_DATE` |
| `source` | 是 | 数据来源标识 | `HSF10` |
| `client` | 是 | 客户端标识 | `PC` |
| `v` | 否 | 缓存破坏参数（时间戳数字） | `03954046750774276` |

## Response

顶层 JSON：

```json
{
  "version": "...",
  "result": {
    "pages": 1,
    "data": [ ... ]
  }
}
```

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `version` | string | 数据版本标识 |
| `result.pages` | int | 总页数 |
| `result.data` | array | 资产负债表记录数组 |

> 本接口无 `code`/`success`/`message` 包装，也无 `result.count` 字段。

### 元数据字段

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `SECUCODE` | string | 证券代码（含市场后缀） | `"601088.SH"` |
| `SECURITY_CODE` | string | 证券代码（纯数字） | `"601088"` |
| `SECURITY_NAME_ABBR` | string | 证券简称 | `"中国神华"` |
| `ORG_CODE` | string | 机构代码 | `"10032705"` |
| `ORG_TYPE` | string | 机构类型 | `"通用"` |
| `REPORT_DATE` | string | 报告期 | `"2026-03-31 00:00:00"` |
| `REPORT_TYPE` | string | 报告类型 | `"一季报"` |
| `REPORT_DATE_NAME` | string | 报告期名称 | `"2026一季报"` |
| `SECURITY_TYPE_CODE` | string | 证券类型代码 | `"058001001"` |
| `NOTICE_DATE` | string | 公告日期 | `"2026-04-25 00:00:00"` |
| `UPDATE_DATE` | string | 更新日期 | `"2026-04-25 00:00:00"` |
| `CURRENCY` | string | 币种 | `"CNY"` |
| `OPINION_TYPE` | string? | 审计意见类型 | `null` |
| `OSOPINION_TYPE` | string? | 内控审计意见类型 | `null` |
| `LISTING_STATE` | string | 上市状态 | `"0"` |

### 绝对值字段（金额，单位：元）

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `ACCEPT_DEPOSIT_INTERBANK` | float? | 同业存放 | `null` |
| `ACCOUNTS_PAYABLE` | float | 应付账款 | `34365000000` |
| `ACCOUNTS_RECE` | float | 应收账款 | `13308000000` |
| `ACCRUED_EXPENSE` | float? | 预提费用 | `null` |
| `ADVANCE_RECEIVABLES` | float | 预收款项 | `85000000` |
| `AGENT_TRADE_SECURITY` | float? | 代理买卖证券款 | `null` |
| `AGENT_UNDERWRITE_SECURITY` | float? | 代理承销证券款 | `null` |
| `AMORTIZE_COST_FINASSET` | float? | 以摊余成本计量的金融资产 | `null` |
| `AMORTIZE_COST_FINLIAB` | float? | 以摊余成本计量的金融负债 | `null` |
| `AMORTIZE_COST_NCFINASSET` | float? | 非流动金融资产（摊余成本） | `null` |
| `AMORTIZE_COST_NCFINLIAB` | float? | 非流动金融负债（摊余成本） | `null` |
| `APPOINT_FVTPL_FINASSET` | float? | 指定为FVTPL的金融资产 | `null` |
| `APPOINT_FVTPL_FINLIAB` | float? | 指定为FVTPL的金融负债 | `null` |
| `ASSET_BALANCE` | float | 资产平衡项 | `0` |
| `ASSET_OTHER` | float? | 资产其他项 | `null` |
| `ASSIGN_CASH_DIVIDEND` | float? | 应付现金股利 | `null` |
| `AVAILABLE_SALE_FINASSET` | float? | 可供出售金融资产 | `null` |
| `BOND_PAYABLE` | float? | 应付债券 | `null` |
| `BORROW_FUND` | float? | 拆入资金 | `null` |
| `BUY_RESALE_FINASSET` | float? | 买入返售金融资产 | `null` |
| `CAPITAL_RESERVE` | float | 资本公积 | `139598000000` |
| `CIP` | float | 在建工程 | `34043000000` |
| `CONSUMPTIVE_BIOLOGICAL_ASSET` | float? | 消耗性生物资产 | `null` |
| `CONTRACT_ASSET` | float? | 合同资产 | `null` |
| `CONTRACT_LIAB` | float | 合同负债 | `4715000000` |
| `CONVERT_DIFF` | float? | 外币报表折算差额 | `null` |
| `CREDITOR_INVEST` | float? | 债权投资 | `null` |
| `CURRENT_ASSET_BALANCE` | float | 流动资产平衡项 | `0` |
| `CURRENT_ASSET_OTHER` | float? | 流动资产其他项 | `null` |
| `CURRENT_LIAB_BALANCE` | float | 流动负债平衡项 | `0` |
| `CURRENT_LIAB_OTHER` | float? | 流动负债其他项 | `null` |
| `DEFER_INCOME` | float? | 递延收益 | `null` |
| `DEFER_INCOME_1YEAR` | float? | 一年内到期递延收益 | `null` |
| `DEFER_TAX_ASSET` | float | 递延所得税资产 | `6926000000` |
| `DEFER_TAX_LIAB` | float | 递延所得税负债 | `1763000000` |
| `DERIVE_FINASSET` | float? | 衍生金融资产 | `null` |
| `DERIVE_FINLIAB` | float? | 衍生金融负债 | `null` |
| `DEVELOP_EXPENSE` | float? | 开发支出 | `null` |
| `DIV_HOLDSALE_ASSET` | float? | 持有待售资产（除） | `null` |
| `DIV_HOLDSALE_LIAB` | float? | 持有待售负债（除） | `null` |
| `DIVIDEND_PAYABLE` | float | 应付股利 | `1277000000` |
| `DIVIDEND_RECE` | float | 应收股利 | `53000000` |
| `EQUITY_BALANCE` | float | 所有者权益平衡项 | `0` |
| `EQUITY_OTHER` | float? | 所有者权益其他项 | `null` |
| `EXPORT_REFUND_RECE` | float? | 应收出口退税 | `null` |
| `FEE_COMMISSION_PAYABLE` | float? | 应付手续费及佣金 | `null` |
| `FIN_FUND` | float? | 金融往来资金 | `null` |
| `FINANCE_RECE` | float | 金融应收款 | `824000000` |
| `FIXED_ASSET` | float | 固定资产 | `271055000000` |
| `FIXED_ASSET_DISPOSAL` | float? | 固定资产清理 | `null` |
| `FVTOCI_FINASSET` | float? | 以公允价值计量且其变动计入其他综合收益的金融资产 | `null` |
| `FVTOCI_NCFINASSET` | float? | 其他非流动金融资产（FVTOCI） | `null` |
| `FVTPL_FINASSET` | float? | 以公允价值计量且其变动计入当期损益的金融资产 | `null` |
| `FVTPL_FINLIAB` | float? | 以公允价值计量且其变动计入当期损益的金融负债 | `null` |
| `GENERAL_RISK_RESERVE` | float? | 一般风险准备 | `null` |
| `GOODWILL` | float? | 商誉 | `null` |
| `HOLD_MATURITY_INVEST` | float? | 持有至到期投资 | `null` |
| `HOLDSALE_ASSET` | float? | 持有待售资产 | `null` |
| `HOLDSALE_LIAB` | float? | 持有待售负债 | `null` |
| `INSURANCE_CONTRACT_RESERVE` | float? | 保险合同准备金 | `null` |
| `INTANGIBLE_ASSET` | float | 无形资产 | `69511000000` |
| `INTEREST_PAYABLE` | float | 应付利息 | `113000000` |
| `INTEREST_RECE` | float | 应收利息 | `40000000` |
| `INTERNAL_PAYABLE` | float? | 内部应付款 | `null` |
| `INTERNAL_RECE` | float? | 内部应收款 | `null` |
| `INVENTORY` | float | 存货 | `10792000000` |
| `INVEST_REALESTATE` | float? | 投资性房地产 | `null` |
| `LEASE_LIAB` | float | 租赁负债 | `996000000` |
| `LEND_FUND` | float? | 拆出资金 | `null` |
| `LIAB_BALANCE` | float | 负债平衡项 | `0` |
| `LIAB_EQUITY_BALANCE` | float? | 负债和所有者权益平衡项 | `null` |
| `LIAB_EQUITY_OTHER` | float? | 负债和所有者权益其他项 | `null` |
| `LIAB_OTHER` | float? | 负债其他项 | `null` |
| `LOAN_ADVANCE` | float? | 发放贷款及垫款 | `null` |
| `LOAN_PBC` | float? | 向央行借款 | `null` |
| `LONG_EQUITY_INVEST` | float | 长期股权投资 | `62243000000` |
| `LONG_LOAN` | float | 长期借款 | `29673000000` |
| `LONG_PAYABLE` | float | 长期应付款 | `13892000000` |
| `LONG_PREPAID_EXPENSE` | float | 长期待摊费用 | `5651000000` |
| `LONG_RECE` | float? | 长期应收款 | `null` |
| `LONG_STAFFSALARY_PAYABLE` | float? | 长期应付职工薪酬 | `null` |
| `MINORITY_EQUITY` | float | 少数股东权益 | `74981000000` |
| `MONETARYFUNDS` | float | 货币资金 | `118585000000` |
| `NONCURRENT_ASSET_1YEAR` | float? | 一年内到期的非流动资产 | `null` |
| `NONCURRENT_ASSET_BALANCE` | float | 非流动资产平衡项 | `0` |
| `NONCURRENT_ASSET_OTHER` | float? | 非流动资产其他项 | `null` |
| `NONCURRENT_LIAB_1YEAR` | float | 一年内到期的非流动负债 | `8174000000` |
| `NONCURRENT_LIAB_BALANCE` | float | 非流动负债平衡项 | `0` |
| `NONCURRENT_LIAB_OTHER` | float? | 非流动负债其他项 | `null` |
| `NOTE_ACCOUNTS_PAYABLE` | float | 应付票据及应付账款 | `34886000000` |
| `NOTE_ACCOUNTS_RECE` | float | 应收票据及应收账款 | `18577000000` |
| `NOTE_PAYABLE` | float | 应付票据 | `521000000` |
| `NOTE_RECE` | float | 应收票据 | `5269000000` |
| `OIL_GAS_ASSET` | float? | 油气资产 | `null` |
| `OTHER_COMPRE_INCOME` | float | 其他综合收益 | `1381000000` |
| `OTHER_CREDITOR_INVEST` | float? | 其他债权投资 | `null` |
| `OTHER_CURRENT_ASSET` | float | 其他流动资产 | `8061000000` |
| `OTHER_CURRENT_LIAB` | float | 其他流动负债 | `2425000000` |
| `OTHER_EQUITY_INVEST` | float | 其他权益工具投资 | `3175000000` |
| `OTHER_EQUITY_OTHER` | float? | 其他权益其他项 | `null` |
| `OTHER_EQUITY_TOOL` | float? | 其他权益工具 | `null` |
| `OTHER_NONCURRENT_ASSET` | float | 其他非流动资产 | `161605000000` |
| `OTHER_NONCURRENT_FINASSET` | float | 其他非流动金融资产 | `113000000` |
| `OTHER_NONCURRENT_LIAB` | float | 其他非流动负债 | `1781000000` |
| `OTHER_PAYABLE` | float? | 其他应付款 | `null` |
| `OTHER_RECE` | float? | 其他应收款 | `null` |
| `PARENT_EQUITY_BALANCE` | float | 归母权益平衡项 | `0` |
| `PARENT_EQUITY_OTHER` | float? | 归母权益其他项 | `null` |
| `PERPETUAL_BOND` | float? | 永续债 | `null` |
| `PERPETUAL_BOND_PAYBALE` | float? | 永续债（负债端） | `null` |
| `PREDICT_CURRENT_LIAB` | float? | 预计流动负债 | `null` |
| `PREDICT_LIAB` | float | 预计负债 | `9997000000` |
| `PREFERRED_SHARES` | float? | 优先股 | `null` |
| `PREFERRED_SHARES_PAYBALE` | float? | 应付优先股 | `null` |
| `PREMIUM_RECE` | float? | 预收保费 | `null` |
| `PREPAYMENT` | float | 预付款项 | `7864000000` |
| `PRODUCTIVE_BIOLOGY_ASSET` | float? | 生产性生物资产 | `null` |
| `PROJECT_MATERIAL` | float? | 工程物资 | `null` |
| `RC_RESERVE_RECE` | float? | 再保合同应收准备金 | `null` |
| `REINSURE_PAYABLE` | float? | 应付再保款 | `null` |
| `REINSURE_RECE` | float? | 应收再保款 | `null` |
| `SELL_REPO_FINASSET` | float? | 卖出回购金融资产款 | `null` |
| `SETTLE_EXCESS_RESERVE` | float? | 清算备付金 | `null` |
| `SHARE_CAPITAL` | float | 实收资本（股本） | `21689000000` |
| `SHORT_BOND_PAYABLE` | float? | 短期应付债券 | `null` |
| `SHORT_FIN_PAYABLE` | float? | 短期金融负债 | `null` |
| `SHORT_LOAN` | float | 短期借款 | `86700000000` |
| `SPECIAL_PAYABLE` | float? | 专项应付款 | `null` |
| `SPECIAL_RESERVE` | float | 专项储备 | `26131000000` |
| `STAFF_SALARY_PAYABLE` | float | 应付职工薪酬 | `10982000000` |
| `SUBSIDY_RECE` | float? | 应收补贴 | `null` |
| `SURPLUS_RESERVE` | float | 盈余公积 | `11433000000` |
| `TAX_PAYABLE` | float | 应交税费 | `6697000000` |
| `TOTAL_ASSETS` | float | 资产总计 | `783279000000` |
| `TOTAL_CURRENT_ASSETS` | float | 流动资产合计 | `167634000000` |
| `TOTAL_CURRENT_LIAB` | float | 流动负债合计 | `169294000000` |
| `TOTAL_EQUITY` | float | 所有者权益合计 | `555883000000` |
| `TOTAL_LIAB_EQUITY` | float | 负债和所有者权益总计 | `783279000000` |
| `TOTAL_LIABILITIES` | float | 负债合计 | `227396000000` |
| `TOTAL_NONCURRENT_ASSETS` | float | 非流动资产合计 | `615645000000` |
| `TOTAL_NONCURRENT_LIAB` | float | 非流动负债合计 | `58102000000` |
| `TOTAL_OTHER_PAYABLE` | float | 其他应付款合计 | `14630000000` |
| `TOTAL_OTHER_RECE` | float | 其他应收款合计 | `2931000000` |
| `TOTAL_PARENT_EQUITY` | float | 归属于母公司股东权益合计 | `480902000000` |
| `TRADE_FINASSET` | float? | 交易性金融资产 | `null` |
| `TRADE_FINASSET_NOTFVTPL` | float | 非FVTPL交易性金融资产 | `0` |
| `TRADE_FINLIAB` | float? | 交易性金融负债 | `null` |
| `TRADE_FINLIAB_NOTFVTPL` | float? | 非FVTPL交易性金融负债 | `null` |
| `TREASURY_SHARES` | float? | 库存股 | `null` |
| `UNASSIGN_RPOFIT` | float | 未分配利润 | `280670000000` |
| `UNCONFIRM_INVEST_LOSS` | float? | 未确认投资损失 | `null` |
| `USERIGHT_ASSET` | float | 使用权资产 | `1323000000` |

### 同比增长率字段（`_YOY` 后缀，百分比）

| 字段 | 类型 | 示例 |
|:-----|:-----|:-----|
| `ACCEPT_DEPOSIT_INTERBANK_YOY` | float | `null` |
| `ACCOUNTS_PAYABLE_YOY` | float | `11.8288317605` |
| `ACCOUNTS_RECE_YOY` | float | `0.037585507` |
| `ACCRUED_EXPENSE_YOY` | float | `null` |
| `ADVANCE_RECEIVABLES_YOY` | float | `-8.6021505376` |
| `AGENT_TRADE_SECURITY_YOY` | float | `null` |
| `AGENT_UNDERWRITE_SECURITY_YOY` | float | `null` |
| `AMORTIZE_COST_FINASSET_YOY` | float | `null` |
| `AMORTIZE_COST_FINLIAB_YOY` | float | `null` |
| `AMORTIZE_COST_NCFINASSET_YOY` | float | `null` |
| `AMORTIZE_COST_NCFINLIAB_YOY` | float | `null` |
| `APPOINT_FVTPL_FINASSET_YOY` | float | `null` |
| `APPOINT_FVTPL_FINLIAB_YOY` | float | `null` |
| `ASSET_BALANCE_YOY` | float | `null` |
| `ASSET_OTHER_YOY` | float | `null` |
| `ASSIGN_CASH_DIVIDEND_YOY` | float | `null` |
| `AVAILABLE_SALE_FINASSET_YOY` | float | `null` |
| `BOND_PAYABLE_YOY` | float | `null` |
| `BORROW_FUND_YOY` | float | `null` |
| `BUY_RESALE_FINASSET_YOY` | float | `null` |
| `CAPITAL_RESERVE_YOY` | float | `71.5806293019` |
| `CIP_YOY` | float | `9.9083101956` |
| `CONSUMPTIVE_BIOLOGICAL_ASSET_YOY` | float | `null` |
| `CONTRACT_ASSET_YOY` | float | `null` |
| `CONTRACT_LIAB_YOY` | float | `21.4580113344` |
| `CONVERT_DIFF_YOY` | float | `null` |
| `CREDITOR_INVEST_YOY` | float | `null` |
| `CURRENT_ASSET_BALANCE_YOY` | float | `null` |
| `CURRENT_ASSET_OTHER_YOY` | float | `null` |
| `CURRENT_LIAB_BALANCE_YOY` | float | `null` |
| `CURRENT_LIAB_OTHER_YOY` | float | `null` |
| `DEFER_INCOME_1YEAR_YOY` | float | `null` |
| `DEFER_INCOME_YOY` | float | `null` |
| `DEFER_TAX_ASSET_YOY` | float | `-0.8304696449` |
| `DEFER_TAX_LIAB_YOY` | float | `28.2181818182` |
| `DERIVE_FINASSET_YOY` | float | `null` |
| `DERIVE_FINLIAB_YOY` | float | `null` |
| `DEVELOP_EXPENSE_YOY` | float | `null` |
| `DIV_HOLDSALE_ASSET_YOY` | float | `null` |
| `DIV_HOLDSALE_LIAB_YOY` | float | `null` |
| `DIVIDEND_PAYABLE_YOY` | float | `-73.0192267061` |
| `DIVIDEND_RECE_YOY` | float | `-8.6206896552` |
| `EQUITY_BALANCE_YOY` | float | `null` |
| `EQUITY_OTHER_YOY` | float | `null` |
| `EXPORT_REFUND_RECE_YOY` | float | `null` |
| `FEE_COMMISSION_PAYABLE_YOY` | float | `null` |
| `FIN_FUND_YOY` | float | `null` |
| `FINANCE_RECE_YOY` | float | `858.1395348837` |
| `FIXED_ASSET_DISPOSAL_YOY` | float | `null` |
| `FIXED_ASSET_YOY` | float | `4.71184974` |
| `FVTOCI_FINASSET_YOY` | float | `null` |
| `FVTOCI_NCFINASSET_YOY` | float | `null` |
| `FVTPL_FINASSET_YOY` | float | `null` |
| `FVTPL_FINLIAB_YOY` | float | `null` |
| `GENERAL_RISK_RESERVE_YOY` | float | `null` |
| `GOODWILL_YOY` | float | `null` |
| `HOLD_MATURITY_INVEST_YOY` | float | `null` |
| `HOLDSALE_ASSET_YOY` | float | `null` |
| `HOLDSALE_LIAB_YOY` | float | `null` |
| `INSURANCE_CONTRACT_RESERVE_YOY` | float | `null` |
| `INTANGIBLE_ASSET_YOY` | float | `3.5530196943` |
| `INTEREST_PAYABLE_YOY` | float | `56.9444444444` |
| `INTEREST_RECE_YOY` | float | `-84.962406015` |
| `INTERNAL_PAYABLE_YOY` | float | `null` |
| `INTERNAL_RECE_YOY` | float | `null` |
| `INVENTORY_YOY` | float | `-12.3171920702` |
| `INVEST_REALESTATE_YOY` | float | `null` |
| `LEASE_LIAB_YOY` | float | `-11.0714285714` |
| `LEND_FUND_YOY` | float | `null` |
| `LIAB_BALANCE_YOY` | float | `null` |
| `LIAB_EQUITY_BALANCE_YOY` | float | `null` |
| `LIAB_EQUITY_OTHER_YOY` | float | `null` |
| `LIAB_OTHER_YOY` | float | `null` |
| `LOAN_ADVANCE_YOY` | float | `null` |
| `LOAN_PBC_YOY` | float | `null` |
| `LONG_EQUITY_INVEST_YOY` | float | `1.9257536804` |
| `LONG_LOAN_YOY` | float | `-8.5662342464` |
| `LONG_PAYABLE_YOY` | float | `-17.5891321113` |
| `LONG_PREPAID_EXPENSE_YOY` | float | `39.6688087` |
| `LONG_RECE_YOY` | float | `null` |
| `LONG_STAFFSALARY_PAYABLE_YOY` | float | `null` |
| `MINORITY_EQUITY_YOY` | float | `-6.0035100915` |
| `MONETARYFUNDS_YOY` | float | `-23.6909672396` |
| `NONCURRENT_ASSET_1YEAR_YOY` | float | `null` |
| `NONCURRENT_ASSET_BALANCE_YOY` | float | `null` |
| `NONCURRENT_ASSET_OTHER_YOY` | float | `null` |
| `NONCURRENT_LIAB_1YEAR_YOY` | float | `-41.4050179211` |
| `NONCURRENT_LIAB_BALANCE_YOY` | float | `null` |
| `NONCURRENT_LIAB_OTHER_YOY` | float | `null` |
| `NOTE_ACCOUNTS_PAYABLE_YOY` | float | `12.8011122967` |
| `NOTE_ACCOUNTS_RECE_YOY` | float | `21.220228385` |
| `NOTE_PAYABLE_YOY` | float | `164.4670050761` |
| `NOTE_RECE_YOY` | float | `160.5835806133` |
| `OIL_GAS_ASSET_YOY` | float | `null` |
| `OTHER_COMPRE_INCOME_YOY` | float | `-14.4891640867` |
| `OTHER_CREDITOR_INVEST_YOY` | float | `null` |
| `OTHER_CURRENT_ASSET_YOY` | float | `1.2052730697` |
| `OTHER_CURRENT_LIAB_YOY` | float | `4.0326040326` |
| `OTHER_EQUITY_INVEST_YOY` | float | `13.9217796914` |
| `OTHER_EQUITY_OTHER_YOY` | float | `null` |
| `OTHER_EQUITY_TOOL_YOY` | float | `null` |
| `OTHER_NONCURRENT_ASSET_YOY` | float | `434.4787670327` |
| `OTHER_NONCURRENT_FINASSET_YOY` | float | `88.3333333333` |
| `OTHER_NONCURRENT_LIAB_YOY` | float | `19.050802139` |
| `OTHER_PAYABLE_YOY` | float | `null` |
| `OTHER_RECE_YOY` | float | `null` |
| `PARENT_EQUITY_BALANCE_YOY` | float | `null` |
| `PARENT_EQUITY_OTHER_YOY` | float | `null` |
| `PERPETUAL_BOND_PAYBALE_YOY` | float | `null` |
| `PERPETUAL_BOND_YOY` | float | `null` |
| `PREDICT_CURRENT_LIAB_YOY` | float | `null` |
| `PREDICT_LIAB_YOY` | float | `1.0512483574` |
| `PREFERRED_SHARES_PAYBALE_YOY` | float | `null` |
| `PREFERRED_SHARES_YOY` | float | `null` |
| `PREMIUM_RECE_YOY` | float | `null` |
| `PREPAYMENT_YOY` | float | `12.182596291` |
| `PRODUCTIVE_BIOLOGY_ASSET_YOY` | float | `null` |
| `PROJECT_MATERIAL_YOY` | float | `null` |
| `RC_RESERVE_RECE_YOY` | float | `null` |
| `REINSURE_PAYABLE_YOY` | float | `null` |
| `REINSURE_RECE_YOY` | float | `null` |
| `SELL_REPO_FINASSET_YOY` | float | `null` |
| `SETTLE_EXCESS_RESERVE_YOY` | float | `null` |
| `SHARE_CAPITAL_YOY` | float | `9.1599979868` |
| `SHORT_BOND_PAYABLE_YOY` | float | `null` |
| `SHORT_FIN_PAYABLE_YOY` | float | `null` |
| `SHORT_LOAN_YOY` | float | `2861.0655737705` |
| `SPECIAL_PAYABLE_YOY` | float | `null` |
| `SPECIAL_RESERVE_YOY` | float | `3.1581856224` |
| `STAFF_SALARY_PAYABLE_YOY` | float | `-16.2063177171` |
| `SUBSIDY_RECE_YOY` | float | `null` |
| `SURPLUS_RESERVE_YOY` | float | `0.0087473758` |
| `TAX_PAYABLE_YOY` | float | `-20.3022729977` |
| `TOTAL_ASSETS_YOY` | float | `16.5061497203` |
| `TOTAL_CURRENT_ASSETS_YOY` | float | `-19.6805121006` |
| `TOTAL_CURRENT_LIAB_YOY` | float | `75.9282544763` |
| `TOTAL_EQUITY_YOY` | float | `8.3837670896` |
| `TOTAL_LIAB_EQUITY_YOY` | float | `16.5061497203` |
| `TOTAL_LIABILITIES_YOY` | float | `42.6368842639` |
| `TOTAL_NONCURRENT_ASSETS_YOY` | float | `32.797164785` |
| `TOTAL_NONCURRENT_LIAB_YOY` | float | `-8.0577269994` |
| `TOTAL_OTHER_PAYABLE_YOY` | float | `-29.0115968752` |
| `TOTAL_OTHER_RECE_YOY` | float | `0.7909215956` |
| `TOTAL_PARENT_EQUITY_YOY` | float | `11.0335846913` |
| `TRADE_FINASSET_NOTFVTPL_YOY` | float | `-100` |
| `TRADE_FINASSET_YOY` | float | `null` |
| `TRADE_FINLIAB_NOTFVTPL_YOY` | float | `null` |
| `TRADE_FINLIAB_YOY` | float | `null` |
| `TREASURY_SHARES_YOY` | float | `null` |
| `UNASSIGN_RPOFIT_YOY` | float | `-4.3736605941` |
| `UNCONFIRM_INVEST_LOSS_YOY` | float | `null` |
| `USERIGHT_ASSET_YOY` | float | `-9.3835616438` |

## Filter DSL

`filter` 是东方财富数据中心自带的查询表达式。多个条件直接拼接，无显式 AND/OR 运算符。

> **注意**：filter 参数值需正确 URL 编码（单引号 `%27`，双引号 `%22`）。`>=`/`<=` 操作符需配合正确编码才可用。

**按证券代码**：

```
(SECUCODE="601088.SH")
```

证券代码格式：`{6位代码}.{市场}`，沪市 `SH`，深市 `SZ`。

**按证券代码批量**（使用 `in` 语法）：

```
(SECUCODE in ("601088.SH","000001.SZ","600519.SH"))
```

**按报告期区间**：

```
(SECUCODE="601088.SH")(REPORT_DATE>='2025-01-01')(REPORT_DATE<='2025-12-31')
```

**按公告日期区间**：

```
(SECUCODE="601088.SH")(NOTICE_DATE>='2025-01-01')(NOTICE_DATE<='2025-12-31')
```

**按报告期精确匹配**（使用 `in` 语法）：

```
(SECUCODE="601088.SH")(REPORT_DATE in ('2026-03-31','2025-12-31','2025-09-30'))
```

## Sorting

支持单字段和多字段排序，通过 `st`（排序字段）和 `sr`（排序方向）控制。

- 单字段排序：`st=REPORT_DATE&sr=-1`
- 多字段排序：`st=REPORT_DATE,SECURITY_CODE&sr=-1,-1`

`sr` 值：`-1` 降序，`1` 升序。多字段时逗号分隔，与 `st` 一一对应。

> **稳定性警告**：仅用 `REPORT_DATE` 单字段排序时，当同一报告期有多条记录且跨越分页边界，服务端分页偏移计算与数据返回的内部排序不一致，会导致**分页数据重复**。
>
> 复现条件（需同时满足）：
> 1. 使用日期区间查询（跨多个报告期）
> 2. 不限定证券代码或股票数量较多
> 3. 仅用 `REPORT_DATE` 单字段排序
> 4. 同一报告期的记录数跨越分页边界
>
> **解决方案**：始终添加第二排序字段（如 `SECURITY_CODE`），使同日期组内顺序完全确定：
> ```
> st=REPORT_DATE,SECURITY_CODE&sr=-1,-1
> ```

## Pagination

标准 URL 分页，通过 `p`（页码）/ `ps`（每页条数）控制。使用多字段排序时分页数据不会重复。

## cURL 示例

**单只股票**：

```bash
curl -s 'https://datacenter.eastmoney.com/securities/api/data/get?\
type=RPT_F10_FINANCE_GBALANCE&\
sty=F10_FINANCE_GBALANCE&\
filter=(SECUCODE="601088.SH")&\
p=1&\
ps=5&\
sr=-1&\
st=REPORT_DATE&\
source=HSF10&\
client=PC'
```

> 单只股票时单字段排序即可，不会出现分页重复。

**多股票 + 日期区间 + 多字段排序（推荐）**：

```bash
curl -s -G 'https://datacenter.eastmoney.com/securities/api/data/get' \
  --data-urlencode 'type=RPT_F10_FINANCE_GBALANCE' \
  --data-urlencode 'sty=F10_FINANCE_GBALANCE' \
  --data-urlencode 'filter=(SECUCODE in ("601088.SH","000001.SZ","600519.SH"))(REPORT_DATE>='"'"'2024-01-01'"'"')(REPORT_DATE<='"'"'2025-12-31'"'"')' \
  --data-urlencode 'p=1' \
  --data-urlencode 'ps=10' \
  --data-urlencode 'sr=-1,-1' \
  --data-urlencode 'st=REPORT_DATE,SECURITY_CODE' \
  --data-urlencode 'source=HSF10' \
  --data-urlencode 'client=PC'
```

## Sample Response

`601088.SH` 2026Q1（`ps=1` 时第 1 条）：

```json
{
  "SECUCODE": "601088.SH",
  "SECURITY_CODE": "601088",
  "SECURITY_NAME_ABBR": "中国神华",
  "ORG_CODE": "10032705",
  "ORG_TYPE": "通用",
  "REPORT_DATE": "2026-03-31 00:00:00",
  "TOTAL_ASSETS": 783279000000,
  "TOTAL_LIABILITIES": 227396000000,
  "TOTAL_EQUITY": 555883000000,
  "TOTAL_PARENT_EQUITY": 480902000000,
  "MINORITY_EQUITY": 74981000000,
  "TOTAL_LIAB_EQUITY": 783279000000,
  "TOTAL_CURRENT_ASSETS": 167634000000,
  "TOTAL_NONCURRENT_ASSETS": 615645000000,
  "TOTAL_CURRENT_LIAB": 169294000000,
  "TOTAL_NONCURRENT_LIAB": 58102000000,
  "MONETARYFUNDS": 118585000000,
  "ACCOUNTS_RECE": 13308000000,
  "NOTE_RECE": 5269000000,
  "INVENTORY": 10792000000,
  "FIXED_ASSET": 271055000000,
  "INTANGIBLE_ASSET": 69511000000,
  "SHORT_LOAN": 86700000000,
  "ACCOUNTS_PAYABLE": 34365000000,
  "SHARE_CAPITAL": 21689000000,
  "CAPITAL_RESERVE": 139598000000,
  "SURPLUS_RESERVE": 11433000000,
  "UNASSIGN_RPOFIT": 280670000000,
  "OPINION_TYPE": null,
  "OSOPINION_TYPE": null,
  "LISTING_STATE": "0"
}
```
