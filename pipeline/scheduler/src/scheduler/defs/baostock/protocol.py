from __future__ import annotations

import json
import zlib
from dataclasses import dataclass

CLIENT_VERSION = "00.9.10"
SERVER_VERSION = "00.9.00"
MESSAGE_SPLIT = "\x01"
MESSAGE_END = b"<![CDATA[]]>\n"
MESSAGE_HEADER_LENGTH = 21
DEFAULT_PAGE_SIZE = 10000
COMPRESSED_RESPONSE_CODES = {"96"}
LOGIN_API_NAMES = {"login", "logout"}


class BaostockError(Exception):
    """Base BaoStock client error."""


class BaostockNetworkError(BaostockError):
    """Raised for TCP connection, send, receive, or timeout failures."""


class BaostockProtocolError(BaostockError):
    """Raised when a BaoStock message cannot be encoded or decoded."""


class BaostockAuthenticationError(BaostockError):
    """Raised when BaoStock authentication fails or cannot be refreshed."""


class BaostockResponseError(BaostockError):
    """Raised when BaoStock returns a non-success business error."""

    def __init__(
        self,
        error_code: str,
        error_message: str,
        api_name: str,
        params: list[str] | None = None,
    ) -> None:
        super().__init__(f"{api_name} failed with {error_code}: {error_message}")
        self.error_code = error_code
        self.error_message = error_message
        self.api_name = api_name
        self.params = params or []


@dataclass(frozen=True)
class BaostockResponse:
    response_code: str
    error_code: str
    error_message: str
    api_name: str
    user_id: str
    page: int
    page_size: int
    records: list[list[str]]
    field_names: list[str]
    params: list[str]

    def has_next_page(self) -> bool:
        return len(self.records) == self.page_size and len(self.records) > 0


def encode_request(
    request_code: str,
    api_name: str,
    user_id: str,
    params: list[str],
    page: int = 1,
    page_size: int = DEFAULT_PAGE_SIZE,
) -> bytes:
    body_parts = [api_name, user_id]
    if api_name not in LOGIN_API_NAMES:
        body_parts.extend([str(page), str(page_size)])
    body_parts.extend(params)
    body = MESSAGE_SPLIT.join(body_parts)
    header = f"{CLIENT_VERSION}{MESSAGE_SPLIT}{request_code}{MESSAGE_SPLIT}{len(body):010d}"
    head_body = f"{header}{body}"
    crc32_value = zlib.crc32(head_body.encode("utf-8"))
    return f"{head_body}{MESSAGE_SPLIT}{crc32_value}\n".encode("utf-8")


def decode_response(message: bytes) -> BaostockResponse:
    if not message.endswith(MESSAGE_END):
        msg = "BaoStock response did not end with the expected CDATA marker"
        raise BaostockProtocolError(msg)
    if len(message) < MESSAGE_HEADER_LENGTH:
        msg = "BaoStock response is shorter than the fixed header"
        raise BaostockProtocolError(msg)

    header_bytes = message[:MESSAGE_HEADER_LENGTH]
    try:
        header = header_bytes.decode("utf-8")
    except UnicodeDecodeError as error:
        msg = "BaoStock response header is not valid UTF-8"
        raise BaostockProtocolError(msg) from error

    header_parts = header.split(MESSAGE_SPLIT)
    if len(header_parts) != 3:
        msg = f"BaoStock response header has {len(header_parts)} fields"
        raise BaostockProtocolError(msg)

    _, response_code, body_length_text = header_parts
    if not body_length_text.isdigit():
        msg = f"BaoStock response body length is invalid: {body_length_text!r}"
        raise BaostockProtocolError(msg)
    body_length = int(body_length_text)

    if response_code in COMPRESSED_RESPONSE_CODES:
        body_bytes = message[MESSAGE_HEADER_LENGTH : MESSAGE_HEADER_LENGTH + body_length]
        try:
            body = zlib.decompress(body_bytes).decode("utf-8")
        except (zlib.error, UnicodeDecodeError) as error:
            msg = "BaoStock compressed response body could not be decoded"
            raise BaostockProtocolError(msg) from error
    else:
        body_end = -(len(MESSAGE_END))
        plain_message = message[:body_end].decode("utf-8")
        _validate_crc(plain_message)
        body = plain_message.rsplit(MESSAGE_SPLIT, 1)[0][MESSAGE_HEADER_LENGTH:]

    return _decode_response_body(response_code, body)


def aggregate_responses(responses: list[BaostockResponse]) -> BaostockResponse:
    if not responses:
        msg = "Cannot aggregate an empty BaoStock response list"
        raise ValueError(msg)

    first = responses[0]
    records: list[list[str]] = []
    for response in responses:
        records.extend(response.records)

    return BaostockResponse(
        response_code=first.response_code,
        error_code=first.error_code,
        error_message=first.error_message,
        api_name=first.api_name,
        user_id=first.user_id,
        page=first.page,
        page_size=first.page_size,
        records=records,
        field_names=first.field_names,
        params=first.params,
    )


def _validate_crc(message_without_end: str) -> None:
    parts = message_without_end.rsplit(MESSAGE_SPLIT, 1)
    if len(parts) != 2:
        msg = "BaoStock response is missing CRC"
        raise BaostockProtocolError(msg)
    head_body, crc_text = parts
    if not crc_text.isdigit():
        msg = f"BaoStock response CRC is invalid: {crc_text!r}"
        raise BaostockProtocolError(msg)
    expected_crc = zlib.crc32(head_body.encode("utf-8"))
    actual_crc = int(crc_text)
    if actual_crc != expected_crc:
        msg = f"BaoStock response CRC mismatch: expected {expected_crc}, got {actual_crc}"
        raise BaostockProtocolError(msg)


def _decode_response_body(response_code: str, body: str) -> BaostockResponse:
    body_parts = body.split(MESSAGE_SPLIT)
    if len(body_parts) < 4:
        msg = f"BaoStock response body has too few fields: {body_parts!r}"
        raise BaostockProtocolError(msg)

    error_code = body_parts[0]
    error_message = body_parts[1]
    api_name = body_parts[2] if len(body_parts) > 2 else ""
    user_id = body_parts[3] if len(body_parts) > 3 else ""
    if error_code != "0":
        return BaostockResponse(
            response_code=response_code,
            error_code=error_code,
            error_message=error_message,
            api_name=api_name,
            user_id=user_id,
            page=1,
            page_size=DEFAULT_PAGE_SIZE,
            records=[],
            field_names=[],
            params=[],
        )

    if api_name in LOGIN_API_NAMES:
        return BaostockResponse(
            response_code=response_code,
            error_code=error_code,
            error_message=error_message,
            api_name=api_name,
            user_id=user_id,
            page=1,
            page_size=DEFAULT_PAGE_SIZE,
            records=[],
            field_names=[],
            params=[],
        )

    if len(body_parts) < 9:
        msg = f"BaoStock success response body has too few fields: {body_parts!r}"
        raise BaostockProtocolError(msg)

    page = _parse_int(body_parts[4], "page")
    page_size = _parse_int(body_parts[5], "page_size")
    records = _decode_records(body_parts[6])
    params, field_names = _decode_params_and_fields(api_name, body_parts)
    return BaostockResponse(
        response_code=response_code,
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


def _decode_records(records_json: str) -> list[list[str]]:
    if not records_json.strip():
        return []
    try:
        payload = json.loads("".join(records_json.split()))
    except json.JSONDecodeError as error:
        msg = "BaoStock response record JSON could not be decoded"
        raise BaostockProtocolError(msg) from error
    records = payload.get("record")
    if not isinstance(records, list):
        msg = "BaoStock response record JSON is missing a list record field"
        raise BaostockProtocolError(msg)
    return [[str(value) for value in row] for row in records]


def _decode_params_and_fields(
    api_name: str,
    body_parts: list[str],
) -> tuple[list[str], list[str]]:
    if api_name == "query_history_k_data_plus":
        if len(body_parts) < 13:
            msg = f"BaoStock {api_name} response body has too few fields: {body_parts!r}"
            raise BaostockProtocolError(msg)
        params = [body_parts[7], body_parts[9], body_parts[10], body_parts[11], body_parts[12]]
        field_names = _decode_field_names(body_parts[8])
        return params, field_names

    if api_name == "query_stock_basic":
        if len(body_parts) < 10:
            msg = f"BaoStock {api_name} response body has too few fields: {body_parts!r}"
            raise BaostockProtocolError(msg)
        params = body_parts[7:-1]
        field_names = _decode_field_names(body_parts[-1])
        return params, field_names

    params = body_parts[7:-1]
    field_names = _decode_field_names(body_parts[-1])
    return params, field_names


def _decode_field_names(fields: str) -> list[str]:
    if not fields.strip():
        return []
    return [field.strip() for field in fields.split(",")]


def _parse_int(value: str, field_name: str) -> int:
    if not value.isdigit():
        msg = f"BaoStock response {field_name} is invalid: {value!r}"
        raise BaostockProtocolError(msg)
    return int(value)
