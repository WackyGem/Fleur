#!/bin/sh
set -eu

readonly_user="${FLEUR_MARTS_READONLY_USER:-}"
readonly_password="${FLEUR_MARTS_READONLY_PASSWORD:-}"
marts_database="${REARVIEW_CLICKHOUSE_MARTS_DATABASE:-fleur_marts}"

if [ -z "$readonly_user" ] && [ -z "$readonly_password" ]; then
  exit 0
fi

if [ -z "$readonly_user" ] || [ -z "$readonly_password" ]; then
  printf '%s\n' \
    "FLEUR_MARTS_READONLY_USER and FLEUR_MARTS_READONLY_PASSWORD must be set together" >&2
  exit 1
fi

validate_identifier() {
  name="$1"
  value="$2"
  case "$value" in
    "" | *[!A-Za-z0-9_-]*)
      printf '%s\n' "$name must contain only letters, digits, underscores, or hyphens" >&2
      exit 1
      ;;
  esac
}

quote_identifier() {
  name="$1"
  value="$2"
  validate_identifier "$name" "$value"
  printf '`%s`' "$value"
}

quote_string() {
  printf "'"
  printf '%s' "$1" | sed "s/\\\\/\\\\\\\\/g; s/'/\\\\'/g"
  printf "'"
}

readonly_role_identifier="$(quote_identifier READONLY_ROLE fleur_marts_readonly)"
readonly_profile_identifier="$(quote_identifier READONLY_PROFILE fleur_marts_readonly_profile)"
readonly_user_identifier="$(quote_identifier FLEUR_MARTS_READONLY_USER "$readonly_user")"
marts_database_identifier="$(quote_identifier REARVIEW_CLICKHOUSE_MARTS_DATABASE "$marts_database")"
readonly_password_literal="$(quote_string "$readonly_password")"

clickhouse-client --host 127.0.0.1 --user "$CLICKHOUSE_USER" --password "$CLICKHOUSE_PASSWORD" --multiquery <<-EOSQL
CREATE SETTINGS PROFILE IF NOT EXISTS $readonly_profile_identifier
SETTINGS
    readonly = 2,
    max_execution_time = 30,
    timeout_before_checking_execution_speed = 0,
    max_rows_to_read = 1000000000,
    max_bytes_to_read = 100000000000,
    max_result_rows = 10000,
    result_overflow_mode = 'break';

ALTER SETTINGS PROFILE $readonly_profile_identifier
SETTINGS
    readonly = 2,
    max_execution_time = 30,
    timeout_before_checking_execution_speed = 0,
    max_rows_to_read = 1000000000,
    max_bytes_to_read = 100000000000,
    max_result_rows = 10000,
    result_overflow_mode = 'break';

CREATE ROLE IF NOT EXISTS $readonly_role_identifier;

GRANT SELECT ON $marts_database_identifier.* TO $readonly_role_identifier;
GRANT SELECT ON system.databases TO $readonly_role_identifier;
GRANT SELECT ON system.tables TO $readonly_role_identifier;
GRANT SELECT ON system.columns TO $readonly_role_identifier;

CREATE USER IF NOT EXISTS $readonly_user_identifier
IDENTIFIED WITH sha256_password BY $readonly_password_literal;

ALTER USER $readonly_user_identifier
IDENTIFIED WITH sha256_password BY $readonly_password_literal
SETTINGS PROFILE $readonly_profile_identifier;

GRANT $readonly_role_identifier TO $readonly_user_identifier;
EOSQL
