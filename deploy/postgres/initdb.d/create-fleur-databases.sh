#!/bin/sh
set -eu

create_database_if_missing() {
  database_name="$1"
  psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    SELECT 'CREATE DATABASE "${database_name}"'
    WHERE NOT EXISTS (
      SELECT FROM pg_database WHERE datname = '${database_name}'
    )\gexec
EOSQL
}

create_database_if_missing pipeline
create_database_if_missing rearview
