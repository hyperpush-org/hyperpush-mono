#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERIFY_ROOT="$ROOT_DIR/.tmp/m034-s05/verify"
CURRENT_PHASE_PATH="$VERIFY_ROOT/current-phase.txt"
FAILED_PHASE_PATH="$VERIFY_ROOT/failed-phase.txt"
STATUS_PATH="$VERIFY_ROOT/status.txt"
PHASE_REPORT_PATH="$VERIFY_ROOT/phase-report.txt"
PUBLIC_HTTP_LOG="$VERIFY_ROOT/public-http.log"
CANDIDATE_TAGS_PATH="$VERIFY_ROOT/candidate-tags.json"
REMOTE_RUNS_PATH="$VERIFY_ROOT/remote-runs.json"
GH_REPO="snowdamiz/mesh-lang"
PUBLIC_SURFACE_HELPER="$ROOT_DIR/scripts/lib/m034_public_surface_contract.py"
LAST_LOG_PATH=""
STOP_AFTER_PHASE=""

print_usage() {
  cat <<'EOF' >&2
usage: bash scripts/verify-m034-s05.sh [--stop-after remote-evidence]
EOF
}

parse_args() {
  local requested_phase="${VERIFY_M034_S05_STOP_AFTER:-}"

  if [[ "$#" -gt 0 ]]; then
    if [[ "$#" -ne 2 || "$1" != "--stop-after" ]]; then
      print_usage
      exit 64
    fi
    requested_phase="$2"
  fi

  case "$requested_phase" in
    "")
      STOP_AFTER_PHASE=""
      ;;
    remote-evidence)
      STOP_AFTER_PHASE="$requested_phase"
      ;;
    *)
      echo "unsupported stop-after phase: ${requested_phase}" >&2
      print_usage
      exit 64
      ;;
  esac
}

should_stop_after_phase() {
  local phase_name="$1"
  [[ -n "$STOP_AFTER_PHASE" && "$STOP_AFTER_PHASE" == "$phase_name" ]]
}

complete_stop_after_phase() {
  local phase_name="$1"

  if [[ "$STOP_AFTER_PHASE" != "$phase_name" ]]; then
    fail_phase "$phase_name" "stop-after contract drifted while completing ${phase_name}"
  fi

  printf 'ok\n' >"$STATUS_PATH"
  printf 'stopped-after-%s\n' "$phase_name" >"$CURRENT_PHASE_PATH"
  rm -f "$FAILED_PHASE_PATH"

  echo "verify-m034-s05: stopped after ${phase_name}"
  exit 0
}

prepare_verify_root() {
  rm -rf "$VERIFY_ROOT"
  mkdir -p "$VERIFY_ROOT"
  printf 'bootstrap\n' >"$CURRENT_PHASE_PATH"
  printf 'running\n' >"$STATUS_PATH"
  : >"$PHASE_REPORT_PATH"
  : >"$PUBLIC_HTTP_LOG"
  rm -f "$FAILED_PHASE_PATH"
}

set_phase() {
  local phase_name="$1"
  printf '%s\n' "$phase_name" >"$CURRENT_PHASE_PATH"
}

record_phase() {
  local phase_name="$1"
  local status="$2"
  printf '%s\t%s\n' "$phase_name" "$status" >>"$PHASE_REPORT_PATH"
}

start_phase() {
  local phase_name="$1"
  local detail="$2"
  set_phase "$phase_name"
  record_phase "$phase_name" "started"
  echo "==> [${phase_name}] ${detail}"
}

finish_phase() {
  local phase_name="$1"
  record_phase "$phase_name" "passed"
}

combine_command_log() {
  local display="$1"
  local cwd="$2"
  local stdout_path="$3"
  local stderr_path="$4"
  local log_path="$5"

  {
    echo "display: ${display}"
    echo "cwd: ${cwd#$ROOT_DIR/}"
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

fail_phase() {
  local phase_name="$1"
  local reason="$2"
  local log_path="${3:-}"
  local upstream_artifacts="${4:-}"

  set_phase "$phase_name"
  printf '%s\n' "$phase_name" >"$FAILED_PHASE_PATH"
  printf 'failed\n' >"$STATUS_PATH"
  record_phase "$phase_name" "failed"

  echo "verification drift: ${reason}" >&2
  echo "first failing phase: ${phase_name}" >&2
  echo "artifacts: ${VERIFY_ROOT#$ROOT_DIR/}" >&2
  if [[ -n "$upstream_artifacts" ]]; then
    echo "upstream artifacts: ${upstream_artifacts}" >&2
  fi
  if [[ -n "$log_path" && -f "$log_path" ]]; then
    echo "--- ${log_path#$ROOT_DIR/} ---" >&2
    sed -n '1,320p' "$log_path" >&2
  fi
  exit 1
}

run_command() {
  local phase_name="$1"
  local label="$2"
  local timeout_seconds="$3"
  local cwd="$4"
  local display="$5"
  local upstream_artifacts="$6"
  shift 6

  local stdout_path="$VERIFY_ROOT/${label}.stdout"
  local stderr_path="$VERIFY_ROOT/${label}.stderr"
  local log_path="$VERIFY_ROOT/${label}.log"
  local status=0

  echo "   -> ${display}"

  if python3 - "$timeout_seconds" "$cwd" "$stdout_path" "$stderr_path" "$@" <<'PY'
from pathlib import Path
import subprocess
import sys

timeout_seconds = float(sys.argv[1])
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
  then
    status=0
  else
    status=$?
  fi

  combine_command_log "$display" "$cwd" "$stdout_path" "$stderr_path" "$log_path"
  LAST_LOG_PATH="$log_path"

  if [[ "$status" -ne 0 ]]; then
    if [[ "$status" -eq 124 ]]; then
      fail_phase "$phase_name" "${display} timed out" "$log_path" "$upstream_artifacts"
    fi
    fail_phase "$phase_name" "${display} failed" "$log_path" "$upstream_artifacts"
  fi
}

assert_file_exists() {
  local phase_name="$1"
  local file_path="$2"
  local description="$3"

  if [[ ! -f "$file_path" ]]; then
    fail_phase "$phase_name" "expected ${description} at ${file_path#$ROOT_DIR/}" "$LAST_LOG_PATH"
  fi
}

assert_file_content_exact() {
  local phase_name="$1"
  local file_path="$2"
  local expected_line="$3"
  local description="$4"

  assert_file_exists "$phase_name" "$file_path" "$description"
  if ! grep -Fxq "$expected_line" "$file_path"; then
    fail_phase "$phase_name" "${description} at ${file_path#$ROOT_DIR/} did not equal ${expected_line}" "$LAST_LOG_PATH"
  fi
}


run_prereq_sweep() {
  local phase_name="prereq-sweep"
  local log_path="$VERIFY_ROOT/prereq-sweep.log"

  start_phase "$phase_name" "verify required inputs and verifier entrypoints exist"
  if ! python3 - "$ROOT_DIR" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import json
import sys

root = Path(sys.argv[1])
required_files = [
    'scripts/lib/m034_public_surface_contract.py',
    'scripts/verify-m034-s05-workflows.sh',
    'scripts/verify-m034-s02-workflows.sh',
    'scripts/verify-m034-s03.sh',
    'scripts/verify-m034-s04-extension.sh',
    'scripts/verify-m034-s04-workflows.sh',
    'scripts/verify-m034-s01.sh',
    '.github/workflows/deploy.yml',
    '.github/workflows/deploy-services.yml',
    '.github/workflows/authoritative-verification.yml',
    '.github/workflows/release.yml',
    '.github/workflows/extension-release-proof.yml',
    '.github/workflows/publish-extension.yml',
    'README.md',
    'website/package.json',
    'website/docs/docs/getting-started/index.md',
    'website/docs/docs/tooling/index.md',
    'website/docs/public/install.sh',
    'website/docs/public/install.ps1',
    'compiler/meshc/Cargo.toml',
    'compiler/meshpkg/Cargo.toml',
    'tools/editors/vscode-mesh/package.json',
]
errors = []
for relative in required_files:
    path = root / relative
    if not path.exists():
        errors.append(f'missing required input {relative}')
    else:
        print(f'ok: {relative}')

website_package = json.loads((root / 'website/package.json').read_text())
if website_package.get('scripts', {}).get('build') != 'vitepress build docs':
    errors.append('website/package.json must keep scripts.build = "vitepress build docs"')
else:
    print('ok: website/package.json build script is vitepress build docs')

if errors:
    print()
    print('errors:')
    for error in errors:
        print(f'- {error}')
    raise SystemExit(1)
PY
  then
    fail_phase "$phase_name" "required S05 inputs were missing or drifted" "$log_path"
  fi
  LAST_LOG_PATH="$log_path"
  finish_phase "$phase_name"
}

run_candidate_tags() {
  local phase_name="candidate-tags"
  local log_path="$VERIFY_ROOT/candidate-tags.log"

  start_phase "$phase_name" "derive the binary and extension release candidate tags"
  if ! python3 - "$ROOT_DIR" "$CANDIDATE_TAGS_PATH" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import json
import re
import sys

root = Path(sys.argv[1])
output_path = Path(sys.argv[2])
errors = []


def cargo_version(path: Path) -> str:
    match = re.search(r'^version = "([^"]+)"', path.read_text(), re.MULTILINE)
    if not match:
        raise SystemExit(f"missing version in {path.relative_to(root)}")
    return match.group(1)


meshc_version = cargo_version(root / 'compiler/meshc/Cargo.toml')
meshpkg_version = cargo_version(root / 'compiler/meshpkg/Cargo.toml')
extension_package = json.loads((root / 'tools/editors/vscode-mesh/package.json').read_text())
extension_version = extension_package.get('version')
if not isinstance(extension_version, str) or not extension_version:
    errors.append('tools/editors/vscode-mesh/package.json must declare a non-empty version string')
    extension_version = ''

version_pattern = re.compile(r'^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$')
for label, value in {
    'meshc': meshc_version,
    'meshpkg': meshpkg_version,
    'vscode extension': extension_version,
}.items():
    if value and not version_pattern.fullmatch(value):
        errors.append(f'{label} version is malformed: {value!r}')

if meshc_version != meshpkg_version:
    errors.append(
        f'compiler/meshc and compiler/meshpkg versions diverged ({meshc_version!r} vs {meshpkg_version!r})'
    )

binary_tag = f'v{meshc_version}'
extension_tag = f'ext-v{extension_version}' if extension_version else ''
if binary_tag == extension_tag:
    errors.append('binary and extension candidate tags must stay independently versioned')

artifact = {
    'meshcVersion': meshc_version,
    'meshpkgVersion': meshpkg_version,
    'extensionVersion': extension_version,
    'binaryTag': binary_tag,
    'extensionTag': extension_tag,
    'sources': {
        'meshc': 'compiler/meshc/Cargo.toml',
        'meshpkg': 'compiler/meshpkg/Cargo.toml',
        'extension': 'tools/editors/vscode-mesh/package.json',
    },
}
output_path.write_text(json.dumps(artifact, indent=2) + '\n')

print(f'meshc version: {meshc_version}')
print(f'meshpkg version: {meshpkg_version}')
print(f'extension version: {extension_version}')
print(f'binary tag: {binary_tag}')
print(f'extension tag: {extension_tag}')
print(f'artifact: {output_path.relative_to(root)}')

if errors:
    print()
    print('errors:')
    for error in errors:
        print(f'- {error}')
    raise SystemExit(1)
PY
  then
    fail_phase "$phase_name" "candidate tag derivation drifted" "$log_path"
  fi
  LAST_LOG_PATH="$log_path"
  finish_phase "$phase_name"
}

run_local_docs_truth() {
  local phase_name="docs-truth-local"

  start_phase "$phase_name" "verify README/docs/installers/extension metadata exact local release contract"
  run_command \
    "$phase_name" \
    "docs-truth-local" \
    120 \
    "$ROOT_DIR" \
    "python3 scripts/lib/m034_public_surface_contract.py local-docs --root $ROOT_DIR" \
    "" \
    python3 "$PUBLIC_SURFACE_HELPER" local-docs --root "$ROOT_DIR"
  finish_phase "$phase_name"
}

run_built_docs_truth() {
  local phase_name="docs-truth-built"

  start_phase "$phase_name" "verify built VitePress output preserves the local public contract"
  run_command \
    "$phase_name" \
    "docs-truth-built" \
    180 \
    "$ROOT_DIR" \
    "python3 scripts/lib/m034_public_surface_contract.py built-docs --root $ROOT_DIR --dist-root $ROOT_DIR/website/docs/.vitepress/dist" \
    "" \
    python3 "$PUBLIC_SURFACE_HELPER" built-docs --root "$ROOT_DIR" --dist-root "$ROOT_DIR/website/docs/.vitepress/dist"
  finish_phase "$phase_name"
}

run_remote_evidence() {
  local phase_name="remote-evidence"
  local log_path="$VERIFY_ROOT/remote-evidence.log"

  start_phase "$phase_name" "record hosted GitHub Actions evidence for the current release candidate"
  if ! python3 - "$ROOT_DIR" "$CANDIDATE_TAGS_PATH" "$REMOTE_RUNS_PATH" "$VERIFY_ROOT" "$GH_REPO" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import json
import shlex
import subprocess
import sys
from datetime import datetime, timezone

root = Path(sys.argv[1])
candidate_path = Path(sys.argv[2])
output_path = Path(sys.argv[3])
verify_root = Path(sys.argv[4])
repo = sys.argv[5]
candidate_tags = json.loads(candidate_path.read_text())
binary_tag = candidate_tags['binaryTag']
extension_tag = candidate_tags['extensionTag']


def shell_join(command):
    return ' '.join(shlex.quote(part) for part in command)


def slug_for_workflow(workflow_file: str) -> str:
    return workflow_file.replace('.yml', '').replace('.', '-')


def relative(path: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def write_command_log(command, stdout_path: Path, stderr_path: Path, log_path: Path):
    with log_path.open('w', encoding='utf-8') as handle:
        handle.write(f'command: {shell_join(command)}\n')
        handle.write(f'cwd: {root.as_posix()}\n')
        if stdout_path.read_text():
            handle.write('\n[stdout]\n')
            handle.write(stdout_path.read_text())
        if stderr_path.read_text():
            handle.write('\n[stderr]\n')
            handle.write(stderr_path.read_text())


def run_text_command(command, slug: str, suffix: str, timeout_seconds: int = 45):
    stdout_path = verify_root / f'remote-{slug}-{suffix}.stdout'
    stderr_path = verify_root / f'remote-{slug}-{suffix}.stderr'
    log_path = verify_root / f'remote-{slug}-{suffix}.log'

    stdout_text = ''
    stderr_text = ''
    exit_code = 0
    timeout_hit = False
    try:
        completed = subprocess.run(
            command,
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout_seconds,
            check=False,
        )
        stdout_text = completed.stdout
        stderr_text = completed.stderr
        exit_code = completed.returncode
    except subprocess.TimeoutExpired as exc:
        stdout_text = exc.stdout or ''
        stderr_text = (exc.stderr or '') + f'\n[verify-timeout] command exceeded {timeout_seconds} seconds\n'
        exit_code = 124
        timeout_hit = True

    stdout_path.write_text(stdout_text)
    stderr_path.write_text(stderr_text)
    write_command_log(command, stdout_path, stderr_path, log_path)

    return {
        'command': shell_join(command),
        'exitCode': exit_code,
        'timedOut': timeout_hit,
        'stdoutPath': relative(stdout_path),
        'stderrPath': relative(stderr_path),
        'logPath': relative(log_path),
        'stdout': stdout_text,
        'stderr': stderr_text,
    }


def run_json_command(command, slug: str, suffix: str, timeout_seconds: int = 45):
    result = run_text_command(command, slug, suffix, timeout_seconds=timeout_seconds)
    parsed = None
    parse_error = None
    if result['stdout'].strip():
        try:
            parsed = json.loads(result['stdout'])
        except json.JSONDecodeError as exc:
            parse_error = str(exc)
    result['parsed'] = parsed
    result['parseError'] = parse_error
    return result


def record_query_result(result):
    payload = {
        'command': result['command'],
        'exitCode': result['exitCode'],
        'timedOut': result['timedOut'],
        'stdoutPath': result['stdoutPath'],
        'stderrPath': result['stderrPath'],
        'logPath': result['logPath'],
    }
    if result.get('parseError'):
        payload['parseError'] = result['parseError']
    return payload


def fail_entry(entry, results, errors, reason, *, freshness_reason=None):
    entry['status'] = 'failed'
    entry['failure'] = reason
    if freshness_reason is not None:
        entry['freshnessStatus'] = 'failed'
        entry['freshnessFailure'] = freshness_reason
        if entry.get('headShaMatchesExpected') is None:
            entry['headShaMatchesExpected'] = False
    elif entry.get('freshnessStatus') != 'ok':
        entry['freshnessStatus'] = 'failed'
        entry['freshnessFailure'] = reason
        if entry.get('headShaMatchesExpected') is None:
            entry['headShaMatchesExpected'] = False
    results.append(entry)
    errors.append(reason)


def job_name_matches(actual_name, required_name):
    if not isinstance(actual_name, str):
        return False
    if actual_name == required_name:
        return True
    reusable_suffix = f' / {required_name}'
    return actual_name.endswith(reusable_suffix)


def find_matching_job(jobs, required_name):
    for job in jobs:
        if isinstance(job, dict) and job_name_matches(job.get('name'), required_name):
            return job
    return None


def resolve_expected_ref(entry, spec, slug):
    ref_candidates = [spec['expectedRef']]
    peeled_ref = spec.get('expectedPeeledRef')
    if peeled_ref:
        ref_candidates.append(peeled_ref)

    command = ['git', 'ls-remote', '--quiet', 'origin', *ref_candidates]
    result = run_text_command(command, slug, 'expected-ref', timeout_seconds=20)
    entry['expectedRefQuery'] = record_query_result(result)
    entry['expectedRefCandidates'] = ref_candidates

    if result['exitCode'] != 0:
        if result['timedOut']:
            reason = f"{spec['workflowFile']} expected ref resolution timed out for {spec['expectedRef']!r}"
        else:
            reason = f"{spec['workflowFile']} expected ref resolution failed for {spec['expectedRef']!r}"
        return None, None, reason

    parsed_lines = []
    malformed_lines = []
    for raw_line in result['stdout'].splitlines():
        line = raw_line.strip()
        if not line:
            continue
        parts = line.split()
        if len(parts) != 2:
            malformed_lines.append(raw_line)
            continue
        sha, ref_name = parts
        parsed_lines.append((sha, ref_name))

    if malformed_lines:
        reason = f"{spec['workflowFile']} expected ref resolution returned malformed git ls-remote output"
        return None, None, reason

    ref_map = {}
    duplicate_refs = []
    for sha, ref_name in parsed_lines:
        if ref_name in ref_map and ref_map[ref_name] != sha:
            duplicate_refs.append(ref_name)
            continue
        ref_map[ref_name] = sha

    if duplicate_refs:
        reason = f"{spec['workflowFile']} expected ref resolution returned ambiguous refs: {sorted(set(duplicate_refs))}"
        return None, None, reason

    resolved_ref = None
    expected_head_sha = None
    if peeled_ref and ref_map.get(peeled_ref):
        resolved_ref = peeled_ref
        expected_head_sha = ref_map[peeled_ref]
    elif ref_map.get(spec['expectedRef']):
        resolved_ref = spec['expectedRef']
        expected_head_sha = ref_map[spec['expectedRef']]

    if not expected_head_sha:
        reason = f"{spec['workflowFile']} could not resolve expected remote ref {spec['expectedRef']!r}"
        return None, None, reason

    return resolved_ref, expected_head_sha, None


workflow_specs = [
    {
        'workflowFile': 'deploy.yml',
        'requiredEvent': 'push',
        'requiredHeadBranch': 'main',
        'expectedRef': 'refs/heads/main',
        'requiredJobs': ['build', 'deploy'],
        'requiredSteps': {
            'build': ['Verify public docs contract'],
        },
    },
    {
        'workflowFile': 'deploy-services.yml',
        'requiredEvent': 'push',
        'requiredHeadBranch': binary_tag,
        'expectedRef': f'refs/tags/{binary_tag}',
        'expectedPeeledRef': f'refs/tags/{binary_tag}^{{}}',
        'requiredJobs': [
            'Deploy mesh-registry',
            'Deploy mesh-packages website',
            'Deploy hyperpush landing',
            'Post-deploy health checks',
        ],
        'requiredSteps': {
            'Post-deploy health checks': [
                'Verify public surface contract',
                'Verify hyperpush landing',
            ],
        },
    },
    {
        'workflowFile': 'authoritative-verification.yml',
        'requiredEvent': 'push',
        'requiredHeadBranch': 'main',
        'expectedRef': 'refs/heads/main',
        'requiredJobs': ['Authoritative live proof'],
    },
    {
        'workflowFile': 'release.yml',
        'requiredEvent': 'push',
        'requiredHeadBranch': binary_tag,
        'expectedRef': f'refs/tags/{binary_tag}',
        'expectedPeeledRef': f'refs/tags/{binary_tag}^{{}}',
        'requiredJobs': ['Authoritative live proof', 'Create Release'],
        'requiredJobPrefixes': ['Build (', 'Build meshpkg (', 'Verify release assets ('],
    },
    {
        'workflowFile': 'extension-release-proof.yml',
        'queryWorkflowFile': 'publish-extension.yml',
        'requiredEvent': 'push',
        'requiredHeadBranch': extension_tag,
        'expectedRef': f'refs/tags/{extension_tag}',
        'expectedPeeledRef': f'refs/tags/{extension_tag}^{{}}',
        'requiredJobs': ['Verify extension release proof'],
        'requiredJobSuccesses': ['Verify extension release proof'],
        'successFromJobsOnly': True,
    },
    {
        'workflowFile': 'publish-extension.yml',
        'requiredEvent': 'push',
        'requiredHeadBranch': extension_tag,
        'expectedRef': f'refs/tags/{extension_tag}',
        'expectedPeeledRef': f'refs/tags/{extension_tag}^{{}}',
        'requiredJobs': ['Verify extension release proof', 'Publish verified extension'],
        'requiredJobSuccesses': ['Verify extension release proof', 'Publish verified extension'],
    },
]

results = []
errors = []

for spec in workflow_specs:
    slug = slug_for_workflow(spec['workflowFile'])
    query_workflow_file = spec.get('queryWorkflowFile', spec['workflowFile'])
    entry = {
        'workflowFile': spec['workflowFile'],
        'queryWorkflowFile': query_workflow_file,
        'repository': repo,
        'requiredEvent': spec['requiredEvent'],
        'requiredHeadBranch': spec['requiredHeadBranch'],
        'expectedRef': spec['expectedRef'],
        'expectedResolvedRef': None,
        'expectedHeadSha': None,
        'observedHeadSha': None,
        'headShaMatchesExpected': None,
        'freshnessStatus': 'pending',
        'freshnessFailure': None,
        'requiredJobs': spec.get('requiredJobs', []),
        'requiredJobPrefixes': spec.get('requiredJobPrefixes', []),
        'requiredSteps': spec.get('requiredSteps', {}),
        'status': 'pending',
    }

    resolved_ref, expected_head_sha, expected_ref_error = resolve_expected_ref(entry, spec, slug)
    if expected_ref_error:
        fail_entry(entry, results, errors, expected_ref_error)
        continue
    entry['expectedResolvedRef'] = resolved_ref
    entry['expectedHeadSha'] = expected_head_sha

    list_command = [
        'gh', 'run', 'list',
        '-R', repo,
        '--workflow', query_workflow_file,
        '--event', spec['requiredEvent'],
        '--branch', spec['requiredHeadBranch'],
        '--limit', '1',
        '--json', 'databaseId,workflowName,event,status,conclusion,headBranch,headSha,displayTitle,createdAt,url',
    ]
    list_result = run_json_command(list_command, slug, 'list')
    entry['listQuery'] = record_query_result(list_result)

    if list_result['exitCode'] != 0:
        reason = f"{spec['workflowFile']} gh run list failed"
        stderr_text = list_result['stderr']
        if list_result['timedOut']:
            reason = f"{spec['workflowFile']} gh run list timed out"
        elif 'HTTP 404' in stderr_text or 'workflow' in stderr_text.lower() and 'not found' in stderr_text.lower():
            reason = f"{spec['workflowFile']} is missing on the remote default branch"
        fail_entry(entry, results, errors, reason)
        continue

    if list_result['parseError']:
        reason = f"{spec['workflowFile']} gh run list output was not valid JSON: {list_result['parseError']}"
        fail_entry(entry, results, errors, reason)
        continue

    runs = list_result['parsed']
    if not isinstance(runs, list) or not runs:
        reason = (
            f"{spec['workflowFile']} has no hosted run for event {spec['requiredEvent']!r} "
            f"on {spec['requiredHeadBranch']!r}"
        )
        fallback_command = [
            'gh', 'run', 'list',
            '-R', repo,
            '--workflow', query_workflow_file,
            '--limit', '1',
            '--json', 'databaseId,workflowName,event,status,conclusion,headBranch,headSha,displayTitle,createdAt,url',
        ]
        fallback_result = run_json_command(fallback_command, slug, 'latest-available')
        entry['latestAvailableQuery'] = record_query_result(fallback_result)
        if fallback_result['exitCode'] == 0 and not fallback_result['parseError'] and isinstance(fallback_result['parsed'], list) and fallback_result['parsed']:
            entry['latestAvailableRun'] = fallback_result['parsed'][0]
        fail_entry(entry, results, errors, reason)
        continue

    run_summary = runs[0]
    entry['runSummary'] = run_summary
    entry['latestAvailableRun'] = run_summary
    entry['observedHeadSha'] = run_summary.get('headSha')

    view_command = [
        'gh', 'run', 'view', str(run_summary['databaseId']),
        '-R', repo,
        '--json', 'databaseId,workflowName,event,status,conclusion,headBranch,headSha,displayTitle,url,jobs',
    ]
    view_result = run_json_command(view_command, slug, 'view', timeout_seconds=60)
    entry['viewQuery'] = record_query_result(view_result)

    if view_result['exitCode'] != 0:
        reason = f"{spec['workflowFile']} gh run view failed for run {run_summary['databaseId']}"
        if view_result['timedOut']:
            reason = f"{spec['workflowFile']} gh run view timed out for run {run_summary['databaseId']}"
        fail_entry(entry, results, errors, reason)
        continue

    if view_result['parseError']:
        reason = f"{spec['workflowFile']} gh run view output was not valid JSON: {view_result['parseError']}"
        fail_entry(entry, results, errors, reason)
        continue

    run_view = view_result['parsed']
    jobs = run_view.get('jobs') if isinstance(run_view, dict) else None
    if not isinstance(run_view, dict) or not isinstance(jobs, list):
        reason = f"{spec['workflowFile']} gh run view did not include the jobs payload"
        fail_entry(entry, results, errors, reason)
        continue

    entry['runView'] = run_view
    observed_head_sha = run_view.get('headSha') or run_summary.get('headSha')
    entry['observedHeadSha'] = observed_head_sha

    if not observed_head_sha:
        reason = f"{spec['workflowFile']} hosted run omitted headSha for {spec['requiredHeadBranch']!r}"
        fail_entry(entry, results, errors, reason)
        continue

    if observed_head_sha != expected_head_sha:
        reason = (
            f"{spec['workflowFile']} hosted run headSha {observed_head_sha!r} "
            f"did not match expected {resolved_ref!r} sha {expected_head_sha!r}"
        )
        fail_entry(entry, results, errors, reason, freshness_reason=reason)
        continue

    entry['headShaMatchesExpected'] = True
    entry['freshnessStatus'] = 'ok'

    if run_summary.get('event') != spec['requiredEvent']:
        reason = (
            f"{spec['workflowFile']} latest hosted run event {run_summary.get('event')!r} "
            f"did not match required {spec['requiredEvent']!r}"
        )
        fail_entry(entry, results, errors, reason)
        continue

    if run_summary.get('headBranch') != spec['requiredHeadBranch']:
        reason = (
            f"{spec['workflowFile']} latest hosted run branch {run_summary.get('headBranch')!r} "
            f"did not match required {spec['requiredHeadBranch']!r}"
        )
        fail_entry(entry, results, errors, reason)
        continue

    if not spec.get('successFromJobsOnly', False):
        if run_summary.get('status') != 'completed' or run_summary.get('conclusion') != 'success':
            reason = (
                f"{spec['workflowFile']} latest hosted run concluded {run_summary.get('status')!r}/"
                f"{run_summary.get('conclusion')!r} instead of completed/success"
            )
            fail_entry(entry, results, errors, reason)
            continue

    job_names = [job.get('name') for job in jobs if isinstance(job, dict)]
    missing_jobs = [job_name for job_name in spec.get('requiredJobs', []) if find_matching_job(jobs, job_name) is None]
    if missing_jobs:
        reason = f"{spec['workflowFile']} hosted run is missing required jobs: {missing_jobs}"
        fail_entry(entry, results, errors, reason)
        continue

    missing_prefixes = [
        prefix for prefix in spec.get('requiredJobPrefixes', [])
        if not any(isinstance(job_name, str) and job_name.startswith(prefix) for job_name in job_names)
    ]
    if missing_prefixes:
        reason = f"{spec['workflowFile']} hosted run is missing required job prefixes: {missing_prefixes}"
        fail_entry(entry, results, errors, reason)
        continue

    required_job_successes = spec.get('requiredJobSuccesses', spec.get('requiredJobs', []))
    matched_jobs = {}
    failing_jobs = []
    for job_name in required_job_successes:
        matched_job = find_matching_job(jobs, job_name)
        if matched_job is None:
            continue
        matched_jobs[job_name] = matched_job.get('name')
        if matched_job.get('status') != 'completed' or matched_job.get('conclusion') != 'success':
            failing_jobs.append(f"{job_name}: {matched_job.get('status')!r}/{matched_job.get('conclusion')!r}")
    if matched_jobs:
        entry['matchedJobs'] = matched_jobs
    if failing_jobs:
        reason = f"{spec['workflowFile']} hosted run has non-green required jobs: {failing_jobs}"
        fail_entry(entry, results, errors, reason)
        continue

    jobs_by_name = {
        job.get('name'): job
        for job in jobs
        if isinstance(job, dict) and isinstance(job.get('name'), str)
    }
    missing_steps = []
    for job_name, required_steps in spec.get('requiredSteps', {}).items():
        job = find_matching_job(jobs, job_name) or jobs_by_name.get(job_name, {})
        actual_steps = [
            step.get('name')
            for step in job.get('steps', [])
            if isinstance(step, dict)
        ]
        for step_name in required_steps:
            if step_name not in actual_steps:
                missing_steps.append(f'{job_name}: {step_name}')
    if missing_steps:
        reason = f"{spec['workflowFile']} hosted run is missing required steps: {missing_steps}"
        fail_entry(entry, results, errors, reason)
        continue

    entry['status'] = 'ok'
    entry['freshnessStatus'] = 'ok'
    entry['freshnessFailure'] = None
    results.append(entry)

artifact = {
    'generatedAt': datetime.now(timezone.utc).isoformat(),
    'repository': repo,
    'candidateTags': {
        'binaryTag': binary_tag,
        'extensionTag': extension_tag,
    },
    'workflows': results,
}
output_path.write_text(json.dumps(artifact, indent=2) + '\n')

print(f'repository: {repo}')
print(f'binary tag: {binary_tag}')
print(f'extension tag: {extension_tag}')
print(f'artifact: {relative(output_path)}')
for entry in results:
    print(f"{entry['workflowFile']}: {entry['status']}")
    print(
        f"  freshness: {entry['freshnessStatus']} "
        f"expected={entry.get('expectedHeadSha')!r} observed={entry.get('observedHeadSha')!r}"
    )
    if entry.get('failure'):
        print(f"  reason: {entry['failure']}")

if errors:
    print()
    print('errors:')
    for error in errors:
        print(f'- {error}')
    raise SystemExit(1)
PY
  then
    fail_phase "$phase_name" "hosted workflow evidence drifted" "$log_path"
  fi
  LAST_LOG_PATH="$log_path"
  finish_phase "$phase_name"
}

run_public_http_truth() {
  local phase_name="public-http"

  start_phase "$phase_name" "verify exact public HTTP status, bodies, and bounded freshness wait"
  run_command \
    "$phase_name" \
    "public-http" \
    480 \
    "$ROOT_DIR" \
    "python3 scripts/lib/m034_public_surface_contract.py public-http --root $ROOT_DIR --artifact-dir $VERIFY_ROOT" \
    ".tmp/m034-s05/verify" \
    python3 "$PUBLIC_SURFACE_HELPER" public-http --root "$ROOT_DIR" --artifact-dir "$VERIFY_ROOT"
  assert_file_exists "$phase_name" "$PUBLIC_HTTP_LOG" "public HTTP contract log"
  LAST_LOG_PATH="$PUBLIC_HTTP_LOG"
  finish_phase "$phase_name"
}

parse_args "$@"

prepare_verify_root
run_prereq_sweep
run_candidate_tags

start_phase "s05-workflows" "reuse the S05 deploy workflow verifier unchanged"
run_command "s05-workflows" "s05-workflows" 120 "$ROOT_DIR" "bash scripts/verify-m034-s05-workflows.sh" ".tmp/m034-s05/workflows" bash scripts/verify-m034-s05-workflows.sh
assert_file_exists "s05-workflows" "$ROOT_DIR/.tmp/m034-s05/workflows/phase-report.txt" "S05 workflow phase report"
finish_phase "s05-workflows"

start_phase "docs-build" "build VitePress before any live HTTP or publish work"
run_command "docs-build" "docs-build" 1200 "$ROOT_DIR" "npm --prefix website run build" "" npm --prefix website run build
finish_phase "docs-build"

run_local_docs_truth
run_built_docs_truth

start_phase "s02-workflows" "reuse the S02 workflow verifier unchanged"
run_command "s02-workflows" "s02-workflows" 180 "$ROOT_DIR" "bash scripts/verify-m034-s02-workflows.sh" ".tmp/m034-s02/verify" bash scripts/verify-m034-s02-workflows.sh
assert_file_exists "s02-workflows" "$ROOT_DIR/.tmp/m034-s02/verify/full-contract.log" "S02 workflow full-contract log"
finish_phase "s02-workflows"

start_phase "s03-installer" "reuse the S03 installer verifier unchanged"
run_command "s03-installer" "s03-installer" 2400 "$ROOT_DIR" "bash scripts/verify-m034-s03.sh" ".tmp/m034-s03/verify" bash scripts/verify-m034-s03.sh
assert_file_exists "s03-installer" "$ROOT_DIR/.tmp/m034-s03/verify/run/00-context.log" "S03 context log"
assert_file_exists "s03-installer" "$ROOT_DIR/.tmp/m034-s03/verify/run/06-install-good.log" "S03 install log"
finish_phase "s03-installer"

start_phase "s04-extension" "reuse the S04 extension verifier unchanged"
run_command "s04-extension" "s04-extension" 3600 "$ROOT_DIR" "bash scripts/verify-m034-s04-extension.sh" ".tmp/m034-s04/verify" bash scripts/verify-m034-s04-extension.sh
assert_file_exists "s04-extension" "$ROOT_DIR/.tmp/m034-s04/verify/verified-vsix-path.txt" "S04 verified VSIX path"
assert_file_content_exact "s04-extension" "$ROOT_DIR/.tmp/m034-s04/verify/status.txt" "ok" "S04 extension status"
finish_phase "s04-extension"

start_phase "s04-workflows" "reuse the S04 workflow verifier unchanged"
run_command "s04-workflows" "s04-workflows" 180 "$ROOT_DIR" "bash scripts/verify-m034-s04-workflows.sh" ".tmp/m034-s04/workflows" bash scripts/verify-m034-s04-workflows.sh
assert_file_exists "s04-workflows" "$ROOT_DIR/.tmp/m034-s04/workflows/phase-report.txt" "S04 workflow phase report"
finish_phase "s04-workflows"

run_remote_evidence
if should_stop_after_phase "remote-evidence"; then
  complete_stop_after_phase "remote-evidence"
fi
run_public_http_truth

start_phase "s01-live-proof" "reuse the S01 live registry publish/install verifier unchanged"
run_command "s01-live-proof" "s01-live-proof" 3600 "$ROOT_DIR" "bash -c 'set -euo pipefail; test -f .env; set -a; source .env; set +a; bash scripts/verify-m034-s01.sh'" ".tmp/m034-s01/verify" bash -c 'set -euo pipefail; test -f .env; set -a; source .env; set +a; bash scripts/verify-m034-s01.sh'
if ! find "$ROOT_DIR/.tmp/m034-s01/verify" -mindepth 2 -maxdepth 2 -name package-version.txt -print -quit | grep -q .; then
  fail_phase "s01-live-proof" "expected S01 live proof to emit package-version.txt under .tmp/m034-s01/verify/" "$LAST_LOG_PATH" ".tmp/m034-s01/verify"
fi
finish_phase "s01-live-proof"

printf 'ok\n' >"$STATUS_PATH"
printf 'complete\n' >"$CURRENT_PHASE_PATH"
rm -f "$FAILED_PHASE_PATH"

echo "verify-m034-s05: ok"
