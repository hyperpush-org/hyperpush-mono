#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_DIR=".tmp/m034-s02/verify"
REUSABLE_WORKFLOW_PATH=".github/workflows/authoritative-live-proof.yml"
CALLER_WORKFLOW_PATH=".github/workflows/authoritative-verification.yml"
RELEASE_WORKFLOW_PATH=".github/workflows/release.yml"
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

run_caller_contract_check() {
  local phase_name="caller"
  local command_text="ruby caller workflow contract sweep ${CALLER_WORKFLOW_PATH}"
  local log_path="$ARTIFACT_DIR/caller.log"

  echo "==> [${phase_name}] ${command_text}"
  if ! ruby - "$CALLER_WORKFLOW_PATH" >"$log_path" 2>&1 <<'RUBY'
require "yaml"

workflow_path = ARGV.fetch(0)
workflow = YAML.load_file(workflow_path)
raw = File.read(workflow_path)

errors = []

errors << "caller workflow file is missing" unless File.file?(workflow_path)

on_key = if workflow.key?("on")
  "on"
elsif workflow.key?(true)
  true
else
  "on"
end
on_block = workflow[on_key]

errors << "workflow name must stay 'Authoritative verification'" unless workflow["name"] == "Authoritative verification"

unless on_block.is_a?(Hash)
  errors << "workflow must define an on block"
  on_block = {}
end

expected_trigger_keys = ["pull_request", "push", "workflow_dispatch", "schedule"]
unless on_block.keys == expected_trigger_keys
  errors << "workflow triggers must stay pull_request, push, workflow_dispatch, and schedule"
end
if on_block.key?("pull_request_target")
  errors << "workflow must not trigger on pull_request_target"
end

pull_request_block = on_block["pull_request"]
unless pull_request_block.nil? || pull_request_block.is_a?(Hash)
  errors << "pull_request trigger must stay unfiltered or use a mapping"
end

push_block = on_block["push"]
unless push_block.is_a?(Hash) && push_block["branches"] == ["main"]
  errors << "push trigger must stay limited to the main branch"
end

workflow_dispatch_block = on_block["workflow_dispatch"]
unless workflow_dispatch_block.nil? || workflow_dispatch_block.is_a?(Hash)
  errors << "workflow_dispatch trigger must stay present"
end

schedule_block = on_block["schedule"]
unless schedule_block.is_a?(Array) && schedule_block.length == 1
  errors << "workflow must keep exactly one scheduled drift-monitor run"
end
if schedule_block.is_a?(Array) && schedule_block.first.is_a?(Hash)
  unless schedule_block.first["cron"] == "17 4 * * 1"
    errors << "scheduled drift-monitor cadence drifted away from the bounded weekly run"
  end
end

permissions = workflow["permissions"]
unless permissions.is_a?(Hash) && permissions == { "contents" => "read" }
  errors << "caller workflow permissions must stay read-only"
end

concurrency = workflow["concurrency"]
unless concurrency.is_a?(Hash)
  errors << "workflow must declare concurrency"
  concurrency = {}
end
unless concurrency["group"] == "${{ github.workflow }}-${{ github.ref }}"
  errors << "workflow concurrency group must stay keyed to github.workflow and github.ref"
end
unless concurrency["cancel-in-progress"] == false
  errors << "workflow concurrency must serialize same-ref runs without canceling in-flight proofs"
end

jobs = workflow["jobs"]
unless jobs.is_a?(Hash) && jobs.keys == ["live-proof"]
  errors << "caller workflow must define exactly one live-proof job"
end
job = jobs.is_a?(Hash) ? jobs["live-proof"] : nil
if job.is_a?(Hash)
  errors << "caller job name must stay 'Authoritative live proof'" unless job["name"] == "Authoritative live proof"
  unless job["uses"] == "./.github/workflows/authoritative-live-proof.yml"
    errors << "caller must invoke the reusable workflow at ./.github/workflows/authoritative-live-proof.yml"
  end

  trust_guard = job["if"].to_s.gsub(/\s+/, " ").strip
  unless trust_guard.include?("github.event_name != 'pull_request'")
    errors << "caller must allow trusted non-pull_request events through the guard"
  end
  unless trust_guard.include?("github.event.pull_request.head.repo.full_name == github.repository")
    errors << "caller must fail closed for fork PRs using head.repo.full_name == github.repository"
  end

  secrets = job["secrets"]
  unless secrets.is_a?(Hash) && secrets["MESH_PUBLISH_OWNER"] == "${{ secrets.MESH_PUBLISH_OWNER }}"
    errors << "caller must map MESH_PUBLISH_OWNER explicitly into the reusable workflow"
  end
  unless secrets.is_a?(Hash) && secrets["MESH_PUBLISH_TOKEN"] == "${{ secrets.MESH_PUBLISH_TOKEN }}"
    errors << "caller must map MESH_PUBLISH_TOKEN explicitly into the reusable workflow"
  end
end

unless raw.include?("Fork PRs stay on the repo's secret-free build/test lanes.")
  errors << "caller workflow must explain why fork PRs skip the live proof"
end

if raw.include?("bash scripts/verify-m034-s01.sh")
  errors << "caller workflow must not inline the live proof script"
end

if raw.scan("./.github/workflows/authoritative-live-proof.yml").length != 1
  errors << "caller workflow must reference the reusable workflow exactly once"
end

if errors.empty?
  puts "caller workflow contract ok"
else
  raise errors.join("\n")
end
RUBY
  then
    fail_with_log "$phase_name" "$command_text" "caller workflow contract drifted" "$log_path"
  fi
}

run_release_contract_check() {
  local phase_name="release"
  local command_text="ruby release workflow contract sweep ${RELEASE_WORKFLOW_PATH}"
  local log_path="$ARTIFACT_DIR/release.log"

  echo "==> [${phase_name}] ${command_text}"
  if ! ruby - "$RELEASE_WORKFLOW_PATH" >"$log_path" 2>&1 <<'RUBY'
require "yaml"

workflow_path = ARGV.fetch(0)
workflow = YAML.load_file(workflow_path)
raw = File.read(workflow_path)

errors = []

errors << "release workflow file is missing" unless File.file?(workflow_path)

on_key = if workflow.key?("on")
  "on"
elsif workflow.key?(true)
  true
else
  "on"
end
on_block = workflow[on_key]

errors << "workflow name must stay 'Release'" unless workflow["name"] == "Release"

unless on_block.is_a?(Hash)
  errors << "release workflow must define an on block"
  on_block = {}
end

unless on_block.keys == ["push", "pull_request"]
  errors << "release workflow triggers must stay push and pull_request"
end

push_block = on_block["push"]
unless push_block.is_a?(Hash) && push_block["branches"] == ["main"] && push_block["tags"] == ["v*"]
  errors << "release workflow push trigger must keep main branches and v* tags"
end

pull_request_block = on_block["pull_request"]
unless pull_request_block.nil? || pull_request_block.is_a?(Hash)
  errors << "release workflow pull_request trigger must stay unfiltered or use a mapping"
end

permissions = workflow["permissions"]
unless permissions.is_a?(Hash) && permissions == { "contents" => "read" }
  errors << "release workflow permissions must stay read-only outside the publish job"
end

jobs = workflow["jobs"]
unless jobs.is_a?(Hash) && jobs.keys == ["build", "build-meshpkg", "authoritative-live-proof", "release"]
  errors << "release workflow must define build, build-meshpkg, authoritative-live-proof, and release jobs"
end

build = jobs.is_a?(Hash) ? jobs["build"] : nil
if build.is_a?(Hash)
  build_permissions = build["permissions"]
  if build_permissions.is_a?(Hash) && build_permissions["contents"] == "write"
    errors << "build job must not request contents: write"
  end
else
  errors << "release workflow must keep the build job"
end

build_meshpkg = jobs.is_a?(Hash) ? jobs["build-meshpkg"] : nil
if build_meshpkg.is_a?(Hash)
  build_meshpkg_permissions = build_meshpkg["permissions"]
  if build_meshpkg_permissions.is_a?(Hash) && build_meshpkg_permissions["contents"] == "write"
    errors << "build-meshpkg job must not request contents: write"
  end
else
  errors << "release workflow must keep the build-meshpkg job"
end

proof = jobs.is_a?(Hash) ? jobs["authoritative-live-proof"] : nil
if proof.is_a?(Hash)
  errors << "release proof job name must stay 'Authoritative live proof'" unless proof["name"] == "Authoritative live proof"
  unless proof["if"].to_s.include?("startsWith(github.ref, 'refs/tags/v')")
    errors << "release proof job must stay tag-only"
  end
  unless proof["uses"] == "./.github/workflows/authoritative-live-proof.yml"
    errors << "release proof job must invoke the reusable workflow at ./.github/workflows/authoritative-live-proof.yml"
  end

  proof_permissions = proof["permissions"]
  if proof_permissions.is_a?(Hash) && proof_permissions["contents"] == "write"
    errors << "release proof job must not request contents: write"
  end

  proof_secrets = proof["secrets"]
  unless proof_secrets.is_a?(Hash) && proof_secrets["MESH_PUBLISH_OWNER"] == "${{ secrets.MESH_PUBLISH_OWNER }}"
    errors << "release proof job must map MESH_PUBLISH_OWNER explicitly into the reusable workflow"
  end
  unless proof_secrets.is_a?(Hash) && proof_secrets["MESH_PUBLISH_TOKEN"] == "${{ secrets.MESH_PUBLISH_TOKEN }}"
    errors << "release proof job must map MESH_PUBLISH_TOKEN explicitly into the reusable workflow"
  end
else
  errors << "release workflow must define the authoritative-live-proof job"
end

release = jobs.is_a?(Hash) ? jobs["release"] : nil
if release.is_a?(Hash)
  errors << "release job name must stay 'Create Release'" unless release["name"] == "Create Release"
  unless release["if"].to_s.include?("startsWith(github.ref, 'refs/tags/v')")
    errors << "release job must stay tag-only"
  end
  errors << "release job must stay on ubuntu-latest" unless release["runs-on"] == "ubuntu-latest"

  release_needs = release["needs"]
  expected_release_needs = %w[authoritative-live-proof build build-meshpkg]
  unless release_needs.is_a?(Array) && release_needs.sort == expected_release_needs.sort
    errors << "release job must depend on build, build-meshpkg, and authoritative-live-proof"
  end

  release_permissions = release["permissions"]
  unless release_permissions.is_a?(Hash) && release_permissions == { "contents" => "write" }
    errors << "release job must be the only job that requests contents: write"
  end

  steps = release["steps"]
  unless steps.is_a?(Array)
    errors << "release job must define steps"
    steps = []
  end

  find_step = lambda do |name|
    steps.find { |step| step.is_a?(Hash) && step["name"] == name }
  end

  download = find_step.call("Download all artifacts")
  unless download.is_a?(Hash) && download["uses"] == "actions/download-artifact@v4"
    errors << "release job must keep the artifact download step"
  end

  checksum = find_step.call("Generate SHA256SUMS")
  unless checksum.is_a?(Hash) && checksum["run"].to_s.include?("sha256sum *.tar.gz *.zip > SHA256SUMS")
    errors << "release job must keep SHA256SUMS generation"
  end

  publish = find_step.call("Create GitHub Release")
  if publish.is_a?(Hash)
    unless publish["uses"] == "softprops/action-gh-release@v2"
      errors << "release job must keep softprops/action-gh-release@v2"
    end
    publish_with = publish["with"]
    unless publish_with.is_a?(Hash) && publish_with["files"] == "artifacts/*"
      errors << "release job must keep publishing artifacts/*"
    end
  else
    errors << "release job must keep the Create GitHub Release step"
  end
else
  errors << "release workflow must keep the release job"
end

if raw.include?("bash scripts/verify-m034-s01.sh")
  errors << "release workflow must not inline the live proof script"
end

if raw.scan("./.github/workflows/authoritative-live-proof.yml").length != 1
  errors << "release workflow must reference the reusable workflow exactly once"
end

if errors.empty?
  puts "release workflow contract ok"
else
  raise errors.join("\n")
end
RUBY
  then
    fail_with_log "$phase_name" "$command_text" "release workflow contract drifted" "$log_path"
  fi
}

run_full_contract_check() {
  local phase_name="full-contract"
  local command_text="full slice contract sweep"
  local log_path="$ARTIFACT_DIR/full-contract.log"

  echo "==> [${phase_name}] ${command_text}"
  if ! (
    run_reusable_contract_check
    run_caller_contract_check
    run_release_contract_check
  ) >"$log_path" 2>&1; then
    fail_with_log "$phase_name" "$command_text" "slice-level workflow contract drifted" "$log_path"
  fi
}

mode="${1:-all}"
case "$mode" in
  reusable)
    run_reusable_contract_check
    ;;
  caller)
    run_caller_contract_check
    ;;
  release)
    run_release_contract_check
    ;;
  all)
    run_full_contract_check
    ;;
  *)
    echo "unknown mode: $mode" >&2
    echo "usage: bash scripts/verify-m034-s02-workflows.sh [reusable|caller|release|all]" >&2
    exit 1
    ;;
esac

echo "verify-m034-s02-workflows: ok (${mode})"
