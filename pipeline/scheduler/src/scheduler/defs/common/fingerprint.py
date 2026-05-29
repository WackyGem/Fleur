from __future__ import annotations

import json
from collections.abc import Mapping


def row_fingerprint(row: Mapping[str, object]) -> str:
    return json.dumps(row, sort_keys=True, ensure_ascii=False, default=str, separators=(",", ":"))
