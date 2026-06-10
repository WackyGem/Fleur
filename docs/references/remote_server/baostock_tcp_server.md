# BaoStock TCP 服务器对接规范

> 本文档描述 BaoStock TCP 服务器的通讯协议格式、状态码定义及 API 接口规范。

---

## 目录

- [通讯协议格式](#通讯协议格式)
  - [请求协议](#请求协议格式)
  - [响应协议](#响应协议格式)
  - [编解码要求](#编解码要求)
- [状态码定义](#状态码定义)
- [分页机制](#分页机制)
- [API 接口定义](#api-接口定义)

---

## 通讯协议格式

### 请求协议格式

以 `query_stock_basic` 请求为例：

```
b'00.9.10\x0145\x010000000037query_stock_basic\x01anonymous\x011\x0110000\x01sh.601088\x01\x011304847754\n'
```

| 字段 | 示例值 | 说明 |
|:-----|:-------|:-----|
| A | `00.9.10` | 客户端版本号 |
| B | `45` | 请求接口编码 |
| C | `0000000037` | 10 位消息体长度（从 D 开始计算） |
| D | `query_stock_basic` | 接口名称 |
| E | `anonymous` | 用户 ID |
| F | `1` | 分页参数 - 当前页码 |
| G | `10000` | 分页参数 - 每页条数 |
| params | `sh.601088` | 参数列表（`\x01` 分隔） |
| CRC | `1304847754` | `zlib.crc32` 校验码 |
| 结束符 | `\n` | 换行符 |

> ⚠️ **注意**：字段间使用 `\x01` 分隔，C 和 D 之间无分隔符。

---

### 响应协议格式

```
b'00.9.00\x0146\x0100000001620\x01success\x01query_stock_basic\x01anonymous\x011\x0110000\x01{"record":[...]}\x01sh.601088\x01\x01code, code_name, ...\x013171182943<![CDATA[]]>\n'
```

| 字段 | 示例值 | 说明 |
|:-----|:-------|:-----|
| A | `00.9.00` | 服务端版本号 |
| B | `46` | 响应接口编码 |
| C | `0000000162` | 10 位消息体长度 |
| D | `0` | 响应状态码（0 = 成功） |
| E | `success` | 响应状态描述 |
| F | `query_stock_basic` | 接口名称 |
| G | `anonymous` | 用户 ID |
| H | `1` | 当前页码 |
| I | `10000` | 每页条数 |
| J | `{"record":[...]}` | 查询结果集（JSON 格式） |
| params | `sh.601088` | 参数列表 |
| K | `code, code_name, ...` | 结果集列字段名称 |
| CRC | `3171182943` | `zlib.crc32` 校验码 |
| 结束符 | `<![CDATA[]]>\n` | 结束标记 |

---

### 编解码要求

| 功能 | 说明 |
|:-----|:-----|
| 消息体长度计算 | 自动计算并填充 10 位长度字段 |
| CRC 校验 | 使用 `zlib.crc32` 生成校验码 |
| 分页处理 | 自动处理分页参数，特殊处理登录/登出接口 |
| 数据解压 | 支持自动解压压缩数据（响应编码 96） |

---

## 状态码定义

### 成功状态码

| 状态码 | 描述 |
|:-------|:-----|
| `0` | 正确返回值 |

### 用户相关错误 (10001xxx)

| 错误码 | 描述 |
|:-------|:-----|
| `10001001` | 用户未登陆 |
| `10001002` | 用户名或密码错误 |
| `10001003` | 获取用户信息失败 |
| `10001004` | 客户端版本号过期 |
| `10001005` | 账号登陆数达到上限 |
| `10001006` | 用户权限不足 |
| `10001007` | 需要登录激活 |
| `10001008` | 用户名为空 |
| `10001009` | 密码为空 |
| `10001010` | 用户登出失败 |
| `10001011` | 黑名单用户 |

### 网络相关错误 (10002xxx)

| 错误码 | 描述 |
|:-------|:-----|
| `10002001` | 网络错误 |
| `10002002` | 网络连接失败 |
| `10002003` | 网络连接超时 |
| `10002004` | 网络接收时连接断开 |
| `10002005` | 网络发送失败 |
| `10002006` | 网络发送超时 |
| `10002007` | 网络接收错误 |
| `10002008` | 网络接收超时 |

### 数据相关错误 (10004xxx)

| 错误码 | 描述 |
|:-------|:-----|
| `10004001` | 解析数据错误 |
| `10004002` | gzip 解压失败 |
| `10004003` | 客户端未知错误 |
| `10004004` | 数组越界 |
| `10004005` | 传入参数为空 |
| `10004006` | 参数错误 |
| `10004007` | 起始日期格式不正确 |
| `10004008` | 截止日期格式不正确 |
| `10004009` | 起始日期大于终止日期 |
| `10004010` | 日期格式不正确 |
| `10004011` | 无效的证券代码 |
| `10004012` | 无效的指标 |
| `10004013` | 超出日期支持范围 |
| `10004014` | 不支持的混合证券品种 |
| `10004015` | 不支持的证券代码品种 |
| `10004016` | 交易条数超过上限 |
| `10004017` | 不支持的交易信息 |
| `10004018` | 指标重复 |
| `10004019` | 消息格式不正确 |
| `10004020` | 错误的消息类型 |

### 系统错误 (10005xxx)

| 错误码 | 描述 |
|:-------|:-----|
| `10005001` | 系统级别错误 |

---

## API 接口定义

### 基础配置

| 配置项 | 值 |
|:-------|:---|
| 客户端版本号 | `00.9.10` |
| 服务端版本号 | `00.9.00` |
| 用户 ID | `anonymous` |
| 服务端地址 | `public-api.baostock.com:10030` |

---

### 认证接口

#### login - 登录

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `00` |
| 响应编码 | `01` |
| 接口名称 | `login` |

**参数：**

| 参数 | 默认值 | 说明 |
|:-----|:-------|:-----|
| password | `123456` | 密码，非空 |
| option | `0` | 选项，非空；若通过 `set_API_key` 设置了 API Key，则发送 API Key |

#### set_API_key - 设置 API Key

新增于 v0.9.1。设置后，login 请求中的 `option` 字段会替换为该 API Key。

| 参数 | 默认值 | 说明 |
|:-----|:-------|:-----|
| apiKey | `''` | API Key 字符串 |

#### logout - 登出

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `02` |
| 响应编码 | `03` |
| 接口名称 | `logout` |

**参数：**

| 参数 | 格式 | 说明 |
|:-----|:-----|:-----|
| now_time | `%Y%m%d%H%M%S` | 当前时间，非空 |

---

### 行情数据接口

#### query_history_k_data_plus - 获取历史 A 股 K 线数据

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `95` |
| 响应编码 | `96` |
| 数据范围 | 1990-12-19 至今 |

**参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| code | ✅ | 证券代码，如 `sh.601398` |
| fields | ✅ | 字段列表（见下表） |
| start_date | ✅ | 开始日期，`YYYY-MM-DD` |
| end_date | ✅ | 结束日期，`YYYY-MM-DD` |
| frequency | ✅ | 数据类型：`d` / `w` / `m` / `5` / `15` / `30` / `60` |
| adjustflag | ✅ | 复权类型：`1` 后复权 / `2` 前复权 / `3` 不复权 |

**fields 字段说明：**

| 类型 | 可用字段 |
|:-----|:---------|
| 日线 | `date`, `code`, `open`, `high`, `low`, `close`, `preclose`, `volume`, `amount`, `adjustflag`, `turn`, `tradestatus`, `pctChg`, `isST` |
| 分钟线 | `date`, `time`, `code`, `open`, `high`, `low`, `close`, `volume`, `amount`, `adjustflag` |
| 周月线 | `date`, `code`, `open`, `high`, `low`, `close`, `volume`, `amount`, `adjustflag`, `turn`, `pctChg` |

---

### 证券信息接口

#### query_stock_basic - 证券基本资料

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `45` |
| 响应编码 | `46` |

**参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| code | ❌ | 证券代码，如 `sh.600000`；与 `code_name` 二选一或同时为空 |
| code_name | ❌ | 证券名称，支持按名称模糊查询，如 `浦发银行` |
| page | ❌ | 分页页码，默认 `1` |
| page_size | ❌ | 分页大小，默认 `10000` |

**说明：**

- `code` 与 `code_name` 都为空时，返回全部证券基础资料。
- TCP 协议报文中，`code` 与 `code_name` 作为参数列表按顺序传输；分页参数位于协议头固定位置。
- 响应结果字段由服务端返回的 `field_names` 决定，常见字段包括 `code`、`code_name`、`ipoDate`、`outDate`、`type`、`status`。

**返回字段：**

| 字段 | 说明 |
|:-----|:-----|
| `code` | 证券代码 |
| `code_name` | 证券名称 |
| `ipoDate` | 上市日期 |
| `outDate` | 退市日期 |
| `type` | 证券类型：`1` 股票，`2` 指数，`3` 其它，`4` 可转债，`5` ETF |
| `status` | 上市状态：`1` 上市，`0` 退市 |

#### query_stock_industry - 行业分类

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `59` |
| 响应编码 | `60` |

**参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| code | ❌ | 证券代码，如 `sh.600000` |
| date | ❌ | 查询日期，`YYYY-MM-DD` |

---

### 交易日与全量证券接口

#### query_trade_dates - 查询交易日信息

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `33` |
| 响应编码 | `34` |

**参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| start_date | ❌ | 起始日期，默认 `2015-01-01` |
| end_date | ❌ | 终止日期，默认当前日期 |

**返回字段：** `calendar_date`（日期）、`is_trading_day`（0=非交易日，1=交易日）

#### query_all_stock - 查询指定日期全部证券

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `35` |
| 响应编码 | `36` |

**参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| day | ❌ | 查询日期，默认当前日期 |

---

### 板块与成分股接口

| 接口名称 | 请求编码 | 响应编码 | 参数 | 说明 |
|:---------|:---------|:---------|:-----|:-----|
| `query_hs300_stocks` | `61` | `62` | `date` | 沪深300成分股 |
| `query_sz50_stocks` | `63` | `64` | `date` | 上证50成分股 |
| `query_zz500_stocks` | `65` | `66` | `date` | 中证500成分股 |
| `query_terminated_stocks` | `67` | `68` | `date` | 终止上市股票 |
| `query_suspended_stocks` | `69` | `70` | `date` | 暂停上市股票 |
| `query_st_stocks` | `71` | `72` | `date` | ST股票列表 |
| `query_starst_stocks` | `73` | `74` | `date` | *ST股票列表 |

**通用参数：** `date`（可选，`YYYY-MM-DD`，查询日期）

---

### 分类查询接口

| 接口名称 | 请求编码 | 响应编码 | 参数 | 说明 |
|:---------|:---------|:---------|:-----|:-----|
| `query_stock_concept` | `81` | `82` | `code`, `date` | 概念分类 |
| `query_stock_area` | `83` | `84` | `code`, `date` | 地域分类 |
| `query_ame_stocks` | `85` | `86` | `date` | 中小板分类 |
| `query_gem_stocks` | `87` | `88` | `date` | 创业板分类 |
| `query_shhk_stocks` | `89` | `90` | `date` | 沪港通 |
| `query_szhk_stocks` | `91` | `92` | `date` | 深港通 |
| `query_stocks_in_risk` | `93` | `94` | `date` | 风险警示板 |

**通用参数说明：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| code | ❌ | 证券代码（仅 `query_stock_concept`、`query_stock_area`） |
| date | ❌ | 查询日期，`YYYY-MM-DD` |

---

### 除权除息接口

#### query_dividend_data - 查询除权除息信息

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `13` |
| 响应编码 | `14` |

**参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| code | ✅ | 证券代码 |
| year | ✅ | 统计年份，默认当前年 |
| yearType | ✅ | `report` = 预案公告年份，`operate` = 除权除息年份 |

#### query_adjust_factor - 查询复权因子信息

| 属性 | 值 |
|:-----|:---|
| 请求编码 | `15` |
| 响应编码 | `16` |

**参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| code | ✅ | 证券代码 |
| start_date | ❌ | 起始日期，默认 `2015-01-01` |
| end_date | ❌ | 终止日期，默认当前时间 |

---

### 季频财务数据接口

> 📅 数据范围：2007 年至今

| 接口名称 | 请求编码 | 响应编码 | 说明 |
|:---------|:---------|:---------|:-----|
| `query_profit_data` | `17` | `18` | 季频盈利能力 |
| `query_operation_data` | `19` | `20` | 季频营运能力 |
| `query_growth_data` | `21` | `22` | 季频成长能力 |
| `query_dupont_data` | `23` | `24` | 季频杜邦指数 |
| `query_balance_data` | `25` | `26` | 季频偿债能力 |
| `query_cash_flow_data` | `27` | `28` | 季频现金流量 |

**通用参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| code | ✅ | 证券代码 |
| year | ✅ | 统计年份，默认当前年 |
| quarter | ✅ | 统计季度：`1` / `2` / `3` / `4` |

---

### 业绩报告接口

| 接口名称 | 请求编码 | 响应编码 | 数据范围 | 说明 |
|:---------|:---------|:---------|:---------|:-----|
| `query_performance_express_report` | `29` | `30` | 2006 年至今 | 季频业绩快报 |
| `query_forecast_report` | `31` | `32` | 2003 年至今 | 季频业绩预告 |

**通用参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| code | ✅ | 证券代码 |
| start_date | ❌ | 开始日期，默认 `2015-01-01` |
| end_date | ❌ | 结束日期，默认当前日期 |

---

### 其他查询接口

| 接口名称 | 请求编码 | 响应编码 | 说明 |
|:---------|:---------|:---------|:-----|
| `query_deposit_rate_data` | `47` | `48` | 存款利率 |
| `query_loan_rate_data` | `49` | `50` | 贷款利率 |
| `query_required_reserve_ratio_data` | `51` | `52` | 存款准备金率 |
| `query_money_supply_data_month` | `53` | `54` | 货币供应量（月度） |
| `query_money_supply_data_year` | `55` | `56` | 货币供应量（年底余额） |

**通用参数（存款/贷款利率、货币供应量）：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| start_date | ❌ | 起始日期；月度为 `yyyy-MM`，年度为 `yyyy`，其余为 `YYYY-MM-DD` |
| end_date | ❌ | 结束日期，格式同上 |

**`query_required_reserve_ratio_data` 额外参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| yearType | ❌ | `0` = 公告日期（默认），`1` = 生效日期 |

---

### 宏观经济指数接口

| 接口名称 | 请求编码 | 响应编码 | 说明 |
|:---------|:---------|:---------|:-----|
| `query_cpi_data` | `75` | `76` | 居民价格消费指数（CPI） |
| `query_ppi_data` | `77` | `78` | 工业品出厂价格指数（PPI） |
| `query_pmi_data` | `79` | `80` | 采购经理人指数（PMI） |

**通用参数：**

| 参数 | 必填 | 说明 |
|:-----|:-----|:-----|
| start_date | ❌ | 起始日期，`YYYY-MM-DD` |
| end_date | ❌ | 结束日期，`YYYY-MM-DD` |


### 协议实现参考
```python

"""
协议编解码模块
处理 baostock 协议的编码和解码
"""
import json
import zlib
from dataclasses import dataclass
from typing import List, Optional


# 协议常量
CLIENT_VERSION = "00.9.10"
SEPARATOR = "\x01"
BYTE_SEPARATOR = b'\x01'
END_MARKER = b'<![CDATA[]]>\n'
DEFAULT_PAGE_SIZE = 10000
NO_PAGING_APIS = frozenset(['login', 'logout'])


@dataclass
class DecodedResponse:
    """解码后的响应数据"""
    server_version: str
    response_code: str
    body_length: str
    error_code: str
    error_message: str
    api_name: str
    user_id: str
    page: int
    page_size: int
    records: List[List[str]]
    field_names: List[str]
    params: List[str]

    @property
    def record_count(self) -> int:
        return len(self.records)

    @property
    def has_next_page(self) -> bool:
        return self.record_count > 0 and self.record_count == self.page_size


def encode_request(
    request_code: str,
    api_name: str,
    user_id: str,
    params: List[str],
    page: int = 1,
    page_size: int = DEFAULT_PAGE_SIZE
) -> bytes:
    """编码请求消息"""
    # 构建消息体
    if api_name in NO_PAGING_APIS:
        parts = [api_name, user_id]
    else:
        parts = [api_name, user_id, str(page), str(page_size)]
    parts.extend(params)
    body = SEPARATOR.join(parts)

    # 构建完整消息
    body_length = str(len(body)).zfill(10)
    message = SEPARATOR.join([CLIENT_VERSION, request_code, body_length + body])
    message_bytes = message.encode('utf-8')

    # 添加 CRC 和结束符
    crc = str(zlib.crc32(message_bytes)).encode('utf-8')
    return message_bytes + BYTE_SEPARATOR + crc + b'\n'


def encode_next_page(resp: DecodedResponse) -> bytes:
    """编码下一页请求"""
    return encode_request(
        request_code=str(int(resp.response_code) - 1),
        api_name=resp.api_name,
        user_id=resp.user_id,
        params=resp.params,
        page=resp.page + 1,
        page_size=resp.page_size
    )


def decode_response(data: bytes) -> DecodedResponse:
    """解码响应消息"""
    # 移除结束标记
    if data.endswith(END_MARKER):
        data = data[:-len(END_MARKER)]

    # 解压（如果需要）
    data = _decompress_if_needed(data)

    # 解析字段
    parts = data.decode('utf-8').split(SEPARATOR)

    server_version = parts[0]
    response_code = parts[1]
    body_length = parts[2][:10]
    error_code = parts[2][10:]
    error_message = parts[3] if len(parts) > 3 else ''
    api_name = parts[4] if len(parts) > 4 else ''
    user_id = parts[5] if len(parts) > 5 else ''
    page = int(parts[6]) if len(parts) > 6 else 1
    page_size = int(parts[7]) if len(parts) > 7 else DEFAULT_PAGE_SIZE
    result_data = parts[8] if len(parts) > 8 else ''

    # 解析参数和字段名
    params, field_names = _parse_params_and_fields(parts[9:-1], response_code)

    # 解析记录
    records = _parse_records(result_data, field_names)

    return DecodedResponse(
        server_version=server_version,
        response_code=response_code,
        body_length=body_length,
        error_code=error_code,
        error_message=error_message,
        api_name=api_name,
        user_id=user_id,
        page=page,
        page_size=page_size,
        records=records,
        field_names=field_names,
        params=params,
    )


def _decompress_if_needed(data: bytes) -> bytes:
    """解压数据（响应编码 96）"""
    if b'\x0196\x01' not in data[7:11]:
        return data

    parts = data.split(BYTE_SEPARATOR, 2)
    server_version = parts[0]
    response_code = parts[1]
    length_and_body = parts[2]

    body_length = int(length_and_body[:10])
    compressed_body = length_and_body[10:10 + body_length]
    remaining = length_and_body[10 + body_length:]

    decompressed = zlib.decompress(compressed_body)
    new_length = str(len(decompressed)).zfill(10).encode('utf-8')

    return server_version + BYTE_SEPARATOR + response_code + BYTE_SEPARATOR + new_length + decompressed + remaining



def _parse_params_and_fields(parts: List[str], response_code: str) -> tuple[List[str], List[str]]:
    """解析参数和字段名"""
    params = []
    field_names = []

    for part in parts:
        if part and ',' in part:
            field_names = [s.strip() for s in part.split(',')]
            if response_code == '96':
                params.append(part)
        else:
            params.append(part)

    return params, field_names


def _parse_records(result_data: str, field_names: List[str]) -> List[List[str]]:
    """解析记录数据"""
    if not result_data or not result_data.startswith('{'):
        return []

    data = json.loads(result_data)
    records = data.get('record', [])

    # 转换 time 字段格式
    if field_names and 'time' in field_names:
        time_idx = field_names.index('time')
        for record in records:
            if len(record) > time_idx and record[time_idx]:
                record[time_idx] = _format_time(record[time_idx])

    return records


def _format_time(time_str: str) -> str:
    """将 YYYYMMDDHHMMSSsss 转换为 YYYY-MM-DD HH:MM:SS"""
    if not time_str or len(time_str) < 14:
        return time_str
    return f"{time_str[:4]}-{time_str[4:6]}-{time_str[6:8]} {time_str[8:10]}:{time_str[10:12]}:{time_str[12:14]}"

```

---

## 每日最新数据更新时间

| 触发条件 | 更新内容 |
|:---------|:---------|
| 当前交易日 17:30 | 完成日 K 线数据入库 |
| 当前交易日 18:00 | 完成复权因子数据入库 |
| 当前交易日 20:00 | 完成分钟 K 线数据入库 |
| 第二自然日 1:30 | 完成前交易日"其它财务报告数据"入库 |
| 周六 17:30 | 完成周 K 线数据入库 |
| 每月 1 号 17:30 | 完成上月月 K 线数据入库 |

**每周数据更新时间：**

每周一下午，完成上证50成份股、沪深300成份股、中证500成份股信息数据入库。

---

## 数据范围说明

### 股票数据

| 数据类型 | 时间范围 |
|:---------|:---------|
| 日、周、月 K 线数据 | 1990-12-19 至今 |
| 5、15、30、60 分钟 K 线数据 | 2020-01-03 至今（近 5 年） |

### ETF 数据

| 数据类型 | 时间范围 |
|:---------|:---------|
| 日、周、月 K 线数据 | 2026-01-05 至今 |
| 5、15、30、60 分钟 K 线数据 | 2026-01-05 至今 |

### 指数数据

日、周、月 K 线已包含指数（不提供分钟 K 线数据）：综合指数，规模指数，一级行业指数，二级行业指数，策略指数，成长指数，价值指数，主题指数，基金指数，债券指数。

时间范围：2006-01-01 至今。

### 季频财务数据

已包含的财务数据：部分上市公司资产负债信息、上市公司现金流量信息、上市公司利润信息、上市公司杜邦指标信息。

时间范围：2007 年至今。

### 季频公司报告

| 报告类型 | 时间范围 |
|:---------|:---------|
| 上市公司业绩预告信息 | 2003 年至今 |
| 上市公司业绩快报信息 | 2006 年至今 |
