# eastmoney_cashflow_ytd

东方财富 F10 — 现金流量表（累计）

## Endpoint

```
GET https://datacenter.eastmoney.com/securities/api/data/get
```

无需认证，直接 GET 请求。

> **注意**：本接口使用 `data/get`（无版本号），参数风格与 `data/v1/get` 不同。
> 参数名为 `type`/`sty`/`p`/`ps`/`sr`/`st`，而非 `reportName`/`columns`/`pageNumber`/`pageSize`/`sortColumns`/`sortTypes`。

**与单季度版（`cashflow_sq`）的区别**：本接口返回**累计**口径数据，同比增长率使用 `_YOY` 后缀（无 `_QOQ`）。

## Query Parameters

| 参数 | 必填 | 说明 | 示例值 |
|:-----|:-----|:-----|:-------|
| `type` | 是 | 报表标识，固定值 | `RPT_F10_FINANCE_GCASHFLOW` |
| `sty` | 是 | 返回字段样式，固定值 | `APP_F10_GCASHFLOW` |
| `filter` | 是 | DataCenter DSL 过滤条件 | `(SECUCODE="601088.SH")` |
| `p` | 是 | 页码，从 1 开始 | `1` |
| `ps` | 是 | 每页条数 | `5` |
| `sr` | 是 | 排序方向：`-1` 降序，`1` 升序 | `-1` |
| `st` | 是 | 排序字段 | `REPORT_DATE` |
| `source` | 是 | 数据来源标识 | `HSF10` |
| `client` | 是 | 客户端标识 | `PC` |
| `v` | 否 | 缓存破坏参数（时间戳数字） | — |

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
| `result.data` | array | 记录数组 |

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
| `SALES_SERVICES` | float | 销售商品、提供劳务收到的现金 | `77839000000` |
| `DEPOSIT_INTERBANK_ADD` | float? | 同业存放净增加额 | `null` |
| `LOAN_PBC_ADD` | float? | 向央行借款净增加额 | `null` |
| `OFI_BF_ADD` | float? | 向其他金融机构拆入资金净增加额 | `null` |
| `RECEIVE_ORIGIC_PREMIUM` | float? | 收到原保险合同保费现金 | `null` |
| `RECEIVE_REINSURE_NET` | float? | 收到再保险业务现金净额 | `null` |
| `INSURED_INVEST_ADD` | float? | 保户储金及投资款净增加额 | `null` |
| `DISPOSAL_TFA_ADD` | float? | 处置交易性金融资产净增加额 | `null` |
| `RECEIVE_INTEREST_COMMISSION` | float? | 收取利息和手续费现金 | `null` |
| `BORROW_FUND_ADD` | float? | 拆入资金净增加额 | `null` |
| `LOAN_ADVANCE_REDUCE` | float? | 发放贷款及垫款净减少额 | `null` |
| `REPO_BUSINESS_ADD` | float? | 回购业务资金净增加额 | `null` |
| `RECEIVE_TAX_REFUND` | float? | 收到的税费返还 | `null` |
| `RECEIVE_OTHER_OPERATE` | float | 收到其他与经营活动有关的现金 | `2064000000` |
| `OPERATE_INFLOW_OTHER` | float? | 经营活动现金流入其他 | `null` |
| `OPERATE_INFLOW_BALANCE` | float | 经营活动现金流入平衡项 | `0` |
| `TOTAL_OPERATE_INFLOW` | float | 经营活动现金流入小计 | `79903000000` |
| `BUY_SERVICES` | float | 购买商品、接受劳务支付的现金 | `35617000000` |
| `LOAN_ADVANCE_ADD` | float? | 发放贷款及垫款净增加额 | `null` |
| `PBC_INTERBANK_ADD` | float? | 向央行借款净增加额 | `null` |
| `PAY_ORIGIC_COMPENSATE` | float? | 支付原保险合同赔付款项现金 | `null` |
| `PAY_INTEREST_COMMISSION` | float? | 支付利息和手续费现金 | `null` |
| `PAY_POLICY_BONUS` | float? | 保单红利支出 | `null` |
| `PAY_STAFF_CASH` | float | 支付给职工以及为职工支付的现金 | `9911000000` |
| `PAY_ALL_TAX` | float | 支付的各项税费 | `14196000000` |
| `PAY_OTHER_OPERATE` | float | 支付其他与经营活动有关的现金 | `2816000000` |
| `OPERATE_OUTFLOW_OTHER` | float? | 经营活动现金流出其他 | `null` |
| `OPERATE_OUTFLOW_BALANCE` | float | 经营活动现金流出平衡项 | `0` |
| `TOTAL_OPERATE_OUTFLOW` | float | 经营活动现金流出小计 | `62540000000` |
| `OPERATE_NETCASH_OTHER` | float? | 经营活动净现金流量其他 | `null` |
| `OPERATE_NETCASH_BALANCE` | float | 经营活动净现金流量平衡项 | `0` |
| `NETCASH_OPERATE` | float | 经营活动产生的现金流量净额 | `17363000000` |
| `WITHDRAW_INVEST` | float | 收回投资收到的现金 | `151000000` |
| `RECEIVE_INVEST_INCOME` | float | 取得投资收益收到的现金 | `704000000` |
| `DISPOSAL_LONG_ASSET` | float | 处置固定资产等收回的现金净额 | `330000000` |
| `DISPOSAL_SUBSIDIARY_OTHER` | float | 处置子公司及其他营业单位收到的现金净额 | `0` |
| `REDUCE_PLEDGE_TIMEDEPOSITS` | float? | 减少质押定期存款 | `null` |
| `RECEIVE_OTHER_INVEST` | float | 收到其他与投资活动有关的现金 | `2685000000` |
| `INVEST_INFLOW_OTHER` | float? | 投资活动现金流入其他 | `null` |
| `INVEST_INFLOW_BALANCE` | float | 投资活动现金流入平衡项 | `0` |
| `TOTAL_INVEST_INFLOW` | float | 投资活动现金流入小计 | `3870000000` |
| `CONSTRUCT_LONG_ASSET` | float | 购建固定资产等支付的现金 | `9583000000` |
| `INVEST_PAY_CASH` | float | 投资支付的现金 | `2601000000` |
| `PLEDGE_LOAN_ADD` | float? | 质押贷款净增加额 | `null` |
| `OBTAIN_SUBSIDIARY_OTHER` | float | 取得子公司及其他营业单位支付的现金净额 | `90942000000` |
| `ADD_PLEDGE_TIMEDEPOSITS` | float? | 增加质押定期存款 | `null` |
| `PAY_OTHER_INVEST` | float | 支付其他与投资活动有关的现金 | `3090000000` |
| `INVEST_OUTFLOW_OTHER` | float? | 投资活动现金流出其他 | `null` |
| `INVEST_OUTFLOW_BALANCE` | float | 投资活动现金流出平衡项 | `0` |
| `TOTAL_INVEST_OUTFLOW` | float | 投资活动现金流出小计 | `106216000000` |
| `INVEST_NETCASH_OTHER` | float? | 投资活动净现金流量其他 | `null` |
| `INVEST_NETCASH_BALANCE` | float | 投资活动净现金流量平衡项 | `0` |
| `NETCASH_INVEST` | float | 投资活动产生的现金流量净额 | `-102346000000` |
| `ACCEPT_INVEST_CASH` | float | 吸收投资收到的现金 | `20144000000` |
| `SUBSIDIARY_ACCEPT_INVEST` | float | 子公司吸收少数股东投资收到的现金 | `178000000` |
| `RECEIVE_LOAN_CASH` | float | 取得借款收到的现金 | `96057000000` |
| `ISSUE_BOND` | float? | 发行债券收到的现金 | `null` |
| `RECEIVE_OTHER_FINANCE` | float? | 收到其他与筹资活动有关的现金 | `null` |
| `FINANCE_INFLOW_OTHER` | float? | 筹资活动现金流入其他 | `null` |
| `FINANCE_INFLOW_BALANCE` | float | 筹资活动现金流入平衡项 | `0` |
| `TOTAL_FINANCE_INFLOW` | float | 筹资活动现金流入小计 | `116201000000` |
| `PAY_DEBT_CASH` | float | 偿还债务支付的现金 | `8594000000` |
| `ASSIGN_DIVIDEND_PORFIT` | float | 分配股利、利润或偿付利息支付的现金 | `921000000` |
| `SUBSIDIARY_PAY_DIVIDEND` | float | 子公司向少数股东支付的现金股利 | `541000000` |
| `BUY_SUBSIDIARY_EQUITY` | float? | 子公司减少现金 | `null` |
| `PAY_OTHER_FINANCE` | float | 支付其他与筹资活动有关的现金 | `151000000` |
| `SUBSIDIARY_REDUCE_CASH` | float? | 子公司减少现金 | `null` |
| `FINANCE_OUTFLOW_OTHER` | float? | 筹资活动现金流出其他 | `null` |
| `FINANCE_OUTFLOW_BALANCE` | float | 筹资活动现金流出平衡项 | `0` |
| `TOTAL_FINANCE_OUTFLOW` | float | 筹资活动现金流出小计 | `9666000000` |
| `FINANCE_NETCASH_OTHER` | float? | 筹资活动净现金流量其他 | `null` |
| `FINANCE_NETCASH_BALANCE` | float | 筹资活动净现金流量平衡项 | `0` |
| `NETCASH_FINANCE` | float | 筹资活动产生的现金流量净额 | `106535000000` |
| `RATE_CHANGE_EFFECT` | float | 汇率变动对现金及现金等价物的影响 | `-143000000` |
| `CCE_ADD_OTHER` | float? | 现金及现金等价物净增加额其他 | `null` |
| `CCE_ADD_BALANCE` | float | 现金及现金等价物净增加额平衡项 | `0` |
| `CCE_ADD` | float | 现金及现金等价物净增加额 | `21409000000` |
| `BEGIN_CCE` | float | 期初现金及现金等价物余额 | `23288000000` |
| `END_CCE_OTHER` | float? | 期末现金及现金等价物余额其他 | `null` |
| `END_CCE_BALANCE` | float | 期末现金及现金等价物余额平衡项 | `0` |
| `END_CCE` | float | 期末现金及现金等价物余额 | `44697000000` |
| `NETPROFIT` | float? | 净利润（间接法起点） | `null` |
| `ASSET_IMPAIRMENT` | float? | 资产减值准备 | `null` |
| `FA_IR_DEPR` | float? | 固定资产折旧、油气资产折耗、生产性生物资产折旧 | `null` |
| `OILGAS_BIOLOGY_DEPR` | float? | 油气资产折耗、生产性生物资产折旧 | `null` |
| `IR_DEPR` | float? | 折旧与摊销 | `null` |
| `IA_AMORTIZE` | float? | 无形资产摊销 | `null` |
| `LPE_AMORTIZE` | float? | 长期待摊费用摊销 | `null` |
| `DEFER_INCOME_AMORTIZE` | float? | 待摊费用减少（减：增加） | `null` |
| `PREPAID_EXPENSE_REDUCE` | float? | 预提费用增加（减：减少） | `null` |
| `ACCRUED_EXPENSE_ADD` | float? | 预提费用变动 | `null` |
| `DISPOSAL_LONGASSET_LOSS` | float? | 处置固定资产等的损失 | `null` |
| `FA_SCRAP_LOSS` | float? | 固定资产报废损失 | `null` |
| `FAIRVALUE_CHANGE_LOSS` | float? | 公允价值变动损失 | `null` |
| `FINANCE_EXPENSE` | float? | 财务费用 | `null` |
| `INVEST_LOSS` | float? | 投资损失 | `null` |
| `DEFER_TAX` | float? | 递延所得税资产减少（增加以"-"号填列） | `null` |
| `DT_ASSET_REDUCE` | float? | 递延所得税资产减少 | `null` |
| `DT_LIAB_ADD` | float? | 递延所得税负债增加 | `null` |
| `PREDICT_LIAB_ADD` | float? | 预计负债增加 | `null` |
| `INVENTORY_REDUCE` | float? | 存货的减少（增加以"-"号填列） | `null` |
| `OPERATE_RECE_REDUCE` | float? | 经营性应收项目的减少 | `null` |
| `OPERATE_PAYABLE_ADD` | float? | 经营性应付项目的增加 | `null` |
| `OTHER` | float? | 其他 | `null` |
| `OPERATE_NETCASH_OTHERNOTE` | float? | 经营活动产生的现金流量净额（附注） | `null` |
| `OPERATE_NETCASH_BALANCENOTE` | float? | 经营活动净现金流量（附注）平衡项 | `null` |
| `NETCASH_OPERATENOTE` | float? | 经营活动产生的现金流量净额（附注） | `null` |
| `DEBT_TRANSFER_CAPITAL` | float? | 债务转为资本 | `null` |
| `CONVERT_BOND_1YEAR` | float? | 一年内到期的可转换公司债券 | `null` |
| `FINLEASE_OBTAIN_FA` | float? | 融资租入固定资产 | `null` |
| `UNINVOLVE_INVESTFIN_OTHER` | float? | 不涉及现金收支的投资和筹资活动其他 | `null` |
| `END_CASH` | float? | 现金期末余额 | `null` |
| `BEGIN_CASH` | float? | 现金期初余额 | `null` |
| `END_CASH_EQUIVALENTS` | float? | 现金等价物期末余额 | `null` |
| `BEGIN_CASH_EQUIVALENTS` | float? | 现金等价物期初余额 | `null` |
| `CCE_ADD_OTHERNOTE` | float? | 现金及现金等价物净增加额（附注） | `null` |
| `CCE_ADD_BALANCENOTE` | float? | 现金及现金等价物净增加额（附注）平衡项 | `null` |
| `CCE_ADDNOTE` | float? | 现金及现金等价物净增加额（附注） | `null` |
| `MINORITY_INTEREST` | float? | 少数股东损益 | `null` |
| `USERIGHT_ASSET_AMORTIZE` | float? | 使用权资产摊销 | `null` |

### 同比增长率字段（`_YOY` 后缀，百分比）

| 字段 | 示例 |
|:-----|:-----|
| `SALES_SERVICES_YOY` | `2.4966092991` |
| `DEPOSIT_INTERBANK_ADD_YOY` | `null` |
| `LOAN_PBC_ADD_YOY` | `null` |
| `OFI_BF_ADD_YOY` | `null` |
| `RECEIVE_ORIGIC_PREMIUM_YOY` | `null` |
| `RECEIVE_REINSURE_NET_YOY` | `null` |
| `INSURED_INVEST_ADD_YOY` | `null` |
| `DISPOSAL_TFA_ADD_YOY` | `null` |
| `RECEIVE_INTEREST_COMMISSION_YOY` | `null` |
| `BORROW_FUND_ADD_YOY` | `null` |
| `LOAN_ADVANCE_REDUCE_YOY` | `null` |
| `REPO_BUSINESS_ADD_YOY` | `null` |
| `RECEIVE_TAX_REFUND_YOY` | `null` |
| `RECEIVE_OTHER_OPERATE_YOY` | `-12.0579463144` |
| `OPERATE_INFLOW_OTHER_YOY` | `null` |
| `OPERATE_INFLOW_BALANCE_YOY` | `null` |
| `TOTAL_OPERATE_INFLOW_YOY` | `2.0602886703` |
| `BUY_SERVICES_YOY` | `13.9780472975` |
| `LOAN_ADVANCE_ADD_YOY` | `null` |
| `PBC_INTERBANK_ADD_YOY` | `null` |
| `PAY_ORIGIC_COMPENSATE_YOY` | `null` |
| `PAY_INTEREST_COMMISSION_YOY` | `null` |
| `PAY_POLICY_BONUS_YOY` | `null` |
| `PAY_STAFF_CASH_YOY` | `2.4075222153` |
| `PAY_ALL_TAX_YOY` | `3.1686046512` |
| `PAY_OTHER_OPERATE_YOY` | `-8.1239804241` |
| `OPERATE_OUTFLOW_OTHER_YOY` | `null` |
| `OPERATE_OUTFLOW_BALANCE_YOY` | `null` |
| `TOTAL_OPERATE_OUTFLOW_YOY` | `8.2906219698` |
| `OPERATE_NETCASH_OTHER_YOY` | `null` |
| `OPERATE_NETCASH_BALANCE_YOY` | `null` |
| `NETCASH_OPERATE_YOY` | `-15.4591488947` |
| `WITHDRAW_INVEST_YOY` | `-99.1514947179` |
| `RECEIVE_INVEST_INCOME_YOY` | `-0.8450704225` |
| `DISPOSAL_LONG_ASSET_YOY` | `-25.6756756757` |
| `DISPOSAL_SUBSIDIARY_OTHER_YOY` | `-100` |
| `REDUCE_PLEDGE_TIMEDEPOSITS_YOY` | `null` |
| `RECEIVE_OTHER_INVEST_YOY` | `-25.6026600166` |
| `INVEST_INFLOW_OTHER_YOY` | `null` |
| `INVEST_INFLOW_BALANCE_YOY` | `null` |
| `TOTAL_INVEST_INFLOW_YOY` | `-82.9837752275` |
| `CONSTRUCT_LONG_ASSET_YOY` | `-17.123583845` |
| `INVEST_PAY_CASH_YOY` | `-70.3048293184` |
| `PLEDGE_LOAN_ADD_YOY` | `null` |
| `OBTAIN_SUBSIDIARY_OTHER_YOY` | `10561.4302461899` |
| `ADD_PLEDGE_TIMEDEPOSITS_YOY` | `null` |
| `PAY_OTHER_INVEST_YOY` | `8.9562764457` |
| `INVEST_OUTFLOW_OTHER_YOY` | `null` |
| `INVEST_OUTFLOW_BALANCE_YOY` | `null` |
| `TOTAL_INVEST_OUTFLOW_YOY` | `342.3639165383` |
| `INVEST_NETCASH_OTHER_YOY` | `null` |
| `INVEST_NETCASH_BALANCE_YOY` | `null` |
| `NETCASH_INVEST_YOY` | `-7971.4511041009` |
| `ACCEPT_INVEST_CASH_YOY` | `21560.2150537634` |
| `SUBSIDIARY_ACCEPT_INVEST_YOY` | `91.3978494624` |
| `RECEIVE_LOAN_CASH_YOY` | `4538.194109126` |
| `ISSUE_BOND_YOY` | `null` |
| `RECEIVE_OTHER_FINANCE_YOY` | `null` |
| `FINANCE_INFLOW_OTHER_YOY` | `null` |
| `FINANCE_INFLOW_BALANCE_YOY` | `null` |
| `TOTAL_FINANCE_INFLOW_YOY` | `5269.7319778189` |
| `PAY_DEBT_CASH_YOY` | `1.8850029638` |
| `ASSIGN_DIVIDEND_PORFIT_YOY` | `88.3435582822` |
| `SUBSIDIARY_PAY_DIVIDEND_YOY` | `4408.3333333333` |
| `BUY_SUBSIDIARY_EQUITY_YOY` | `null` |
| `PAY_OTHER_FINANCE_YOY` | `-6.7901234568` |
| `SUBSIDIARY_REDUCE_CASH_YOY` | `null` |
| `FINANCE_OUTFLOW_OTHER_YOY` | `null` |
| `FINANCE_OUTFLOW_BALANCE_YOY` | `null` |
| `TOTAL_FINANCE_OUTFLOW_YOY` | `6.3834470614` |
| `FINANCE_NETCASH_OTHER_YOY` | `null` |
| `FINANCE_NETCASH_BALANCE_YOY` | `null` |
| `NETCASH_FINANCE_YOY` | `1639.0783010691` |
| `RATE_CHANGE_EFFECT_YOY` | `-652.6315789474` |
| `CCE_ADD_OTHER_YOY` | `null` |
| `CCE_ADD_BALANCE_YOY` | `null` |
| `CCE_ADD_YOY` | `73.6474977695` |
| `BEGIN_CCE_YOY` | `-64.9345760619` |
| `END_CCE_OTHER_YOY` | `null` |
| `END_CCE_BALANCE_YOY` | `null` |
| `END_CCE_YOY` | `-43.2361382744` |
| `NETPROFIT_YOY` | `null` |
| `ASSET_IMPAIRMENT_YOY` | `null` |
| `FA_IR_DEPR_YOY` | `null` |
| `OILGAS_BIOLOGY_DEPR_YOY` | `null` |
| `IR_DEPR_YOY` | `null` |
| `IA_AMORTIZE_YOY` | `null` |
| `LPE_AMORTIZE_YOY` | `null` |
| `DEFER_INCOME_AMORTIZE_YOY` | `null` |
| `PREPAID_EXPENSE_REDUCE_YOY` | `null` |
| `ACCRUED_EXPENSE_ADD_YOY` | `null` |
| `DISPOSAL_LONGASSET_LOSS_YOY` | `null` |
| `FA_SCRAP_LOSS_YOY` | `null` |
| `FAIRVALUE_CHANGE_LOSS_YOY` | `null` |
| `FINANCE_EXPENSE_YOY` | `null` |
| `INVEST_LOSS_YOY` | `null` |
| `DEFER_TAX_YOY` | `null` |
| `DT_ASSET_REDUCE_YOY` | `null` |
| `DT_LIAB_ADD_YOY` | `null` |
| `PREDICT_LIAB_ADD_YOY` | `null` |
| `INVENTORY_REDUCE_YOY` | `null` |
| `OPERATE_RECE_REDUCE_YOY` | `null` |
| `OPERATE_PAYABLE_ADD_YOY` | `null` |
| `OTHER_YOY` | `null` |
| `OPERATE_NETCASH_OTHERNOTE_YOY` | `null` |
| `OPERATE_NETCASH_BALANCENOTE_YOY` | `null` |
| `NETCASH_OPERATENOTE_YOY` | `null` |
| `DEBT_TRANSFER_CAPITAL_YOY` | `null` |
| `CONVERT_BOND_1YEAR_YOY` | `null` |
| `FINLEASE_OBTAIN_FA_YOY` | `null` |
| `UNINVOLVE_INVESTFIN_OTHER_YOY` | `null` |
| `END_CASH_YOY` | `null` |
| `BEGIN_CASH_YOY` | `null` |
| `END_CASH_EQUIVALENTS_YOY` | `null` |
| `BEGIN_CASH_EQUIVALENTS_YOY` | `null` |
| `CCE_ADD_OTHERNOTE_YOY` | `null` |
| `CCE_ADD_BALANCENOTE_YOY` | `null` |
| `CCE_ADDNOTE_YOY` | `null` |
| `MINORITY_INTEREST_YOY` | `null` |
| `USERIGHT_ASSET_AMORTIZE_YOY` | `null` |

## Filter DSL

`filter` 是东方财富数据中心自带的查询表达式。多个条件直接拼接，无显式 AND/OR 运算符。

**按证券代码**：

```
(SECUCODE="601088.SH")
```

证券代码格式：`{6位代码}.{市场}`，沪市 `SH`，深市 `SZ`。

**按报告期精确匹配**：

```
(SECUCODE="601088.SH")(REPORT_DATE in ('2026-03-31','2025-12-31','2025-09-30'))
```

## Pagination

标准 URL 分页，通过 `p`（页码）/ `ps`（每页条数）控制。

## cURL 示例

```bash
curl -s 'https://datacenter.eastmoney.com/securities/api/data/get?\
type=RPT_F10_FINANCE_GCASHFLOW&\
sty=APP_F10_GCASHFLOW&\
filter=(SECUCODE="601088.SH")&\
p=1&\
ps=5&\
sr=-1&\
st=REPORT_DATE&\
source=HSF10&\
client=PC'
```

## Sample Response

`601088.SH` 2026Q1（`ps=1` 时第 1 条）：

```json
{
  "SECUCODE": "601088.SH",
  "SECURITY_CODE": "601088",
  "SECURITY_NAME_ABBR": "中国神华",
  "REPORT_DATE": "2026-03-31 00:00:00",
  "REPORT_TYPE": "一季报",
  "REPORT_DATE_NAME": "2026一季报",
  "NOTICE_DATE": "2026-04-25 00:00:00",
  "CURRENCY": "CNY",
  "SALES_SERVICES": 77839000000,
  "DEPOSIT_INTERBANK_ADD": null,
  "LOAN_PBC_ADD": null,
  "OFI_BF_ADD": null,
  "RECEIVE_ORIGIC_PREMIUM": null,
  "RECEIVE_REINSURE_NET": null,
  "INSURED_INVEST_ADD": null,
  "DISPOSAL_TFA_ADD": null
}
```
