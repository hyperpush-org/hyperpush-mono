#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m047-s04"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
RETAINED_ARTIFACTS_DIR="$ARTIFACT_DIR/retained-m047-s04-artifacts"

rm -rf "$ARTIFACT_DIR"
mkdir -p "$ARTIFACT_DIR"
exec > >(tee "$ARTIFACT_DIR/full-contract.log") 2>&1

: >"$PHASE_REPORT_PATH"
printf 'running\n' >"$STATUS_PATH"
printf 'init\n' >"$CURRENT_PHASE_PATH"

on_exit() {
  local exit_code=$?
  if [[ $exit_code -eq 0 ]]; then
    printf 'ok\n' >"$STATUS_PATH"
    printf 'complete\n' >"$CURRENT_PHASE_PATH"
  elif [[ ! -f "$STATUS_PATH" || "$(<"$STATUS_PATH")" != "failed" ]]; then
    printf 'failed\n' >"$STATUS_PATH"
  fi
}
trap on_exit EXIT

record_phase() {
  printf '%s\t%s\n' "$1" "$2" >>"$PHASE_REPORT_PATH"
}

print_log_excerpt() {
  local log_path="$1"
  python3 - "$log_path" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
if not path.exists():
    print(f"missing log: {path}")
    raise SystemExit(0)
lines = path.read_text(errors="replace").splitlines()
for line in lines[:220]:
    print(line)
if len(lines) > 220:
    print(f"... truncated after 220 lines (total {len(lines)})")
PY
}

fail_phase() {
  local phase="$1"
  local reason="$2"
  local log_path="${3:-}"
  local artifact_hint="${4:-}"
  printf 'failed\n' >"$STATUS_PATH"
  printf '%s\n' "$phase" >"$CURRENT_PHASE_PATH"
  echo "verification drift: ${reason}" >&2
  if [[ -n "$artifact_hint" ]]; then
    echo "artifact hint: ${artifact_hint}" >&2
  fi
  if [[ -n "$log_path" ]]; then
    echo "failing log: ${log_path}" >&2
    echo "--- ${log_path} ---" >&2
    print_log_excerpt "$log_path" >&2
  fi
  exit 1
}

assert_file_contains_regex() {
  local phase="$1"
  local path="$2"
  local regex="$3"
  local description="$4"
  local log_path="${5:-}"
  if ! python3 - "$path" "$regex" "$description" >"$ARTIFACT_DIR/${phase}.content-check.log" 2>&1 <<'PY'
from pathlib import Path
import re
import sys

path = Path(sys.argv[1])
regex = sys.argv[2]
description = sys.argv[3]
text = path.read_text(errors="replace")
if not re.search(regex, text, re.MULTILINE):
    raise SystemExit(f"{description}: missing regex {regex!r} in {path}")
print(f"{description}: matched {regex!r}")
PY
  then
    fail_phase "$phase" "$description" "$ARTIFACT_DIR/${phase}.content-check.log" "${log_path:-$path}"
  fi
}

assert_file_lacks_regex() {
  local phase="$1"
  local path="$2"
  local regex="$3"
  local description="$4"
  if ! python3 - "$path" "$regex" "$description" >"$ARTIFACT_DIR/${phase}.content-check.log" 2>&1 <<'PY'
from pathlib import Path
import re
import sys

path = Path(sys.argv[1])
regex = sys.argv[2]
description = sys.argv[3]
text = path.read_text(errors="replace")
match = re.search(regex, text, re.MULTILINE)
if match:
    raise SystemExit(
        f"{description}: matched forbidden regex {regex!r} in {path} at {match.start()}..{match.end()}"
    )
print(f"{description}: no match for {regex!r}")
PY
  then
    fail_phase "$phase" "$description" "$ARTIFACT_DIR/${phase}.content-check.log" "$path"
  fi
}

assert_path_absent() {
  local phase="$1"
  local path="$2"
  local description="$3"
  local log_path="$ARTIFACT_DIR/${phase}.path-check.log"
  if [[ -e "$path" ]]; then
    printf '%s\n' "${description}: ${path} still exists" >"$log_path"
    fail_phase "$phase" "$description" "$log_path" "$path"
  fi
}

run_command() {
  local timeout_secs="$1"
  local log_path="$2"
  shift 2
  local -a cmd=("$@")
  {
    printf '$'
    printf ' %q' "${cmd[@]}"
    printf '\n'
    "${cmd[@]}"
  } >"$log_path" 2>&1 &
  local cmd_pid=$!
  local deadline=$((SECONDS + timeout_secs))
  while kill -0 "$cmd_pid" 2>/dev/null; do
    if (( SECONDS >= deadline )); then
      echo "command timed out after ${timeout_secs}s" >>"$log_path"
      kill -TERM "$cmd_pid" 2>/dev/null || true
      sleep 1
      kill -KILL "$cmd_pid" 2>/dev/null || true
      wait "$cmd_pid" 2>/dev/null || true
      return 124
    fi
    sleep 1
  done
  wait "$cmd_pid"
}

assert_test_filter_ran() {
  local phase="$1"
  local log_path="$2"
  local label="$3"
  if ! python3 - "$log_path" "$label" >"$ARTIFACT_DIR/${label}.test-count.log" 2>&1 <<'PY'
import re
import sys
from pathlib import Path

text = Path(sys.argv[1]).read_text(errors="replace")
label = sys.argv[2]
counts = [int(value) for value in re.findall(r"running (\d+) test", text)]
if not counts:
    raise SystemExit(f"{label}: missing 'running N test' line")
if max(counts) <= 0:
    raise SystemExit(f"{label}: test filter ran 0 tests")
print(f"{label}: running-counts={counts}")
PY
  then
    fail_phase "$phase" "named test filter ran 0 tests or produced malformed output" "$ARTIFACT_DIR/${label}.test-count.log"
  fi
}

run_expect_success() {
  local phase="$1"
  local label="$2"
  local require_tests="$3"
  local timeout_secs="$4"
  shift 4
  local -a cmd=("$@")
  local log_path="$ARTIFACT_DIR/${label}.log"
  record_phase "$phase" started
  printf '%s\n' "$phase" >"$CURRENT_PHASE_PATH"
  echo "==> ${cmd[*]}"
  if ! run_command "$timeout_secs" "$log_path" "${cmd[@]}"; then
    record_phase "$phase" failed
    fail_phase "$phase" "expected success within ${timeout_secs}s" "$log_path"
  fi
  if [[ "$require_tests" == "yes" ]]; then
    assert_test_filter_ran "$phase" "$log_path" "$label"
  fi
  record_phase "$phase" passed
}

capture_snapshot() {
  local source_root="$1"
  local snapshot_path="$2"
  python3 - "$source_root" "$snapshot_path" <<'PY'
from pathlib import Path
import sys

source_root = Path(sys.argv[1])
snapshot_path = Path(sys.argv[2])
names = []
if source_root.exists():
    names = sorted(
        path.name
        for path in source_root.iterdir()
        if path.is_dir() and path.name != 'verify'
    )
snapshot_path.write_text(''.join(f"{name}\n" for name in names))
PY
}

copy_new_artifacts_or_fail() {
  local phase="$1"
  local before_snapshot="$2"
  local source_root="$3"
  local dest_root="$4"
  local manifest_path="$5"

  if ! python3 - "$before_snapshot" "$source_root" "$dest_root" >"$manifest_path" 2>"$ARTIFACT_DIR/${phase}.artifact-check.log" <<'PY'
from pathlib import Path
import shutil
import sys

before_snapshot = Path(sys.argv[1])
source_root = Path(sys.argv[2])
dest_root = Path(sys.argv[3])

before = {
    line.strip()
    for line in before_snapshot.read_text(errors='replace').splitlines()
    if line.strip()
}
after_paths = {
    path.name: path
    for path in source_root.iterdir()
    if path.is_dir() and path.name != 'verify'
}
new_names = sorted(name for name in after_paths if name not in before)
if not new_names:
    raise SystemExit('expected fresh .tmp/m047-s04 artifact directories from the M047 cutover e2e replay')

if dest_root.exists():
    shutil.rmtree(dest_root)
dest_root.mkdir(parents=True, exist_ok=True)
manifest_lines = []
for name in new_names:
    src = after_paths[name]
    if not any(src.iterdir()):
        raise SystemExit(f'{src}: expected non-empty artifact directory')
    dst = dest_root / name
    shutil.copytree(src, dst)
    manifest_lines.append(f'{name}\t{src}')
    for child in sorted(src.rglob('*')):
        if child.is_file():
            manifest_lines.append(f'  - {child}')

print('\n'.join(manifest_lines))
PY
  then
    fail_phase "$phase" "missing or malformed copied evidence" "$ARTIFACT_DIR/${phase}.artifact-check.log" "$dest_root"
  fi
}

assert_retained_bundle_shape() {
  local phase="$1"
  local bundle_root="$2"
  local manifest_path="$3"
  local pointer_path="$4"
  local log_path="$ARTIFACT_DIR/${phase}.bundle-check.log"
  if ! python3 - "$bundle_root" "$manifest_path" "$pointer_path" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import json
import sys

bundle_root = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
pointer_path = Path(sys.argv[3])
expected_pointer = str(bundle_root)
actual_pointer = pointer_path.read_text(errors='replace').strip()
if actual_pointer != expected_pointer:
    raise SystemExit(
        f'latest-proof-bundle pointer drifted: expected {expected_pointer!r}, got {actual_pointer!r}'
    )
manifest_lines = [line for line in manifest_path.read_text(errors='replace').splitlines() if line.strip()]
if not manifest_lines:
    raise SystemExit(f'{manifest_path}: expected non-empty copied-artifact manifest')

children = sorted(path for path in bundle_root.iterdir() if path.is_dir())
if len(children) < 3:
    raise SystemExit(f'{bundle_root}: expected at least three retained contract snapshots')

required_prefixes = [
    'cutover-verifier-contract-',
    'cutover-docs-contract-',
    'cutover-runbook-contract-',
]
for prefix in required_prefixes:
    matches = [path for path in children if path.name.startswith(prefix)]
    if len(matches) != 1:
        raise SystemExit(f'{bundle_root}: expected exactly one retained bundle for {prefix}, found {[path.name for path in matches]}')

verifier_dir = next(path for path in children if path.name.startswith('cutover-verifier-contract-'))
if not (verifier_dir / 'scenario-meta.json').is_file():
    raise SystemExit(f'{verifier_dir}: missing scenario-meta.json')
scenario = json.loads((verifier_dir / 'scenario-meta.json').read_text())
if scenario.get('authoritative_verifier') != 'scripts/verify-m047-s04.sh':
    raise SystemExit(f'{verifier_dir}/scenario-meta.json: authoritative verifier drifted: {scenario!r}')
for rel in [
    'contract/verify-m047-s04.sh',
    'contract/verify-m045-s04.sh',
    'contract/verify-m045-s05.sh',
    'contract/verify-m046-s04.sh',
    'contract/verify-m046-s05.sh',
    'contract/verify-m046-s06.sh',
    'contract/docs.vitepress.config.mts',
    'contract/README.md',
    'contract/distributed-proof.index.md',
]:
    if not (verifier_dir / rel).exists():
        raise SystemExit(f'{verifier_dir}: missing retained contract file {rel}')

print('retained-bundle-shape: ok')
PY
  then
    fail_phase "$phase" "missing retained proof artifacts or malformed bundle pointer" "$log_path" "$bundle_root"
  fi
}

record_phase contract-guards started
printf 'contract-guards\n' >"$CURRENT_PHASE_PATH"
assert_path_absent \
  contract-root-tiny-cluster \
  tiny-cluster \
  'repo root still contains the retired tiny-cluster proof package directory'
assert_path_absent \
  contract-root-cluster-proof \
  cluster-proof \
  'repo root still contains the retired cluster-proof proof package directory'

assert_file_contains_regex \
  contract-sidebar-proof-surfaces \
  website/docs/.vitepress/config.mts \
  "text: 'Proof Surfaces'" \
  'docs sidebar lost the public-secondary proof group'
assert_file_contains_regex \
  contract-sidebar-distributed-proof-link \
  website/docs/.vitepress/config.mts \
  "link: '/docs/distributed-proof/'" \
  'docs sidebar lost the Distributed Proof public link'
assert_file_contains_regex \
  contract-sidebar-production-proof-link \
  website/docs/.vitepress/config.mts \
  "link: '/docs/production-backend-proof/'" \
  'docs sidebar lost the Production Backend Proof public link'
assert_file_contains_regex \
  contract-sidebar-proof-footer-opt-out \
  website/docs/.vitepress/config.mts \
  'includeInFooter: false' \
  'docs sidebar lost the proof-page footer opt-out marker'

assert_file_contains_regex \
  contract-readme-todo-postgres \
  README.md \
  'examples/todo-postgres/README\.md' \
  'README lost the public PostgreSQL starter reference'
assert_file_contains_regex \
  contract-readme-todo-sqlite \
  README.md \
  'examples/todo-sqlite/README\.md' \
  'README lost the public SQLite starter reference'
assert_file_contains_regex \
  contract-readme-reference-backend \
  README.md \
  'reference-backend/README\.md' \
  'README lost the deeper backend proof reference'
assert_file_contains_regex \
  contract-readme-distributed-proof-link \
  README.md \
  'https://meshlang\.dev/docs/distributed-proof/' \
  'README lost the public distributed-proof link'
assert_file_contains_regex \
  contract-readme-production-proof-link \
  README.md \
  'https://meshlang\.dev/docs/production-backend-proof/' \
  'README lost the public production-proof link'
assert_file_lacks_regex \
  contract-readme-proof-rail-commands \
  README.md \
  'scripts/verify-m047-s04\.sh|scripts/verify-m047-s06\.sh|scripts/verify-m046-s06\.sh|scripts/verify-m046-s05\.sh|scripts/verify-m046-s04\.sh|scripts/verify-m045-s05\.sh|scripts/verify-m045-s04\.sh|scripts/verify-m045-s03\.sh' \
  'README still presents retained proof-rail commands as first-contact onboarding text'
assert_file_lacks_regex \
  contract-readme-stale-fixtures \
  README.md \
  'tiny-cluster/README\.md|cluster-proof/README\.md' \
  'README still presents retained proof fixtures as public onboarding surfaces'

CANONICAL_PROOF_SURFACE='website/docs/docs/distributed-proof/index.md'
assert_file_contains_regex \
  contract-distributed-proof-canonical-marker \
  "$CANONICAL_PROOF_SURFACE" \
  'only public-secondary docs page that carries the named clustered verifier rails' \
  'Distributed Proof lost the canonical clustered proof-map marker'
assert_file_contains_regex \
  contract-distributed-proof-clustered-example-page \
  "$CANONICAL_PROOF_SURFACE" \
  '/docs/getting-started/clustered-example/' \
  'Distributed Proof lost the Clustered Example handoff'
assert_file_contains_regex \
  contract-distributed-proof-production-proof-page \
  "$CANONICAL_PROOF_SURFACE" \
  '/docs/production-backend-proof/' \
  'Distributed Proof lost the Production Backend Proof handoff'
assert_file_contains_regex \
  contract-distributed-proof-s04 \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m047-s04.sh' \
  'Distributed Proof lost the S04 cutover rail reference'
assert_file_contains_regex \
  contract-distributed-proof-s05 \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m047-s05.sh' \
  'Distributed Proof lost the S05 historical Todo subrail reference'
assert_file_contains_regex \
  contract-distributed-proof-s06 \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m047-s06.sh' \
  'Distributed Proof lost the S06 closeout rail reference'
assert_file_contains_regex \
  contract-distributed-proof-s07 \
  "$CANONICAL_PROOF_SURFACE" \
  'e2e_m047_s07' \
  'Distributed Proof lost the repo S07 clustered-route rail handoff'
assert_file_contains_regex \
  contract-distributed-proof-fly \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m043-s04-fly.sh' \
  'Distributed Proof lost the read-only Fly verifier handoff'
assert_file_contains_regex \
  contract-distributed-proof-historical-m046-closeout \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m046-s06.sh' \
  'Distributed Proof lost the retained M046 closeout alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m046-equal \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m046-s05.sh' \
  'Distributed Proof lost the retained M046 equal-surface alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m046-package \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m046-s04.sh' \
  'Distributed Proof lost the retained M046 package alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m045-closeout \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m045-s05.sh' \
  'Distributed Proof lost the retained M045 closeout alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m045-assembled \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m045-s04.sh' \
  'Distributed Proof lost the retained M045 assembled alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m045-failover \
  "$CANONICAL_PROOF_SURFACE" \
  'scripts/verify-m045-s03.sh' \
  'Distributed Proof lost the retained M045 failover subrail'
assert_file_lacks_regex \
  contract-distributed-proof-stale-fixtures \
  "$CANONICAL_PROOF_SURFACE" \
  'tiny-cluster/README\.md|cluster-proof/README\.md' \
  'Distributed Proof still treats retained proof fixtures as public onboarding surfaces'

for surface in \
  website/docs/docs/distributed/index.md \
  website/docs/docs/tooling/index.md; do
  safe_name="$(printf '%s' "$surface" | tr '/.' '__')"
  assert_file_contains_regex \
    "${safe_name}-clustered-example-page" \
    "$surface" \
    '/docs/getting-started/clustered-example/' \
    "$surface lost the Clustered Example handoff"
  assert_file_contains_regex \
    "${safe_name}-distributed-proof-page" \
    "$surface" \
    '/docs/distributed-proof/' \
    "$surface lost the Distributed Proof handoff"
  assert_file_contains_regex \
    "${safe_name}-production-proof-page" \
    "$surface" \
    '/docs/production-backend-proof/' \
    "$surface lost the Production Backend Proof handoff"
  assert_file_contains_regex \
    "${safe_name}-todo-postgres" \
    "$surface" \
    'examples/todo-postgres/README\.md' \
    "$surface lost the public PostgreSQL starter reference"
  assert_file_contains_regex \
    "${safe_name}-todo-sqlite" \
    "$surface" \
    'examples/todo-sqlite/README\.md' \
    "$surface lost the public SQLite starter reference"
  assert_file_contains_regex \
    "${safe_name}-reference-backend" \
    "$surface" \
    'reference-backend/README\.md' \
    "$surface lost the deeper backend proof reference"
  assert_file_contains_regex \
    "${safe_name}-starter-split-local" \
    "$surface" \
    'honest local' \
    "$surface lost the honest local starter split"
  assert_file_contains_regex \
    "${safe_name}-starter-split-shared" \
    "$surface" \
    'shared/deployable' \
    "$surface lost the shared/deployable starter split"
  assert_file_lacks_regex \
    "${safe_name}-proof-ledger" \
    "$surface" \
    'scripts/verify-m047-s04\.sh|scripts/verify-m047-s05\.sh|scripts/verify-m047-s06\.sh|e2e_m047_s07|scripts/verify-m043-s04-fly\.sh|scripts/verify-m046-s06\.sh|scripts/verify-m046-s05\.sh|scripts/verify-m046-s04\.sh|scripts/verify-m045-s05\.sh|scripts/verify-m045-s04\.sh|scripts/verify-m045-s03\.sh' \
    "$surface still duplicates the named clustered proof ledger instead of handing off to Distributed Proof"
  assert_file_lacks_regex \
    "${safe_name}-stale-fixtures" \
    "$surface" \
    'tiny-cluster/README\.md|cluster-proof/README\.md' \
    "$surface still treats retained proof fixtures as public onboarding surfaces"
done

CLUSTERED_EXAMPLE_SURFACE='website/docs/docs/getting-started/clustered-example/index.md'
CLUSTERED_EXAMPLE_SAFE_NAME="$(printf '%s' "$CLUSTERED_EXAMPLE_SURFACE" | tr '/.' '__')"
assert_file_contains_regex \
  "${CLUSTERED_EXAMPLE_SAFE_NAME}-todo-postgres" \
  "$CLUSTERED_EXAMPLE_SURFACE" \
  'examples/todo-postgres/README\.md' \
  "$CLUSTERED_EXAMPLE_SURFACE lost the public PostgreSQL starter reference"
assert_file_contains_regex \
  "${CLUSTERED_EXAMPLE_SAFE_NAME}-todo-sqlite" \
  "$CLUSTERED_EXAMPLE_SURFACE" \
  'examples/todo-sqlite/README\.md' \
  "$CLUSTERED_EXAMPLE_SURFACE lost the public SQLite starter reference"
assert_file_contains_regex \
  "${CLUSTERED_EXAMPLE_SAFE_NAME}-reference-backend" \
  "$CLUSTERED_EXAMPLE_SURFACE" \
  'reference-backend/README\.md' \
  "$CLUSTERED_EXAMPLE_SURFACE lost the deeper backend proof reference"
assert_file_contains_regex \
  "${CLUSTERED_EXAMPLE_SAFE_NAME}-distributed-proof-page" \
  "$CLUSTERED_EXAMPLE_SURFACE" \
  '/docs/distributed-proof/' \
  "$CLUSTERED_EXAMPLE_SURFACE lost the secondary distributed-proof page handoff"
assert_file_lacks_regex \
  "${CLUSTERED_EXAMPLE_SAFE_NAME}-direct-proof-rails" \
  "$CLUSTERED_EXAMPLE_SURFACE" \
  'scripts/verify-m047-s04\.sh|scripts/verify-m047-s05\.sh|scripts/verify-m047-s06\.sh|e2e_m047_s07|scripts/verify-m043-s04-fly\.sh|scripts/verify-m046-s06\.sh|scripts/verify-m046-s05\.sh|scripts/verify-m046-s04\.sh|scripts/verify-m045-s05\.sh|scripts/verify-m045-s04\.sh|scripts/verify-m045-s03\.sh' \
  "$CLUSTERED_EXAMPLE_SURFACE still presents retained proof-rail commands as first-contact text"
assert_file_lacks_regex \
  "${CLUSTERED_EXAMPLE_SAFE_NAME}-stale-fixtures" \
  "$CLUSTERED_EXAMPLE_SURFACE" \
  'tiny-cluster/README\.md|cluster-proof/README\.md' \
  "$CLUSTERED_EXAMPLE_SURFACE still treats retained proof fixtures as public onboarding surfaces"

assert_file_contains_regex \
  contract-todo-postgres-init \
  examples/todo-postgres/README.md \
  'meshc init --template todo-api --db postgres' \
  'todo-postgres README lost the explicit PostgreSQL starter command'
assert_file_contains_regex \
  contract-todo-postgres-clustered-source \
  examples/todo-postgres/README.md \
  '@cluster pub fn sync_todos\(\)' \
  'todo-postgres README lost the source-first clustered work marker'
assert_file_contains_regex \
  contract-todo-postgres-clustered-route \
  examples/todo-postgres/README.md \
  'HTTP\.clustered\(1, \.\.\.\)' \
  'todo-postgres README lost the bounded clustered-route marker'
assert_file_contains_regex \
  contract-todo-postgres-health \
  examples/todo-postgres/README.md \
  'GET /health' \
  'todo-postgres README lost the local health-route boundary marker'
assert_file_contains_regex \
  contract-todo-postgres-cluster-status \
  examples/todo-postgres/README.md \
  'meshc cluster status' \
  'todo-postgres README lost the runtime-owned operator surface marker'
assert_file_contains_regex \
  contract-todo-postgres-env \
  examples/todo-postgres/README.md \
  'DATABASE_URL' \
  'todo-postgres README lost the shared Postgres env marker'
assert_file_lacks_regex \
  contract-todo-postgres-old-runbook \
  examples/todo-postgres/README.md \
  'tiny-cluster/README\.md|cluster-proof/README\.md|clustered\(work\)' \
  'todo-postgres README still points at retired proof fixtures or legacy clustered(work) wording'
assert_file_contains_regex \
  contract-todo-sqlite-init \
  examples/todo-sqlite/README.md \
  'meshc init --template todo-api --db sqlite' \
  'todo-sqlite README lost the explicit SQLite starter command'
assert_file_contains_regex \
  contract-todo-sqlite-tests \
  examples/todo-sqlite/README.md \
  'meshc test \.' \
  'todo-sqlite README lost the generated package-test marker'
assert_file_contains_regex \
  contract-todo-sqlite-health \
  examples/todo-sqlite/README.md \
  'GET /health' \
  'todo-sqlite README lost the local health-route marker'
assert_file_contains_regex \
  contract-todo-sqlite-db-path \
  examples/todo-sqlite/README.md \
  'TODO_DB_PATH' \
  'todo-sqlite README lost the SQLite env marker'
assert_file_contains_regex \
  contract-todo-sqlite-postgres-branch \
  examples/todo-sqlite/README.md \
  'meshc init --template todo-api --db postgres' \
  'todo-sqlite README lost the explicit Postgres branch link'
assert_file_contains_regex \
  contract-todo-sqlite-clustered-branch \
  examples/todo-sqlite/README.md \
  'meshc init --clustered' \
  'todo-sqlite README lost the scaffold-first clustered branch link'
assert_file_lacks_regex \
  contract-todo-sqlite-old-clustered \
  examples/todo-sqlite/README.md \
  'tiny-cluster/README\.md|cluster-proof/README\.md|@cluster pub fn sync_todos\(\)|meshc cluster continuity|HTTP\.clustered\(1, \.\.\.\)' \
  'todo-sqlite README still claims clustered/public-proof behavior'
record_phase contract-guards passed

run_expect_success m050-s01-onboarding-graph 00-m050-s01-onboarding-graph no 1800 \
  node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs

run_expect_success m047-s04-parser 00-m047-s04-parser yes 2400 \
  cargo test -p mesh-parser m047_s04 -- --nocapture
run_expect_success m047-s04-pkg 01-m047-s04-pkg yes 2400 \
  cargo test -p mesh-pkg m047_s04 -- --nocapture
run_expect_success m047-s04-compiler 02-m047-s04-compiler yes 3600 \
  cargo test -p meshc --test e2e_m047_s01 -- --nocapture
run_expect_success m047-s04-scaffold-unit 03-m047-s04-scaffold-unit yes 1800 \
  cargo test -p mesh-pkg scaffold_clustered_project_writes_public_cluster_contract -- --nocapture
run_expect_success m047-s04-scaffold-smoke 04-m047-s04-scaffold-smoke yes 2400 \
  cargo test -p meshc --test tooling_e2e test_init_clustered_creates_project -- --nocapture
run_expect_success m047-s04-tiny-cluster-tests 05-m047-s04-tiny-cluster-tests no 1800 \
  cargo run -q -p meshc -- test scripts/fixtures/clustered/tiny-cluster/tests
run_expect_success m047-s04-tiny-cluster-build 06-m047-s04-tiny-cluster-build no 1800 \
  cargo run -q -p meshc -- build scripts/fixtures/clustered/tiny-cluster
run_expect_success m047-s04-cluster-proof-tests 07-m047-s04-cluster-proof-tests no 1800 \
  cargo run -q -p meshc -- test scripts/fixtures/clustered/cluster-proof/tests
run_expect_success m047-s04-cluster-proof-build 08-m047-s04-cluster-proof-build no 1800 \
  cargo run -q -p meshc -- build scripts/fixtures/clustered/cluster-proof
run_expect_success m047-s04-docs-build 09-m047-s04-docs-build no 2400 \
  npm --prefix website run build

BEFORE_SNAPSHOT="$ARTIFACT_DIR/10-m047-s04.before.txt"
capture_snapshot .tmp/m047-s04 "$BEFORE_SNAPSHOT"
run_expect_success m047-s04-e2e 10-m047-s04-e2e yes 2400 \
  cargo test -p meshc --test e2e_m047_s04 -- --nocapture
record_phase m047-s04-artifacts started
copy_new_artifacts_or_fail \
  m047-s04-artifacts \
  "$BEFORE_SNAPSHOT" \
  .tmp/m047-s04 \
  "$RETAINED_ARTIFACTS_DIR" \
  "$ARTIFACT_DIR/10-m047-s04-artifacts.txt"
printf '%s\n' "$RETAINED_ARTIFACTS_DIR" >"$LATEST_PROOF_BUNDLE_PATH"
record_phase m047-s04-artifacts passed
record_phase m047-s04-bundle-shape started
assert_retained_bundle_shape \
  m047-s04-bundle-shape \
  "$RETAINED_ARTIFACTS_DIR" \
  "$ARTIFACT_DIR/10-m047-s04-artifacts.txt" \
  "$LATEST_PROOF_BUNDLE_PATH"
record_phase m047-s04-bundle-shape passed

for expected_phase in \
  contract-guards \
  m050-s01-onboarding-graph \
  m047-s04-parser \
  m047-s04-pkg \
  m047-s04-compiler \
  m047-s04-scaffold-unit \
  m047-s04-scaffold-smoke \
  m047-s04-tiny-cluster-tests \
  m047-s04-tiny-cluster-build \
  m047-s04-cluster-proof-tests \
  m047-s04-cluster-proof-build \
  m047-s04-docs-build \
  m047-s04-e2e \
  m047-s04-artifacts \
  m047-s04-bundle-shape; do
  if ! rg -q "^${expected_phase}\\tpassed$" "$PHASE_REPORT_PATH"; then
    fail_phase verifier-status "missing ${expected_phase} pass marker" "$ARTIFACT_DIR/full-contract.log" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m047-s04: ok"
