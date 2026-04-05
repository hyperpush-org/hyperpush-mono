#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m050-s02"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
BUILT_HTML_DIR="$ARTIFACT_DIR/built-html"
BUILT_HTML_SUMMARY_PATH="$BUILT_HTML_DIR/summary.json"
GETTING_STARTED_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/getting-started/index.html"
CLUSTERED_EXAMPLE_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/getting-started/clustered-example/index.html"
TOOLING_HTML_PATH="$ROOT_DIR/website/docs/.vitepress/dist/docs/tooling/index.html"

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

assert_built_html_contract() {
  local phase="$1"
  local getting_started_path="$2"
  local clustered_example_path="$3"
  local tooling_path="$4"
  local summary_path="$5"
  local log_path="$ARTIFACT_DIR/${phase}.assert.log"

  if ! python3 - \
    "$getting_started_path" \
    "$clustered_example_path" \
    "$tooling_path" \
    "$summary_path" >"$log_path" 2>&1 <<'PY'
from html.parser import HTMLParser
from pathlib import Path
import json
import re
import sys

getting_started_path = Path(sys.argv[1])
clustered_example_path = Path(sys.argv[2])
tooling_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])

CURRENT_REPO_BLOB_BASE = 'https://github.com/snowdamiz/mesh-lang/blob/main/'
STALE_REPO_BLOB_BASE = 'https://github.com/hyperpush-org/hyperpush-mono/blob/main/'


class MainTextExtractor(HTMLParser):
    def __init__(self):
        super().__init__()
        self.parts = []
        self.skip_depth = 0

    def handle_starttag(self, tag, attrs):
        if tag in {'script', 'style'}:
            self.skip_depth += 1

    def handle_endtag(self, tag):
        if tag in {'script', 'style'} and self.skip_depth:
            self.skip_depth -= 1

    def handle_data(self, data):
        if not self.skip_depth:
            self.parts.append(data)


def load_main(path: Path):
    if not path.is_file():
        raise SystemExit(f'missing built HTML snapshot: {path}')
    html = path.read_text(errors='replace')
    match = re.search(r'<main(?:\s[^>]*)?>(?P<body>[\s\S]*?)</main>', html)
    if not match:
        raise SystemExit(f'missing <main> content in {path}')
    extractor = MainTextExtractor()
    extractor.feed(match.group('body'))
    text = ' '.join(' '.join(extractor.parts).split())
    return html, text


def marker_map(text: str, markers: list[str], label: str):
    positions = {}
    cursor = -1
    for marker in markers:
        index = text.find(marker)
        if index == -1:
            raise SystemExit(f'{label}: missing marker {marker!r}')
        if index <= cursor:
            raise SystemExit(f'{label}: marker order drifted around {marker!r}')
        positions[marker] = index
        cursor = index
    return positions


getting_started_html, getting_started_text = load_main(getting_started_path)
clustered_example_html, clustered_example_text = load_main(clustered_example_path)
tooling_html, tooling_text = load_main(tooling_path)

summary = {
    'getting_started': {
        'path': str(getting_started_path),
        'text_length': len(getting_started_text),
        'markers': marker_map(
            getting_started_text,
            [
                'Hello World',
                'Choose your next starter',
                'meshc init --clustered',
                'meshc init --template todo-api --db sqlite',
                'meshc init --template todo-api --db postgres',
                'Clustered Example',
                'Production Backend Proof',
            ],
            'getting-started',
        ),
    },
    'clustered_example': {
        'path': str(clustered_example_path),
        'text_length': len(clustered_example_text),
        'markers': marker_map(
            clustered_example_text,
            [
                'meshc init --clustered hello_cluster',
                'After the scaffold, pick the follow-on starter',
                'meshc init --template todo-api --db sqlite my_local_todo',
                'meshc init --template todo-api --db postgres my_shared_todo',
                'Need the retained verifier map?',
                'Distributed Proof',
            ],
            'clustered-example',
        ),
    },
    'tooling': {
        'path': str(tooling_path),
        'text_length': len(tooling_text),
        'markers': marker_map(
            tooling_text,
            [
                'Install the CLI tools',
                'Update an installed toolchain',
                'Creating a New Project',
                'meshc init --clustered',
                'meshc init --template todo-api --db sqlite',
                'meshc init --template todo-api --db postgres',
                'Inspect a running clustered app with the same operator order used by the scaffold',
                'Editor Support',
                'Assembled first-contact docs verifier',
                'bash scripts/verify-m050-s02.sh',
                'Release Assembly Runbook',
                'Assembled contract verifier',
                'bash scripts/verify-m048-s05.sh',
                'Assembled scaffold/example verifier',
                'bash scripts/verify-m049-s05.sh',
            ],
            'tooling',
        ),
    },
}

for marker in [
    '### Support tiers',
    '### VS Code',
    '### Neovim',
    '### Best-effort editors',
    'bash scripts/verify-m036-s03.sh',
]:
    if marker.replace('### ', '') not in tooling_text and marker not in tooling_text:
        raise SystemExit(f'tooling: missing retained editor marker {marker!r}')

for html_label, html in [
    ('tooling', tooling_html),
    ('clustered-example', clustered_example_html),
]:
    if STALE_REPO_BLOB_BASE in html:
        raise SystemExit(f'{html_label}: stale repo link leaked into built HTML')

for current_link in [
    f'{CURRENT_REPO_BLOB_BASE}examples/todo-postgres/README.md',
    f'{CURRENT_REPO_BLOB_BASE}examples/todo-sqlite/README.md',
]:
    if current_link not in tooling_html:
        raise SystemExit(f'tooling: missing current repo link {current_link!r} in built HTML')

if '/docs/production-backend-proof/' not in tooling_html:
    raise SystemExit("tooling: missing /docs/production-backend-proof/ link in built HTML")

summary_path.write_text(json.dumps(summary, indent=2) + '\n')
print('built-html-contract: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "built HTML first-contact contract drifted" "$log_path" "$BUILT_HTML_DIR"
  fi
}

assert_bundle_shape() {
  local phase="$1"
  local artifact_dir="$2"
  local pointer_path="$3"
  local built_html_dir="$4"
  local summary_path="$5"
  local log_path="$ARTIFACT_DIR/${phase}.bundle-check.log"

  if ! python3 - "$artifact_dir" "$pointer_path" "$built_html_dir" "$summary_path" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import json
import sys

artifact_dir = Path(sys.argv[1])
pointer_path = Path(sys.argv[2])
built_html_dir = Path(sys.argv[3])
summary_path = Path(sys.argv[4])
expected_pointer = str(artifact_dir)
actual_pointer = pointer_path.read_text(errors='replace').strip()
if actual_pointer != expected_pointer:
    raise SystemExit(
        f"latest-proof-bundle pointer drifted: expected {expected_pointer!r}, got {actual_pointer!r}"
    )

required_files = [
    'status.txt',
    'current-phase.txt',
    'phase-report.txt',
    'full-contract.log',
    'latest-proof-bundle.txt',
    'first-contact-contract.log',
    'm047-s05-docs-contract.log',
    'm047-s06-docs-contract.log',
    'm048-s05-tooling-contract.log',
    'm036-s03-tooling-contract.log',
    'docs-build.log',
]
for rel in required_files:
    path = artifact_dir / rel
    if not path.is_file():
        raise SystemExit(f'missing required verify file: {path}')
    if not path.read_text(errors='replace').strip():
        raise SystemExit(f'expected non-empty verify file: {path}')

if not built_html_dir.is_dir():
    raise SystemExit(f'missing built HTML evidence directory: {built_html_dir}')
for rel in [
    'getting-started.index.html',
    'clustered-example.index.html',
    'tooling.index.html',
    'summary.json',
]:
    path = built_html_dir / rel
    if not path.is_file():
        raise SystemExit(f'missing built HTML artifact: {path}')
    if not path.read_text(errors='replace').strip():
        raise SystemExit(f'expected non-empty built HTML artifact: {path}')

summary = json.loads(summary_path.read_text(errors='replace'))
for key in ['getting_started', 'clustered_example', 'tooling']:
    if key not in summary:
        raise SystemExit(f'built HTML summary missing key {key!r}')
    markers = summary[key].get('markers')
    if not isinstance(markers, dict) or not markers:
        raise SystemExit(f'built HTML summary missing marker map for {key!r}')

phase_report = (artifact_dir / 'phase-report.txt').read_text(errors='replace')
for marker in [
    'init\tpassed',
    'first-contact-contract\tpassed',
    'm047-s05-docs-contract\tpassed',
    'm047-s06-docs-contract\tpassed',
    'm048-s05-tooling-contract\tpassed',
    'm036-s03-tooling-contract\tpassed',
    'docs-build\tpassed',
    'retain-built-html\tpassed',
    'built-html\tpassed',
]:
    if marker not in phase_report:
        raise SystemExit(f'phase report missing marker {marker!r}')

print('bundle-shape: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "missing built HTML evidence, malformed bundle pointer, or malformed verify bundle" "$log_path" "$artifact_dir"
  fi
}

require_command init node "Node.js is required for the first-contact contract" "scripts/tests/verify-m050-s02-first-contact-contract.test.mjs"
require_command init npm "npm is required for the VitePress build" "website/package.json"
require_command init cargo "cargo is required for the retained docs/tooling Rust rails" "compiler/meshc/tests"
require_command init python3 "python3 is required for built HTML assertions" "$BUILT_HTML_DIR"
require_command init rg "rg is required for final phase-marker checks" "$PHASE_REPORT_PATH"
require_file init "$ROOT_DIR/scripts/tests/verify-m050-s02-first-contact-contract.test.mjs" "M050 first-contact source contract" "scripts/tests/verify-m050-s02-first-contact-contract.test.mjs"
require_file init "$ROOT_DIR/scripts/tests/verify-m048-s05-contract.test.mjs" "retained M048 tooling contract" "scripts/tests/verify-m048-s05-contract.test.mjs"
require_file init "$ROOT_DIR/scripts/tests/verify-m036-s03-contract.test.mjs" "retained M036 tooling contract" "scripts/tests/verify-m036-s03-contract.test.mjs"
record_phase init passed

run_expect_success first-contact-contract first-contact-contract no 300 "scripts/tests/verify-m050-s02-first-contact-contract.test.mjs" \
  node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs
run_expect_success m047-s05-docs-contract m047-s05-docs-contract yes 2400 "compiler/meshc/tests/e2e_m047_s05.rs" \
  cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture
run_expect_success m047-s06-docs-contract m047-s06-docs-contract yes 2400 "compiler/meshc/tests/e2e_m047_s06.rs" \
  cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture
run_expect_success m048-s05-tooling-contract m048-s05-tooling-contract no 300 "scripts/tests/verify-m048-s05-contract.test.mjs" \
  node --test scripts/tests/verify-m048-s05-contract.test.mjs
run_expect_success m036-s03-tooling-contract m036-s03-tooling-contract no 300 "scripts/tests/verify-m036-s03-contract.test.mjs" \
  node --test scripts/tests/verify-m036-s03-contract.test.mjs
run_expect_success docs-build docs-build no 2400 "website/docs/.vitepress/dist/docs" \
  npm --prefix website run build

begin_phase retain-built-html
copy_file_or_fail retain-built-html "$GETTING_STARTED_HTML_PATH" "$BUILT_HTML_DIR/getting-started.index.html" "missing built Getting Started HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$CLUSTERED_EXAMPLE_HTML_PATH" "$BUILT_HTML_DIR/clustered-example.index.html" "missing built Clustered Example HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$TOOLING_HTML_PATH" "$BUILT_HTML_DIR/tooling.index.html" "missing built Tooling HTML snapshot after docs build"
record_phase retain-built-html passed

begin_phase built-html
assert_built_html_contract \
  built-html \
  "$BUILT_HTML_DIR/getting-started.index.html" \
  "$BUILT_HTML_DIR/clustered-example.index.html" \
  "$BUILT_HTML_DIR/tooling.index.html" \
  "$BUILT_HTML_SUMMARY_PATH"
record_phase built-html passed

begin_phase m050-s02-bundle-shape
assert_bundle_shape \
  m050-s02-bundle-shape \
  "$ARTIFACT_DIR" \
  "$LATEST_PROOF_BUNDLE_PATH" \
  "$BUILT_HTML_DIR" \
  "$BUILT_HTML_SUMMARY_PATH"
record_phase m050-s02-bundle-shape passed

for expected_phase in \
  init \
  first-contact-contract \
  m047-s05-docs-contract \
  m047-s06-docs-contract \
  m048-s05-tooling-contract \
  m036-s03-tooling-contract \
  docs-build \
  retain-built-html \
  built-html \
  m050-s02-bundle-shape; do
  if ! rg -q "^${expected_phase}\\tpassed$" "$PHASE_REPORT_PATH"; then
    fail_phase verifier-status "missing ${expected_phase} pass marker" "$ARTIFACT_DIR/full-contract.log" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m050-s02: ok"
echo "artifacts: $(repo_rel "$ARTIFACT_DIR")"
echo "proof bundle: $(repo_rel "$ARTIFACT_DIR")"
