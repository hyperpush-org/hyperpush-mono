#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

REGISTRY_URL="https://api.packages.meshlang.dev"
PACKAGES_SITE_URL="https://packages.meshlang.dev"
PACKAGE_SLUG="mesh-registry-proof"
PACKAGE_DESCRIPTION="Real registry publish/install proof fixture for M034 S01"
TMP_ROOT="$ROOT_DIR/.tmp/m034-s01"
VERIFY_ROOT="$TMP_ROOT/verify"
HOME_DIR="$TMP_ROOT/home"
WORK_ROOT="$TMP_ROOT/work"
PROOF_WORK_DIR="$WORK_ROOT/proof-package"
CONSUMER_WORK_DIR="$WORK_ROOT/consumer"
NAMED_INSTALL_WORK_DIR="$WORK_ROOT/named-install"
FIXTURE_ROOT="$ROOT_DIR/scripts/fixtures"
PROOF_FIXTURE_DIR="$FIXTURE_ROOT/m034-s01-proof-package"
CONSUMER_FIXTURE_DIR="$FIXTURE_ROOT/m034-s01-consumer"
MESHPKG_BIN="$ROOT_DIR/target/debug/meshpkg"
MESHC_BIN="$ROOT_DIR/target/debug/meshc"
LAST_STDOUT_PATH=""
LAST_STDERR_PATH=""
LAST_LOG_PATH=""
LAST_BODY_PATH=""
LAST_HEADERS_PATH=""
LAST_STATUS=""
FETCH_RETRY_ATTEMPTS="${M034_S01_FETCH_RETRY_ATTEMPTS:-3}"
FETCH_RETRY_DELAY_SECONDS="${M034_S01_FETCH_RETRY_DELAY_SECONDS:-2}"

fail_phase() {
  local phase_name="$1"
  local reason="$2"
  local log_path="${3:-}"

  echo "verification drift: ${reason}" >&2
  echo "first failing phase: ${phase_name}" >&2
  echo "package: ${PACKAGE_NAME:-<unset>}@${PROOF_VERSION:-<unset>}" >&2
  if [[ -n "${RUN_DIR:-}" ]]; then
    echo "artifacts: ${RUN_DIR}" >&2
  fi
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

run_command_with_stdin() {
  local phase_name="$1"
  local label="$2"
  local display="$3"
  local stdin_text="$4"
  shift 4

  local stdout_path="$RUN_DIR/${label}.stdout"
  local stderr_path="$RUN_DIR/${label}.stderr"
  local log_path="$RUN_DIR/${label}.log"

  echo "==> [${phase_name}] ${display}"
  if ! printf '%s\n' "$stdin_text" | "$@" >"$stdout_path" 2>"$stderr_path"; then
    combine_command_log "$display" "$stdout_path" "$stderr_path" "$log_path"
    fail_phase "$phase_name" "${display} failed" "$log_path"
  fi

  combine_command_log "$display" "$stdout_path" "$stderr_path" "$log_path"
  LAST_STDOUT_PATH="$stdout_path"
  LAST_STDERR_PATH="$stderr_path"
  LAST_LOG_PATH="$log_path"
}

run_command_in_dir() {
  local phase_name="$1"
  local label="$2"
  local display="$3"
  local work_dir="$4"
  shift 4

  local stdout_path="$RUN_DIR/${label}.stdout"
  local stderr_path="$RUN_DIR/${label}.stderr"
  local log_path="$RUN_DIR/${label}.log"

  echo "==> [${phase_name}] ${display}"
  if ! (
    cd "$work_dir"
    "$@"
  ) >"$stdout_path" 2>"$stderr_path"; then
    combine_command_log "$display" "$stdout_path" "$stderr_path" "$log_path"
    fail_phase "$phase_name" "${display} failed" "$log_path"
  fi

  combine_command_log "$display" "$stdout_path" "$stderr_path" "$log_path"
  LAST_STDOUT_PATH="$stdout_path"
  LAST_STDERR_PATH="$stderr_path"
  LAST_LOG_PATH="$log_path"
}

validate_version() {
  python3 - "$1" <<'PY'
import re
import sys

version = sys.argv[1]
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?", version):
    raise SystemExit(1)
PY
}

generate_version() {
  python3 <<'PY'
from datetime import datetime, timezone
import os

stamp = datetime.now(timezone.utc).strftime("%Y%m%d%H%M%S")
print(f"0.34.0-{stamp}-{os.getpid()}")
PY
}

fetch_url() {
  local label="$1"
  local url="$2"
  local timeout_seconds="$3"

  local body_path="$RUN_DIR/${label}.body"
  local headers_path="$RUN_DIR/${label}.headers"
  local status_path="$RUN_DIR/${label}.status"
  local stderr_path="$RUN_DIR/${label}.stderr"
  local log_path="$RUN_DIR/${label}.log"

  if curl --silent --show-error --location --connect-timeout 10 --max-time "$timeout_seconds" \
    --dump-header "$headers_path" \
    --output "$body_path" \
    --write-out '%{http_code}' \
    "$url" >"$status_path" 2>"$stderr_path"; then
    :
  else
    {
      echo "url: ${url}"
      echo "curl transport failure"
      if [[ -s "$stderr_path" ]]; then
        echo
        echo "[stderr]"
        cat "$stderr_path"
      fi
    } >"$log_path"
    LAST_BODY_PATH="$body_path"
    LAST_HEADERS_PATH="$headers_path"
    LAST_STATUS=""
    LAST_LOG_PATH="$log_path"
    return 1
  fi

  LAST_STATUS="$(tr -d '\n' <"$status_path")"
  {
    echo "url: ${url}"
    echo "status: ${LAST_STATUS}"
    echo "body: ${body_path#$ROOT_DIR/}"
    echo "headers: ${headers_path#$ROOT_DIR/}"
    if [[ -s "$stderr_path" ]]; then
      echo
      echo "[stderr]"
      cat "$stderr_path"
    fi
  } >"$log_path"
  LAST_BODY_PATH="$body_path"
  LAST_HEADERS_PATH="$headers_path"
  LAST_LOG_PATH="$log_path"
  return 0
}

copy_fetch_attempt_artifacts() {
  local base_label="$1"
  local attempt_label="$2"

  local ext
  for ext in body headers status stderr log; do
    local src="$RUN_DIR/${attempt_label}.${ext}"
    local dst="$RUN_DIR/${base_label}.${ext}"
    if [[ -f "$src" ]]; then
      cp "$src" "$dst"
    fi
  done
}

fetch_required_status_with_retry() {
  local phase_name="$1"
  local label="$2"
  local url="$3"
  local expected_status="$4"
  local timeout_seconds="$5"
  local retry_attempts="${6:-$FETCH_RETRY_ATTEMPTS}"
  local retry_delay_seconds="${7:-$FETCH_RETRY_DELAY_SECONDS}"

  local attempt
  for ((attempt = 1; attempt <= retry_attempts; attempt++)); do
    local attempt_label="$label-attempt${attempt}"
    echo "==> [${phase_name}] GET ${url} (attempt ${attempt}/${retry_attempts})"
    if fetch_url "$attempt_label" "$url" "$timeout_seconds"; then
      copy_fetch_attempt_artifacts "$label" "$attempt_label"
      LAST_BODY_PATH="$RUN_DIR/${label}.body"
      LAST_HEADERS_PATH="$RUN_DIR/${label}.headers"
      LAST_LOG_PATH="$RUN_DIR/${label}.log"
      if [[ "$LAST_STATUS" == "$expected_status" ]]; then
        return 0
      fi
      fail_phase "$phase_name" "GET ${url} returned HTTP ${LAST_STATUS}, expected ${expected_status}" "$LAST_LOG_PATH"
    fi

    copy_fetch_attempt_artifacts "$label" "$attempt_label"
    LAST_BODY_PATH="$RUN_DIR/${label}.body"
    LAST_HEADERS_PATH="$RUN_DIR/${label}.headers"
    LAST_LOG_PATH="$RUN_DIR/${label}.log"

    if (( attempt < retry_attempts )); then
      sleep "$retry_delay_seconds"
    fi
  done

  fail_phase "$phase_name" "GET ${url} timed out or failed to connect after ${retry_attempts} attempts" "$LAST_LOG_PATH"
}

fetch_required_status() {
  local phase_name="$1"
  local label="$2"
  local url="$3"
  local expected_status="$4"
  local timeout_seconds="$5"

  echo "==> [${phase_name}] GET ${url}"
  if ! fetch_url "$label" "$url" "$timeout_seconds"; then
    fail_phase "$phase_name" "GET ${url} timed out or failed to connect" "$LAST_LOG_PATH"
  fi
  if [[ "$LAST_STATUS" != "$expected_status" ]]; then
    fail_phase "$phase_name" "GET ${url} returned HTTP ${LAST_STATUS}, expected ${expected_status}" "$LAST_LOG_PATH"
  fi
}

fetch_visibility_page() {
  local phase_name="$1"
  local label_prefix="$2"
  local url="$3"
  local expected_a="$4"
  local expected_b="$5"

  local attempt
  for attempt in 1 2; do
    local label="${label_prefix}-attempt${attempt}"
    echo "==> [${phase_name}] GET ${url} (attempt ${attempt})"
    if fetch_url "$label" "$url" 20 && [[ "$LAST_STATUS" == "200" ]] \
      && grep -Fq "$expected_a" "$LAST_BODY_PATH" \
      && grep -Fq "$expected_b" "$LAST_BODY_PATH"; then
      return 0
    fi
    if [[ "$attempt" -eq 1 ]]; then
      sleep 3
    fi
  done

  fail_phase "$phase_name" "visibility check failed for ${url}" "$LAST_LOG_PATH"
}

sha256_of_file() {
  python3 - "$1" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

print(sha256(Path(sys.argv[1]).read_bytes()).hexdigest())
PY
}

post_duplicate_publish() {
  local label="$1"
  local url="$2"
  local tarball_path="$3"
  local sha256="$4"

  local body_path="$RUN_DIR/${label}.body"
  local headers_path="$RUN_DIR/${label}.headers"
  local status_path="$RUN_DIR/${label}.status"
  local stderr_path="$RUN_DIR/${label}.stderr"
  local log_path="$RUN_DIR/${label}.log"

  if curl --silent --show-error --location --connect-timeout 10 --max-time 20 \
    --request POST \
    --header "Authorization: Bearer ${MESH_PUBLISH_TOKEN}" \
    --header "Content-Type: application/octet-stream" \
    --header "X-Package-Name: ${PACKAGE_NAME}" \
    --header "X-Package-Version: ${PROOF_VERSION}" \
    --header "X-Package-SHA256: ${sha256}" \
    --header "X-Package-Description: ${PACKAGE_DESCRIPTION}" \
    --header 'Expect:' \
    --data-binary "@${tarball_path}" \
    --dump-header "$headers_path" \
    --output "$body_path" \
    --write-out '%{http_code}' \
    "$url" >"$status_path" 2>"$stderr_path"; then
    :
  else
    {
      echo "url: ${url}"
      echo "duplicate publish transport failure"
      if [[ -s "$stderr_path" ]]; then
        echo
        echo "[stderr]"
        cat "$stderr_path"
      fi
    } >"$log_path"
    LAST_STATUS=""
    LAST_BODY_PATH="$body_path"
    LAST_HEADERS_PATH="$headers_path"
    LAST_LOG_PATH="$log_path"
    return 1
  fi

  LAST_STATUS="$(tr -d '\n' <"$status_path")"
  {
    echo "url: ${url}"
    echo "status: ${LAST_STATUS}"
    echo "body: ${body_path#$ROOT_DIR/}"
    echo "headers: ${headers_path#$ROOT_DIR/}"
    if [[ -s "$stderr_path" ]]; then
      echo
      echo "[stderr]"
      cat "$stderr_path"
    fi
  } >"$log_path"
  LAST_BODY_PATH="$body_path"
  LAST_HEADERS_PATH="$headers_path"
  LAST_LOG_PATH="$log_path"
  return 0
}

if [[ "${M034_S01_LIB_ONLY:-0}" == "1" ]]; then
  return 0 2>/dev/null || exit 0
fi

if [[ -z "${MESH_PUBLISH_OWNER:-}" ]]; then
  echo "MESH_PUBLISH_OWNER is required" >&2
  exit 1
fi
if [[ -z "${MESH_PUBLISH_TOKEN:-}" ]]; then
  echo "MESH_PUBLISH_TOKEN is required" >&2
  exit 1
fi
if ! [[ "$MESH_PUBLISH_OWNER" =~ ^[A-Za-z0-9-]+$ ]]; then
  echo "MESH_PUBLISH_OWNER must look like a GitHub login" >&2
  exit 1
fi

if [[ -n "${MESH_PROOF_VERSION+x}" ]]; then
  if [[ -z "$MESH_PROOF_VERSION" ]]; then
    echo "MESH_PROOF_VERSION cannot be empty when set" >&2
    exit 1
  fi
  if ! validate_version "$MESH_PROOF_VERSION"; then
    echo "MESH_PROOF_VERSION must look like semver (for example 0.34.0-proof.1)" >&2
    exit 1
  fi
  PROOF_VERSION="$MESH_PROOF_VERSION"
else
  PROOF_VERSION="$(generate_version)"
fi

PACKAGE_NAME="${MESH_PUBLISH_OWNER}/${PACKAGE_SLUG}"
RUN_DIR="$VERIFY_ROOT/$PROOF_VERSION"
SEARCH_QUERY="$(python3 - "$PACKAGE_NAME" <<'PY'
import sys
import urllib.parse
print(urllib.parse.quote(sys.argv[1], safe=''))
PY
)"
PACKAGE_URL="$REGISTRY_URL/api/v1/packages/$MESH_PUBLISH_OWNER/$PACKAGE_SLUG"
VERSION_URL="$PACKAGE_URL/$PROOF_VERSION"
VERSIONS_URL="$PACKAGE_URL/versions"
SEARCH_URL="$REGISTRY_URL/api/v1/packages?search=$SEARCH_QUERY"
DOWNLOAD_URL="$VERSION_URL/download"
DETAIL_PAGE_URL="$PACKAGES_SITE_URL/packages/$MESH_PUBLISH_OWNER/$PACKAGE_SLUG"
SEARCH_PAGE_URL="$PACKAGES_SITE_URL/search?q=$SEARCH_QUERY"
export PACKAGE_NAME PROOF_VERSION PACKAGE_DESCRIPTION DOWNLOAD_URL

rm -rf "$HOME_DIR" "$WORK_ROOT" "$RUN_DIR"
mkdir -p "$HOME_DIR" "$PROOF_WORK_DIR" "$CONSUMER_WORK_DIR" "$NAMED_INSTALL_WORK_DIR" "$RUN_DIR"

cat >"$RUN_DIR/00-context.log" <<EOF
package=${PACKAGE_NAME}
version=${PROOF_VERSION}
registry_url=${REGISTRY_URL}
packages_site_url=${PACKAGES_SITE_URL}
proof_workspace=${PROOF_WORK_DIR#$ROOT_DIR/}
consumer_workspace=${CONSUMER_WORK_DIR#$ROOT_DIR/}
named_install_workspace=${NAMED_INSTALL_WORK_DIR#$ROOT_DIR/}
EOF

python3 - "$PROOF_FIXTURE_DIR/mesh.toml.template" "$PROOF_WORK_DIR/mesh.toml" <<'PY'
from pathlib import Path
import os
import sys

src, dst = sys.argv[1:3]
text = Path(src).read_text()
text = text.replace("__PACKAGE_NAME__", os.environ["PACKAGE_NAME"])
text = text.replace("__VERSION__", os.environ["PROOF_VERSION"])
if "__PACKAGE_NAME__" in text or "__VERSION__" in text:
    raise SystemExit("unrendered placeholder remained in proof package template")
Path(dst).write_text(text)
PY
cp "$PROOF_FIXTURE_DIR/registry_proof.mpl" "$PROOF_WORK_DIR/registry_proof.mpl"

python3 - "$CONSUMER_FIXTURE_DIR/mesh.toml.template" "$CONSUMER_WORK_DIR/mesh.toml" <<'PY'
from pathlib import Path
import os
import sys

src, dst = sys.argv[1:3]
text = Path(src).read_text()
text = text.replace("__PACKAGE_NAME__", os.environ["PACKAGE_NAME"])
text = text.replace("__VERSION__", os.environ["PROOF_VERSION"])
if "__PACKAGE_NAME__" in text or "__VERSION__" in text:
    raise SystemExit("unrendered placeholder remained in consumer template")
Path(dst).write_text(text)
PY
cp "$CONSUMER_FIXTURE_DIR/main.mpl" "$CONSUMER_WORK_DIR/main.mpl"
cp "$CONSUMER_WORK_DIR/mesh.toml" "$NAMED_INSTALL_WORK_DIR/mesh.toml"
cp "$CONSUMER_WORK_DIR/main.mpl" "$NAMED_INSTALL_WORK_DIR/main.mpl"
cp "$PROOF_WORK_DIR/mesh.toml" "$RUN_DIR/proof-package.mesh.toml"
cp "$CONSUMER_WORK_DIR/mesh.toml" "$RUN_DIR/consumer.mesh.toml"
cp "$NAMED_INSTALL_WORK_DIR/mesh.toml" "$RUN_DIR/named-install.mesh.toml.before"

if ! grep -Fq "\"$PACKAGE_NAME\" = \"$PROOF_VERSION\"" "$CONSUMER_WORK_DIR/mesh.toml"; then
  fail_phase "render" "consumer manifest did not render a quoted scoped dependency key" "$RUN_DIR/00-context.log"
fi
if ! grep -Fq "\"$PACKAGE_NAME\" = \"$PROOF_VERSION\"" "$NAMED_INSTALL_WORK_DIR/mesh.toml"; then
  fail_phase "render" "named-install manifest did not preserve the quoted scoped dependency key" "$RUN_DIR/00-context.log"
fi

run_command contract 00a-docs-scoped-key "grep -nF '\"your-login/your-package\" = \"1.0.0\"' website/docs/docs/tooling/index.md" grep -nF '"your-login/your-package" = "1.0.0"' website/docs/docs/tooling/index.md
run_command contract 00b-named-install-contract "grep -nE 'does not edit mesh.toml|updates mesh.lock' website/docs/docs/tooling/index.md compiler/meshpkg/src/install.rs" grep -nE 'does not edit mesh.toml|updates mesh.lock' website/docs/docs/tooling/index.md compiler/meshpkg/src/install.rs

run_command tooling 01-build-tooling "cargo build -q -p mesh-rt -p meshpkg -p meshc" cargo build -q -p mesh-rt -p meshpkg -p meshc
[[ -f "$ROOT_DIR/target/debug/libmesh_rt.a" ]] || fail_phase "tooling" "mesh-rt static library was not built" "$LAST_LOG_PATH"
[[ -x "$MESHPKG_BIN" ]] || fail_phase "tooling" "meshpkg binary was not built" "$LAST_LOG_PATH"
[[ -x "$MESHC_BIN" ]] || fail_phase "tooling" "meshc binary was not built" "$LAST_LOG_PATH"

run_command_with_stdin auth 02-login "meshpkg --json login" "$MESH_PUBLISH_TOKEN" env HOME="$HOME_DIR" "$MESHPKG_BIN" --json login
python3 - "$LAST_STDOUT_PATH" <<'PY'
from pathlib import Path
import json
import sys

data = json.loads(Path(sys.argv[1]).read_text())
if data.get("status") != "ok":
    raise SystemExit("login JSON missing ok status")
PY
cp "$LAST_STDOUT_PATH" "$RUN_DIR/login.json"

run_command_in_dir publish 03-publish "meshpkg --json publish" "$PROOF_WORK_DIR" env HOME="$HOME_DIR" "$MESHPKG_BIN" --json publish --registry "$REGISTRY_URL"
PUBLISH_SHA="$(python3 - "$LAST_STDOUT_PATH" <<'PY'
from pathlib import Path
import json
import os
import re
import sys

data = json.loads(Path(sys.argv[1]).read_text())
if data.get("status") != "ok":
    raise SystemExit("publish JSON missing ok status")
expected_name = os.environ["PACKAGE_NAME"]
expected_version = os.environ["PROOF_VERSION"]
if data.get("name") != expected_name:
    raise SystemExit(f"publish JSON reported {data.get('name')!r}, expected {expected_name!r}")
if data.get("version") != expected_version:
    raise SystemExit(f"publish JSON reported {data.get('version')!r}, expected {expected_version!r}")
sha256_value = data.get("sha256")
if not isinstance(sha256_value, str) or not re.fullmatch(r"[0-9a-f]{64}", sha256_value):
    raise SystemExit("publish JSON missing a 64-character sha256")
print(sha256_value)
PY
)"
cp "$LAST_STDOUT_PATH" "$RUN_DIR/publish.json"

fetch_required_status_with_retry metadata 04-package-meta "$PACKAGE_URL" 200 20
cp "$LAST_BODY_PATH" "$RUN_DIR/package.json"
fetch_required_status_with_retry metadata 05-version-meta "$VERSION_URL" 200 20
cp "$LAST_BODY_PATH" "$RUN_DIR/version.json"
fetch_required_status_with_retry metadata 06-versions "$VERSIONS_URL" 200 20
cp "$LAST_BODY_PATH" "$RUN_DIR/versions.json"
fetch_required_status_with_retry metadata 07-search "$SEARCH_URL" 200 20
cp "$LAST_BODY_PATH" "$RUN_DIR/search.json"
fetch_required_status download 08-download "$DOWNLOAD_URL" 200 20
cp "$LAST_BODY_PATH" "$RUN_DIR/download.tar.gz"
DOWNLOAD_SHA="$(sha256_of_file "$RUN_DIR/download.tar.gz")"
printf '%s\n' "$DOWNLOAD_SHA" >"$RUN_DIR/download.sha256"

run_command_in_dir install 09-install "meshpkg --json install" "$CONSUMER_WORK_DIR" env HOME="$HOME_DIR" "$MESHPKG_BIN" --json install --registry "$REGISTRY_URL"
cp "$LAST_STDOUT_PATH" "$RUN_DIR/install.json"
cp "$CONSUMER_WORK_DIR/mesh.lock" "$RUN_DIR/mesh.lock"

python3 - "$RUN_DIR/publish.json" "$RUN_DIR/package.json" "$RUN_DIR/version.json" "$RUN_DIR/versions.json" "$RUN_DIR/search.json" "$RUN_DIR/install.json" "$RUN_DIR/mesh.lock" "$DOWNLOAD_SHA" <<'PY'
from pathlib import Path
import json
import os
import sys
import tomllib

publish_path, package_path, version_path, versions_path, search_path, install_path, lock_path, download_sha = sys.argv[1:9]
package_name = os.environ["PACKAGE_NAME"]
proof_version = os.environ["PROOF_VERSION"]
expected_download_url = os.environ["DOWNLOAD_URL"]
publish = json.loads(Path(publish_path).read_text())
publish_sha = publish["sha256"]
package = json.loads(Path(package_path).read_text())
if package.get("name") != package_name:
    raise SystemExit(f"package metadata name drifted: {package.get('name')!r}")
latest = package.get("latest")
if not isinstance(latest, dict):
    raise SystemExit("package metadata missing latest object")
if latest.get("version") != proof_version:
    raise SystemExit(f"package latest version drifted: {latest.get('version')!r}")
if latest.get("sha256") != publish_sha:
    raise SystemExit("package latest sha256 drifted")
if package.get("description") != os.environ["PACKAGE_DESCRIPTION"]:
    raise SystemExit("package description drifted")
version_meta = json.loads(Path(version_path).read_text())
if version_meta.get("sha256") != publish_sha:
    raise SystemExit("version metadata sha256 drifted")
versions = json.loads(Path(versions_path).read_text())
if not isinstance(versions, list) or not any(item.get("version") == proof_version for item in versions):
    raise SystemExit("versions list does not include the published version")
search = json.loads(Path(search_path).read_text())
if not isinstance(search, list):
    raise SystemExit("search response was not an array")
matching = [item for item in search if item.get("name") == package_name]
if not matching:
    raise SystemExit("search results did not include the published package")
if not any(item.get("version") == proof_version for item in matching):
    raise SystemExit("search results did not expose the published version")
install = json.loads(Path(install_path).read_text())
if install.get("status") != "ok":
    raise SystemExit("install JSON missing ok status")
if install.get("lockfile") != "mesh.lock":
    raise SystemExit("install JSON did not report mesh.lock")
if download_sha != publish_sha:
    raise SystemExit("downloaded tarball sha256 did not match publish metadata")
lock = tomllib.loads(Path(lock_path).read_text())
if lock.get("version") != 1:
    raise SystemExit("mesh.lock format version drifted")
packages = lock.get("packages")
if not isinstance(packages, list):
    raise SystemExit("mesh.lock packages was not a list")
entry = next((pkg for pkg in packages if pkg.get("name") == package_name), None)
if entry is None:
    raise SystemExit("mesh.lock did not record the published package")
if entry.get("version") != proof_version:
    raise SystemExit("mesh.lock version drifted")
if entry.get("revision") != proof_version:
    raise SystemExit("mesh.lock revision drifted")
if entry.get("source") != expected_download_url:
    raise SystemExit("mesh.lock source drifted")
if entry.get("sha256") != publish_sha:
    raise SystemExit("mesh.lock sha256 drifted")
PY

INSTALLED_PACKAGE_DIR="$CONSUMER_WORK_DIR/.mesh/packages/$MESH_PUBLISH_OWNER/$PACKAGE_SLUG@$PROOF_VERSION"
[[ -d "$INSTALLED_PACKAGE_DIR" ]] || fail_phase "install" "installed package directory was not created" "$RUN_DIR/09-install.log"
[[ -f "$INSTALLED_PACKAGE_DIR/registry_proof.mpl" ]] || fail_phase "install" "installed package module was not extracted" "$RUN_DIR/09-install.log"
[[ -f "$INSTALLED_PACKAGE_DIR/mesh.toml" ]] || fail_phase "install" "installed package manifest was not extracted" "$RUN_DIR/09-install.log"

run_command_in_dir install 09b-named-install "meshpkg --json install $PACKAGE_NAME" "$NAMED_INSTALL_WORK_DIR" env HOME="$HOME_DIR" "$MESHPKG_BIN" --json install "$PACKAGE_NAME" --registry "$REGISTRY_URL"
cp "$LAST_STDOUT_PATH" "$RUN_DIR/named-install.json"
cp "$NAMED_INSTALL_WORK_DIR/mesh.lock" "$RUN_DIR/named-install.mesh.lock"
cp "$NAMED_INSTALL_WORK_DIR/mesh.toml" "$RUN_DIR/named-install.mesh.toml.after"
if ! cmp -s "$RUN_DIR/named-install.mesh.toml.before" "$RUN_DIR/named-install.mesh.toml.after"; then
  diff -u "$RUN_DIR/named-install.mesh.toml.before" "$RUN_DIR/named-install.mesh.toml.after" >"$RUN_DIR/09b-named-install-manifest.log" || true
  fail_phase "install" "named install edited mesh.toml" "$RUN_DIR/09b-named-install-manifest.log"
fi

python3 - "$RUN_DIR/named-install.json" "$RUN_DIR/named-install.mesh.lock" <<'PY'
from pathlib import Path
import json
import os
import sys
import tomllib

install_path, lock_path = sys.argv[1:3]
package_name = os.environ["PACKAGE_NAME"]
proof_version = os.environ["PROOF_VERSION"]
expected_download_url = os.environ["DOWNLOAD_URL"]
install = json.loads(Path(install_path).read_text())
if install.get("status") != "ok":
    raise SystemExit("named install JSON missing ok status")
if install.get("name") != package_name:
    raise SystemExit("named install JSON reported the wrong package name")
if install.get("version") != proof_version:
    raise SystemExit("named install JSON reported the wrong version")
if install.get("lockfile") != "mesh.lock":
    raise SystemExit("named install JSON did not report mesh.lock")
if install.get("manifest_changed") is not False:
    raise SystemExit("named install JSON did not report manifest stability")
lock = tomllib.loads(Path(lock_path).read_text())
if lock.get("version") != 1:
    raise SystemExit("named install mesh.lock format version drifted")
packages = lock.get("packages")
if not isinstance(packages, list):
    raise SystemExit("named install mesh.lock packages was not a list")
entry = next((pkg for pkg in packages if pkg.get("name") == package_name), None)
if entry is None:
    raise SystemExit("named install mesh.lock did not record the published package")
if entry.get("version") != proof_version:
    raise SystemExit("named install mesh.lock version drifted")
if entry.get("revision") != proof_version:
    raise SystemExit("named install mesh.lock revision drifted")
if entry.get("source") != expected_download_url:
    raise SystemExit("named install mesh.lock source drifted")
PY

NAMED_INSTALL_PACKAGE_DIR="$NAMED_INSTALL_WORK_DIR/.mesh/packages/$MESH_PUBLISH_OWNER/$PACKAGE_SLUG@$PROOF_VERSION"
[[ -d "$NAMED_INSTALL_PACKAGE_DIR" ]] || fail_phase "install" "named install package directory was not created" "$RUN_DIR/09b-named-install.log"
[[ -f "$NAMED_INSTALL_PACKAGE_DIR/registry_proof.mpl" ]] || fail_phase "install" "named install package module was not extracted" "$RUN_DIR/09b-named-install.log"
[[ -f "$NAMED_INSTALL_PACKAGE_DIR/mesh.toml" ]] || fail_phase "install" "named install package manifest was not extracted" "$RUN_DIR/09b-named-install.log"

run_command build 10-consumer-build "meshc build consumer" "$MESHC_BIN" build "$CONSUMER_WORK_DIR" --output "$RUN_DIR/m034-s01-consumer.bin" --no-color
run_command runtime 11-consumer-run "run built consumer" "$RUN_DIR/m034-s01-consumer.bin"
if [[ "$(tr -d '\r\n' <"$LAST_STDOUT_PATH")" != "registry proof ok" ]]; then
  fail_phase "runtime" "consumer binary printed unexpected output" "$LAST_LOG_PATH"
fi

echo "==> [duplicate] POST ${REGISTRY_URL}/api/v1/packages"
if ! post_duplicate_publish 12-duplicate-publish "$REGISTRY_URL/api/v1/packages" "$RUN_DIR/download.tar.gz" "$PUBLISH_SHA"; then
  fail_phase "duplicate" "duplicate publish transport failed" "$LAST_LOG_PATH"
fi
if [[ "$LAST_STATUS" != "409" ]]; then
  fail_phase "duplicate" "duplicate publish returned HTTP ${LAST_STATUS}, expected 409" "$LAST_LOG_PATH"
fi

fetch_visibility_page visibility 13-detail-page "$DETAIL_PAGE_URL" "$PACKAGE_NAME" "v$PROOF_VERSION"
fetch_visibility_page visibility 14-search-page "$SEARCH_PAGE_URL" "$PACKAGE_NAME" "v$PROOF_VERSION"

printf '%s\n' "$PACKAGE_NAME@$PROOF_VERSION" >"$RUN_DIR/package-version.txt"
echo "verify-m034-s01: ok"
