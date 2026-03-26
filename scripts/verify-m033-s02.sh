#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_DIR=".tmp/m033-s02/verify"
mkdir -p "$ARTIFACT_DIR"

fail_with_log() {
  local command_text="$1"
  local reason="$2"
  local log_path="${3:-}"

  echo "verification drift: ${reason}" >&2
  echo "failing command: ${command_text}" >&2
  if [[ -n "$log_path" && -f "$log_path" ]]; then
    echo "--- ${log_path} ---" >&2
    sed -n '1,260p' "$log_path" >&2
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

queries = Path("mesher/storage/queries.mpl").read_text()
writer = Path("mesher/storage/writer.mpl").read_text()


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


def assert_owned_boundary(name: str, block: str, *, allowed_where_raw: tuple[str, ...] = ()) -> None:
    body = code_only(block)
    for token in ("Repo.query_raw", "Repo.execute_raw", "Query.select_raw"):
        if token in body:
            raise SystemExit(f"{name} regressed to {token}:\n{block}")

    raw_where_lines = [
        line.strip() for line in body.splitlines() if "Query.where_raw" in line
    ]
    unexpected = [
        line
        for line in raw_where_lines
        if not any(allowed in line for allowed in allowed_where_raw)
    ]
    if unexpected:
        raise SystemExit(
            f"{name} has unexpected Query.where_raw keep-sites:\n"
            + "\n".join(unexpected)
            + "\n--- function ---\n"
            + block
        )


assert_owned_boundary(
    "create_user",
    fn_block(queries, "create_user"),
)
assert_owned_boundary(
    "authenticate_user",
    fn_block(queries, "authenticate_user"),
)
assert_owned_boundary(
    "search_events_fulltext",
    fn_block(queries, "search_events_fulltext"),
    allowed_where_raw=("received_at > now() - interval '24 hours'",),
)
assert_owned_boundary(
    "filter_events_by_tag",
    fn_block(queries, "filter_events_by_tag"),
    allowed_where_raw=("received_at > now() - interval '24 hours'",),
)
assert_owned_boundary(
    "event_breakdown_by_tag",
    fn_block(queries, "event_breakdown_by_tag"),
    allowed_where_raw=("received_at > now() - interval '24 hours'",),
)
assert_owned_boundary(
    "create_alert_rule",
    fn_block(queries, "create_alert_rule"),
)
assert_owned_boundary(
    "fire_alert",
    fn_block(queries, "fire_alert"),
)
assert_owned_boundary("insert_event", fn_block(writer, "insert_event"))

extract_block = fn_block(queries, "extract_event_fields")
extract_code = code_only(extract_block)
if "Repo.query_raw(pool, sql, [event_json])" not in extract_code:
    raise SystemExit(
        "extract_event_fields no longer exposes the explicit S03 raw keep-site:\n"
        + extract_block
    )
if "Honest raw S03 keep-site" not in extract_block:
    raise SystemExit(
        "extract_event_fields keep-site comment drifted; future agents lose the boundary cue:\n"
        + extract_block
    )

print("raw keep-list ok")
PY
  then
    fail_with_log "python keep-list sweep" "raw keep-list drifted" "$log_path"
  fi
}

run_expect_success e2e_m033_s02 cargo test -p meshc --test e2e_m033_s02 -- --nocapture
run_expect_success fmt_mesher cargo run -q -p meshc -- fmt --check mesher
run_expect_success build_mesher cargo run -q -p meshc -- build mesher
run_python_check raw_keep_list

echo "verify-m033-s02: ok"
