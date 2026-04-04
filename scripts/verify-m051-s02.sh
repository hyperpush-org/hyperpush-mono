#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m051-s02"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
CONTRACT_ARTIFACT_MANIFEST_PATH="$ARTIFACT_DIR/retained-contract-artifacts.manifest.txt"
RETAINED_PROOF_BUNDLE_DIR=""
CONTRACT_SNAPSHOT_PATH="$ARTIFACT_DIR/m051-s02-before.snapshot"
FIXTURE_RUNBOOK="scripts/fixtures/backend/reference-backend/README.md"
FIXTURE_STAGE_SCRIPT="scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh"
FIXTURE_APPLY_SCRIPT="scripts/fixtures/backend/reference-backend/scripts/apply-deploy-migrations.sh"
FIXTURE_DEPLOY_SMOKE_SCRIPT="scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh"
FIXTURE_SMOKE_SCRIPT="scripts/fixtures/backend/reference-backend/scripts/smoke.sh"
FIXTURE_TESTS_DIR="scripts/fixtures/backend/reference-backend/tests"
FIXTURE_RUNTIME_DIR="$ROOT_DIR/.tmp/m051-s02/reference-backend-runtime"
FIXTURE_SMOKE_DIR="$ROOT_DIR/.tmp/m051-s02/fixture-smoke"
E2E_CONTRACT_FILE="compiler/meshc/tests/e2e_m051_s02.rs"
E2E_RUNTIME_FILE="compiler/meshc/tests/e2e_reference_backend.rs"
COMPAT_RUNBOOK="reference-backend/README.md"
COMPAT_PROOF_SCRIPT="reference-backend/scripts/verify-production-proof-surface.sh"
DB_TEST_TARGET="cargo test -p meshc --test e2e_reference_backend"

repo_rel() {
  local candidate="$1"
  if [[ "$candidate" == "$ROOT_DIR/"* ]]; then
    printf '%s\n' "${candidate#$ROOT_DIR/}"
  else
    printf '%s\n' "$candidate"
  fi
}

rm -rf "$ARTIFACT_DIR"
mkdir -p "$ARTIFACT_DIR"
rm -rf "$FIXTURE_RUNTIME_DIR" "$FIXTURE_SMOKE_DIR"
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

begin_phase() {
  record_phase "$1" started
  printf '%s\n' "$1" >"$CURRENT_PHASE_PATH"
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
    echo "artifact hint: $(repo_rel "$artifact_hint")" >&2
  fi
  if [[ -n "$log_path" ]]; then
    echo "failing log: $(repo_rel "$log_path")" >&2
    echo "--- $(repo_rel "$log_path") ---" >&2
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

require_file() {
  local phase="$1"
  local path="$2"
  local description="$3"
  local artifact_hint="${4:-}"
  if [[ -f "$path" ]]; then
    return 0
  fi

  local log_path="$ARTIFACT_DIR/${phase}.preflight.log"
  {
    echo "preflight: missing required file"
    echo "description: ${description}"
    echo "path: $(repo_rel "$path")"
  } >"$log_path"
  record_phase "$phase" failed
  fail_phase "$phase" "missing required file: $(repo_rel "$path")" "$log_path" "$artifact_hint"
}

cleanup_fixture_smoke_processes() {
  local phase="$1"
  local log_path="$ARTIFACT_DIR/${phase}.cleanup.log"
  local pattern="$FIXTURE_SMOKE_DIR/build/reference-backend"
  : >"$log_path"

  if ! command -v pgrep >/dev/null 2>&1 || ! command -v pkill >/dev/null 2>&1; then
    echo "pgrep/pkill unavailable; skipping stale fixture-smoke cleanup" >>"$log_path"
    return 0
  fi

  if ! pgrep -f "$pattern" >/dev/null 2>&1; then
    echo "no stale fixture-smoke workers" >>"$log_path"
    return 0
  fi

  echo "stale fixture-smoke workers detected" >>"$log_path"
  pgrep -fal "$pattern" >>"$log_path" || true
  pkill -TERM -f "$pattern" >/dev/null 2>&1 || true
  sleep 1

  if pgrep -f "$pattern" >/dev/null 2>&1; then
    echo "fixture-smoke workers survived SIGTERM; sending SIGKILL" >>"$log_path"
    pgrep -fal "$pattern" >>"$log_path" || true
    pkill -KILL -f "$pattern" >/dev/null 2>&1 || true
    sleep 1
  fi

  if pgrep -f "$pattern" >/dev/null 2>&1; then
    echo "fixture-smoke workers still running after SIGKILL" >>"$log_path"
    pgrep -fal "$pattern" >>"$log_path" || true
    fail_phase "$phase" "stale retained fixture workers are still running" "$log_path" "$FIXTURE_SMOKE_DIR"
  fi

  echo "stale fixture-smoke workers cleaned" >>"$log_path"
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
  local count_log="$ARTIFACT_DIR/${label}.test-count.log"

  if ! python3 - "$log_path" "$label" >"$count_log" 2>&1 <<'PY'
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
    record_phase "$phase" failed
    fail_phase "$phase" "named test filter ran 0 tests or produced malformed output" "$count_log"
  fi
}

run_expect_success() {
  local phase="$1"
  local label="$2"
  local require_tests="$3"
  local timeout_secs="$4"
  local artifact_hint="$5"
  shift 5
  local -a cmd=("$@")
  local log_path="$ARTIFACT_DIR/${label}.log"

  begin_phase "$phase"
  echo "==> ${cmd[*]}"
  if ! run_command "$timeout_secs" "$log_path" "${cmd[@]}"; then
    record_phase "$phase" failed
    fail_phase "$phase" "expected success within ${timeout_secs}s" "$log_path" "$artifact_hint"
  fi
  if [[ "$require_tests" == "yes" ]]; then
    assert_test_filter_ran "$phase" "$log_path" "$label"
  fi
  record_phase "$phase" passed
}

capture_snapshot() {
  local source_root="$1"
  local snapshot_path="$2"
  shift 2
  python3 - "$source_root" "$snapshot_path" "$@" <<'PY'
from pathlib import Path
import sys

source_root = Path(sys.argv[1])
snapshot_path = Path(sys.argv[2])
ignored = set(sys.argv[3:])
names = []
if source_root.exists():
    names = sorted(
        path.name
        for path in source_root.iterdir()
        if path.is_dir() and path.name not in ignored
    )
snapshot_path.write_text(''.join(f"{name}\n" for name in names))
PY
}

copy_fixed_dir_or_fail() {
  local phase="$1"
  local source_dir="$2"
  local dest_dir="$3"
  local description="$4"
  shift 4
  local log_path="$ARTIFACT_DIR/${phase}.$(basename "$dest_dir").copy.log"

  if ! python3 - "$source_dir" "$dest_dir" "$description" "$@" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import shutil
import sys

source_dir = Path(sys.argv[1])
dest_dir = Path(sys.argv[2])
description = sys.argv[3]
required = sys.argv[4:]

if not source_dir.is_dir():
    raise SystemExit(f"{description}: missing source directory {source_dir}")
for rel in required:
    if not (source_dir / rel).exists():
        raise SystemExit(f"{description}: missing {rel} in {source_dir}")
if dest_dir.exists():
    shutil.rmtree(dest_dir)
dest_dir.parent.mkdir(parents=True, exist_ok=True)
shutil.copytree(source_dir, dest_dir, symlinks=True)
print(f"copied {source_dir} -> {dest_dir}")
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "$description" "$log_path" "$source_dir"
  fi
}

copy_new_prefixed_artifacts_or_fail() {
  local phase="$1"
  local before_snapshot="$2"
  local source_root="$3"
  local dest_root="$4"
  local manifest_path="$5"
  local expected_message="$6"
  shift 6
  local -a prefixes=("$@")
  local log_path="$ARTIFACT_DIR/${phase}.artifact-check.log"

  if ! python3 - "$before_snapshot" "$source_root" "$dest_root" "$manifest_path" "$expected_message" "${prefixes[@]}" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import shutil
import sys

before_snapshot = Path(sys.argv[1])
source_root = Path(sys.argv[2])
dest_root = Path(sys.argv[3])
manifest_path = Path(sys.argv[4])
expected_message = sys.argv[5]
prefixes = sys.argv[6:]

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
new_paths = {name: path for name, path in after_paths.items() if name not in before}
if not new_paths:
    raise SystemExit(expected_message)

selected = []
for prefix in prefixes:
    matches = [path for name, path in new_paths.items() if name.startswith(prefix)]
    if len(matches) != 1:
        raise SystemExit(
            f"expected exactly one fresh artifact with prefix {prefix!r}, found {[path.name for path in matches]}"
        )
    selected.append(matches[0])

if dest_root.exists():
    shutil.rmtree(dest_root)
dest_root.mkdir(parents=True, exist_ok=True)
manifest_lines = []
for src in selected:
    if not any(src.iterdir()):
        raise SystemExit(f"{src}: expected non-empty artifact directory")
    dst = dest_root / src.name
    shutil.copytree(src, dst, symlinks=True)
    manifest_lines.append(f"{src.name}\t{src}")
    for child in sorted(src.rglob('*')):
        if child.is_file():
            manifest_lines.append(f"  - {child}")

manifest_path.write_text('\n'.join(manifest_lines) + ('\n' if manifest_lines else ''))
print(f"copied {len(selected)} fresh prefixed artifact directories into {dest_root}")
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "missing or malformed copied artifacts" "$log_path" "$source_root"
  fi
}

pick_unused_port() {
  python3 - <<'PY'
import socket

with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

run_contract_checks() {
  local log_path="$1"
  python3 - "$ROOT_DIR" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
runbook = root / 'scripts/fixtures/backend/reference-backend/README.md'
stage = root / 'scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh'
apply = root / 'scripts/fixtures/backend/reference-backend/scripts/apply-deploy-migrations.sh'
deploy_smoke = root / 'scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh'
smoke = root / 'scripts/fixtures/backend/reference-backend/scripts/smoke.sh'
e2e_contract = root / 'compiler/meshc/tests/e2e_m051_s02.rs'
e2e_runtime = root / 'compiler/meshc/tests/e2e_reference_backend.rs'
verifier = root / 'scripts/verify-m051-s02.sh'
compat_runbook = root / 'reference-backend/README.md'
compat_verifier = root / 'reference-backend/scripts/verify-production-proof-surface.sh'

texts = {
    'runbook': runbook.read_text(errors='replace'),
    'stage': stage.read_text(errors='replace'),
    'apply': apply.read_text(errors='replace'),
    'deploy_smoke': deploy_smoke.read_text(errors='replace'),
    'smoke': smoke.read_text(errors='replace'),
    'e2e_contract': e2e_contract.read_text(errors='replace'),
    'e2e_runtime': e2e_runtime.read_text(errors='replace'),
    'verifier': verifier.read_text(errors='replace'),
    'compat_runbook': compat_runbook.read_text(errors='replace'),
    'compat_verifier': compat_verifier.read_text(errors='replace'),
}


def require_contains(label: str, needle: str, description: str) -> None:
    if needle not in texts[label]:
        raise SystemExit(f"{description}: missing {needle!r} in {label}")


def require_not_contains(label: str, needle: str, description: str) -> None:
    if needle in texts[label]:
        raise SystemExit(f"{description}: stale {needle!r} still present in {label}")


def require_order(label: str, needles: list[str], description: str) -> None:
    current = -1
    for needle in needles:
        index = texts[label].find(needle)
        if index == -1:
            raise SystemExit(f"{description}: missing {needle!r} in {label}")
        if index <= current:
            raise SystemExit(f"{description}: expected {needle!r} after the prior ordered marker in {label}")
        current = index

for needle in [
    'This README is the canonical maintainer runbook',
    'maintainer-only/internal fixture',
    '## Startup contract',
    '## Repo-root maintainer loop',
    '## Staged deploy bundle',
    '## Live runtime smoke',
    '## `/health` recovery interpretation',
    '## Authoritative proof rail',
    '## Compatibility boundary',
    'cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests',
    'DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate scripts/fixtures/backend/reference-backend status',
    'DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate scripts/fixtures/backend/reference-backend up',
    'DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} PORT=18080 JOB_POLL_MS=500 bash scripts/fixtures/backend/reference-backend/scripts/smoke.sh',
    'tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir"',
    'bash "$bundle_dir/apply-deploy-migrations.sh" "$bundle_dir/reference-backend.up.sql"',
    'BASE_URL=http://127.0.0.1:18080 \\\n  bash "$bundle_dir/deploy-smoke.sh"',
    'restart_count',
    'last_exit_reason',
    'recovered_jobs',
    'last_recovery_at',
    'last_recovery_job_id',
    'last_recovery_count',
    'recovery_active',
    'bash scripts/verify-m051-s02.sh',
    'reference-backend/README.md',
    'reference-backend/scripts/verify-production-proof-surface.sh',
]:
    require_contains('runbook', needle, 'retained runbook contract')

require_order(
    'runbook',
    [
        '## Startup contract',
        '## Repo-root maintainer loop',
        '## Staged deploy bundle',
        '## Live runtime smoke',
        '## `/health` recovery interpretation',
        '## Authoritative proof rail',
        '## Compatibility boundary',
    ],
    'retained runbook section order',
)

for needle in [
    'meshlang.dev/install',
    'meshc init --template',
    'website/docs/docs/production-backend-proof',
]:
    require_not_contains('runbook', needle, 'retained runbook first-contact drift')

for needle in [
    'PACKAGE_REL="scripts/fixtures/backend/reference-backend"',
    'required command missing from PATH: $command_name',
    'cargo run -q -p meshc -- build "$PACKAGE_REL" --output "$TARGET_BINARY"',
    'require_file "deploy SQL artifact" "$SOURCE_SQL"',
    'fixture source tree contains an in-place binary',
]:
    require_contains('stage', needle, 'stage-deploy contract')

for needle in [
    'psql is required but was not found on PATH',
    'DATABASE_URL must be set',
    'MIGRATION_VERSION="20260323010000"',
]:
    require_contains('apply', needle, 'apply-deploy contract')

for needle in [
    'required command missing from PATH: $command_name',
    'PORT must be a positive integer',
    'BASE_URL must start with http:// or https://',
    '/health never became ready at',
    'job $JOB_ID never reached processed state',
]:
    require_contains('deploy_smoke', needle, 'deploy-smoke contract')

for needle in [
    'usage: bash $PACKAGE_REL/scripts/smoke.sh',
    '.tmp/m051-s02/fixture-smoke',
    'required command missing from PATH: $command_name',
    'jobs table is missing; run either: cargo run -q -p meshc -- migrate $PACKAGE_REL up OR bash $PACKAGE_REL/scripts/apply-deploy-migrations.sh $PACKAGE_REL/deploy/reference-backend.up.sql',
    'ensure_source_tree_clean',
]:
    require_contains('smoke', needle, 'fixture smoke contract')
require_not_contains('smoke', 'reference-backend/reference-backend', 'fixture smoke repo-root binary drift')

for needle in [
    'e2e_reference_backend_migration_status_and_apply',
    'e2e_reference_backend_deploy_artifact_smoke',
    'e2e_reference_backend_worker_crash_recovers_job',
    'e2e_reference_backend_worker_restart_is_visible_in_health',
    'e2e_reference_backend_process_restart_recovers_inflight_job',
    'scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh',
]:
    require_contains('e2e_runtime', needle, 'retained backend e2e rail')

for needle in [
    'This README is the canonical maintainer runbook',
    'm051_s02_retained_backend_verifier_replays_backend_rails_and_retains_bundle_markers',
    'verify-m051-s02.sh',
    'latest-proof-bundle.txt',
    'retained-reference-backend-runtime',
    'retained-fixture-smoke',
    'reference-backend.compat.verify-production-proof-surface.sh',
]:
    require_contains('e2e_contract', needle, 'slice contract target marker')

for needle in [
    'm051-s02-contract',
    'm051-s02-package-tests',
    'm051-s02-e2e',
    'm051-s02-db-env-preflight',
    'm051-s02-migration-status-apply',
    'm051-s02-fixture-smoke',
    'm051-s02-deploy-artifact-smoke',
    'm051-s02-worker-crash-recovery',
    'm051-s02-worker-restart-visibility',
    'm051-s02-process-restart-recovery',
    'retain-reference-backend-runtime',
    'retain-fixture-smoke',
    'retain-contract-artifacts',
    'm051-s02-bundle-shape',
    'status.txt',
    'current-phase.txt',
    'phase-report.txt',
    'full-contract.log',
    'latest-proof-bundle.txt',
    'retained-reference-backend-runtime',
    'retained-fixture-smoke',
    'retained-contract-artifacts',
    'verify-m051-s02: ok',
]:
    require_contains('verifier', needle, 'verifier contract marker')

require_order(
    'verifier',
    [
        'run_contract_checks "$ARTIFACT_DIR/m051-s02-contract.log"',
        'run_expect_success m051-s02-package-tests',
        'run_expect_success m051-s02-e2e',
        'begin_phase m051-s02-db-env-preflight',
        'run_expect_success m051-s02-migration-status-apply',
        'run_expect_success m051-s02-fixture-smoke',
        'run_expect_success m051-s02-deploy-artifact-smoke',
        'run_expect_success m051-s02-worker-crash-recovery',
        'run_expect_success m051-s02-worker-restart-visibility',
        'run_expect_success m051-s02-process-restart-recovery',
        'copy_fixed_dir_or_fail retain-reference-backend-runtime',
        'copy_fixed_dir_or_fail retain-fixture-smoke',
        'copy_new_prefixed_artifacts_or_fail \\',
        'assert_retained_bundle_shape \\',
        'echo "verify-m051-s02: ok"',
    ],
    'verifier replay order',
)


for label in ['compat_runbook', 'compat_verifier']:
    if not texts[label].strip():
        raise SystemExit(f'{label}: expected preserved repo-root compatibility surface to stay non-empty')

print('m051-s02 retained backend contract: ok')
PY
}

assert_retained_bundle_shape() {
  local phase="$1"
  local bundle_root="$2"
  local pointer_path="$3"
  local contract_log="$4"
  local artifact_dir="$5"
  local manifest_path="$6"
  local log_path="$ARTIFACT_DIR/${phase}.bundle-check.log"

  if ! python3 - "$ROOT_DIR" "$bundle_root" "$pointer_path" "$contract_log" "$artifact_dir" "$manifest_path" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import re
import sys

repo_root = Path(sys.argv[1]).resolve()
bundle_root = Path(sys.argv[2]).resolve()
pointer_path = Path(sys.argv[3])
contract_log = Path(sys.argv[4])
artifact_dir = Path(sys.argv[5])
manifest_path = Path(sys.argv[6])
expected_pointer = str(bundle_root)
actual_pointer = pointer_path.read_text(errors='replace').strip()
if actual_pointer != expected_pointer:
    raise SystemExit(
        f"latest-proof-bundle pointer drifted: expected {expected_pointer!r}, got {actual_pointer!r}"
    )
if bundle_root == repo_root or repo_root in bundle_root.parents:
    raise SystemExit(f"retained proof bundle must stay outside the repo root: {bundle_root}")
if not bundle_root.is_dir():
    raise SystemExit(f"missing retained proof bundle directory: {bundle_root}")
if not manifest_path.is_file() or not manifest_path.read_text(errors='replace').strip():
    raise SystemExit(f"expected non-empty copied-artifact manifest: {manifest_path}")

for relative in [
    'fixture.README.md',
    'verify-m051-s02.sh',
    'e2e_m051_s02.rs',
    'reference-backend.compat.README.md',
    'reference-backend.compat.verify-production-proof-surface.sh',
]:
    if not (bundle_root / relative).exists():
        raise SystemExit(f"{bundle_root}: missing required retained file {relative}")

runtime_root = bundle_root / 'retained-reference-backend-runtime'
for relative in ['reference-backend', 'build-output.json']:
    if not (runtime_root / relative).exists():
        raise SystemExit(f"{runtime_root}: missing {relative}")

fixture_smoke_root = bundle_root / 'retained-fixture-smoke'
for relative in ['build/reference-backend', 'reference-backend.log']:
    if not (fixture_smoke_root / relative).exists():
        raise SystemExit(f"{fixture_smoke_root}: missing {relative}")

contract_root = bundle_root / 'retained-contract-artifacts'
children = sorted(path for path in contract_root.iterdir() if path.is_dir())
if not children:
    raise SystemExit(f"{contract_root}: expected copied contract artifact directories")


def find_one(prefix: str) -> Path:
    matches = [path for path in children if path.name.startswith(prefix)]
    if len(matches) != 1:
        raise SystemExit(
            f"{contract_root}: expected exactly one copied artifact directory with prefix {prefix!r}, found {[path.name for path in matches]}"
        )
    return matches[0]

stage_artifact = find_one('retained-backend-stage-deploy-bundle-')
wrong_root_artifact = find_one('retained-backend-wrong-root-')
for relative in ['scenario-meta.json', 'bundle-manifest.txt', 'latest-proof-bundle.txt']:
    if not (stage_artifact / relative).exists():
        raise SystemExit(f"{stage_artifact}: missing {relative}")
if not (wrong_root_artifact / 'wrong-root.error.txt').exists():
    raise SystemExit(f"{wrong_root_artifact}: missing wrong-root.error.txt")

scan_paths = []
scan_paths.extend(path for path in artifact_dir.rglob('*') if path.is_file())
for subdir in [runtime_root, fixture_smoke_root, contract_root]:
    scan_paths.extend(path for path in subdir.rglob('*') if path.is_file())

allowed_suffixes = {
    '.log',
    '.txt',
    '.json',
    '.http',
    '.meta',
}
leak_pattern = re.compile(r'postgres(?:ql)?://|DATABASE_URL=')
for path in scan_paths:
    if path.suffix not in allowed_suffixes and path.name not in {'reference-backend.log'}:
        continue
    try:
        text = path.read_text(errors='replace')
    except Exception as exc:  # pragma: no cover - file-system failure path
        raise SystemExit(f"failed to read {path} while scanning for secrets: {exc}")
    if leak_pattern.search(text):
        raise SystemExit(f"secret-looking database marker leaked into {path}")

print('retained bundle shape: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "retained bundle pointer, runtime artifact shape, or redaction contract drifted" "$log_path" "$bundle_root"
  fi
}

record_phase init started
for command_name in cargo python3 rg bash psql curl; do
  require_command "$command_name"
done
for path in \
  "$ROOT_DIR/$FIXTURE_RUNBOOK" \
  "$ROOT_DIR/$FIXTURE_STAGE_SCRIPT" \
  "$ROOT_DIR/$FIXTURE_APPLY_SCRIPT" \
  "$ROOT_DIR/$FIXTURE_DEPLOY_SMOKE_SCRIPT" \
  "$ROOT_DIR/$FIXTURE_SMOKE_SCRIPT" \
  "$ROOT_DIR/$E2E_CONTRACT_FILE" \
  "$ROOT_DIR/$E2E_RUNTIME_FILE" \
  "$ROOT_DIR/$COMPAT_RUNBOOK" \
  "$ROOT_DIR/$COMPAT_PROOF_SCRIPT"; do
  require_file init "$path" "required retained backend surface"
done
record_phase init passed

capture_snapshot "$ROOT_DIR/$ARTIFACT_ROOT" "$CONTRACT_SNAPSHOT_PATH" verify reference-backend-runtime fixture-smoke

begin_phase m051-s02-contract
if ! run_contract_checks "$ARTIFACT_DIR/m051-s02-contract.log"; then
  record_phase m051-s02-contract failed
  fail_phase m051-s02-contract "retained backend maintainer contract drifted" "$ARTIFACT_DIR/m051-s02-contract.log"
fi
record_phase m051-s02-contract passed

run_expect_success m051-s02-package-tests m051-s02-package-tests no 1800 "$FIXTURE_TESTS_DIR" \
  cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests
run_expect_success m051-s02-e2e m051-s02-e2e yes 2400 "$ARTIFACT_ROOT" \
  cargo test -p meshc --test e2e_m051_s02 -- --nocapture

begin_phase m051-s02-db-env-preflight
DB_ENV_LOG="$ARTIFACT_DIR/m051-s02-db-env-preflight.log"
if [[ -z "${DATABASE_URL:-}" ]]; then
  printf 'DATABASE_URL must be set for scripts/verify-m051-s02.sh\n' >"$DB_ENV_LOG"
  record_phase m051-s02-db-env-preflight failed
  fail_phase m051-s02-db-env-preflight "DATABASE_URL must be set for the DB-backed retained replay" "$DB_ENV_LOG"
fi
if [[ "$DATABASE_URL" == *$'\n'* || "$DATABASE_URL" == *$'\r'* ]]; then
  printf 'DATABASE_URL must not contain newlines\n' >"$DB_ENV_LOG"
  record_phase m051-s02-db-env-preflight failed
  fail_phase m051-s02-db-env-preflight "DATABASE_URL must not contain newlines" "$DB_ENV_LOG"
fi
printf 'DATABASE_URL present for retained backend replay\n' >"$DB_ENV_LOG"
record_phase m051-s02-db-env-preflight passed

cleanup_fixture_smoke_processes m051-s02-fixture-smoke-preflight

run_expect_success m051-s02-migration-status-apply m051-s02-migration-status-apply yes 3600 "$ARTIFACT_ROOT" \
  cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture

SMOKE_PORT="$(pick_unused_port)"
run_expect_success m051-s02-fixture-smoke m051-s02-fixture-smoke no 1800 "$FIXTURE_SMOKE_DIR" \
  env PORT="$SMOKE_PORT" JOB_POLL_MS=200 bash scripts/fixtures/backend/reference-backend/scripts/smoke.sh
cleanup_fixture_smoke_processes m051-s02-fixture-smoke-postflight

run_expect_success m051-s02-deploy-artifact-smoke m051-s02-deploy-artifact-smoke yes 3600 "$ARTIFACT_ROOT" \
  cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture
run_expect_success m051-s02-worker-crash-recovery m051-s02-worker-crash-recovery yes 3600 "$ARTIFACT_ROOT" \
  cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture
run_expect_success m051-s02-worker-restart-visibility m051-s02-worker-restart-visibility yes 3600 "$ARTIFACT_ROOT" \
  cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture
run_expect_success m051-s02-process-restart-recovery m051-s02-process-restart-recovery yes 3600 "$ARTIFACT_ROOT" \
  cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture

RETAINED_PROOF_BUNDLE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/m051-s02-proof.XXXXXX")"

begin_phase retain-reference-backend-runtime
copy_fixed_dir_or_fail retain-reference-backend-runtime \
  "$FIXTURE_RUNTIME_DIR" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-reference-backend-runtime" \
  "retained backend runtime artifacts are missing or malformed" \
  reference-backend \
  build-output.json
record_phase retain-reference-backend-runtime passed

begin_phase retain-fixture-smoke
copy_fixed_dir_or_fail retain-fixture-smoke \
  "$FIXTURE_SMOKE_DIR" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-fixture-smoke" \
  "retained fixture smoke artifacts are missing or malformed" \
  build/reference-backend \
  reference-backend.log
record_phase retain-fixture-smoke passed

begin_phase retain-contract-artifacts
copy_new_prefixed_artifacts_or_fail \
  retain-contract-artifacts \
  "$CONTRACT_SNAPSHOT_PATH" \
  "$ROOT_DIR/$ARTIFACT_ROOT" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-contract-artifacts" \
  "$CONTRACT_ARTIFACT_MANIFEST_PATH" \
  "expected fresh .tmp/m051-s02 contract artifact directories from e2e_m051_s02" \
  retained-backend-stage-deploy-bundle- \
  retained-backend-wrong-root-
record_phase retain-contract-artifacts passed

begin_phase m051-s02-bundle-shape
cp "$ROOT_DIR/$FIXTURE_RUNBOOK" "$RETAINED_PROOF_BUNDLE_DIR/fixture.README.md"
cp "$ROOT_DIR/$E2E_CONTRACT_FILE" "$RETAINED_PROOF_BUNDLE_DIR/e2e_m051_s02.rs"
cp "$ROOT_DIR/scripts/verify-m051-s02.sh" "$RETAINED_PROOF_BUNDLE_DIR/verify-m051-s02.sh"
cp "$ROOT_DIR/$COMPAT_RUNBOOK" "$RETAINED_PROOF_BUNDLE_DIR/reference-backend.compat.README.md"
cp "$ROOT_DIR/$COMPAT_PROOF_SCRIPT" "$RETAINED_PROOF_BUNDLE_DIR/reference-backend.compat.verify-production-proof-surface.sh"
bundle_root_resolved="$(python3 -c 'from pathlib import Path; import sys; print(Path(sys.argv[1]).resolve())' "$RETAINED_PROOF_BUNDLE_DIR")"
printf '%s\n' "$bundle_root_resolved" >"$LATEST_PROOF_BUNDLE_PATH"
assert_retained_bundle_shape \
  m051-s02-bundle-shape \
  "$RETAINED_PROOF_BUNDLE_DIR" \
  "$LATEST_PROOF_BUNDLE_PATH" \
  "$ARTIFACT_DIR/full-contract.log" \
  "$ARTIFACT_DIR" \
  "$CONTRACT_ARTIFACT_MANIFEST_PATH"
record_phase m051-s02-bundle-shape passed

for expected_phase in \
  init \
  m051-s02-contract \
  m051-s02-package-tests \
  m051-s02-e2e \
  m051-s02-db-env-preflight \
  m051-s02-migration-status-apply \
  m051-s02-fixture-smoke \
  m051-s02-deploy-artifact-smoke \
  m051-s02-worker-crash-recovery \
  m051-s02-worker-restart-visibility \
  m051-s02-process-restart-recovery \
  retain-reference-backend-runtime \
  retain-fixture-smoke \
  retain-contract-artifacts \
  m051-s02-bundle-shape; do
  if ! rg -Fq "${expected_phase}	passed" "$PHASE_REPORT_PATH"; then
    fail_phase final-phase-report "phase report missing passed marker for ${expected_phase}" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m051-s02: ok"
echo "artifacts: $(repo_rel "$ARTIFACT_DIR")"
echo "proof bundle: $RETAINED_PROOF_BUNDLE_DIR"
