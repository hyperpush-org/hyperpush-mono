#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TMP_ROOT="$ROOT_DIR/.tmp/m034-s04"
VERIFY_ROOT="$TMP_ROOT/verify"
EXTENSION_ROOT="$ROOT_DIR/tools/editors/vscode-mesh"
PACKAGE_JSON_PATH="$EXTENSION_ROOT/package.json"
VSIX_HELPER_PATH="$EXTENSION_ROOT/scripts/vsix-path.mjs"
README_PATH="$EXTENSION_ROOT/README.md"
TOOLING_DOC_PATH="$ROOT_DIR/website/docs/docs/tooling/index.md"
PUBLISH_WORKFLOW_PATH="$ROOT_DIR/.github/workflows/publish-extension.yml"
CURRENT_PHASE_PATH="$VERIFY_ROOT/current-phase.txt"
FAILED_PHASE_PATH="$VERIFY_ROOT/failed-phase.txt"
INTENDED_VSIX_PATH_FILE="$VERIFY_ROOT/intended-vsix-path.txt"
VERIFIED_VSIX_PATH_FILE="$VERIFY_ROOT/verified-vsix-path.txt"
VSIX_CONTENTS_PATH="$VERIFY_ROOT/vsix-contents.txt"
EXTENSION_VERSION_PATH="$VERIFY_ROOT/extension-version.txt"
EXPECTED_TAG_PATH="$VERIFY_ROOT/expected-tag.txt"
STATUS_PATH="$VERIFY_ROOT/status.txt"
LAST_STDOUT_PATH=""
LAST_STDERR_PATH=""
LAST_LOG_PATH=""
EXTENSION_VERSION=""
RELATIVE_VSIX_PATH=""
REPO_RELATIVE_VSIX_PATH=""
ABSOLUTE_VSIX_PATH=""
EXPECTED_TAG_VALUE=""

prepare_verify_root() {
  rm -rf "$VERIFY_ROOT"
  mkdir -p "$VERIFY_ROOT"
  : >"$CURRENT_PHASE_PATH"
  rm -f "$FAILED_PHASE_PATH" "$VERIFIED_VSIX_PATH_FILE" "$VSIX_CONTENTS_PATH" "$STATUS_PATH"
}

set_phase() {
  local phase_name="$1"
  printf '%s\n' "$phase_name" >"$CURRENT_PHASE_PATH"
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

  printf '%s\n' "$phase_name" >"$FAILED_PHASE_PATH"
  printf 'failed\n' >"$STATUS_PATH"

  echo "verification drift: ${reason}" >&2
  echo "first failing phase: ${phase_name}" >&2
  echo "artifacts: ${VERIFY_ROOT#$ROOT_DIR/}" >&2
  if [[ -f "$INTENDED_VSIX_PATH_FILE" ]]; then
    echo "intended VSIX: $(<"$INTENDED_VSIX_PATH_FILE")" >&2
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
  shift 5

  local stdout_path="$VERIFY_ROOT/${label}.stdout"
  local stderr_path="$VERIFY_ROOT/${label}.stderr"
  local log_path="$VERIFY_ROOT/${label}.log"

  set_phase "$phase_name"
  echo "==> [${phase_name}] ${display}"

  local status=0
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
  if [[ "$status" -ne 0 ]]; then
    combine_command_log "$display" "$cwd" "$stdout_path" "$stderr_path" "$log_path"
    if [[ "$status" -eq 124 ]]; then
      fail_phase "$phase_name" "${display} timed out" "$log_path"
    fi
    fail_phase "$phase_name" "${display} failed" "$log_path"
  fi

  combine_command_log "$display" "$cwd" "$stdout_path" "$stderr_path" "$log_path"
  LAST_STDOUT_PATH="$stdout_path"
  LAST_STDERR_PATH="$stderr_path"
  LAST_LOG_PATH="$log_path"
}

prepare_verify_root

run_command \
  "derive-version" \
  "extension-version" \
  20 \
  "$ROOT_DIR" \
  "node -p \"require('./tools/editors/vscode-mesh/package.json').version\"" \
  node -p "require('./tools/editors/vscode-mesh/package.json').version"
EXTENSION_VERSION="$(tr -d '\r\n' <"$LAST_STDOUT_PATH")"
if [[ -z "$EXTENSION_VERSION" ]]; then
  fail_phase "derive-version" "extension version was empty" "$LAST_LOG_PATH"
fi
printf '%s\n' "$EXTENSION_VERSION" >"$EXTENSION_VERSION_PATH"

run_command \
  "derive-vsix-path" \
  "vsix-path-relative" \
  20 \
  "$ROOT_DIR" \
  "node tools/editors/vscode-mesh/scripts/vsix-path.mjs" \
  node tools/editors/vscode-mesh/scripts/vsix-path.mjs
RELATIVE_VSIX_PATH="$(tr -d '\r\n' <"$LAST_STDOUT_PATH")"
if [[ -z "$RELATIVE_VSIX_PATH" ]]; then
  fail_phase "derive-vsix-path" "relative VSIX path was empty" "$LAST_LOG_PATH"
fi
REPO_RELATIVE_VSIX_PATH="tools/editors/vscode-mesh/${RELATIVE_VSIX_PATH}"
printf '%s\n' "$REPO_RELATIVE_VSIX_PATH" >"$INTENDED_VSIX_PATH_FILE"

run_command \
  "derive-vsix-path" \
  "vsix-path-absolute" \
  20 \
  "$ROOT_DIR" \
  "node tools/editors/vscode-mesh/scripts/vsix-path.mjs --absolute" \
  node tools/editors/vscode-mesh/scripts/vsix-path.mjs --absolute
ABSOLUTE_VSIX_PATH="$(tr -d '\r\n' <"$LAST_STDOUT_PATH")"
if [[ -z "$ABSOLUTE_VSIX_PATH" ]]; then
  fail_phase "derive-vsix-path" "absolute VSIX path was empty" "$LAST_LOG_PATH"
fi

EXPECTED_TAG_VALUE="${EXPECTED_TAG:-ext-v${EXTENSION_VERSION}}"
printf '%s\n' "$EXPECTED_TAG_VALUE" >"$EXPECTED_TAG_PATH"

set_phase "prereq-sweep"
echo "==> [prereq-sweep] extension tag/docs/workflow contract"
if ! python3 - "$PACKAGE_JSON_PATH" "$README_PATH" "$TOOLING_DOC_PATH" "$PUBLISH_WORKFLOW_PATH" "$EXTENSION_VERSION" "$EXPECTED_TAG_VALUE" "$RELATIVE_VSIX_PATH" >"$VERIFY_ROOT/prereq-sweep.log" 2>&1 <<'PY'
from pathlib import Path
import json
import re
import sys

package_json_path = Path(sys.argv[1])
readme_path = Path(sys.argv[2])
tooling_doc_path = Path(sys.argv[3])
publish_workflow_path = Path(sys.argv[4])
expected_version = sys.argv[5]
expected_tag = sys.argv[6]
relative_vsix_path = sys.argv[7]

errors = []

version_pattern = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?")
tag_pattern = re.compile(r"ext-v[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?")
hardcoded_vsix_pattern = re.compile(r"mesh-lang-[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?\.vsix")

if not version_pattern.fullmatch(expected_version):
    errors.append(f"package version is malformed: {expected_version!r}")

if not tag_pattern.fullmatch(expected_tag):
    errors.append(f"EXPECTED_TAG is malformed: {expected_tag!r}")

canonical_tag = f"ext-v{expected_version}"
if expected_tag != canonical_tag:
    errors.append(
        f"EXPECTED_TAG drift: expected {canonical_tag!r} from package version {expected_version!r}, got {expected_tag!r}"
    )

try:
    package_json = json.loads(package_json_path.read_text())
except Exception as exc:  # pragma: no cover - surfaced by verifier
    raise SystemExit(f"failed to read {package_json_path}: {exc}")

actual_version = package_json.get("version")
if actual_version != expected_version:
    errors.append(
        f"package.json version drift: expected {expected_version!r}, found {actual_version!r}"
    )

scripts = package_json.get("scripts") or {}
if scripts.get("package") != "node ./scripts/vsix-path.mjs package":
    errors.append("package.json scripts.package must stay 'node ./scripts/vsix-path.mjs package'")
if scripts.get("install-local") != "node ./scripts/vsix-path.mjs install-local":
    errors.append(
        "package.json scripts.install-local must stay 'node ./scripts/vsix-path.mjs install-local'"
    )

package_name = package_json.get("name")
expected_relative_vsix = f"dist/{package_name}-{expected_version}.vsix"
if relative_vsix_path != expected_relative_vsix:
    errors.append(
        f"VSIX helper path drift: expected {expected_relative_vsix!r}, found {relative_vsix_path!r}"
    )

readme = readme_path.read_text()
tooling = tooling_doc_path.read_text()
workflow = publish_workflow_path.read_text()

expected_placeholder = f"dist/{package_name}-<version>.vsix"
required_doc_checks = {
    str(readme_path): ["npm run package", "npm run install-local", expected_placeholder],
    str(tooling_doc_path): ["npm run package", "npm run install-local", expected_placeholder],
}
for path, needles in required_doc_checks.items():
    text = readme if path == str(readme_path) else tooling
    for needle in needles:
        if needle not in text:
            errors.append(f"{path} must contain {needle!r}")

for path, text in {
    str(readme_path): readme,
    str(tooling_doc_path): tooling,
    str(publish_workflow_path): workflow,
}.items():
    match = hardcoded_vsix_pattern.search(text)
    if match:
        errors.append(f"{path} still hardcodes a versioned VSIX filename: {match.group(0)!r}")

if 'tags:\n      - "ext-v*"' not in workflow:
    errors.append("publish-extension workflow must keep the ext-v* tag trigger")

accepted_trigger_examples = {
    "# Trigger: git tag ext-vX.Y.Z && git push origin ext-vX.Y.Z",
    f"# Trigger: git tag {expected_tag} && git push origin {expected_tag}",
}
workflow_lines = set(workflow.splitlines())
if not workflow_lines.intersection(accepted_trigger_examples):
    errors.append(
        "publish-extension workflow comment must show either the generic ext-vX.Y.Z example or the current EXPECTED_TAG"
    )

print(f"package version: {expected_version}")
print(f"expected tag: {expected_tag}")
print(f"expected VSIX path: {relative_vsix_path}")
for path, needles in required_doc_checks.items():
    for needle in needles:
        print(f"ok: {path} contains {needle!r}")
print("ok: publish workflow keeps ext-v* trigger")
print("ok: publish workflow comment uses a non-stale tag example")
print("ok: docs/workflow avoid hardcoded versioned VSIX filenames")

if errors:
    print()
    print("errors:")
    for error in errors:
        print(f"- {error}")
    raise SystemExit(1)
PY
then
  fail_phase "prereq-sweep" "tag/package/docs/workflow prereq drift detected" "$VERIFY_ROOT/prereq-sweep.log"
fi

run_command \
  "npm-ci" \
  "npm-ci" \
  900 \
  "$EXTENSION_ROOT" \
  "npm ci" \
  npm ci

run_command \
  "compile" \
  "compile" \
  600 \
  "$EXTENSION_ROOT" \
  "npm run compile" \
  npm run compile

run_command \
  "package" \
  "package" \
  900 \
  "$EXTENSION_ROOT" \
  "npm run package" \
  npm run package

if [[ ! -f "$ABSOLUTE_VSIX_PATH" ]]; then
  fail_phase "package" "packaging completed without producing ${RELATIVE_VSIX_PATH}" "$LAST_LOG_PATH"
fi

set_phase "zip-audit"
echo "==> [zip-audit] python3 zipfile audit ${RELATIVE_VSIX_PATH}"
if ! python3 - "$ABSOLUTE_VSIX_PATH" "$VSIX_CONTENTS_PATH" >"$VERIFY_ROOT/zip-audit.log" 2>&1 <<'PY'
from pathlib import Path
import zipfile
import sys

vsix_path = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])

required_entries = [
    "extension/package.json",
    "extension/out/extension.js",
    "extension/readme.md",
    "extension/changelog.md",
    "extension/images/icon.png",
]
runtime_roots = [
    "extension/node_modules/vscode-languageclient",
    "extension/node_modules/vscode-jsonrpc",
]

if not vsix_path.is_file():
    raise SystemExit(f"VSIX is missing: {vsix_path}")

with zipfile.ZipFile(vsix_path) as archive:
    entries = sorted(info.filename for info in archive.infolist())

manifest_path.write_text("\n".join(entries) + "\n")
entry_set = set(entries)
errors = []

for required_entry in required_entries:
    if required_entry not in entry_set:
        errors.append(f"missing required VSIX entry {required_entry!r}")

for runtime_root in runtime_roots:
    runtime_entries = [entry for entry in entries if entry.startswith(runtime_root + "/")]
    if not runtime_entries:
      errors.append(f"missing runtime dependency entries under {runtime_root!r}")
      continue
    js_entries = [entry for entry in runtime_entries if entry.endswith('.js')]
    if not js_entries:
      errors.append(f"runtime dependency {runtime_root!r} contains no shipped .js files")
    package_json_entry = runtime_root + "/package.json"
    if package_json_entry not in entry_set:
      errors.append(f"runtime dependency is missing {package_json_entry!r}")
    print(f"runtime root: {runtime_root}")
    print(f"  entries: {len(runtime_entries)}")
    print(f"  js entries: {len(js_entries)}")

print(f"VSIX entries: {len(entries)}")
print(f"manifest: {manifest_path}")

if errors:
    print()
    print("errors:")
    for error in errors:
        print(f"- {error}")
    raise SystemExit(1)
PY
then
  fail_phase "zip-audit" "VSIX content audit failed for ${RELATIVE_VSIX_PATH}" "$VERIFY_ROOT/zip-audit.log"
fi

run_command \
  "e2e-lsp" \
  "e2e-lsp" \
  1800 \
  "$ROOT_DIR" \
  "cargo test -q -p meshc --test e2e_lsp -- --nocapture" \
  cargo test -q -p meshc --test e2e_lsp -- --nocapture

if ! python3 - "$LAST_LOG_PATH" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text(errors='replace')
counts = [int(value) for value in re.findall(r"running (\d+) test", text)]
if not counts:
    raise SystemExit("e2e_lsp output did not report a test count")
if max(counts) <= 0:
    raise SystemExit("e2e_lsp output reported 0 tests")
PY
then
  fail_phase "e2e-lsp" "e2e_lsp output did not report a non-zero test count" "$LAST_LOG_PATH"
fi
if ! grep -Fq 'test result: ok.' "$LAST_LOG_PATH"; then
  fail_phase "e2e-lsp" "e2e_lsp output did not report a passing result" "$LAST_LOG_PATH"
fi

printf '%s\n' "$REPO_RELATIVE_VSIX_PATH" >"$VERIFIED_VSIX_PATH_FILE"
printf 'ok\n' >"$STATUS_PATH"
printf 'complete\n' >"$CURRENT_PHASE_PATH"
rm -f "$FAILED_PHASE_PATH"

echo "verify-m034-s04-extension: ok"
