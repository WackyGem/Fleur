from __future__ import annotations

import os
from logging.config import fileConfig

from alembic import context
import psycopg
from psycopg import sql
from sqlalchemy import MetaData, engine_from_config, pool
from sqlalchemy.engine import make_url

config = context.config

if config.config_file_name is not None:
    fileConfig(config.config_file_name)

target_metadata = MetaData()


def _database_url() -> str:
    value = os.environ.get("PIPELINE_DATABASE_URL")
    if value is None or not value.strip():
        msg = "PIPELINE_DATABASE_URL is required for Alembic migrations"
        raise RuntimeError(msg)
    if value.startswith("postgresql://"):
        return value.replace("postgresql://", "postgresql+psycopg://", 1)
    return value


def _ensure_database_exists() -> None:
    target_url = make_url(_database_url())
    database_name = target_url.database
    if database_name is None or not database_name.strip():
        msg = "PIPELINE_DATABASE_URL must include a database name"
        raise RuntimeError(msg)

    maintenance_database = "postgres" if database_name != "postgres" else "template1"
    maintenance_url = target_url.set(database=maintenance_database)
    dsn = maintenance_url.render_as_string(hide_password=False).replace(
        "postgresql+psycopg://",
        "postgresql://",
        1,
    )

    with psycopg.connect(dsn, autocommit=True) as connection:
        with connection.cursor() as cursor:
            cursor.execute(
                "select 1 from pg_database where datname = %s",
                (database_name,),
            )
            if cursor.fetchone() is not None:
                return
            cursor.execute(sql.SQL("create database {}").format(sql.Identifier(database_name)))


def run_migrations_offline() -> None:
    context.configure(
        url=_database_url(),
        target_metadata=target_metadata,
        literal_binds=True,
        dialect_opts={"paramstyle": "named"},
    )

    with context.begin_transaction():
        context.run_migrations()


def run_migrations_online() -> None:
    _ensure_database_exists()
    section = config.get_section(config.config_ini_section) or {}
    section["sqlalchemy.url"] = _database_url()
    connectable = engine_from_config(
        section,
        prefix="sqlalchemy.",
        poolclass=pool.NullPool,
    )

    with connectable.connect() as connection:
        context.configure(connection=connection, target_metadata=target_metadata)

        with context.begin_transaction():
            context.run_migrations()


if context.is_offline_mode():
    run_migrations_offline()
else:
    run_migrations_online()
