#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="$ROOT_DIR/.tmp/m055-s04/landing-surface/verify"
STATUS_PATH="$ARTIFACT_DIR/status.txt"
CURRENT_PHASE_PATH="$ARTIFACT_DIR/current-phase.txt"
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
FULL_LOG_PATH="$ARTIFACT_DIR/full-contract.log"

record_phase() {
  printf '%s\t%s\n' "$1" "$2" >>"$PHASE_REPORT_PATH"
}

begin_phase() {
  printf '%s\n' "$1" >"$CURRENT_PHASE_PATH"
  record_phase "$1" started
  echo "==> [$1] $2"
}

fail_phase() {
  local phase_name="$1"
  local reason="$2"
  local log_path="${3:-}"

  printf 'failed\n' >"$STATUS_PATH"
  printf '%s\n' "$phase_name" >"$CURRENT_PHASE_PATH"
  record_phase "$phase_name" failed

  echo "verification drift: ${reason}" >&2
  echo "artifacts: ${ARTIFACT_DIR#$ROOT_DIR/}" >&2
  if [[ -n "$log_path" && -f "$log_path" ]]; then
    echo "failing log: ${log_path#$ROOT_DIR/}" >&2
    sed -n '1,220p' "$log_path" >&2
  fi
  exit 1
}

finish_phase() {
  record_phase "$1" passed
}

require_file() {
  local phase_name="$1"
  local relative_path="$2"
  local description="$3"
  local absolute_path="$ROOT_DIR/$relative_path"
  if [[ -f "$absolute_path" ]]; then
    return 0
  fi
  local log_path="$ARTIFACT_DIR/${phase_name}.log"
  printf 'missing %s: %s\n' "$description" "$relative_path" >"$log_path"
  fail_phase "$phase_name" "missing ${description}: ${relative_path}" "$log_path"
}

require_contains() {
  local phase_name="$1"
  local relative_path="$2"
  local needle="$3"
  local log_path="$ARTIFACT_DIR/${phase_name}.log"
  if ! grep -Fq -- "$needle" "$ROOT_DIR/$relative_path"; then
    printf '%s missing %s\n' "$relative_path" "$needle" >"$log_path"
    fail_phase "$phase_name" "${relative_path} missing ${needle}" "$log_path"
  fi
}

require_absent() {
  local phase_name="$1"
  local relative_path="$2"
  local needle="$3"
  local log_path="$ARTIFACT_DIR/${phase_name}.log"
  if grep -Fq -- "$needle" "$ROOT_DIR/$relative_path"; then
    printf '%s still contains %s\n' "$relative_path" "$needle" >"$log_path"
    fail_phase "$phase_name" "${relative_path} still contains stale text ${needle}" "$log_path"
  fi
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

rm -rf "$ARTIFACT_DIR"
mkdir -p "$ARTIFACT_DIR"
: >"$PHASE_REPORT_PATH"
printf 'running\n' >"$STATUS_PATH"
printf 'bootstrap\n' >"$CURRENT_PHASE_PATH"
exec > >(tee "$FULL_LOG_PATH") 2>&1

begin_phase init "check product-root landing surfaces"
for relative_path in \
  README.md \
  .github/dependabot.yml \
  .github/workflows/deploy-landing.yml \
  scripts/verify-landing-surface.sh \
  scripts/verify-m051-s01.sh \
  mesher/README.md \
  mesher/scripts/verify-maintainer-surface.sh \
  mesher/landing/package.json \
  mesher/landing/package-lock.json \
  mesher/landing/.env.example \
  mesher/landing/lib/external-links.ts; do
  require_file init "$relative_path" "required product-root surface"
done
finish_phase init

begin_phase readme-contract "verify product README wording"
require_contains readme-contract README.md '# Hyperpush Mono'
require_contains readme-contract README.md 'hyperpush-mono/mesher'
require_contains readme-contract README.md 'mesher/landing'
require_contains readme-contract README.md 'bash mesher/scripts/verify-maintainer-surface.sh'
require_contains readme-contract README.md 'bash scripts/verify-landing-surface.sh'
require_contains readme-contract README.md 'https://github.com/hyperpush-org/hyperpush'
require_absent readme-contract README.md 'bash scripts/verify-m051-s01.sh'
require_absent readme-contract README.md 'bash scripts/verify-production-proof-surface.sh'
finish_phase readme-contract

begin_phase landing-links "verify landing links point at the product repo"
require_contains landing-links mesher/landing/lib/external-links.ts 'https://github.com/hyperpush-org/hyperpush'
require_contains landing-links mesher/landing/lib/external-links.ts 'github.com/hyperpush-org/hyperpush'
require_contains landing-links mesher/landing/lib/external-links.ts 'NEXT_PUBLIC_DISCORD_URL'
finish_phase landing-links

begin_phase dependabot-contract "verify product dependabot scope"
require_contains dependabot-contract .github/dependabot.yml 'package-ecosystem: github-actions'
require_contains dependabot-contract .github/dependabot.yml 'package-ecosystem: npm'
require_contains dependabot-contract .github/dependabot.yml 'directory: "/mesher/landing"'
require_absent dependabot-contract .github/dependabot.yml 'directory: "/website"'
require_absent dependabot-contract .github/dependabot.yml 'directory: "/packages-website"'
require_absent dependabot-contract .github/dependabot.yml 'directory: "/tools/editors/vscode-mesh"'
finish_phase dependabot-contract

begin_phase workflow-contract "verify landing workflow uses the product-root paths"
require_contains workflow-contract .github/workflows/deploy-landing.yml 'name: Deploy landing'
require_contains workflow-contract .github/workflows/deploy-landing.yml "mesher/landing/**"
require_contains workflow-contract .github/workflows/deploy-landing.yml 'bash scripts/verify-landing-surface.sh'
require_contains workflow-contract .github/workflows/deploy-landing.yml 'npm --prefix mesher/landing ci'
require_contains workflow-contract .github/workflows/deploy-landing.yml 'npm --prefix mesher/landing run build'
require_contains workflow-contract .github/workflows/deploy-landing.yml 'actions/checkout@v4'
require_contains workflow-contract .github/workflows/deploy-landing.yml 'actions/setup-node@v4'
require_contains workflow-contract .github/workflows/deploy-landing.yml 'mesher/landing/package-lock.json'
require_absent workflow-contract .github/workflows/deploy-landing.yml 'website/'
finish_phase workflow-contract

echo 'verify-landing-surface: ok'
echo "artifacts: ${ARTIFACT_DIR#$ROOT_DIR/}"
