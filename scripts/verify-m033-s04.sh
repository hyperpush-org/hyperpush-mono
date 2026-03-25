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

schema = Path("mesher/storage/schema.mpl").read_text()
queries = Path("mesher/storage/queries.mpl").read_text()
retention = Path("mesher/services/retention.mpl").read_text()
main = Path("mesher/main.mpl").read_text()


def fn_block(text: str, name: str) -> str:
    marker = f"pub fn {name}("
    start = text.index(marker)
    end = text.find("\npub fn ", start + 1)
    return text[start:] if end == -1 else text[start:end]


def code_only(block: str) -> str:
    return "\n".join(
        line for line in block.splitlines() if not line.lstrip().startswith("#")
    )


expected_schema_calls = {
    "create_partitions_ahead": "Pg.create_daily_partitions_ahead(pool, \"events\", days)",
    "get_expired_partitions": "Pg.list_daily_partitions_before(pool, \"events\", max_days)",
    "drop_partition": "Pg.drop_partition(pool, partition_name)",
}

for name, call in expected_schema_calls.items():
    block = fn_block(schema, name)
    body = code_only(block)
    if call not in body:
        raise SystemExit(f"{name} drifted away from the Pg helper boundary:\n{block}")
    for token in ("Repo.query_raw", "Repo.execute_raw", "Query.select_raw"):
        if token in body:
            raise SystemExit(f"{name} regressed to {token}:\n{block}")

for banned in ("pub fn get_expired_partitions(", "pub fn drop_partition("):
    if banned in queries:
        raise SystemExit(f"mesher/storage/queries.mpl still exports an S04-owned partition helper: {banned}")

if "from Storage.Schema import get_expired_partitions, drop_partition" not in retention:
    raise SystemExit("retention.mpl no longer imports partition lifecycle helpers from Storage.Schema")
if "get_expired_partitions(pool, 90)" not in retention:
    raise SystemExit("retention.mpl no longer uses the Int-based Storage.Schema.get_expired_partitions(pool, 90) call")
for expected_log in (
    "Retention event cleanup failed for project",
    "Retention partition listing failed",
    "Retention partition drop failed for",
):
    if expected_log not in retention:
        raise SystemExit(f"retention.mpl lost the observability string: {expected_log}")

if 'from Storage.Schema import create_partitions_ahead' not in main:
    raise SystemExit("main.mpl no longer imports create_partitions_ahead from Storage.Schema")
for expected_log in (
    'Partition bootstrap succeeded (7 days ahead)',
    'Partition bootstrap failed: #{e}',
):
    if expected_log not in main:
        raise SystemExit(f"main.mpl lost the startup partition log: {expected_log}")

print("s04 helper ownership and logging ok")
PY
  then
    fail_with_log "python ownership/logging sweep" "S04 helper ownership or logs drifted" "$log_path"
  fi
}

run_expect_success e2e_m033_s04 cargo test -p meshc --test e2e_m033_s04 -- --nocapture
run_expect_success fmt_mesher cargo run -q -p meshc -- fmt --check mesher
run_expect_success build_mesher cargo run -q -p meshc -- build mesher
run_python_check helper_ownership

echo "verify-m033-s04: ok"
