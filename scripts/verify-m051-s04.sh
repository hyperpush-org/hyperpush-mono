#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m051-s04"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
BUILT_HTML_DIR="$ARTIFACT_DIR/built-html"
BUILT_HTML_SUMMARY_PATH="$BUILT_HTML_DIR/summary.json"
RETAINED_PROOF_BUNDLE_DIR="$ARTIFACT_DIR/retained-proof-bundle"
RETAINED_M051_S04_ARTIFACTS_MANIFEST_PATH="$ARTIFACT_DIR/retained-m051-s04-artifacts.manifest.txt"
M051_S04_SNAPSHOT_PATH="$ARTIFACT_DIR/m051-s04-before.snapshot"

GETTING_STARTED_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/getting-started/index.html"
CLUSTERED_EXAMPLE_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/getting-started/clustered-example/index.html"
TOOLING_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/tooling/index.html"
DISTRIBUTED_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/distributed/index.html"
DISTRIBUTED_PROOF_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/distributed-proof/index.html"
PRODUCTION_BACKEND_PROOF_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/production-backend-proof/index.html"

repo_rel() {
  local candidate="$1"
  if [[ "$candidate" == "$ROOT_DIR/"* ]]; then
    printf '%s\n' "${candidate#$ROOT_DIR/}"
  else
    printf '%s\n' "$candidate"
  fi
}

rm -rf "$ARTIFACT_DIR"
mkdir -p "$ARTIFACT_DIR" "$BUILT_HTML_DIR"
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

copy_file_or_fail() {
  local phase="$1"
  local source_path="$2"
  local dest_path="$3"
  local description="$4"
  local log_path="$ARTIFACT_DIR/${phase}.copy.log"

  if [[ ! -f "$source_path" ]]; then
    {
      echo "copy: missing source file"
      echo "description: ${description}"
      echo "source: $(repo_rel "$source_path")"
    } >"$log_path"
    record_phase "$phase" failed
    fail_phase "$phase" "$description" "$log_path" "$source_path"
  fi

  mkdir -p "$(dirname "$dest_path")"
  cp "$source_path" "$dest_path"
  if [[ ! -s "$dest_path" ]]; then
    {
      echo "copy: destination file is empty"
      echo "description: ${description}"
      echo "source: $(repo_rel "$source_path")"
      echo "destination: $(repo_rel "$dest_path")"
    } >"$log_path"
    record_phase "$phase" failed
    fail_phase "$phase" "$description" "$log_path" "$dest_path"
  fi

  printf 'copied %s -> %s\n' "$(repo_rel "$source_path")" "$(repo_rel "$dest_path")" >>"$log_path"
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

assert_built_html_contract() {
  local phase="$1"
  local summary_path="$2"
  local log_path="$ARTIFACT_DIR/${phase}.assert.log"

  if ! python3 - \
    "$GETTING_STARTED_HTML_PATH" \
    "$CLUSTERED_EXAMPLE_HTML_PATH" \
    "$TOOLING_HTML_PATH" \
    "$DISTRIBUTED_HTML_PATH" \
    "$DISTRIBUTED_PROOF_HTML_PATH" \
    "$PRODUCTION_BACKEND_PROOF_HTML_PATH" \
    "$summary_path" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import json
import sys

pages = {
    'getting_started': Path(sys.argv[1]),
    'clustered_example': Path(sys.argv[2]),
    'tooling': Path(sys.argv[3]),
    'distributed': Path(sys.argv[4]),
    'distributed_proof': Path(sys.argv[5]),
    'production_backend_proof': Path(sys.argv[6]),
}
summary_path = Path(sys.argv[7])

checks = {
    'getting_started': {
        'contains': [
            '/docs/getting-started/clustered-example/',
            '/docs/production-backend-proof/',
        ],
        'omits': ['reference-backend/README.md'],
    },
    'clustered_example': {
        'contains': [
            '/docs/distributed-proof/',
            '/docs/production-backend-proof/',
            'examples/todo-sqlite/README.md',
            'examples/todo-postgres/README.md',
        ],
        'omits': ['reference-backend/README.md'],
    },
    'tooling': {
        'contains': [
            '/docs/getting-started/clustered-example/',
            '/docs/production-backend-proof/',
            'examples/todo-sqlite/README.md',
            'examples/todo-postgres/README.md',
            'small backend-shaped Mesh project over real stdio JSON-RPC',
            'same-file go-to-definition inside backend-shaped project code',
        ],
        'omits': [
            'meshc test reference-backend',
            'meshc fmt --check reference-backend',
            'reference-backend/api/jobs.mpl',
        ],
    },
    'distributed': {
        'contains': [
            '/docs/distributed-proof/',
            '/docs/production-backend-proof/',
            'mesher/README.md',
            'bash scripts/verify-m051-s01.sh',
            'bash scripts/verify-m051-s02.sh',
        ],
        'omits': ['reference-backend/README.md'],
    },
    'distributed_proof': {
        'contains': [
            '/docs/production-backend-proof/',
            'mesher/README.md',
            'bash scripts/verify-m051-s01.sh',
            'bash scripts/verify-m051-s02.sh',
            'bash scripts/verify-m047-s04.sh',
            'keep the deeper backend handoff on Production Backend Proof, Mesher, and the retained backend-only verifier instead of promoting any repo-root runbook as a coequal first-contact clustered starter',
        ],
        'omits': [
            'reference-backend/README.md',
            'keep `reference-backend` as the deeper backend proof surface rather than a coequal first-contact clustered starter',
        ],
    },
    'production_backend_proof': {
        'contains': [
            'mesher/README.md',
            'bash scripts/verify-m051-s01.sh',
            'bash scripts/verify-m051-s02.sh',
            'bash scripts/verify-production-proof-surface.sh',
        ],
        'omits': ['reference-backend/README.md'],
    },
}

summary = {}
for name, path in pages.items():
    if not path.is_file():
        raise SystemExit(f'missing built HTML snapshot: {path}')
    text = path.read_text(errors='replace')
    for needle in checks[name]['contains']:
        if needle not in text:
            raise SystemExit(f'{name}: missing marker {needle!r}')
    for needle in checks[name]['omits']:
        if needle in text:
            raise SystemExit(f'{name}: stale marker leaked into built HTML {needle!r}')
    summary[name] = {
        'path': str(path),
        'contains': checks[name]['contains'],
        'omits': checks[name]['omits'],
    }

summary_path.write_text(json.dumps(summary, indent=2) + '\n')
print('built-html-contract: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "built HTML docs contract drifted" "$log_path" "$BUILT_HTML_DIR"
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
import json
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

for rel in [
    'README.md',
    'scaffold.rs',
    'clustering.SKILL.md',
    'production-backend-proof.index.md',
    'e2e_m051_s04.rs',
    'verify-m051-s04.sh',
    'retained-m051-s04-artifacts.manifest.txt',
]:
    if not (bundle_root / rel).is_file():
        raise SystemExit(f'{bundle_root}: missing required retained file {rel}')

for rel_dir, required in {
    'retained-m050-s01-verify': ['status.txt', 'phase-report.txt', 'latest-proof-bundle.txt', 'built-html/summary.json'],
    'retained-m050-s02-verify': ['status.txt', 'phase-report.txt', 'latest-proof-bundle.txt', 'built-html/summary.json'],
    'retained-m050-s03-verify': ['status.txt', 'phase-report.txt', 'latest-proof-bundle.txt', 'built-html/summary.json'],
}.items():
    base = bundle_root / rel_dir
    if not base.is_dir():
        raise SystemExit(f'{bundle_root}: missing {rel_dir}')
    for rel in required:
        if not (base / rel).exists():
            raise SystemExit(f'{base}: missing {rel}')

built_html_dir = bundle_root / 'built-html'
for rel in [
    'getting-started.index.html',
    'clustered-example.index.html',
    'tooling.index.html',
    'distributed.index.html',
    'distributed-proof.index.html',
    'production-backend-proof.index.html',
    'summary.json',
]:
    if not (built_html_dir / rel).is_file():
        raise SystemExit(f'{built_html_dir}: missing {rel}')
summary = json.loads((built_html_dir / 'summary.json').read_text(errors='replace'))
for key in [
    'getting_started',
    'clustered_example',
    'tooling',
    'distributed',
    'distributed_proof',
    'production_backend_proof',
]:
    if key not in summary:
        raise SystemExit(f'built HTML summary missing key {key!r}')

manifest_lines = [line for line in manifest_path.read_text(errors='replace').splitlines() if line.strip()]
if not manifest_lines:
    raise SystemExit(f'expected non-empty copied-artifact manifest: {manifest_path}')

artifacts_root = bundle_root / 'retained-m051-s04-artifacts'
children = [path for path in artifacts_root.iterdir() if path.is_dir()]
for prefix, required in {
    'public-surface-contract-': ['README.md', 'website__docs__docs__tooling__index.md'],
    'verifier-contract-': ['scripts__verify-m051-s04.sh', 'scripts__verify-m050-s01.sh', 'scripts__verify-m050-s02.sh', 'scripts__verify-m050-s03.sh'],
}.items():
    matches = [path for path in children if path.name.startswith(prefix)]
    if len(matches) != 1:
        raise SystemExit(f'{artifacts_root}: expected exactly one retained artifact for {prefix}, found {[path.name for path in matches]}')
    for rel in required:
        if not (matches[0] / rel).exists():
            raise SystemExit(f'{matches[0]}: missing {rel}')

print('retained-bundle-shape: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "retained proof bundle pointer or artifact shape drifted" "$log_path" "$bundle_root"
  fi
}

record_phase init started
for command_name in node npm cargo python3 rg bash; do
  require_command init "$command_name" "required command for the M051 S04 assembled replay"
done
for path in \
  "$ROOT_DIR/scripts/tests/verify-m049-s04-onboarding-contract.test.mjs" \
  "$ROOT_DIR/scripts/tests/verify-m048-s04-skill-contract.test.mjs" \
  "$ROOT_DIR/scripts/verify-m050-s01.sh" \
  "$ROOT_DIR/scripts/verify-m050-s02.sh" \
  "$ROOT_DIR/scripts/verify-m050-s03.sh" \
  "$ROOT_DIR/compiler/meshc/tests/e2e_m051_s04.rs"; do
  require_file init "$path" "required S04 surface"
done
record_phase init passed

capture_snapshot "$ROOT_DIR/$ARTIFACT_ROOT" "$M051_S04_SNAPSHOT_PATH" verify

run_expect_success onboarding-contract onboarding-contract no 300 "scripts/tests/verify-m049-s04-onboarding-contract.test.mjs" \
  node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs
run_expect_success skill-contract skill-contract no 300 "scripts/tests/verify-m048-s04-skill-contract.test.mjs" \
  node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs
run_expect_success m050-s01-wrapper m050-s01-wrapper no 2400 ".tmp/m050-s01/verify" \
  bash scripts/verify-m050-s01.sh
run_expect_success m050-s02-wrapper m050-s02-wrapper no 2400 ".tmp/m050-s02/verify" \
  bash scripts/verify-m050-s02.sh
run_expect_success m050-s03-wrapper m050-s03-wrapper no 2400 ".tmp/m050-s03/verify" \
  bash scripts/verify-m050-s03.sh
run_expect_success m051-s04-contract m051-s04-contract yes 2400 ".tmp/m051-s04" \
  cargo test -p meshc --test e2e_m051_s04 -- --nocapture
run_expect_success docs-build docs-build no 2400 "website/docs/.vitepress/dist/docs" \
  npm --prefix website run build

begin_phase retain-m050-s01-verify
copy_fixed_dir_or_fail retain-m050-s01-verify \
  "$ROOT_DIR/.tmp/m050-s01/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m050-s01-verify" \
  "M050 S01 verify artifacts are missing or malformed" \
  status.txt \
  phase-report.txt \
  latest-proof-bundle.txt \
  built-html/summary.json
record_phase retain-m050-s01-verify passed

begin_phase retain-m050-s02-verify
copy_fixed_dir_or_fail retain-m050-s02-verify \
  "$ROOT_DIR/.tmp/m050-s02/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m050-s02-verify" \
  "M050 S02 verify artifacts are missing or malformed" \
  status.txt \
  phase-report.txt \
  latest-proof-bundle.txt \
  built-html/summary.json
record_phase retain-m050-s02-verify passed

begin_phase retain-m050-s03-verify
copy_fixed_dir_or_fail retain-m050-s03-verify \
  "$ROOT_DIR/.tmp/m050-s03/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m050-s03-verify" \
  "M050 S03 verify artifacts are missing or malformed" \
  status.txt \
  phase-report.txt \
  latest-proof-bundle.txt \
  built-html/summary.json
record_phase retain-m050-s03-verify passed

begin_phase retain-m051-s04-artifacts
copy_new_prefixed_artifacts_or_fail \
  retain-m051-s04-artifacts \
  "$M051_S04_SNAPSHOT_PATH" \
  "$ROOT_DIR/$ARTIFACT_ROOT" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s04-artifacts" \
  "$RETAINED_M051_S04_ARTIFACTS_MANIFEST_PATH" \
  "expected fresh .tmp/m051-s04 artifact directories from e2e_m051_s04" \
  public-surface-contract- \
  verifier-contract-
record_phase retain-m051-s04-artifacts passed

begin_phase retain-built-html
copy_file_or_fail retain-built-html "$GETTING_STARTED_HTML_PATH" "$BUILT_HTML_DIR/getting-started.index.html" "missing built Getting Started HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$CLUSTERED_EXAMPLE_HTML_PATH" "$BUILT_HTML_DIR/clustered-example.index.html" "missing built Clustered Example HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$TOOLING_HTML_PATH" "$BUILT_HTML_DIR/tooling.index.html" "missing built Tooling HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$DISTRIBUTED_HTML_PATH" "$BUILT_HTML_DIR/distributed.index.html" "missing built Distributed HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$DISTRIBUTED_PROOF_HTML_PATH" "$BUILT_HTML_DIR/distributed-proof.index.html" "missing built Distributed Proof HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$PRODUCTION_BACKEND_PROOF_HTML_PATH" "$BUILT_HTML_DIR/production-backend-proof.index.html" "missing built Production Backend Proof HTML snapshot after docs build"
record_phase retain-built-html passed

begin_phase built-html
assert_built_html_contract built-html "$BUILT_HTML_SUMMARY_PATH"
record_phase built-html passed

begin_phase m051-s04-bundle-shape
mkdir -p "$RETAINED_PROOF_BUNDLE_DIR"
cp "$ROOT_DIR/README.md" "$RETAINED_PROOF_BUNDLE_DIR/README.md"
cp "$ROOT_DIR/compiler/mesh-pkg/src/scaffold.rs" "$RETAINED_PROOF_BUNDLE_DIR/scaffold.rs"
cp "$ROOT_DIR/tools/skill/mesh/skills/clustering/SKILL.md" "$RETAINED_PROOF_BUNDLE_DIR/clustering.SKILL.md"
cp "$ROOT_DIR/website/docs/docs/production-backend-proof/index.md" "$RETAINED_PROOF_BUNDLE_DIR/production-backend-proof.index.md"
cp "$ROOT_DIR/compiler/meshc/tests/e2e_m051_s04.rs" "$RETAINED_PROOF_BUNDLE_DIR/e2e_m051_s04.rs"
cp "$ROOT_DIR/scripts/verify-m051-s04.sh" "$RETAINED_PROOF_BUNDLE_DIR/verify-m051-s04.sh"
cp "$RETAINED_M051_S04_ARTIFACTS_MANIFEST_PATH" "$RETAINED_PROOF_BUNDLE_DIR/retained-m051-s04-artifacts.manifest.txt"
cp -R "$BUILT_HTML_DIR" "$RETAINED_PROOF_BUNDLE_DIR/built-html"
printf '%s\n' "$RETAINED_PROOF_BUNDLE_DIR" >"$LATEST_PROOF_BUNDLE_PATH"
assert_retained_bundle_shape \
  m051-s04-bundle-shape \
  "$RETAINED_PROOF_BUNDLE_DIR" \
  "$LATEST_PROOF_BUNDLE_PATH" \
  "$RETAINED_M051_S04_ARTIFACTS_MANIFEST_PATH"
record_phase m051-s04-bundle-shape passed

for expected_phase in \
  init \
  onboarding-contract \
  skill-contract \
  m050-s01-wrapper \
  m050-s02-wrapper \
  m050-s03-wrapper \
  m051-s04-contract \
  docs-build \
  retain-m050-s01-verify \
  retain-m050-s02-verify \
  retain-m050-s03-verify \
  retain-m051-s04-artifacts \
  retain-built-html \
  built-html \
  m051-s04-bundle-shape; do
  if ! rg -q "^${expected_phase}\\tpassed$" "$PHASE_REPORT_PATH"; then
    fail_phase verifier-status "phase report missing passed marker for ${expected_phase}" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m051-s04: ok"
echo "artifacts: $(repo_rel "$ARTIFACT_DIR")"
echo "proof bundle: $(repo_rel "$RETAINED_PROOF_BUNDLE_DIR")"
