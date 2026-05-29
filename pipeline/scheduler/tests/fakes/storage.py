from __future__ import annotations

from typing import Any, cast

import pyarrow.fs as pafs


def local_filesystem() -> Any:
    return cast(Any, pafs).LocalFileSystem()


class InMemoryFilesystem:
    def __init__(self) -> None:
        self.data: dict[str, bytes] = {}

    def open_output_stream(self, path: str) -> object:
        filesystem = self

        class OutputStream:
            def __enter__(self) -> OutputStream:
                self.buffer = bytearray()
                return self

            def __exit__(
                self,
                exc_type: type[BaseException] | None,
                exc_value: BaseException | None,
                traceback: object,
            ) -> None:
                filesystem.data[path] = bytes(self.buffer)

            def write(self, data: bytes) -> None:
                self.buffer.extend(data)

        return OutputStream()

    def open_input_file(self, path: str) -> object:
        filesystem = self

        class InputFile:
            def __enter__(self) -> InputFile:
                return self

            def __exit__(
                self,
                exc_type: type[BaseException] | None,
                exc_value: BaseException | None,
                traceback: object,
            ) -> None:
                return None

            def read(self) -> bytes:
                return filesystem.data[path]

        return InputFile()
