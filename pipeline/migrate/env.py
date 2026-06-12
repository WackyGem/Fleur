from __future__ import annotations

import os
from logging.config import fileConfig
from pathlib import Path
from typing import Final, NamedTuple

import psycopg
from alembic import context
from psycopg import sql
from sqlalchemy import MetaData, engine_from_config, pool
from sqlalchemy.engine import make_url

config = context.config

if config.config_file_name is not None:
    fileConfig(config.config_file_name)

target_metadata = MetaData()

PIPELINE_TARGET: Final = "pipeline"
REARVIEW_TARGET: Final = "rearview"
ALL_TARGET: Final = "all"


class MigrationTarget(NamedTuple):
    name: str
    database_url_env: str


TARGETS: Final = {
    PIPELINE_TARGET: MigrationTarget(
        name=PIPELINE_TARGET,
        database_url_env="PIPELINE_DATABASE_URL",
    ),
    REARVIEW_TARGET: MigrationTarget(
        name=REARVIEW_TARGET,
        database_url_env="REARVIEW_DATABASE_URL",
    ),
}


def _load_repo_dotenv() -> None:
    """Load root `.env` for local migration commands without overriding the shell."""
    env_path = Path(__file__).resolve().parents[2] / ".env"
    if not env_path.exists():
        return

    for raw_line in env_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, raw_value = line.split("=", 1)
        key = key.strip()
        value = raw_value.strip().strip("\"'")
        if key and key not in os.environ:
            os.environ[key] = value


def _selected_target_names() -> list[str]:
    raw_target = context.get_x_argument(as_dictionary=True).get("target", PIPELINE_TARGET)
    target = raw_target.strip().lower()
    if target == ALL_TARGET:
        return [PIPELINE_TARGET, REARVIEW_TARGET]
    if target in TARGETS:
        return [target]
    valid_targets = ", ".join([*TARGETS, ALL_TARGET])
    msg = f"invalid Alembic target {raw_target!r}; expected one of: {valid_targets}"
    raise RuntimeError(msg)


def _database_url(target: MigrationTarget) -> str:
    value = os.environ.get(target.database_url_env)
    if value is None or not value.strip():
        msg = f"{target.database_url_env} is required for {target.name} Alembic migrations"
        raise RuntimeError(msg)
    if value.startswith("postgresql://"):
        return value.replace("postgresql://", "postgresql+psycopg://", 1)
    return value


def _ensure_database_exists(target: MigrationTarget) -> None:
    target_url = make_url(_database_url(target))
    database_name = target_url.database
    if database_name is None or not database_name.strip():
        msg = f"{target.database_url_env} must include a database name"
        raise RuntimeError(msg)

    maintenance_database = "postgres" if database_name != "postgres" else "template1"
    maintenance_url = target_url.set(database=maintenance_database)
    dsn = maintenance_url.render_as_string(hide_password=False).replace(
        "postgresql+psycopg://",
        "postgresql://",
        1,
    )

    with psycopg.connect(dsn, autocommit=True) as connection, connection.cursor() as cursor:
        cursor.execute(
            "select 1 from pg_database where datname = %s",
            (database_name,),
        )
        if cursor.fetchone() is not None:
            return
        cursor.execute(sql.SQL("create database {}").format(sql.Identifier(database_name)))


def run_migrations_offline() -> None:
    target_names = _selected_target_names()
    if len(target_names) != 1:
        msg = "offline migrations require -x target=pipeline or -x target=rearview"
        raise RuntimeError(msg)

    target = TARGETS[target_names[0]]
    config.attributes["target"] = target.name
    context.configure(
        url=_database_url(target),
        target_metadata=target_metadata,
        literal_binds=True,
        dialect_opts={"paramstyle": "named"},
    )

    with context.begin_transaction():
        context.run_migrations()


def run_migrations_online() -> None:
    section = config.get_section(config.config_ini_section) or {}
    for target_name in _selected_target_names():
        target = TARGETS[target_name]
        config.attributes["target"] = target.name
        _ensure_database_exists(target)
        section["sqlalchemy.url"] = _database_url(target)
        connectable = engine_from_config(
            section,
            prefix="sqlalchemy.",
            poolclass=pool.NullPool,
        )

        with connectable.connect() as connection:
            context.configure(connection=connection, target_metadata=target_metadata)

            with context.begin_transaction():
                context.run_migrations()


_load_repo_dotenv()

if context.is_offline_mode():
    run_migrations_offline()
else:
    run_migrations_online()
