from __future__ import annotations

from collections.abc import Sequence
from typing import Protocol


class ClickHouseQueryResult(Protocol):
    @property
    def result_rows(self) -> Sequence[Sequence[object]]: ...


class ClickHouseClientProtocol(Protocol):
    @property
    def server_version(self) -> str: ...

    def ping(self) -> bool: ...

    def command(
        self,
        cmd: str,
        *,
        settings: dict[str, object] | None = None,
    ) -> object: ...

    def query(
        self,
        query: str,
        *,
        settings: dict[str, object] | None = None,
    ) -> ClickHouseQueryResult: ...

    def close(self) -> None: ...
