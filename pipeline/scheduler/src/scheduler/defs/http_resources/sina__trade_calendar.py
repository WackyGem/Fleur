from __future__ import annotations

import re
import random
import time
from collections.abc import Callable
from dataclasses import dataclass, field
from datetime import date, timedelta

import dagster as dg
import pyarrow as pa
import requests

BASE64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
BASE64_INDEX = {char: index for index, char in enumerate(BASE64_CHARS)}
SINA_TRADE_CALENDAR_URL = "https://finance.sina.com.cn/realstock/company/klc_td_sh.txt"
CHROME_USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/125.0.0.0 Safari/537.36"
)
REQUEST_TIMEOUT_SECONDS = (5, 20)
MAX_REQUEST_ATTEMPTS = 4
UNIX_EPOCH_DATE = date(1970, 1, 1)
SINA_EPOCH_DAY_OFFSET = 7657
SINA_KNOWN_MISSING_DATE = date(1992, 5, 4)
# The original Sina decoder advances an integer day counter, then skips
# remainders 3 and 4. With 1970-01-01 as Thursday and the 7657-day Sina epoch
# offset, serial remainders map as 0=Wed, 1=Thu, 2=Fri, 3=Sat, 4=Sun,
# 5=Mon, 6=Tue. The +2 / +1 entries skip encoded weekends and land on Monday.
WEEKEND_SKIP_DAYS_BY_REMAINDER = (0, 0, 0, 2, 1, 0, 0)
DATELIST_PATTERN = re.compile(r'var datelist="([^"]+)"')

TradeCalendarDates = list[date]
RequestGet = Callable[..., requests.Response]
Sleep = Callable[[float], None]
RandomUniform = Callable[[float, float], float]


class SinaCalendarDecodeError(ValueError):
    """Raised when Sina's compact calendar payload cannot be decoded."""


@dataclass(frozen=True)
class ExponentialBackoffPolicy:
    """Configurable exponential backoff schedule for transient HTTP failures."""

    base_delay: float = 1.0
    factor: float = 2.0
    max_delay: float = 60.0
    jitter: bool = True
    jitter_ratio: float = 0.25
    random_uniform: RandomUniform = field(default=random.uniform, repr=False, compare=False)

    def delays(self, max_attempts: int) -> list[float]:
        if max_attempts < 1:
            msg = "max_attempts must be positive"
            raise ValueError(msg)

        delays = []
        for attempt in range(max_attempts - 1):
            delay = self.base_delay * (self.factor**attempt)
            delay = min(delay, self.max_delay)
            if self.jitter:
                delay = self.random_uniform(
                    delay * (1 - self.jitter_ratio),
                    delay * (1 + self.jitter_ratio),
                )
            delays.append(delay)
        return delays

    def metadata(self, max_attempts: int) -> dict[str, object]:
        return {
            "type": "exponential_backoff",
            "base_delay": self.base_delay,
            "factor": self.factor,
            "max_delay": self.max_delay,
            "jitter": self.jitter,
            "jitter_ratio": self.jitter_ratio,
            "max_attempts": max_attempts,
            "max_retries": max_attempts - 1,
            "nominal_delays": ExponentialBackoffPolicy(
                base_delay=self.base_delay,
                factor=self.factor,
                max_delay=self.max_delay,
                jitter=False,
            ).delays(max_attempts),
        }


DEFAULT_RETRY_POLICY = ExponentialBackoffPolicy(jitter=False)
TRADE_CALENDAR_AUTOMATION_CONDITION = (
    dg.AutomationCondition.on_missing()
    | (
        dg.AutomationCondition.initial_evaluation()
        & dg.AutomationCondition.missing()
        & ~dg.AutomationCondition.any_deps_missing()
        & ~dg.AutomationCondition.in_progress()
    ).with_label("on_initial_missing")
)


@dataclass
class SinaDecodeState:
    run_length_bits: int = 0
    serial_day_number: int = 0

    def next_trade_date(self) -> date:
        self.serial_day_number += 1
        self.serial_day_number += WEEKEND_SKIP_DAYS_BY_REMAINDER[
            self.serial_day_number % 7
        ]

        return UNIX_EPOCH_DATE + timedelta(
            days=SINA_EPOCH_DAY_OFFSET + self.serial_day_number
        )


@dataclass
class SinaBitReader:
    data_bits: list[int]
    data_index: int = 0
    bit_offset: int = 0

    @classmethod
    def from_encoded_data(cls, encoded_data: str) -> SinaBitReader:
        data_bits = []
        for char in encoded_data:
            if char not in BASE64_INDEX:
                msg = f"Unsupported Sina calendar character: {char!r}"
                raise SinaCalendarDecodeError(msg)
            data_bits.append(BASE64_INDEX[char])
        return cls(data_bits=data_bits)

    def is_exhausted(self) -> bool:
        return self.data_index >= len(self.data_bits)

    def read_bit(self) -> bool:
        if self.is_exhausted():
            raise SinaCalendarDecodeError("Unexpected end of Sina calendar bitstream")

        bit = self.data_bits[self.data_index] & (1 << self.bit_offset)
        self.bit_offset += 1
        if self.bit_offset >= 6:
            self.bit_offset = 0
            self.data_index += 1

        return bit != 0

    def read_signed_delta(self) -> int:
        is_positive = self.read_bit()
        magnitude = 1

        while self.read_bit():
            magnitude += 1

        if is_positive:
            return magnitude
        return -magnitude

    def read_values(
        self,
        lengths: tuple[int, ...],
        signed_flags: tuple[bool, ...] = (),
        force_zero_flags: tuple[bool, ...] = (),
    ) -> list[int]:
        """Read integer values from the stream.

        `force_zero_flags` mirrors Sina's decoder branch where some fields are
        intentionally decoded for cursor movement but returned as zero.
        """

        values: list[int] = []
        for index, length in enumerate(lengths):
            if length <= 0:
                values.append(0)
                continue
            if self.is_exhausted():
                raise SinaCalendarDecodeError("Unexpected end of Sina calendar bitstream")

            value = self._read_value(length, self._flag_at(signed_flags, index))
            if self._flag_at(force_zero_flags, index):
                value = 0
            values.append(value)

        return values

    def _read_value(self, length: int, is_signed: bool) -> int:
        if length <= 30:
            value = self._read_short_value(length)
            if is_signed and value >= (1 << (length - 1)):
                value -= 1 << length
            return value

        lower, upper = self.read_values((30, length - 30), (False, is_signed))
        return lower + upper * (1 << 30)

    def _read_short_value(self, length: int) -> int:
        value = 0
        remaining_bits = length

        while remaining_bits > 0:
            if self.is_exhausted():
                raise SinaCalendarDecodeError("Unexpected end of Sina calendar bitstream")

            available_bits = 6 - self.bit_offset
            bits_to_take = min(remaining_bits, available_bits)
            mask = (1 << bits_to_take) - 1
            shifted_data = self.data_bits[self.data_index] >> self.bit_offset
            value |= (shifted_data & mask) << (length - remaining_bits)

            self.bit_offset += bits_to_take
            if self.bit_offset >= 6:
                self.bit_offset = 0
                self.data_index += 1
            remaining_bits -= bits_to_take

        return value

    def _flag_at(self, flags: tuple[bool, ...], index: int) -> bool:
        return index < len(flags) and flags[index]


@dataclass(frozen=True)
class SinaCalendarParseResult:
    dates: TradeCalendarDates
    missing_date_added: bool = False
    error_message: str | None = None


class SinaCalendarParser:
    """Parse Sina's compact A-share trade-calendar payload."""

    def parse(self, content: str) -> TradeCalendarDates:
        return self.parse_with_diagnostics(content).dates

    def parse_with_diagnostics(self, content: str) -> SinaCalendarParseResult:
        match = DATELIST_PATTERN.search(content)
        if match is None:
            return SinaCalendarParseResult(
                dates=[],
                error_message="Sina calendar response did not contain var datelist",
            )

        try:
            dates = self._decode_sina_data(match.group(1))
        except SinaCalendarDecodeError as error:
            return SinaCalendarParseResult(dates=[], error_message=str(error))

        if not dates:
            return SinaCalendarParseResult(
                dates=[],
                error_message="Sina calendar decoded to an empty date list",
            )

        missing_date_added = False
        if SINA_KNOWN_MISSING_DATE not in dates:
            dates.append(SINA_KNOWN_MISSING_DATE)
            dates.sort()
            missing_date_added = True

        return SinaCalendarParseResult(
            dates=dates,
            missing_date_added=missing_date_added,
        )

    def _decode_sina_data(self, encoded_data: str) -> list[date]:
        reader = SinaBitReader.from_encoded_data(encoded_data)
        header = reader.read_values((12, 6))
        checksum = header[1] ^ 63
        if checksum > 1:
            raise SinaCalendarDecodeError("Sina calendar checksum validation failed")

        start_index = reader.read_values((18,))[0] - 1
        end_index = reader.read_values((18,))[0]
        state = SinaDecodeState(serial_day_number=start_index)
        dates: list[date] = []
        consecutive_trade_days_remaining = -1

        while state.serial_day_number < end_index:
            trade_date = state.next_trade_date()
            if consecutive_trade_days_remaining <= 0:
                consecutive_trade_days_remaining = self._read_next_run_length(reader, state)
                if not dates:
                    dates.append(trade_date)
                    consecutive_trade_days_remaining -= 1
            else:
                dates.append(trade_date)
            consecutive_trade_days_remaining -= 1

        return dates

    def _read_next_run_length(
        self,
        reader: SinaBitReader,
        state: SinaDecodeState,
    ) -> int:
        if reader.read_bit():
            state.run_length_bits += reader.read_signed_delta()

        if state.run_length_bits < 0:
            raise SinaCalendarDecodeError("Sina calendar run-length width became negative")

        encoded_run_length = reader.read_values((state.run_length_bits * 3,), (False,))[0]
        # Sina stores run length as length - 1, so add one after decoding.
        return encoded_run_length + 1


def fetch_sina_trade_calendar(
    request_get: RequestGet = requests.get,
    sleep: Sleep = time.sleep,
    retry_policy: ExponentialBackoffPolicy = DEFAULT_RETRY_POLICY,
    max_attempts: int = MAX_REQUEST_ATTEMPTS,
) -> str:
    headers = {
        "User-Agent": CHROME_USER_AGENT,
        "Accept": "text/plain,*/*",
    }

    last_error: requests.RequestException | None = None
    retry_delays = retry_policy.delays(max_attempts)
    for attempt_index in range(max_attempts):
        try:
            response = request_get(
                SINA_TRADE_CALENDAR_URL,
                headers=headers,
                timeout=REQUEST_TIMEOUT_SECONDS,
            )
            response.raise_for_status()
            return response.text
        except requests.RequestException as error:
            last_error = error
            if attempt_index >= len(retry_delays):
                break
            sleep(retry_delays[attempt_index])

    msg = f"Failed to fetch Sina trade calendar after {max_attempts} attempts"
    raise RuntimeError(msg) from last_error


def trade_calendar_dates_to_table(trade_dates: TradeCalendarDates) -> pa.Table:
    schema = pa.schema([pa.field("trade_date", pa.date32())])
    return pa.Table.from_arrays(
        [pa.array(trade_dates, type=pa.date32())],
        schema=schema,
    )


@dg.asset(
    group_name="http_sources",
    io_manager_key="s3_io_manager",
    automation_condition=TRADE_CALENDAR_AUTOMATION_CONDITION,
    tags={
        "source": "sina",
        "layer": "raw",
        "storage": "s3",
    },
)
def sina__trade_calendar(context) -> dg.MaterializeResult[pa.Table]:
    """A-share trade calendar decoded from Sina Finance."""

    retry_policy = DEFAULT_RETRY_POLICY
    max_attempts = MAX_REQUEST_ATTEMPTS
    content = fetch_sina_trade_calendar(
        retry_policy=retry_policy,
        max_attempts=max_attempts,
    )
    parse_result = SinaCalendarParser().parse_with_diagnostics(content)
    if not parse_result.dates:
        context.log.warning(
            "Sina trade-calendar parser returned no rows: %s",
            parse_result.error_message or "unknown parser failure",
        )
        msg = "Sina trade calendar parser returned no rows"
        raise RuntimeError(msg)
    if parse_result.missing_date_added:
        context.log.debug("Added known missing Sina trade date %s", SINA_KNOWN_MISSING_DATE)

    trade_dates = parse_result.dates
    table = trade_calendar_dates_to_table(trade_dates)
    metadata = {
        "source_url": dg.MetadataValue.url(SINA_TRADE_CALENDAR_URL),
        "row_count": len(trade_dates),
        "min_trade_date": min(trade_dates).isoformat(),
        "max_trade_date": max(trade_dates).isoformat(),
        "file_format": "parquet",
        "compression": "zstd",
        "retry_policy": dg.MetadataValue.json(retry_policy.metadata(max_attempts=max_attempts)),
    }
    context.log.info("Parsed %s Sina trade-calendar rows", len(trade_dates))

    return dg.MaterializeResult(
        value=table,
        metadata=metadata,
    )
