from __future__ import annotations

from scheduler.defs.clickhouse.specs import ENABLED_CLICKHOUSE_RAW_POOL_NAMES


def main() -> int:
    for pool_name in ENABLED_CLICKHOUSE_RAW_POOL_NAMES:
        print(pool_name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
