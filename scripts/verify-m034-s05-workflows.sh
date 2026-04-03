#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_DIR=".tmp/m034-s05/workflows"
DEPLOY_WORKFLOW_PATH=".github/workflows/deploy.yml"
SERVICES_WORKFLOW_PATH=".github/workflows/deploy-services.yml"
HELPER_PATH="scripts/lib/m034_public_surface_contract.py"
DEPLOY_COMMAND='python3 scripts/lib/m034_public_surface_contract.py built-docs --root "$GITHUB_WORKSPACE" --dist-root "$GITHUB_WORKSPACE/website/docs/.vitepress/dist"'
SERVICES_COMMAND='python3 scripts/lib/m034_public_surface_contract.py public-http --root "$GITHUB_WORKSPACE" --artifact-dir "$RUNNER_TEMP/m034-public-surface-contract"'
PHASE_REPORT_PATH="$ARTIFACT_DIR/phase-report.txt"
mkdir -p "$ARTIFACT_DIR"
: >"$PHASE_REPORT_PATH"

record_phase() {
  local phase_name="$1"
  local status="$2"
  printf '%s\t%s\n' "$phase_name" "$status" >>"$PHASE_REPORT_PATH"
}

fail_with_log() {
  local phase_name="$1"
  local command_text="$2"
  local reason="$3"
  local log_path="${4:-}"

  record_phase "$phase_name" "failed"
  echo "verification drift: ${reason}" >&2
  echo "first failing phase: ${phase_name}" >&2
  echo "failing command: ${command_text}" >&2
  echo "artifacts: ${ARTIFACT_DIR}" >&2
  if [[ -n "$log_path" && -f "$log_path" ]]; then
    echo "--- ${log_path} ---" >&2
    sed -n '1,320p' "$log_path" >&2
  fi
  exit 1
}

run_docs_contract_check() {
  local phase_name="docs"
  local command_text="ruby deploy workflow contract sweep ${DEPLOY_WORKFLOW_PATH}"
  local log_path="$ARTIFACT_DIR/docs.log"

  record_phase "$phase_name" "started"
  echo "==> [${phase_name}] ${command_text}"
  if ! ruby - "$DEPLOY_WORKFLOW_PATH" "$HELPER_PATH" "$DEPLOY_COMMAND" >"$log_path" 2>&1 <<'RUBY'
require "yaml"

workflow_path = ARGV.fetch(0)
helper_path = ARGV.fetch(1)
expected_command = ARGV.fetch(2)
workflow = YAML.load_file(workflow_path)
raw = File.read(workflow_path)
errors = []

errors << "deploy workflow file is missing" unless File.file?(workflow_path)
errors << "shared public surface helper is missing" unless File.file?(helper_path)
errors << "workflow name must stay 'Deploy to GitHub Pages'" unless workflow["name"] == "Deploy to GitHub Pages"

on_key = if workflow.key?("on")
  "on"
elsif workflow.key?(true)
  true
else
  "on"
end
on_block = workflow[on_key]
unless on_block.is_a?(Hash)
  errors << "workflow must define an on block"
  on_block = {}
end

unless on_block.keys == ["push", "workflow_dispatch"]
  errors << "deploy workflow triggers must stay push and workflow_dispatch"
end

push_block = on_block["push"]
unless push_block.is_a?(Hash) && push_block["branches"] == ["main"] && push_block["tags"] == ["v*"]
  errors << "deploy workflow push trigger must keep main branches and v* tags"
end

workflow_dispatch_block = on_block["workflow_dispatch"]
unless workflow_dispatch_block.nil? || workflow_dispatch_block.is_a?(Hash)
  errors << "workflow_dispatch trigger must stay present"
end

permissions = workflow["permissions"]
expected_permissions = {
  "contents" => "read",
  "pages" => "write",
  "id-token" => "write",
}
unless permissions.is_a?(Hash) && permissions == expected_permissions
  errors << "deploy workflow permissions must stay contents: read, pages: write, id-token: write"
end

concurrency = workflow["concurrency"]
unless concurrency.is_a?(Hash)
  errors << "deploy workflow must declare concurrency"
  concurrency = {}
end
errors << "deploy workflow concurrency group must stay 'pages'" unless concurrency["group"] == "pages"
errors << "deploy workflow concurrency must keep cancel-in-progress false" unless concurrency["cancel-in-progress"] == false

jobs = workflow["jobs"]
unless jobs.is_a?(Hash) && jobs.keys == ["build", "deploy"]
  errors << "deploy workflow must define exactly build and deploy jobs"
end

build = jobs.is_a?(Hash) ? jobs["build"] : nil
if build.is_a?(Hash)
  errors << "build job must stay on ubuntu-latest" unless build["runs-on"] == "ubuntu-latest"
  unless build["timeout-minutes"].is_a?(Integer) && build["timeout-minutes"] >= 10
    errors << "build job must declare timeout-minutes"
  end

  steps = build["steps"]
  unless steps.is_a?(Array)
    errors << "build job must define steps"
    steps = []
  end

  find_step = lambda do |name|
    steps.find { |step| step.is_a?(Hash) && step["name"] == name }
  end

  checkout = find_step.call("Checkout")
  unless checkout.is_a?(Hash) && checkout["uses"] == "actions/checkout@v4"
    errors << "Checkout step must use actions/checkout@v4"
  end
  unless checkout.is_a?(Hash) && checkout.dig("with", "fetch-depth") == 0
    errors << "Checkout step must keep fetch-depth 0"
  end

  setup_node = find_step.call("Setup Node")
  if setup_node.is_a?(Hash)
    errors << "Setup Node step must use actions/setup-node@v4" unless setup_node["uses"] == "actions/setup-node@v4"
    setup_with = setup_node["with"]
    unless setup_with.is_a?(Hash) && setup_with["node-version"] == 20
      errors << "Setup Node must pin node-version 20"
    end
    unless setup_with.is_a?(Hash) && setup_with["cache"] == "npm"
      errors << "Setup Node must enable npm cache"
    end
    unless setup_with.is_a?(Hash) && setup_with["cache-dependency-path"] == "website/package-lock.json"
      errors << "Setup Node cache path must target website/package-lock.json"
    end
  else
    errors << "build job must set up Node"
  end

  setup_pages = find_step.call("Setup Pages")
  unless setup_pages.is_a?(Hash) && setup_pages["uses"] == "actions/configure-pages@v4"
    errors << "Setup Pages step must use actions/configure-pages@v4"
  end

  install = find_step.call("Install dependencies")
  unless install.is_a?(Hash) && install["run"].to_s.strip == "npm ci" && install["working-directory"] == "website"
    errors << "Install dependencies step must run npm ci from website/"
  end

  build_step = find_step.call("Build with VitePress")
  unless build_step.is_a?(Hash) && build_step["run"].to_s.strip == "npm run build" && build_step["working-directory"] == "website"
    errors << "Build with VitePress step must run npm run build from website/"
  end

  verify = find_step.call("Verify public docs contract")
  if verify.is_a?(Hash)
    errors << "Verify public docs contract step must run under bash" unless verify["shell"] == "bash"
    verify_run = verify["run"].to_s.strip
    errors << "Verify public docs contract step must call the shared helper" unless verify_run.include?(expected_command)
    errors << "Verify public docs contract step must not keep inline DIST_ROOT checks" if verify_run.include?("DIST_ROOT=")
    errors << "Verify public docs contract step must not keep inline proof-marker Python" if verify_run.include?("missing exact public proof markers")
    errors << "Verify public docs contract step must not diff installers inline" if verify_run.include?("diff -u website/docs/public/install.sh")
  else
    errors << "build job must verify the built public docs contract before upload"
  end

  upload = find_step.call("Upload artifact")
  if upload.is_a?(Hash)
    errors << "Upload artifact step must use actions/upload-pages-artifact@v3" unless upload["uses"] == "actions/upload-pages-artifact@v3"
    errors << "Upload artifact step must publish website/docs/.vitepress/dist" unless upload.dig("with", "path") == "website/docs/.vitepress/dist"
  else
    errors << "build job must upload the Pages artifact"
  end
else
  errors << "deploy workflow must keep the build job"
end

deploy = jobs.is_a?(Hash) ? jobs["deploy"] : nil
if deploy.is_a?(Hash)
  errors << "deploy job must depend on build" unless deploy["needs"] == "build"
  errors << "deploy job must stay on ubuntu-latest" unless deploy["runs-on"] == "ubuntu-latest"
  unless deploy["timeout-minutes"].is_a?(Integer) && deploy["timeout-minutes"] >= 10
    errors << "deploy job must declare timeout-minutes"
  end

  environment = deploy["environment"]
  unless environment.is_a?(Hash) && environment["name"] == "github-pages"
    errors << "deploy job environment name must stay github-pages"
  end
  unless environment.is_a?(Hash) && environment["url"] == "${{ steps.deployment.outputs.page_url }}"
    errors << "deploy job environment url must stay wired to steps.deployment.outputs.page_url"
  end

  steps = deploy["steps"]
  unless steps.is_a?(Array) && steps.length == 1
    errors << "deploy job must define exactly one deployment step"
    steps = []
  end
  deployment = steps.first
  if deployment.is_a?(Hash)
    errors << "deploy step name must stay 'Deploy to GitHub Pages'" unless deployment["name"] == "Deploy to GitHub Pages"
    errors << "deploy step id must stay 'deployment'" unless deployment["id"] == "deployment"
    errors << "deploy step must use actions/deploy-pages@v4" unless deployment["uses"] == "actions/deploy-pages@v4"
  else
    errors << "deploy job must keep the actions/deploy-pages step"
  end
else
  errors << "deploy workflow must keep the deploy job"
end

if raw.include?("https://meshlang.dev > /dev/null")
  errors << "deploy workflow must verify exact docs/install surfaces instead of homepage-only curls"
end

if errors.empty?
  puts "deploy workflow contract ok"
else
  raise errors.join("\n")
end
RUBY
  then
    fail_with_log "$phase_name" "$command_text" "deploy workflow contract drifted" "$log_path"
  fi

  record_phase "$phase_name" "passed"
}

run_services_contract_check() {
  local phase_name="services"
  local command_text="ruby deploy-services workflow contract sweep ${SERVICES_WORKFLOW_PATH}"
  local log_path="$ARTIFACT_DIR/services.log"

  record_phase "$phase_name" "started"
  echo "==> [${phase_name}] ${command_text}"
  if ! ruby - "$SERVICES_WORKFLOW_PATH" "$HELPER_PATH" "$SERVICES_COMMAND" >"$log_path" 2>&1 <<'RUBY'
require "yaml"

workflow_path = ARGV.fetch(0)
helper_path = ARGV.fetch(1)
expected_command = ARGV.fetch(2)
workflow = YAML.load_file(workflow_path)
raw = File.read(workflow_path)
errors = []

errors << "deploy-services workflow file is missing" unless File.file?(workflow_path)
errors << "shared public surface helper is missing" unless File.file?(helper_path)
errors << "workflow name must stay 'Deploy Services to Fly.io'" unless workflow["name"] == "Deploy Services to Fly.io"

on_key = if workflow.key?("on")
  "on"
elsif workflow.key?(true)
  true
else
  "on"
end
on_block = workflow[on_key]
unless on_block.is_a?(Hash)
  errors << "workflow must define an on block"
  on_block = {}
end

unless on_block.keys == ["push", "workflow_dispatch"]
  errors << "deploy-services workflow triggers must stay push and workflow_dispatch"
end

push_block = on_block["push"]
unless push_block.is_a?(Hash) && push_block["branches"] == ["main"] && push_block["tags"] == ["v*"]
  errors << "deploy-services workflow push trigger must keep main branches and v* tags"
end

workflow_dispatch_block = on_block["workflow_dispatch"]
unless workflow_dispatch_block.nil? || workflow_dispatch_block.is_a?(Hash)
  errors << "workflow_dispatch trigger must stay present"
end

permissions = workflow["permissions"]
unless permissions.is_a?(Hash) && permissions == { "contents" => "read" }
  errors << "deploy-services workflow permissions must stay read-only"
end

concurrency = workflow["concurrency"]
unless concurrency.is_a?(Hash)
  errors << "deploy-services workflow must declare concurrency"
  concurrency = {}
end
errors << "deploy-services workflow concurrency group must stay deploy-fly-${{ github.ref_name }}" unless concurrency["group"] == "deploy-fly-${{ github.ref_name }}"
errors << "deploy-services workflow concurrency must keep cancel-in-progress false" unless concurrency["cancel-in-progress"] == false

jobs = workflow["jobs"]
unless jobs.is_a?(Hash) && jobs.keys == ["deploy-registry", "deploy-packages-website", "deploy-hyperpush-landing", "health-check"]
  errors << "deploy-services workflow must define deploy-registry, deploy-packages-website, deploy-hyperpush-landing, and health-check jobs"
end

verify_fly_deploy_job = lambda do |job_key, expected_name, expected_dir, deploy_step_name|
  job = jobs.is_a?(Hash) ? jobs[job_key] : nil
  unless job.is_a?(Hash)
    errors << "workflow must keep the #{job_key} job"
    next
  end

  errors << "#{job_key} job name drifted" unless job["name"] == expected_name
  errors << "#{job_key} job must stay on ubuntu-latest" unless job["runs-on"] == "ubuntu-latest"
  unless job["timeout-minutes"].is_a?(Integer) && job["timeout-minutes"] >= 10
    errors << "#{job_key} job must declare timeout-minutes"
  end

  steps = job["steps"]
  unless steps.is_a?(Array)
    errors << "#{job_key} job must define steps"
    next
  end

  find_step = lambda do |name|
    steps.find { |step| step.is_a?(Hash) && step["name"] == name }
  end

  checkout = find_step.call("Checkout")
  unless checkout.is_a?(Hash) && checkout["uses"] == "actions/checkout@v4"
    errors << "#{job_key} Checkout step must use actions/checkout@v4"
  end

  setup_flyctl = find_step.call("Setup flyctl")
  unless setup_flyctl.is_a?(Hash) && setup_flyctl["uses"] == "superfly/flyctl-actions/setup-flyctl@master"
    errors << "#{job_key} Setup flyctl step must use superfly/flyctl-actions/setup-flyctl@master"
  end

  deploy_step = find_step.call(deploy_step_name)
  if deploy_step.is_a?(Hash)
    errors << "#{job_key} deploy step must shell out to flyctl deploy --remote-only" unless deploy_step["run"].to_s.strip == "flyctl deploy --remote-only"
    errors << "#{job_key} deploy step must run from #{expected_dir}/" unless deploy_step["working-directory"] == expected_dir
    errors << "#{job_key} deploy step must read FLY_API_TOKEN from secrets.FLY_API_TOKEN" unless deploy_step.dig("env", "FLY_API_TOKEN") == "${{ secrets.FLY_API_TOKEN }}"
  else
    errors << "#{job_key} job must keep the Fly deploy step"
  end
end

verify_fly_deploy_job.call("deploy-registry", "Deploy mesh-registry", "registry", "Deploy registry to Fly.io")
verify_fly_deploy_job.call("deploy-packages-website", "Deploy mesh-packages website", "packages-website", "Deploy packages website to Fly.io")
verify_fly_deploy_job.call("deploy-hyperpush-landing", "Deploy hyperpush landing", "mesher/landing", "Deploy landing to Fly.io")

health = jobs.is_a?(Hash) ? jobs["health-check"] : nil
if health.is_a?(Hash)
  errors << "health-check job name must stay 'Post-deploy health checks'" unless health["name"] == "Post-deploy health checks"
  expected_needs = %w[deploy-hyperpush-landing deploy-packages-website deploy-registry]
  unless health["needs"].is_a?(Array) && health["needs"].sort == expected_needs.sort
    errors << "health-check job must depend on deploy-registry, deploy-packages-website, and deploy-hyperpush-landing"
  end
  errors << "health-check job must stay on ubuntu-latest" unless health["runs-on"] == "ubuntu-latest"
  unless health["timeout-minutes"].is_a?(Integer) && health["timeout-minutes"] >= 15
    errors << "health-check job must declare timeout-minutes"
  end

  steps = health["steps"]
  unless steps.is_a?(Array)
    errors << "health-check job must define steps"
    steps = []
  end

  find_step = lambda do |name|
    steps.find { |step| step.is_a?(Hash) && step["name"] == name }
  end

  checkout = find_step.call("Checkout")
  unless checkout.is_a?(Hash) && checkout["uses"] == "actions/checkout@v4"
    errors << "health-check job must check out the repo before calling the shared helper"
  end

  verify = find_step.call("Verify public surface contract")
  if verify.is_a?(Hash)
    errors << "Verify public surface contract step must run under bash" unless verify["shell"] == "bash"
    unless verify["timeout-minutes"].is_a?(Integer) && verify["timeout-minutes"] >= 5
      errors << "Verify public surface contract step must declare timeout-minutes"
    end
    verify_run = verify["run"].to_s.strip
    errors << "Verify public surface contract step must call the shared helper" unless verify_run.include?(expected_command)
  else
    errors << "health-check job must call the shared public surface contract helper"
  end

  verify_landing = find_step.call("Verify hyperpush landing")
  if verify_landing.is_a?(Hash)
    errors << "Verify hyperpush landing step must run under bash" unless verify_landing["shell"] == "bash"
    unless verify_landing["timeout-minutes"].is_a?(Integer) && verify_landing["timeout-minutes"] >= 3
      errors << "Verify hyperpush landing step must declare timeout-minutes"
    end
    verify_landing_run = verify_landing["run"].to_s
    errors << "Verify hyperpush landing step must probe the Fly hostname" unless verify_landing_run.include?("https://hyperpush-landing.fly.dev")
    errors << "Verify hyperpush landing step must assert the hyperpush marker" unless verify_landing_run.include?("grep -q \"hyperpush\"")
  else
    errors << "health-check job must verify the hyperpush landing deployment"
  end
else
  errors << "deploy-services workflow must keep the health-check job"
end

legacy_needles = [
  "Check registry package search proof",
  "Check packages detail page proof",
  "Check installer endpoints",
  "Check docs pages",
  "--retry 5 --retry-delay 10 --retry-connrefused",
  "curl --silent --show-error --fail --location",
]
legacy_needles.each do |needle|
  if raw.include?(needle)
    errors << "deploy-services workflow must not keep pre-S07 inline public proof logic (found #{needle.inspect})"
  end
end

if errors.empty?
  puts "deploy-services workflow contract ok"
else
  raise errors.join("\n")
end
RUBY
  then
    fail_with_log "$phase_name" "$command_text" "deploy-services workflow contract drifted" "$log_path"
  fi

  record_phase "$phase_name" "passed"
}

run_full_contract_check() {
  local phase_name="full-contract"
  local command_text="full deploy workflow contract sweep"
  local log_path="$ARTIFACT_DIR/full-contract.log"

  record_phase "$phase_name" "started"
  echo "==> [${phase_name}] ${command_text}"
  if ! (
    run_docs_contract_check
    run_services_contract_check
  ) >"$log_path" 2>&1; then
    fail_with_log "$phase_name" "$command_text" "workflow contract drifted" "$log_path"
  fi

  record_phase "$phase_name" "passed"
}

mode="${1:-all}"
case "$mode" in
  docs)
    run_docs_contract_check
    ;;
  services)
    run_services_contract_check
    ;;
  all)
    run_full_contract_check
    ;;
  *)
    echo "unknown mode: $mode" >&2
    echo "usage: bash scripts/verify-m034-s05-workflows.sh [docs|services|all]" >&2
    exit 1
    ;;
esac

echo "verify-m034-s05-workflows: ok (${mode})"
