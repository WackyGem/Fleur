# Plan 0010: S3 Parquet Schema 类型优化实施计划

状态：草案

计划日期：2026-05-30

关联 RFC：

- `docs/RFC/0007-dbt-raw-layer-and-dagster-dbt-integration.md`（阶段 1 前置改造）

参考资料：

- `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/http/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/fields.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py`
- `pipeline/scheduler/src/scheduler/defs/sources/sina/trade_calendar.py`
- `pipeline/scheduler/src/scheduler/defs/common/strings.py`
- `pipeline/scheduler/src/scheduler/defs/storage/parquet.py`
- `pipeline/scheduler/tests/`

## 目标

将 S3 parquet 文件的列类型从全 `pa.string()` 改为真实数据类型，为后续 ClickHouse raw 层和 dbt 集成奠定基础。

核心目标：

- 为每个数据源定义带类型的 schema，区分日期、数值、字符串、布尔列。
- 改造 `pa.Table` 构造逻辑，使用原生 PyArrow 类型。
- 保持 S3 路径、asset key、数据语义不变。
- 现有测试套件全部通过（可能需要更新测试夹具）。

## 非目标

本计划不包含：

- ClickHouse raw 表建表和 `REPLACE PARTITION` 逻辑（RFC 0007 阶段 2）。
- dbt 模型创建和 Declarative Automation 配置（RFC 0007 阶段 3）。
- 修改 S3 路径结构、asset key 或分区策略。
- 修改 Dagster schedule、sensor 或 job 定义。
- 修改 BaoStock TCP 协议、HTTP 客户端或 API 请求逻辑。

## 当前状态分析

### 数据源 Schema 概览

| 数据源 | 文件 | 列数 | 当前类型 | 目标类型分布 |
|--------|------|------|---------|-------------|
| BaoStock Stock Basic | `baostock/schemas.py` | 6 | 全 string | 2 date32, 2 string, 2 int8 |
| BaoStock K History Daily | `baostock/schemas.py` | 14 | 全 string | 1 date32, 2 string, 7 float64, 2 int64, 2 int8 |
| EastMoney Balance | `eastmoney/schema.py` | 319 | 全 string | ~150 date32, ~150 float64, ~10 string, ~9 bool |
| EastMoney Cashflow SQ | `eastmoney/schema.py` | 372 | 全 string | ~180 date32, ~180 float64, ~10 string, ~2 bool |
| EastMoney Cashflow YTD | `eastmoney/schema.py` | 254 | 全 string | ~120 date32, ~120 float64, ~10 string, ~4 bool |
| EastMoney Dividend Allotment | `eastmoney/schema.py` | 10 | 全 string | 3 date32, 4 float64, 2 string, 1 bool |
| EastMoney Dividend Main | `eastmoney/schema.py` | 30 | 全 string | 5 date32, 15 float64, 8 string, 2 bool |
| EastMoney Equity History | `eastmoney/schema.py` | 69 | 全 string | 5 date32, 40 float64, 20 string, 4 bool |
| EastMoney Income SQ | `eastmoney/schema.py` | 299 | 全 string | ~150 date32, ~140 float64, ~5 string, ~4 bool |
| EastMoney Income YTD | `eastmoney/schema.py` | 203 | 全 string | ~100 date32, ~95 float64, ~5 string, ~3 bool |
| THS Limit Up Pool | `http/schemas.py` | 22 | 全 string | 1 date32, 7 float64, 2 int64, 2 timestamp, 5 string, 2 bool |
| JiuYan Action Field | `http/schemas.py` | 18 | 全 string | 1 date32, 1 float64, 3 int64, 12 string |
| JiuYan Industry List | `http/schemas.py` | 17 | 全 string | 3 int64, 2 bool, 12 string |
| JiuYan Industry OCR | `jiuyan/ocr_schema.py` | 5 | 全 string | 5 string（无需改动） |
| Sina Trade Calendar | `sina/trade_calendar.py` | 1 | `pa.date32()` | ✅ 已优化 |

### 关键转换函数

| 函数 | 位置 | 作用 | 改造影响 |
|------|------|------|---------|
| `string_or_null()` | `common/strings.py` | 任意值 → `str \| None` | 保留作为 fallback，新增类型感知版本 |
| `eastmoney_string_or_null()` | `eastmoney/schema.py` | 任意值 → `str \| None` | 保留作为 fallback，新增类型感知版本 |
| `rows_to_string_table()` | `http/schemas.py` | rows → 全 string pa.Table | 保留作为 fallback，新增 `rows_to_typed_table()` |
| `response_to_table()` | `baostock/schemas.py` | BaoStock response → 全 string pa.Table | 修改为类型感知版本 |
| `eastmoney_rows_to_table()` | `eastmoney/schema.py` | EastMoney rows → 全 string pa.Table | 修改为类型感知版本 |
| `to_date32()` | `common/types.py` (新增) | 任意值 → `date \| None` | 支持多种日期格式 |
| `to_float64()` | `common/types.py` (新增) | 任意值 → `float \| None` | 支持字符串和数值 |
| `to_int64()` | `common/types.py` (新增) | 任意值 → `int \| None` | 支持字符串和数值 |
| `to_int8()` | `common/types.py` (新增) | 任意值 → `int \| None` | 小整数，支持标志位 |
| `to_bool()` | `common/types.py` (新增) | 任意值 → `bool \| None` | 支持 '0'/'1' 和 0/1 |
| `to_timestamp()` | `common/types.py` (新增) | Unix 时间戳 → `datetime \| None` | THS 时间字段，转为 Asia/Shanghai 时区 |

## 设计决策

### 决策 0：时间字段处理策略

**选择：根据 API 返回的实际格式选择最合适的类型。**

通过 OpenAPI 示例数据验证，各数据源的时间字段格式如下：

| 数据源 | 字段 | API 格式 | 选择类型 | 说明 |
|--------|------|---------|---------|------|
| BaoStock | `date` | `'2026-05-25'` | `pa.date32()` | 纯日期 |
| BaoStock | `ipoDate`, `outDate` | `'1999-11-10'` 或 `''` | `pa.date32()` | 纯日期，空字符串转 null |
| EastMoney | `NOTICE_DATE`, `REPORT_DATE` 等 | `'2026-03-31 00:00:00'` | `pa.date32()` | datetime 格式，时间部分始终为 00:00:00 |
| EastMoney | `REPORT_DATE_NAME` | `'2026一季报'` | `pa.string()` | 文本名称，非日期 |
| EastMoney | `IS_*`, `HAS_*` | `'0'`/`'1'` | `pa.bool_()` | 字符串编码的布尔值 |
| THS | `date` | `'20260508'` | `pa.date32()` | YYYYMMDD 格式 |
| THS | `first_limit_up_time` | `'1776130842'` | `pa.timestamp("ns", tz="UTC")` | Unix 时间戳 → UTC |
| THS | `is_new`, `is_again_limit` | `0`/`1` | `pa.bool_()` | 数字编码的布尔值 |
| Sina | `trade_date` | `date` 对象 | `pa.date32()` | 已优化，无需改动 |

**理由：**

1. **日期字段统一为 `pa.date32()`**：所有日期字段（无论格式是 `YYYY-MM-DD`、`YYYY-MM-DD 00:00:00` 还是 `YYYYMMDD`）都转为 `pa.date32()`，节省存储空间并支持 ClickHouse 日期函数。

2. **Unix 时间戳转为 `pa.timestamp("ns", tz="UTC")`**：THS 的 `first_limit_up_time` 等字段是 Unix 时间戳，转为 UTC 时区的 timestamp，因为：
   - UTC 是系统内部时间存储标准，避免时区转换错误和 DST 问题
   - ClickHouse 推荐 UTC 存储，查询时用 `toTimeZone()` 按需转换
   - 跨系统 interoperability 更好
   - 下游查询时可用 `toTimeZone(first_limit_up_time, 'Asia/Shanghai')` 转为本地时间

3. **布尔值统一为 `pa.bool_()`**：无论 API 返回 `'0'`/`'1'` 字符串还是 `0`/`1` 数字，都转为 `pa.bool_()`，节省存储空间并支持 ClickHouse 布尔查询。

### 决策 1：Schema 定义方式

**选择：为每个数据源定义带类型的 `pa.Schema`，保持 `pa.field()` 声明式风格。**

```python
# 示例：BaoStock K History Daily
K_HISTORY_DAILY_SCHEMA = pa.schema([
    pa.field("date", pa.date32()),
    pa.field("code", pa.string()),
    pa.field("open", pa.float64()),
    pa.field("high", pa.float64()),
    pa.field("low", pa.float64()),
    pa.field("close", pa.float64()),
    pa.field("preclose", pa.float64()),
    pa.field("volume", pa.int64()),
    pa.field("amount", pa.float64()),
    pa.field("adjustflag", pa.int8()),
    pa.field("turn", pa.float64()),
    pa.field("tradestatus", pa.int8()),
    pa.field("pctChg", pa.float64()),
    pa.field("isST", pa.int8()),
])
```

**理由：**

1. 声明式 schema 便于维护和文档化。
2. PyArrow 在写入 parquet 时会自动校验类型，提前发现数据问题。
3. 与 ClickHouse 建表语句一一对应。

### 决策 2：类型转换策略

**选择：在 `pa.Table` 构造阶段做类型转换，而非在值收集阶段。**

保持现有的值收集逻辑（收集为 Python 原生类型），在构造 `pa.Table` 时通过 schema 指定类型，让 PyArrow 自动转换。

```python
# 当前：全 string
columns = {"open": ["8.9400", "9.1200"]}
schema = pa.schema([("open", pa.string())])
table = pa.table(columns, schema=schema)

# 改造后：float64
columns = {"open": [8.94, 9.12]}  # Python float
schema = pa.schema([("open", pa.float64())])
table = pa.table(columns, schema=schema)
```

**理由：**

1. 最小化代码改动，只修改 schema 定义和值转换函数。
2. PyArrow 的类型推断和转换逻辑比手写更可靠。
3. 转换失败会在写入时立即暴露，而非延迟到查询时。

### 决策 3：空值处理

**选择：保持 `None` 作为 null 值，PyArrow 自动处理 nullable 列。**

```python
# None 值在任何类型列中都是合法的
columns = {"open": [8.94, None, 9.12]}
schema = pa.schema([pa.field("open", pa.float64())])  # nullable=True (default)
table = pa.table(columns, schema=schema)
```

**理由：**

1. PyArrow 原生支持 nullable 列，无需特殊处理。
2. 与 ClickHouse 的 `Nullable()` 类型对齐。

### 决策 4：EastMoney 字段类型识别

**选择：通过字段名模式自动推断类型，无需手动标注每个字段。**

EastMoney 的 8 个端点有 200-370 个字段，手动标注不现实。通过字段名模式推断：

| 字段名模式 | 推断类型 | 示例 | 说明 |
|-----------|---------|------|------|
| `*_DATE`（排除 `*_DATE_NAME`） | `pa.date32()` | `REPORT_DATE`, `NOTICE_DATE` | API 返回 `'YYYY-MM-DD 00:00:00'` 格式 |
| `*_DATE_NAME` | `pa.string()` | `REPORT_DATE_NAME` | 文本名称如 `2026一季报`，非日期 |
| `*_TIME` | `pa.string()` | `REPORT_TIME` | API 返回 `'2025-06-30 00:00:00'` 格式，可转 date32 |
| 以 `IS_` / `HAS_` 开头 | `pa.bool_()` | `IS_UNASSIGN`, `IS_PAYCASH` | API 返回 `'0'`/`'1'` 字符串 |
| `*_CODE` / `*_NAME` / `*_ABBR` | `pa.string()` | `SECURITY_CODE`, `SECURITY_NAME_ABBR` | 标识符和名称 |
| `*_RATIO` / `*_RATE` / `_YOY` / `_QOQ` | `pa.float64()` | `LIMITED_SHARES_RATIO`, `TOTAL_ASSETS_YOY` | 比率和同比/环比 |
| 其他数值字段 | `pa.float64()` | `TOTAL_ASSETS`, `NETPROFIT` | 金额和数量 |
| 其他文本字段 | `pa.string()` | `CHANGE_REASON`, `EVENT_EXPLAIN` | 自由文本 |

**关键发现（来自 OpenAPI 示例数据）：**

1. **日期字段**：所有 `*_DATE` 字段（除 `*_DATE_NAME`）格式为 `'YYYY-MM-DD 00:00:00'`，时间部分始终为 `00:00:00`，可安全转为 `pa.date32()`。

2. **时间字段**：`*_TIME` 字段如 `REPORT_TIME` 格式也是 `'YYYY-MM-DD 00:00:00'`，同样可转为 `pa.date32()`。

3. **布尔字段**：`IS_*` / `HAS_*` 字段在 API 中返回为字符串 `'0'` 或 `'1'`，应转为 `pa.bool_()`。

4. **例外字段**：
   - `REPORT_DATE_NAME`: `2026一季报` — 文本名称，非日期
   - `*_TODAY` 字段: `'0'`/`'1'` — 布尔值，非日期

**理由：**

1. EastMoney 字段命名高度规范化，模式推断准确率高。
2. 避免手动标注 2000+ 个字段的巨大工作量。
3. 新增字段自动继承类型推断规则。
4. 通过 OpenAPI 示例数据验证了推断规则的准确性。

## 实施方案

### 阶段 1：通用类型转换基础设施

**目标：** 创建类型感知的值转换函数和 schema 构建工具。

#### 1.1 新增 `common/types.py`

```python
"""类型感知的值转换函数，替代全 string 转换。"""

from __future__ import annotations

import json
from datetime import date, datetime, timezone
from typing import Any

import pyarrow as pa


def to_date32(value: Any) -> date | None:
    """将值转换为 date，用于 pa.date32() 列。

    支持格式：
    - 'YYYY-MM-DD' (BaoStock, Sina)
    - 'YYYY-MM-DD 00:00:00' (EastMoney)
    - 'YYYYMMDD' (THS)
    - date/datetime 对象
    """
    if value is None:
        return None
    if isinstance(value, date) and not isinstance(value, datetime):
        return value
    if isinstance(value, datetime):
        return value.date()
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        # 支持 YYYYMMDD 格式（THS）
        if len(cleaned) == 8 and cleaned.isdigit():
            return date(int(cleaned[:4]), int(cleaned[4:6]), int(cleaned[6:8]))
        # 支持 YYYY-MM-DD 和 YYYY-MM-DD 00:00:00 格式
        if "-" in cleaned:
            # 截取日期部分（忽略时间部分）
            date_part = cleaned.split(" ")[0]
            parts = date_part.split("-")
            return date(int(parts[0]), int(parts[1]), int(parts[2]))
    return None


def to_float64(value: Any) -> float | None:
    """将值转换为 float，用于 pa.float64() 列。"""
    if value is None:
        return None
    if isinstance(value, bool):
        return 1.0 if value else 0.0
    if isinstance(value, int | float):
        return float(value)
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        try:
            return float(cleaned)
        except ValueError:
            return None
    return None


def to_int64(value: Any) -> int | None:
    """将值转换为 int，用于 pa.int64() 列。"""
    if value is None:
        return None
    if isinstance(value, bool):
        return 1 if value else 0
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        try:
            return int(cleaned)
        except ValueError:
            return None
    return None


def to_int8(value: Any) -> int | None:
    """将值转换为 int8，用于小整数列（标志位、状态码）。"""
    result = to_int64(value)
    if result is None:
        return None
    if -128 <= result <= 127:
        return result
    return None


def to_timestamp(value: Any) -> datetime | None:
    """将 Unix 时间戳转为 UTC datetime。

    THS 时间字段格式：'1776130842'（秒级 Unix 时间戳）
    转换为 UTC datetime，作为系统内部时间存储标准。

    使用 UTC 的理由：
    - 避免时区转换错误和 DST 问题
    - ClickHouse 推荐 UTC 存储，查询时用 toTimeZone() 转换
    - 跨系统 interoperability 更好

    示例：
        >>> to_timestamp("1776130842")
        datetime(2026, 4, 14, 2, 0, 42, tzinfo=timezone.utc)
    """
    if value is None:
        return None
    if isinstance(value, datetime):
        return value
    if isinstance(value, int | float):
        # Unix 时间戳 → UTC datetime
        return datetime.fromtimestamp(value, tz=timezone.utc)
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return None
        try:
            return to_timestamp(int(cleaned))
        except ValueError:
            return None
    return None


def to_bool(value: Any) -> bool | None:
    """将值转换为 bool，用于 pa.bool_() 列。"""
    if value is None:
        return None
    if isinstance(value, bool):
        return value
    if isinstance(value, int | float):
        return bool(value)
    if isinstance(value, str):
        cleaned = value.strip().lower()
        if cleaned in ("true", "1", "yes"):
            return True
        if cleaned in ("false", "0", "no", ""):
            return False
    return None


def to_string(value: Any) -> str | None:
    """将值转换为 string，用于 pa.string() 列。保持原有 string_or_null 行为。"""
    if value is None:
        return None
    if isinstance(value, str):
        return value
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int | float):
        return str(value)
    return json.dumps(value, ensure_ascii=False, sort_keys=True, default=str)
```

#### 1.2 新增 `common/schema.py`

```python
"""Schema 构建工具，支持声明式类型定义。"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any, Callable

import pyarrow as pa


def typed_schema(
    fields: Sequence[tuple[str, pa.DataType]],
) -> pa.Schema:
    """从 (字段名, 类型) 元组列表构建 schema。"""
    return pa.schema([pa.field(name, dtype) for name, dtype in fields])


def typed_table(
    rows: Sequence[dict[str, Any]],
    schema: pa.Schema,
    converters: dict[str, Callable[[Any], Any]] | None = None,
) -> pa.Table:
    """从行数据和 schema 构建 pa.Table，自动应用类型转换。"""
    if converters is None:
        converters = {}

    columns: dict[str, list[Any]] = {}
    for field in schema:
        converter = converters.get(field.name, _default_converter(field.type))
        columns[field.name] = [converter(row.get(field.name)) for row in rows]

    return pa.table(columns, schema=schema)


def _default_converter(dtype: pa.DataType) -> Callable[[Any], Any]:
    """根据 PyArrow 类型返回默认转换函数。"""
    if pa.types.is_date(dtype):
        from scheduler.defs.common.types import to_date32
        return to_date32
    if pa.types.is_floating(dtype):
        from scheduler.defs.common.types import to_float64
        return to_float64
    if pa.types.is_integer(dtype):
        from scheduler.defs.common.types import to_int64
        return to_int64
    if pa.types.is_boolean(dtype):
        from scheduler.defs.common.types import to_bool
        return to_bool
    # 默认：string
    from scheduler.defs.common.types import to_string
    return to_string
```

### 阶段 2：BaoStock Schema 改造

**目标：** 将 BaoStock 2 个 schema 从全 string 改为带类型。

#### 2.1 修改 `baostock/schemas.py`

新增带类型 schema：

```python
STOCK_BASIC_SCHEMA = pa.schema([
    pa.field("code", pa.string()),
    pa.field("code_name", pa.string()),
    pa.field("ipoDate", pa.date32()),
    pa.field("outDate", pa.date32()),
    pa.field("type", pa.int8()),
    pa.field("status", pa.int8()),
])

K_HISTORY_DAILY_SCHEMA = pa.schema([
    pa.field("date", pa.date32()),
    pa.field("code", pa.string()),
    pa.field("open", pa.float64()),
    pa.field("high", pa.float64()),
    pa.field("low", pa.float64()),
    pa.field("close", pa.float64()),
    pa.field("preclose", pa.float64()),
    pa.field("volume", pa.int64()),
    pa.field("amount", pa.float64()),
    pa.field("adjustflag", pa.int8()),
    pa.field("turn", pa.float64()),
    pa.field("tradestatus", pa.int8()),
    pa.field("pctChg", pa.float64()),
    pa.field("isST", pa.int8()),
])
```

#### 2.2 修改 `response_to_table()`

改造为类型感知版本：

```python
def response_to_table(response: BaostockResponse, schema: pa.Schema) -> pa.Table:
    """将 BaoStock 响应转换为 pa.Table，使用指定 schema 的类型。"""
    field_names = [field.name for field in schema]
    if not field_names:
        return pa.table({})

    columns: dict[str, list[Any]] = {field_name: [] for field_name in field_names}
    for record in response.records:
        if len(record) != len(field_names):
            msg = (
                f"BaoStock {response.api_name} returned {len(record)} values "
                f"for {len(field_names)} fields"
            )
            raise BaostockProtocolError(msg)
        for index, field_name in enumerate(field_names):
            columns[field_name].append(record[index])

    # 使用 typed_table 做类型转换
    from scheduler.defs.common.schema import typed_table
    rows = [
        {field_name: columns[field_name][i] for field_name in field_names}
        for i in range(len(columns[field_names[0]])) if field_names
    ]
    return typed_table(rows, schema)
```

#### 2.3 修改 `stock_basic_response_to_table()` 和 `k_history_daily_response_to_table()`

```python
def stock_basic_response_to_table(response: BaostockResponse) -> pa.Table:
    table = response_to_table(response, STOCK_BASIC_SCHEMA)
    _validate_expected_columns(table, STOCK_BASIC_FIELDS, response.api_name)
    return table


def k_history_daily_response_to_table(response: BaostockResponse) -> pa.Table:
    table = response_to_table(response, K_HISTORY_DAILY_SCHEMA)
    if table.num_columns == 0:
        return pa.table(
            {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
            schema=K_HISTORY_DAILY_SCHEMA,
        )
    _validate_expected_columns(table, K_HISTORY_DAILY_FIELDS, response.api_name)
    return table
```

#### 2.4 修改 `baostock/assets.py` 中的 `empty_k_history_table()`

```python
def empty_k_history_table() -> pa.Table:
    from scheduler.defs.baostock.schemas import K_HISTORY_DAILY_SCHEMA
    return pa.table(
        {field.name: [] for field in K_HISTORY_DAILY_SCHEMA},
        schema=K_HISTORY_DAILY_SCHEMA,
    )
```

### 阶段 3：EastMoney Schema 改造

**目标：** 将 EastMoney 8 个端点的 schema 从全 string 改为带类型。

#### 3.1 新增 `eastmoney/schema.py` 中的类型推断逻辑

```python
import re
from datetime import date, datetime

import pyarrow as pa

from scheduler.defs.common.types import to_date32, to_float64, to_bool, to_string


# 字段名模式 → PyArrow 类型映射
# 注意：必须先排除 *_DATE_NAME，再匹配 *_DATE
_DATE_NAME_PATTERN = re.compile(r"^(.*_DATE_NAME)$")
_DATE_PATTERN = re.compile(r"^(.*_DATE|LISTING_DATE|END_DATE)$")
_BOOL_PATTERN = re.compile(r"^(IS_|HAS_|)_")
_NUMERIC_PATTERN = re.compile(
    r"^(TOTAL_|NET_|PARENT_|OPERATE_|SALES_|ASSETS|LIABILITIES|EQUITY|"
    r"CASH|INVEST|FINANCE|RECEIVE|PAY|PROFIT|INCOME|COST|EXPENSE|"
    r"TAX|DEPRECIATION|AMORTIZATION|IMPAIRMENT|"
    r"LIMITED_|UNLIMITED_|SHARES|STOCK|BOND|"
    r".*_RATIO|.*_RATE|.*_YOY|.*_QOQ|.*_MOM)$"
)


def eastmoney_field_type(field_name: str) -> pa.DataType:
    """根据字段名推断 PyArrow 类型。

    基于 OpenAPI 示例数据验证：
    - 日期字段格式: 'YYYY-MM-DD 00:00:00' → pa.date32()
    - 布尔字段格式: '0'/'1' 字符串 → pa.bool_()
    - 数值字段格式: 浮点数字符串 → pa.float64()
    - 文本字段格式: 自由文本 → pa.string()
    """
    upper_name = field_name.upper()

    # 排除 *_DATE_NAME（文本名称如 "2026一季报"）
    if _DATE_NAME_PATTERN.match(upper_name):
        return pa.string()

    # 日期字段（排除 *_DATE_NAME 后）
    # API 返回 'YYYY-MM-DD 00:00:00' 格式，时间部分始终为 00:00:00
    if _DATE_PATTERN.match(upper_name):
        return pa.date32()

    # 布尔字段（IS_*, HAS_*）
    # API 返回 '0'/'1' 字符串
    if _BOOL_PATTERN.match(upper_name):
        return pa.bool_()

    # 数值字段（金额、比率、同比、环比）
    if _NUMERIC_PATTERN.match(upper_name):
        return pa.float64()

    # 默认：字符串
    return pa.string()


def eastmoney_typed_schema(endpoint: EastmoneyEndpointConfig) -> pa.Schema:
    """为 EastMoney 端点生成带类型的 schema。"""
    field_names = eastmoney_business_field_names(endpoint.asset_name)
    return pa.schema([
        pa.field(field_name, eastmoney_field_type(field_name))
        for field_name in field_names
    ])


# 类型转换函数映射
EASTMONEY_CONVERTERS: dict[pa.DataType, callable] = {
    pa.date32(): to_date32,
    pa.float64(): to_float64,
    pa.bool_(): to_bool,
    pa.string(): to_string,
}


def eastmoney_rows_to_typed_table(
    endpoint: EastmoneyEndpointConfig,
    rows: Sequence[EastmoneyFetchedRow],
) -> EastmoneyTableResult:
    """将 EastMoney 行数据转换为带类型的 pa.Table。"""
    schema = eastmoney_typed_schema(endpoint)
    business_field_names = eastmoney_business_field_names(endpoint.asset_name)

    columns: dict[str, list[Any]] = {field.name: [] for field in schema}
    unknown_field_count = 0

    for row in rows:
        unknown_field_count += unknown_field_count_for_mapping(
            row.data,
            allowed_top_level=business_field_names,
        )
        for field in schema:
            converter = EASTMONEY_CONVERTERS.get(field.type, to_string)
            columns[field.name].append(converter(row.data.get(field.name)))

    return EastmoneyTableResult(
        table=pa.table(columns, schema=schema),
        unknown_field_count=unknown_field_count,
    )
```

#### 3.2 修改 `eastmoney_schema()` 和 `eastmoney_rows_to_table()`

保持向后兼容，同时支持新旧两种模式：

```python
def eastmoney_schema(endpoint: EastmoneyEndpointConfig) -> pa.Schema:
    """返回带类型的 schema（新版本）。"""
    return eastmoney_typed_schema(endpoint)


def eastmoney_rows_to_table(
    endpoint: EastmoneyEndpointConfig,
    rows: Sequence[EastmoneyFetchedRow],
) -> EastmoneyTableResult:
    """将 EastMoney 行数据转换为带类型的 pa.Table。"""
    return eastmoney_rows_to_typed_table(endpoint, rows)
```

#### 3.3 修改 `empty_eastmoney_table()`

```python
def empty_eastmoney_table(endpoint: EastmoneyEndpointConfig) -> pa.Table:
    schema = eastmoney_typed_schema(endpoint)
    return pa.table({field.name: [] for field in schema}, schema=schema)
```

### 阶段 4：HTTP Schema 改造（THS、JiuYan）

**目标：** 将 HTTP 数据源的 schema 从全 string 改为带类型。

#### 4.1 定义 THS Limit Up Pool 带类型 schema

```python
# http/schemas.py
from datetime import datetime, timezone

# THS 时间字段格式：Unix 时间戳（秒），如 '1776130842'
# 转为 pa.timestamp() 以便 ClickHouse 直接使用时间函数
# 使用 UTC 作为系统内部时间存储标准，下游查询时按需转为本地时间

THS_LIMIT_UP_POOL_SCHEMA = pa.schema([
    pa.field("date", pa.date32()),                              # '20260508' → date32
    pa.field("open_num", pa.int64()),                           # 开板次数
    pa.field("first_limit_up_time", pa.timestamp("ns", tz="UTC")),  # Unix 时间戳 → UTC
    pa.field("last_limit_up_time", pa.timestamp("ns", tz="UTC")),   # Unix 时间戳 → UTC
    pa.field("code", pa.string()),                              # 股票代码
    pa.field("limit_up_type", pa.string()),                     # 涨停类型
    pa.field("order_volume", pa.float64()),                     # 封单量（手），可能有小数
    pa.field("is_new", pa.bool_()),                             # 是否新股
    pa.field("limit_up_suc_rate", pa.float64()),                # 涨停成功率
    pa.field("currency_value", pa.float64()),                   # 流通市值（元）
    pa.field("market_id", pa.int64()),                          # 市场 ID
    pa.field("is_again_limit", pa.bool_()),                     # 是否回封
    pa.field("change_rate", pa.float64()),                      # 涨跌幅 (%)
    pa.field("turnover_rate", pa.float64()),                    # 换手率 (%)
    pa.field("reason_type", pa.string()),                       # 涨停原因标签
    pa.field("order_amount", pa.float64()),                     # 封单金额（元）
    pa.field("high_days", pa.string()),                         # 连板天数描述（"首板"）
    pa.field("name", pa.string()),                              # 股票名称
    pa.field("high_days_value", pa.int64()),                    # 连板天数数值
    pa.field("change_tag", pa.string()),                        # 变动标签
    pa.field("market_type", pa.string()),                       # 市场类型
    pa.field("latest", pa.float64()),                           # 最新价
])
```
```

#### 4.2 定义 JiuYan Action Field 带类型 schema

```python
JIUYAN_ACTION_FIELD_SCHEMA = pa.schema([
    pa.field("action_field_id", pa.string()),
    pa.field("name", pa.string()),
    pa.field("date", pa.date32()),
    pa.field("reason", pa.string()),
    pa.field("sort_no", pa.int64()),
    pa.field("is_delete", pa.bool_()),
    pa.field("delete_time", pa.string()),
    pa.field("create_time", pa.string()),
    pa.field("update_time", pa.string()),
    pa.field("count", pa.int64()),
    pa.field("code", pa.string()),
    pa.field("time", pa.string()),
    pa.field("num", pa.string()),
    pa.field("price", pa.float64()),
    pa.field("day", pa.string()),
    pa.field("edition", pa.string()),
    pa.field("shares_range", pa.string()),
    pa.field("expound", pa.string()),
])
```

#### 4.3 定义 JiuYan Industry List 带类型 schema

```python
JIUYAN_INDUSTRY_LIST_SCHEMA = pa.schema([
    pa.field("industry_id", pa.string()),
    pa.field("title_red", pa.string()),
    pa.field("title_bold", pa.string()),
    pa.field("title", pa.string()),
    pa.field("author", pa.string()),
    pa.field("imgs", pa.string()),
    pa.field("keyword", pa.string()),
    pa.field("content", pa.string()),
    pa.field("is_top", pa.bool_()),
    pa.field("status", pa.string()),
    pa.field("sort_no", pa.int64()),
    pa.field("forward_count", pa.int64()),
    pa.field("browsers_count", pa.int64()),
    pa.field("is_delete", pa.bool_()),
    pa.field("delete_time", pa.string()),
    pa.field("create_time", pa.string()),
    pa.field("update_time", pa.string()),
])
```

#### 4.4 修改 `rows_to_string_table()` → `rows_to_typed_table()`

新增类型感知版本：

```python
def rows_to_typed_table(
    rows: Sequence[Mapping[str, object]],
    schema: pa.Schema,
) -> pa.Table:
    """从行数据和 schema 构建 pa.Table，自动应用类型转换。"""
    from scheduler.defs.common.schema import typed_table
    return typed_table(rows, schema)


# 保持向后兼容
def rows_to_string_table(
    rows: Sequence[Mapping[str, object]],
    columns: Sequence[str],
) -> pa.Table:
    """原有全 string 版本，保持向后兼容。"""
    arrays = {
        column: pa.array(
            [string_or_null(row.get(column)) for row in rows],
            type=pa.string(),
        )
        for column in columns
    }
    return pa.table(arrays, schema=string_schema(columns))
```

#### 4.5 修改 `ths_limit_up_pool_to_table()`

```python
def ths_limit_up_pool_to_table(
    pages: Sequence[Mapping[str, object]],
) -> TableConversionResult:
    rows: list[dict[str, object]] = []
    unknown_field_count = 0
    for page in pages:
        unknown_field_count += unknown_field_count_for_mapping(
            page,
            allowed_top_level=("date", "info"),
        )
        info = page.get("info")
        if not isinstance(info, list):
            continue

        for item in info:
            if not isinstance(item, Mapping):
                unknown_field_count += 1
                continue
            unknown_field_count += unknown_field_count_for_mapping(
                item,
                allowed_top_level=THS_LIMIT_UP_POOL_INFO_COLUMNS,
            )
            output_row = _blank_row(THS_LIMIT_UP_POOL_COLUMNS)
            output_row["date"] = page.get("date")
            copy_selected_fields(output_row, item, THS_LIMIT_UP_POOL_INFO_COLUMNS)
            rows.append(output_row)

    return TableConversionResult(
        table=rows_to_typed_table(rows, THS_LIMIT_UP_POOL_SCHEMA),
        unknown_field_count=unknown_field_count,
    )
```

#### 4.6 修改 `jiuyan_action_field_to_table()` 和 `jiuyan_industry_list_to_table()`

同理，将 `rows_to_string_table(rows, COLUMNS)` 替换为 `rows_to_typed_table(rows, SCHEMA)`。

### 阶段 5：JiuYan OCR Schema 改造

**目标：** JiuYan OCR 全为字符串列，无需类型改造，保持现状。

`jiuyan/ocr_schema.py` 中的 `JIUYAN_INDUSTRY_OCR_SCHEMA` 已经使用 `pa.field()` 声明式风格，所有列确实是字符串（`industry_id`, `stock_name`, `theme_path`, `relation`, `source`），无需改动。

### 阶段 6：测试更新

**目标：** 更新测试夹具和断言以反映新 schema 类型。

#### 6.1 需要更新的测试文件

| 测试文件 | 改造内容 |
|---------|---------|
| `tests/unit/baostock/test_baostock.py` | 更新 schema 断言、测试数据类型 |
| `tests/unit/sources/eastmoney/test_eastmoney.py` | 更新 schema 断言、字段类型断言 |
| `tests/unit/http/test_market_event_partitioning_and_schemas.py` | 更新 THS/JiuYan schema 断言 |
| `tests/unit/sources/jiuyan/test_ocr_schema.py` | 无需改动（全 string） |
| `tests/unit/storage/test_parquet_readers.py` | 更新 parquet 读取后的类型断言 |
| `tests/unit/compacted/test_action_field_compact.py` | 更新 compact 后的 schema 断言 |
| `tests/unit/compacted/test_limit_up_pool_compact.py` | 更新 compact 后的 schema 断言 |

#### 6.2 测试夹具更新示例

```python
# 原来
self.assertEqual(table.schema.field("open").type, pa.string())
self.assertEqual(table.column("open")[0].as_py(), "8.9400")

# 改造后
self.assertEqual(table.schema.field("open").type, pa.float64())
self.assertAlmostEqual(table.column("open")[0].as_py(), 8.94, places=4)
```

#### 6.3 新增类型转换测试

```python
def test_to_date32_handles_yyyy_mm_dd():
    from scheduler.defs.common.types import to_date32
    result = to_date32("2024-01-15")
    assert result == date(2024, 1, 15)

def test_to_date32_handles_yyyy_mm_dd_hh_mm_ss():
    """EastMoney 日期格式: '2026-03-31 00:00:00'"""
    from scheduler.defs.common.types import to_date32
    result = to_date32("2026-03-31 00:00:00")
    assert result == date(2026, 3, 31)

def test_to_date32_handles_yyyymmdd():
    """THS 日期格式: '20260508'"""
    from scheduler.defs.common.types import to_date32
    result = to_date32("20260508")
    assert result == date(2026, 5, 8)

def test_to_date32_handles_empty_string():
    """BaoStock outDate 空字符串"""
    from scheduler.defs.common.types import to_date32
    result = to_date32("")
    assert result is None

def test_to_float64_handles_string():
    from scheduler.defs.common.types import to_float64
    result = to_float64("19471149555.9")
    assert result == 19471149555.9

def test_to_float64_handles_empty_string():
    from scheduler.defs.common.types import to_float64
    result = to_float64("")
    assert result is None

def test_to_int8_handles_flag():
    from scheduler.defs.common.types import to_int8
    assert to_int8("1") == 1
    assert to_int8("0") == 0
    assert to_int8("999") is None  # 超出 int8 范围

def test_to_timestamp_handles_ths_time():
    """THS 时间字段格式: '1776130842' (Unix 时间戳) → UTC"""
    from scheduler.defs.common.types import to_timestamp
    result = to_timestamp("1776130842")
    assert result is not None
    assert result.tzinfo is not None
    assert result.year == 2026
    assert result.month == 4
    assert result.day == 14
    assert result.hour == 2  # UTC 时间

def test_to_bool_handles_ths_flag():
    """THS 布尔字段格式: 0/1 数字"""
    from scheduler.defs.common.types import to_bool
    assert to_bool(1) is True
    assert to_bool(0) is False
    assert to_bool("1") is True
    assert to_bool("0") is False
```

## 实施顺序

### 步骤 1：通用基础设施（低风险）

1. 创建 `common/types.py`，实现 `to_date32()`, `to_float64()`, `to_int64()`, `to_int8()`, `to_bool()`, `to_string()`。
2. 创建 `common/schema.py`，实现 `typed_schema()`, `typed_table()`, `_default_converter()`。
3. 编写单元测试覆盖所有转换函数。

### 步骤 2：BaoStock 改造（低风险，独立模块）

1. 在 `baostock/schemas.py` 中新增 `STOCK_BASIC_SCHEMA` 和 `K_HISTORY_DAILY_SCHEMA`。
2. 修改 `response_to_table()` 使用新 schema。
3. 修改 `stock_basic_response_to_table()` 和 `k_history_daily_response_to_table()`。
4. 修改 `baostock/assets.py` 中的 `empty_k_history_table()`。
5. 更新 `tests/unit/baostock/test_baostock.py`。
6. 运行测试验证。

### 步骤 3：HTTP Schema 改造（中风险，影响多个数据源）

1. 在 `http/schemas.py` 中新增 `THS_LIMIT_UP_POOL_SCHEMA`, `JIUYAN_ACTION_FIELD_SCHEMA`, `JIUYAN_INDUSTRY_LIST_SCHEMA`。
2. 新增 `rows_to_typed_table()` 函数。
3. 修改 `ths_limit_up_pool_to_table()` 使用新 schema。
4. 修改 `jiuyan_action_field_to_table()` 使用新 schema。
5. 修改 `jiuyan_industry_list_to_table()` 使用新 schema。
6. 更新 `tests/unit/http/test_market_event_partitioning_and_schemas.py`。
7. 运行测试验证。

### 步骤 4：EastMoney 改造（高风险，字段最多）

1. 在 `eastmoney/schema.py` 中新增 `eastmoney_field_type()` 和 `eastmoney_typed_schema()`。
2. 新增 `eastmoney_rows_to_typed_table()` 函数。
3. 修改 `eastmoney_schema()` 和 `eastmoney_rows_to_table()`。
4. 修改 `empty_eastmoney_table()`。
5. 更新 `tests/unit/sources/eastmoney/test_eastmoney.py`。
6. 运行测试验证。

### 步骤 5：Compact 层适配（中风险，依赖上游 schema）

1. 更新 `tests/unit/compacted/test_action_field_compact.py`。
2. 更新 `tests/unit/compacted/test_limit_up_pool_compact.py`。
3. 验证 compact 资产的 schema 与上游一致。

### 步骤 6：Parquet 读取适配（低风险）

1. 更新 `tests/unit/storage/test_parquet_readers.py`。
2. 验证 parquet 写入 → 读取的类型 round-trip。

### 步骤 7：集成测试（低风险）

1. 运行 `tests/integration/test_definitions_and_schedules.py`。
2. 验证所有 Dagster definitions 加载正常。
3. 运行 `dg check defs` 验证。

## 验收标准

1. 所有数据源的 schema 使用真实 PyArrow 类型（`date32`, `float64`, `int64`, `int8`, `bool_`, `string`）。
2. S3 parquet 文件的列类型与 schema 一致。
3. 所有现有测试通过（可能需要更新测试夹具）。
4. 新增类型转换函数的单元测试覆盖。
5. `dg check defs` 通过。
6. S3 路径、asset key、数据语义保持不变。
7. parquet 文件压缩率可测量提升（数值列从 string 改为原生类型）。

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 类型转换失败 | 数据丢失或 null 增多 | 转换函数返回 None 而非抛异常，记录警告 |
| EastMoney 字段类型推断错误 | 日期列被识别为数值 | 通过字段名模式验证，必要时手动覆盖 |
| 测试夹具不兼容 | 测试失败 | 分步骤更新测试，每步验证 |
| `pa.concat_tables` 类型不匹配 | compact 资产写入失败 | 确保上游和 compact 使用相同 schema |
| 空字符串无法转为 date | 日期列出现 null | 空字符串转为 None，与现有 null 语义一致 |
| BaoStock `outDate` 空字符串 | 无法转为 date32 | 空字符串转为 None（未退市的股票 outDate 为空） |

## 向后兼容性

| 维度 | 兼容性 | 说明 |
|------|--------|------|
| S3 路径 | ✅ 兼容 | 路径结构不变 |
| Dagster asset key | ✅ 兼容 | asset key 不变 |
| 数据内容 | ✅ 兼容 | 数据值不变，只改变存储类型 |
| Parquet 文件 | ❌ 不兼容 | 列类型从 string 改为原生类型 |
| 现有测试 | ❌ 不兼容 | 需要更新测试夹具 |

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `common/types.py` | 新增 | 类型转换函数：`to_date32`, `to_float64`, `to_int64`, `to_int8`, `to_bool`, `to_string`, `to_timestamp` |
| `common/schema.py` | 新增 | Schema 构建工具：`typed_schema`, `typed_table`, `_default_converter` |
| `baostock/schemas.py` | 修改 | 新增 `STOCK_BASIC_SCHEMA`, `K_HISTORY_DAILY_SCHEMA`，修改转换函数 |
| `baostock/assets.py` | 修改 | 修改 `empty_k_history_table()` 使用新 schema |
| `http/schemas.py` | 修改 | 新增 `THS_LIMIT_UP_POOL_SCHEMA`, `JIUYAN_ACTION_FIELD_SCHEMA`, `JIUYAN_INDUSTRY_LIST_SCHEMA`，新增 `rows_to_typed_table()` |
| `eastmoney/schema.py` | 修改 | 新增 `eastmoney_field_type()`, `eastmoney_typed_schema()`, `eastmoney_rows_to_typed_table()` |
| `tests/unit/baostock/test_baostock.py` | 修改 | 更新 schema 断言 |
| `tests/unit/sources/eastmoney/test_eastmoney.py` | 修改 | 更新 schema 断言 |
| `tests/unit/http/test_market_event_partitioning_and_schemas.py` | 修改 | 更新 schema 断言 |
| `tests/unit/compacted/test_action_field_compact.py` | 修改 | 更新 schema 断言 |
| `tests/unit/compacted/test_limit_up_pool_compact.py` | 修改 | 更新 schema 断言 |
| `tests/unit/storage/test_parquet_readers.py` | 修改 | 更新类型断言 |
| `tests/unit/common/test_types.py` | 新增 | 类型转换函数测试 |
