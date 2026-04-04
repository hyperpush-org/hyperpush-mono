#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_ROOT=".tmp/m049-s05"
ARTIFACT_DIR="$ARTIFACT_ROOT/verify"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
LATEST_PROOF_BUNDLE_PATH="$ARTIFACT_DIR/latest-proof-bundle.txt"
RETAINED_PROOF_BUNDLE_DIR="$ARTIFACT_DIR/retained-proof-bundle"
MESHC_BIN_PATH="$ROOT_DIR/target/debug/meshc"
RESOLVED_POSTGRES_CONNECTION=""
RESOLVED_POSTGRES_CONNECTION_SOURCE=""

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

require_executable() {
  local phase="$1"
  local path="$2"
  local description="$3"
  local artifact_hint="${4:-}"
  if [[ -x "$path" ]]; then
    return 0
  fi

  local log_path="$ARTIFACT_DIR/${phase}.preflight.log"
  {
    echo "preflight: missing required executable"
    echo "description: ${description}"
    echo "path: $(repo_rel "$path")"
  } >"$log_path"
  record_phase "$phase" failed
  fail_phase "$phase" "missing required executable: $(repo_rel "$path")" "$log_path" "$artifact_hint"
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

copy_new_artifacts_or_fail() {
  local phase="$1"
  local before_snapshot="$2"
  local source_root="$3"
  local dest_root="$4"
  local manifest_path="$5"
  local expected_message="$6"
  shift 6
  local log_path="$ARTIFACT_DIR/${phase}.artifact-check.log"

  if ! python3 - "$before_snapshot" "$source_root" "$dest_root" "$manifest_path" "$expected_message" "$@" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import shutil
import sys

before_snapshot = Path(sys.argv[1])
source_root = Path(sys.argv[2])
dest_root = Path(sys.argv[3])
manifest_path = Path(sys.argv[4])
expected_message = sys.argv[5]
ignored = set(sys.argv[6:])

if not source_root.is_dir():
    raise SystemExit(f"missing artifact source root: {source_root}")

before = {
    line.strip()
    for line in before_snapshot.read_text(errors='replace').splitlines()
    if line.strip()
}
after_paths = {
    path.name: path
    for path in source_root.iterdir()
    if path.is_dir() and path.name not in ignored
}
new_names = sorted(name for name in after_paths if name not in before)
if not new_names:
    raise SystemExit(expected_message)

if dest_root.exists():
    shutil.rmtree(dest_root)
dest_root.parent.mkdir(parents=True, exist_ok=True)
dest_root.mkdir(parents=True, exist_ok=True)
manifest_lines = []
for name in new_names:
    src = after_paths[name]
    if not any(src.iterdir()):
        raise SystemExit(f"{src}: expected non-empty artifact directory")
    dst = dest_root / name
    shutil.copytree(src, dst, symlinks=True)
    manifest_lines.append(f"{name}\t{src}")
    for child in sorted(src.rglob('*')):
        if child.is_file():
            manifest_lines.append(f"  - {child}")

manifest_path.write_text('\n'.join(manifest_lines) + ('\n' if manifest_lines else ''))
print(f"copied {len(new_names)} artifact directories into {dest_root}")
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "missing or malformed copied artifacts" "$log_path" "$source_root"
  fi
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

resolve_postgres_connection() {
  local phase="$1"
  local log_path="$ARTIFACT_DIR/${phase}.env-resolution.log"
  local source_path="$ARTIFACT_DIR/${phase}.source.txt"
  local resolved

  if ! resolved="$(python3 - "$ROOT_DIR/.env" "$ROOT_DIR/.tmp/m049-s01/local-postgres/connection.env" 2>"$log_path" <<'PY'
import os
import sys
from pathlib import Path

root_env = Path(sys.argv[1])
fallback_env = Path(sys.argv[2])


def fail(message: str) -> None:
    raise SystemExit(message)


def validate(value: str, label: str) -> str:
    candidate = value.strip()
    if not candidate:
        fail(f"{label}: required Postgres connection string is empty")
    if "\n" in candidate or "\r" in candidate:
        fail(f"{label}: required Postgres connection string contains newlines")
    if not (
        candidate.startswith("postgres://")
        or candidate.startswith("postgresql://")
    ):
        fail(
            f"{label}: required Postgres connection string must start with postgres:// or postgresql://"
        )
    return candidate


def parse_env_file(path: Path):
    if not path.exists():
        return None
    try:
        text = path.read_text(errors="strict")
    except Exception as exc:  # pragma: no cover - shell integration path
        fail(f"{path}: unreadable env file ({exc})")
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("export "):
            stripped = stripped[len("export "):].lstrip()
        if "=" not in stripped:
            continue
        key, value = stripped.split("=", 1)
        if key.strip() != "DATABASE_URL":
            continue
        candidate = value.strip()
        if candidate[:1] in {'"', "'"} and candidate[-1:] == candidate[:1]:
            candidate = candidate[1:-1]
        return validate(candidate, str(path))
    return None

process_value = os.environ.get("DATABASE_URL")
if process_value:
    print(f"process-env\t{validate(process_value, 'process environment')}")
    raise SystemExit(0)

root_value = parse_env_file(root_env)
if root_value is not None:
    print(f"repo-root-env\t{root_value}")
    raise SystemExit(0)

fallback_value = parse_env_file(fallback_env)
if fallback_value is not None:
    print(f"m049-s01-fallback-env\t{fallback_value}")
    raise SystemExit(0)

fail(
    "missing Postgres connection source; checked process environment, .env, and .tmp/m049-s01/local-postgres/connection.env"
)
PY
)"; then
    record_phase "$phase" failed
    fail_phase "$phase" "missing or malformed Postgres connection source" "$log_path"
  fi

  local source_label=""
  local connection=""
  IFS=$'\t' read -r source_label connection <<<"$resolved"
  if [[ -z "$source_label" || -z "$connection" ]]; then
    record_phase "$phase" failed
    fail_phase "$phase" "Postgres connection resolution returned malformed output" "$log_path"
  fi

  RESOLVED_POSTGRES_CONNECTION_SOURCE="$source_label"
  RESOLVED_POSTGRES_CONNECTION="$connection"
  printf 'source=%s\n' "$RESOLVED_POSTGRES_CONNECTION_SOURCE" >"$source_path"
}

wait_for_postgres_connection() {
  local connection="$1"
  local timeout_secs="$2"

  python3 - "$connection" "$timeout_secs" <<'PY'
import socket
import sys
import time
from urllib.parse import urlsplit

connection = sys.argv[1]
timeout_secs = float(sys.argv[2])
parsed = urlsplit(connection)
host = parsed.hostname
port = parsed.port
if not host or port is None:
    raise SystemExit("connection string is missing host or port")

deadline = time.time() + timeout_secs
last_error = "unknown error"
while time.time() < deadline:
    try:
        with socket.create_connection((host, port), timeout=1.5):
            raise SystemExit(0)
    except OSError as exc:
        last_error = str(exc)
        time.sleep(0.5)

raise SystemExit(
    f"Postgres connection remained unreachable at {host}:{port} after {timeout_secs:.0f}s: {last_error}"
)
PY
}

refresh_fallback_postgres_connection_from_container() {
  local meta_path="$ROOT_DIR/.tmp/m049-s01/local-postgres/container-meta.txt"
  local connection_path="$ROOT_DIR/.tmp/m049-s01/local-postgres/connection.env"

  python3 - "$meta_path" "$connection_path" <<'PY'
from pathlib import Path
import subprocess
import sys
from urllib.parse import urlsplit, urlunsplit

meta_path = Path(sys.argv[1])
connection_path = Path(sys.argv[2])

if not meta_path.is_file():
    raise SystemExit(f"missing fallback Postgres metadata: {meta_path}")
if not connection_path.is_file():
    raise SystemExit(f"missing fallback Postgres connection file: {connection_path}")

meta = {}
for line in meta_path.read_text(errors="strict").splitlines():
    stripped = line.strip()
    if not stripped or "=" not in stripped:
        continue
    key, value = stripped.split("=", 1)
    meta[key.strip()] = value.strip()

container = meta.get("container")
if not container:
    raise SystemExit(f"{meta_path}: missing container name")

connection_value = None
for line in connection_path.read_text(errors="strict").splitlines():
    stripped = line.strip()
    if not stripped or stripped.startswith("#"):
        continue
    if stripped.startswith("export "):
        stripped = stripped[len("export ") :].lstrip()
    if not stripped.startswith("DATABASE_URL="):
        continue
    connection_value = stripped.split("=", 1)[1].strip()
    if connection_value[:1] in {'"', "'"} and connection_value[-1:] == connection_value[:1]:
        connection_value = connection_value[1:-1]
    break

if not connection_value:
    raise SystemExit(f"{connection_path}: missing DATABASE_URL")

parts = urlsplit(connection_value)
if not parts.scheme or not parts.hostname or parts.port is None:
    raise SystemExit(f"{connection_path}: malformed DATABASE_URL")

try:
    running = subprocess.check_output(
        ["docker", "inspect", "-f", "{{.State.Running}}", container],
        text=True,
        stderr=subprocess.STDOUT,
    ).strip()
except subprocess.CalledProcessError as exc:
    raise SystemExit(
        f"failed to inspect fallback Postgres container {container}: {exc.output.strip()}"
    )

if running.lower() != "true":
    try:
        subprocess.run(
            ["docker", "start", container],
            check=True,
            text=True,
            capture_output=True,
        )
    except subprocess.CalledProcessError as exc:
        combined = ((exc.stdout or "") + (exc.stderr or "")).strip()
        raise SystemExit(
            f"failed to start fallback Postgres container {container}: {combined}"
        )

try:
    published = subprocess.check_output(
        ["docker", "port", container, "5432/tcp"],
        text=True,
        stderr=subprocess.STDOUT,
    )
except subprocess.CalledProcessError as exc:
    raise SystemExit(
        f"failed to resolve published port for fallback Postgres container {container}: {exc.output.strip()}"
    )

mappings = [line.strip() for line in published.splitlines() if line.strip()]
if not mappings:
    raise SystemExit(
        f"fallback Postgres container {container} has no published 5432/tcp port"
    )

try:
    _, published_port = mappings[0].rsplit(":", 1)
except ValueError as exc:
    raise SystemExit(
        f"could not parse published port mapping {mappings[0]!r}: {exc}"
    )
if not published_port.isdigit():
    raise SystemExit(f"published port is not numeric: {mappings[0]!r}")

userinfo = ""
if parts.username is not None:
    userinfo = parts.username
    if parts.password is not None:
        userinfo += f":{parts.password}"
    userinfo += "@"

host = parts.hostname
if ":" in host and not host.startswith("["):
    host = f"[{host}]"

new_url = urlunsplit(
    (
        parts.scheme,
        f"{userinfo}{host}:{published_port}",
        parts.path,
        parts.query,
        parts.fragment,
    )
)

connection_path.write_text(f"export DATABASE_URL={new_url}\n")
meta["port"] = published_port
meta_path.write_text(
    "".join(f"{key}={value}\n" for key, value in meta.items())
)
print(f"fallback Postgres container {container} refreshed")
PY
}

ensure_resolved_postgres_connection_reachable() {
  local phase="$1"
  local reachability_log="$ARTIFACT_DIR/${phase}.reachability.log"
  local refresh_log="$ARTIFACT_DIR/${phase}.fallback-refresh.log"

  if wait_for_postgres_connection "$RESOLVED_POSTGRES_CONNECTION" 2 >"$reachability_log" 2>&1; then
    return 0
  fi

  if [[ "$RESOLVED_POSTGRES_CONNECTION_SOURCE" != "m049-s01-fallback-env" ]]; then
    record_phase "$phase" failed
    fail_phase "$phase" "resolved Postgres connection is unreachable" "$reachability_log"
  fi

  if ! command -v docker >/dev/null 2>&1; then
    record_phase "$phase" failed
    fail_phase "$phase" "fallback Postgres connection is unreachable and docker is unavailable for container refresh" "$reachability_log"
  fi

  if ! refresh_fallback_postgres_connection_from_container >"$refresh_log" 2>&1; then
    record_phase "$phase" failed
    fail_phase "$phase" "failed to refresh fallback Postgres container metadata" "$refresh_log" ".tmp/m049-s01/local-postgres"
  fi

  resolve_postgres_connection "$phase"
  if ! wait_for_postgres_connection "$RESOLVED_POSTGRES_CONNECTION" 20 >"$reachability_log" 2>&1; then
    record_phase "$phase" failed
    fail_phase "$phase" "fallback Postgres connection remained unreachable after container refresh" "$reachability_log" ".tmp/m049-s01/local-postgres"
  fi
}

run_expect_success_with_postgres_connection() {
  local phase="$1"
  local label="$2"
  local require_tests="$3"
  local timeout_secs="$4"
  local artifact_hint="$5"
  shift 5
  local old_set=0
  local old_value="${DATABASE_URL-}"
  if [[ "${DATABASE_URL+x}" == x ]]; then
    old_set=1
  fi

  export DATABASE_URL="$RESOLVED_POSTGRES_CONNECTION"
  run_expect_success "$phase" "$label" "$require_tests" "$timeout_secs" "$artifact_hint" "$@"

  if [[ $old_set -eq 1 ]]; then
    export DATABASE_URL="$old_value"
  else
    unset DATABASE_URL
  fi
}

assert_retained_bundle_shape() {
  local phase="$1"
  local bundle_root="$2"
  local pointer_path="$3"
  local contract_log="$4"
  local log_path="$ARTIFACT_DIR/${phase}.bundle-check.log"

  if ! python3 - "$bundle_root" "$pointer_path" "$contract_log" >"$log_path" 2>&1 <<'PY'
from pathlib import Path
import re
import sys

bundle_root = Path(sys.argv[1])
pointer_path = Path(sys.argv[2])
contract_log = Path(sys.argv[3])
expected_pointer = str(bundle_root)
actual_pointer = pointer_path.read_text(errors='replace').strip()
if actual_pointer != expected_pointer:
    raise SystemExit(
        f"latest-proof-bundle pointer drifted: expected {expected_pointer!r}, got {actual_pointer!r}"
    )


def require_dir(path: Path) -> Path:
    if not path.is_dir():
        raise SystemExit(f"missing required directory: {path}")
    return path


def require_file(path: Path) -> Path:
    if not path.is_file():
        raise SystemExit(f"missing required file: {path}")
    return path


def assert_text(path: Path, expected: str, label: str) -> None:
    actual = path.read_text(errors='replace').strip()
    if actual != expected:
        raise SystemExit(f"{label}: expected {expected!r}, got {actual!r}")


def assert_non_empty(path: Path, label: str) -> None:
    if not path.read_text(errors='replace').strip():
        raise SystemExit(f"{label}: expected non-empty file at {path}")


def assert_phase_marker(path: Path, marker: str) -> None:
    text = path.read_text(errors='replace')
    if marker not in text:
        raise SystemExit(f"{path}: missing phase marker {marker!r}")


def assert_exact_prefixes(root: Path, expectations: dict[str, list[str]]) -> None:
    children = sorted(path for path in root.iterdir() if path.is_dir())
    for prefix, required in expectations.items():
        matches = [path for path in children if path.name.startswith(prefix)]
        if len(matches) != 1:
            raise SystemExit(
                f"{root}: expected exactly one retained artifact for {prefix!r}, found {[path.name for path in matches]}"
            )
        for rel in required:
            if not (matches[0] / rel).exists():
                raise SystemExit(f"{matches[0]}: missing {rel}")

m039 = require_dir(bundle_root / 'retained-m039-s01-verify')
for rel in [
    'phase-report.txt',
    '00-build-tooling.log',
    '01-build-cluster-proof.log',
    '02-mesh-rt-discovery.log',
    '03-e2e-converges.log',
    '04-e2e-node-loss.log',
]:
    require_file(m039 / rel)
for marker in [
    'build-tooling\tpassed',
    'build-cluster-proof\tpassed',
    'mesh-rt-discovery\tpassed',
    'convergence\tpassed',
    'node-loss\tpassed',
]:
    assert_phase_marker(m039 / 'phase-report.txt', marker)

m045 = require_dir(bundle_root / 'retained-m045-s02-verify')
for rel in [
    'status.txt',
    'current-phase.txt',
    'phase-report.txt',
    'full-contract.log',
    'latest-proof-bundle.txt',
    'retained-m045-s02-artifacts',
]:
    require_file(m045 / rel) if rel.endswith('.txt') or rel.endswith('.log') else require_dir(m045 / rel)
assert_text(m045 / 'status.txt', 'ok', 'retained m045 status')
assert_text(m045 / 'current-phase.txt', 'complete', 'retained m045 current phase')
assert_non_empty(m045 / 'latest-proof-bundle.txt', 'retained m045 bundle pointer')
assert_phase_marker(m045 / 'phase-report.txt', 'm045-s02-bundle-shape\tpassed')

m047 = require_dir(bundle_root / 'retained-m047-s05-verify')
for rel in [
    'status.txt',
    'current-phase.txt',
    'phase-report.txt',
    'full-contract.log',
    'latest-proof-bundle.txt',
]:
    require_file(m047 / rel)
for rel in [
    'retained-m047-s04-verify',
    'retained-m047-s05-artifacts',
    'retained-proof-bundle',
]:
    require_dir(m047 / rel)
assert_text(m047 / 'status.txt', 'ok', 'retained m047 status')
assert_text(m047 / 'current-phase.txt', 'complete', 'retained m047 current phase')
assert_non_empty(m047 / 'latest-proof-bundle.txt', 'retained m047 bundle pointer')
assert_phase_marker(m047 / 'phase-report.txt', 'm047-s05-bundle-shape\tpassed')

m048 = require_dir(bundle_root / 'retained-m048-s05-verify')
for rel in [
    'status.txt',
    'current-phase.txt',
    'phase-report.txt',
    'full-contract.log',
    'latest-proof-bundle.txt',
]:
    require_file(m048 / rel)
require_dir(m048 / 'retained-proof-bundle')
assert_text(m048 / 'status.txt', 'ok', 'retained m048 status')
assert_text(m048 / 'current-phase.txt', 'complete', 'retained m048 current phase')
assert_non_empty(m048 / 'latest-proof-bundle.txt', 'retained m048 bundle pointer')
assert_phase_marker(m048 / 'phase-report.txt', 'm048-s05-bundle-shape\tpassed')

m050 = require_dir(bundle_root / 'retained-m050-s02-verify')
for rel in [
    'status.txt',
    'current-phase.txt',
    'phase-report.txt',
    'full-contract.log',
    'latest-proof-bundle.txt',
    'built-html/getting-started.index.html',
    'built-html/clustered-example.index.html',
    'built-html/tooling.index.html',
    'built-html/summary.json',
]:
    require_file(m050 / rel)
assert_text(m050 / 'status.txt', 'ok', 'retained m050 s02 status')
assert_text(m050 / 'current-phase.txt', 'complete', 'retained m050 s02 current phase')
assert_non_empty(m050 / 'latest-proof-bundle.txt', 'retained m050 s02 bundle pointer')
for marker in [
    'first-contact-contract\tpassed',
    'm047-s05-docs-contract\tpassed',
    'm047-s06-docs-contract\tpassed',
    'm048-s05-tooling-contract\tpassed',
    'm036-s03-tooling-contract\tpassed',
    'docs-build\tpassed',
    'retain-built-html\tpassed',
    'built-html\tpassed',
    'm050-s02-bundle-shape\tpassed',
]:
    assert_phase_marker(m050 / 'phase-report.txt', marker)

m050_s03 = require_dir(bundle_root / 'retained-m050-s03-verify')
for rel in [
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
    'built-html/distributed.index.html',
    'built-html/distributed-proof.index.html',
    'built-html/production-backend-proof.index.html',
    'built-html/summary.json',
]:
    require_file(m050_s03 / rel)
assert_text(m050_s03 / 'status.txt', 'ok', 'retained m050 s03 status')
assert_text(m050_s03 / 'current-phase.txt', 'complete', 'retained m050 s03 current phase')
assert_non_empty(m050_s03 / 'latest-proof-bundle.txt', 'retained m050 s03 bundle pointer')
for marker in [
    'secondary-surfaces-contract\tpassed',
    'm047-s04-docs-contract\tpassed',
    'm047-s05-docs-contract\tpassed',
    'm047-s06-docs-contract\tpassed',
    'production-proof-surface\tpassed',
    'docs-build\tpassed',
    'retain-built-html\tpassed',
    'built-html\tpassed',
    'm050-s03-bundle-shape\tpassed',
]:
    assert_phase_marker(m050_s03 / 'phase-report.txt', marker)

manifest_paths = [
    bundle_root / 'retained-m049-s01-artifacts.manifest.txt',
    bundle_root / 'retained-m049-s02-artifacts.manifest.txt',
    bundle_root / 'retained-m049-s03-artifacts.manifest.txt',
]
for manifest_path in manifest_paths:
    lines = [line for line in manifest_path.read_text(errors='replace').splitlines() if line.strip()]
    if not lines:
        raise SystemExit(f"{manifest_path}: expected non-empty copied-artifact manifest")

assert_exact_prefixes(
    require_dir(bundle_root / 'retained-m049-s01-artifacts'),
    {
        'todo-api-postgres-runtime-truth-': [
            'workspace/todo-starter/mesh.toml',
            'init.log',
            'migrate-up.stdout.log',
            'meshc-test.stdout.log',
            'build-output.json',
            'runtime.stdout.log',
            'runtime.stderr.log',
        ],
        'todo-api-postgres-missing-database-url-': [
            'workspace/todo-starter/mesh.toml',
            'init.log',
            'build-output.json',
            'missing-database-url.stdout.log',
            'missing-database-url.stderr.log',
            'missing-database-url.meta.txt',
        ],
        'todo-api-postgres-unmigrated-database-': [
            'workspace/todo-starter/mesh.toml',
            'init.log',
            'build-output.json',
            'runtime.stdout.log',
            'runtime.stderr.log',
            'todos-unmigrated.http',
            'todos-unmigrated.json',
        ],
    },
)
assert_exact_prefixes(
    require_dir(bundle_root / 'retained-m049-s02-artifacts'),
    {
        'todo-api-sqlite-runtime-truth-': [
            'scenario-meta.json',
            'workspace/todo-starter/mesh.toml',
            'meshc-test.stdout.log',
            'build-meta.json',
            'runtime.stdout.log',
            'runtime.stderr.log',
            'restart-runtime.stdout.log',
            'restart-runtime.stderr.log',
        ],
        'todo-api-sqlite-bad-db-path-': [
            'scenario-meta.json',
            'workspace/todo-starter/mesh.toml',
            'meshc-test.stdout.log',
            'build-meta.json',
            'bad-db-path-runtime.stdout.log',
            'bad-db-path-runtime.stderr.log',
            'bad-db-path-health.connect-error.txt',
        ],
    },
)
assert_exact_prefixes(
    require_dir(bundle_root / 'retained-m049-s03-artifacts'),
    {
        'todo-examples-parity-': [
            'scenario-meta.json',
            'materializer/materializer-summary.json',
            'materializer/retained-session.txt',
            'materializer/generated/todo-sqlite/mesh.toml',
            'materializer/generated/todo-postgres/mesh.toml',
        ],
        'todo-examples-missing-root-': [
            'scenario-meta.json',
            'materializer/materializer-check.stderr.log',
            'materializer/materializer-check.meta.txt',
            'input-examples-root/todo-postgres/mesh.toml',
        ],
        'todo-examples-drift-report-': [
            'scenario-meta.json',
            'materializer/materializer-check.stderr.log',
            'materializer/materializer-check.meta.txt',
            'input-examples-root/todo-postgres/HAND_EDITED.txt',
            'input-examples-root/todo-sqlite/mesh.toml',
        ],
        'todo-sqlite-test-build-': [
            'scenario-meta.json',
            'project/mesh.toml',
            'meshc-test/meshc-test.stdout.log',
            'build/build-meta.json',
        ],
        'todo-postgres-test-build-': [
            'scenario-meta.json',
            'project/mesh.toml',
            'meshc-test/meshc-test.stdout.log',
            'build/build-output.json',
        ],
    },
)

contract_text = contract_log.read_text(errors='replace')
if re.search(r'postgres(?:ql)?://', contract_text):
    raise SystemExit(f"{contract_log}: wrapper log leaked a Postgres connection string")
for manifest_path in manifest_paths:
    text = manifest_path.read_text(errors='replace')
    if re.search(r'postgres(?:ql)?://', text):
        raise SystemExit(f"{manifest_path}: retained manifest leaked a Postgres connection string")
    if 'DATABASE_URL=' in text:
        raise SystemExit(f"{manifest_path}: retained manifest leaked a raw env assignment")

print('retained-bundle-shape: ok')
PY
  then
    record_phase "$phase" failed
    fail_phase "$phase" "missing retained proof artifacts, malformed bundle pointer, or leaked manifest/log secret" "$log_path" "$bundle_root"
  fi
}

record_phase init passed

require_file m050-s01-preflight "$ROOT_DIR/scripts/verify-m050-s01.sh" "fast M050 docs-graph preflight" ".tmp/m050-s01/verify"
run_expect_success m050-s01-preflight m050-s01-preflight no 1800 ".tmp/m050-s01/verify" \
  bash scripts/verify-m050-s01.sh
require_file m050-s02-preflight "$ROOT_DIR/scripts/verify-m050-s02.sh" "first-contact docs preflight" ".tmp/m050-s02/verify"
run_expect_success m050-s02-preflight m050-s02-preflight no 2400 ".tmp/m050-s02/verify" \
  bash scripts/verify-m050-s02.sh
require_file m050-s03-preflight "$ROOT_DIR/scripts/verify-m050-s03.sh" "secondary-surface docs preflight" ".tmp/m050-s03/verify"
run_expect_success m050-s03-preflight m050-s03-preflight no 2400 ".tmp/m050-s03/verify" \
  bash scripts/verify-m050-s03.sh
run_expect_success m049-s04-onboarding-contract m049-s04-onboarding-contract no 120 "" \
  node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs
run_expect_success m049-scaffold-mesh-pkg m049-scaffold-mesh-pkg yes 2400 "" \
  cargo test -p mesh-pkg m049_s0 -- --nocapture
run_expect_success m049-scaffold-tooling m049-scaffold-tooling yes 2400 "" \
  cargo test -p meshc --test tooling_e2e test_init_todo_template_ -- --nocapture
run_expect_success meshc-build-preflight meshc-build-preflight no 2400 "target/debug" \
  cargo build -q -p meshc
require_executable meshc-build-preflight "$MESHC_BIN_PATH" "repo-local meshc binary required by the direct materializer check" "target/debug"

run_expect_success m049-s03-materialize-direct m049-s03-materialize-direct no 600 "examples" \
  node scripts/tests/verify-m049-s03-materialize-examples.mjs --check

S01_BEFORE="$ARTIFACT_DIR/m049-s01-before.snapshot"
S02_BEFORE="$ARTIFACT_DIR/m049-s02-before.snapshot"
S03_BEFORE="$ARTIFACT_DIR/m049-s03-before.snapshot"
capture_snapshot "$ROOT_DIR/.tmp/m049-s01" "$S01_BEFORE" verify local-postgres
capture_snapshot "$ROOT_DIR/.tmp/m049-s02" "$S02_BEFORE" verify
capture_snapshot "$ROOT_DIR/.tmp/m049-s03" "$S03_BEFORE" verify

begin_phase m049-s01-env-preflight
resolve_postgres_connection m049-s01-env-preflight
ensure_resolved_postgres_connection_reachable m049-s01-env-preflight
record_phase m049-s01-env-preflight passed

run_expect_success_with_postgres_connection m049-s01-e2e m049-s01-e2e yes 5400 ".tmp/m049-s01" \
  cargo test -p meshc --test e2e_m049_s01 -- --nocapture
run_expect_success m049-s02-e2e m049-s02-e2e yes 5400 ".tmp/m049-s02" \
  cargo test -p meshc --test e2e_m049_s02 -- --nocapture
run_expect_success m049-s03-e2e m049-s03-e2e yes 5400 ".tmp/m049-s03" \
  cargo test -p meshc --test e2e_m049_s03 -- --nocapture

require_file m039-s01-replay "$ROOT_DIR/scripts/verify-m039-s01.sh" "retained M039 verifier" ".tmp/m039-s01/verify"
run_expect_success m039-s01-replay m039-s01-replay no 2400 ".tmp/m039-s01/verify" \
  bash scripts/verify-m039-s01.sh
require_file m045-s02-replay "$ROOT_DIR/scripts/verify-m045-s02.sh" "retained M045 verifier" ".tmp/m045-s02/verify"
run_expect_success m045-s02-replay m045-s02-replay no 3600 ".tmp/m045-s02/verify" \
  bash scripts/verify-m045-s02.sh
require_file m047-s05-replay "$ROOT_DIR/scripts/verify-m047-s05.sh" "retained M047 verifier" ".tmp/m047-s05/verify"
run_expect_success m047-s05-replay m047-s05-replay no 5400 ".tmp/m047-s05/verify" \
  bash scripts/verify-m047-s05.sh
require_file m048-s05-replay "$ROOT_DIR/scripts/verify-m048-s05.sh" "retained M048 verifier" ".tmp/m048-s05/verify"
run_expect_success m048-s05-replay m048-s05-replay no 7200 ".tmp/m048-s05/verify" \
  bash scripts/verify-m048-s05.sh

rm -rf "$RETAINED_PROOF_BUNDLE_DIR"
mkdir -p "$RETAINED_PROOF_BUNDLE_DIR"

begin_phase retain-m039-s01-verify
copy_fixed_dir_or_fail retain-m039-s01-verify \
  "$ROOT_DIR/.tmp/m039-s01/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m039-s01-verify" \
  "retained M039 S01 verify directory is missing or malformed" \
  phase-report.txt \
  00-build-tooling.log \
  01-build-cluster-proof.log \
  02-mesh-rt-discovery.log \
  03-e2e-converges.log \
  04-e2e-node-loss.log
record_phase retain-m039-s01-verify passed

begin_phase retain-m045-s02-verify
copy_fixed_dir_or_fail retain-m045-s02-verify \
  "$ROOT_DIR/.tmp/m045-s02/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m045-s02-verify" \
  "retained M045 S02 verify directory is missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt \
  retained-m045-s02-artifacts
record_phase retain-m045-s02-verify passed

begin_phase retain-m047-s05-verify
copy_fixed_dir_or_fail retain-m047-s05-verify \
  "$ROOT_DIR/.tmp/m047-s05/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m047-s05-verify" \
  "retained M047 S05 verify directory is missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt \
  retained-m047-s04-verify \
  retained-m047-s05-artifacts \
  retained-proof-bundle
record_phase retain-m047-s05-verify passed

begin_phase retain-m048-s05-verify
copy_fixed_dir_or_fail retain-m048-s05-verify \
  "$ROOT_DIR/.tmp/m048-s05/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m048-s05-verify" \
  "retained M048 S05 verify directory is missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt \
  retained-proof-bundle
record_phase retain-m048-s05-verify passed

begin_phase retain-m050-s02-verify
copy_fixed_dir_or_fail retain-m050-s02-verify \
  "$ROOT_DIR/.tmp/m050-s02/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m050-s02-verify" \
  "retained M050 S02 verify directory is missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt \
  built-html/getting-started.index.html \
  built-html/clustered-example.index.html \
  built-html/tooling.index.html \
  built-html/summary.json
record_phase retain-m050-s02-verify passed

begin_phase retain-m050-s03-verify
copy_fixed_dir_or_fail retain-m050-s03-verify \
  "$ROOT_DIR/.tmp/m050-s03/verify" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m050-s03-verify" \
  "retained M050 S03 verify directory is missing or malformed" \
  status.txt \
  current-phase.txt \
  phase-report.txt \
  full-contract.log \
  latest-proof-bundle.txt \
  secondary-surfaces-contract.log \
  m047-s04-docs-contract.log \
  m047-s05-docs-contract.log \
  m047-s06-docs-contract.log \
  production-proof-surface.log \
  docs-build.log \
  built-html/distributed.index.html \
  built-html/distributed-proof.index.html \
  built-html/production-backend-proof.index.html \
  built-html/summary.json
record_phase retain-m050-s03-verify passed

begin_phase retain-m049-s01-artifacts
copy_new_artifacts_or_fail \
  retain-m049-s01-artifacts \
  "$S01_BEFORE" \
  "$ROOT_DIR/.tmp/m049-s01" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m049-s01-artifacts" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m049-s01-artifacts.manifest.txt" \
  "expected fresh .tmp/m049-s01 artifact directories from the Postgres scaffold replay" \
  verify \
  local-postgres
record_phase retain-m049-s01-artifacts passed

begin_phase retain-m049-s02-artifacts
copy_new_artifacts_or_fail \
  retain-m049-s02-artifacts \
  "$S02_BEFORE" \
  "$ROOT_DIR/.tmp/m049-s02" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m049-s02-artifacts" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m049-s02-artifacts.manifest.txt" \
  "expected fresh .tmp/m049-s02 artifact directories from the SQLite scaffold replay" \
  verify
record_phase retain-m049-s02-artifacts passed

begin_phase retain-m049-s03-artifacts
copy_new_artifacts_or_fail \
  retain-m049-s03-artifacts \
  "$S03_BEFORE" \
  "$ROOT_DIR/.tmp/m049-s03" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m049-s03-artifacts" \
  "$RETAINED_PROOF_BUNDLE_DIR/retained-m049-s03-artifacts.manifest.txt" \
  "expected fresh .tmp/m049-s03 artifact directories from the example parity replay" \
  verify
record_phase retain-m049-s03-artifacts passed

begin_phase m049-s05-bundle-shape
printf '%s\n' "$RETAINED_PROOF_BUNDLE_DIR" >"$LATEST_PROOF_BUNDLE_PATH"
assert_retained_bundle_shape \
  m049-s05-bundle-shape \
  "$RETAINED_PROOF_BUNDLE_DIR" \
  "$LATEST_PROOF_BUNDLE_PATH" \
  "$ARTIFACT_DIR/full-contract.log"
record_phase m049-s05-bundle-shape passed

for expected_phase in \
  init \
  m050-s01-preflight \
  m050-s02-preflight \
  m050-s03-preflight \
  m049-s04-onboarding-contract \
  m049-scaffold-mesh-pkg \
  m049-scaffold-tooling \
  meshc-build-preflight \
  m049-s03-materialize-direct \
  m049-s01-env-preflight \
  m049-s01-e2e \
  m049-s02-e2e \
  m049-s03-e2e \
  m039-s01-replay \
  m045-s02-replay \
  m047-s05-replay \
  m048-s05-replay \
  retain-m039-s01-verify \
  retain-m045-s02-verify \
  retain-m047-s05-verify \
  retain-m048-s05-verify \
  retain-m050-s02-verify \
  retain-m050-s03-verify \
  retain-m049-s01-artifacts \
  retain-m049-s02-artifacts \
  retain-m049-s03-artifacts \
  m049-s05-bundle-shape; do
  if ! rg -q "^${expected_phase}\\tpassed$" "$PHASE_REPORT_PATH"; then
    fail_phase verifier-status "missing ${expected_phase} pass marker" "$ARTIFACT_DIR/full-contract.log" "$PHASE_REPORT_PATH"
  fi
done

echo "verify-m049-s05: ok"
echo "artifacts: $(repo_rel "$ARTIFACT_DIR")"
echo "proof bundle: $(repo_rel "$RETAINED_PROOF_BUNDLE_DIR")"
