#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_DIR=".tmp/m033-s03/verify"
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

queries = Path("mesher/storage/queries.mpl").read_text()


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


def assert_no_whole_raw(name: str) -> None:
    block = fn_block(queries, name)
    body = code_only(block)
    for token in ("Repo.query_raw", "Repo.execute_raw", "Query.select_raw"):
        if token in body:
            raise SystemExit(f"{name} regressed to {token}:\n{block}")


allowed_s03_keep_sites = {
    "list_issues_filtered",
    "event_volume_hourly",
    "check_volume_spikes",
    "extract_event_fields",
    "get_event_neighbors",
    "get_event_alert_rules",
    "list_alerts",
    "get_threshold_rules",
    "should_fire_by_cooldown",
    "evaluate_threshold_rule",
    "check_sample_rate",
}

all_function_names = set()
for line in queries.splitlines():
    if line.startswith("pub fn "):
        name = line[len("pub fn ") :].split("(", 1)[0].strip()
        all_function_names.add(name)

raw_site_functions = set()
for name in all_function_names:
    block = fn_block(queries, name)
    body = code_only(block)
    if "Repo.query_raw" in body or "Repo.execute_raw" in body or "Query.select_raw" in body:
        raw_site_functions.add(name)

unexpected = sorted(raw_site_functions - allowed_s03_keep_sites)
if unexpected:
    details = []
    for name in unexpected:
        details.append(f"unexpected raw keep-site: {name}\n--- function ---\n{fn_block(queries, name)}")
    raise SystemExit("\n\n".join(details))

missing = sorted(name for name in allowed_s03_keep_sites if name not in raw_site_functions)
if missing:
    raise SystemExit("expected named S03 raw keep-sites missing raw boundary: " + ", ".join(missing))

for name in [
    "count_unresolved_issues",
    "get_issue_project_id",
    "get_project_by_api_key",
    "validate_session",
    "list_issues_by_status",
    "search_events_fulltext",
    "filter_events_by_tag",
    "list_events_for_issue",
    "error_breakdown_by_level",
    "top_issues_by_frequency",
    "event_breakdown_by_tag",
    "project_health_summary",
    "get_event_detail",
    "get_members_with_users",
    "list_api_keys",
    "list_alert_rules",
    "check_new_issue",
    "get_all_project_retention",
    "get_project_storage",
    "get_project_settings",
]:
    assert_no_whole_raw(name)

print("s03 raw keep-list ok")
PY
  then
    fail_with_log "python keep-list sweep" "S03 raw keep-list drifted" "$log_path"
  fi
}

run_expect_success e2e_m033_s03 cargo test -p meshc --test e2e_m033_s03 -- --nocapture
run_expect_success fmt_mesher cargo run -q -p meshc -- fmt --check mesher
run_expect_success build_mesher cargo run -q -p meshc -- build mesher
run_python_check raw_keep_list

echo "verify-m033-s03: ok"
