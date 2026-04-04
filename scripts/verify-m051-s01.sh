#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m051-s01"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
RETAINED_M051_S01_ARTIFACTS_DIR="$ARTIFACT_DIR/retained-m051-s01-artifacts"
RETAINED_PROOF_BUNDLE_DIR="$ARTIFACT_DIR/retained-proof-bundle"
RUNBOOK_FILE="mesher/README.md"
ENV_FILE="mesher/.env.example"
CONFIG_FILE="mesher/config.mpl"
MAIN_FILE="mesher/main.mpl"
AUTH_FILE="mesher/ingestion/auth.mpl"
SEED_FILE="mesher/migrations/20260226000000_seed_default_org.mpl"
E2E_FILE="compiler/meshc/tests/e2e_m051_s01.rs"
VERIFIER_FILE="scripts/verify-m051-s01.sh"

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

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    fail_phase init "required command missing from PATH: ${command_name}"
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
    raise SystemExit('expected fresh .tmp/m051-s01 artifact directories from the Mesher e2e replay')

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

sys.stdout.write('\n'.join(manifest_lines) + ('\n' if manifest_lines else ''))
PY
  then
    fail_phase "$phase" "expected fresh retained Mesher artifacts from e2e_m051_s01" "$ARTIFACT_DIR/${phase}.artifact-check.log" "$source_root"
  fi
}

run_contract_checks() {
  local log_path="$1"
  python3 - "$ROOT_DIR" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
runbook = root / 'mesher/README.md'
env_file = root / 'mesher/.env.example'
config = root / 'mesher/config.mpl'
main = root / 'mesher/main.mpl'
auth = root / 'mesher/ingestion/auth.mpl'
seed = root / 'mesher/migrations/20260226000000_seed_default_org.mpl'
e2e = root / 'compiler/meshc/tests/e2e_m051_s01.rs'
verifier = root / 'scripts/verify-m051-s01.sh'

texts = {
    'runbook': runbook.read_text(errors='replace'),
    'env': env_file.read_text(errors='replace'),
    'config': config.read_text(errors='replace'),
    'main': main.read_text(errors='replace'),
    'auth': auth.read_text(errors='replace'),
    'seed': seed.read_text(errors='replace'),
    'e2e': e2e.read_text(errors='replace'),
    'verifier': verifier.read_text(errors='replace'),
}


def require_contains(label: str, needle: str, description: str) -> None:
    if needle not in texts[label]:
        raise SystemExit(f"{description}: missing {needle!r} in {label}")


def require_not_contains(label: str, needle: str, description: str) -> None:
    if needle in texts[label]:
        raise SystemExit(f"{description}: stale {needle!r} still present in {label}")


def require_order(label: str, needles: list[str], description: str) -> None:
    current_index = -1
    for needle in needles:
        index = texts[label].find(needle)
        if index == -1:
            raise SystemExit(f"{description}: missing {needle!r} in {label}")
        if index <= current_index:
            raise SystemExit(f"{description}: expected {needle!r} after the prior ordered marker in {label}")
        current_index = index

required_runbook_markers = [
    'This README is the canonical maintainer runbook',
    '## Startup contract',
    '## Seeded development data',
    '## Repo-root maintainer loop',
    '## Live seed-event smoke',
    '## Runtime inspection',
    '## Authoritative proof rail',
    'Node.start_from_env()',
    'mesher/.env.example',
    './mesher/mesher',
    'bash scripts/verify-m051-s01.sh',
]
for needle in required_runbook_markers:
    require_contains('runbook', needle, 'maintainer runbook marker')

require_order(
    'runbook',
    [
        '## Startup contract',
        '## Seeded development data',
        '## Repo-root maintainer loop',
        '## Live seed-event smoke',
        '## Runtime inspection',
        '## Authoritative proof rail',
    ],
    'runbook section order',
)

canonical_commands = [
    'cargo run -q -p meshc -- test mesher/tests',
    'DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate mesher status',
    'DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate mesher up',
    'cargo run -q -p meshc -- build mesher',
    './mesher/mesher',
    'curl -sSf http://127.0.0.1:8080/api/v1/projects/default/settings',
    'http://127.0.0.1:8080/api/v1/events',
    'http://127.0.0.1:8080/api/v1/projects/default/issues?status=unresolved',
    'http://127.0.0.1:8080/api/v1/projects/default/storage',
    'cargo test -p meshc --test e2e_m051_s01 -- --nocapture',
]
for needle in canonical_commands:
    require_contains('runbook', needle, 'maintainer runbook command')

all_env_keys = [
    'DATABASE_URL',
    'PORT',
    'MESHER_WS_PORT',
    'MESHER_RATE_LIMIT_WINDOW_SECONDS',
    'MESHER_RATE_LIMIT_MAX_EVENTS',
    'MESH_CLUSTER_COOKIE',
    'MESH_NODE_NAME',
    'MESH_DISCOVERY_SEED',
    'MESH_CLUSTER_PORT',
    'MESH_CONTINUITY_ROLE',
    'MESH_CONTINUITY_PROMOTION_EPOCH',
]
for key in all_env_keys:
    require_contains('runbook', key, 'runbook env contract')
    require_contains('env', key, '.env example env contract')
    require_contains('e2e', key, 'e2e env contract')

for key in [
    'DATABASE_URL',
    'PORT',
    'MESHER_WS_PORT',
    'MESHER_RATE_LIMIT_WINDOW_SECONDS',
    'MESHER_RATE_LIMIT_MAX_EVENTS',
]:
    require_contains('config', key, 'Mesher config env contract')

route_contract_pairs = [
    ('/api/v1/events', '/api/v1/events', 'event ingest route'),
    ('/api/v1/projects/default/settings', '/api/v1/projects/:project_id/settings', 'settings readiness route'),
    ('/api/v1/projects/default/issues?status=unresolved', '/api/v1/projects/:project_id/issues', 'issues readback route'),
    ('/api/v1/projects/default/storage', '/api/v1/projects/:project_id/storage', 'storage readback route'),
]
for runbook_route, source_route, description in route_contract_pairs:
    require_contains('runbook', runbook_route, description)
    require_contains('main', source_route, description)
    require_contains('e2e', runbook_route, description)

require_contains('auth', 'x-sentry-auth', 'auth header contract')
require_contains('runbook', 'x-sentry-auth', 'auth header contract')
require_contains('e2e', 'x-sentry-auth', 'auth header contract')

seed_markers = [
    'default',
    'dev-default',
    'mshr_devdefaultapikey000000000000000000000000000',
]
for needle in seed_markers:
    require_contains('runbook', needle, 'seeded smoke marker')
    require_contains('seed', needle, 'seed migration marker')
    require_contains('e2e', needle, 'e2e seeded smoke marker')

for needle in [
    'reference-backend/README.md',
    'reference-backend/scripts/verify-production-proof-surface.sh',
    'GET /health',
    'POST /jobs',
    'GET /jobs/:id',
    'JOB_POLL_MS',
    'meshc init --template',
    'meshlang.dev/install',
    'website/docs',
]:
    require_not_contains('runbook', needle, 'maintainer-only boundary')

for needle in [
    'cargo run -q -p meshc -- test mesher/tests',
    'cargo run -q -p meshc -- build mesher',
    'cargo test -p meshc --test e2e_m051_s01 -- --nocapture',
    'm051-s01-package-tests',
    'm051-s01-build',
    'm051-s01-contract',
    'm051-s01-e2e',
    'retain-m051-s01-artifacts',
    'm051-s01-bundle-shape',
    'latest-proof-bundle.txt',
    'phase-report.txt',
    'status.txt',
    'current-phase.txt',
    'full-contract.log',
    'retained-m051-s01-artifacts',
    'verify-m051-s01: ok',
]:
    require_contains('verifier', needle, 'verifier contract marker')

expected_run_expect_success = {
    'm051-s01-package-tests': 'cargo run -q -p meshc -- test mesher/tests',
    'm051-s01-build': 'cargo run -q -p meshc -- build mesher',
    'm051-s01-e2e': 'cargo test -p meshc --test e2e_m051_s01 -- --nocapture',
}
actual_run_expect_success = {}
for line_index, line in enumerate(texts['verifier'].splitlines()):
    stripped = line.strip()
    if not stripped.startswith('run_expect_success '):
        continue
    parts = stripped.split()
    if len(parts) < 6:
        raise SystemExit(f'verifier replay line is malformed: {line!r}')
    phase = parts[1]
    if not line.rstrip().endswith('\\'):
        raise SystemExit(f'verifier replay line no longer uses the expected continuation form: {line!r}')
    command_index = line_index + 1
    if command_index >= len(texts['verifier'].splitlines()):
        raise SystemExit(f'verifier replay line {line!r} is missing its command continuation')
    command_line = texts['verifier'].splitlines()[command_index].strip()
    actual_run_expect_success[phase] = command_line

if set(actual_run_expect_success) != set(expected_run_expect_success):
    raise SystemExit(
        f'verifier replay phases drifted: expected {sorted(expected_run_expect_success)}, got {sorted(actual_run_expect_success)}'
    )
for phase, expected_command in expected_run_expect_success.items():
    actual_command = actual_run_expect_success[phase]
    if actual_command != expected_command:
        raise SystemExit(
            f'verifier replay command drifted for {phase}: expected {expected_command!r}, got {actual_command!r}'
        )

print('m051-s01 maintainer contract: ok')
PY
}

assert_retained_bundle_shape() {
  local phase="$1"
  local bundle_root="$2"
  local manifest_path="$3"
  local pointer_path="$4"
  local log_path="$ARTIFACT_DIR/${phase}.bundle-check.log"
  if ! python3 - "$bundle_root" "$manifest_path" "$pointer_path" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
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

for relative in [
    'mesher.README.md',
    'mesher.env.example',
    'verify-m051-s01.sh',
    'e2e_m051_s01.rs',
    'retained-m051-s01-artifacts',
]:
    if not (bundle_root / relative).exists():
        raise SystemExit(f'{bundle_root}: missing required retained file {relative}')

retained_root = bundle_root / 'retained-m051-s01-artifacts'
children = sorted(path for path in retained_root.iterdir() if path.is_dir())
if not children:
    raise SystemExit(f'{retained_root}: expected copied e2e artifact directories')


def find_one(prefix: str) -> Path:
    matches = [path for path in children if path.name.startswith(prefix)]
    if not matches:
        raise SystemExit(f'{retained_root}: missing copied artifact directory with prefix {prefix!r}')
    if len(matches) != 1:
        raise SystemExit(
            f'{retained_root}: expected exactly one copied artifact directory with prefix {prefix!r}, found {[path.name for path in matches]}'
        )
    return matches[0]

missing_database = find_one('mesher-missing-database-url-')
runtime_truth = find_one('mesher-postgres-runtime-truth-')

for relative in [
    'bin/mesher',
    'build-output.json',
    'build.meta.txt',
    'build.stderr.log',
    'build.stdout.log',
    'missing-database-url.meta.txt',
    'missing-database-url.stderr.log',
    'missing-database-url.stdout.log',
]:
    if not (missing_database / relative).exists():
        raise SystemExit(f'{missing_database}: missing required retained file {relative}')

for relative in [
    'bin/mesher',
    'build-output.json',
    'migrate-up.meta.txt',
    'migrate-up.stderr.log',
    'migrate-up.stdout.log',
    'postgres.inspect.json',
    'postgres.logs.txt',
    'postgres.meta.json',
    'seed-project.json',
    'seed-api-key.json',
    'project-settings-ready.http',
    'project-settings-ready.json',
    'events-ingest-accepted.http',
    'events-ingest-accepted.json',
    'project-issues-readback.http',
    'project-issues-readback.json',
    'project-storage-after-ingest.http',
    'project-storage-after-ingest.json',
    'runtime.stdout.log',
    'runtime.stderr.log',
]:
    if not (runtime_truth / relative).exists():
        raise SystemExit(f'{runtime_truth}: missing required retained file {relative}')

print('retained bundle shape: ok')
PY
  then
    fail_phase "$phase" "retained bundle pointer or artifact shape drifted" "$log_path" "$bundle_root"
  fi
}

record_phase init started
require_command cargo
require_command python3
require_command rg
record_phase init passed

run_expect_success m051-s01-package-tests m051-s01-package-tests no 1800 \
  cargo run -q -p meshc -- test mesher/tests
run_expect_success m051-s01-build m051-s01-build no 1800 \
  cargo run -q -p meshc -- build mesher

record_phase m051-s01-contract started
printf '%s\n' 'm051-s01-contract' >"$CURRENT_PHASE_PATH"
if ! run_contract_checks "$ARTIFACT_DIR/m051-s01-contract.log"; then
  record_phase m051-s01-contract failed
  fail_phase m051-s01-contract "Mesher maintainer contract drifted" "$ARTIFACT_DIR/m051-s01-contract.log"
fi
record_phase m051-s01-contract passed

SNAPSHOT_BEFORE_E2E="$ARTIFACT_DIR/m051-s01-before.snapshot"
capture_snapshot "$ARTIFACT_ROOT" "$SNAPSHOT_BEFORE_E2E"
run_expect_success m051-s01-e2e m051-s01-e2e yes 3600 \
  cargo test -p meshc --test e2e_m051_s01 -- --nocapture

record_phase retain-m051-s01-artifacts started
copy_new_artifacts_or_fail \
  retain-m051-s01-artifacts \
  "$SNAPSHOT_BEFORE_E2E" \
  "$ARTIFACT_ROOT" \
  "$RETAINED_M051_S01_ARTIFACTS_DIR" \
  "$ARTIFACT_DIR/retained-m051-s01-artifacts.manifest.txt"
record_phase retain-m051-s01-artifacts passed

record_phase m051-s01-bundle-shape started
rm -rf "$RETAINED_PROOF_BUNDLE_DIR"
mkdir -p "$RETAINED_PROOF_BUNDLE_DIR"
cp "$RUNBOOK_FILE" "$RETAINED_PROOF_BUNDLE_DIR/mesher.README.md"
cp "$ENV_FILE" "$RETAINED_PROOF_BUNDLE_DIR/mesher.env.example"
cp "$VERIFIER_FILE" "$RETAINED_PROOF_BUNDLE_DIR/verify-m051-s01.sh"
cp "$E2E_FILE" "$RETAINED_PROOF_BUNDLE_DIR/e2e_m051_s01.rs"
cp -R "$RETAINED_M051_S01_ARTIFACTS_DIR" "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s01-artifacts"
printf '%s\n' "$RETAINED_PROOF_BUNDLE_DIR" >"$LATEST_PROOF_BUNDLE_PATH"
assert_retained_bundle_shape \
  m051-s01-bundle-shape \
  "$RETAINED_PROOF_BUNDLE_DIR" \
  "$ARTIFACT_DIR/retained-m051-s01-artifacts.manifest.txt" \
  "$LATEST_PROOF_BUNDLE_PATH"
record_phase m051-s01-bundle-shape passed

for expected_phase in \
  init \
  m051-s01-package-tests \
  m051-s01-build \
  m051-s01-contract \
  m051-s01-e2e \
  retain-m051-s01-artifacts \
  m051-s01-bundle-shape
  do
  if ! rg -Fq "${expected_phase}	passed" "$PHASE_REPORT_PATH"; then
    fail_phase final-phase-report "phase report missing passed marker for ${expected_phase}" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m051-s01: ok"
