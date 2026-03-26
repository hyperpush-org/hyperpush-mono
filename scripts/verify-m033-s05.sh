#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_DIR=".tmp/m033-s05/verify"
DOCS_FILE="website/docs/docs/databases/index.md"
mkdir -p "$ARTIFACT_DIR"

fail_with_log() {
  local phase_name="$1"
  local command_text="$2"
  local reason="$3"
  local log_path="${4:-}"

  echo "verification drift: ${reason}" >&2
  echo "first failing phase: ${phase_name}" >&2
  echo "failing command: ${command_text}" >&2
  if [[ -n "$log_path" && -f "$log_path" ]]; then
    echo "--- ${log_path} ---" >&2
    sed -n '1,320p' "$log_path" >&2
  fi
  exit 1
}

run_expect_success() {
  local phase_name="$1"
  local label="$2"
  shift 2
  local -a cmd=("$@")
  local log_path="$ARTIFACT_DIR/${label}.log"
  local command_text="${cmd[*]}"

  echo "==> [${phase_name}] ${command_text}"
  if ! "${cmd[@]}" >"$log_path" 2>&1; then
    fail_with_log "$phase_name" "$command_text" "expected success" "$log_path"
  fi
}

run_docs_truth_sweep() {
  local phase_name="docs-truth"
  local log_path="$ARTIFACT_DIR/02-docs-truth.log"
  local command_text="python3 docs truth sweep ${DOCS_FILE}"

  echo "==> [${phase_name}] ${command_text}"
  if ! python3 >"$log_path" 2>&1 <<'PY'
from pathlib import Path

text = Path("website/docs/docs/databases/index.md").read_text()
required_strings = [
    "a neutral expression/query/write surface built from `Expr`, `Query`, `Repo`, and `Migration`",
    "explicit PostgreSQL-only helpers under `Pg.*`",
    "a named set of raw escape hatches that stays honest instead of pretending every SQL shape is portable",
    "SQLite-specific extras are later work and are **not** the proof target for this page.",
    "M033's shipped API uses `Expr.label`, not `Expr.alias`.",
    "- `Expr.value(...)` — bind a literal or parameter",
    "- `Expr.column(...)` — refer to a column",
    "- `Expr.null()` — write an actual `NULL`",
    "- `Expr.case_when(...)` — keep SQL branching in the expression tree",
    "- `Expr.coalesce([...])` — express fallback/default logic",
    "- `Expr.label(expr, \"name\")` — name derived output columns",
    "The builder calls here are the neutral part: `Query.where_expr(...)`, `Query.select_expr(...)`, and `Query.select_exprs([...])`.",
    "### `Repo.insert_expr`, `Repo.update_where_expr`, and `Repo.insert_or_update_expr` accept expression-valued writes",
    "### `Migration.create_index(...)` is the neutral DDL path",
    "- `Pg.uuid(Expr.value(project_id))`",
    "- `Pg.timestamptz(Expr.fn_call(\"now\", []))`",
    "- `Pg.text(Expr.column(\"created_at\"))`",
    "- `Pg.cast(Expr.value(\"1024\"), \"bigint\")`",
    "Pg.to_tsvector",
    "Pg.plainto_tsquery",
    "Pg.tsvector_matches",
    "Pg.ts_rank",
    "Pg.jsonb_contains",
    "Pg.create_gin_index(pool, \"events\", \"idx_events_tags\", \"tags\", \"jsonb_path_ops\")",
    "Pg.crypt(Expr.value(password), Pg.gen_salt(\"bf\", 12))",
    "Pg.create_extension(pool, \"pgcrypto\")",
    "Pg.create_range_partitioned_table(pool, \"events\", [...], \"received_at\")",
    "Pg.create_daily_partitions_ahead(pool, \"events\", days)",
    "Pg.list_daily_partitions_before(pool, \"events\", max_days)",
    "Pg.drop_partition(pool, partition_name)",
    "The escape hatches are part of the public contract:",
    "- `Repo.query_raw(pool, sql, params)` — raw read boundary",
    "- `Repo.execute_raw(pool, sql, params)` — raw write/update boundary",
    "- `Migration.execute(pool, sql)` — raw DDL boundary when a migration shape does not fit `Migration.*` or `Pg.*`",
    "The current Mesher storage layer still keeps a named raw list in `mesher/storage/queries.mpl` instead of pretending those shapes are portable:",
    "`mesher/storage/writer.mpl` and `mesher/storage/queries.mpl`",
    "`mesher/migrations/20260216120000_create_initial_schema.mpl`",
    "`mesher/storage/schema.mpl`",
    "For the assembled docs + live-Postgres acceptance replay, run `bash scripts/verify-m033-s05.sh`.",
    "| assembled docs + live-Postgres proof replay | `bash scripts/verify-m033-s05.sh` | `website/docs/docs/databases/index.md`, `scripts/verify-m033-s02.sh`, `scripts/verify-m033-s03.sh`, `scripts/verify-m033-s04.sh` |",
    "bash scripts/verify-m033-s01.sh",
    "bash scripts/verify-m033-s02.sh",
    "bash scripts/verify-m033-s03.sh",
    "bash scripts/verify-m033-s04.sh",
    "bash scripts/verify-m033-s05.sh",
    "npm --prefix website run build",
]

missing = [needle for needle in required_strings if needle not in text]
if missing:
    raise SystemExit(
        "database docs truth drifted; missing exact strings:\n"
        + "\n".join(f"- {needle}" for needle in missing)
    )

print("database docs truth sweep ok")
PY
  then
    fail_with_log "$phase_name" "$command_text" "docs truth drifted" "$log_path"
  fi
}

run_expect_success "docs-build" "01-docs-build" npm --prefix website run build
run_docs_truth_sweep
run_expect_success "s02" "03-verify-m033-s02" bash scripts/verify-m033-s02.sh
run_expect_success "s03" "04-verify-m033-s03" bash scripts/verify-m033-s03.sh
run_expect_success "s04" "05-verify-m033-s04" bash scripts/verify-m033-s04.sh

echo "verify-m033-s05: ok"
