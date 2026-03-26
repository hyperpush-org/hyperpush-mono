#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_DIR=".tmp/m033-s04/verify"
mkdir -p "$ARTIFACT_DIR"

fail_with_log() {
  local command_text="$1"
  local reason="$2"
  local log_path="${3:-}"

  echo "verification drift: ${reason}" >&2
  echo "failing command: ${command_text}" >&2
  if [[ -n "$log_path" && -f "$log_path" ]]; then
    echo "--- ${log_path} ---" >&2
    sed -n '1,320p' "$log_path" >&2
  fi
  exit 1
}

run_expect_success() {
  local label="$1"
  shift
  local -a cmd=("$@")
  local log_path="$ARTIFACT_DIR/${label}.log"
  local command_text="${cmd[*]}"

  echo "==> ${command_text}"
  if ! "${cmd[@]}" >"$log_path" 2>&1; then
    fail_with_log "$command_text" "expected success" "$log_path"
  fi
}

run_python_check() {
  local label="$1"
  local log_path="$ARTIFACT_DIR/${label}.log"

  if ! python3 >"$log_path" 2>&1 <<'PY'
from pathlib import Path

migration = Path("mesher/migrations/20260216120000_create_initial_schema.mpl").read_text()
schema = Path("mesher/storage/schema.mpl").read_text()
queries = Path("mesher/storage/queries.mpl").read_text()
retention = Path("mesher/services/retention.mpl").read_text()
main = Path("mesher/main.mpl").read_text()

RAW_BOUNDARY_TOKENS = (
    "Pool.execute(pool",
    "Migration.execute(pool",
    "Repo.query_raw",
    "Repo.execute_raw",
    "Query.select_raw",
)


def fn_block(text: str, name: str) -> str:
    marker = f"pub fn {name}("
    start = text.index(marker)
    lines = text[start:].splitlines()
    collected = []
    for idx, line in enumerate(lines):
        if idx != 0 and (line.startswith("pub fn ") or line.startswith("fn ")):
            break
        collected.append(line)
    return "\n".join(collected)


def code_only(block: str) -> str:
    return "\n".join(
        line for line in block.splitlines() if not line.lstrip().startswith("#")
    )


def assert_contains(text: str, needle: str, description: str) -> None:
    if needle not in text:
        raise SystemExit(f"{description} missing expected snippet: {needle}")


def assert_no_raw_tokens(text: str, description: str, tokens=RAW_BOUNDARY_TOKENS) -> None:
    for token in tokens:
        if token in text:
            raise SystemExit(f"{description} regressed to raw boundary token {token}")


assert_no_raw_tokens(
    migration,
    "mesher/migrations/20260216120000_create_initial_schema.mpl",
    ("Pool.execute(pool", "Migration.execute(pool", "Repo.query_raw", "Repo.execute_raw", "Query.select_raw"),
)
for needle in (
    'Pg.create_extension(pool, "pgcrypto")',
    'Pg.create_range_partitioned_table(pool,',
    'Pg.create_gin_index(pool, "events", "idx_events_tags", "tags", "jsonb_path_ops")',
):
    assert_contains(
        migration,
        needle,
        "mesher/migrations/20260216120000_create_initial_schema.mpl",
    )

expected_schema_calls = {
    "create_partitions_ahead": 'Pg.create_daily_partitions_ahead(pool, "events", days)',
    "get_expired_partitions": 'Pg.list_daily_partitions_before(pool, "events", max_days)',
    "drop_partition": 'Pg.drop_partition(pool, partition_name)',
}

for name, call in expected_schema_calls.items():
    block = fn_block(schema, name)
    body = code_only(block)
    if call not in body:
        raise SystemExit(f"{name} drifted away from the Pg helper boundary:\n{block}")
    assert_no_raw_tokens(body, f"mesher/storage/schema.mpl::{name}")

for banned in ("pub fn get_expired_partitions(", "pub fn drop_partition("):
    if banned in queries:
        raise SystemExit(f"mesher/storage/queries.mpl still exports an S04-owned partition helper: {banned}")

assert_contains(
    retention,
    "from Storage.Schema import get_expired_partitions, drop_partition",
    "mesher/services/retention.mpl",
)
assert_contains(
    retention,
    "get_expired_partitions(pool, 90)",
    "mesher/services/retention.mpl",
)
assert_contains(
    retention,
    "drop_partition(pool, partition_name)",
    "mesher/services/retention.mpl",
)
assert_no_raw_tokens(retention, "mesher/services/retention.mpl")
for expected_log in (
    "Retention event cleanup failed for project",
    "Retention partition listing failed",
    "Retention partition drop failed for",
):
    assert_contains(retention, expected_log, "mesher/services/retention.mpl")

assert_contains(
    main,
    'from Storage.Schema import create_partitions_ahead',
    "mesher/main.mpl",
)
assert_contains(main, "create_partitions_ahead(pool, 7)", "mesher/main.mpl")
assert_no_raw_tokens(main, "mesher/main.mpl")
for expected_log in (
    'Partition bootstrap succeeded (7 days ahead)',
    'Partition bootstrap failed: #{e}',
):
    assert_contains(main, expected_log, "mesher/main.mpl")

print("s04 migration/runtime raw-boundary sweep ok")
PY
  then
    fail_with_log "python raw-boundary sweep" "S04 migration/runtime raw-boundary or observability drifted" "$log_path"
  fi
}

run_expect_success e2e_m033_s04 cargo test -p meshc --test e2e_m033_s04 -- --nocapture
run_expect_success fmt_mesher cargo run -q -p meshc -- fmt --check mesher
run_expect_success build_mesher cargo run -q -p meshc -- build mesher
run_python_check raw_boundary

echo "verify-m033-s04: ok"
