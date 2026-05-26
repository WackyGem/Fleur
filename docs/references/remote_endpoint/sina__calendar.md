# Sina Calendar API

## Endpoint

```
GET https://finance.sina.com.cn/realstock/company/klc_td_sh.txt
```

## Response Format

返回一个 JavaScript 变量声明，包含 Base64 编码的交易日历数据：

```txt
var datelist="LC/AAApNDXCw6mHbaPgkryxXv10eAJP1LW0SD39aT7+NV44Xba3PxCgTdrp5BkYVAc11hWvg0c/19UAc7jNtHQyWBAu2xmGuZI1NVAc3FepphjnTBw1X4hmGu+ypVAcvFenpBXPqCc6F4ZmGueLFwbIN8QTDXPsCc1FepphjvOoCc8FepphjvcgFO3CP00wxXXWhrkUdZrIJpw9X3ThrlEp6hlGc88Kcem0VeFpZM46VV4MrTC2KScKc811U4aLXUdlzINc9lTrwFW3T52KPj0mDueVFuUR1RtiEoCXfdgFOOSGRXnUhrXWhb0kt6Rk2pU44JV4SrTyU9wSDHPwCnXdP1FuiUM44r7qwdKqcYrIZpw1DqgrlU5IrHRawxjrwBaqcbrIt9gr3UhDtOpyVNjEnCHPnC3royNWvi0gjHXBXYdRlLbFpdJFueSFcqkK30sSDO+68K46IVOwVkaBX/";var KLC_TD_SH=datelist;
```

## 解码算法

### 解码入口

```python
"""新浪财经交易日历解析器"""
import re
from datetime import datetime, timedelta
from ..types import Parser, CrawlerResult


BASE64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"


class SinaCalendarParser(Parser):
    """新浪财经交易日历解析器"""

    def parse(self, content: str) -> CrawlerResult:
        """
        解析新浪交易日历 TXT 响应

        Args:
            content: TXT 内容

        Returns:
            CrawlerResult: 解析结果（不包含 title）
        """
        # 提取编码数据
        match = re.search(r'var datelist="([^"]+)"', content)
        if not match:
            return CrawlerResult(result=[])

        encoded_data = match.group(1)

        # 解码交易日期
        dates = self._decode_sina_data(encoded_data)

        # 补充缺失日期
        missing_date = datetime(1992, 5, 4).date()
        if missing_date not in dates:
            dates.append(missing_date)
            dates.sort()

        # 转换为字符串格式
        result = [[date.strftime("%Y-%m-%d")] for date in dates]

        return CrawlerResult(result=result)
```

### 核心解码逻辑

```python
    def _decode_sina_data(self, encoded_data: str) -> list:
        """解码新浪股票数据"""
        data_length = len(encoded_data)
        data_bits = [BASE64_CHARS.index(char) for char in encoded_data]

        state = {'l': 0, 'd': 0}
        data_index = 0
        bit_offset = 0

        # 获取校验和
        header, data_index, bit_offset = self._decode_bits(
            data_bits, data_length, state, data_index, bit_offset, [12, 6]
        )
        checksum = header[1] ^ 63

        if checksum > 1:
            return []

        # 解码日期数据
        count = -1
        result1, data_index, bit_offset = self._decode_bits(
            data_bits, data_length, state, data_index, bit_offset, [18]
        )
        state['d'] = result1[0] - 1

        result2, data_index, bit_offset = self._decode_bits(
            data_bits, data_length, state, data_index, bit_offset, [18]
        )
        end_index = result2[0]

        dates = []

        while state['d'] < end_index:
            date = self._calculate_date(state, 1)
            if count <= 0:
                data_index, bit_offset, bit = self._get_next_bit(
                    data_bits, data_length, data_index, bit_offset
                )
                if bit:
                    sign_value, data_index, bit_offset = self._decode_sign(
                        data_bits, data_length, data_index, bit_offset
                    )
                    state['l'] += sign_value

                bits_result, data_index, bit_offset = self._decode_bits(
                    data_bits, data_length, state, data_index, bit_offset,
                    [state['l'] * 3], [0]
                )
                count = bits_result[0] + 1

                if len(dates) == 0:
                    dates.append(date)
                    count -= 1
            else:
                dates.append(date)
            count -= 1

        return dates
```

### 辅助方法

```python
    def _calculate_date(self, state: dict, offset: int):
        """计算日期偏移"""
        for _ in range(offset):
            state['d'] += 1
            remainder = state['d'] % 7
            if remainder == 3 or remainder == 4:
                state['d'] += 5 - remainder

        date = datetime.fromtimestamp(0)
        date += timedelta(milliseconds=(7657 + state['d']) * 86400000)
        return date.date()

    def _decode_sign(self, data_bits: list, data_length: int, data_index: int, bit_offset: int):
        """解码符号位"""
        data_index, bit_offset, bit = self._get_next_bit(
            data_bits, data_length, data_index, bit_offset
        )
        count = 1

        while True:
            data_index, bit_offset, next_bit = self._get_next_bit(
                data_bits, data_length, data_index, bit_offset
            )
            if not next_bit:
                return count * (bit * 2 - 1), data_index, bit_offset
            count += 1

    def _get_next_bit(self, data_bits: list, data_length: int, data_index: int, bit_offset: int):
        """获取下一个比特位"""
        if data_index >= data_length:
            return data_index, bit_offset, 0

        bit = data_bits[data_index] & (1 << bit_offset)
        bit_offset += 1

        if bit_offset >= 6:
            bit_offset -= 6
            data_index += 1

        return data_index, bit_offset, bit != 0

    def _decode_short_bits(self, data_bits: list, data_index: int, bit_offset: int, length: int):
        """解码短位数据（长度<=30位）"""
        value = 0
        remaining_bits = length

        while remaining_bits > 0:
            available_bits = 6 - bit_offset
            bits_to_take = min(remaining_bits, available_bits)

            mask = (1 << bits_to_take) - 1
            shifted_data = data_bits[data_index] >> bit_offset
            masked_data = shifted_data & mask
            value |= masked_data << (length - remaining_bits)

            bit_offset += bits_to_take
            if bit_offset >= 6:
                bit_offset -= 6
                data_index += 1
            remaining_bits -= bits_to_take

        return value, data_index, bit_offset

    def _decode_bits(self, data_bits: list, data_length: int, state: dict,
                     data_index: int, bit_offset: int, lengths: list,
                     signed_flags: list = None, special_flags: list = None):
        """解码指定长度的比特数据"""
        if signed_flags is None:
            signed_flags = []
        if special_flags is None:
            special_flags = []

        result = []

        for i, length in enumerate(lengths):
            if not length:
                result.append(0)
                continue

            if data_index >= data_length:
                return result, data_index, bit_offset

            value = 0

            if length <= 0:
                value = 0
            elif length <= 30:
                value, data_index, bit_offset = self._decode_short_bits(
                    data_bits, data_index, bit_offset, length
                )

                if i < len(signed_flags) and signed_flags[i] and value >= (1 << (length - 1)):
                    value -= (1 << length)
            else:
                parts, data_index, bit_offset = self._decode_bits(
                    data_bits, data_length, state, data_index, bit_offset,
                    [30, length - 30],
                    [0, signed_flags[i] if i < len(signed_flags) else 0]
                )

                if i >= len(special_flags) or not special_flags[i]:
                    value = parts[0] + parts[1] * (1 << 30)

            result.append(value)

        return result, data_index, bit_offset
```
