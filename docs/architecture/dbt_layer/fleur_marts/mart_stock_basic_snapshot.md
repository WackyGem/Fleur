# mart_stock_basic_snapshot 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_basic_snapshot')`
- 目标 SQL：`pipeline/elt/models/marts/mart_stock_basic_snapshot.sql`
- 目标 YAML：`pipeline/elt/models/marts/mart_stock_basic_snapshot.yml`
- 消费方：Rearview preview rows、preview pool page 和 preview security analysis 的证券显示信息

## 1. 模型定位

A 股股票基础显示信息 mart 当前快照。模型只暴露页面展示需要的证券代码、证券名称、交易所代码和 A 股交易板块，作为 Rearview 从 marts 层补齐证券显示信息的稳定消费接口。

非目标：

- 不输出行业、概念、地域或同类分组字段；`security_board` 仅表示交易板块，不表示行业板块。
- 不承载上市状态、交易状态、股本、财务、行情或指标事实。
- 不直接参与选股、评分、回测或组合计算语义。

## 2. 数据粒度与依赖

- 粒度：每证券一行，当前基础信息快照。
- 候选键：`security_code`。
- 唯一上游：`int_stock_basic_snapshot`。
- Join 策略：无 join。

## 3. 字段分组

| 字段组 | 来源 | 字段 |
|---|---|---|
| 主键 | `int_stock_basic_snapshot` | `security_code` |
| 显示信息 | `int_stock_basic_snapshot` | `security_name`, `exchange_code`, `security_board` |

## 4. NULL 语义

- `security_code` 必须非空。
- `security_name` 允许为空；消费方应回退展示 `security_code`。
- `exchange_code` 允许为空；消费方不应因此阻断 preview 主结果。
- `security_board` 允许为空；消费方应回退为缺失状态，不把 `exchange_code` 伪装成板块。
- `security_board` 当前允许值为 `sse_main_board`、`szse_main_board`、`chinext`、`star_market`。
- mart 层不填充、不推断、不从其他数据源补全展示字段。

## 5. 验证

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_basic_snapshot
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_basic_snapshot --limit 20
```

补充质量检查：

- `security_code` 唯一且非空。
- `security_code` 符合 A 股标准代码格式。
- 字段集合只包含 `security_code`、`security_name`、`exchange_code` 和 `security_board`。
