# RFC 0007: dbt Raw 层设计与 Dagster-dbt 集成架构

状态：草案（2026-05-30）

## 摘要

本文档定义了 pipeline 项目中 dbt raw 层的设计方案，以及 Dagster 与 dbt 的集成架构。核心决策：

1. **前置改造（必须先完成）：S3 Parquet Schema 类型优化**：将所有列从 `pa.string()` 改为真实数据类型，提升压缩率和查询性能。这是后续所有设计的基础。
2. **Raw 层采用 Table 物化**：ClickHouse raw 表物化 S3 parquet 数据，后续所有转换基于本地高速列存储。
3. **Staging 层采用 View 物化**：业务逻辑转换、列重命名等轻量操作不额外占用存储。
4. **Dagster 负责 raw 表刷新**：Dagster 写完 S3 后，执行 ClickHouse SQL（`REPLACE PARTITION`）原子更新 raw 表。
5. **dbt 无状态运行**：dbt 通过 `source()` 引用 raw 表作为 Dagster asset，Dagster 自动编排依赖。

## 前置改造：S3 Parquet Schema 类型优化

### 问题

当前所有数据源的 S3 parquet 文件全部使用 `pa.string()` 存储所有列，无论原始数据类型。这种设计虽然简化了写入逻辑，但带来以下问题：

1. **压缩效率低下**：Parquet 的 zstd 压缩对数值列的压缩率远高于字符串列。例如，成交量 `12345678` 存为 string 需要 8-10 字节，存为 Int64 只需要 4-5 字节。

2. **ClickHouse 性能退化**：ClickHouse 是列式数据库，类型感知是其核心优势。全 String 列无法利用 SIMD 指令加速聚合、排序、过滤。

3. **数据质量缺失**：无法区分 `"null"` 字符串和真正的 NULL，无法验证数值列是否真的是数值，数据问题延迟到查询时才暴露。

4. **下游转换负担**：dbt staging 层需要处理大量 `CAST` 操作，而不是专注于业务逻辑转换。

### 改造方案

在 Dagster 写入 S3 时，根据数据源的真实 schema 做类型优化：

```python
# 当前：全部 string
schema = pa.schema([
    ("volume", pa.string()),
    ("turn", pa.string()),
    ("pctChg", pa.string()),
])

# 改造后：按真实类型
schema = pa.schema([
    ("date", pa.date32()),
    ("code", pa.string()),      # 证券代码确实是字符串
    ("volume", pa.int64()),
    ("turn", pa.float64()),
    ("pctChg", pa.float64()),
])
```

### 各数据源 Schema 定义

#### BaoStock 日 K 线

| 列名 | 当前类型 | 建议类型 | 说明 |
|------|---------|---------|------|
| date | string | date32 | 日期 |
| code | string | string | 证券代码 |
| open | string | float64 | 开盘价 |
| high | string | float64 | 最高价 |
| low | string | float64 | 最低价 |
| close | string | float64 | 收盘价 |
| preclose | string | float64 | 昨收价 |
| volume | string | int64 | 成交量 |
| amount | string | float64 | 成交额 |
| adjustflag | string | string | 复权标志 |
| turn | string | float64 | 换手率 |
| tradestatus | string | string | 交易状态 |
| pctChg | string | float64 | 涨跌幅 |
| isST | string | string | ST 标志 |

#### BaoStock 股票基础信息

| 列名 | 当前类型 | 建议类型 | 说明 |
|------|---------|---------|------|
| code | string | string | 证券代码 |
| code_name | string | string | 证券名称 |
| ipoDate | string | date32 | 上市日期 |
| outDate | string | date32 | 退市日期 |
| type | string | string | 证券类型 |
| status | string | string | 证券状态 |

#### 东方财富财务报表（示例：利润表）

| 列名模式 | 当前类型 | 建议类型 | 说明 |
|---------|---------|---------|------|
| REPORT_DATE_NAME | string | string | 报告期名称 |
| TOTAL_OPERATE_INCOME | string | float64 | 营业总收入 |
| OPERATE_INCOME | string | float64 | 营业收入 |
| OPERATE_COST | string | float64 | 营业成本 |
| ... | string | float64 | 数值字段 |

#### 同花顺涨停池

| 列名 | 当前类型 | 建议类型 | 说明 |
|------|---------|---------|------|
| date | string | date32 | 日期 |
| code | string | string | 证券代码 |
| open_num | string | int64 | 开板次数 |
| change_rate | string | float64 | 涨跌幅 |
| turnover_rate | string | float64 | 换手率 |
| ... | string | 按业务类型 | 其他字段 |

### 改造范围

需要改造的文件：

| 文件 | 改造内容 |
|------|---------|
| `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py` | 定义 BaoStock 真实 schema |
| `pipeline/scheduler/src/scheduler/defs/http/schemas.py` | 定义 HTTP 数据源真实 schema |
| `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/fields.py` | 定义东方财富字段类型 |
| `pipeline/scheduler/src/scheduler/defs/sources/ths/limit_up_pool.py` | 定义同花顺字段类型 |
| 各数据源的 `pa.Table` 构造逻辑 | 使用新 schema 构造 table |

### 预期收益

1. **压缩率提升 30-50%**：数值列使用原生类型后，zstd 压缩率显著提升
2. **ClickHouse 查询性能提升**：原生类型支持 SIMD 加速，聚合查询更快
3. **数据质量提升**：写入时即可验证数据类型，提前发现数据问题
4. **下游转换简化**：dbt staging 层专注于业务逻辑，而非类型转换

### 迁移策略

1. **新数据源直接使用新 schema**：新增数据源从一开始就使用正确类型
2. **存量数据源分批迁移**：按数据源优先级逐步迁移，避免一次性大规模变更
3. **向后兼容**：迁移期间保持 S3 路径不变，只改变 parquet 内部 schema
4. **ClickHouse 表结构同步更新**：raw 表建表语句与新 schema 对齐

### 替代方案

| 方案 | 优点 | 缺点 | 选择理由 |
|------|------|------|---------|
| 保持全 String | 实现简单，无需维护 schema | 压缩率低，ClickHouse 性能差 | ❌ 不选择 |
| 按数据源定义 schema | 类型正确，压缩率高 | 需要为每个数据源维护 schema | ✅ 选择 |
| 使用 Avro 替代 Parquet | schema 演进更灵活 | ClickHouse 支持有限，生态不成熟 | ❌ 不选择 |

### 向后兼容性

| 维度 | 兼容性 | 说明 |
|------|--------|------|
| S3 路径 | ✅ 兼容 | 路径结构不变，只改变 parquet 内部 schema |
| Dagster asset key | ✅ 兼容 | asset key 不变 |
| 数据内容 | ✅ 兼容 | 数据值不变，只改变存储类型 |
| ClickHouse 表结构 | ❌ 不兼容 | 列类型从 String 改为原生类型，需要重建表 |
| 下游查询 | ❌ 不兼容 | 依赖 String 类型的查询需要更新 |
| dbt 模型 | ❌ 不兼容 | 需要更新模型以适应新 schema |

## 背景

### 当前数据流

```
Dagster (Python)
  - 从 API/TCP 拉取数据
  - 写入 S3 parquet (RustFS)
  - 压缩: zstd
  - 分区: 无分区(snapshot) / year / trade_date
  - 所有列为 pa.string() ← 问题：无法利用类型优化
```

### 目标数据流

```
Dagster (Python)
  - 从 API/TCP 拉取数据
  - 写入 S3 parquet (按真实类型)
  - 刷新 ClickHouse raw 表 (REPLACE PARTITION)
       │
       ▼
ClickHouse raw 层 (MergeTree)
  - 物化 S3 parquet 数据
  - 列类型与 S3 parquet 一致 (Date, Float64, Int64, String)
  - 分区键与 S3 分区对齐
       │
       ▼
dbt staging 层 (View)
  - 业务逻辑转换（日期格式标准化、字段映射等）
  - 列重命名
  - 基础清洗
       │
       ▼
dbt marts 层 (Table / Incremental)
  - 业务聚合
  - 面向查询优化
```

## 设计决策

### 决策 1：Raw 层物化策略

**选择：Table（MergeTree）**

对比三种方案：

| 维度 | View (方案 A) | Table (方案 B) | Table (方案 C) |
|------|--------------|----------------|----------------|
| S3 读取 | 每次查询都读 S3 | 导入时读一次 | 导入时读一次 |
| 存储 | 零（S3 是唯一 source） | S3 + ClickHouse raw | S3 + ClickHouse raw + staging |
| 查询性能 | 取决于 S3 读取 | 本地列存储 | 本地列存储 |
| 上游更新影响 | 每次查询都受影响 | 只影响刷新时 | 只影响刷新时 |

**选择方案 B 的理由：**

1. **S3 读取成本只发生一次**：raw table 物化后，所有下游转换都是本地列存储查询，不再碰 S3。即使 Dagster 只改了几行数据（按年重写分区），也不会触发全量 S3 扫描。

2. **存储总量与方案 A 持平甚至更低**：raw 表使用真实类型存储，压缩率比 staging（可能包含额外转换列）更高。总存储 = S3 + ClickHouse raw ≈ S3 + ClickHouse staging。

3. **查询性能一致**：所有下游都是本地查询，不会因为上游重写分区而退化。

### 决策 2：分区级刷新策略

**选择：Dagster 执行 `REPLACE PARTITION`**

ClickHouse 的 MergeTree 引擎支持 `ALTER TABLE ... REPLACE PARTITION`，可以原子性地替换整个分区：

```sql
-- 方式 1：临时表 + REPLACE PARTITION（推荐）
CREATE TABLE raw_xxx_tmp AS raw_xxx;
INSERT INTO raw_xxx_tmp SELECT * FROM s3(...);
ALTER TABLE raw_xxx REPLACE PARTITION '2025' FROM raw_xxx_tmp;
DROP TABLE raw_xxx_tmp;

-- 方式 2：DROP + INSERT（更简单，但非原子）
ALTER TABLE raw_xxx DROP PARTITION '2025';
INSERT INTO raw_xxx SELECT * FROM s3(...);
```

**选择方式 1 的理由：**

- 原子性：`REPLACE PARTITION` 是原子操作，不会出现中间状态
- 安全性：如果 INSERT 失败，原始数据不受影响
- 性能：ClickHouse 内部优化了分区替换操作

### 决策 3：dbt 触发方式

**选择：Declarative Automation + `eager()`**

Dagster 提供三种触发方式：

| 方式 | 适用场景 | 复杂度 |
|------|---------|--------|
| Declarative Automation (`eager()`) | 资产中心化管道，自动依赖感知 | 低 |
| Asset Sensor | 跨 job 依赖，需要条件触发 | 中 |
| 单 Job 串联 | 简单管道，耦合度高 | 低 |

**选择 Declarative Automation 的理由：**

1. **自动依赖感知**：dbt 模型通过 `source('dagster', 'raw_xxx')` 引用 raw 资产，Dagster 自动在 raw 物化后触发 dbt 模型执行。无需手动编写 sensor 或 schedule。

2. **声明式配置**：在 raw 资产上设置 `automation_condition=dg.AutomationCondition.eager()`，Dagster 自动处理依赖等待、并发控制、分区状态。

3. **状态分离**：Dagster 管 raw 资产的物化状态，dbt 完全无状态。每次 dbt 运行都是全量构建 staging views + marts tables。

## 模型组织

### 目录结构

```
pipeline/elt/
├── dbt_project.yml
├── models/
│   ├── raw/
│   │   ├── _raw__sources.yml          # source 定义
│   │   ├── _raw__models.yml           # raw 模型 schema 定义
│   │   ├── baostock/
│   │   │   ├── raw_baostock__query_stock_basic.sql
│   │   │   └── raw_baostock__query_history_k_data_plus_daily.sql
│   │   ├── sina/
│   │   │   └── raw_sina__trade_calendar.sql
│   │   ├── eastmoney/
│   │   │   ├── raw_eastmoney__balance.sql
│   │   │   ├── raw_eastmoney__cashflow_sq.sql
│   │   │   └── ... (8 张表)
│   │   ├── ths/
│   │   │   ├── raw_ths__limit_up_pool.sql
│   │   │   └── raw_ths__limit_up_pool_compacted.sql
│   │   └── jiuyan/
│   │       └── raw_jiuyan__industry_ocr.sql
│   ├── staging/
│   │   ├── _staging__models.yml
│   │   ├── stg_baostock__stock_basic.sql
│   │   ├── stg_baostock__k_data_daily.sql
│   │   └── ...
│   └── marts/
│       ├── _marts__models.yml
│       └── ...
└── macros/
    └── ...
```

### 命名约定

保持与 Dagster asset name 一致，加 `raw_` 前缀：

| S3 asset name | dbt model name |
|---|---|
| `baostock__query_stock_basic` | `raw_baostock__query_stock_basic` |
| `eastmoney__balance` | `raw_eastmoney__balance` |
| `ths__limit_up_pool` | `raw_ths__limit_up_pool` |

### dbt_project.yml 配置

```yaml
name: elt
version: '1.0.0'
profile: elt

models:
  elt:
    raw:
      +materialized: view
      +schema: raw
    staging:
      +materialized: view
      +schema: staging
    marts:
      +materialized: table
      +schema: marts

vars:
  s3_endpoint: "{{ env_var('RUSTFS_ENDPOINT') }}"
  s3_bucket: "{{ env_var('RUSTFS_BUCKET') }}"
  s3_access_key: "{{ env_var('RUSTFS_ACCESS_KEY') }}"
  s3_secret_key: "{{ env_var('RUSTFS_SECRET_KEY') }}"
```

## Dagster 集成架构

### Raw 资产定义

```python
import dagster as dg
from dagster_dbt import DbtCliResource, DbtProjectComponent

@dg.asset(
    automation_condition=dg.AutomationCondition.eager(),
    group_name="raw",
    metadata={
        "partition_key_name": "year",
        "storage_mode": "partitioned",
    },
)
def raw_baostock__query_history_k_data_plus_daily(context):
    """写 S3 parquet + 刷新 ClickHouse raw 表"""
    # 1. 已有逻辑：写 S3 parquet
    # 2. 新增：执行 ClickHouse SQL
    clickhouse.execute("ALTER TABLE raw_xxx DROP PARTITION '2025'")
    clickhouse.execute("INSERT INTO raw_xxx SELECT * FROM s3(...)")
```

### dbt Source 定义

```yaml
# models/raw/_raw__sources.yml
version: 2

sources:
  - name: dagster
    tables:
      - name: raw_baostock__query_stock_basic
      - name: raw_baostock__query_history_k_data_plus_daily
      - name: raw_sina__trade_calendar
      - name: raw_eastmoney__balance
      - name: raw_eastmoney__cashflow_sq
      - name: raw_eastmoney__cashflow_ytd
      - name: raw_eastmoney__dividend_allotment
      - name: raw_eastmoney__dividend_main
      - name: raw_eastmoney__equity_history
      - name: raw_eastmoney__income_sq
      - name: raw_eastmoney__income_ytd
      - name: raw_ths__limit_up_pool
      - name: raw_ths__limit_up_pool_compacted
      - name: raw_jiuyan__industry_ocr
```

### dbt Staging 模型

```sql
-- models/staging/stg_baostock__stock_basic.sql
-- 类型已在 raw 层优化，staging 专注于业务逻辑
select
    code,
    code_name,
    ipo_date,
    out_date,
    type,
    status
from {{ source('dagster', 'raw_baostock__query_stock_basic') }}
```

### Dagster-dbt 集成配置

```yaml
# scheduler 项目中的 defs.yaml
type: dagster_dbt.DbtProjectComponent
attributes:
  project:
    project-path: elt
  select: "staging+marts"  # 只运行 staging 和 marts，跳过 raw
  cli-args:
    - build
    - --full-refresh
```

## 数据流时序

```
1. Dagster schedule 触发 raw 资产物化
2. Dagster 写入 S3 parquet (按真实类型)
3. Dagster 执行 ClickHouse SQL:
   a. CREATE TABLE raw_xxx_tmp AS raw_xxx
   b. INSERT INTO raw_xxx_tmp SELECT * FROM s3(...)
   c. ALTER TABLE raw_xxx REPLACE PARTITION '2025' FROM raw_xxx_tmp
   d. DROP TABLE raw_xxx_tmp
4. Dagster 标记 raw 资产为已物化
5. Declarative Automation 检测到 raw 资产更新
6. Dagster 触发 dbt build --select staging+marts
7. dbt 执行 staging views (业务逻辑转换、列重命名)
8. dbt 执行 marts tables (业务聚合)
9. 完成
```

## S3 Key 模式

所有 parquet key 遵循以下模板：

```
{bucket}/source/{asset_name}/[{partition_name}={partition_value}/]000000_0.parquet
```

具体示例：

```
s3://<bucket>/source/sina__trade_calendar/000000_0.parquet
s3://<bucket>/source/baostock__query_stock_basic/000000_0.parquet
s3://<bucket>/source/baostock__query_history_k_data_plus_daily/year=2024/000000_0.parquet
s3://<bucket>/source/ths__limit_up_pool/trade_date=2025-03-15/000000_0.parquet
s3://<bucket>/source/eastmoney__balance/year=2024/000000_0.parquet
```

关键特性：
- 每个分区一个文件，文件名统一为 `000000_0.parquet`
- 压缩格式：zstd
- 列类型按真实数据类型存储（详见前置改造章节）

## ClickHouse Raw 表设计

### 建表模板

```sql
CREATE TABLE raw_baostock__query_history_k_data_plus_daily (
    date Date,
    code String,
    open Float64,
    high Float64,
    low Float64,
    close Float64,
    preclose Float64,
    volume Int64,
    amount Float64,
    adjustflag String,
    turn Float64,
    tradestatus String,
    pctChg Float64,
    isST String
)
ENGINE = MergeTree()
PARTITION BY toYear(date)
ORDER BY (code, date);
```

### 设计要点

1. **类型与 S3 parquet 一致**：ClickHouse 列类型与 S3 parquet schema 对齐，无需 CAST
2. **分区键与 S3 分区对齐**：`toYear(date)` 分区对应 S3 的 `year=YYYY/` 路径
3. **排序键选择**：按查询模式选择，如 `(code, date)` 适合按证券代码+日期查询
4. **原生类型优势**：ClickHouse 可以利用 SIMD 指令加速数值列的聚合、排序、过滤

## 验收标准

1. S3 parquet schema 类型优化完成，所有数据源使用真实数据类型
2. ClickHouse raw 表建表语句完成，列类型与 S3 parquet 一致
3. dbt 项目配置完成，包含 raw、staging、marts 三层模型
4. Dagster raw 资产定义完成，包含 S3 写入和 ClickHouse 刷新逻辑
5. dbt 通过 `source()` 正确引用 raw 表作为 Dagster asset
6. Declarative Automation 配置完成，raw 资产更新后自动触发 dbt 运行
7. 所有现有 asset key、S3 路径、数据语义保持不变
8. 端到端数据流验证通过：Dagster → S3 → ClickHouse raw → dbt staging → dbt marts
9. 现有测试套件通过（可能需要更新测试夹具以反映新 schema）
10. 压缩率提升 30%+（数值列从 string 改为原生类型后）

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Schema 类型迁移导致数据丢失 | 旧数据无法正确解析 | 迁移前备份，分批迁移，验证数据完整性 |
| ClickHouse 表结构变更 | 依赖旧表结构的查询失败 | 提前通知下游，提供迁移指南 |
| 现有测试失败 | 测试夹具依赖旧 schema | 更新测试夹具，确保测试通过 |
| `REPLACE PARTITION` 性能 | 大分区替换耗时 | 监控替换耗时，必要时优化分区粒度 |
| dbt `--full-refresh` 每次全量 | staging/marts 重建开销 | 仅在 schema 变更时使用 full-refresh |
| Declarative Automation 未启用 | dbt 不会自动触发 | 确保 `default_automation_condition_sensor` 已启用 |
| `dbt-clickhouse` adapter 兼容性 | dbt 无法连接 ClickHouse | 测试 adapter 版本兼容性 |
| 空分区处理 | ClickHouse 查询空分区报错 | 确保空分区返回空结果集而非报错 |

## 参考资料

- RFC 0006: Pipeline Scheduler 代码质量、模块化与可复用性优化
- Dagster-dbt Integration: https://docs.dagster.io/integrations/libraries/dbt
- ClickHouse MergeTree: https://clickhouse.com/docs/en/engines/table-engines/mergetree-family
- ClickHouse s3() Table Function: https://clickhouse.com/docs/en/sql-reference/table-functions/s3
- ClickHouse Data Types: https://clickhouse.com/docs/en/sql-reference/data-types
- Apache Parquet Format: https://parquet.apache.org/documentation/latest/

## 实施顺序

本 RFC 的实施必须按以下顺序进行：

### 阶段 1：S3 Parquet Schema 类型优化（前置改造）

1. 定义各数据源的真实 schema（BaoStock、EastMoney、THS、Sina、Jiuyan）
2. 改造 Dagster 资产的 `pa.Table` 构造逻辑，使用新 schema
3. 验证 S3 parquet 文件的 schema 正确性
4. 测试压缩率提升效果

### 阶段 2：ClickHouse Raw 表设计

1. 设计 raw 表的建表语句，列类型与 S3 parquet 一致
2. 实现 `REPLACE PARTITION` 刷新逻辑
3. 测试分区替换性能

### 阶段 3：dbt 项目搭建

1. 配置 dbt 项目和 ClickHouse adapter
2. 创建 raw、staging、marts 三层模型
3. 配置 Declarative Automation

### 阶段 4：端到端集成测试

1. 验证完整数据流：Dagster → S3 → ClickHouse raw → dbt staging → dbt marts
2. 测试分区刷新和 dbt 触发机制
3. 性能基准测试
