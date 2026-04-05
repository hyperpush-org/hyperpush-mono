#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m051-s05"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
RETAINED_PROOF_BUNDLE_DIR="$ARTIFACT_DIR/retained-proof-bundle"
RETAINED_M051_S05_ARTIFACTS_MANIFEST_PATH="$ARTIFACT_DIR/retained-m051-s05-artifacts.manifest.txt"
M051_S05_SNAPSHOT_PATH="$ARTIFACT_DIR/m051-s05-before.snapshot"
E2E_CONTRACT_PATH="$ROOT_DIR/compiler/meshc/tests/e2e_m051_s05.rs"
PROOF_SURFACE_SCRIPT="$ROOT_DIR/scripts/verify-production-proof-surface.sh"
VERIFY_M051_S05_PATH="$ROOT_DIR/scripts/verify-m051-s05.sh"

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
exec > >(tee "$ARTIFACT_DIR/full-contract.log") 2>&1

: >"$PHASE_REPORT_PATH"
printf 'running\n' >"$STATUS_PATH"
printf 'init\n' >"$CURRENT_PHASE_PATH"
printf '%s\n' "$ARTIFACT_DIR" >"$LATEST_PROOF_BUNDLE_PATH"

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
  local phase="$1"
  local command_name="$2"
  local description="$3"
  local artifact_hint="${4:-}"
  if command -v "$command_name" >/dev/null 2>&1; then
    return 0
  fi

  local log_path="$ARTIFACT_DIR/${phase}.preflight.log"
  {
    echo "preflight: missing required command"
    echo "description: ${description}"
    echo "command: ${command_name}"
  } >"$log_path"
  record_phase "$phase" failed
  fail_phase "$phase" "missing required command: ${command_name}" "$log_path" "$artifact_hint"
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
  local artifact_hint="${4:-}"
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
    fail_phase "$phase" "named test filter ran 0 tests or produced malformed output" "$count_log" "$artifact_hint"
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
    assert_test_filter_ran "$phase" "$log_path" "$label" "$artifact_hint"
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

copy_pointed_bundle_or_fail() {
  local phase="$1"
  local source_verify_dir="$2"
  local dest_pointer_path="$3"
  local dest_bundle_dir="$4"
  local description="$5"
  shift 5
  local log_path="$ARTIFACT_DIR/${phase}.copy.log"

  if ! python3 - "$ROOT_DIR" "$source_verify_dir" "$dest_pointer_path" "$dest_bundle_dir" "$description" "$@" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import shutil
import sys

repo_root = Path(sys.argv[1]).resolve()
source_verify_dir = Path(sys.argv[2])
dest_pointer_path = Path(sys.argv[3])
dest_bundle_dir = Path(sys.argv[4])
description = sys.argv[5]
required = sys.argv[6:]

pointer_path = source_verify_dir / 'latest-proof-bundle.txt'
if not pointer_path.is_file():
    raise SystemExit(f"{description}: missing latest-proof-bundle.txt in {source_verify_dir}")
pointer_text = pointer_path.read_text(errors='replace').strip()
if not pointer_text:
    raise SystemExit(f"{description}: empty latest-proof-bundle.txt in {source_verify_dir}")
source_bundle_dir = Path(pointer_text)
if not source_bundle_dir.is_absolute():
    source_bundle_dir = (repo_root / source_bundle_dir).resolve()
if not source_bundle_dir.is_dir():
    raise SystemExit(
        f"{description}: bundle pointer {pointer_text!r} resolved to missing directory {source_bundle_dir}"
    )
for rel in required:
    if not (source_bundle_dir / rel).exists():
        raise SystemExit(f"{description}: missing {rel} in {source_bundle_dir}")
if dest_bundle_dir.exists():
    shutil.rmtree(dest_bundle_dir)
dest_bundle_dir.parent.mkdir(parents=True, exist_ok=True)
shutil.copytree(source_bundle_dir, dest_bundle_dir, symlinks=True)
dest_pointer_path.write_text(str(dest_bundle_dir.resolve()) + '\n')
print(f"copied {source_bundle_dir} -> {dest_bundle_dir}")
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "$description" "$log_path" "$source_verify_dir"
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
  local log_path="$ARTIFACT_DIR/${phase}.copy.log"

  if ! python3 - "$before_snapshot" "$source_root" "$dest_root" "$manifest_path" "$expected_message" "$@" >"$log_path" 2>&1 <<'PY'
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
    fail_phase "$phase" "$expected_message" "$log_path" "$source_root"
  fi
}

assert_retained_bundle_shape() {
  local phase="$1"
  local bundle_root="$2"
  local pointer_path="$3"
  local manifest_path="$4"
  local log_path="$ARTIFACT_DIR/${phase}.bundle-check.log"

  if ! python3 - "$bundle_root" "$pointer_path" "$manifest_path" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import sys

bundle_root = Path(sys.argv[1])
pointer_path = Path(sys.argv[2])
manifest_path = Path(sys.argv[3])
expected_pointer = str(bundle_root)
actual_pointer = pointer_path.read_text(errors='replace').strip()
if actual_pointer != expected_pointer:
    raise SystemExit(
        f"latest-proof-bundle pointer drifted: expected {expected_pointer!r}, got {actual_pointer!r}"
    )

required_top_level_files = [
    'e2e_m051_s05.rs',
    'verify-m051-s05.sh',
    'scripts.verify-production-proof-surface.sh',
    'retained-m051-s05-artifacts.manifest.txt',
]
for rel in required_top_level_files:
    if not (bundle_root / rel).is_file():
        raise SystemExit(f'{bundle_root}: missing required retained file {rel}')

verify_contracts = {
    'retained-m051-s01-verify': {
        'bundle': 'retained-m051-s01-proof-bundle',
        'verify_required': ['status.txt', 'current-phase.txt', 'phase-report.txt', 'full-contract.log', 'latest-proof-bundle.txt'],
        'bundle_required': ['mesher.README.md', 'verify-m051-s01.sh', 'retained-m051-s01-artifacts'],
    },
    'retained-m051-s02-verify': {
        'bundle': 'retained-m051-s02-proof-bundle',
        'verify_required': ['status.txt', 'current-phase.txt', 'phase-report.txt', 'full-contract.log', 'latest-proof-bundle.txt'],
        'bundle_required': ['fixture.README.md', 'verify-m051-s02.sh', 'retained-reference-backend-runtime', 'retained-fixture-smoke', 'retained-contract-artifacts'],
    },
    'retained-m051-s03-verify': {
        'bundle': 'retained-m051-s03-proof-bundle',
        'verify_required': ['status.txt', 'current-phase.txt', 'phase-report.txt', 'full-contract.log', 'latest-proof-bundle.txt'],
        'bundle_required': ['vscode.README.md', 'neovim.README.md', 'verify-m051-s03.sh', 'retained-m036-s03-verify', 'retained-m051-s03-artifacts'],
    },
    'retained-m051-s04-verify': {
        'bundle': 'retained-m051-s04-proof-bundle',
        'verify_required': ['status.txt', 'current-phase.txt', 'phase-report.txt', 'full-contract.log', 'latest-proof-bundle.txt'],
        'bundle_required': ['README.md', 'verify-m051-s04.sh', 'retained-m050-s01-verify', 'retained-m050-s02-verify', 'retained-m050-s03-verify', 'retained-m051-s04-artifacts', 'built-html'],
    },
}

for verify_name, contract in verify_contracts.items():
    verify_dir = bundle_root / verify_name
    if not verify_dir.is_dir():
        raise SystemExit(f'{bundle_root}: missing {verify_name}')
    for rel in contract['verify_required']:
        if not (verify_dir / rel).exists():
            raise SystemExit(f'{verify_dir}: missing {rel}')
    status = (verify_dir / 'status.txt').read_text(errors='replace').strip()
    current_phase = (verify_dir / 'current-phase.txt').read_text(errors='replace').strip()
    if status != 'ok':
        raise SystemExit(f'{verify_dir}/status.txt expected ok, got {status!r}')
    if current_phase != 'complete':
        raise SystemExit(f'{verify_dir}/current-phase.txt expected complete, got {current_phase!r}')

    bundle_dir = bundle_root / contract['bundle']
    if not bundle_dir.is_dir():
        raise SystemExit(f'{bundle_root}: missing {contract["bundle"]}')
    for rel in contract['bundle_required']:
        if not (bundle_dir / rel).exists():
            raise SystemExit(f'{bundle_dir}: missing {rel}')
    expected_child_pointer = str(bundle_dir.resolve())
    actual_child_pointer = (verify_dir / 'latest-proof-bundle.txt').read_text(errors='replace').strip()
    if actual_child_pointer != expected_child_pointer:
        raise SystemExit(
            f'{verify_dir}/latest-proof-bundle.txt drifted: expected {expected_child_pointer!r}, got {actual_child_pointer!r}'
        )

manifest_lines = [line for line in manifest_path.read_text(errors='replace').splitlines() if line.strip()]
if not manifest_lines:
    raise SystemExit(f'expected non-empty copied-artifact manifest: {manifest_path}')

artifacts_root = bundle_root / 'retained-m051-s05-artifacts'
children = [path for path in artifacts_root.iterdir() if path.is_dir()]
for prefix, required in {
    'post-deletion-contract-': ['scripts__verify-production-proof-surface.sh', 'scripts__verify-m051-s05.sh'],
    'verifier-contract-': ['scripts__verify-m051-s05.sh'],
}.items():
    matches = [path for path in children if path.name.startswith(prefix)]
    if len(matches) != 1:
        raise SystemExit(
            f'{artifacts_root}: expected exactly one retained artifact for {prefix}, found {[path.name for path in matches]}'
        )
    for rel in required:
        if not (matches[0] / rel).exists():
            raise SystemExit(f'{matches[0]}: missing {rel}')

print('retained-bundle-shape: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "retained proof bundle pointer or copied child bundle shape drifted" "$log_path" "$bundle_root"
  fi
}

record_phase init started
for command_name in cargo python3 rg bash; do
  require_command init "$command_name" "required command for the M051 S05 assembled replay"
done
for path in \
  "$PROOF_SURFACE_SCRIPT" \
  "$ROOT_DIR/scripts/verify-m051-s01.sh" \
  "$ROOT_DIR/scripts/verify-m051-s02.sh" \
  "$ROOT_DIR/scripts/verify-m051-s03.sh" \
  "$ROOT_DIR/scripts/verify-m051-s04.sh" \
  "$E2E_CONTRACT_PATH"; do
  require_file init "$path" "required M051 S05 surface"
done
record_phase init passed

capture_snapshot "$ROOT_DIR/$ARTIFACT_ROOT" "$M051_S05_SNAPSHOT_PATH" verify

run_expect_success m051-s05-contract m051-s05-contract yes 2400 "$ARTIFACT_ROOT" \
  cargo test -p meshc --test e2e_m051_s05 -- --nocapture

begin_phase m051-s05-db-env-preflight
DB_ENV_LOG="$ARTIFACT_DIR/m051-s05-db-env-preflight.log"
if [[ -z "${DATABASE_URL:-}" ]]; then
  printf 'DATABASE_URL must be set for scripts/verify-m051-s05.sh\n' >"$DB_ENV_LOG"
  record_phase m051-s05-db-env-preflight failed
  fail_phase m051-s05-db-env-preflight "DATABASE_URL must be set for scripts/verify-m051-s05.sh" "$DB_ENV_LOG"
fi
if [[ "$DATABASE_URL" == *$'\n'* || "$DATABASE_URL" == *$'\r'* ]]; then
  printf 'DATABASE_URL must not contain newlines\n' >"$DB_ENV_LOG"
  record_phase m051-s05-db-env-preflight failed
  fail_phase m051-s05-db-env-preflight "DATABASE_URL must not contain newlines" "$DB_ENV_LOG"
fi
printf 'DATABASE_URL present for M051 S05 delegated replay\n' >"$DB_ENV_LOG"
record_phase m051-s05-db-env-preflight passed

run_expect_success m051-s01-wrapper m051-s01-wrapper no 7200 ".tmp/m051-s01/verify" \
  bash scripts/verify-m051-s01.sh
run_expect_success m051-s02-wrapper m051-s02-wrapper no 7200 ".tmp/m051-s02/verify" \
  bash scripts/verify-m051-s02.sh
run_expect_success m051-s03-wrapper m051-s03-wrapper no 7200 ".tmp/m051-s03/verify" \
  bash scripts/verify-m051-s03.sh
run_expect_success m051-s04-wrapper m051-s04-wrapper no 7200 ".tmp/m051-s04/verify" \
  bash scripts/verify-m051-s04.sh

begin_phase retain-m051-s01-verify
copy_fixed_dir_or_fail retain-m051-s01-verify \
  "$ROOT_DIR/.tmp/m051-s01/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s01-verify" \
  "M051 S01 verify artifacts are missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt
record_phase retain-m051-s01-verify passed

begin_phase retain-m051-s01-proof-bundle
copy_pointed_bundle_or_fail retain-m051-s01-proof-bundle \
  "$ROOT_DIR/.tmp/m051-s01/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s01-verify/latest-proof-bundle.txt" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s01-proof-bundle" \
  "M051 S01 pointed proof bundle is missing or malformed" \
  mesher.README.md \
  verify-m051-s01.sh \
  retained-m051-s01-artifacts
record_phase retain-m051-s01-proof-bundle passed

begin_phase retain-m051-s02-verify
copy_fixed_dir_or_fail retain-m051-s02-verify \
  "$ROOT_DIR/.tmp/m051-s02/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s02-verify" \
  "M051 S02 verify artifacts are missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt
record_phase retain-m051-s02-verify passed

begin_phase retain-m051-s02-proof-bundle
copy_pointed_bundle_or_fail retain-m051-s02-proof-bundle \
  "$ROOT_DIR/.tmp/m051-s02/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s02-verify/latest-proof-bundle.txt" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s02-proof-bundle" \
  "M051 S02 pointed proof bundle is missing or malformed" \
  fixture.README.md \
  verify-m051-s02.sh \
  retained-reference-backend-runtime \
  retained-fixture-smoke \
  retained-contract-artifacts
record_phase retain-m051-s02-proof-bundle passed

begin_phase retain-m051-s03-verify
copy_fixed_dir_or_fail retain-m051-s03-verify \
  "$ROOT_DIR/.tmp/m051-s03/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s03-verify" \
  "M051 S03 verify artifacts are missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt
record_phase retain-m051-s03-verify passed

begin_phase retain-m051-s03-proof-bundle
copy_pointed_bundle_or_fail retain-m051-s03-proof-bundle \
  "$ROOT_DIR/.tmp/m051-s03/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s03-verify/latest-proof-bundle.txt" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s03-proof-bundle" \
  "M051 S03 pointed proof bundle is missing or malformed" \
  vscode.README.md \
  neovim.README.md \
  verify-m051-s03.sh \
  retained-m036-s03-verify \
  retained-m051-s03-artifacts
record_phase retain-m051-s03-proof-bundle passed

begin_phase retain-m051-s04-verify
copy_fixed_dir_or_fail retain-m051-s04-verify \
  "$ROOT_DIR/.tmp/m051-s04/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s04-verify" \
  "M051 S04 verify artifacts are missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt
record_phase retain-m051-s04-verify passed

begin_phase retain-m051-s04-proof-bundle
copy_pointed_bundle_or_fail retain-m051-s04-proof-bundle \
  "$ROOT_DIR/.tmp/m051-s04/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s04-verify/latest-proof-bundle.txt" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s04-proof-bundle" \
  "M051 S04 pointed proof bundle is missing or malformed" \
  README.md \
  verify-m051-s04.sh \
  retained-m050-s01-verify \
  retained-m050-s02-verify \
  retained-m050-s03-verify \
  retained-m051-s04-artifacts \
  built-html
record_phase retain-m051-s04-proof-bundle passed

begin_phase retain-m051-s05-artifacts
copy_new_prefixed_artifacts_or_fail \
  retain-m051-s05-artifacts \
  "$M051_S05_SNAPSHOT_PATH" \
  "$ROOT_DIR/$ARTIFACT_ROOT" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s05-artifacts" \
  "$RETAINED_M051_S05_ARTIFACTS_MANIFEST_PATH" \
  "expected fresh .tmp/m051-s05 artifact directories from e2e_m051_s05" \
  post-deletion-contract- \
  verifier-contract-
record_phase retain-m051-s05-artifacts passed

begin_phase m051-s05-bundle-shape
mkdir -p "$RETAINED_PROOF_BUNDLE_DIR"
cp "$E2E_CONTRACT_PATH" "$RETAINED_PROOF_BUNDLE_DIR/e2e_m051_s05.rs"
cp "$VERIFY_M051_S05_PATH" "$RETAINED_PROOF_BUNDLE_DIR/verify-m051-s05.sh"
cp "$PROOF_SURFACE_SCRIPT" "$RETAINED_PROOF_BUNDLE_DIR/scripts.verify-production-proof-surface.sh"
cp "$RETAINED_M051_S05_ARTIFACTS_MANIFEST_PATH" "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s05-artifacts.manifest.txt"
printf '%s\n' "$RETAINED_PROOF_BUNDLE_DIR" >"$LATEST_PROOF_BUNDLE_PATH"
assert_retained_bundle_shape \
  m051-s05-bundle-shape \
  "$RETAINED_PROOF_BUNDLE_DIR" \
  "$LATEST_PROOF_BUNDLE_PATH" \
  "$RETAINED_M051_S05_ARTIFACTS_MANIFEST_PATH"
record_phase m051-s05-bundle-shape passed

for expected_phase in \
  init \
  m051-s05-contract \
  m051-s05-db-env-preflight \
  m051-s01-wrapper \
  m051-s02-wrapper \
  m051-s03-wrapper \
  m051-s04-wrapper \
  retain-m051-s01-verify \
  retain-m051-s01-proof-bundle \
  retain-m051-s02-verify \
  retain-m051-s02-proof-bundle \
  retain-m051-s03-verify \
  retain-m051-s03-proof-bundle \
  retain-m051-s04-verify \
  retain-m051-s04-proof-bundle \
  retain-m051-s05-artifacts \
  m051-s05-bundle-shape; do
  if ! rg -q "^${expected_phase}\\tpassed$" "$PHASE_REPORT_PATH"; then
    fail_phase verifier-status "phase report missing passed marker for ${expected_phase}" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m051-s05: ok"
echo "artifacts: $(repo_rel "$ARTIFACT_DIR")"
echo "proof bundle: $(repo_rel "$RETAINED_PROOF_BUNDLE_DIR")"
