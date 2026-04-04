#!/usr/bin/env bash
set -euo pipefail

MIGRATION_VERSION="20260323010000"
SQL_PATH="${1:-}"

usage() {
  echo "usage: bash apply-deploy-migrations.sh <deploy-sql-path>" >&2
}

if [[ $# -ne 1 ]]; then
  usage
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "[deploy-apply] psql is required but was not found on PATH" >&2
  exit 1
fi

if [[ ! -f "$SQL_PATH" ]]; then
  echo "[deploy-apply] missing deploy SQL artifact: $SQL_PATH" >&2
  exit 1
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "[deploy-apply] DATABASE_URL must be set" >&2
  exit 1
fi

printf '[deploy-apply] sql artifact=%s\n' "$SQL_PATH"
printf '[deploy-apply] applying migration version=%s via psql\n' "$MIGRATION_VERSION"
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$SQL_PATH" >/dev/null

applied_version="$(psql "$DATABASE_URL" -Atqc "SELECT version::text FROM _mesh_migrations WHERE version = ${MIGRATION_VERSION}")"
if [[ "$applied_version" != "$MIGRATION_VERSION" ]]; then
  echo "[deploy-apply] migration record missing after apply version=$MIGRATION_VERSION" >&2
  exit 1
fi

printf '[deploy-apply] migration recorded version=%s\n' "$MIGRATION_VERSION"
