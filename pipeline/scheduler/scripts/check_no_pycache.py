from __future__ import annotations

from pathlib import Path


def main() -> int:
    scheduler_root = Path(__file__).resolve().parents[1]
    roots = (scheduler_root / "src", scheduler_root / "tests")
    pycache_files = [
        path
        for root in roots
        for path in root.rglob("*")
        if "__pycache__" in path.parts and path.is_file()
    ]
    if pycache_files:
        for path in pycache_files:
            print(path)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
