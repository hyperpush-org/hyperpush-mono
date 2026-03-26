#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_DIR=".tmp/m034-s02/verify"
REUSABLE_WORKFLOW_PATH=".github/workflows/authoritative-live-proof.yml"
CALLER_WORKFLOW_PATH=".github/workflows/authoritative-verification.yml"
mkdir -p "$ARTIFACT_DIR"

fail_with_log() {
  local phase_name="$1"
  local command_text="$2"
  local reason="$3"
  local log_path="${4:-}"

  echo "verification drift: ${reason}" >&2
  echo "first failing phase: ${phase_name}" >&2
  echo "failing command: ${command_text}" >&2
  if [[ -n "$log_path" && -f "$log_path" ]]; then
    echo "--- ${log_path} ---" >&2
    sed -n '1,320p' "$log_path" >&2
  fi
  exit 1
}

run_reusable_contract_check() {
  local phase_name="reusable"
  local command_text="ruby reusable workflow contract sweep ${REUSABLE_WORKFLOW_PATH}"
  local log_path="$ARTIFACT_DIR/reusable.log"

  echo "==> [${phase_name}] ${command_text}"
  if ! ruby - "$REUSABLE_WORKFLOW_PATH" "$ROOT_DIR" >"$log_path" 2>&1 <<'RUBY'
require "yaml"

workflow_path = ARGV.fetch(0)
root_dir = ARGV.fetch(1)
workflow = YAML.load_file(workflow_path)
raw = File.read(workflow_path)
script_path = File.join(root_dir, "scripts/verify-m034-s01.sh")

errors = []

unless File.file?(script_path)
  errors << "scripts/verify-m034-s01.sh is missing"
end

on_key = if workflow.key?("on")
  "on"
elsif workflow.key?(true)
  true
else
  "on"
end
on_block = workflow[on_key]
unless on_block.is_a?(Hash) && on_block.keys == ["workflow_call"]
  errors << "workflow must trigger only via workflow_call"
end

call_block = on_block.is_a?(Hash) ? on_block["workflow_call"] : nil
secrets_block = call_block.is_a?(Hash) ? call_block["secrets"] : nil
{
  "MESH_PUBLISH_OWNER" => true,
  "MESH_PUBLISH_TOKEN" => true,
}.each do |secret_name, expected_required|
  secret = secrets_block.is_a?(Hash) ? secrets_block[secret_name] : nil
  unless secret.is_a?(Hash) && secret["required"] == expected_required
    errors << "workflow_call secret #{secret_name} must be declared required"
  end
end

permissions = workflow["permissions"]
unless permissions.is_a?(Hash) && permissions["contents"] == "read"
  errors << "workflow must set permissions.contents to read"
end

jobs = workflow["jobs"]
unless jobs.is_a?(Hash) && jobs.keys == ["live-proof"]
  errors << "workflow must define exactly one live-proof job"
end
job = jobs.is_a?(Hash) ? jobs["live-proof"] : nil
if job.is_a?(Hash)
  errors << "job name must stay 'Authoritative live proof'" unless job["name"] == "Authoritative live proof"
  errors << "job must run on ubuntu-24.04" unless job["runs-on"] == "ubuntu-24.04"
  unless job["timeout-minutes"].is_a?(Integer) && job["timeout-minutes"] >= 30
    errors << "job timeout-minutes must be set for the reusable proof"
  end

  steps = job["steps"]
  unless steps.is_a?(Array)
    errors << "live-proof job must define steps"
    steps = []
  end

  find_step = lambda do |name|
    steps.find { |step| step.is_a?(Hash) && step["name"] == name }
  end

  checkout = find_step.call("Checkout")
  unless checkout.is_a?(Hash) && checkout["uses"] == "actions/checkout@v4"
    errors << "Checkout step must use actions/checkout@v4"
  end

  preflight = find_step.call("Verify live-proof entrypoint")
  unless preflight.is_a?(Hash) && preflight["run"].to_s.include?("test -f scripts/verify-m034-s01.sh")
    errors << "workflow must fail early if scripts/verify-m034-s01.sh is missing"
  end

  cache_llvm = find_step.call("Cache LLVM")
  if cache_llvm.is_a?(Hash)
    unless cache_llvm["uses"] == "actions/cache@v4"
      errors << "Cache LLVM step must use actions/cache@v4"
    end
    unless cache_llvm["id"] == "cache-llvm"
      errors << "Cache LLVM step must keep id cache-llvm"
    end
    cache_with = cache_llvm["with"]
    unless cache_with.is_a?(Hash) && cache_with["path"] == "~/llvm"
      errors << "Cache LLVM step must cache ~/llvm"
    end
    unless cache_with.is_a?(Hash) && cache_with["key"] == "llvm-21.1.8-v3-x86_64-unknown-linux-gnu"
      errors << "Cache LLVM key drifted away from the Linux x86_64 release bootstrap"
    end
  else
    errors << "workflow must cache the Linux LLVM toolchain"
  end

  install_llvm = find_step.call("Install LLVM 21 (Linux x86_64)")
  if install_llvm.is_a?(Hash)
    install_run = install_llvm["run"].to_s
    unless install_llvm["if"].to_s.include?("steps.cache-llvm.outputs.cache-hit != 'true'")
      errors << "LLVM install step must skip when the cache hits"
    end
    unless install_llvm["timeout-minutes"].is_a?(Integer) && install_llvm["timeout-minutes"] >= 5
      errors << "LLVM install step must declare timeout-minutes"
    end
    [
      'LLVM_VERSION="21.1.8"',
      'LLVM_ARCHIVE="LLVM-${LLVM_VERSION}-Linux-X64.tar.xz"',
      'llvmorg-${LLVM_VERSION}',
      'tar xf llvm.tar.xz --strip-components=1 -C "$HOME/llvm"',
    ].each do |needle|
      errors << "LLVM install step missing #{needle}" unless install_run.include?(needle)
    end
  else
    errors << "workflow must install LLVM 21 for Linux x86_64"
  end

  set_prefix = find_step.call("Set LLVM prefix (Linux tarball)")
  unless set_prefix.is_a?(Hash) && set_prefix["run"].to_s.include?('echo "LLVM_SYS_211_PREFIX=$HOME/llvm" >> "$GITHUB_ENV"')
    errors << "workflow must export LLVM_SYS_211_PREFIX from the Linux tarball location"
  end

  install_rust = find_step.call("Install Rust")
  if install_rust.is_a?(Hash)
    unless install_rust["uses"] == "dtolnay/rust-toolchain@stable"
      errors << "Install Rust step must use dtolnay/rust-toolchain@stable"
    end
    unless install_rust["timeout-minutes"].is_a?(Integer) && install_rust["timeout-minutes"] >= 5
      errors << "Install Rust step must declare timeout-minutes"
    end
    targets = install_rust.fetch("with", {})["targets"]
    unless targets == "x86_64-unknown-linux-gnu"
      errors << "Install Rust step must target x86_64-unknown-linux-gnu"
    end
  else
    errors << "workflow must install the Rust toolchain"
  end

  cargo_cache = find_step.call("Cargo cache")
  if cargo_cache.is_a?(Hash)
    unless cargo_cache["uses"] == "Swatinem/rust-cache@v2"
      errors << "Cargo cache step must use Swatinem/rust-cache@v2"
    end
    cache_with = cargo_cache["with"]
    unless cache_with.is_a?(Hash) && cache_with["key"] == "authoritative-live-proof-x86_64-unknown-linux-gnu"
      errors << "Cargo cache key drifted away from the single-host proof contract"
    end
  else
    errors << "workflow must cache Cargo outputs for the proof job"
  end

  proof = find_step.call("Run authoritative live proof")
  if proof.is_a?(Hash)
    unless proof["id"] == "proof"
      errors << "proof step id must stay 'proof'"
    end
    unless proof["run"].to_s.strip == "bash scripts/verify-m034-s01.sh"
      errors << "proof step must shell out to bash scripts/verify-m034-s01.sh unchanged"
    end
    unless proof["timeout-minutes"].is_a?(Integer) && proof["timeout-minutes"] >= 10
      errors << "proof step must declare timeout-minutes"
    end
    env = proof["env"]
    unless env.is_a?(Hash) && env["MESH_PUBLISH_OWNER"] == "${{ secrets.MESH_PUBLISH_OWNER }}"
      errors << "proof step must wire MESH_PUBLISH_OWNER from workflow_call secrets"
    end
    unless env.is_a?(Hash) && env["MESH_PUBLISH_TOKEN"] == "${{ secrets.MESH_PUBLISH_TOKEN }}"
      errors << "proof step must wire MESH_PUBLISH_TOKEN from workflow_call secrets"
    end
  else
    errors << "workflow must contain the authoritative proof step"
  end

  upload = find_step.call("Upload live proof diagnostics")
  if upload.is_a?(Hash)
    unless upload["uses"] == "actions/upload-artifact@v4"
      errors << "diagnostic upload must use actions/upload-artifact@v4"
    end
    upload_if = upload["if"].to_s
    unless upload_if.include?("failure()") && upload_if.include?("steps.proof.outcome == 'failure'")
      errors << "diagnostic upload must run only when the proof step fails"
    end
    unless upload["timeout-minutes"].is_a?(Integer) && upload["timeout-minutes"] >= 1
      errors << "diagnostic upload must declare timeout-minutes"
    end
    upload_with = upload["with"]
    unless upload_with.is_a?(Hash) && upload_with["name"] == "authoritative-live-proof-diagnostics"
      errors << "diagnostic upload artifact name drifted"
    end
    unless upload_with.is_a?(Hash) && upload_with["path"] == ".tmp/m034-s01/verify/**"
      errors << "diagnostic upload must retain .tmp/m034-s01/verify/**"
    end
    unless upload_with.is_a?(Hash) && upload_with["if-no-files-found"] == "error"
      errors << "diagnostic upload must fail when proof artifacts are missing"
    end
  else
    errors << "workflow must upload failure diagnostics"
  end
end

workflow_glob = File.join(root_dir, ".github/workflows/*.yml")
direct_proof_workflows = Dir.glob(workflow_glob).select do |path|
  File.read(path).include?("bash scripts/verify-m034-s01.sh")
end.map { |path| File.expand_path(path) }
expected_direct_workflow = File.expand_path(workflow_path)
unless direct_proof_workflows == [expected_direct_workflow]
  errors << "the reusable workflow must be the only workflow file that directly runs bash scripts/verify-m034-s01.sh"
end

if raw.scan("bash scripts/verify-m034-s01.sh").length != 1
  errors << "workflow must invoke bash scripts/verify-m034-s01.sh exactly once"
end

[
  "meshpkg --json",
  "meshc build",
  "api.packages.meshlang.dev/api/v1/packages",
  "packages.meshlang.dev",
].each do |forbidden|
  if raw.include?(forbidden)
    errors << "workflow must stay thin and not inline live-proof logic (found #{forbidden.inspect})"
  end
end

if errors.empty?
  puts "reusable workflow contract ok"
else
  raise errors.join("\n")
end
RUBY
  then
    fail_with_log "$phase_name" "$command_text" "reusable workflow contract drifted" "$log_path"
  fi
}

run_full_contract_placeholder() {
  local phase_name="full-contract"
  local command_text="full slice contract preflight"
  local log_path="$ARTIFACT_DIR/full-contract.log"

  echo "==> [${phase_name}] ${command_text}"
  if ! (
    run_reusable_contract_check
    if [[ ! -f "$CALLER_WORKFLOW_PATH" ]]; then
      echo "missing ${CALLER_WORKFLOW_PATH}; T02 caller lane not landed yet"
      exit 1
    fi
  ) >"$log_path" 2>&1; then
    fail_with_log "$phase_name" "$command_text" "slice-level workflow contract is not complete yet" "$log_path"
  fi
}

mode="${1:-all}"
case "$mode" in
  reusable)
    run_reusable_contract_check
    ;;
  all)
    run_full_contract_placeholder
    ;;
  *)
    echo "unknown mode: $mode" >&2
    echo "usage: bash scripts/verify-m034-s02-workflows.sh [reusable|all]" >&2
    exit 1
    ;;
esac

echo "verify-m034-s02-workflows: ok (${mode})"
