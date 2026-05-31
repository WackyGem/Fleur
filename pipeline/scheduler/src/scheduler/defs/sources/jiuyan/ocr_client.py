from __future__ import annotations

from scheduler.defs.http.client import HttpRequest
from scheduler.defs.http.protocols import HttpJsonClientProtocol
from scheduler.defs.ocr.client import build_image_data_url, extract_ocr_content

OCR_MAX_TOKENS = 16_384
OCR_TEMPERATURE = 0.2
OCR_TOP_P = 0.95
OCR_TOP_K = 20
OCR_PRESENCE_PENALTY = 1.5
OCR_REPETITION_PENALTY = 1.0


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
        "max_tokens": OCR_MAX_TOKENS,
        "temperature": OCR_TEMPERATURE,
        "top_p": OCR_TOP_P,
        "top_k": OCR_TOP_K,
        "presence_penalty": OCR_PRESENCE_PENALTY,
        "repetition_penalty": OCR_REPETITION_PENALTY,
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
    client: HttpJsonClientProtocol,
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
