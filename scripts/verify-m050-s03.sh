#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m050-s03"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
BUILT_HTML_DIR="$ARTIFACT_DIR/built-html"
BUILT_HTML_SUMMARY_PATH="$BUILT_HTML_DIR/summary.json"
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
  local distributed_path="$2"
  local distributed_proof_path="$3"
  local production_backend_path="$4"
  local summary_path="$5"
  local log_path="$ARTIFACT_DIR/${phase}.assert.log"

  if ! python3 - \
    "$distributed_path" \
    "$distributed_proof_path" \
    "$production_backend_path" \
    "$summary_path" >"$log_path" 2>&1 <<'PY'
from html.parser import HTMLParser
from pathlib import Path
import json
import re
import sys

distributed_path = Path(sys.argv[1])
distributed_proof_path = Path(sys.argv[2])
production_backend_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])

mesher_runbook_link = 'https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md'
sqlite_link = 'https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md'
postgres_link = 'https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md'
proof_direct_rails = [
    'bash scripts/verify-m047-s04.sh',
    'bash scripts/verify-m047-s05.sh',
    'bash scripts/verify-m047-s06.sh',
    'cargo test -p meshc --test e2e_m047_s07 -- --nocapture',
    'bash scripts/verify-m043-s04-fly.sh --help',
]
proof_page_forbidden_first_contact = [
    'meshc init --clustered',
    'meshc init --template todo-api --db sqlite',
    'meshc init --template todo-api --db postgres',
    'curl -sSf https://meshlang.dev/install.sh | sh',
    'irm https://meshlang.dev/install.ps1 | iex',
    'Install the CLI tools',
    'Choose your next starter',
]


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
    main_html = match.group('body')
    extractor = MainTextExtractor()
    extractor.feed(main_html)
    text = ' '.join(' '.join(extractor.parts).split())
    return main_html, text



def require_substrings(label: str, haystack: str, needles: list[str]) -> None:
    for needle in needles:
        if needle not in haystack:
            raise SystemExit(f'{label}: missing marker {needle!r}')



def require_absent(label: str, haystack: str, needles: list[str]) -> None:
    for needle in needles:
        if needle in haystack:
            raise SystemExit(f'{label}: stale marker leaked into built HTML {needle!r}')



def marker_map(text: str, markers: list[str], label: str):
    positions = {}
    cursor = -1
    for marker in markers:
        index = text.find(marker)
        if index == -1:
            raise SystemExit(f'{label}: missing ordered marker {marker!r}')
        if index <= cursor:
            raise SystemExit(f'{label}: marker order drifted around {marker!r}')
        positions[marker] = index
        cursor = index
    return positions


summary = {}

distributed_html, distributed_text = load_main(distributed_path)
require_substrings(
    'distributed',
    distributed_text,
    [
        'Distributed Actors',
        'Clustered proof surfaces:',
        'Clustered Example',
        'Distributed Proof',
        'Production Backend Proof',
    ],
)
require_substrings(
    'distributed',
    distributed_html,
    [
        '/docs/getting-started/clustered-example/',
        '/docs/distributed-proof/',
        '/docs/production-backend-proof/',
        mesher_runbook_link,
        sqlite_link,
        postgres_link,
    ],
)
require_absent('distributed', distributed_text, proof_direct_rails)
summary['distributed'] = {
    'path': str(distributed_path),
    'text_length': len(distributed_text),
    'markers': marker_map(
        distributed_text,
        [
            'Clustered proof surfaces:',
            'Clustered Example',
            'Distributed Proof',
            'Production Backend Proof',
        ],
        'distributed',
    ),
    'links': {
        'clustered_example': '/docs/getting-started/clustered-example/' in distributed_html,
        'distributed_proof': '/docs/distributed-proof/' in distributed_html,
        'production_backend_proof': '/docs/production-backend-proof/' in distributed_html,
        'runbook': mesher_runbook_link in distributed_html,
        'todo_sqlite': sqlite_link in distributed_html,
        'todo_postgres': postgres_link in distributed_html,
    },
}

distributed_proof_html, distributed_proof_text = load_main(distributed_proof_path)
require_substrings(
    'distributed-proof',
    distributed_proof_text,
    [
        'This is the only public-secondary docs page that carries the named clustered verifier rails.',
        'Public surfaces and verifier rails',
        'Named proof commands',
        'Operator workflow across the public clustered surfaces',
        'Failure inspection map',
    ]
    + proof_direct_rails,
)
require_substrings(
    'distributed-proof',
    distributed_proof_html,
    [
        '/docs/distributed/',
        '/docs/getting-started/clustered-example/',
        '/docs/production-backend-proof/',
        mesher_runbook_link,
        sqlite_link,
        postgres_link,
    ],
)
summary['distributed_proof'] = {
    'path': str(distributed_proof_path),
    'text_length': len(distributed_proof_text),
    'markers': marker_map(
        distributed_proof_text,
        [
            'This is the only public-secondary docs page that carries the named clustered verifier rails.',
            'Public surfaces and verifier rails',
            'Named proof commands',
            'Operator workflow across the public clustered surfaces',
            'Failure inspection map',
        ],
        'distributed-proof',
    ),
    'links': {
        'distributed': '/docs/distributed/' in distributed_proof_html,
        'clustered_example': '/docs/getting-started/clustered-example/' in distributed_proof_html,
        'production_backend_proof': '/docs/production-backend-proof/' in distributed_proof_html,
        'runbook': mesher_runbook_link in distributed_proof_html,
        'todo_sqlite': sqlite_link in distributed_proof_html,
        'todo_postgres': postgres_link in distributed_proof_html,
    },
}

production_backend_html, production_backend_text = load_main(production_backend_path)
require_substrings(
    'production-backend-proof',
    production_backend_text,
    [
        "This is the compact public-secondary handoff for Mesh's backend proof story.",
        'Canonical surfaces',
        'Named maintainer verifiers',
        'Retained backend-only recovery signals',
        'When to use this page vs the generic guides',
        'Failure inspection map',
        'bash reference-backend/scripts/verify-production-proof-surface.sh',
        'restart_count',
        'recovery_active',
    ],
)
require_substrings(
    'production-backend-proof',
    production_backend_html,
    [
        '/docs/web/',
        '/docs/databases/',
        '/docs/testing/',
        '/docs/concurrency/',
        '/docs/tooling/',
        mesher_runbook_link,
    ],
)
require_absent('production-backend-proof', production_backend_text, proof_page_forbidden_first_contact)
summary['production_backend_proof'] = {
    'path': str(production_backend_path),
    'text_length': len(production_backend_text),
    'markers': marker_map(
        production_backend_text,
        [
            "This is the compact public-secondary handoff for Mesh's backend proof story.",
            'Canonical surfaces',
            'Named maintainer verifiers',
            'Retained backend-only recovery signals',
            'When to use this page vs the generic guides',
            'Failure inspection map',
        ],
        'production-backend-proof',
    ),
    'links': {
        'web': '/docs/web/' in production_backend_html,
        'databases': '/docs/databases/' in production_backend_html,
        'testing': '/docs/testing/' in production_backend_html,
        'concurrency': '/docs/concurrency/' in production_backend_html,
        'tooling': '/docs/tooling/' in production_backend_html,
        'runbook': mesher_runbook_link in production_backend_html,
    },
}

summary_path.write_text(json.dumps(summary, indent=2) + '\n')
print('built-html-contract: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "built HTML secondary-surface contract drifted" "$log_path" "$BUILT_HTML_DIR"
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
import re
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
    'secondary-surfaces-contract.log',
    'm047-s04-docs-contract.log',
    'm047-s05-docs-contract.log',
    'm047-s06-docs-contract.log',
    'production-proof-surface.log',
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
    'distributed.index.html',
    'distributed-proof.index.html',
    'production-backend-proof.index.html',
    'summary.json',
]:
    path = built_html_dir / rel
    if not path.is_file():
        raise SystemExit(f'missing built HTML artifact: {path}')
    if not path.read_text(errors='replace').strip():
        raise SystemExit(f'expected non-empty built HTML artifact: {path}')

summary = json.loads(summary_path.read_text(errors='replace'))
for key in ['distributed', 'distributed_proof', 'production_backend_proof']:
    if key not in summary:
        raise SystemExit(f'built HTML summary missing key {key!r}')
    markers = summary[key].get('markers')
    if not isinstance(markers, dict) or not markers:
        raise SystemExit(f'built HTML summary missing marker map for {key!r}')

phase_report = (artifact_dir / 'phase-report.txt').read_text(errors='replace')
for marker in [
    'init\tpassed',
    'secondary-surfaces-contract\tpassed',
    'm047-s04-docs-contract\tpassed',
    'm047-s05-docs-contract\tpassed',
    'm047-s06-docs-contract\tpassed',
    'production-proof-surface\tpassed',
    'docs-build\tpassed',
    'retain-built-html\tpassed',
    'built-html\tpassed',
]:
    if marker not in phase_report:
        raise SystemExit(f'phase report missing marker {marker!r}')

full_contract_log = (artifact_dir / 'full-contract.log').read_text(errors='replace')
if 'DATABASE_URL=' in full_contract_log:
    raise SystemExit('verify log leaked DATABASE_URL text despite the env-free contract')
if re.search(r'postgres(?:ql)?://', full_contract_log):
    raise SystemExit('verify log leaked a Postgres connection string despite the env-free contract')

print('bundle-shape: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "missing built HTML evidence, malformed bundle pointer, or malformed verify bundle" "$log_path" "$artifact_dir"
  fi
}

require_command init node "Node.js is required for the S03 secondary-surface contract" "scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs"
require_command init npm "npm is required for the VitePress build" "website/package.json"
require_command init cargo "cargo is required for the retained docs-contract Rust rails" "compiler/meshc/tests"
require_command init python3 "python3 is required for built HTML assertions" "$BUILT_HTML_DIR"
require_command init rg "rg is required for final phase-marker checks" "$PHASE_REPORT_PATH"
require_file init "$ROOT_DIR/scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs" "M050 S03 secondary-surface source contract" "scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs"
require_file init "$ROOT_DIR/reference-backend/scripts/verify-production-proof-surface.sh" "production proof-surface verifier" "reference-backend/scripts/verify-production-proof-surface.sh"
record_phase init passed

run_expect_success secondary-surfaces-contract secondary-surfaces-contract no 300 "scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs" \
  node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs
run_expect_success m047-s04-docs-contract m047-s04-docs-contract yes 1800 "compiler/meshc/tests/e2e_m047_s04.rs" \
  cargo test -p meshc --test e2e_m047_s04 m047_s04_ -- --nocapture
run_expect_success m047-s05-docs-contract m047-s05-docs-contract yes 1800 "compiler/meshc/tests/e2e_m047_s05.rs" \
  cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture
run_expect_success m047-s06-docs-contract m047-s06-docs-contract yes 1800 "compiler/meshc/tests/e2e_m047_s06.rs" \
  cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture
run_expect_success production-proof-surface production-proof-surface no 300 "reference-backend/scripts/verify-production-proof-surface.sh" \
  bash reference-backend/scripts/verify-production-proof-surface.sh
run_expect_success docs-build docs-build no 1800 "website/docs/.vitepress/dist/docs" \
  npm --prefix website run build

begin_phase retain-built-html
copy_file_or_fail retain-built-html "$DISTRIBUTED_HTML_PATH" "$BUILT_HTML_DIR/distributed.index.html" "missing built Distributed Actors HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$DISTRIBUTED_PROOF_HTML_PATH" "$BUILT_HTML_DIR/distributed-proof.index.html" "missing built Distributed Proof HTML snapshot after docs build"
copy_file_or_fail retain-built-html "$PRODUCTION_BACKEND_PROOF_HTML_PATH" "$BUILT_HTML_DIR/production-backend-proof.index.html" "missing built Production Backend Proof HTML snapshot after docs build"
record_phase retain-built-html passed

begin_phase built-html
assert_built_html_contract \
  built-html \
  "$BUILT_HTML_DIR/distributed.index.html" \
  "$BUILT_HTML_DIR/distributed-proof.index.html" \
  "$BUILT_HTML_DIR/production-backend-proof.index.html" \
  "$BUILT_HTML_SUMMARY_PATH"
record_phase built-html passed

begin_phase m050-s03-bundle-shape
assert_bundle_shape \
  m050-s03-bundle-shape \
  "$ARTIFACT_DIR" \
  "$LATEST_PROOF_BUNDLE_PATH" \
  "$BUILT_HTML_DIR" \
  "$BUILT_HTML_SUMMARY_PATH"
record_phase m050-s03-bundle-shape passed

for expected_phase in \
  init \
  secondary-surfaces-contract \
  m047-s04-docs-contract \
  m047-s05-docs-contract \
  m047-s06-docs-contract \
  production-proof-surface \
  docs-build \
  retain-built-html \
  built-html \
  m050-s03-bundle-shape; do
  if ! rg -q "^${expected_phase}\\tpassed$" "$PHASE_REPORT_PATH"; then
    fail_phase verifier-status "missing ${expected_phase} pass marker" "$ARTIFACT_DIR/full-contract.log" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m050-s03: ok"
echo "artifacts: $(repo_rel "$ARTIFACT_DIR")"
echo "proof bundle: $(repo_rel "$ARTIFACT_DIR")"
