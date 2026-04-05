#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m051-s03"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
M051_S03_SNAPSHOT_PATH="$ARTIFACT_DIR/m051-s03-before.snapshot"
RETAINED_M051_S03_ARTIFACT_MANIFEST_PATH="$ARTIFACT_DIR/retained-m051-s03-artifacts.manifest.txt"
CONTRACT_TEST_PATH="$ROOT_DIR/scripts/tests/verify-m036-s03-contract.test.mjs"
RUST_TEST_TARGET_PATH="$ROOT_DIR/compiler/meshc/tests/e2e_m051_s03.rs"
VSCODE_PACKAGE_JSON="$ROOT_DIR/tools/editors/vscode-mesh/package.json"
M036_S02_VERIFY_SCRIPT="$ROOT_DIR/scripts/verify-m036-s02.sh"
M036_S03_VERIFY_SCRIPT="$ROOT_DIR/scripts/verify-m036-s03.sh"
NEOVIM_VENDOR_BIN_REL=".tmp/m036-s02/vendor/nvim-macos-arm64/bin/nvim"
NEOVIM_BIN="${NEOVIM_BIN:-nvim}"
NEOVIM_BIN_RESOLVED=""
RETAINED_PROOF_BUNDLE_DIR=""
LAST_STDOUT_PATH=""
LAST_STDERR_PATH=""
LAST_LOG_PATH=""

repo_rel() {
  local candidate="$1"
  if [[ "$candidate" == "$ROOT_DIR/"* ]]; then
    printf '%s\n' "${candidate#$ROOT_DIR/}"
  else
    printf '%s\n' "$candidate"
  fi
}

prepare_artifact_dir() {
  rm -rf "$ARTIFACT_DIR"
  mkdir -p "$ARTIFACT_DIR"
  : >"$PHASE_REPORT_PATH"
  printf 'running\n' >"$STATUS_PATH"
  printf 'init\n' >"$CURRENT_PHASE_PATH"
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
lines = path.read_text(errors='replace').splitlines()
for line in lines[:220]:
    print(line)
if len(lines) > 220:
    print(f"... truncated after 220 lines (total {len(lines)})")
PY
}

record_phase() {
  printf '%s\t%s\n' "$1" "$2" >>"$PHASE_REPORT_PATH"
}

begin_phase() {
  record_phase "$1" started
  printf '%s\n' "$1" >"$CURRENT_PHASE_PATH"
}

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

fail_phase() {
  local phase="$1"
  local reason="$2"
  local log_path="${3:-}"
  local artifact_hint="${4:-}"

  printf 'failed\n' >"$STATUS_PATH"
  printf '%s\n' "$phase" >"$CURRENT_PHASE_PATH"
  echo "verification drift: ${reason}" >&2
  echo "artifacts: $(repo_rel "$ARTIFACT_DIR")" >&2
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

resolve_tool_path() {
  local candidate="$1"
  if [[ -x "$candidate" ]]; then
    printf '%s\n' "$candidate"
    return 0
  fi
  command -v "$candidate" 2>/dev/null || return 1
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
  fail_phase "$phase" "required command missing from PATH: ${command_name}" "$log_path" "$artifact_hint"
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

require_package_script() {
  local phase="$1"
  local package_json_path="$2"
  local script_name="$3"
  local artifact_hint="${4:-}"
  local stdout_path="$ARTIFACT_DIR/${phase}-script-check.stdout"
  local stderr_path="$ARTIFACT_DIR/${phase}-script-check.stderr"
  local log_path="$ARTIFACT_DIR/${phase}-script-check.log"
  local status=0

  python3 - "$package_json_path" "$script_name" >"$stdout_path" 2>"$stderr_path" <<'PY' || status=$?
import json
import sys
from pathlib import Path

package_path = Path(sys.argv[1])
script_name = sys.argv[2]

try:
    package_json = json.loads(package_path.read_text())
except Exception as exc:
    print(f"failed to read {package_path}: {exc}", file=sys.stderr)
    raise SystemExit(1)

scripts = package_json.get("scripts") or {}
if script_name not in scripts:
    print(f"missing npm script {script_name!r} in {package_path}", file=sys.stderr)
    raise SystemExit(1)

print(scripts[script_name])
PY

  combine_command_log \
    "python3 <package-json script check> $(repo_rel "$package_json_path") :: ${script_name}" \
    "$ROOT_DIR" \
    "$stdout_path" \
    "$stderr_path" \
    "$log_path"

  if [[ "$status" -ne 0 ]]; then
    record_phase "$phase" failed
    fail_phase "$phase" "missing npm script ${script_name} in $(repo_rel "$package_json_path")" "$log_path" "$artifact_hint"
  fi
}

require_resolvable_tool() {
  local phase="$1"
  local candidate="$2"
  local description="$3"
  local artifact_hint="${4:-}"
  local resolved_path

  if resolved_path="$(resolve_tool_path "$candidate")"; then
    printf '%s\n' "$resolved_path" >"$ARTIFACT_DIR/${phase}.resolved-tool.txt"
    return 0
  fi

  local log_path="$ARTIFACT_DIR/${phase}.preflight.log"
  {
    echo "preflight: missing required tool"
    echo "description: ${description}"
    echo "tool: ${candidate}"
  } >"$log_path"
  record_phase "$phase" failed
  fail_phase "$phase" "missing required executable: ${candidate}" "$log_path" "$artifact_hint"
}

ensure_historical_neovim_vendor_link() {
  local phase="$1"
  local vendor_path="$ROOT_DIR/$NEOVIM_VENDOR_BIN_REL"
  local log_path="$ARTIFACT_DIR/${phase}.vendor-link.log"

  if [[ -x "$vendor_path" ]]; then
    return 0
  fi

  mkdir -p "$(dirname "$vendor_path")"
  rm -f "$vendor_path"
  ln -s "$NEOVIM_BIN_RESOLVED" "$vendor_path"
  {
    echo "bridged historical Neovim vendor path"
    echo "resolved_tool: $NEOVIM_BIN_RESOLVED"
    echo "vendor_path: $vendor_path"
  } >"$log_path"

  if [[ ! -x "$vendor_path" ]]; then
    record_phase "$phase" failed
    fail_phase "$phase" "failed to materialize the historical Neovim vendor path" "$log_path" ".tmp/m036-s02"
  fi
}

require_grep() {
  local phase="$1"
  local target_path="$2"
  local pattern="$3"
  local reason="$4"
  local artifact_hint="${5:-}"

  if grep -Fq "$pattern" "$target_path"; then
    return 0
  fi

  local log_path="$ARTIFACT_DIR/${phase}.postcheck.log"
  {
    echo "postcheck: pattern missing"
    echo "path: $(repo_rel "$target_path")"
    echo "pattern: ${pattern}"
    echo "reason: ${reason}"
  } >"$log_path"
  record_phase "$phase" failed
  fail_phase "$phase" "$reason" "$log_path" "$artifact_hint"
}

combine_command_log() {
  local display="$1"
  local cwd="$2"
  local stdout_path="$3"
  local stderr_path="$4"
  local log_path="$5"

  {
    echo "display: ${display}"
    echo "cwd: $(repo_rel "$cwd")"
    if [[ -s "$stdout_path" ]]; then
      echo
      echo "[stdout]"
      cat "$stdout_path"
    fi
    if [[ -s "$stderr_path" ]]; then
      echo
      echo "[stderr]"
      cat "$stderr_path"
    fi
  } >"$log_path"
}

run_command() {
  local phase="$1"
  local label="$2"
  local timeout_seconds="$3"
  local cwd="$4"
  local display="$5"
  local artifact_hint="${6:-}"
  shift 6

  local stdout_path="$ARTIFACT_DIR/${label}.stdout"
  local stderr_path="$ARTIFACT_DIR/${label}.stderr"
  local log_path="$ARTIFACT_DIR/${label}.log"
  local status=0

  begin_phase "$phase"
  echo "==> [${phase}] ${display}"

  python3 - "$timeout_seconds" "$cwd" "$stdout_path" "$stderr_path" "$@" <<'PY' || status=$?
from pathlib import Path
import subprocess
import sys

try:
    timeout_seconds = float(sys.argv[1])
except ValueError as exc:
    raise SystemExit(f"invalid timeout {sys.argv[1]!r}: {exc}")

cwd = sys.argv[2]
stdout_path = Path(sys.argv[3])
stderr_path = Path(sys.argv[4])
command = sys.argv[5:]

try:
    completed = subprocess.run(
        command,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=timeout_seconds,
        check=False,
    )
except subprocess.TimeoutExpired as exc:
    stdout_path.write_text(exc.stdout or "")
    timeout_note = f"\n[verify-timeout] command exceeded {timeout_seconds:g} seconds\n"
    stderr_path.write_text((exc.stderr or "") + timeout_note)
    raise SystemExit(124)

stdout_path.write_text(completed.stdout)
stderr_path.write_text(completed.stderr)
raise SystemExit(completed.returncode)
PY

  combine_command_log "$display" "$cwd" "$stdout_path" "$stderr_path" "$log_path"

  if [[ "$status" -ne 0 ]]; then
    if [[ "$status" -eq 124 ]]; then
      record_phase "$phase" failed
      fail_phase "$phase" "${display} timed out" "$log_path" "$artifact_hint"
    fi
    record_phase "$phase" failed
    fail_phase "$phase" "${display} failed" "$log_path" "$artifact_hint"
  fi

  LAST_STDOUT_PATH="$stdout_path"
  LAST_STDERR_PATH="$stderr_path"
  LAST_LOG_PATH="$log_path"
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

text = Path(sys.argv[1]).read_text(errors='replace')
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

  run_command "$phase" "$label" "$timeout_secs" "$ROOT_DIR" "${cmd[*]}" "$artifact_hint" "${cmd[@]}"
  if [[ "$require_tests" == "yes" ]]; then
    assert_test_filter_ran "$phase" "$LAST_LOG_PATH" "$label"
  fi
  record_phase "$phase" passed
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
  local log_path="$ARTIFACT_DIR/${phase}.bundle-check.log"

  if ! python3 - "$bundle_root" "$pointer_path" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import sys

bundle_root = Path(sys.argv[1])
pointer_path = Path(sys.argv[2])
expected_pointer = str(bundle_root)
actual_pointer = pointer_path.read_text(errors='replace').strip()
if actual_pointer != expected_pointer:
    raise SystemExit(
        f"latest-proof-bundle pointer drifted: expected {expected_pointer!r}, got {actual_pointer!r}"
    )

for relative in [
    'vscode.README.md',
    'neovim.README.md',
    'verify-m036-s03-contract.test.mjs',
    'e2e_m051_s03.rs',
    'verify-m051-s03.sh',
    'retained-m051-s03-artifacts.manifest.txt',
]:
    if not (bundle_root / relative).is_file():
        raise SystemExit(f"{bundle_root}: missing required contract file {relative}")

required_fixed = {
    'retained-m036-s02-syntax': [
        'neovim-smoke.log',
        'corpus/materialized-corpus.json',
    ],
    'retained-m036-s02-lsp': [
        'neovim-smoke.log',
        'upstream-lsp.log',
    ],
    'retained-m036-s02-all': [
        'neovim-smoke.log',
        'upstream-lsp.log',
    ],
    'retained-m036-s03-vscode-smoke': [
        'context.json',
        'smoke.log',
        'workspace/mesh-smoke.code-workspace',
    ],
    'retained-m036-s03-verify': [
        'status.txt',
        'current-phase.txt',
        'docs-contract.log',
        'docs-build.log',
        'vsix-proof.log',
        'vscode-smoke.log',
        'neovim.log',
        'vscode-smoke/context.json',
        'vscode-smoke/smoke.log',
        'vscode-smoke/workspace/mesh-smoke.code-workspace',
    ],
}
for rel_dir, required_files in required_fixed.items():
    base = bundle_root / rel_dir
    if not base.is_dir():
        raise SystemExit(f"{bundle_root}: missing {rel_dir}")
    for rel in required_files:
        if not (base / rel).exists():
            raise SystemExit(f"{base}: missing {rel}")

syntax_log = (bundle_root / 'retained-m036-s02-syntax' / 'neovim-smoke.log').read_text(errors='replace')
if '[m036-s02] phase=syntax result=pass' not in syntax_log:
    raise SystemExit('retained-m036-s02-syntax/neovim-smoke.log missing syntax pass marker')

lsp_log = (bundle_root / 'retained-m036-s02-lsp' / 'neovim-smoke.log').read_text(errors='replace')
if '[m036-s02] phase=lsp result=pass' not in lsp_log:
    raise SystemExit('retained-m036-s02-lsp/neovim-smoke.log missing lsp pass marker')

all_log = (bundle_root / 'retained-m036-s02-all' / 'neovim-smoke.log').read_text(errors='replace')
if '[m036-s02] phase=syntax result=pass' not in all_log or '[m036-s02] phase=lsp result=pass' not in all_log:
    raise SystemExit('retained-m036-s02-all/neovim-smoke.log missing syntax or lsp pass marker')

vscode_log = (bundle_root / 'retained-m036-s03-vscode-smoke' / 'smoke.log').read_text(errors='replace')
if 'Extension Development Host smoke passed' not in vscode_log:
    raise SystemExit('retained-m036-s03-vscode-smoke/smoke.log missing pass marker')

verify_root = bundle_root / 'retained-m036-s03-verify'
if (verify_root / 'status.txt').read_text(errors='replace').strip() != 'ok':
    raise SystemExit('retained-m036-s03-verify/status.txt missing ok marker')
if (verify_root / 'current-phase.txt').read_text(errors='replace').strip() != 'complete':
    raise SystemExit('retained-m036-s03-verify/current-phase.txt missing complete marker')
if 'verify-m034-s04-extension: ok' not in (verify_root / 'vsix-proof.log').read_text(errors='replace'):
    raise SystemExit('retained-m036-s03-verify/vsix-proof.log missing pass marker')
if 'Extension Development Host smoke passed' not in (verify_root / 'vscode-smoke/smoke.log').read_text(errors='replace'):
    raise SystemExit('retained-m036-s03-verify/vscode-smoke/smoke.log missing pass marker')

manifest_lines = [
    line for line in (bundle_root / 'retained-m051-s03-artifacts.manifest.txt').read_text(errors='replace').splitlines() if line.strip()
]
if not manifest_lines:
    raise SystemExit('retained-m051-s03-artifacts.manifest.txt should stay non-empty')

s03_root = bundle_root / 'retained-m051-s03-artifacts'
expected = {
    'support-helpers-': ['resolved-paths.txt'],
    'source-rails-': ['source-targets.txt'],
    'meshc-test-retained-fixture-': ['meshc-test.combined.log'],
    'formatter-contract-': ['fmt-bounded-api.combined.log', 'fmt-known-red-fixture-test.combined.log'],
    'editor-and-corpus-targets-': ['editor-and-corpus-targets.txt'],
    'editor-readmes-': ['readme-paths.txt'],
}
children = [path for path in s03_root.iterdir() if path.is_dir()]
for prefix, required_files in expected.items():
    matches = [path for path in children if path.name.startswith(prefix)]
    if len(matches) != 1:
        raise SystemExit(f'{s03_root}: expected exactly one retained artifact for {prefix}, found {[path.name for path in matches]}')
    for rel in required_files:
        if not (matches[0] / rel).exists():
            raise SystemExit(f'{matches[0]}: missing {rel}')

print('retained-bundle-shape: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "missing retained proof artifacts or malformed bundle pointer" "$log_path" "$bundle_root"
  fi
}

prepare_artifact_dir
exec > >(tee "$ARTIFACT_DIR/full-contract.log") 2>&1

record_phase init started
for command_name in cargo node npm python3 bash rg; do
  require_command init "$command_name" "required command for the M051 S03 assembled replay"
done
require_file init "$CONTRACT_TEST_PATH" "M036 S03 editor contract test" "$ARTIFACT_DIR"
require_file init "$RUST_TEST_TARGET_PATH" "M051 S03 Rust source contract target" "$ARTIFACT_DIR"
require_file init "$VSCODE_PACKAGE_JSON" "VS Code extension package manifest" ".tmp/m036-s03/vscode-smoke"
require_package_script init "$VSCODE_PACKAGE_JSON" test:smoke ".tmp/m036-s03/vscode-smoke"
require_file init "$M036_S02_VERIFY_SCRIPT" "M036 S02 retained Neovim verifier" ".tmp/m036-s02"
require_file init "$M036_S03_VERIFY_SCRIPT" "M036 S03 historical tooling verifier" ".tmp/m036-s03"
require_resolvable_tool init "$NEOVIM_BIN" "Neovim binary for the retained editor-host replay" ".tmp/m036-s02"
NEOVIM_BIN_RESOLVED="$(<"$ARTIFACT_DIR/init.resolved-tool.txt")"
ensure_historical_neovim_vendor_link init
echo "[m051-s03] neovim_bin=${NEOVIM_BIN_RESOLVED} historical_vendor=${NEOVIM_VENDOR_BIN_REL}"
record_phase init passed
capture_snapshot "$ROOT_DIR/.tmp/m051-s03" "$M051_S03_SNAPSHOT_PATH" verify

run_expect_success m051-s03-contract m051-s03-contract no 300 "$CONTRACT_TEST_PATH" \
  node --test scripts/tests/verify-m036-s03-contract.test.mjs

run_expect_success m051-s03-rust-rails m051-s03-rust-rails yes 2400 ".tmp/m051-s03" \
  cargo test -p meshc --test e2e_m051_s03 -- --nocapture

run_expect_success m051-s03-vscode-smoke m051-s03-vscode-smoke no 2400 ".tmp/m036-s03/vscode-smoke" \
  npm --prefix tools/editors/vscode-mesh run test:smoke
require_file m051-s03-vscode-smoke "$ROOT_DIR/.tmp/m036-s03/vscode-smoke/context.json" "VS Code smoke context artifact" ".tmp/m036-s03/vscode-smoke"
require_file m051-s03-vscode-smoke "$ROOT_DIR/.tmp/m036-s03/vscode-smoke/smoke.log" "VS Code smoke log artifact" ".tmp/m036-s03/vscode-smoke"
require_file m051-s03-vscode-smoke "$ROOT_DIR/.tmp/m036-s03/vscode-smoke/workspace/mesh-smoke.code-workspace" "VS Code smoke workspace artifact" ".tmp/m036-s03/vscode-smoke"
require_grep m051-s03-vscode-smoke "$ROOT_DIR/.tmp/m036-s03/vscode-smoke/smoke.log" "Extension Development Host smoke passed" "VS Code smoke completed without the expected pass marker" ".tmp/m036-s03/vscode-smoke"

run_expect_success m051-s03-neovim-syntax m051-s03-neovim-syntax no 2400 ".tmp/m036-s02/syntax" \
  env "NEOVIM_BIN=$NEOVIM_BIN_RESOLVED" bash scripts/verify-m036-s02.sh syntax
require_file m051-s03-neovim-syntax "$ROOT_DIR/.tmp/m036-s02/syntax/neovim-smoke.log" "M036 S02 syntax smoke log" ".tmp/m036-s02/syntax"
require_file m051-s03-neovim-syntax "$ROOT_DIR/.tmp/m036-s02/syntax/corpus/materialized-corpus.json" "M036 S02 syntax materialized corpus" ".tmp/m036-s02/syntax"
require_grep m051-s03-neovim-syntax "$ROOT_DIR/.tmp/m036-s02/syntax/neovim-smoke.log" "[m036-s02] phase=syntax result=pass" "Neovim syntax replay completed without the expected pass marker" ".tmp/m036-s02/syntax"

run_expect_success m051-s03-neovim-lsp m051-s03-neovim-lsp no 2400 ".tmp/m036-s02/lsp" \
  env "NEOVIM_BIN=$NEOVIM_BIN_RESOLVED" bash scripts/verify-m036-s02.sh lsp
require_file m051-s03-neovim-lsp "$ROOT_DIR/.tmp/m036-s02/lsp/neovim-smoke.log" "M036 S02 LSP smoke log" ".tmp/m036-s02/lsp"
require_file m051-s03-neovim-lsp "$ROOT_DIR/.tmp/m036-s02/lsp/upstream-lsp.log" "M036 S02 upstream LSP log" ".tmp/m036-s02/lsp"
require_grep m051-s03-neovim-lsp "$ROOT_DIR/.tmp/m036-s02/lsp/neovim-smoke.log" "[m036-s02] phase=lsp result=pass" "Neovim LSP replay completed without the expected pass marker" ".tmp/m036-s02/lsp"

run_expect_success m051-s03-historical-wrapper m051-s03-historical-wrapper no 2400 ".tmp/m036-s03" \
  env "NEOVIM_BIN=$NEOVIM_BIN_RESOLVED" bash scripts/verify-m036-s03.sh
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s02/all/neovim-smoke.log" "M036 S02 combined replay log" ".tmp/m036-s02/all"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s02/all/upstream-lsp.log" "M036 S02 combined upstream LSP log" ".tmp/m036-s02/all"
require_grep m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s02/all/neovim-smoke.log" "[m036-s02] phase=syntax result=pass" "Combined Neovim replay log is missing the syntax pass marker" ".tmp/m036-s02/all"
require_grep m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s02/all/neovim-smoke.log" "[m036-s02] phase=lsp result=pass" "Combined Neovim replay log is missing the LSP pass marker" ".tmp/m036-s02/all"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/status.txt" "historical M036 verifier status" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/current-phase.txt" "historical M036 verifier current phase" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/docs-contract.log" "historical M036 verifier docs-contract log" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/docs-build.log" "historical M036 verifier docs-build log" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/vsix-proof.log" "historical M036 verifier vsix-proof log" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/vscode-smoke.log" "historical M036 verifier vscode-smoke log" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/neovim.log" "historical M036 verifier neovim log" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/vscode-smoke/context.json" "historical M036 verifier VS Code smoke context" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/vscode-smoke/smoke.log" "historical M036 verifier VS Code smoke log" ".tmp/m036-s03"
require_file m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/vscode-smoke/workspace/mesh-smoke.code-workspace" "historical M036 verifier VS Code workspace artifact" ".tmp/m036-s03"
require_grep m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/status.txt" "ok" "historical M036 verifier completed without writing an ok status" ".tmp/m036-s03"
require_grep m051-s03-historical-wrapper "$ROOT_DIR/.tmp/m036-s03/current-phase.txt" "complete" "historical M036 verifier did not reach the complete phase marker" ".tmp/m036-s03"

RETAINED_PROOF_BUNDLE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/m051-s03-proof.XXXXXX")"

begin_phase retain-m036-s02-syntax
copy_fixed_dir_or_fail retain-m036-s02-syntax \
  "$ROOT_DIR/.tmp/m036-s02/syntax" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m036-s02-syntax" \
  "retained M036 S02 syntax artifacts are missing or malformed" \
  neovim-smoke.log \
  corpus/materialized-corpus.json
record_phase retain-m036-s02-syntax passed

begin_phase retain-m036-s02-lsp
copy_fixed_dir_or_fail retain-m036-s02-lsp \
  "$ROOT_DIR/.tmp/m036-s02/lsp" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m036-s02-lsp" \
  "retained M036 S02 LSP artifacts are missing or malformed" \
  neovim-smoke.log \
  upstream-lsp.log
record_phase retain-m036-s02-lsp passed

begin_phase retain-m036-s02-all
copy_fixed_dir_or_fail retain-m036-s02-all \
  "$ROOT_DIR/.tmp/m036-s02/all" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m036-s02-all" \
  "retained M036 S02 combined artifacts are missing or malformed" \
  neovim-smoke.log \
  upstream-lsp.log
record_phase retain-m036-s02-all passed

begin_phase retain-m036-s03-vscode-smoke
copy_fixed_dir_or_fail retain-m036-s03-vscode-smoke \
  "$ROOT_DIR/.tmp/m036-s03/vscode-smoke" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m036-s03-vscode-smoke" \
  "retained M036 S03 VS Code smoke artifacts are missing or malformed" \
  context.json \
  smoke.log \
  workspace/mesh-smoke.code-workspace
record_phase retain-m036-s03-vscode-smoke passed

begin_phase retain-m036-s03-verify
copy_fixed_dir_or_fail retain-m036-s03-verify \
  "$ROOT_DIR/.tmp/m036-s03" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m036-s03-verify" \
  "retained M036 S03 verifier artifacts are missing or malformed" \
  status.txt \
  current-phase.txt \
  docs-contract.log \
  docs-build.log \
  vsix-proof.log \
  vscode-smoke.log \
  neovim.log \
  vscode-smoke/context.json \
  vscode-smoke/smoke.log \
  vscode-smoke/workspace/mesh-smoke.code-workspace
record_phase retain-m036-s03-verify passed

begin_phase retain-m051-s03-artifacts
copy_new_prefixed_artifacts_or_fail \
  retain-m051-s03-artifacts \
  "$M051_S03_SNAPSHOT_PATH" \
  "$ROOT_DIR/.tmp/m051-s03" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s03-artifacts" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s03-artifacts.manifest.txt" \
  "expected fresh .tmp/m051-s03 artifact directories from the retained tooling/editor replay" \
  support-helpers- \
  source-rails- \
  meshc-test-retained-fixture- \
  formatter-contract- \
  editor-and-corpus-targets- \
  editor-readmes-
record_phase retain-m051-s03-artifacts passed

begin_phase m051-s03-bundle-shape
cp "$ROOT_DIR/tools/editors/vscode-mesh/README.md" "$RETAINED_PROOF_BUNDLE_DIR/vscode.README.md"
cp "$ROOT_DIR/tools/editors/neovim-mesh/README.md" "$RETAINED_PROOF_BUNDLE_DIR/neovim.README.md"
cp "$CONTRACT_TEST_PATH" "$RETAINED_PROOF_BUNDLE_DIR/verify-m036-s03-contract.test.mjs"
cp "$RUST_TEST_TARGET_PATH" "$RETAINED_PROOF_BUNDLE_DIR/e2e_m051_s03.rs"
cp "$ROOT_DIR/scripts/verify-m051-s03.sh" "$RETAINED_PROOF_BUNDLE_DIR/verify-m051-s03.sh"
RETAINED_PROOF_BUNDLE_DIR="$(python3 -c 'from pathlib import Path; import sys; print(Path(sys.argv[1]).resolve())' "$RETAINED_PROOF_BUNDLE_DIR")"
printf '%s\n' "$RETAINED_PROOF_BUNDLE_DIR" >"$LATEST_PROOF_BUNDLE_PATH"
assert_retained_bundle_shape \
  m051-s03-bundle-shape \
  "$RETAINED_PROOF_BUNDLE_DIR" \
  "$LATEST_PROOF_BUNDLE_PATH"
record_phase m051-s03-bundle-shape passed

for expected_phase in \
  init \
  m051-s03-contract \
  m051-s03-rust-rails \
  m051-s03-vscode-smoke \
  m051-s03-neovim-syntax \
  m051-s03-neovim-lsp \
  m051-s03-historical-wrapper \
  retain-m036-s02-syntax \
  retain-m036-s02-lsp \
  retain-m036-s02-all \
  retain-m036-s03-vscode-smoke \
  retain-m036-s03-verify \
  retain-m051-s03-artifacts \
  m051-s03-bundle-shape; do
  if ! rg -q "^${expected_phase}\\tpassed$" "$PHASE_REPORT_PATH"; then
    fail_phase verifier-status "phase report missing passed marker for ${expected_phase}" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m051-s03: ok"
echo "artifacts: $(repo_rel "$ARTIFACT_DIR")"
echo "proof bundle: $(repo_rel "$RETAINED_PROOF_BUNDLE_DIR")"
