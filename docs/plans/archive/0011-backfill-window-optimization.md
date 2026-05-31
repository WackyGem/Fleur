# Plan 0011: 回填窗口优化

状态：草案

计划日期：2026-05-30

关联 RFC：

- 无独立 RFC，本计划自包含设计决策

参考资料：

- `pipeline/scheduler/src/scheduler/defs/http/partitioning.py`
- `pipeline/scheduler/src/scheduler/defs/sources/ths/limit_up_pool.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/action_field.py`
- `pipeline/scheduler/src/scheduler/defs/market/trade_calendar.py`

## 目标

优化 `ths__limit_up_pool` 和 `jiuyan__action_field` 两个资产的回填窗口限制逻辑：

1. **`ths__limit_up_pool`**：移除硬编码的 20 分区限制，改为动态裁切：如果回填窗口超过执行日期往回推 380 个自然日，则自动裁切到 380 个自然日。交易日过滤逻辑保持不变。

2. **`jiuyan__action_field`**：移除硬编码的 20 分区限制，改为动态裁切：最多保留 80 个交易日。交易日过滤逻辑保持不变。

核心目标：

- 提升大规模回填的灵活性
- 保持对上游 API 的合理保护
- 确保回填窗口在数据源保留期内
- 保持向后兼容，不影响现有的 schedule 触发逻辑

## 非目标

本计划不包含：

- 修改 schedule 的 cron 表达式或触发逻辑
- 修改 `materialize_partition_range()` 的并发控制逻辑
- 修改资产的分区定义（`DailyPartitionsDefinition`）
- 修改 S3 写入逻辑或 parquet 格式
- 修改 `build_trade_date_schedule()` 的交易日过滤逻辑

## 当前状态分析

### 现有限制

```python
# partitioning.py
TRADE_DATE_BACKFILL_HARD_LIMIT = 20

# materialize_trade_date_range() 中
if len(partition_keys) > TRADE_DATE_BACKFILL_HARD_LIMIT:
    msg = (
        "Single-run market-event backfill is limited to "
        f"{TRADE_DATE_BACKFILL_HARD_LIMIT} natural-date partitions"
    )
    raise ValueError(msg)
```

### 问题

1. **硬编码限制过小**：20 个自然日分区对于历史回填来说太小
2. **不区分数据源特性**：不同数据源有不同的保留期和业务需求
3. **错误信息不明确**：用户不知道为什么要限制 20 个分区

### 数据源特性

| 数据源 | API 保留期 | 业务需求 | 当前限制 |
|--------|-----------|---------|---------|
| THS 涨停池 | 380 天 | 需要回填较长时间窗口 | 20 个自然日 |
| 韭研异动 | 未知 | 需要回填较多交易日 | 20 个自然日 |

## 设计决策

### 决策 1：THS 涨停池回填窗口

**选择：380 个自然日，从执行日期往前推**

理由：

1. **API 保留期**：THS API 保留 380 天数据，超过后返回 `status_code: -1`
2. **避免无效请求**：裁切到 380 天可以避免对已过期数据的无效 API 调用
3. **覆盖完整年度**：380 天足够覆盖一个完整自然年 + 节假日
4. **动态裁切**：自动裁切比硬编码更灵活

实现逻辑：

```python
# 执行日期
execution_date = date.today()  # 或从 context 获取

# 最早可用日期
earliest_allowed = execution_date - timedelta(days=380)

# 裁切分区键
truncated_keys = [
    key for key in partition_keys
    if _parse_date_partition_key(key) >= earliest_allowed
]

# 如果有裁切，记录日志
if len(truncated_keys) < len(partition_keys):
    logger.warning(
        f"Truncated backfill window from {len(partition_keys)} to {len(truncated_keys)} "
        f"partitions (earliest allowed: {earliest_allowed.isoformat()})"
    )
```

### 决策 2：韭研异动回填窗口

**选择：80 个交易日**

理由：

1. **业务需求**：80 个交易日约等于 4 个月，足够覆盖季度分析
2. **API 压力控制**：韭研 API 可能有请求频率限制
3. **执行时间**：80 个交易日的回填在合理时间内完成
4. **交易日而非自然日**：韭研数据只在交易日有意义

实现逻辑：

```python
# 读取交易日历
s3_config = S3Config.from_env()
calendar_dates = read_trade_dates_from_s3(s3_config)

# 获取所有请求的交易日
all_trade_dates = sorted([
    _parse_date_partition_key(key)
    for key in partition_keys
    if _parse_date_partition_key(key) in calendar_dates
])

# 裁切到 80 个交易日
if len(all_trade_dates) > 80:
    truncated_trade_dates = all_trade_dates[-80:]  # 保留最近 80 个交易日
    truncated_keys = {d.isoformat() for d in truncated_trade_dates}
    partition_keys = [key for key in partition_keys if key in truncated_keys]
```

### 决策 3：配置化

**选择：将限制参数提取为可配置常量**

理由：

1. **灵活性**：不同环境可能需要不同的限制
2. **可维护性**：常量集中在一处，便于调整
3. **可测试性**：测试时可以覆盖常量

```python
# partitioning.py
THS_BACKFILL_MAX_NATURAL_DAYS = 380
JIUYAN_BACKFILL_MAX_TRADE_DATES = 80
```

## 实施方案

### 步骤 1：修改 `partitioning.py`

#### 1.1 新增常量

```python
# 回填窗口限制
THS_BACKFILL_MAX_NATURAL_DAYS = 380
JIUYAN_BACKFILL_MAX_TRADE_DATES = 80
```

#### 1.2 修改 `materialize_trade_date_range()` 签名

新增 `backfill_window_limit` 参数：

```python
async def materialize_trade_date_range(
    context: PartitionedAssetContextProtocol,
    *,
    max_concurrent_trade_dates: int,
    fetch_table_for_trade_date: FetchTableForTradeDate,
    backfill_window_limit: int | None = None,  # 新增：回填窗口限制
) -> TradeDateRangeMaterializationResult:
```

#### 1.3 修改回填窗口裁切逻辑

```python
async def materialize_trade_date_range(
    context: PartitionedAssetContextProtocol,
    *,
    max_concurrent_trade_dates: int,
    fetch_table_for_trade_date: FetchTableForTradeDate,
    backfill_window_limit: int | None = None,
) -> TradeDateRangeMaterializationResult:
    if max_concurrent_trade_dates < 1:
        msg = "max_concurrent_trade_dates must be positive"
        raise ValueError(msg)

    partition_keys = sorted(context.partition_keys)
    if not partition_keys:
        msg = "Market-event asset requires at least one trade_date partition"
        raise RuntimeError(msg)

    # 回填窗口裁切
    if backfill_window_limit is not None and len(partition_keys) > backfill_window_limit:
        partition_keys = partition_keys[-backfill_window_limit:]

    s3_config = S3Config.from_env()
    natural_dates = [_parse_date_partition_key(key) for key in partition_keys]
    calendar_dates = read_trade_dates_from_s3(s3_config)
    trade_dates = [item for item in natural_dates if item in calendar_dates]
    skipped_non_trade_dates = [item for item in natural_dates if item not in calendar_dates]
    trade_date_keys = {item.isoformat() for item in trade_dates}

    # ... 其余逻辑不变
```

#### 1.4 移除硬编码限制

移除以下代码：

```python
# 移除
if max_concurrent_trade_dates > TRADE_DATE_BACKFILL_HARD_LIMIT:
    msg = f"max_concurrent_trade_dates must be <= {TRADE_DATE_BACKFILL_HARD_LIMIT}"
    raise ValueError(msg)

# 移除
if len(partition_keys) > TRADE_DATE_BACKFILL_HARD_LIMIT:
    msg = (
        "Single-run market-event backfill is limited to "
        f"{TRADE_DATE_BACKFILL_HARD_LIMIT} natural-date partitions"
    )
    raise ValueError(msg)
```

### 步骤 2：修改 `ths/limit_up_pool.py`

#### 2.1 更新导入

```python
from scheduler.defs.http.partitioning import (
    TRADE_DATE_PARTITION_KEY_NAME,
    TradeDateRangeMaterializationResult,
    materialize_trade_date_range,
    ths_limit_up_pool_daily_partitions,
    THS_BACKFILL_MAX_NATURAL_DAYS,  # 新增
)
```

#### 2.2 修改资产调用

```python
async def _materialize_limit_up_pool_range(
    context: dg.AssetExecutionContext,
    config: MarketEventBackfillConfig,
) -> TradeDateRangeMaterializationResult:
    async with AioHttpClient(
        headers=with_referer(browser_json_headers(), THS_LIMIT_UP_POOL_REFERER),
        retry_policy=DEFAULT_RETRY_POLICY,
    ) as client:
        result = await materialize_trade_date_range(
            context,
            max_concurrent_trade_dates=config.max_concurrent_trade_dates,
            fetch_table_for_trade_date=lambda trade_date: fetch_limit_up_pool_table_with_client(
                client,
                trade_date=trade_date,
            ),
            backfill_window_limit=THS_BACKFILL_MAX_NATURAL_DAYS,  # 新增
        )
        result.metadata.update(http_stats_metadata(client.stats))
        return result
```

### 步骤 3：修改 `jiuyan/action_field.py`

#### 3.1 更新导入

```python
from scheduler.defs.http.partitioning import (
    TRADE_DATE_PARTITION_KEY_NAME,
    TradeDateRangeMaterializationResult,
    jiuyan_action_field_daily_partitions,
    materialize_trade_date_range,
    JIUYAN_BACKFILL_MAX_TRADE_DATES,  # 新增
)
```

#### 3.2 修改资产调用

```python
async def _materialize_action_field_range(
    context: dg.AssetExecutionContext,
    config: MarketEventBackfillConfig,
) -> TradeDateRangeMaterializationResult:
    async with AioHttpClient(
        headers=jiuyan_header_factory(),
        retry_policy=DEFAULT_RETRY_POLICY,
    ) as client:
        result = await materialize_trade_date_range(
            context,
            max_concurrent_trade_dates=config.max_concurrent_trade_dates,
            fetch_table_for_trade_date=lambda trade_date: fetch_action_field_table_with_client(
                client,
                trade_date=trade_date,
            ),
            backfill_window_limit=JIUYAN_BACKFILL_MAX_TRADE_DATES,  # 新增
        )
        result.metadata.update(http_stats_metadata(client.stats))
        return result
```

### 步骤 4：更新测试

#### 4.1 更新 `test_partitioning.py`

```python
def test_materialize_trade_date_range_applies_backfill_window_limit():
    """测试回填窗口限制逻辑。"""
    # 创建 mock context，包含 100 个分区
    # 调用 materialize_trade_date_range，设置 backfill_window_limit=50
    # 验证只处理了 50 个分区
    pass


def test_materialize_trade_date_range_no_limit_when_none():
    """测试不设置限制时的行为。"""
    # 创建 mock context，包含 100 个分区
    # 调用 materialize_trade_date_range，不设置 backfill_window_limit
    # 验证处理了所有 100 个分区
    pass
```

#### 4.2 更新 `test_ths_limit_up_pool.py`

```python
def test_ths_limit_up_pool_uses_380_day_limit():
    """测试 THS 资产使用 380 天限制。"""
    # 验证 _materialize_limit_up_pool_range 传递了正确的 backfill_window_limit
    pass
```

#### 4.3 更新 `test_jiuyan_action_field.py`

```python
def test_jiuyan_action_field_uses_80_trade_date_limit():
    """测试韭研资产使用 80 个交易日限制。"""
    # 验证 _materialize_action_field_range 传递了正确的 backfill_window_limit
    pass
```

## 验收标准

1. `ths__limit_up_pool` 资产支持超过 20 个分区的回填
2. `jiuyan__action_field` 资产支持超过 20 个分区的回填
3. THS 回填窗口自动裁切到 380 个自然日
4. 韭研回填窗口自动裁切到 80 个交易日
5. 现有 schedule 触发逻辑不受影响
6. 所有现有测试通过
7. 新增测试覆盖回填窗口限制逻辑

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 大规模回填导致 API 限流 | 请求被拒绝 | 并发控制 `max_concurrent_trade_dates` 保持不变 |
| 回填窗口过大导致内存溢出 | 进程崩溃 | 保持 `backfill_window_limit` 合理值 |
| 交易日历不可用 | 无法过滤非交易日 | 保持现有 fallback 逻辑 |
| 向后兼容性 | 现有调用方受影响 | `backfill_window_limit` 参数默认为 `None`（无限制） |

## 向后兼容性

| 维度 | 兼容性 | 说明 |
|------|--------|------|
| `materialize_trade_date_range()` 签名 | ✅ 兼容 | 新增参数有默认值 |
| `TRADE_DATE_BACKFILL_HARD_LIMIT` | ❌ 移除 | 不再使用硬编码限制 |
| 现有 schedule 触发 | ✅ 兼容 | 不受影响 |
| 现有回填逻辑 | ✅ 兼容 | 默认无限制，需要显式设置 |

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `http/partitioning.py` | 修改 | 新增常量，修改 `materialize_trade_date_range()` 签名和逻辑 |
| `sources/ths/limit_up_pool.py` | 修改 | 传递 `backfill_window_limit=THS_BACKFILL_MAX_NATURAL_DAYS` |
| `sources/jiuyan/action_field.py` | 修改 | 传递 `backfill_window_limit=JIUYAN_BACKFILL_MAX_TRADE_DATES` |
| `tests/unit/http/test_partitioning.py` | 修改 | 新增回填窗口限制测试 |
| `tests/unit/sources/ths/test_limit_up_pool.py` | 修改 | 新增 THS 限制测试 |
| `tests/unit/sources/jiuyan/test_action_field.py` | 修改 | 新增韭研限制测试 |

## 实施顺序

1. **步骤 1**：修改 `partitioning.py`（低风险，核心逻辑）
2. **步骤 2**：修改 `ths/limit_up_pool.py`（低风险，简单修改）
3. **步骤 3**：修改 `jiuyan/action_field.py`（低风险，简单修改）
4. **步骤 4**：更新测试（中风险，需要验证）
5. **步骤 5**：运行完整测试套件（验证）
6. **步骤 6**：运行质量门禁（验证）

## 测试执行顺序

```bash
# 1. 运行分区测试
cd pipeline
uv run pytest scheduler/tests/unit/http/test_partitioning.py -v

# 2. 运行 THS 测试
uv run pytest scheduler/tests/unit/sources/ths/ -v

# 3. 运行韭研测试
uv run pytest scheduler/tests/unit/sources/jiuyan/ -v

# 4. 运行完整测试套件
uv run pytest scheduler/tests/ -v

# 5. 运行质量门禁
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run dg check defs
```
