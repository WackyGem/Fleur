# Raw 数据画像：eastmoney__freeholders

日期：2026-06-06

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__freeholders.yml`
- dbt source：`source('raw', 'eastmoney__freeholders')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：`stg_eastmoney__freeholders`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__freeholders`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__freeholders --key SECUCODE --key END_DATE --key HOLDER_RANK --date-column END_DATE --enum-column HOLDER_RANK --enum-column HOLDER_TYPE --enum-column SHARES_TYPE --enum-column HOLD_NUM_CHANGE --format-column SECUCODE --format-column SECURITY_CODE --format-column HOLDER_NEW --numeric-column HOLDER_RANK --numeric-column HOLD_NUM --numeric-column FREE_HOLDNUM_RATIO --numeric-column CHANGE_RATIO --execute --output ../docs/references/raw_profile/eastmoney__freeholders.md`
- 补充查询：`dbt show --inline` 定向检查候选键、空字符串、distinct、rank 长尾和变动比例空值语义。
- 行数：2,736,392 行
- 数据范围：`END_DATE` 覆盖 2003-12-31 至 2026-06-03，2,287 个 distinct 报告日期。
- 分区范围：按 `toYear(END_DATE)` 观察覆盖 2003 至 2026，共 24 个年份。
- 契约数据集：`eastmoney__freeholders`
- ClickHouse raw 表：`fleur_raw.eastmoney__freeholders`
- 表说明：EastMoney free-float top holders F10 rows by natural-year raw partition.

## 2. 数据分析发现

基于当前 raw 表的现状分析：

- 数据量与覆盖
  - 总记录数：2,736,392 行。
  - 覆盖主体数：5,496 个 `SECUCODE`，5,496 个 `SECURITY_CODE`。
  - 日期 / 分区范围：`END_DATE` 覆盖 2003-12-31 至 2026-06-03；年份覆盖 2003 至 2026。
- 粒度与候选键
  - contract 中的声明粒度为每证券、报告期、流通股东名次一行，但实际 raw 数据中 `SECUCODE + END_DATE + HOLDER_RANK` 不唯一。
  - `SECUCODE + END_DATE + HOLDER_RANK` 重复样本最高 123 行，例如 `002781.SZ / 2015-12-31 / rank 7`。
  - 补充检查确认 `SECUCODE + END_DATE + HOLDER_RANK + HOLDER_NEW + HOLDER_NAME + SHARES_TYPE` 无重复；staging 模型应以这组字段作为可测试自然键。
- 缺失与占位
  - `SECUCODE`、`SECURITY_CODE`、`HOLDER_NEW`、`HOLDER_NAME`、`HOLDER_TYPE`、`SHARES_TYPE`、`HOLD_NUM_CHANGE` 空字符串均为 0 行。
  - `END_DATE` NULL 为 0 行，`1970-01-01` 占位为 0 行。
  - `CHANGE_RATIO` NULL 为 1,937,925 行，其中 `HOLD_NUM_CHANGE = '不变'` 1,079,368 行、`HOLD_NUM_CHANGE = '新进'` 858,557 行；`HOLD_NUM_CHANGE` 不属于这两类但 `CHANGE_RATIO` 为空的记录为 0 行。
  - 占位值：本次已画像日期/时间字段未发现 `1970-01-01` 占位值
  - 预期缺失：`CHANGE_RATIO` 在“不变”和“新进”语义下为空，属于 source-local 预期缺失，不应在 staging 填 0。
- 格式与参照完整性
  - `SECUCODE` 2,736,392 行全部为 `000001.SZ` 这类 EastMoney 后缀格式，无空值。
  - `SECURITY_CODE` 2,736,392 行全部为 6 位纯数字，无空值。
  - `HOLDER_NEW` 无空值，其中 383,590 行为 6 位纯数字；其余可能是其他编码或直接为股东姓名，不能当作跨源稳定股东 ID。
  - 本报告只检查直接 raw input，不做跨源证券主数据或股东主数据参照裁决。
- 分布与相关性
  - `HOLDER_TYPE` 共 25 类，top 值包括个人 1,166,755 行、其它 503,653 行、证券投资基金 383,556 行、投资公司 230,382 行。
  - `SHARES_TYPE` 共 12 类，A股 2,689,113 行占绝大多数；也存在 B股、H股、A股,H股、S股、CDR、ADR 和少量“不详”。
  - `HOLD_NUM_CHANGE` 混合文本枚举和数值文本，top 值为“不变”1,079,368 行、“新进”858,557 行，后续为正负持股变动数值文本。
- 时间字段合理性
  - 日期范围：`END_DATE` 最小 2003-12-31，最大 2026-06-03。
  - 日期先后关系异常：本 raw 表只有一个日期字段，本次不做字段间日期先后检查。
  - 批次时间范围：raw contract 未暴露独立 ingestion timestamp，本次未检查。
- 数值字段合理性
  - `HOLDER_RANK` 范围 1 至 50，0 行负数或 0；其中 rank 1-10 为 2,733,405 行，rank > 10 为 2,987 行，rank > 20 为 195 行。
  - `HOLD_NUM` 范围 500 至 190,271,558,107 股，0 行负数或 0。
  - `FREE_HOLDNUM_RATIO` 无 NULL、无负数、无 0，最大值超过 100 的记录有 3 行；这是 source 数据异常或特殊股份口径，不在 staging 静默修正。
  - `CHANGE_RATIO` 范围约 -99.817 至 148,069.865，392,768 行为负数，30,578 行绝对值超过 100；该字段表示变动比例，极端值可由小基数导致，staging 透传百分比数值。
- 其他观察
  - 对 staging 设计有影响、但不应在 staging 静默修正的事实：`HOLDER_RANK` 长尾超过 10，`FREE_HOLDNUM_RATIO` 极少数超过 100，`HOLDER_NEW` 不总是稳定数字编码。

## 3. 粒度与键

- 观察到的粒度：每证券、报告期、名次、股东标识/名称、股份类别一行。
- 旧候选键：`SECUCODE`, `END_DATE`, `HOLDER_RANK`。
- 旧候选键重复检查：存在重复；最高重复组为 `002781.SZ / 2015-12-31 / 7`，123 行。
- 新候选自然键：`SECUCODE`, `END_DATE`, `HOLDER_RANK`, `HOLDER_NEW`, `HOLDER_NAME`, `SHARES_TYPE`。
- 新候选键重复检查：补充查询返回 0 个重复组。
- 粒度注意事项：不应在 staging 按“前十大”过滤为 rank 1-10；raw 中 rank 11-50 虽少但真实存在。

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| SECUCODE | LowCardinality(String) | 0 | 空字符串 0 | 5,496 个证券代码；2,736,392 行全部为后缀格式 | 标准化为 `security_code`。 |
| SECURITY_CODE | LowCardinality(String) | 0 | 空字符串 0 | 5,496 个本地代码；2,736,392 行全部为 6 位纯数字 | 已画像但不输出到当前 staging 模型。 |
| END_DATE | Date | 0 | `1970-01-01` 0 | 2,287 个报告日期；2003-12-31 至 2026-06-03 | staging 保持为 `end_date`。 |
| HOLDER_RANK | Int64 | 0 | 0 值 0，负数 0 | 1 至 50 | 不能限制为 1-10。 |
| HOLDER_NEW | String | 0 | 空字符串 0 | 231,974 个值；383,590 行为 6 位纯数字格式 | 命名为 `holder_identifier`，不作为跨源稳定 ID。 |
| HOLDER_NAME | String | 0 | 空字符串 0 | 250,807 个股东名称 | 保留原始披露名称。 |
| HOLDER_TYPE | LowCardinality(String) | 0 | 空字符串 0 | 25 类 | 保留供应商分类，不做同义归一。 |
| SHARES_TYPE | LowCardinality(String) | 0 | 空字符串 0 | 12 类 | 保留供应商股份类别。 |
| HOLD_NUM | Int64 | 0 | 0 值 0，负数 0 | 500 至 190,271,558,107 | 单位为股，命名为 `free_float_hold_shares`。 |
| FREE_HOLDNUM_RATIO | Float64 | 0 | 0 值 0，负数 0 | 最大约 151.770；超过 100 的记录 3 行 | 百分比数值，命名为 `free_float_holdnum_ratio_pct`。 |
| HOLD_NUM_CHANGE | String | 0 | 空字符串 0 | top: 不变、新进，另有数值文本 | 保留为 `hold_num_change_text`，不在 staging 解析。 |
| CHANGE_RATIO | Nullable(Float64) | 1,937,925 | 负数 392,768 | 约 -99.817 至 148,069.865 | 百分比数值，预期可空，命名为 `change_ratio_pct`。 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `HOLDER_NEW`
- 观察到的格式：`SECUCODE` 全部为 `<6位>.<SH/SZ/BJ>`；`SECURITY_CODE` 全部为 6 位纯数字；`HOLDER_NEW` 混合数字编码和文本。
- 无效样例：未发现 `SECUCODE` 或 `SECURITY_CODE` 空值/格式漂移。
- 建议 staging 处理：`SECUCODE` 使用 `normalize_cn_security_code(..., input_format='eastmoney_suffix')` 标准化为 `security_code`；`SECURITY_CODE` 已画像但不输出到当前 staging 模型；`HOLDER_NEW` 仅重命名为 `holder_identifier`。

### 日期与时间字段

- 已画像字段：`END_DATE`
- 范围：`END_DATE`: 2003-12-31 至 2026-06-03，NULL 0 行，`1970-01-01` 占位 0 行
- 无效值或占位值：本次已画像日期/时间字段未发现 `1970-01-01` 占位值
- 建议 staging 处理：命名为 `end_date`，保留 Date 类型，添加 `not_null`。

### 枚举字段

- 已画像字段：`HOLDER_RANK`, `HOLDER_TYPE`, `SHARES_TYPE`, `HOLD_NUM_CHANGE`
- 取值：`HOLDER_RANK` 1 至 50；`HOLDER_TYPE` 25 类；`SHARES_TYPE` 12 类；`HOLD_NUM_CHANGE` 混合“不变”“新进”和持股变动数值文本。
- 未知或异常取值：`SHARES_TYPE` 有 86 行“不详”和 5 行“A股,不详”；这是供应商披露语义，staging 保留。
- 建议 staging 处理：仅做重命名和 metadata 记录，不做 accepted_values 限定和复杂同义归一。

### 数值字段

- 已画像字段：`HOLDER_RANK`, `HOLD_NUM`, `FREE_HOLDNUM_RATIO`, `CHANGE_RATIO`
- 最小/最大值：`HOLDER_RANK` 1 至 50；`HOLD_NUM` 500 至 190,271,558,107；`FREE_HOLDNUM_RATIO` 最大约 151.770；`CHANGE_RATIO` 约 -99.817 至 148,069.865。
- 负数/零值/极端值：`HOLD_NUM`、`FREE_HOLDNUM_RATIO` 无负数和 0；`CHANGE_RATIO` 392,768 行负数，30,578 行绝对值超过 100。
- 单位假设：`HOLD_NUM` 单位为股；`FREE_HOLDNUM_RATIO` 和 `CHANGE_RATIO` 是百分比数值，不是小数比例。
- 建议 staging 处理：保留原数值单位，字段名加 `_pct` 并在 YAML metadata 写入 `unit: percent`、`scale: percent_value_not_fraction`。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECUCODE + END_DATE + HOLDER_RANK` 不唯一 | 中 | 最高重复组 123 行；新候选键 0 重复 | 不添加旧候选键唯一测试；按股东编码/名称/股份类别保留明细 | 如后续要回到“每名次一行”，需在 intermediate 定义业务优先级 |
| `HOLDER_RANK` 存在 11 至 50 | 低 | rank > 10 有 2,987 行 | 不过滤，不写 1-10 accepted_values | 下游如只要前十，应在业务模型显式过滤 |
| `HOLDER_NEW` 不总是稳定数字编码 | 中 | 只有 383,590 行为 6 位纯数字，其余可能为文本 | 命名为 `holder_identifier`，不作为主数据 ID | 股东身份归并延后到 intermediate/mart |
| `FREE_HOLDNUM_RATIO` 极少数超过 100 | 低 | 超过 100 的记录 3 行 | 透传并记录百分比单位 | 业务质量告警或修正需单独建规则 |
| `CHANGE_RATIO` 大量为空 | 低 | 1,937,925 行为空，全部对应“不变”或“新进” | 保留 nullable，不填 0 | 需要解析变动方向时在 intermediate 定义 |

## 7. Staging 设计决策

- 重命名：
  - `SECUCODE` -> `security_code`
  - `END_DATE` -> `end_date`
  - `HOLDER_RANK` -> `holder_rank`
  - `HOLDER_NEW` -> `holder_identifier`
  - `HOLDER_NAME` -> `holder_name`
  - `HOLDER_TYPE` -> `holder_type`
  - `SHARES_TYPE` -> `shares_type`
  - `HOLD_NUM` -> `free_float_hold_shares`
  - `FREE_HOLDNUM_RATIO` -> `free_float_holdnum_ratio_pct`
  - `HOLD_NUM_CHANGE` -> `hold_num_change_text`
  - `CHANGE_RATIO` -> `change_ratio_pct`
- 类型转换：raw ClickHouse 类型已经满足 staging 需求，staging 只保留 Date / Int64 / Float64 / Nullable(Float64) / String / LowCardinality(String)。
- 标准化：证券代码字段使用现有 `normalize_cn_security_code` macro；其他字段只做 source-local rename。
- NULL 处理：不对 `CHANGE_RATIO` 填 0；“不变”和“新进”的空变动比例保留为 NULL。
- 测试：添加新候选键 `security_code + end_date + holder_rank + holder_identifier + holder_name + shares_type` 唯一测试；对证券代码、报告截止日、股东名次、股东标识、股东名称、股份类别、持股数量和持股比例添加高价值 not_null / format tests。
- YAML 元数据：所有列记录 `config.meta.source_columns`；证券代码列记录 normalization metadata；百分比字段记录 `unit: percent` 和 `scale: percent_value_not_fraction`。

## 8. 延后到 Intermediate/Mart

- 跨源 join：证券主数据、股东主数据和机构类型标准字典关联延后。
- 需要优先级判断的去重：如果下游需要“每证券每报告期每名次一行”，必须在 intermediate 说明如何从同名次多股东中排序或聚合。
- 主数据修正：不在 staging 修正证券代码历史、退市状态或股东身份同义项。
- 粒度变化：不在 staging 聚合到“每证券每报告期”或“每股东每报告期”。
- 业务指标逻辑：不解析 `HOLD_NUM_CHANGE` 为数值变动，不重算变动比例，不裁剪前十。

## 待确认问题

- [x] 已基于实际 raw profiling 确认 staging 设计。
- [ ] 如后续业务需要“严格前十大”语义，确认是否过滤 rank > 10 或保留全部披露行。

## 关键 SQL 证据摘要

- 行数：2,736,392
- 日期 / 分区范围：`END_DATE`: 2003-12-31 至 2026-06-03，NULL 0 行，`1970-01-01` 占位 0 行
- 候选键重复：`SECUCODE + END_DATE + HOLDER_RANK` 有重复；`SECUCODE + END_DATE + HOLDER_RANK + HOLDER_NEW + HOLDER_NAME + SHARES_TYPE` 0 个重复组。
- 关键 NULL / 占位值：`END_DATE` 无 NULL / `1970-01-01`；核心字符串无空字符串；`CHANGE_RATIO` 空值均可由“不变”或“新进”解释。
- 枚举 / 文本分布：`HOLDER_TYPE` 25 类，`SHARES_TYPE` 12 类，`HOLD_NUM_CHANGE` 混合文本状态和数值文本。
- 数值范围：`HOLDER_RANK` 1 至 50；`HOLD_NUM` 500 至 190,271,558,107；`FREE_HOLDNUM_RATIO` 极少数 > 100；`CHANGE_RATIO` 可负且有极端正值。

## 9. 验收清单

- [x] 已抽样 raw source。
- [x] 已记录行数和日期/分区范围。
- [x] 已评估粒度和候选键。
- [x] 已完成关键字段画像。
- [x] 已列出 staging 转换建议。
- [x] 已列出延后处理事项。
- [x] 已提出测试或明确豁免。

## Profiling SQL 与结果

### 样例行

```sql
select *
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:11:54  Running with dbt=1.11.11
09:11:54  Registered adapter: clickhouse=1.10.0
09:11:58  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:11:59  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:11:59
09:11:59  Concurrency: 1 threads (target='dev')
09:11:59
Previewing inline node:
| SECUCODE  | SECURITY_CODE |   END_DATE | HOLDER_RANK | HOLDER_NEW | HOLDER_NAME          | ... |
| --------- | ------------- | ---------- | ----------- | ---------- | -------------------- | --- |
| 000001.SZ | 000001        | 2003-12-31 |           1 | 050002     | 博时裕富证券投资基金           | ... |
| 000001.SZ | 000001        | 2003-12-31 |           2 | 10013067   | 深圳国债服务中心             | ... |
| 000001.SZ | 000001        | 2003-12-31 |           3 | 张绍红        | 张绍红                  | ... |
| 000001.SZ | 000001        | 2003-12-31 |           4 | 10013884   | 深圳市投资管理公司            | ... |
| 000001.SZ | 000001        | 2003-12-31 |           5 | 孟常春        | 孟常春                  | ... |
| 000001.SZ | 000001        | 2003-12-31 |           6 | 10004843   | 中关村证券股份有限公司          | ... |
| 000001.SZ | 000001        | 2003-12-31 |           7 | 161604     | 融通深证100指数证券投资基金      | ... |
| 000001.SZ | 000001        | 2003-12-31 |           8 | 王秋生        | 王秋生                  | ... |
| 000001.SZ | 000001        | 2003-12-31 |           9 | 靳艳敏        | 靳艳敏                  | ... |
| 000001.SZ | 000001        | 2003-12-31 |          10 | 200001     | 长城久恒平衡型证券投资基金        | ... |
| 000002.SZ | 000002        | 2003-12-31 |           1 | 10011587   | CREDIT LYONNAIS S... | ... |
| 000002.SZ | 000002        | 2003-12-31 |           2 | 10014604   | TOYO SECURITIES A... | ... |
| 000002.SZ | 000002        | 2003-12-31 |           3 | 10006187   | HOLY TIME GROUP L... | ... |
| 000002.SZ | 000002        | 2003-12-31 |           4 | 050001     | 博时价值增长证券投资基金         | ... |
| 000002.SZ | 000002        | 2003-12-31 |           5 | 000001     | 华夏成长证券投资基金           | ... |
| 000002.SZ | 000002        | 2003-12-31 |           6 | 10011602   | 内藤证券株式会社             | ... |
| 000002.SZ | 000002        | 2003-12-31 |           7 | 184699     | 同盛证券投资基金             | ... |
| 000002.SZ | 000002        | 2003-12-31 |           8 | 10059224   | BONY A/C MIF-MATT... | ... |
| 000002.SZ | 000002        | 2003-12-31 |           9 | 500016     | 裕元证券投资基金             | ... |
| 000002.SZ | 000002        | 2003-12-31 |          10 | 050002     | 裕富证券投资基金             | ... |
| 000004.SZ | 000004        | 2003-12-31 |           1 | 10004876   | 中国银河证券有限责任公司         | ... |
| 000004.SZ | 000004        | 2003-12-31 |           2 | 10006256   | 郑州迅通计算机系统工程有限公司      | ... |
| 000004.SZ | 000004        | 2003-12-31 |           3 | 10018311   | 川化集团有限责任公司           | ... |
| 000004.SZ | 000004        | 2003-12-31 |           4 | 10042931   | 上海宝业集团有限公司           | ... |
| 000004.SZ | 000004        | 2003-12-31 |           5 | 10019767   | 四川川化集团实业开发有限公司       | ... |
| 000004.SZ | 000004        | 2003-12-31 |           6 | 徐秀英        | 徐秀英                  | ... |
| 000004.SZ | 000004        | 2003-12-31 |           7 | 崔光辉        | 崔光辉                  | ... |
| 000004.SZ | 000004        | 2003-12-31 |           8 | 林金铭        | 林金铭                  | ... |
| 000004.SZ | 000004        | 2003-12-31 |           9 | 陈一雄        | 陈一雄                  | ... |
| 000004.SZ | 000004        | 2003-12-31 |          10 | 戎留青        | 戎留青                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           1 | 唐卫星        | 唐卫星                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           2 | 吕志强        | 吕志强                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           3 | 张映浩        | 张映浩                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           4 | 曾五女        | 曾五女                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           5 | 黄盾斌        | 黄盾斌                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           6 | 张雄棠        | 张雄棠                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           7 | 王益龙        | 王益龙                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           8 | 李美英        | 李美英                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |           9 | 鲁宁馨        | 鲁宁馨                  | ... |
| 000005.SZ | 000005        | 2003-12-31 |          10 | 10013760   | 河南兆鑫实业有限公司           | ... |
| 000006.SZ | 000006        | 2003-12-31 |           1 | 许凤兴        | 许凤兴                  | ... |
| 000006.SZ | 000006        | 2003-12-31 |           2 | 10016395   | 北京亚商投资咨询有限公司         | ... |
| 000006.SZ | 000006        | 2003-12-31 |           3 | 10047489   | 安徽省粮食集团公司            | ... |
| 000006.SZ | 000006        | 2003-12-31 |           4 | 161604     | 中国工商银行-融通深证100指数证... | ... |
| 000006.SZ | 000006        | 2003-12-31 |           5 | 王利得        | 王利得                  | ... |
| 000006.SZ | 000006        | 2003-12-31 |           6 | 刘波         | 刘波                   | ... |
| 000006.SZ | 000006        | 2003-12-31 |           7 | 罗舜乐        | 罗舜乐                  | ... |
| 000006.SZ | 000006        | 2003-12-31 |           8 | 姜韶辉        | 姜韶辉                  | ... |
| 000006.SZ | 000006        | 2003-12-31 |           9 | 庞翠轻        | 庞翠轻                  | ... |
| 000006.SZ | 000006        | 2003-12-31 |          10 | 10031947   | 北京大学教育基金会            | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:12:04  Running with dbt=1.11.11
09:12:04  Registered adapter: clickhouse=1.10.0
09:12:05  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:05  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:05
09:12:05  Concurrency: 1 threads (target='dev')
09:12:05
Previewing inline node:
| row_count |
| --------- |
|   2736392 |
```

### 日期范围

```sql
select
    min(`END_DATE`) as min_end_date,
    max(`END_DATE`) as max_end_date,
    countIf(isNull(`END_DATE`)) as null_end_date,
    countIf(`END_DATE` = toDate('1970-01-01')) as placeholder_end_date
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:12:09  Running with dbt=1.11.11
09:12:09  Registered adapter: clickhouse=1.10.0
09:12:10  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:11  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:11
09:12:11  Concurrency: 1 threads (target='dev')
09:12:11
Previewing inline node:
| min_end_date | max_end_date | null_end_date | placeholder_end_date |
| ------------ | ------------ | ------------- | -------------------- |
|   2003-12-31 |   2026-06-03 |             0 |                    0 |
```

### 候选键重复检查

```sql
select
    `SECUCODE`, `END_DATE`, `HOLDER_RANK`,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
group by `SECUCODE`, `END_DATE`, `HOLDER_RANK`
having row_count > 1
order by row_count desc
```


结果（成功）：

```text
09:12:15  Running with dbt=1.11.11
09:12:15  Registered adapter: clickhouse=1.10.0
09:12:16  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:16  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:16
09:12:16  Concurrency: 1 threads (target='dev')
09:12:16
Previewing inline node:
| SECUCODE  |   END_DATE | HOLDER_RANK | row_count |
| --------- | ---------- | ----------- | --------- |
| 002781.SZ | 2015-12-31 |           7 |       123 |
| 300385.SZ | 2014-06-30 |           7 |       106 |
| 603589.SH | 2015-06-30 |           2 |        88 |
| 603116.SH | 2015-06-30 |           1 |        56 |
| 603729.SH | 2015-03-31 |           4 |        19 |
| 002736.SZ | 2014-12-31 |           4 |        18 |
| 002393.SZ | 2011-06-30 |           6 |        17 |
| 600730.SH | 2015-03-31 |          10 |        16 |
| 600955.SH | 2026-03-31 |           9 |        15 |
| 002441.SZ | 2010-09-30 |           6 |        14 |
| 002492.SZ | 2010-12-31 |           3 |        12 |
| 600017.SH | 2007-03-31 |           7 |        12 |
| 601998.SH | 2008-06-30 |          10 |        12 |
| 603363.SH | 2017-09-30 |          12 |        12 |
| 601001.SH | 2006-09-30 |           9 |        11 |
| 600039.SH | 2016-12-31 |           3 |        10 |
| 002269.SZ | 2018-03-31 |           4 |        10 |
| 601186.SH | 2023-03-31 |           5 |        10 |
| 600317.SH | 2016-09-30 |           6 |        10 |
| 600528.SH | 2015-09-30 |           6 |        10 |
| 600039.SH | 2019-03-31 |           8 |        10 |
| 600086.SH | 2016-06-30 |           6 |        10 |
| 600019.SH | 2015-09-30 |           8 |        10 |
| 600812.SH | 2018-06-30 |           5 |        10 |
| 600805.SH | 2019-09-30 |           4 |        10 |
| 601179.SH | 2016-09-30 |           9 |        10 |
| 600864.SH | 2015-12-31 |           5 |        10 |
| 002724.SZ | 2015-09-30 |           2 |        10 |
| 601618.SH | 2023-03-31 |           7 |        10 |
| 600587.SH | 2017-09-30 |           5 |        10 |
| 002165.SZ | 2015-09-30 |           9 |        10 |
| 600839.SH | 2018-09-30 |           7 |        10 |
| 600823.SH | 2020-06-30 |           9 |        10 |
| 600587.SH | 2021-09-30 |           7 |        10 |
| 600795.SH | 2019-09-30 |           6 |        10 |
| 600089.SH | 2020-06-30 |           5 |        10 |
| 600029.SH | 2016-09-30 |          10 |        10 |
| 600010.SH | 2016-03-31 |           4 |        10 |
| 601186.SH | 2024-09-30 |           7 |        10 |
| 600256.SH | 2021-03-31 |           9 |        10 |
| 000528.SZ | 2017-03-31 |           7 |        10 |
| 600030.SH | 2017-06-30 |           9 |        10 |
| 600256.SH | 2017-09-30 |           7 |        10 |
| 600256.SH | 2020-09-30 |          10 |        10 |
| 601006.SH | 2019-03-31 |           6 |        10 |
| 300270.SZ | 2015-09-30 |           3 |        10 |
| 601186.SH | 2022-03-31 |           5 |        10 |
| 601618.SH | 2021-09-30 |           7 |        10 |
| 600252.SH | 2017-06-30 |           7 |        10 |
| 601186.SH | 2020-03-31 |           6 |        10 |
```

### 格式分布：SECUCODE

```sql
select
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:12:23  Running with dbt=1.11.11
09:12:24  Registered adapter: clickhouse=1.10.0
09:12:24  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:25  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:25
09:12:25  Concurrency: 1 threads (target='dev')
09:12:25
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|          2736392 |             0 |            0 |             0 |   2736392 |
```

### 格式分布：SECURITY_CODE

```sql
select
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:12:29  Running with dbt=1.11.11
09:12:30  Registered adapter: clickhouse=1.10.0
09:12:30  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:31  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:31
09:12:31  Concurrency: 1 threads (target='dev')
09:12:31
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |      2736392 |             0 |   2736392 |
```

### 格式分布：HOLDER_NEW

```sql
select
    countIf(match(toString(`HOLDER_NEW`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`HOLDER_NEW`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`HOLDER_NEW`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`HOLDER_NEW`) or toString(`HOLDER_NEW`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:12:36  Running with dbt=1.11.11
09:12:36  Registered adapter: clickhouse=1.10.0
09:12:37  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:38  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:38
09:12:38  Concurrency: 1 threads (target='dev')
09:12:38
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |       383590 |             0 |   2736392 |
```

### 高频取值：HOLDER_RANK

```sql
select
    `HOLDER_RANK` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
group by `HOLDER_RANK`
order by row_count desc
```


结果（成功）：

```text
09:12:42  Running with dbt=1.11.11
09:12:42  Registered adapter: clickhouse=1.10.0
09:12:43  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:43  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:43
09:12:43  Concurrency: 1 threads (target='dev')
09:12:43
Previewing inline node:
| value | row_count |
| ----- | --------- |
|     1 |    275503 |
|     6 |    274963 |
|     5 |    274436 |
|     3 |    273735 |
|     7 |    273732 |
|     4 |    273702 |
|     2 |    273632 |
|     8 |    272512 |
|     9 |    272258 |
|    10 |    268932 |
|    11 |       874 |
|    12 |       282 |
|    19 |       268 |
|    18 |       265 |
|    17 |       251 |
|    16 |       221 |
|    15 |       190 |
|    13 |       186 |
|    14 |       169 |
|    20 |        86 |
```

### 高频取值：HOLDER_TYPE

```sql
select
    `HOLDER_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
group by `HOLDER_TYPE`
order by row_count desc
```


结果（成功）：

```text
09:12:47  Running with dbt=1.11.11
09:12:47  Registered adapter: clickhouse=1.10.0
09:12:48  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:49  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:49
09:12:49  Concurrency: 1 threads (target='dev')
09:12:49
Previewing inline node:
| value    | row_count |
| -------- | --------- |
| 个人       |   1166755 |
| 其它       |    503653 |
| 证券投资基金   |    383556 |
| 投资公司     |    230382 |
| 私募基金     |     67701 |
| 基金资产管理计划 |     52825 |
| QFII     |     49513 |
| 全国社保基金   |     49275 |
| 信托计划     |     48304 |
| 证券公司     |     48206 |
| 保险产品     |     44875 |
| 证券账户     |     25436 |
| 集合理财计划   |     14118 |
| 员工持股计划   |      9754 |
| 其他理财产品   |      8078 |
| 信托投资公司   |      6094 |
| 金融       |      5714 |
| 基本养老基金   |      5338 |
| 保险公司     |      4863 |
| 企业年金     |      3229 |
```

### 高频取值：SHARES_TYPE

```sql
select
    `SHARES_TYPE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
group by `SHARES_TYPE`
order by row_count desc
```


结果（成功）：

```text
09:12:53  Running with dbt=1.11.11
09:12:53  Registered adapter: clickhouse=1.10.0
09:12:54  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:12:55  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:12:55
09:12:55  Concurrency: 1 threads (target='dev')
09:12:55
Previewing inline node:
| value | row_count |
| ----- | --------- |
| A股    |   2689113 |
| B股    |     32046 |
| H股    |     11934 |
| A股,H股 |      1681 |
| S股    |       638 |
| A股,B股 |       494 |
| CDR   |       241 |
| 不详    |        86 |
| ADR   |        68 |
| A股,S股 |        44 |
| B股,H股 |        42 |
| A股,不详 |         5 |
```

### 高频取值：HOLD_NUM_CHANGE

```sql
select
    `HOLD_NUM_CHANGE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
group by `HOLD_NUM_CHANGE`
order by row_count desc
```


结果（成功）：

```text
09:12:58  Running with dbt=1.11.11
09:12:59  Registered adapter: clickhouse=1.10.0
09:12:59  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:13:00  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:13:00
09:13:00  Concurrency: 1 threads (target='dev')
09:13:00
Previewing inline node:
| value    | row_count |
| -------- | --------- |
| 不变       |   1079368 |
| 新进       |    858557 |
| -100000  |      3013 |
| 100000   |      2488 |
| -1000000 |      2461 |
| -200000  |      2409 |
| -10000   |      2267 |
| -50000   |      2182 |
| -500000  |      2134 |
| -20000   |      2089 |
| 10000    |      1958 |
| -300000  |      1853 |
| 50000    |      1817 |
| 200000   |      1805 |
| 20000    |      1787 |
| -30000   |      1525 |
| -2000000 |      1491 |
| 500000   |      1358 |
| 300000   |      1355 |
| 30000    |      1291 |
```

### 数值范围：HOLDER_RANK

```sql
select
    min(`HOLDER_RANK`) as min_value,
    max(`HOLDER_RANK`) as max_value,
    countIf(`HOLDER_RANK` = 0) as zero_count,
    countIf(`HOLDER_RANK` < 0) as negative_count,
    countIf(isNull(`HOLDER_RANK`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:13:04  Running with dbt=1.11.11
09:13:04  Registered adapter: clickhouse=1.10.0
09:13:05  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:13:06  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:13:06
09:13:06  Concurrency: 1 threads (target='dev')
09:13:06
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         1 |        50 |          0 |              0 |          0 |   2736392 |
```

### 数值范围：HOLD_NUM

```sql
select
    min(`HOLD_NUM`) as min_value,
    max(`HOLD_NUM`) as max_value,
    countIf(`HOLD_NUM` = 0) as zero_count,
    countIf(`HOLD_NUM` < 0) as negative_count,
    countIf(isNull(`HOLD_NUM`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:13:10  Running with dbt=1.11.11
09:13:10  Registered adapter: clickhouse=1.10.0
09:13:10  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:13:11  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:13:11
09:13:11  Concurrency: 1 threads (target='dev')
09:13:11
Previewing inline node:
| min_value |    max_value | zero_count | negative_count | null_count | row_count |
| --------- | ------------ | ---------- | -------------- | ---------- | --------- |
|       500 | 190271558107 |          0 |              0 |          0 |   2736392 |
```

### 数值范围：FREE_HOLDNUM_RATIO

```sql
select
    min(`FREE_HOLDNUM_RATIO`) as min_value,
    max(`FREE_HOLDNUM_RATIO`) as max_value,
    countIf(`FREE_HOLDNUM_RATIO` = 0) as zero_count,
    countIf(`FREE_HOLDNUM_RATIO` < 0) as negative_count,
    countIf(isNull(`FREE_HOLDNUM_RATIO`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:13:16  Running with dbt=1.11.11
09:13:16  Registered adapter: clickhouse=1.10.0
09:13:17  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:13:18  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:13:18
09:13:18  Concurrency: 1 threads (target='dev')
09:13:18
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|    0.000… |  151.770… |          0 |              0 |          0 |   2736392 |
```

### 数值范围：CHANGE_RATIO

```sql
select
    min(`CHANGE_RATIO`) as min_value,
    max(`CHANGE_RATIO`) as max_value,
    countIf(`CHANGE_RATIO` = 0) as zero_count,
    countIf(`CHANGE_RATIO` < 0) as negative_count,
    countIf(isNull(`CHANGE_RATIO`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__freeholders') }}
```


结果（成功）：

```text
09:13:21  Running with dbt=1.11.11
09:13:21  Registered adapter: clickhouse=1.10.0
09:13:22  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 1 unused configuration paths:
- models.elt.marts
09:13:23  Found 3 operations, 18 models, 142 data tests, 1 sql operation, 16 sources, 530 macros
09:13:23
09:13:23  Concurrency: 1 threads (target='dev')
09:13:23
Previewing inline node:
| min_value |    max_value | zero_count | negative_count | null_count | row_count |
| --------- | ------------ | ---------- | -------------- | ---------- | --------- |
|  -99.817… | 148,069.865… |          0 |         392768 |    1937925 |   2736392 |
```
