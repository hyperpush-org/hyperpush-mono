#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m047-s06"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
RETAINED_M047_S05_VERIFY_DIR="$ARTIFACT_DIR/retained-m047-s05-verify"
RETAINED_M047_S05_BUNDLE_POINTER_PATH="$ARTIFACT_DIR/retained-m047-s05-latest-proof-bundle.txt"
RETAINED_M047_S06_ARTIFACTS_DIR="$ARTIFACT_DIR/retained-m047-s06-artifacts"
RETAINED_PROOF_BUNDLE_DIR="$ARTIFACT_DIR/retained-proof-bundle"

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
    fail_phase "$phase" "$description" "$ARTIFACT_DIR/${phase}.content-check.log" "$path"
  fi
}

assert_file_omits_regex() {
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
if re.search(regex, text, re.MULTILINE):
    raise SystemExit(f"{description}: unexpected regex {regex!r} found in {path}")
print(f"{description}: omitted {regex!r}")
PY
  then
    fail_phase "$phase" "$description" "$ARTIFACT_DIR/${phase}.content-check.log" "$path"
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
    raise SystemExit('expected fresh .tmp/m047-s06 artifact directories from the S06 docs/verifier contract replay')

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
    fail_phase "$phase" "missing or malformed copied S06 contract artifacts" "$ARTIFACT_DIR/${phase}.artifact-check.log" "$dest_root"
  fi
}

retain_delegated_verify_or_fail() {
  local phase="$1"
  local source_dir="$2"
  local log_path="$ARTIFACT_DIR/${phase}.log"
  record_phase "$phase" started
  printf '%s\n' "$phase" >"$CURRENT_PHASE_PATH"

  if [[ ! -d "$source_dir" ]]; then
    printf 'missing delegated verify dir: %s\n' "$source_dir" >"$log_path"
    record_phase "$phase" failed
    fail_phase "$phase" "missing delegated verify directory" "$log_path"
  fi

  rm -rf "$RETAINED_M047_S05_VERIFY_DIR"
  cp -R "$source_dir" "$RETAINED_M047_S05_VERIFY_DIR" >"$log_path" 2>&1 || {
    record_phase "$phase" failed
    fail_phase "$phase" "failed to retain delegated verify directory" "$log_path" "$source_dir"
  }

  for required in status.txt current-phase.txt phase-report.txt full-contract.log latest-proof-bundle.txt; do
    if [[ ! -f "$RETAINED_M047_S05_VERIFY_DIR/$required" ]]; then
      printf 'missing retained delegated file: %s\n' "$RETAINED_M047_S05_VERIFY_DIR/$required" >"$log_path"
      record_phase "$phase" failed
      fail_phase "$phase" "delegated S05 verifier retention is malformed" "$log_path" "$RETAINED_M047_S05_VERIFY_DIR"
    fi
  done

  if [[ "$(<"$RETAINED_M047_S05_VERIFY_DIR/status.txt")" != "ok" ]]; then
    printf 'delegated verifier status drifted: %s\n' "$(<"$RETAINED_M047_S05_VERIFY_DIR/status.txt")" >"$log_path"
    record_phase "$phase" failed
    fail_phase "$phase" "delegated S05 verifier did not finish successfully" "$log_path" "$RETAINED_M047_S05_VERIFY_DIR/status.txt"
  fi

  if [[ "$(<"$RETAINED_M047_S05_VERIFY_DIR/current-phase.txt")" != "complete" ]]; then
    printf 'delegated verifier current-phase drifted: %s\n' "$(<"$RETAINED_M047_S05_VERIFY_DIR/current-phase.txt")" >"$log_path"
    record_phase "$phase" failed
    fail_phase "$phase" "delegated S05 verifier did not reach complete phase" "$log_path" "$RETAINED_M047_S05_VERIFY_DIR/current-phase.txt"
  fi

  for expected_phase in \
    m047-s04-replay \
    retain-m047-s04-verify \
    m047-s05-pkg \
    m047-s05-tooling \
    m047-s05-e2e \
    m047-s05-docs-build \
    retain-m047-s05-artifacts \
    m047-s05-bundle-shape; do
    if ! rg -q "^${expected_phase}\\tpassed$" "$RETAINED_M047_S05_VERIFY_DIR/phase-report.txt"; then
      printf 'delegated phase report missing %s pass marker\n' "$expected_phase" >"$log_path"
      record_phase "$phase" failed
      fail_phase "$phase" "delegated S05 verifier phase report drifted" "$log_path" "$RETAINED_M047_S05_VERIFY_DIR/phase-report.txt"
    fi
  done

  local delegated_bundle_path retained_bundle_name retained_bundle_path
  delegated_bundle_path="$(<"$RETAINED_M047_S05_VERIFY_DIR/latest-proof-bundle.txt")"
  if [[ -z "$delegated_bundle_path" ]]; then
    printf 'delegated latest-proof-bundle pointer was empty\n' >"$log_path"
    record_phase "$phase" failed
    fail_phase "$phase" "delegated S05 verifier bundle pointer was empty" "$log_path" "$RETAINED_M047_S05_VERIFY_DIR/latest-proof-bundle.txt"
  fi

  retained_bundle_name="$(basename "$delegated_bundle_path")"
  retained_bundle_path="$RETAINED_M047_S05_VERIFY_DIR/$retained_bundle_name"
  if [[ ! -d "$retained_bundle_path" ]]; then
    printf 'retained delegated bundle directory missing: %s\n' "$retained_bundle_path" >"$log_path"
    record_phase "$phase" failed
    fail_phase "$phase" "delegated S05 verifier bundle directory was not retained" "$log_path" "$retained_bundle_path"
  fi

  printf '%s\n' "$retained_bundle_path" >"$RETAINED_M047_S05_BUNDLE_POINTER_PATH"
  record_phase "$phase" passed
}

assert_retained_bundle_shape() {
  local phase="$1"
  local bundle_root="$2"
  local manifest_path="$3"
  local pointer_path="$4"
  local delegated_pointer_path="$5"
  local log_path="$ARTIFACT_DIR/${phase}.bundle-check.log"
  if ! python3 - "$bundle_root" "$manifest_path" "$pointer_path" "$delegated_pointer_path" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import sys

bundle_root = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
pointer_path = Path(sys.argv[3])
delegated_pointer_path = Path(sys.argv[4])

expected_pointer = str(bundle_root)
actual_pointer = pointer_path.read_text(errors='replace').strip()
if actual_pointer != expected_pointer:
    raise SystemExit(
        f'latest-proof-bundle pointer drifted: expected {expected_pointer!r}, got {actual_pointer!r}'
    )

manifest_lines = [line for line in manifest_path.read_text(errors='replace').splitlines() if line.strip()]
if not manifest_lines:
    raise SystemExit(f'{manifest_path}: expected non-empty copied-artifact manifest')

retained_verify = bundle_root / 'retained-m047-s05-verify'
if not retained_verify.is_dir():
    raise SystemExit(f'{bundle_root}: missing retained-m047-s05-verify')
for rel in ['status.txt', 'current-phase.txt', 'phase-report.txt', 'full-contract.log', 'latest-proof-bundle.txt']:
    if not (retained_verify / rel).is_file():
        raise SystemExit(f'{retained_verify}: missing {rel}')

retained_s06_artifacts = bundle_root / 'retained-m047-s06-artifacts'
if not retained_s06_artifacts.is_dir():
    raise SystemExit(f'{bundle_root}: missing retained-m047-s06-artifacts')

artifact_children = sorted(path.name for path in retained_s06_artifacts.iterdir() if path.is_dir())
required_prefixes = [
    'docs-authority-contract-',
    'rail-layering-contract-',
    'verifier-contract-',
]
for prefix in required_prefixes:
    matches = [name for name in artifact_children if name.startswith(prefix)]
    if len(matches) != 1:
        raise SystemExit(f'{retained_s06_artifacts}: expected exactly one retained artifact for {prefix}, found {matches}')

retained_delegate_pointer = delegated_pointer_path.read_text(errors='replace').strip()
if not retained_delegate_pointer:
    raise SystemExit(f'{delegated_pointer_path}: expected retained delegated bundle pointer')
if not Path(retained_delegate_pointer).is_dir():
    raise SystemExit(f'{delegated_pointer_path}: retained delegated bundle path does not exist: {retained_delegate_pointer}')
if retained_verify not in Path(retained_delegate_pointer).parents and Path(retained_delegate_pointer) != retained_verify:
    raise SystemExit(
        f'{delegated_pointer_path}: retained delegated bundle path must live inside {retained_verify}, got {retained_delegate_pointer}'
    )

print('retained-bundle-shape: ok')
PY
  then
    fail_phase "$phase" "missing retained proof artifacts or malformed bundle pointer" "$log_path" "$bundle_root"
  fi
}

record_phase contract-guards started
printf 'contract-guards\n' >"$CURRENT_PHASE_PATH"
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
assert_file_contains_regex \
  contract-readme-scaffold \
  README.md \
  'meshc init --clustered' \
  'README lost the scaffold-first clustered entrypoint'
assert_file_omits_regex \
  contract-readme-proof-rail-commands \
  README.md \
  'scripts/verify-m047-s04\.sh|scripts/verify-m047-s05\.sh|scripts/verify-m047-s06\.sh|e2e_m047_s07|tiny-cluster/README\.md|cluster-proof/README\.md' \
  'README still presents retained proof rails as first-contact onboarding text'
assert_file_omits_regex \
  contract-readme-stale-generic-template \
  README.md \
  'meshc init --template todo-api(?! --db (sqlite|postgres))' \
  'README still uses the unsplit generic todo template command'
assert_file_omits_regex \
  contract-readme-stale-sqlite-clustered \
  README.md \
  'adding a SQLite HTTP app|local SQLite/HTTP routes plus explicit-count `HTTP\.clustered\(1, \.\.\.\)`' \
  'README still presents the SQLite starter as part of the clustered wrapper story'
assert_file_omits_regex \
  contract-readme-stale-non-goal \
  README.md \
  'HTTP\.clustered\(\.\.\.\) is still not shipped' \
  'README still claims HTTP.clustered(...) is unshipped'

assert_file_contains_regex \
  contract-distributed-proof-canonical-marker \
  website/docs/docs/distributed-proof/index.md \
  'only public-secondary docs page that carries the named clustered verifier rails' \
  'Distributed Proof lost the canonical clustered proof-map marker'
assert_file_contains_regex \
  contract-distributed-proof-clustered-example-page \
  website/docs/docs/distributed-proof/index.md \
  '/docs/getting-started/clustered-example/' \
  'Distributed Proof lost the Clustered Example handoff'
assert_file_contains_regex \
  contract-distributed-proof-production-proof-page \
  website/docs/docs/distributed-proof/index.md \
  '/docs/production-backend-proof/' \
  'Distributed Proof lost the Production Backend Proof handoff'
assert_file_contains_regex \
  contract-distributed-proof-s04 \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m047-s04.sh' \
  'Distributed Proof lost the S04 cutover rail reference'
assert_file_contains_regex \
  contract-distributed-proof-s05 \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m047-s05.sh' \
  'Distributed Proof lost the S05 historical Todo subrail reference'
assert_file_contains_regex \
  contract-distributed-proof-s06 \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m047-s06.sh' \
  'Distributed Proof lost the S06 closeout rail reference'
assert_file_contains_regex \
  contract-distributed-proof-s07 \
  website/docs/docs/distributed-proof/index.md \
  'e2e_m047_s07' \
  'Distributed Proof lost the repo S07 clustered-route rail handoff'
assert_file_contains_regex \
  contract-distributed-proof-fly \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m043-s04-fly.sh' \
  'Distributed Proof lost the read-only Fly verifier handoff'
assert_file_contains_regex \
  contract-distributed-proof-current-repo-postgres \
  website/docs/docs/distributed-proof/index.md \
  'https://github\.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README\.md' \
  'Distributed Proof lost the current PostgreSQL starter repo URL'
assert_file_contains_regex \
  contract-distributed-proof-current-repo-sqlite \
  website/docs/docs/distributed-proof/index.md \
  'https://github\.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README\.md' \
  'Distributed Proof lost the current SQLite starter repo URL'
assert_file_contains_regex \
  contract-distributed-proof-current-repo-reference-backend \
  website/docs/docs/distributed-proof/index.md \
  'https://github\.com/snowdamiz/mesh-lang/blob/main/reference-backend/README\.md' \
  'Distributed Proof lost the current backend runbook repo URL'
assert_file_contains_regex \
  contract-distributed-proof-sqlite-template \
  website/docs/docs/distributed-proof/index.md \
  'meshc init --template todo-api --db sqlite' \
  'Distributed Proof lost the explicit SQLite starter command'
assert_file_contains_regex \
  contract-distributed-proof-postgres-template \
  website/docs/docs/distributed-proof/index.md \
  'meshc init --template todo-api --db postgres' \
  'Distributed Proof lost the explicit PostgreSQL starter command'
assert_file_contains_regex \
  contract-distributed-proof-historical-m046-closeout \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m046-s06.sh' \
  'Distributed Proof lost the retained M046 closeout alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m046-equal \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m046-s05.sh' \
  'Distributed Proof lost the retained M046 equal-surface alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m046-package \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m046-s04.sh' \
  'Distributed Proof lost the retained M046 package alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m045-closeout \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m045-s05.sh' \
  'Distributed Proof lost the retained M045 closeout alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m045-assembled \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m045-s04.sh' \
  'Distributed Proof lost the retained M045 assembled alias'
assert_file_contains_regex \
  contract-distributed-proof-historical-m045-failover \
  website/docs/docs/distributed-proof/index.md \
  'scripts/verify-m045-s03.sh' \
  'Distributed Proof lost the retained M045 failover subrail'
assert_file_omits_regex \
  contract-distributed-proof-stale-fixture-runbooks \
  website/docs/docs/distributed-proof/index.md \
  'tiny-cluster/README\.md|cluster-proof/README\.md' \
  'Distributed Proof still presents retained fixture runbooks as public onboarding surfaces'
assert_file_omits_regex \
  contract-distributed-proof-stale-repo \
  website/docs/docs/distributed-proof/index.md \
  'https://github\.com/hyperpush-org/hyperpush-mono/blob/main/' \
  'Distributed Proof still points at the stale hyperpush repo identity'

for surface in \
  website/docs/docs/tooling/index.md \
  website/docs/docs/distributed/index.md; do
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
    "${safe_name}-current-repo-postgres" \
    "$surface" \
    'https://github\.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README\.md' \
    "$surface lost the current PostgreSQL starter repo URL"
  assert_file_contains_regex \
    "${safe_name}-current-repo-sqlite" \
    "$surface" \
    'https://github\.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README\.md' \
    "$surface lost the current SQLite starter repo URL"
  assert_file_contains_regex \
    "${safe_name}-current-repo-reference-backend" \
    "$surface" \
    'https://github\.com/snowdamiz/mesh-lang/blob/main/reference-backend/README\.md' \
    "$surface lost the current backend runbook repo URL"
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
  assert_file_omits_regex \
    "${safe_name}-proof-ledger" \
    "$surface" \
    'scripts/verify-m047-s04\.sh|scripts/verify-m047-s05\.sh|scripts/verify-m047-s06\.sh|e2e_m047_s07|scripts/verify-m043-s04-fly\.sh|scripts/verify-m046-s06\.sh|scripts/verify-m046-s05\.sh|scripts/verify-m046-s04\.sh|scripts/verify-m045-s05\.sh|scripts/verify-m045-s04\.sh|scripts/verify-m045-s03\.sh' \
    "$surface still duplicates the named clustered proof ledger instead of handing off to Distributed Proof"
  assert_file_omits_regex \
    "${safe_name}-stale-generic-template" \
    "$surface" \
    'meshc init --template todo-api(?! --db (sqlite|postgres))' \
    "$surface still uses the unsplit generic todo template command"
  assert_file_omits_regex \
    "${safe_name}-stale-fixture-runbooks" \
    "$surface" \
    'tiny-cluster/README\.md|cluster-proof/README\.md' \
    "$surface still presents retained fixture runbooks as public onboarding surfaces"
  assert_file_omits_regex \
    "${safe_name}-stale-sqlite-clustered" \
    "$surface" \
    'adding a SQLite HTTP app|local SQLite/HTTP routes plus explicit-count `HTTP\.clustered\(1, \.\.\.\)`' \
    "$surface still presents the SQLite starter as part of the clustered wrapper story"
  assert_file_omits_regex \
    "${safe_name}-stale-non-goal" \
    "$surface" \
    'HTTP\.clustered\(\.\.\.\) is still not shipped' \
    "$surface still claims HTTP.clustered(...) is unshipped"
  assert_file_omits_regex \
    "${safe_name}-stale-repo" \
    "$surface" \
    'https://github\.com/hyperpush-org/hyperpush-mono/blob/main/' \
    "$surface still points at the stale hyperpush repo identity"
done
assert_file_contains_regex \
  contract-clustered-example-scaffold \
  website/docs/docs/getting-started/clustered-example/index.md \
  'meshc init --clustered' \
  'Clustered Example lost the scaffold-first clustered entrypoint'
assert_file_contains_regex \
  contract-clustered-example-sqlite-template \
  website/docs/docs/getting-started/clustered-example/index.md \
  'meshc init --template todo-api --db sqlite my_local_todo' \
  'Clustered Example lost the honest local starter split'
assert_file_contains_regex \
  contract-clustered-example-postgres-template \
  website/docs/docs/getting-started/clustered-example/index.md \
  'meshc init --template todo-api --db postgres my_shared_todo' \
  'Clustered Example lost the serious shared/deployable starter split'
assert_file_contains_regex \
  contract-clustered-example-current-repo-postgres \
  website/docs/docs/getting-started/clustered-example/index.md \
  'https://github\.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README\.md' \
  'Clustered Example lost the current PostgreSQL starter repo URL'
assert_file_contains_regex \
  contract-clustered-example-current-repo-sqlite \
  website/docs/docs/getting-started/clustered-example/index.md \
  'https://github\.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README\.md' \
  'Clustered Example lost the current SQLite starter repo URL'
assert_file_contains_regex \
  contract-clustered-example-current-repo-reference-backend \
  website/docs/docs/getting-started/clustered-example/index.md \
  'https://github\.com/snowdamiz/mesh-lang/blob/main/reference-backend/README\.md' \
  'Clustered Example lost the current backend runbook repo URL'
assert_file_contains_regex \
  contract-clustered-example-proof-page \
  website/docs/docs/getting-started/clustered-example/index.md \
  '/docs/distributed-proof/' \
  'Clustered Example lost the proof-page handoff'
assert_file_omits_regex \
  contract-clustered-example-direct-rails \
  website/docs/docs/getting-started/clustered-example/index.md \
  'scripts/verify-m047-s04\.sh|scripts/verify-m047-s05\.sh|scripts/verify-m047-s06\.sh|e2e_m047_s07|scripts/verify-m043-s04-fly\.sh|scripts/verify-m046-s06\.sh|scripts/verify-m046-s05\.sh|scripts/verify-m046-s04\.sh|scripts/verify-m045-s05\.sh|scripts/verify-m045-s04\.sh|scripts/verify-m045-s03\.sh' \
  'Clustered Example still inlines retained proof-rail commands instead of handing off to the proof page'
assert_file_omits_regex \
  contract-clustered-example-helper-names \
  website/docs/docs/getting-started/clustered-example/index.md \
  'execute_declared_work\(\.\.\.\)|Work\.execute_declared_work' \
  'Clustered Example still inlines the helper-shaped migration names'
assert_file_omits_regex \
  contract-clustered-example-stale-repo \
  website/docs/docs/getting-started/clustered-example/index.md \
  'https://github\.com/hyperpush-org/hyperpush-mono/blob/main/' \
  'Clustered Example still points at the stale hyperpush repo identity'
assert_file_contains_regex \
  contract-distributed-proof-prev \
  website/docs/docs/distributed-proof/index.md \
  'prev: false' \
  'Distributed Proof lost the footer prev opt-out marker'
assert_file_contains_regex \
  contract-distributed-proof-next \
  website/docs/docs/distributed-proof/index.md \
  'next: false' \
  'Distributed Proof lost the footer next opt-out marker'
record_phase contract-guards passed

run_expect_success m050-s01-onboarding-graph 00-m050-s01-onboarding-graph no 1800 \
  node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs

run_expect_success m047-s05-replay 00-m047-s05-replay no 7200 \
  bash scripts/verify-m047-s05.sh
retain_delegated_verify_or_fail retain-m047-s05-verify .tmp/m047-s05/verify

BEFORE_SNAPSHOT="$ARTIFACT_DIR/10-m047-s06.before.txt"
capture_snapshot "$ARTIFACT_ROOT" "$BEFORE_SNAPSHOT"
run_expect_success m047-s06-e2e 10-e2e-m047-s06 yes 2400 \
  cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture
run_expect_success m047-s06-docs-build 11-m047-s06-docs-build no 2400 \
  npm --prefix website run build

record_phase m047-s06-artifacts started
copy_new_artifacts_or_fail \
  m047-s06-artifacts \
  "$BEFORE_SNAPSHOT" \
  "$ARTIFACT_ROOT" \
  "$RETAINED_M047_S06_ARTIFACTS_DIR" \
  "$ARTIFACT_DIR/retained-m047-s06-artifacts.manifest.txt"
record_phase m047-s06-artifacts passed

record_phase m047-s06-bundle-shape started
rm -rf "$RETAINED_PROOF_BUNDLE_DIR"
mkdir -p "$RETAINED_PROOF_BUNDLE_DIR"
cp -R "$RETAINED_M047_S05_VERIFY_DIR" "$RETAINED_PROOF_BUNDLE_DIR/retained-m047-s05-verify"
cp -R "$RETAINED_M047_S06_ARTIFACTS_DIR" "$RETAINED_PROOF_BUNDLE_DIR/retained-m047-s06-artifacts"
retained_m047_s05_bundle_name="$(basename "$(<"$RETAINED_PROOF_BUNDLE_DIR/retained-m047-s05-verify/latest-proof-bundle.txt")")"
printf '%s\n' "$RETAINED_PROOF_BUNDLE_DIR/retained-m047-s05-verify/$retained_m047_s05_bundle_name" >"$RETAINED_M047_S05_BUNDLE_POINTER_PATH"
printf '%s\n' "$RETAINED_PROOF_BUNDLE_DIR" >"$LATEST_PROOF_BUNDLE_PATH"
assert_retained_bundle_shape \
  m047-s06-bundle-shape \
  "$RETAINED_PROOF_BUNDLE_DIR" \
  "$ARTIFACT_DIR/retained-m047-s06-artifacts.manifest.txt" \
  "$LATEST_PROOF_BUNDLE_PATH" \
  "$RETAINED_M047_S05_BUNDLE_POINTER_PATH"
record_phase m047-s06-bundle-shape passed

for expected_phase in \
  contract-guards \
  m050-s01-onboarding-graph \
  m047-s05-replay \
  retain-m047-s05-verify \
  m047-s06-e2e \
  m047-s06-docs-build \
  m047-s06-artifacts \
  m047-s06-bundle-shape; do
  if ! rg -q "^${expected_phase}\\tpassed$" "$PHASE_REPORT_PATH"; then
    fail_phase verifier-status "missing ${expected_phase} pass marker" "$ARTIFACT_DIR/full-contract.log" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m047-s06: ok"
