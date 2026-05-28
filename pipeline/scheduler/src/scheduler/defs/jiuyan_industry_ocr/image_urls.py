from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from urllib.parse import urlsplit

IMAGE_URL_PATTERN = re.compile(
    r"https?://[^\s\"'<>]+?\.(?:png|jpe?g)(?:\?[^\s\"'<>]*)?",
    re.IGNORECASE,
)
IMAGE_S3_PREFIX = "img/jiuyan__industry_images"


def parse_image_urls(imgs: object) -> list[str]:
    urls: list[str] = []
    seen: set[str] = set()
    for candidate in _image_url_candidates(imgs):
        for match in IMAGE_URL_PATTERN.findall(candidate):
            normalized = normalize_image_url(match)
            if normalized not in seen:
                seen.add(normalized)
                urls.append(normalized)
    return urls


def normalize_image_url(url: str) -> str:
    return url.strip()


def image_filename_from_url(url: str) -> str:
    parsed = urlsplit(normalize_image_url(url))
    filename = Path(parsed.path).name
    if filename and filename not in {".", ".."}:
        return filename

    digest = hashlib.sha256(url.encode("utf-8")).hexdigest()
    return f"{digest}.img"


def image_s3_key(image_filename: str) -> str:
    return f"{IMAGE_S3_PREFIX}/{image_filename}"


def _image_url_candidates(imgs: object) -> list[str]:
    if imgs is None:
        return []
    if isinstance(imgs, str):
        stripped = imgs.strip()
        if not stripped:
            return []
        try:
            parsed = json.loads(stripped)
        except json.JSONDecodeError:
            return [stripped]
        return _string_candidates(parsed)
    return _string_candidates(imgs)


def _string_candidates(value: object) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, bytes):
        return [value.decode("utf-8", errors="replace")]
    if isinstance(value, (list, tuple)):
        candidates: list[str] = []
        for item in value:
            candidates.extend(_string_candidates(item))
        return candidates
    return [str(value)] if value is not None else []
