from __future__ import annotations

import base64
from collections.abc import Mapping

from scheduler.defs.http_resources.client import AioHttpClient, HttpRequest
from scheduler.defs.jiuyan_industry_ocr.ocr_schema import OcrSchemaError


class StockThemeSchema:
    SYSTEM_PROMPT = """Extract the image info into the following Template:
[{"stock_name":"","theme_path":"","relation":"","source":""}]

Rules:
1. Extract all stock/company entries visible in the image.
2. stock_name: 个股、公司或标的名称；必须来自图片文字，不要补充常识。
3. theme_path: 题材路径或分类；多级题材用英文逗号 `,` 连接；不要用空格连接多级题材；empty if not present.
4. relation: 个股与题材的关联信息或原文说明；需要合理识别区分个股名称和关联说明；empty if not present.
5. source: 来源、信源或资料来源；empty if not present.
6. Do not summarize, infer, normalize stock names, or add facts not visible in the image.
7. Return valid JSON array only, no Markdown, no code block, no other text.
"""
    JSON_SCHEMA = {
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "stock_name": {"type": "string"},
                "theme_path": {"type": "string"},
                "relation": {"type": "string"},
                "source": {"type": "string"},
            },
            "required": [
                "stock_name",
                "theme_path",
                "relation",
                "source",
            ],
            "additionalProperties": False,
        },
    }

    @classmethod
    def get_system_prompt(cls) -> str:
        return cls.SYSTEM_PROMPT

    @classmethod
    def get_json_schema(cls) -> dict[str, object]:
        return cls.JSON_SCHEMA


def build_image_data_url(mime_type: str, image_bytes: bytes) -> str:
    encoded = base64.b64encode(image_bytes).decode("ascii")
    return f"data:{mime_type};base64,{encoded}"


def build_ocr_request_payload(model_name: str, image_data_url: str) -> dict[str, object]:
    return {
        "model": model_name,
        "messages": [
            {
                "role": "system",
                "content": [
                    {"type": "text", "text": StockThemeSchema.get_system_prompt()},
                ],
            },
            {
                "role": "user",
                "content": [{"type": "image_url", "image_url": {"url": image_data_url}}],
            },
        ],
        "max_tokens": 8192,
        "temperature": 0.2,
        "top_p": 0.8,
        "top_k": 20,
        "presence_penalty": 0.0,
        "repetition_penalty": 1.0,
        "include_reasoning": False,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "stock_theme",
                "schema": StockThemeSchema.get_json_schema(),
                "strict": True,
            },
        },
        "chat_template_kwargs": {"enable_thinking": False},
    }


async def request_ocr_content(
    client: AioHttpClient,
    *,
    base_url: str,
    model_name: str,
    image_bytes: bytes,
    mime_type: str,
) -> str:
    payload = build_ocr_request_payload(model_name, build_image_data_url(mime_type, image_bytes))
    response = await client.request_json_object(
        HttpRequest(
            method="POST",
            url=f"{base_url.rstrip('/')}/v1/chat/completions",
            json_body=payload,
        )
    )
    return extract_ocr_content(response)


def extract_ocr_content(payload: Mapping[str, object]) -> str:
    choices = payload.get("choices")
    if not isinstance(choices, list) or not choices:
        msg = "OCR response missing choices"
        raise OcrSchemaError(msg)

    first_choice = choices[0]
    if not isinstance(first_choice, Mapping):
        msg = "OCR response choices[0] is not an object"
        raise OcrSchemaError(msg)

    message = first_choice.get("message")
    if not isinstance(message, Mapping):
        msg = "OCR response message is not an object"
        raise OcrSchemaError(msg)

    content = message.get("content")
    if not isinstance(content, str):
        msg = "OCR response content is not a string"
        raise OcrSchemaError(msg)
    return content
