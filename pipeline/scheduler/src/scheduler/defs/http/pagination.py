from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field

from scheduler.defs.common.fingerprint import row_fingerprint


@dataclass
class DuplicateRowTracker:
    seen_fingerprints: set[str] = field(default_factory=set)
    duplicate_count: int = 0

    def record(self, row: Mapping[str, object]) -> bool:
        fingerprint = row_fingerprint(row)
        if fingerprint in self.seen_fingerprints:
            self.duplicate_count += 1
            return False
        self.seen_fingerprints.add(fingerprint)
        return True

    @property
    def has_rows(self) -> bool:
        return bool(self.seen_fingerprints)
