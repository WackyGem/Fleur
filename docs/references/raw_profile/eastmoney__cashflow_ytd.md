# Raw 数据画像：eastmoney__cashflow_ytd

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__cashflow_ytd.yml`
- dbt source：`source('raw', 'eastmoney__cashflow_ytd')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__cashflow_ytd`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__cashflow_ytd --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__cashflow_ytd.md`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`eastmoney__cashflow_ytd`
- ClickHouse raw 表：`fleur_raw.eastmoney__cashflow_ytd`
- 表说明：EastMoney year-to-date cashflow F10 rows by natural-year raw partition.

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
| CURRENCY | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CURRENCY`。 原始字段说明：现金流量表年初至报告期末金额使用的币种。 |
| SALES_SERVICES | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SALES_SERVICES`。 原始字段说明：销售商品、提供劳务收到的现金 |
| DEPOSIT_INTERBANK_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEPOSIT_INTERBANK_ADD`。 原始字段说明：同业存放净增加额 |
| LOAN_PBC_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_PBC_ADD`。 原始字段说明：向央行借款净增加额 |
| OFI_BF_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OFI_BF_ADD`。 原始字段说明：向其他金融机构拆入资金净增加额 |
| RECEIVE_ORIGIC_PREMIUM | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_ORIGIC_PREMIUM`。 原始字段说明：收到原保险合同保费现金 |
| RECEIVE_REINSURE_NET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_REINSURE_NET`。 原始字段说明：收到再保险业务现金净额 |
| INSURED_INVEST_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INSURED_INVEST_ADD`。 原始字段说明：保户储金及投资款净增加额 |
| DISPOSAL_TFA_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISPOSAL_TFA_ADD`。 原始字段说明：处置交易性金融资产净增加额 |
| RECEIVE_INTEREST_COMMISSION | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_INTEREST_COMMISSION`。 原始字段说明：收取利息和手续费现金 |
| BORROW_FUND_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BORROW_FUND_ADD`。 原始字段说明：拆入资金净增加额 |
| LOAN_ADVANCE_REDUCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_ADVANCE_REDUCE`。 原始字段说明：发放贷款及垫款净减少额 |
| REPO_BUSINESS_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REPO_BUSINESS_ADD`。 原始字段说明：回购业务资金净增加额 |
| RECEIVE_TAX_REFUND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_TAX_REFUND`。 原始字段说明：收到的税费返还 |
| RECEIVE_OTHER_OPERATE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_OTHER_OPERATE`。 原始字段说明：收到其他与经营活动有关的现金 |
| OPERATE_INFLOW_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INFLOW_OTHER`。 原始字段说明：经营活动现金流入其他 |
| OPERATE_INFLOW_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INFLOW_BALANCE`。 原始字段说明：经营活动现金流入平衡项 |
| TOTAL_OPERATE_INFLOW | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_INFLOW`。 原始字段说明：经营活动现金流入小计 |
| BUY_SERVICES | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BUY_SERVICES`。 原始字段说明：购买商品、接受劳务支付的现金 |
| LOAN_ADVANCE_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_ADVANCE_ADD`。 原始字段说明：发放贷款及垫款净增加额 |
| PBC_INTERBANK_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PBC_INTERBANK_ADD`。 原始字段说明：向央行借款净增加额 |
| PAY_ORIGIC_COMPENSATE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_ORIGIC_COMPENSATE`。 原始字段说明：支付原保险合同赔付款项现金 |
| PAY_INTEREST_COMMISSION | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_INTEREST_COMMISSION`。 原始字段说明：支付利息和手续费现金 |
| PAY_POLICY_BONUS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_POLICY_BONUS`。 原始字段说明：保单红利支出 |
| PAY_STAFF_CASH | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_STAFF_CASH`。 原始字段说明：支付给职工以及为职工支付的现金 |
| PAY_ALL_TAX | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_ALL_TAX`。 原始字段说明：支付的各项税费 |
| PAY_OTHER_OPERATE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_OTHER_OPERATE`。 原始字段说明：支付其他与经营活动有关的现金 |
| OPERATE_OUTFLOW_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_OUTFLOW_OTHER`。 原始字段说明：经营活动现金流出其他 |
| OPERATE_OUTFLOW_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_OUTFLOW_BALANCE`。 原始字段说明：经营活动现金流出平衡项 |
| TOTAL_OPERATE_OUTFLOW | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_OUTFLOW`。 原始字段说明：经营活动现金流出小计 |
| OPERATE_NETCASH_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_NETCASH_OTHER`。 原始字段说明：经营活动净现金流量其他 |
| OPERATE_NETCASH_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_NETCASH_BALANCE`。 原始字段说明：经营活动净现金流量平衡项 |
| NETCASH_OPERATE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETCASH_OPERATE`。 原始字段说明：经营活动产生的现金流量净额 |
| WITHDRAW_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `WITHDRAW_INVEST`。 原始字段说明：收回投资收到的现金 |
| RECEIVE_INVEST_INCOME | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_INVEST_INCOME`。 原始字段说明：取得投资收益收到的现金 |
| DISPOSAL_LONG_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISPOSAL_LONG_ASSET`。 原始字段说明：处置固定资产等收回的现金净额 |
| DISPOSAL_SUBSIDIARY_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISPOSAL_SUBSIDIARY_OTHER`。 原始字段说明：处置子公司及其他营业单位收到的现金净额 |
| REDUCE_PLEDGE_TIMEDEPOSITS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REDUCE_PLEDGE_TIMEDEPOSITS`。 原始字段说明：减少质押定期存款 |
| RECEIVE_OTHER_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_OTHER_INVEST`。 原始字段说明：收到其他与投资活动有关的现金 |
| INVEST_INFLOW_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INFLOW_OTHER`。 原始字段说明：投资活动现金流入其他 |
| INVEST_INFLOW_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INFLOW_BALANCE`。 原始字段说明：投资活动现金流入平衡项 |
| TOTAL_INVEST_INFLOW | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_INVEST_INFLOW`。 原始字段说明：投资活动现金流入小计 |
| CONSTRUCT_LONG_ASSET | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONSTRUCT_LONG_ASSET`。 原始字段说明：购建固定资产等支付的现金 |
| INVEST_PAY_CASH | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_PAY_CASH`。 原始字段说明：投资支付的现金 |
| PLEDGE_LOAN_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PLEDGE_LOAN_ADD`。 原始字段说明：质押贷款净增加额 |
| OBTAIN_SUBSIDIARY_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OBTAIN_SUBSIDIARY_OTHER`。 原始字段说明：取得子公司及其他营业单位支付的现金净额 |
| ADD_PLEDGE_TIMEDEPOSITS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ADD_PLEDGE_TIMEDEPOSITS`。 原始字段说明：增加质押定期存款 |
| PAY_OTHER_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_OTHER_INVEST`。 原始字段说明：支付其他与投资活动有关的现金 |
| INVEST_OUTFLOW_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_OUTFLOW_OTHER`。 原始字段说明：投资活动现金流出其他 |
| INVEST_OUTFLOW_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_OUTFLOW_BALANCE`。 原始字段说明：投资活动现金流出平衡项 |
| TOTAL_INVEST_OUTFLOW | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_INVEST_OUTFLOW`。 原始字段说明：投资活动现金流出小计 |
| INVEST_NETCASH_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_NETCASH_OTHER`。 原始字段说明：投资活动净现金流量其他 |
| INVEST_NETCASH_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_NETCASH_BALANCE`。 原始字段说明：投资活动净现金流量平衡项 |
| NETCASH_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETCASH_INVEST`。 原始字段说明：投资活动产生的现金流量净额 |
| ACCEPT_INVEST_CASH | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCEPT_INVEST_CASH`。 原始字段说明：吸收投资收到的现金 |
| SUBSIDIARY_ACCEPT_INVEST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SUBSIDIARY_ACCEPT_INVEST`。 原始字段说明：子公司吸收少数股东投资收到的现金 |
| RECEIVE_LOAN_CASH | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_LOAN_CASH`。 原始字段说明：取得借款收到的现金 |
| ISSUE_BOND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ISSUE_BOND`。 原始字段说明：发行债券收到的现金 |
| RECEIVE_OTHER_FINANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_OTHER_FINANCE`。 原始字段说明：收到其他与筹资活动有关的现金 |
| FINANCE_INFLOW_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_INFLOW_OTHER`。 原始字段说明：筹资活动现金流入其他 |
| FINANCE_INFLOW_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_INFLOW_BALANCE`。 原始字段说明：筹资活动现金流入平衡项 |
| TOTAL_FINANCE_INFLOW | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_FINANCE_INFLOW`。 原始字段说明：筹资活动现金流入小计 |
| PAY_DEBT_CASH | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_DEBT_CASH`。 原始字段说明：偿还债务支付的现金 |
| ASSIGN_DIVIDEND_PORFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSIGN_DIVIDEND_PORFIT`。 原始字段说明：分配股利、利润或偿付利息支付的现金 |
| SUBSIDIARY_PAY_DIVIDEND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SUBSIDIARY_PAY_DIVIDEND`。 原始字段说明：子公司向少数股东支付的现金股利 |
| BUY_SUBSIDIARY_EQUITY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BUY_SUBSIDIARY_EQUITY`。 原始字段说明：子公司减少现金 |
| PAY_OTHER_FINANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_OTHER_FINANCE`。 原始字段说明：支付其他与筹资活动有关的现金 |
| SUBSIDIARY_REDUCE_CASH | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SUBSIDIARY_REDUCE_CASH`。 原始字段说明：子公司减少现金 |
| FINANCE_OUTFLOW_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OUTFLOW_OTHER`。 原始字段说明：筹资活动现金流出其他 |
| FINANCE_OUTFLOW_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OUTFLOW_BALANCE`。 原始字段说明：筹资活动现金流出平衡项 |
| TOTAL_FINANCE_OUTFLOW | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_FINANCE_OUTFLOW`。 原始字段说明：筹资活动现金流出小计 |
| FINANCE_NETCASH_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_NETCASH_OTHER`。 原始字段说明：筹资活动净现金流量其他 |
| FINANCE_NETCASH_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_NETCASH_BALANCE`。 原始字段说明：筹资活动净现金流量平衡项 |
| NETCASH_FINANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETCASH_FINANCE`。 原始字段说明：筹资活动产生的现金流量净额 |
| RATE_CHANGE_EFFECT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RATE_CHANGE_EFFECT`。 原始字段说明：汇率变动对现金及现金等价物的影响 |
| CCE_ADD_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_OTHER`。 原始字段说明：现金及现金等价物净增加额其他 |
| CCE_ADD_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_BALANCE`。 原始字段说明：现金及现金等价物净增加额平衡项 |
| CCE_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD`。 原始字段说明：现金及现金等价物净增加额 |
| BEGIN_CCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BEGIN_CCE`。 原始字段说明：期初现金及现金等价物余额 |
| END_CCE_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CCE_OTHER`。 原始字段说明：期末现金及现金等价物余额其他 |
| END_CCE_BALANCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CCE_BALANCE`。 原始字段说明：期末现金及现金等价物余额平衡项 |
| END_CCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CCE`。 原始字段说明：期末现金及现金等价物余额 |
| NETPROFIT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT`。 原始字段说明：净利润（间接法起点） |
| ASSET_IMPAIRMENT | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT`。 原始字段说明：资产减值准备 |
| FA_IR_DEPR | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FA_IR_DEPR`。 原始字段说明：固定资产折旧、油气资产折耗、生产性生物资产折旧 |
| OILGAS_BIOLOGY_DEPR | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OILGAS_BIOLOGY_DEPR`。 原始字段说明：油气资产折耗、生产性生物资产折旧 |
| IR_DEPR | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IR_DEPR`。 原始字段说明：折旧与摊销 |
| IA_AMORTIZE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IA_AMORTIZE`。 原始字段说明：无形资产摊销 |
| LPE_AMORTIZE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LPE_AMORTIZE`。 原始字段说明：长期待摊费用摊销 |
| DEFER_INCOME_AMORTIZE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_INCOME_AMORTIZE`。 原始字段说明：待摊费用减少（减：增加） |
| PREPAID_EXPENSE_REDUCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREPAID_EXPENSE_REDUCE`。 原始字段说明：预提费用增加（减：减少） |
| ACCRUED_EXPENSE_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCRUED_EXPENSE_ADD`。 原始字段说明：预提费用变动 |
| DISPOSAL_LONGASSET_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISPOSAL_LONGASSET_LOSS`。 原始字段说明：处置固定资产等的损失 |
| FA_SCRAP_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FA_SCRAP_LOSS`。 原始字段说明：固定资产报废损失 |
| FAIRVALUE_CHANGE_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FAIRVALUE_CHANGE_LOSS`。 原始字段说明：公允价值变动损失 |
| FINANCE_EXPENSE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_EXPENSE`。 原始字段说明：财务费用 |
| INVEST_LOSS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_LOSS`。 原始字段说明：投资损失 |
| DEFER_TAX | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_TAX`。 原始字段说明：递延所得税资产减少（增加以"-"号填列） |
| DT_ASSET_REDUCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DT_ASSET_REDUCE`。 原始字段说明：递延所得税资产减少 |
| DT_LIAB_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DT_LIAB_ADD`。 原始字段说明：递延所得税负债增加 |
| PREDICT_LIAB_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREDICT_LIAB_ADD`。 原始字段说明：预计负债增加 |
| INVENTORY_REDUCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVENTORY_REDUCE`。 原始字段说明：存货的减少（增加以"-"号填列） |
| OPERATE_RECE_REDUCE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_RECE_REDUCE`。 原始字段说明：经营性应收项目的减少 |
| OPERATE_PAYABLE_ADD | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PAYABLE_ADD`。 原始字段说明：经营性应付项目的增加 |
| OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER`。 原始字段说明：现金流量表年初至报告期末补充资料中的其他项目。 |
| OPERATE_NETCASH_OTHERNOTE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_NETCASH_OTHERNOTE`。 原始字段说明：经营活动产生的现金流量净额（附注） |
| OPERATE_NETCASH_BALANCENOTE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_NETCASH_BALANCENOTE`。 原始字段说明：经营活动净现金流量（附注）平衡项 |
| NETCASH_OPERATENOTE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETCASH_OPERATENOTE`。 原始字段说明：经营活动产生的现金流量净额（附注） |
| DEBT_TRANSFER_CAPITAL | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEBT_TRANSFER_CAPITAL`。 原始字段说明：债务转为资本 |
| CONVERT_BOND_1YEAR | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_BOND_1YEAR`。 原始字段说明：一年内到期的可转换公司债券 |
| FINLEASE_OBTAIN_FA | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINLEASE_OBTAIN_FA`。 原始字段说明：融资租入固定资产 |
| UNINVOLVE_INVESTFIN_OTHER | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNINVOLVE_INVESTFIN_OTHER`。 原始字段说明：不涉及现金收支的投资和筹资活动其他 |
| END_CASH | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CASH`。 原始字段说明：现金期末余额 |
| BEGIN_CASH | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BEGIN_CASH`。 原始字段说明：现金期初余额 |
| END_CASH_EQUIVALENTS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CASH_EQUIVALENTS`。 原始字段说明：现金等价物期末余额 |
| BEGIN_CASH_EQUIVALENTS | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BEGIN_CASH_EQUIVALENTS`。 原始字段说明：现金等价物期初余额 |
| CCE_ADD_OTHERNOTE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_OTHERNOTE`。 原始字段说明：现金及现金等价物净增加额（附注） |
| CCE_ADD_BALANCENOTE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_BALANCENOTE`。 原始字段说明：现金及现金等价物净增加额（附注）平衡项 |
| CCE_ADDNOTE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADDNOTE`。 原始字段说明：现金及现金等价物净增加额（附注） |
| SALES_SERVICES_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SALES_SERVICES_YOY`。 原始字段说明：销售商品、提供劳务收到的现金同比增长率（%） |
| DEPOSIT_INTERBANK_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEPOSIT_INTERBANK_ADD_YOY`。 原始字段说明：同业存放净增加额同比增长率（%） |
| LOAN_PBC_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_PBC_ADD_YOY`。 原始字段说明：向央行借款净增加额同比增长率（%） |
| OFI_BF_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OFI_BF_ADD_YOY`。 原始字段说明：向其他金融机构拆入资金净增加额同比增长率（%） |
| RECEIVE_ORIGIC_PREMIUM_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_ORIGIC_PREMIUM_YOY`。 原始字段说明：收到原保险合同保费现金同比增长率（%） |
| RECEIVE_REINSURE_NET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_REINSURE_NET_YOY`。 原始字段说明：收到再保险业务现金净额同比增长率（%） |
| INSURED_INVEST_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INSURED_INVEST_ADD_YOY`。 原始字段说明：保户储金及投资款净增加额同比增长率（%） |
| DISPOSAL_TFA_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISPOSAL_TFA_ADD_YOY`。 原始字段说明：处置交易性金融资产净增加额同比增长率（%） |
| RECEIVE_INTEREST_COMMISSION_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_INTEREST_COMMISSION_YOY`。 原始字段说明：收取利息和手续费现金同比增长率（%） |
| BORROW_FUND_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BORROW_FUND_ADD_YOY`。 原始字段说明：拆入资金净增加额同比增长率（%） |
| LOAN_ADVANCE_REDUCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_ADVANCE_REDUCE_YOY`。 原始字段说明：发放贷款及垫款净减少额同比增长率（%） |
| REPO_BUSINESS_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REPO_BUSINESS_ADD_YOY`。 原始字段说明：回购业务资金净增加额同比增长率（%） |
| RECEIVE_TAX_REFUND_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_TAX_REFUND_YOY`。 原始字段说明：收到的税费返还同比增长率（%） |
| RECEIVE_OTHER_OPERATE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_OTHER_OPERATE_YOY`。 原始字段说明：收到其他与经营活动有关的现金同比增长率（%） |
| OPERATE_INFLOW_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INFLOW_OTHER_YOY`。 原始字段说明：经营活动现金流入其他同比增长率（%） |
| OPERATE_INFLOW_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_INFLOW_BALANCE_YOY`。 原始字段说明：经营活动现金流入平衡项同比增长率（%） |
| TOTAL_OPERATE_INFLOW_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_INFLOW_YOY`。 原始字段说明：经营活动现金流入小计同比增长率（%） |
| BUY_SERVICES_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BUY_SERVICES_YOY`。 原始字段说明：购买商品、接受劳务支付的现金同比增长率（%） |
| LOAN_ADVANCE_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LOAN_ADVANCE_ADD_YOY`。 原始字段说明：发放贷款及垫款净增加额同比增长率（%） |
| PBC_INTERBANK_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PBC_INTERBANK_ADD_YOY`。 原始字段说明：向央行借款净增加额同比增长率（%） |
| PAY_ORIGIC_COMPENSATE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_ORIGIC_COMPENSATE_YOY`。 原始字段说明：支付原保险合同赔付款项现金同比增长率（%） |
| PAY_INTEREST_COMMISSION_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_INTEREST_COMMISSION_YOY`。 原始字段说明：支付利息和手续费现金同比增长率（%） |
| PAY_POLICY_BONUS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_POLICY_BONUS_YOY`。 原始字段说明：保单红利支出同比增长率（%） |
| PAY_STAFF_CASH_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_STAFF_CASH_YOY`。 原始字段说明：支付给职工以及为职工支付的现金同比增长率（%） |
| PAY_ALL_TAX_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_ALL_TAX_YOY`。 原始字段说明：支付的各项税费同比增长率（%） |
| PAY_OTHER_OPERATE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_OTHER_OPERATE_YOY`。 原始字段说明：支付其他与经营活动有关的现金同比增长率（%） |
| OPERATE_OUTFLOW_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_OUTFLOW_OTHER_YOY`。 原始字段说明：经营活动现金流出其他同比增长率（%） |
| OPERATE_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_OUTFLOW_BALANCE_YOY`。 原始字段说明：经营活动现金流出平衡项同比增长率（%） |
| TOTAL_OPERATE_OUTFLOW_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_OPERATE_OUTFLOW_YOY`。 原始字段说明：经营活动现金流出小计同比增长率（%） |
| OPERATE_NETCASH_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_NETCASH_OTHER_YOY`。 原始字段说明：经营活动净现金流量其他同比增长率（%） |
| OPERATE_NETCASH_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_NETCASH_BALANCE_YOY`。 原始字段说明：经营活动净现金流量平衡项同比增长率（%） |
| NETCASH_OPERATE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETCASH_OPERATE_YOY`。 原始字段说明：经营活动产生的现金流量净额同比增长率（%） |
| WITHDRAW_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `WITHDRAW_INVEST_YOY`。 原始字段说明：收回投资收到的现金同比增长率（%） |
| RECEIVE_INVEST_INCOME_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_INVEST_INCOME_YOY`。 原始字段说明：取得投资收益收到的现金同比增长率（%） |
| DISPOSAL_LONG_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISPOSAL_LONG_ASSET_YOY`。 原始字段说明：处置固定资产等收回的现金净额同比增长率（%） |
| DISPOSAL_SUBSIDIARY_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISPOSAL_SUBSIDIARY_OTHER_YOY`。 原始字段说明：处置子公司及其他营业单位收到的现金净额同比增长率（%） |
| REDUCE_PLEDGE_TIMEDEPOSITS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REDUCE_PLEDGE_TIMEDEPOSITS_YOY`。 原始字段说明：减少质押定期存款同比增长率（%） |
| RECEIVE_OTHER_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_OTHER_INVEST_YOY`。 原始字段说明：收到其他与投资活动有关的现金同比增长率（%） |
| INVEST_INFLOW_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INFLOW_OTHER_YOY`。 原始字段说明：投资活动现金流入其他同比增长率（%） |
| INVEST_INFLOW_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_INFLOW_BALANCE_YOY`。 原始字段说明：投资活动现金流入平衡项同比增长率（%） |
| TOTAL_INVEST_INFLOW_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_INVEST_INFLOW_YOY`。 原始字段说明：投资活动现金流入小计同比增长率（%） |
| CONSTRUCT_LONG_ASSET_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONSTRUCT_LONG_ASSET_YOY`。 原始字段说明：购建固定资产等支付的现金同比增长率（%） |
| INVEST_PAY_CASH_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_PAY_CASH_YOY`。 原始字段说明：投资支付的现金同比增长率（%） |
| PLEDGE_LOAN_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PLEDGE_LOAN_ADD_YOY`。 原始字段说明：质押贷款净增加额同比增长率（%） |
| OBTAIN_SUBSIDIARY_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OBTAIN_SUBSIDIARY_OTHER_YOY`。 原始字段说明：取得子公司及其他营业单位支付的现金净额同比增长率（%） |
| ADD_PLEDGE_TIMEDEPOSITS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ADD_PLEDGE_TIMEDEPOSITS_YOY`。 原始字段说明：增加质押定期存款同比增长率（%） |
| PAY_OTHER_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_OTHER_INVEST_YOY`。 原始字段说明：支付其他与投资活动有关的现金同比增长率（%） |
| INVEST_OUTFLOW_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_OUTFLOW_OTHER_YOY`。 原始字段说明：投资活动现金流出其他同比增长率（%） |
| INVEST_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_OUTFLOW_BALANCE_YOY`。 原始字段说明：投资活动现金流出平衡项同比增长率（%） |
| TOTAL_INVEST_OUTFLOW_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_INVEST_OUTFLOW_YOY`。 原始字段说明：投资活动现金流出小计同比增长率（%） |
| INVEST_NETCASH_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_NETCASH_OTHER_YOY`。 原始字段说明：投资活动净现金流量其他同比增长率（%） |
| INVEST_NETCASH_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_NETCASH_BALANCE_YOY`。 原始字段说明：投资活动净现金流量平衡项同比增长率（%） |
| NETCASH_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETCASH_INVEST_YOY`。 原始字段说明：投资活动产生的现金流量净额同比增长率（%） |
| ACCEPT_INVEST_CASH_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCEPT_INVEST_CASH_YOY`。 原始字段说明：吸收投资收到的现金同比增长率（%） |
| SUBSIDIARY_ACCEPT_INVEST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SUBSIDIARY_ACCEPT_INVEST_YOY`。 原始字段说明：子公司吸收少数股东投资收到的现金同比增长率（%） |
| RECEIVE_LOAN_CASH_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_LOAN_CASH_YOY`。 原始字段说明：取得借款收到的现金同比增长率（%） |
| ISSUE_BOND_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ISSUE_BOND_YOY`。 原始字段说明：发行债券收到的现金同比增长率（%） |
| RECEIVE_OTHER_FINANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RECEIVE_OTHER_FINANCE_YOY`。 原始字段说明：收到其他与筹资活动有关的现金同比增长率（%） |
| FINANCE_INFLOW_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_INFLOW_OTHER_YOY`。 原始字段说明：筹资活动现金流入其他同比增长率（%） |
| FINANCE_INFLOW_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_INFLOW_BALANCE_YOY`。 原始字段说明：筹资活动现金流入平衡项同比增长率（%） |
| TOTAL_FINANCE_INFLOW_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_FINANCE_INFLOW_YOY`。 原始字段说明：筹资活动现金流入小计同比增长率（%） |
| PAY_DEBT_CASH_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_DEBT_CASH_YOY`。 原始字段说明：偿还债务支付的现金同比增长率（%） |
| ASSIGN_DIVIDEND_PORFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSIGN_DIVIDEND_PORFIT_YOY`。 原始字段说明：分配股利、利润或偿付利息支付的现金同比增长率（%） |
| SUBSIDIARY_PAY_DIVIDEND_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SUBSIDIARY_PAY_DIVIDEND_YOY`。 原始字段说明：子公司向少数股东支付的现金股利同比增长率（%） |
| BUY_SUBSIDIARY_EQUITY_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BUY_SUBSIDIARY_EQUITY_YOY`。 原始字段说明：子公司减少现金同比增长率（%） |
| PAY_OTHER_FINANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_OTHER_FINANCE_YOY`。 原始字段说明：支付其他与筹资活动有关的现金同比增长率（%） |
| SUBSIDIARY_REDUCE_CASH_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SUBSIDIARY_REDUCE_CASH_YOY`。 原始字段说明：子公司减少现金同比增长率（%） |
| FINANCE_OUTFLOW_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OUTFLOW_OTHER_YOY`。 原始字段说明：筹资活动现金流出其他同比增长率（%） |
| FINANCE_OUTFLOW_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_OUTFLOW_BALANCE_YOY`。 原始字段说明：筹资活动现金流出平衡项同比增长率（%） |
| TOTAL_FINANCE_OUTFLOW_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_FINANCE_OUTFLOW_YOY`。 原始字段说明：筹资活动现金流出小计同比增长率（%） |
| FINANCE_NETCASH_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_NETCASH_OTHER_YOY`。 原始字段说明：筹资活动净现金流量其他同比增长率（%） |
| FINANCE_NETCASH_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_NETCASH_BALANCE_YOY`。 原始字段说明：筹资活动净现金流量平衡项同比增长率（%） |
| NETCASH_FINANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETCASH_FINANCE_YOY`。 原始字段说明：筹资活动产生的现金流量净额同比增长率（%） |
| RATE_CHANGE_EFFECT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `RATE_CHANGE_EFFECT_YOY`。 原始字段说明：汇率变动对现金及现金等价物的影响同比增长率（%） |
| CCE_ADD_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_OTHER_YOY`。 原始字段说明：现金及现金等价物净增加额其他同比增长率（%） |
| CCE_ADD_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_BALANCE_YOY`。 原始字段说明：现金及现金等价物净增加额平衡项同比增长率（%） |
| CCE_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_YOY`。 原始字段说明：现金及现金等价物净增加额同比增长率（%） |
| BEGIN_CCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BEGIN_CCE_YOY`。 原始字段说明：期初现金及现金等价物余额同比增长率（%） |
| END_CCE_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CCE_OTHER_YOY`。 原始字段说明：期末现金及现金等价物余额其他同比增长率（%） |
| END_CCE_BALANCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CCE_BALANCE_YOY`。 原始字段说明：期末现金及现金等价物余额平衡项同比增长率（%） |
| END_CCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CCE_YOY`。 原始字段说明：期末现金及现金等价物余额同比增长率（%） |
| NETPROFIT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETPROFIT_YOY`。 原始字段说明：净利润同比增长率（%） |
| ASSET_IMPAIRMENT_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSET_IMPAIRMENT_YOY`。 原始字段说明：资产减值准备同比增长率（%） |
| FA_IR_DEPR_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FA_IR_DEPR_YOY`。 原始字段说明：固定资产折旧、油气资产折耗、生产性生物资产折旧同比增长率（%） |
| OILGAS_BIOLOGY_DEPR_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OILGAS_BIOLOGY_DEPR_YOY`。 原始字段说明：油气资产折耗、生产性生物资产折旧同比增长率（%） |
| IR_DEPR_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IR_DEPR_YOY`。 原始字段说明：折旧与摊销同比增长率（%） |
| IA_AMORTIZE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IA_AMORTIZE_YOY`。 原始字段说明：无形资产摊销同比增长率（%） |
| LPE_AMORTIZE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LPE_AMORTIZE_YOY`。 原始字段说明：长期待摊费用摊销同比增长率（%） |
| DEFER_INCOME_AMORTIZE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_INCOME_AMORTIZE_YOY`。 原始字段说明：待摊费用减少（减：增加）同比增长率（%） |
| PREPAID_EXPENSE_REDUCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREPAID_EXPENSE_REDUCE_YOY`。 原始字段说明：预提费用增加（减：减少）同比增长率（%） |
| ACCRUED_EXPENSE_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ACCRUED_EXPENSE_ADD_YOY`。 原始字段说明：预提费用变动同比增长率（%） |
| DISPOSAL_LONGASSET_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DISPOSAL_LONGASSET_LOSS_YOY`。 原始字段说明：处置固定资产等的损失同比增长率（%） |
| FA_SCRAP_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FA_SCRAP_LOSS_YOY`。 原始字段说明：固定资产报废损失同比增长率（%） |
| FAIRVALUE_CHANGE_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FAIRVALUE_CHANGE_LOSS_YOY`。 原始字段说明：公允价值变动损失同比增长率（%） |
| FINANCE_EXPENSE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINANCE_EXPENSE_YOY`。 原始字段说明：财务费用同比增长率（%） |
| INVEST_LOSS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVEST_LOSS_YOY`。 原始字段说明：投资损失同比增长率（%） |
| DEFER_TAX_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEFER_TAX_YOY`。 原始字段说明：递延所得税资产减少（增加以"-"号填列）同比增长率（%） |
| DT_ASSET_REDUCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DT_ASSET_REDUCE_YOY`。 原始字段说明：递延所得税资产减少同比增长率（%） |
| DT_LIAB_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DT_LIAB_ADD_YOY`。 原始字段说明：递延所得税负债增加同比增长率（%） |
| PREDICT_LIAB_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PREDICT_LIAB_ADD_YOY`。 原始字段说明：预计负债增加同比增长率（%） |
| INVENTORY_REDUCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INVENTORY_REDUCE_YOY`。 原始字段说明：存货的减少（增加以"-"号填列）同比增长率（%） |
| OPERATE_RECE_REDUCE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_RECE_REDUCE_YOY`。 原始字段说明：经营性应收项目的减少同比增长率（%） |
| OPERATE_PAYABLE_ADD_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_PAYABLE_ADD_YOY`。 原始字段说明：经营性应付项目的增加同比增长率（%） |
| OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OTHER_YOY`。 原始字段说明：其他同比增长率（%） |
| OPERATE_NETCASH_OTHERNOTE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_NETCASH_OTHERNOTE_YOY`。 原始字段说明：经营活动产生的现金流量净额（附注）同比增长率（%） |
| OPERATE_NETCASH_BALANCENOTE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPERATE_NETCASH_BALANCENOTE_YOY`。 原始字段说明：经营活动净现金流量（附注）平衡项同比增长率（%） |
| NETCASH_OPERATENOTE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NETCASH_OPERATENOTE_YOY`。 原始字段说明：经营活动产生的现金流量净额（附注）同比增长率（%） |
| DEBT_TRANSFER_CAPITAL_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DEBT_TRANSFER_CAPITAL_YOY`。 原始字段说明：债务转为资本同比增长率（%） |
| CONVERT_BOND_1YEAR_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CONVERT_BOND_1YEAR_YOY`。 原始字段说明：一年内到期的可转换公司债券同比增长率（%） |
| FINLEASE_OBTAIN_FA_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `FINLEASE_OBTAIN_FA_YOY`。 原始字段说明：融资租入固定资产同比增长率（%） |
| UNINVOLVE_INVESTFIN_OTHER_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `UNINVOLVE_INVESTFIN_OTHER_YOY`。 原始字段说明：不涉及现金收支的投资和筹资活动其他同比增长率（%） |
| END_CASH_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CASH_YOY`。 原始字段说明：现金期末余额同比增长率（%） |
| BEGIN_CASH_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BEGIN_CASH_YOY`。 原始字段说明：现金期初余额同比增长率（%） |
| END_CASH_EQUIVALENTS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `END_CASH_EQUIVALENTS_YOY`。 原始字段说明：现金等价物期末余额同比增长率（%） |
| BEGIN_CASH_EQUIVALENTS_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `BEGIN_CASH_EQUIVALENTS_YOY`。 原始字段说明：现金等价物期初余额同比增长率（%） |
| CCE_ADD_OTHERNOTE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_OTHERNOTE_YOY`。 原始字段说明：现金及现金等价物净增加额（附注）同比增长率（%） |
| CCE_ADD_BALANCENOTE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADD_BALANCENOTE_YOY`。 原始字段说明：现金及现金等价物净增加额（附注）平衡项同比增长率（%） |
| CCE_ADDNOTE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `CCE_ADDNOTE_YOY`。 原始字段说明：现金及现金等价物净增加额（附注）同比增长率（%） |
| OPINION_TYPE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OPINION_TYPE`。 原始字段说明：审计意见类型 |
| OSOPINION_TYPE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `OSOPINION_TYPE`。 原始字段说明：内控审计意见类型 |
| MINORITY_INTEREST | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_INTEREST`。 原始字段说明：少数股东损益 |
| MINORITY_INTEREST_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `MINORITY_INTEREST_YOY`。 原始字段说明：少数股东损益同比增长率（%） |
| USERIGHT_ASSET_AMORTIZE | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `USERIGHT_ASSET_AMORTIZE`。 原始字段说明：使用权资产摊销 |
| USERIGHT_ASSET_AMORTIZE_YOY | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `USERIGHT_ASSET_AMORTIZE_YOY`。 原始字段说明：使用权资产摊销同比增长率（%） |

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

- 已画像字段：`SALES_SERVICES`, `DEPOSIT_INTERBANK_ADD`, `LOAN_PBC_ADD`, `OFI_BF_ADD`, `RECEIVE_ORIGIC_PREMIUM`, `RECEIVE_REINSURE_NET`, `INSURED_INVEST_ADD`, `DISPOSAL_TFA_ADD`
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
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:39:23  Running with dbt=1.11.11
21:39:23  Registered adapter: clickhouse=1.10.0
21:39:24  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:39:24  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:39:24
21:39:24  Concurrency: 1 threads (target='dev')
21:39:24
Previewing inline node:
| SECUCODE  | SECURITY_CODE | SECURITY_NAME_ABBR | ORG_CODE | ORG_TYPE | REPORT_DATE | ... |
| --------- | ------------- | ------------------ | -------- | -------- | ----------- | --- |
| 000005.SZ | 000005        | ST星源               | 10004089 | 通用       |  1998-06-30 | ... |
| 000014.SZ | 000014        | 沙河股份               | 10004098 | 通用       |  1998-06-30 | ... |
| 000015.SZ | 000015        | PT中浩A              | 10004099 | 通用       |  1998-06-30 | ... |
| 000021.SZ | 000021        | 深科技                | 10004105 | 通用       |  1998-06-30 | ... |
| 000027.SZ | 000027        | 深圳能源               | 10634778 | 通用       |  1998-06-30 | ... |
| 000030.SZ | 000030        | 富奥股份               | 10634796 | 通用       |  1998-06-30 | ... |
| 000040.SZ | 000040        | *ST旭蓝              | 10634782 | 通用       |  1998-06-30 | ... |
| 000048.SZ | 000048        | 京基智农               | 10004131 | 通用       |  1998-06-30 | ... |
| 000049.SZ | 000049        | 德赛电池               | 10004132 | 通用       |  1998-06-30 | ... |
| 000055.SZ | 000055        | 方大集团               | 10004136 | 通用       |  1998-06-30 | ... |
| 000060.SZ | 000060        | 中金岭南               | 10004141 | 通用       |  1998-06-30 | ... |
| 000062.SZ | 000062        | 深圳华强               | 10004143 | 通用       |  1998-06-30 | ... |
| 000063.SZ | 000063        | 中兴通讯               | 10004144 | 通用       |  1998-06-30 | ... |
| 000069.SZ | 000069        | 华侨城A               | 10004148 | 通用       |  1998-06-30 | ... |
| 000400.SZ | 000400        | 许继电气               | 10634813 | 通用       |  1998-06-30 | ... |
| 000403.SZ | 000403        | 派林生物               | 10634809 | 通用       |  1998-06-30 | ... |
| 000404.SZ | 000404        | 长虹华意               | 10634769 | 通用       |  1998-06-30 | ... |
| 000405.SZ | 000405        | ST鑫光               | 10634771 | 通用       |  1998-06-30 | ... |
| 000406.SZ | 000406        | 石油大明               | 10634762 | 通用       |  1998-06-30 | ... |
| 000407.SZ | 000407        | 胜利股份               | 10634829 | 通用       |  1998-06-30 | ... |
| 000409.SZ | 000409        | 云鼎科技               | 10634819 | 通用       |  1998-06-30 | ... |
| 000412.SZ | 000412        | ST五环               | 10564765 | 通用       |  1998-06-30 | ... |
| 000413.SZ | 000413        | ST旭电               | 10564773 | 通用       |  1998-06-30 | ... |
| 000415.SZ | 000415        | 渤海租赁               | 10564778 | 通用       |  1998-06-30 | ... |
| 000418.SZ | 000418        | 小天鹅A               | 10564782 | 通用       |  1998-06-30 | ... |
| 000421.SZ | 000421        | 南京公用               | 10634797 | 通用       |  1998-06-30 | ... |
| 000422.SZ | 000422        | 湖北宜化               | 10634824 | 通用       |  1998-06-30 | ... |
| 000428.SZ | 000428        | 华天酒店               | 10634779 | 通用       |  1998-06-30 | ... |
| 000498.SZ | 000498        | 山东路桥               | 10004337 | 通用       |  1998-06-30 | ... |
| 000503.SZ | 000503        | 国新健康               | 10004340 | 通用       |  1998-06-30 | ... |
| 000506.SZ | 000506        | 招金黄金               | 10004343 | 通用       |  1998-06-30 | ... |
| 000511.SZ | 000511        | 烯碳退                | 10005457 | 通用       |  1998-06-30 | ... |
| 000515.SZ | 000515        | 攀渝钛业               | 10005461 | 通用       |  1998-06-30 | ... |
| 000518.SZ | 000518        | 四环生物               | 10005464 | 通用       |  1998-06-30 | ... |
| 000527.SZ | 000527        | 美的电器               | 10005473 | 通用       |  1998-06-30 | ... |
| 000531.SZ | 000531        | 穗恒运A               | 10005477 | 通用       |  1998-06-30 | ... |
| 000542.SZ | 000542        | TCL通讯              | 10005488 | 通用       |  1998-06-30 | ... |
| 000543.SZ | 000543        | 皖能电力               | 10005489 | 通用       |  1998-06-30 | ... |
| 000548.SZ | 000548        | 湖南投资               | 10005494 | 通用       |  1998-06-30 | ... |
| 000557.SZ | 000557        | 西部创业               | 10005503 | 通用       |  1998-06-30 | ... |
| 000558.SZ | 000558        | 天府文旅               | 10005504 | 通用       |  1998-06-30 | ... |
| 000559.SZ | 000559        | 万向钱潮               | 10005505 | 通用       |  1998-06-30 | ... |
| 000564.SZ | 000564        | 供销大集               | 10005510 | 通用       |  1998-06-30 | ... |
| 000567.SZ | 000567        | 海德股份               | 10005513 | 通用       |  1998-06-30 | ... |
| 000569.SZ | 000569        | 长城股份               | 10005515 | 通用       |  1998-06-30 | ... |
| 000571.SZ | 000571        | 新大洲A               | 10005517 | 通用       |  1998-06-30 | ... |
| 000578.SZ | 000578        | 盐湖集团               | 10005521 | 通用       |  1998-06-30 | ... |
| 000582.SZ | 000582        | 北部湾港               | 10005523 | 通用       |  1998-06-30 | ... |
| 000583.SZ | 000583        | S*ST托普             | 10005524 | 通用       |  1998-06-30 | ... |
| 000587.SZ | 000587        | *ST金洲              | 10005528 | 通用       |  1998-06-30 | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:39:28  Running with dbt=1.11.11
21:39:28  Registered adapter: clickhouse=1.10.0
21:39:28  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:39:29  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:39:29
21:39:29  Concurrency: 1 threads (target='dev')
21:39:29
Previewing inline node:
| row_count |
| --------- |
|    283613 |
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
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:39:32  Running with dbt=1.11.11
21:39:32  Registered adapter: clickhouse=1.10.0
21:39:33  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:39:33  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:39:33
21:39:33  Concurrency: 1 threads (target='dev')
21:39:33
Previewing inline node:
| min_report_date | max_report_date | null_report_date | placeholder_repor... | min_report_date_name | max_report_date_name | ... |
| --------------- | --------------- | ---------------- | -------------------- | -------------------- | -------------------- | --- |
|      1998-06-30 |      2026-03-31 |                0 |                    0 | 1998中报               | 2026一季报              | ... |
```

### 格式分布：SECUCODE

```sql
select
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:39:36  Running with dbt=1.11.11
21:39:37  Registered adapter: clickhouse=1.10.0
21:39:37  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:39:38  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:39:38
21:39:38  Concurrency: 1 threads (target='dev')
21:39:38
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|           283613 |             0 |            0 |             0 |    283613 |
```

### 格式分布：SECURITY_CODE

```sql
select
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:39:41  Running with dbt=1.11.11
21:39:41  Registered adapter: clickhouse=1.10.0
21:39:42  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:39:42  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:39:42
21:39:42  Concurrency: 1 threads (target='dev')
21:39:42
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |       283613 |             0 |    283613 |
```

### 格式分布：ORG_CODE

```sql
select
    countIf(match(toString(`ORG_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`ORG_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`ORG_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`ORG_CODE`) or toString(`ORG_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:39:46  Running with dbt=1.11.11
21:39:46  Registered adapter: clickhouse=1.10.0
21:39:46  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:39:47  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:39:47
21:39:47  Concurrency: 1 threads (target='dev')
21:39:47
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |    283613 |
```

### 格式分布：SECURITY_TYPE_CODE

```sql
select
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_TYPE_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_TYPE_CODE`) or toString(`SECURITY_TYPE_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:39:50  Running with dbt=1.11.11
21:39:50  Registered adapter: clickhouse=1.10.0
21:39:51  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:39:51  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:39:51
21:39:51  Concurrency: 1 threads (target='dev')
21:39:51
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |    283613 |
```

### 高频取值：SECUCODE

```sql
select
    `SECUCODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
group by `SECUCODE`
order by row_count desc
```


结果（成功）：

```text
21:39:55  Running with dbt=1.11.11
21:39:55  Registered adapter: clickhouse=1.10.0
21:39:55  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:39:56  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:39:56
21:39:56  Concurrency: 1 threads (target='dev')
21:39:56
Previewing inline node:
| value     | row_count |
| --------- | --------- |
| 000766.SZ |       104 |
| 600795.SH |       104 |
| 000678.SZ |       104 |
| 600756.SH |       103 |
| 000752.SZ |       103 |
| 600633.SH |       103 |
| 600758.SH |       103 |
| 000651.SZ |       103 |
| 600655.SH |       103 |
| 000531.SZ |       103 |
| 600096.SH |       103 |
| 600693.SH |       103 |
| 600078.SH |       103 |
| 000409.SZ |       103 |
| 600779.SH |       103 |
| 600868.SH |       103 |
| 600780.SH |       103 |
| 600834.SH |       103 |
| 600056.SH |       103 |
| 000702.SZ |       103 |
```

### 高频取值：SECURITY_CODE

```sql
select
    `SECURITY_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
group by `SECURITY_CODE`
order by row_count desc
```


结果（成功）：

```text
21:39:59  Running with dbt=1.11.11
21:39:59  Registered adapter: clickhouse=1.10.0
21:40:00  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:40:00  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:00
21:40:00  Concurrency: 1 threads (target='dev')
21:40:00
Previewing inline node:
| value  | row_count |
| ------ | --------- |
| 000678 |       104 |
| 000766 |       104 |
| 600795 |       104 |
| 600717 |       103 |
| 600708 |       103 |
| 600855 |       103 |
| 600166 |       103 |
| 000700 |       103 |
| 600770 |       103 |
| 600784 |       103 |
| 000415 |       103 |
| 600138 |       103 |
| 600892 |       103 |
| 600722 |       103 |
| 000710 |       103 |
| 000608 |       103 |
| 000848 |       103 |
| 600638 |       103 |
| 600733 |       103 |
| 600601 |       103 |
```

### 高频取值：SECURITY_NAME_ABBR

```sql
select
    `SECURITY_NAME_ABBR` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
group by `SECURITY_NAME_ABBR`
order by row_count desc
```


结果（成功）：

```text
21:40:04  Running with dbt=1.11.11
21:40:04  Registered adapter: clickhouse=1.10.0
21:40:04  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:40:05  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:05
21:40:05  Concurrency: 1 threads (target='dev')
21:40:05
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 东方明珠  |       159 |
| 百联股份  |       145 |
| 通化金马  |       104 |
| 襄阳轴承  |       104 |
| 国电电力  |       104 |
| 厦门象屿  |       103 |
| 派林生物  |       103 |
| 中色股份  |       103 |
| 招金黄金  |       103 |
| 承德露露  |       103 |
| 恒申新材  |       103 |
| 江苏索普  |       103 |
| 燕京啤酒  |       103 |
| 贝瑞基因  |       103 |
| 北汽蓝谷  |       103 |
| 深科技   |       103 |
| 超声电子  |       103 |
| 北京人力  |       103 |
| 风华高科  |       103 |
| 云天化   |       103 |
```

### 高频取值：ORG_CODE

```sql
select
    `ORG_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
group by `ORG_CODE`
order by row_count desc
```


结果（成功）：

```text
21:40:08  Running with dbt=1.11.11
21:40:08  Registered adapter: clickhouse=1.10.0
21:40:09  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:40:09  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:09
21:40:09  Concurrency: 1 threads (target='dev')
21:40:09
Previewing inline node:
| value    | row_count |
| -------- | --------- |
| 10004127 |       176 |
| 10004106 |       174 |
| 10004293 |       132 |
| 10116535 |       128 |
| 10634823 |       104 |
| 10005673 |       104 |
| 10005602 |       104 |
| 10005660 |       103 |
| 10004203 |       103 |
| 10005696 |       103 |
| 10002320 |       103 |
| 10003943 |       103 |
| 10005576 |       103 |
| 10004054 |       103 |
| 10002325 |       103 |
| 10005769 |       103 |
| 10005573 |       103 |
| 10002287 |       103 |
| 10004328 |       103 |
| 10002302 |       103 |
```

### 高频取值：ORG_TYPE

```sql
select
    `ORG_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
group by `ORG_TYPE`
order by row_count desc
```


结果（成功）：

```text
21:40:13  Running with dbt=1.11.11
21:40:13  Registered adapter: clickhouse=1.10.0
21:40:13  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:40:14  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:14
21:40:14  Concurrency: 1 threads (target='dev')
21:40:14
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 通用    |    283613 |
```

### 高频取值：REPORT_TYPE

```sql
select
    `REPORT_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
group by `REPORT_TYPE`
order by row_count desc
```


结果（成功）：

```text
21:40:17  Running with dbt=1.11.11
21:40:17  Registered adapter: clickhouse=1.10.0
21:40:18  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:40:18  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:18
21:40:18  Concurrency: 1 threads (target='dev')
21:40:18
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 年报    |     73138 |
| 一季报   |     71797 |
| 中报    |     70167 |
| 三季报   |     68511 |
```

### 数值范围：SALES_SERVICES

```sql
select
    min(`SALES_SERVICES`) as min_value,
    max(`SALES_SERVICES`) as max_value,
    countIf(`SALES_SERVICES` = 0) as zero_count,
    countIf(`SALES_SERVICES` < 0) as negative_count,
    countIf(isNull(`SALES_SERVICES`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:40:22  Running with dbt=1.11.11
21:40:22  Registered adapter: clickhouse=1.10.0
21:40:22  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:40:23  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:23
21:40:23  Concurrency: 1 threads (target='dev')
21:40:23
Previewing inline node:
|   min_value |         max_value | zero_count | negative_count | null_count | row_count |
| ----------- | ----------------- | ---------- | -------------- | ---------- | --------- |
| -29,150,641 | 3,577,814,000,000 |         18 |              9 |        600 |    283613 |
```

### 数值范围：DEPOSIT_INTERBANK_ADD

```sql
select
    min(`DEPOSIT_INTERBANK_ADD`) as min_value,
    max(`DEPOSIT_INTERBANK_ADD`) as max_value,
    countIf(`DEPOSIT_INTERBANK_ADD` = 0) as zero_count,
    countIf(`DEPOSIT_INTERBANK_ADD` < 0) as negative_count,
    countIf(isNull(`DEPOSIT_INTERBANK_ADD`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:40:26  Running with dbt=1.11.11
21:40:26  Registered adapter: clickhouse=1.10.0
21:40:27  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:40:27  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:27
21:40:27  Concurrency: 1 threads (target='dev')
21:40:27
Previewing inline node:
|          min_value |         max_value | zero_count | negative_count | null_count | row_count |
| ------------------ | ----------------- | ---------- | -------------- | ---------- | --------- |
| -75,888,794,445.99 | 78,286,026,036.56 |       3668 |            628 |     278191 |    283613 |
```

### 数值范围：LOAN_PBC_ADD

```sql
select
    min(`LOAN_PBC_ADD`) as min_value,
    max(`LOAN_PBC_ADD`) as max_value,
    countIf(`LOAN_PBC_ADD` = 0) as zero_count,
    countIf(`LOAN_PBC_ADD` < 0) as negative_count,
    countIf(isNull(`LOAN_PBC_ADD`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:40:31  Running with dbt=1.11.11
21:40:31  Registered adapter: clickhouse=1.10.0
21:40:31  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:40:32  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:32
21:40:32  Concurrency: 1 threads (target='dev')
21:40:32
Previewing inline node:
|      min_value |        max_value | zero_count | negative_count | null_count | row_count |
| -------------- | ---------------- | ---------- | -------------- | ---------- | --------- |
| -5,080,000,000 | 5,227,672,331.67 |       3685 |            117 |     279617 |    283613 |
```

### 数值范围：OFI_BF_ADD

```sql
select
    min(`OFI_BF_ADD`) as min_value,
    max(`OFI_BF_ADD`) as max_value,
    countIf(`OFI_BF_ADD` = 0) as zero_count,
    countIf(`OFI_BF_ADD` < 0) as negative_count,
    countIf(isNull(`OFI_BF_ADD`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:40:35  Running with dbt=1.11.11
21:40:35  Registered adapter: clickhouse=1.10.0
21:40:36  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:40:36  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:36
21:40:36  Concurrency: 1 threads (target='dev')
21:40:36
Previewing inline node:
|       min_value |         max_value | zero_count | negative_count | null_count | row_count |
| --------------- | ----------------- | ---------- | -------------- | ---------- | --------- |
| -16,138,000,000 | 28,859,653,769.21 |       3692 |            177 |     279395 |    283613 |
```

### 数值范围：RECEIVE_ORIGIC_PREMIUM

```sql
select
    min(`RECEIVE_ORIGIC_PREMIUM`) as min_value,
    max(`RECEIVE_ORIGIC_PREMIUM`) as max_value,
    countIf(`RECEIVE_ORIGIC_PREMIUM` = 0) as zero_count,
    countIf(`RECEIVE_ORIGIC_PREMIUM` < 0) as negative_count,
    countIf(isNull(`RECEIVE_ORIGIC_PREMIUM`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:40:40  Running with dbt=1.11.11
21:40:40  Registered adapter: clickhouse=1.10.0
21:40:40  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:40:41  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:41
21:40:41  Concurrency: 1 threads (target='dev')
21:40:41
Previewing inline node:
| min_value |         max_value | zero_count | negative_count | null_count | row_count |
| --------- | ----------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 20,983,152,538.41 |       3497 |              0 |     279773 |    283613 |
```

### 数值范围：RECEIVE_REINSURE_NET

```sql
select
    min(`RECEIVE_REINSURE_NET`) as min_value,
    max(`RECEIVE_REINSURE_NET`) as max_value,
    countIf(`RECEIVE_REINSURE_NET` = 0) as zero_count,
    countIf(`RECEIVE_REINSURE_NET` < 0) as negative_count,
    countIf(isNull(`RECEIVE_REINSURE_NET`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:40:44  Running with dbt=1.11.11
21:40:44  Registered adapter: clickhouse=1.10.0
21:40:45  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:40:45  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:45
21:40:45  Concurrency: 1 threads (target='dev')
21:40:45
Previewing inline node:
|       min_value |      max_value | zero_count | negative_count | null_count | row_count |
| --------------- | -------------- | ---------- | -------------- | ---------- | --------- |
| -314,030,336.15 | 807,228,699.97 |       3504 |             31 |     280027 |    283613 |
```

### 数值范围：INSURED_INVEST_ADD

```sql
select
    min(`INSURED_INVEST_ADD`) as min_value,
    max(`INSURED_INVEST_ADD`) as max_value,
    countIf(`INSURED_INVEST_ADD` = 0) as zero_count,
    countIf(`INSURED_INVEST_ADD` < 0) as negative_count,
    countIf(isNull(`INSURED_INVEST_ADD`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:40:48  Running with dbt=1.11.11
21:40:49  Registered adapter: clickhouse=1.10.0
21:40:49  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:40:50  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:50
21:40:50  Concurrency: 1 threads (target='dev')
21:40:50
Previewing inline node:
|      min_value |       max_value | zero_count | negative_count | null_count | row_count |
| -------------- | --------------- | ---------- | -------------- | ---------- | --------- |
| -39,673,665.86 | 5,016,097,925.1 |       3590 |              2 |     279976 |    283613 |
```

### 数值范围：DISPOSAL_TFA_ADD

```sql
select
    min(`DISPOSAL_TFA_ADD`) as min_value,
    max(`DISPOSAL_TFA_ADD`) as max_value,
    countIf(`DISPOSAL_TFA_ADD` = 0) as zero_count,
    countIf(`DISPOSAL_TFA_ADD` < 0) as negative_count,
    countIf(isNull(`DISPOSAL_TFA_ADD`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__cashflow_ytd') }}
```


结果（成功）：

```text
21:40:53  Running with dbt=1.11.11
21:40:53  Registered adapter: clickhouse=1.10.0
21:40:54  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:40:54  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:40:54
21:40:54  Concurrency: 1 threads (target='dev')
21:40:54
Previewing inline node:
|         min_value |         max_value | zero_count | negative_count | null_count | row_count |
| ----------------- | ----------------- | ---------- | -------------- | ---------- | --------- |
| -5,953,787,991.21 | 24,533,331,064.82 |       1579 |             67 |     281739 |    283613 |
```
