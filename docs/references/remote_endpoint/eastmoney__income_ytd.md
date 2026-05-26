# eastmoney_income_ytd

东方财富 F10 — 利润表（累计）

## Endpoint

```
GET https://datacenter.eastmoney.com/securities/api/data/get
```

无需认证，直接 GET 请求。

> **注意**：本接口使用 `data/get`（无版本号），参数风格与 `data/v1/get` 不同。
> 参数名为 `type`/`sty`/`p`/`ps`/`sr`/`st`，而非 `reportName`/`columns`/`pageNumber`/`pageSize`/`sortColumns`/`sortTypes`。

**与单季度版（`income_sq`）的区别**：本接口返回**累计**口径数据，同比增长率使用 `_YOY` 后缀（无 `_QOQ`）。

## Query Parameters

| 参数 | 必填 | 说明 | 示例值 |
|:-----|:-----|:-----|:-------|
| `type` | 是 | 报表标识，固定值 | `RPT_F10_FINANCE_GINCOME` |
| `sty` | 是 | 返回字段样式，固定值 | `APP_F10_GINCOME` |
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

### 绝对值字段（金额，单位：元）

| 字段 | 类型 | 说明 | 示例 |
|:-----|:-----|:-----|:-----|
| `TOTAL_OPERATE_INCOME` | float | 营业总收入 | `70397000000` |
| `OPERATE_INCOME` | float | 营业收入 | `70397000000` |
| `INTEREST_INCOME` | float? | 利息收入 | `null` |
| `EARNED_PREMIUM` | float? | 已赚保费 | `null` |
| `FEE_COMMISSION_INCOME` | float? | 手续费及佣金收入 | `null` |
| `OTHER_BUSINESS_INCOME` | float? | 其他业务收入 | `null` |
| `TOI_OTHER` | float? | 营业总收入其他 | `null` |
| `TOTAL_OPERATE_COST` | float | 营业总成本 | `54277000000` |
| `OPERATE_COST` | float | 营业成本 | `47244000000` |
| `INTEREST_EXPENSE` | float? | 利息支出 | `null` |
| `FEE_COMMISSION_EXPENSE` | float? | 手续费及佣金支出 | `null` |
| `RESEARCH_EXPENSE` | float | 研发费用 | `333000000` |
| `SURRENDER_VALUE` | float? | 退保金 | `null` |
| `NET_COMPENSATE_EXPENSE` | float? | 分保费用 | `null` |
| `NET_CONTRACT_RESERVE` | float? | 提取保险合同准备金 | `null` |
| `POLICY_BONUS_EXPENSE` | float? | 保单红利支出 | `null` |
| `REINSURE_EXPENSE` | float? | 分保费用支出 | `null` |
| `OTHER_BUSINESS_COST` | float? | 其他业务成本 | `null` |
| `OPERATE_TAX_ADD` | float | 营业税金及附加 | `3884000000` |
| `SALE_EXPENSE` | float | 销售费用 | `114000000` |
| `MANAGE_EXPENSE` | float | 管理费用 | `2410000000` |
| `ME_RESEARCH_EXPENSE` | float? | 管理费用中的研发费用 | `null` |
| `FINANCE_EXPENSE` | float | 财务费用 | `292000000` |
| `FE_INTEREST_EXPENSE` | float | 财务费用之利息费用 | `634000000` |
| `FE_INTEREST_INCOME` | float | 财务费用之利息收入 | `455000000` |
| `ASSET_IMPAIRMENT_LOSS` | float? | 资产减值损失 | `null` |
| `CREDIT_IMPAIRMENT_LOSS` | float? | 信用减值损失 | `null` |
| `TOC_OTHER` | float? | 营业总成本其他 | `null` |
| `FAIRVALUE_CHANGE_INCOME` | float | 公允价值变动收益 | `0` |
| `INVEST_INCOME` | float | 投资收益 | `443000000` |
| `INVEST_JOINT_INCOME` | float | 对联营企业和合营企业的投资收益 | `438000000` |
| `NET_EXPOSURE_INCOME` | float? | 净敞口收益 | `null` |
| `EXCHANGE_INCOME` | float? | 汇兑收益 | `null` |
| `ASSET_DISPOSAL_INCOME` | float | 资产处置收益 | `-8000000` |
| `ASSET_IMPAIRMENT_INCOME` | float | 资产减值收益 | `-3000000` |
| `CREDIT_IMPAIRMENT_INCOME` | float | 信用减值收益 | `23000000` |
| `OTHER_INCOME` | float | 其他收益 | `62000000` |
| `OPERATE_PROFIT_OTHER` | float? | 营业利润其他 | `null` |
| `OPERATE_PROFIT_BALANCE` | float | 营业利润平衡项 | `0` |
| `OPERATE_PROFIT` | float | 营业利润 | `16637000000` |
| `NONBUSINESS_INCOME` | float | 营业外收入 | `64000000` |
| `NONCURRENT_DISPOSAL_INCOME` | float? | 非流动资产处置净收益 | `null` |
| `NONBUSINESS_EXPENSE` | float | 营业外支出 | `107000000` |
| `NONCURRENT_DISPOSAL_LOSS` | float? | 非流动资产处置净损失 | `null` |
| `EFFECT_TP_OTHER` | float? | 影响利润总额其他 | `null` |
| `TOTAL_PROFIT_BALANCE` | float | 利润总额平衡项 | `0` |
| `TOTAL_PROFIT` | float | 利润总额 | `16594000000` |
| `INCOME_TAX` | float | 所得税费用 | `3282000000` |
| `EFFECT_NETPROFIT_OTHER` | float? | 影响净利润其他 | `null` |
| `EFFECT_NETPROFIT_BALANCE` | float? | 净利润平衡项 | `null` |
| `UNCONFIRM_INVEST_LOSS` | float? | 未确认投资损失 | `null` |
| `NETPROFIT` | float | 净利润 | `13312000000` |
| `PRECOMBINE_PROFIT` | float? | 合并前净损益 | `null` |
| `CONTINUED_NETPROFIT` | float | 持续经营净利润 | `13312000000` |
| `DISCONTINUED_NETPROFIT` | float | 终止经营净利润 | `0` |
| `PARENT_NETPROFIT` | float | 归属于母公司股东的净利润 | `10667000000` |
| `MINORITY_INTEREST` | float | 少数股东损益 | `2645000000` |
| `DEDUCT_PARENT_NETPROFIT` | float | 扣除非经常性损益后归属于母公司股东的净利润 | `10712000000` |
| `NETPROFIT_OTHER` | float? | 净利润其他 | `null` |
| `NETPROFIT_BALANCE` | float? | 净利润平衡项 | `null` |
| `BASIC_EPS` | float | 基本每股收益（元/股） | `0.53` |
| `DILUTED_EPS` | float | 稀释每股收益（元/股） | `0.53` |
| `OTHER_COMPRE_INCOME` | float | 其他综合收益总额 | `-347000000` |
| `PARENT_OCI` | float | 归属于母公司股东的其他综合收益 | `-296000000` |
| `MINORITY_OCI` | float | 归属于少数股东的其他综合收益 | `-51000000` |
| `PARENT_OCI_OTHER` | float? | 归母其他综合收益其他 | `null` |
| `PARENT_OCI_BALANCE` | float? | 归母其他综合收益平衡项 | `null` |
| `UNABLE_OCI` | float | 以后将重分类进损益的其他综合收益 | `-165000000` |
| `CREDITRISK_FAIRVALUE_CHANGE` | float? | 信用风险引起的公允价值变动 | `null` |
| `OTHERRIGHT_FAIRVALUE_CHANGE` | float? | 其他权益工具公允价值变动 | `null` |
| `SETUP_PROFIT_CHANGE` | float | 重分类调整变动 | `0` |
| `RIGHTLAW_UNABLE_OCI` | float | 权益法下不能重分类的其他综合收益 | `-165000000` |
| `UNABLE_OCI_OTHER` | float? | 不能重分类其他综合收益其他 | `null` |
| `UNABLE_OCI_BALANCE` | float? | 不能重分类其他综合收益平衡项 | `null` |
| `ABLE_OCI` | float | 以后将重分类进损益的其他综合收益（可重分类） | `-131000000` |
| `RIGHTLAW_ABLE_OCI` | float | 权益法下可重分类的其他综合收益 | `12000000` |
| `AFA_FAIRVALUE_CHANGE` | float? | 可供出售金融资产公允价值变动 | `null` |
| `HMI_AFA` | float? | 持有有待售资产公允价值变动 | `null` |
| `CASHFLOW_HEDGE_VALID` | float? | 现金流量套期有效部分 | `null` |
| `CREDITOR_FAIRVALUE_CHANGE` | float? | 债权投资公允价值变动 | `null` |
| `CREDITOR_IMPAIRMENT_RESERVE` | float? | 债权投资减值准备 | `null` |
| `FINANCE_OCI_AMT` | float? | 金融资产重分类金额 | `null` |
| `CONVERT_DIFF` | float | 外币报表折算差额 | `-143000000` |
| `ABLE_OCI_OTHER` | float? | 可重分类其他综合收益其他 | `null` |
| `ABLE_OCI_BALANCE` | float? | 可重分类其他综合收益平衡项 | `null` |
| `OCI_OTHER` | float? | 其他综合收益其他 | `null` |
| `OCI_BALANCE` | float? | 其他综合收益平衡项 | `null` |
| `TOTAL_COMPRE_INCOME` | float | 综合收益总额 | `12965000000` |
| `PARENT_TCI` | float | 归属于母公司股东的综合收益总额 | `10371000000` |
| `MINORITY_TCI` | float | 归属于少数股东的综合收益总额 | `2594000000` |
| `PRECOMBINE_TCI` | float? | 合并前综合收益总额 | `null` |
| `EFFECT_TCI_BALANCE` | float? | 综合收益总额平衡项 | `null` |
| `TCI_OTHER` | float? | 综合收益总额其他 | `null` |
| `TCI_BALANCE` | float? | 综合收益总额平衡项 | `null` |
| `ACF_END_INCOME` | float? | 持续经营终止经营净损益 | `null` |

### 同比增长率字段（`_YOY` 后缀，百分比）

| 字段 | 示例 |
|:-----|:-----|
| `TOTAL_OPERATE_INCOME_YOY` | `1.1669181576` |
| `OPERATE_INCOME_YOY` | `1.1669181576` |
| `INTEREST_INCOME_YOY` | `null` |
| `EARNED_PREMIUM_YOY` | `null` |
| `FEE_COMMISSION_INCOME_YOY` | `null` |
| `OTHER_BUSINESS_INCOME_YOY` | `null` |
| `TOI_OTHER_YOY` | `null` |
| `TOTAL_OPERATE_COST_YOY` | `2.2531602645` |
| `OPERATE_COST_YOY` | `2.968484373` |
| `INTEREST_EXPENSE_YOY` | `null` |
| `FEE_COMMISSION_EXPENSE_YOY` | `null` |
| `RESEARCH_EXPENSE_YOY` | `38.1742738589` |
| `SURRENDER_VALUE_YOY` | `null` |
| `NET_COMPENSATE_EXPENSE_YOY` | `null` |
| `NET_CONTRACT_RESERVE_YOY` | `null` |
| `POLICY_BONUS_EXPENSE_YOY` | `null` |
| `REINSURE_EXPENSE_YOY` | `null` |
| `OTHER_BUSINESS_COST_YOY` | `null` |
| `OPERATE_TAX_ADD_YOY` | `-2.2155085599` |
| `SALE_EXPENSE_YOY` | `-12.9770992366` |
| `MANAGE_EXPENSE_YOY` | `-10.6083086053` |
| `ME_RESEARCH_EXPENSE_YOY` | `null` |
| `FINANCE_EXPENSE_YOY` | `83.6477987421` |
| `FE_INTEREST_EXPENSE_YOY` | `-4.084720121` |
| `FE_INTEREST_INCOME_YOY` | `-27.4322169059` |
| `ASSET_IMPAIRMENT_LOSS_YOY` | `null` |
| `CREDIT_IMPAIRMENT_LOSS_YOY` | `null` |
| `TOC_OTHER_YOY` | `null` |
| `FAIRVALUE_CHANGE_INCOME_YOY` | `-100` |
| `INVEST_INCOME_YOY` | `-62.2335890878` |
| `INVEST_JOINT_INCOME_YOY` | `-50.8417508418` |
| `NET_EXPOSURE_INCOME_YOY` | `null` |
| `EXCHANGE_INCOME_YOY` | `null` |
| `ASSET_DISPOSAL_INCOME_YOY` | `-300` |
| `ASSET_IMPAIRMENT_INCOME_YOY` | `-125` |
| `CREDIT_IMPAIRMENT_INCOME_YOY` | `43.75` |
| `OTHER_INCOME_YOY` | `-6.0606060606` |
| `OPERATE_PROFIT_OTHER_YOY` | `null` |
| `OPERATE_PROFIT_BALANCE_YOY` | `null` |
| `OPERATE_PROFIT_YOY` | `-6.4285714286` |
| `NONBUSINESS_INCOME_YOY` | `-54.9295774648` |
| `NONCURRENT_DISPOSAL_INCOME_YOY` | `null` |
| `NONBUSINESS_EXPENSE_YOY` | `233.75` |
| `NONCURRENT_DISPOSAL_LOSS_YOY` | `null` |
| `EFFECT_TP_OTHER_YOY` | `null` |
| `TOTAL_PROFIT_BALANCE_YOY` | `null` |
| `TOTAL_PROFIT_YOY` | `-7.821353183` |
| `INCOME_TAX_YOY` | `-1.7365269461` |
| `EFFECT_NETPROFIT_OTHER_YOY` | `null` |
| `EFFECT_NETPROFIT_BALANCE_YOY` | `null` |
| `UNCONFIRM_INVEST_LOSS_YOY` | `null` |
| `NETPROFIT_YOY` | `-9.2074751057` |
| `PRECOMBINE_PROFIT_YOY` | `null` |
| `CONTINUED_NETPROFIT_YOY` | `-9.2074751057` |
| `DISCONTINUED_NETPROFIT_YOY` | `null` |
| `PARENT_NETPROFIT_YOY` | `-10.7289312913` |
| `MINORITY_INTEREST_YOY` | `-2.5064504239` |
| `DEDUCT_PARENT_NETPROFIT_YOY` | `-8.4835540367` |
| `NETPROFIT_OTHER_YOY` | `null` |
| `NETPROFIT_BALANCE_YOY` | `null` |
| `BASIC_EPS_YOY` | `-11.8136439268` |
| `DILUTED_EPS_YOY` | `-11.8136439268` |
| `OTHER_COMPRE_INCOME_YOY` | `-1752.380952381` |
| `PARENT_OCI_YOY` | `-1025` |
| `MINORITY_OCI_YOY` | `-363.6363636364` |
| `PARENT_OCI_OTHER_YOY` | `null` |
| `PARENT_OCI_BALANCE_YOY` | `null` |
| `UNABLE_OCI_YOY` | `-235.2459016393` |
| `CREDITRISK_FAIRVALUE_CHANGE_YOY` | `null` |
| `OTHERRIGHT_FAIRVALUE_CHANGE_YOY` | `null` |
| `SETUP_PROFIT_CHANGE_YOY` | `null` |
| `RIGHTLAW_UNABLE_OCI_YOY` | `-235.2459016393` |
| `UNABLE_OCI_OTHER_YOY` | `null` |
| `UNABLE_OCI_BALANCE_YOY` | `null` |
| `ABLE_OCI_YOY` | `-45.5555555556` |
| `RIGHTLAW_ABLE_OCI_YOY` | `116` |
| `AFA_FAIRVALUE_CHANGE_YOY` | `null` |
| `HMI_AFA_YOY` | `null` |
| `CASHFLOW_HEDGE_VALID_YOY` | `null` |
| `CREDITOR_FAIRVALUE_CHANGE_YOY` | `null` |
| `CREDITOR_IMPAIRMENT_RESERVE_YOY` | `null` |
| `FINANCE_OCI_AMT_YOY` | `null` |
| `CONVERT_DIFF_YOY` | `-853.3333333333` |
| `ABLE_OCI_OTHER_YOY` | `null` |
| `ABLE_OCI_BALANCE_YOY` | `null` |
| `OCI_OTHER_YOY` | `null` |
| `OCI_BALANCE_YOY` | `null` |
| `TOTAL_COMPRE_INCOME_YOY` | `-11.7006061432` |
| `PARENT_TCI_YOY` | `-13.4379434104` |
| `MINORITY_TCI_YOY` | `-3.9970392302` |
| `PRECOMBINE_TCI_YOY` | `null` |
| `EFFECT_TCI_BALANCE_YOY` | `null` |
| `TCI_OTHER_YOY` | `null` |
| `TCI_BALANCE_YOY` | `null` |
| `ACF_END_INCOME_YOY` | `null` |

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
type=RPT_F10_FINANCE_GINCOME&\
sty=APP_F10_GINCOME&\
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
  "TOTAL_OPERATE_INCOME": 70397000000,
  "OPERATE_INCOME": 70397000000,
  "INTEREST_INCOME": null,
  "EARNED_PREMIUM": null,
  "FEE_COMMISSION_INCOME": null,
  "OTHER_BUSINESS_INCOME": null,
  "TOI_OTHER": null,
  "TOTAL_OPERATE_COST": 54277000000,
  "OPERATE_COST": 47244000000,
  "INTEREST_EXPENSE": null
}
```
