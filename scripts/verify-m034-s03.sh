#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TMP_ROOT="$ROOT_DIR/.tmp/m034-s03"
VERIFY_ROOT="$TMP_ROOT/verify"
STAGE_ROOT="$TMP_ROOT/stage"
SERVER_ROOT="$STAGE_ROOT/server"
HOME_ROOT="$TMP_ROOT/home"
WORK_ROOT="$TMP_ROOT/work"
FIXTURE_DIR="$ROOT_DIR/scripts/fixtures/m034-s03-installer-smoke"
INSTALL_SCRIPT="$ROOT_DIR/website/docs/public/install.sh"
INSTALL_PS1="$ROOT_DIR/website/docs/public/install.ps1"
REPO_INSTALL_SCRIPT="$ROOT_DIR/tools/install/install.sh"
REPO_INSTALL_PS1="$ROOT_DIR/tools/install/install.ps1"
MESHC_BIN="$ROOT_DIR/target/debug/meshc"
MESHPKG_BIN="$ROOT_DIR/target/debug/meshpkg"
PREBUILT_RELEASE_DIR="${M034_S03_PREBUILT_RELEASE_DIR:-}"
RUN_DIR="$VERIFY_ROOT/run"
SERVER_PID=""
LAST_STDOUT_PATH=""
LAST_STDERR_PATH=""
LAST_LOG_PATH=""
VERSION=""
TARGET=""
MESHC_ARCHIVE=""
MESHPKG_ARCHIVE=""
GOOD_ROOT="$SERVER_ROOT/good"
BAD_METADATA_ROOT="$SERVER_ROOT/bad-metadata"
BAD_SHA_ROOT="$SERVER_ROOT/bad-sha"
MISSING_MESHPKG_ROOT="$SERVER_ROOT/missing-meshpkg"
MISSING_BINARY_ROOT="$SERVER_ROOT/missing-binary"

cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

fail_phase() {
  local phase_name="$1"
  local reason="$2"
  local log_path="${3:-}"

  echo "verification drift: ${reason}" >&2
  echo "first failing phase: ${phase_name}" >&2
  echo "artifacts: ${RUN_DIR#$ROOT_DIR/}" >&2
  echo "staged root: ${SERVER_ROOT#$ROOT_DIR/}" >&2
  if [[ -n "$log_path" && -f "$log_path" ]]; then
    echo "--- ${log_path#$ROOT_DIR/} ---" >&2
    sed -n '1,260p' "$log_path" >&2
  fi
  exit 1
}

combine_command_log() {
  local display="$1"
  local stdout_path="$2"
  local stderr_path="$3"
  local log_path="$4"

  {
    echo "display: ${display}"
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
  local phase_name="$1"
  local label="$2"
  local display="$3"
  shift 3

  local stdout_path="$RUN_DIR/${label}.stdout"
  local stderr_path="$RUN_DIR/${label}.stderr"
  local log_path="$RUN_DIR/${label}.log"

  echo "==> [${phase_name}] ${display}"
  if ! "$@" >"$stdout_path" 2>"$stderr_path"; then
    combine_command_log "$display" "$stdout_path" "$stderr_path" "$log_path"
    fail_phase "$phase_name" "${display} failed" "$log_path"
  fi

  combine_command_log "$display" "$stdout_path" "$stderr_path" "$log_path"
  LAST_STDOUT_PATH="$stdout_path"
  LAST_STDERR_PATH="$stderr_path"
  LAST_LOG_PATH="$log_path"
}

expect_command_failure() {
  local phase_name="$1"
  local label="$2"
  local display="$3"
  shift 3

  local stdout_path="$RUN_DIR/${label}.stdout"
  local stderr_path="$RUN_DIR/${label}.stderr"
  local log_path="$RUN_DIR/${label}.log"

  echo "==> [${phase_name}] ${display} (expect failure)"
  if "$@" >"$stdout_path" 2>"$stderr_path"; then
    combine_command_log "$display" "$stdout_path" "$stderr_path" "$log_path"
    fail_phase "$phase_name" "${display} unexpectedly succeeded" "$log_path"
  fi

  combine_command_log "$display" "$stdout_path" "$stderr_path" "$log_path"
  LAST_STDOUT_PATH="$stdout_path"
  LAST_STDERR_PATH="$stderr_path"
  LAST_LOG_PATH="$log_path"
}

assert_log_contains() {
  local phase_name="$1"
  local needle="$2"
  local log_path="$3"

  if ! grep -Fq "$needle" "$log_path"; then
    fail_phase "$phase_name" "expected to find ${needle} in ${log_path#$ROOT_DIR/}" "$log_path"
  fi
}

repo_version() {
  python3 - <<'PY'
from pathlib import Path
import tomllib

meshc = tomllib.loads(Path('compiler/meshc/Cargo.toml').read_text())['package']['version']
meshpkg = tomllib.loads(Path('compiler/meshpkg/Cargo.toml').read_text())['package']['version']
if meshc != meshpkg:
    raise SystemExit(f"meshc ({meshc}) and meshpkg ({meshpkg}) versions diverged")
print(meshc)
PY
}

detect_target() {
  local ostype cputype
  ostype="$(uname -s)"
  cputype="$(uname -m)"

  case "$ostype" in
    Linux)
      ostype="unknown-linux-gnu"
      ;;
    Darwin)
      ostype="apple-darwin"
      if [[ "$cputype" == "x86_64" ]] && sysctl -n hw.optional.arm64 2>/dev/null | grep -q '1'; then
        cputype="aarch64"
      fi
      ;;
    *)
      echo "unsupported host OS: $ostype" >&2
      exit 1
      ;;
  esac

  case "$cputype" in
    x86_64|amd64)
      cputype="x86_64"
      ;;
    aarch64|arm64)
      cputype="aarch64"
      ;;
    *)
      echo "unsupported host architecture: $cputype" >&2
      exit 1
      ;;
  esac

  printf '%s\n' "${cputype}-${ostype}"
}

sha256_of_file() {
  python3 - "$1" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

print(sha256(Path(sys.argv[1]).read_bytes()).hexdigest())
PY
}

pick_free_port() {
  python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(('127.0.0.1', 0))
    print(sock.getsockname()[1])
PY
}

find_single_file() {
  local dir="$1"
  local pattern="$2"
  shopt -s nullglob
  local matches=("$dir"/$pattern)
  shopt -u nullglob

  if (( ${#matches[@]} != 1 )); then
    echo "expected exactly one match for ${dir}/${pattern}, found ${#matches[@]}" >&2
    return 1
  fi

  printf '%s\n' "${matches[0]}"
}

version_from_archive_name() {
  local prefix="$1"
  local archive_name="$2"
  local target="$3"
  local extension="$4"
  local version="${archive_name#${prefix}-v}"

  version="${version%-${target}.${extension}}"
  if [[ -z "$version" || "$version" == "$archive_name" ]]; then
    echo "could not infer version from ${archive_name}" >&2
    return 1
  fi

  printf '%s\n' "$version"
}

stage_tarball() {
  local archive_path="$1"
  local source_path="$2"
  local binary_name="$3"
  local tmpdir="$STAGE_ROOT/archive-${binary_name}"

  rm -rf "$tmpdir"
  mkdir -p "$tmpdir"
  cp "$source_path" "$tmpdir/$binary_name"
  tar czf "$archive_path" -C "$tmpdir" "$binary_name"
}

write_release_json() {
  local path="$1"
  local meshc_archive="$2"
  local meshpkg_archive="$3"

  cat >"$path" <<EOF
{
  "tag_name": "v${VERSION}",
  "name": "M034 S03 staged release",
  "assets": [
    {"name": "${meshc_archive}"},
    {"name": "${meshpkg_archive}"},
    {"name": "SHA256SUMS"}
  ]
}
EOF
}

setup_prebuilt_release_assets() {
  local asset_dir="$1"
  local meshc_source meshpkg_source checksum_source meshc_version meshpkg_version

  [[ -d "$asset_dir" ]] || fail_phase "setup" "prebuilt release asset dir was missing: ${asset_dir}"

  meshc_source="$(find_single_file "$asset_dir" "meshc-v*-${TARGET}.tar.gz")" || fail_phase "setup" "missing meshc archive for ${TARGET} in ${asset_dir}"
  meshpkg_source="$(find_single_file "$asset_dir" "meshpkg-v*-${TARGET}.tar.gz")" || fail_phase "setup" "missing meshpkg archive for ${TARGET} in ${asset_dir}"
  checksum_source="$asset_dir/SHA256SUMS"
  [[ -f "$checksum_source" ]] || fail_phase "setup" "missing SHA256SUMS in ${asset_dir}"

  MESHC_ARCHIVE="$(basename "$meshc_source")"
  MESHPKG_ARCHIVE="$(basename "$meshpkg_source")"
  meshc_version="$(version_from_archive_name "meshc" "$MESHC_ARCHIVE" "$TARGET" "tar.gz")" || fail_phase "setup" "could not infer meshc version from ${MESHC_ARCHIVE}"
  meshpkg_version="$(version_from_archive_name "meshpkg" "$MESHPKG_ARCHIVE" "$TARGET" "tar.gz")" || fail_phase "setup" "could not infer meshpkg version from ${MESHPKG_ARCHIVE}"
  [[ "$meshc_version" == "$meshpkg_version" ]] || fail_phase "setup" "meshc (${meshc_version}) and meshpkg (${meshpkg_version}) archive versions diverged"
  VERSION="$meshc_version"

  mkdir -p "$GOOD_ROOT/api/releases" "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}"
  cp "$meshc_source" "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}"
  cp "$meshpkg_source" "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}"
  cp "$checksum_source" "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/SHA256SUMS"
  write_release_json "$GOOD_ROOT/api/releases/latest.json" "$MESHC_ARCHIVE" "$MESHPKG_ARCHIVE"
}

setup_local_release_assets() {
  local meshc_sha meshpkg_sha bad_binary_sha

  VERSION="$(repo_version)"
  MESHC_ARCHIVE="meshc-v${VERSION}-${TARGET}.tar.gz"
  MESHPKG_ARCHIVE="meshpkg-v${VERSION}-${TARGET}.tar.gz"

  run_command tooling 05-build-tooling "cargo build -q -p mesh-rt -p meshc -p meshpkg" cargo build -q -p mesh-rt -p meshc -p meshpkg
  [[ -f "$ROOT_DIR/target/debug/libmesh_rt.a" ]] || fail_phase "tooling" "mesh-rt static library was not built" "$LAST_LOG_PATH"
  [[ -x "$MESHC_BIN" ]] || fail_phase "tooling" "meshc binary was not built" "$LAST_LOG_PATH"
  [[ -x "$MESHPKG_BIN" ]] || fail_phase "tooling" "meshpkg binary was not built" "$LAST_LOG_PATH"

  mkdir -p "$GOOD_ROOT/api/releases" "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}"
  mkdir -p "$BAD_METADATA_ROOT/api/releases" "$BAD_METADATA_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}"
  mkdir -p "$BAD_SHA_ROOT/api/releases" "$BAD_SHA_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}"
  mkdir -p "$MISSING_MESHPKG_ROOT/api/releases" "$MISSING_MESHPKG_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}"
  mkdir -p "$MISSING_BINARY_ROOT/api/releases" "$MISSING_BINARY_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}"

  stage_tarball "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}" "$MESHC_BIN" meshc
  stage_tarball "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}" "$MESHPKG_BIN" meshpkg

  meshc_sha="$(sha256_of_file "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}")"
  meshpkg_sha="$(sha256_of_file "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}")"
  cat >"$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/SHA256SUMS" <<EOF
${meshc_sha}  ${MESHC_ARCHIVE}
${meshpkg_sha}  ${MESHPKG_ARCHIVE}
EOF
  write_release_json "$GOOD_ROOT/api/releases/latest.json" "$MESHC_ARCHIVE" "$MESHPKG_ARCHIVE"

  cp "$GOOD_ROOT/api/releases/latest.json" "$BAD_SHA_ROOT/api/releases/latest.json"
  cp "$GOOD_ROOT/api/releases/latest.json" "$MISSING_MESHPKG_ROOT/api/releases/latest.json"
  cp "$GOOD_ROOT/api/releases/latest.json" "$MISSING_BINARY_ROOT/api/releases/latest.json"
  cp "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}" "$BAD_SHA_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}"
  cp "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}" "$BAD_SHA_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}"
  cp "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}" "$MISSING_MESHPKG_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}"
  cp "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}" "$MISSING_BINARY_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHC_ARCHIVE}"
  cp "$GOOD_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}" "$MISSING_BINARY_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}"

  cat >"$BAD_SHA_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/SHA256SUMS" <<EOF
${meshc_sha}  ${MESHC_ARCHIVE}
not-a-sha256  ${MESHPKG_ARCHIVE}
EOF
  cat >"$MISSING_MESHPKG_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/SHA256SUMS" <<EOF
${meshc_sha}  ${MESHC_ARCHIVE}
${meshpkg_sha}  ${MESHPKG_ARCHIVE}
EOF

  BAD_BINARY_TMP="$STAGE_ROOT/bad-meshpkg"
  rm -rf "$BAD_BINARY_TMP"
  mkdir -p "$BAD_BINARY_TMP"
  cp "$MESHPKG_BIN" "$BAD_BINARY_TMP/not-meshpkg"
  tar czf "$MISSING_BINARY_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}" -C "$BAD_BINARY_TMP" not-meshpkg
  bad_binary_sha="$(sha256_of_file "$MISSING_BINARY_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/${MESHPKG_ARCHIVE}")"
  cat >"$MISSING_BINARY_ROOT/snowdamiz/mesh-lang/releases/download/v${VERSION}/SHA256SUMS" <<EOF
${meshc_sha}  ${MESHC_ARCHIVE}
${bad_binary_sha}  ${MESHPKG_ARCHIVE}
EOF

  cat >"$BAD_METADATA_ROOT/api/releases/latest.json" <<EOF
{
  "name": "M034 S03 staged release without tag_name",
  "assets": []
}
EOF
}

start_server() {
  local port="$1"
  python3 -m http.server "$port" --bind 127.0.0.1 --directory "$SERVER_ROOT" >"$RUN_DIR/http-server.log" 2>&1 &
  SERVER_PID="$!"

  local attempt
  for attempt in {1..40}; do
    if curl -fsS "http://127.0.0.1:${port}/" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done

  fail_phase "server" "local staged release server did not become ready" "$RUN_DIR/http-server.log"
}

run_installer_failure() {
  local phase_name="$1"
  local label="$2"
  local api_url="$3"
  local base_url="$4"
  local home_dir="$5"
  shift 5

  mkdir -p "$home_dir"
  : >"$home_dir/.bashrc"

  expect_command_failure "$phase_name" "$label" "sh website/docs/public/install.sh --yes" \
    env \
      HOME="$home_dir" \
      SHELL="/bin/bash" \
      MESH_INSTALL_RELEASE_API_URL="$api_url" \
      MESH_INSTALL_RELEASE_BASE_URL="$base_url" \
      MESH_INSTALL_STRICT_PROOF=1 \
      MESH_INSTALL_DOWNLOAD_TIMEOUT_SEC=20 \
      sh "$INSTALL_SCRIPT" --yes

  local needle
  for needle in "$@"; do
    assert_log_contains "$phase_name" "$needle" "$LAST_LOG_PATH"
  done
}

rm -rf "$TMP_ROOT"
mkdir -p "$RUN_DIR" "$STAGE_ROOT" "$HOME_ROOT" "$WORK_ROOT"

TARGET="$(detect_target)"

run_command contract 01-shell-diff "diff -u tools/install/install.sh website/docs/public/install.sh" diff -u "$REPO_INSTALL_SCRIPT" "$INSTALL_SCRIPT"
run_command contract 02-ps1-diff "diff -u tools/install/install.ps1 website/docs/public/install.ps1" diff -u "$REPO_INSTALL_PS1" "$INSTALL_PS1"
run_command contract 03-shell-hooks "grep -nE 'MESH_INSTALL_RELEASE_API_URL|MESH_INSTALL_RELEASE_BASE_URL|MESH_INSTALL_STRICT_PROOF' website/docs/public/install.sh" grep -nE 'MESH_INSTALL_RELEASE_API_URL|MESH_INSTALL_RELEASE_BASE_URL|MESH_INSTALL_STRICT_PROOF' website/docs/public/install.sh
run_command contract 04-ps1-contract "grep -nE 'snowdamiz/mesh-lang|meshpkg' website/docs/public/install.ps1" grep -nE 'snowdamiz/mesh-lang|meshpkg' website/docs/public/install.ps1

if [[ -n "$PREBUILT_RELEASE_DIR" ]]; then
  setup_prebuilt_release_assets "$PREBUILT_RELEASE_DIR"
else
  setup_local_release_assets
fi

cat >"$RUN_DIR/00-context.log" <<EOF
version=${VERSION}
target=${TARGET}
prebuilt_release_dir=${PREBUILT_RELEASE_DIR:-none}
verify_root=${RUN_DIR#$ROOT_DIR/}
stage_root=${STAGE_ROOT#$ROOT_DIR/}
fixture_dir=${FIXTURE_DIR#$ROOT_DIR/}
EOF

find "$SERVER_ROOT" -type f | sort | sed "s#^$ROOT_DIR/##" >"$RUN_DIR/staged-layout.txt"

SERVER_PORT="$(pick_free_port)"
SERVER_URL="http://127.0.0.1:${SERVER_PORT}"
start_server "$SERVER_PORT"

GOOD_API_URL="$SERVER_URL/good/api/releases/latest.json"
GOOD_BASE_URL="$SERVER_URL/good/snowdamiz/mesh-lang/releases/download"
BAD_METADATA_API_URL="$SERVER_URL/bad-metadata/api/releases/latest.json"
BAD_METADATA_BASE_URL="$SERVER_URL/bad-metadata/snowdamiz/mesh-lang/releases/download"
BAD_SHA_API_URL="$SERVER_URL/bad-sha/api/releases/latest.json"
BAD_SHA_BASE_URL="$SERVER_URL/bad-sha/snowdamiz/mesh-lang/releases/download"
MISSING_MESHPKG_API_URL="$SERVER_URL/missing-meshpkg/api/releases/latest.json"
MISSING_MESHPKG_BASE_URL="$SERVER_URL/missing-meshpkg/snowdamiz/mesh-lang/releases/download"
MISSING_BINARY_API_URL="$SERVER_URL/missing-binary/api/releases/latest.json"
MISSING_BINARY_BASE_URL="$SERVER_URL/missing-binary/snowdamiz/mesh-lang/releases/download"

cat >"$RUN_DIR/server-urls.log" <<EOF
server_url=${SERVER_URL}
good_api_url=${GOOD_API_URL}
good_base_url=${GOOD_BASE_URL}
bad_metadata_api_url=${BAD_METADATA_API_URL}
bad_sha_base_url=${BAD_SHA_BASE_URL}
missing_meshpkg_base_url=${MISSING_MESHPKG_BASE_URL}
missing_binary_base_url=${MISSING_BINARY_BASE_URL}
EOF

GOOD_HOME="$HOME_ROOT/good"
mkdir -p "$GOOD_HOME"
: >"$GOOD_HOME/.bashrc"
run_command install 06-install-good "sh website/docs/public/install.sh --yes against staged release" \
  env \
    HOME="$GOOD_HOME" \
    SHELL="/bin/bash" \
    MESH_INSTALL_RELEASE_API_URL="$GOOD_API_URL" \
    MESH_INSTALL_RELEASE_BASE_URL="$GOOD_BASE_URL" \
    MESH_INSTALL_STRICT_PROOF=1 \
    MESH_INSTALL_DOWNLOAD_TIMEOUT_SEC=20 \
    sh "$INSTALL_SCRIPT" --yes

[[ -x "$GOOD_HOME/.mesh/bin/meshc" ]] || fail_phase "install" "installed meshc was missing" "$LAST_LOG_PATH"
[[ -x "$GOOD_HOME/.mesh/bin/meshpkg" ]] || fail_phase "install" "installed meshpkg was missing" "$LAST_LOG_PATH"
[[ -f "$GOOD_HOME/.mesh/version" ]] || fail_phase "install" "version file was not written" "$LAST_LOG_PATH"
[[ "$(tr -d '\r\n' <"$GOOD_HOME/.mesh/version")" == "$VERSION" ]] || fail_phase "install" "version file did not match staged version" "$LAST_LOG_PATH"

run_command version 07-meshc-version "installed meshc --version" "$GOOD_HOME/.mesh/bin/meshc" --version
assert_log_contains version "meshc ${VERSION}" "$LAST_LOG_PATH"
run_command version 08-meshpkg-version "installed meshpkg --version" "$GOOD_HOME/.mesh/bin/meshpkg" --version
assert_log_contains version "meshpkg ${VERSION}" "$LAST_LOG_PATH"

SMOKE_WORK_DIR="$WORK_ROOT/installer-smoke"
rm -rf "$SMOKE_WORK_DIR"
mkdir -p "$SMOKE_WORK_DIR"
cp "$FIXTURE_DIR/mesh.toml" "$SMOKE_WORK_DIR/mesh.toml"
cp "$FIXTURE_DIR/main.mpl" "$SMOKE_WORK_DIR/main.mpl"
run_command build 09-hello-build "installed meshc build installer smoke fixture" \
  env HOME="$GOOD_HOME" PATH="$GOOD_HOME/.mesh/bin:$PATH" "$GOOD_HOME/.mesh/bin/meshc" build "$SMOKE_WORK_DIR" --output "$RUN_DIR/installer-smoke.bin" --no-color
run_command runtime 10-hello-run "run installed hello binary" "$RUN_DIR/installer-smoke.bin"
if [[ "$(tr -d '\r\n' <"$LAST_STDOUT_PATH")" != "hello" ]]; then
  fail_phase "runtime" "installer smoke binary printed unexpected output" "$LAST_LOG_PATH"
fi

if [[ -z "$PREBUILT_RELEASE_DIR" ]]; then
  run_installer_failure metadata 11-missing-tag "$BAD_METADATA_API_URL" "$BAD_METADATA_BASE_URL" "$HOME_ROOT/bad-metadata" "tag_name" "$BAD_METADATA_API_URL"
  run_installer_failure checksum 12-bad-sha "$BAD_SHA_API_URL" "$BAD_SHA_BASE_URL" "$HOME_ROOT/bad-sha" "SHA256SUMS" "$MESHPKG_ARCHIVE"
  run_installer_failure download 13-missing-meshpkg "$MISSING_MESHPKG_API_URL" "$MISSING_MESHPKG_BASE_URL" "$HOME_ROOT/missing-meshpkg" "Failed to download meshpkg" "$MISSING_MESHPKG_BASE_URL/v${VERSION}/${MESHPKG_ARCHIVE}"
  run_installer_failure extract 14-missing-binary "$MISSING_BINARY_API_URL" "$MISSING_BINARY_BASE_URL" "$HOME_ROOT/missing-binary" "meshpkg was not found after extracting" "$MESHPKG_ARCHIVE"
fi

echo "verify-m034-s03: ok"
