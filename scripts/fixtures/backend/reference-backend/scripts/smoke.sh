#!/usr/bin/env bash
set -euo pipefail

PACKAGE_REL="scripts/fixtures/backend/reference-backend"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT="$(cd "$PACKAGE_DIR/../../../.." && pwd)"
PORT="${PORT:-18080}"
JOB_POLL_MS="${JOB_POLL_MS:-500}"
BASE_URL="${BASE_URL:-http://127.0.0.1:${PORT}}"
ARTIFACT_DIR="$ROOT/.tmp/m051-s02/fixture-smoke"
BUILD_DIR="$ARTIFACT_DIR/build"
BINARY_PATH="$BUILD_DIR/reference-backend"
LOG_FILE="$ARTIFACT_DIR/reference-backend.log"
SERVER_PID=""
ACCIDENTAL_BINARY="$PACKAGE_DIR/reference-backend"
SOURCE_MANIFEST="$PACKAGE_DIR/mesh.toml"
SOURCE_DEPLOY_SMOKE="$PACKAGE_DIR/scripts/deploy-smoke.sh"

usage() {
  echo "usage: bash $PACKAGE_REL/scripts/smoke.sh" >&2
}

fail() {
  echo "[smoke] $1" >&2
  exit 1
}

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    fail "required command missing from PATH: $command_name"
  fi
}

require_file() {
  local label="$1"
  local path="$2"
  if [[ ! -f "$path" ]]; then
    fail "missing required ${label}: $path"
  fi
}

ensure_source_tree_clean() {
  if [[ -e "$ACCIDENTAL_BINARY" ]]; then
    fail "fixture source tree contains an in-place binary: $ACCIDENTAL_BINARY"
  fi
}

cleanup() {
  local status=$?
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  if [[ $status -ne 0 ]]; then
    echo "[smoke] failure; tailing server log from $LOG_FILE" >&2
    tail -n 200 "$LOG_FILE" >&2 || true
  fi
}
trap cleanup EXIT

if [[ $# -ne 0 ]]; then
  usage
  exit 1
fi

: "${DATABASE_URL:?set DATABASE_URL}"

for required_command in cargo psql; do
  require_command "$required_command"
done
require_file "fixture manifest" "$SOURCE_MANIFEST"
require_file "deploy smoke script" "$SOURCE_DEPLOY_SMOKE"
ensure_source_tree_clean

if [[ "$(psql "$DATABASE_URL" -Atqc "SELECT to_regclass('public.jobs') IS NOT NULL")" != "t" ]]; then
  fail "jobs table is missing; run either: cargo run -q -p meshc -- migrate $PACKAGE_REL up OR bash $PACKAGE_REL/scripts/apply-deploy-migrations.sh $PACKAGE_REL/deploy/reference-backend.up.sql"
fi

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"
rm -f "$LOG_FILE"

echo "[smoke] building reference-backend fixture into $BINARY_PATH"
(
  cd "$ROOT"
  cargo run -q -p meshc -- build "$PACKAGE_REL" --output "$BINARY_PATH"
)

ensure_source_tree_clean

if [[ ! -x "$BINARY_PATH" ]]; then
  fail "built binary is missing or not executable: $BINARY_PATH"
fi

echo "[smoke] starting reference-backend on :$PORT"
(
  cd "$BUILD_DIR"
  PORT="$PORT" JOB_POLL_MS="$JOB_POLL_MS" DATABASE_URL="$DATABASE_URL" "$BINARY_PATH" >"$LOG_FILE" 2>&1
) &
SERVER_PID=$!

echo "[smoke] probing running instance via deploy-smoke.sh"
env PORT="$PORT" BASE_URL="$BASE_URL" bash "$PACKAGE_DIR/scripts/deploy-smoke.sh"
