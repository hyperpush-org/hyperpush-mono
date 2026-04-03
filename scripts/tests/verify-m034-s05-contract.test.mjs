import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDir, "..", "..");
const helperPath = path.join(root, "scripts", "lib", "m034_public_surface_contract.py");

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

function cargoVersion(relativePath) {
  const match = read(relativePath).match(/^version = "([^"]+)"/m);
  assert.ok(match, `missing version in ${relativePath}`);
  return match[1];
}

function describeContract() {
  const result = spawnSync("python3", [helperPath, "describe"], {
    cwd: root,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  return JSON.parse(result.stdout);
}

const contract = describeContract();
const meshcVersion = cargoVersion("compiler/meshc/Cargo.toml");
const meshpkgVersion = cargoVersion("compiler/meshpkg/Cargo.toml");
const extensionPackage = JSON.parse(read("tools/editors/vscode-mesh/package.json"));
const extensionVersion = extensionPackage.version;
const binaryTag = `v${meshcVersion}`;
const extensionTag = `ext-v${extensionVersion}`;
const readme = read("README.md");
const tooling = read("website/docs/docs/tooling/index.md");
const releaseWorkflow = read(".github/workflows/release.yml");
const publishExtensionWorkflow = read(".github/workflows/publish-extension.yml");
const s05Verifier = read("scripts/verify-m034-s05.sh");
const workflowVerifier = read("scripts/verify-m034-s05-workflows.sh");
const deployWorkflow = read(".github/workflows/deploy.yml");
const deployServicesWorkflow = read(".github/workflows/deploy-services.yml");

const requiredRunbookStrings = [
  "set -a && source .env && set +a && bash scripts/verify-m034-s05.sh",
  "v<Cargo version>",
  "ext-v<extension version>",
  "deploy.yml",
  "deploy-services.yml",
  "authoritative-verification.yml",
  "release.yml",
  "extension-release-proof.yml",
  "publish-extension.yml",
  "https://meshlang.dev/install.sh",
  "https://meshlang.dev/install.ps1",
  "https://meshlang.dev/docs/getting-started/",
  "https://meshlang.dev/docs/tooling/",
  "https://packages.meshlang.dev/packages/snowdamiz/mesh-registry-proof",
  "https://packages.meshlang.dev/search?q=snowdamiz%2Fmesh-registry-proof",
  "https://api.packages.meshlang.dev/api/v1/packages?search=snowdamiz%2Fmesh-registry-proof",
  ".tmp/m034-s05/verify/candidate-tags.json",
  ".tmp/m034-s05/verify/remote-runs.json",
];

test("candidate tags derive from current version sources and stay independent", () => {
  assert.equal(meshcVersion, meshpkgVersion, "meshc and meshpkg must share one Cargo version");
  assert.match(meshcVersion, /^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$/);
  assert.match(extensionVersion, /^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$/);
  assert.equal(binaryTag, `v${meshpkgVersion}`);
  assert.equal(extensionTag, `ext-v${extensionVersion}`);
  assert.notEqual(binaryTag, extensionTag, "binary and extension candidates must not collapse to one shared tag");
});

test("README and tooling docs publish the canonical S05 runbook contract", () => {
  for (const text of [readme, tooling]) {
    for (const needle of requiredRunbookStrings) {
      assert.ok(text.includes(needle), `expected runbook text to include ${needle}`);
    }
  }
});

test("shared helper owns the retry budget and all call-sites consume it without overrides", () => {
  assert.equal(contract.contractVersion, "m034-s07-public-surface-v1");
  assert.equal(contract.helperPath, "scripts/lib/m034_public_surface_contract.py");
  assert.deepEqual(contract.retryBudget, {
    attempts: 6,
    sleepSeconds: 15,
    fetchTimeoutSeconds: 20,
  });

  assert.match(s05Verifier, /PUBLIC_SURFACE_HELPER="\$ROOT_DIR\/scripts\/lib\/m034_public_surface_contract\.py"/);
  assert.match(s05Verifier, /python3 "\$PUBLIC_SURFACE_HELPER" local-docs --root "\$ROOT_DIR"/);
  assert.match(s05Verifier, /python3 "\$PUBLIC_SURFACE_HELPER" built-docs --root "\$ROOT_DIR" --dist-root "\$ROOT_DIR\/website\/docs\/\.vitepress\/dist"/);
  assert.match(s05Verifier, /python3 "\$PUBLIC_SURFACE_HELPER" public-http --root "\$ROOT_DIR" --artifact-dir "\$VERIFY_ROOT"/);
  assert.match(s05Verifier, /'scripts\/lib\/m034_public_surface_contract\.py'/);

  assert.ok(deployWorkflow.includes(contract.workflowContract.deployDocsCommand));
  assert.ok(deployServicesWorkflow.includes(contract.workflowContract.deployServicesCommand));
  assert.ok(!deployWorkflow.includes("--retry-attempts"), "deploy.yml must use the helper default retry budget");
  assert.ok(!deployServicesWorkflow.includes("--retry-attempts"), "deploy-services.yml must use the helper default retry budget");
  assert.ok(!deployServicesWorkflow.includes("--retry-sleep-seconds"), "deploy-services.yml must use the helper default wait budget");

  assert.ok(workflowVerifier.includes('HELPER_PATH="scripts/lib/m034_public_surface_contract.py"'));
  assert.ok(workflowVerifier.includes(contract.workflowContract.deployDocsCommand));
  assert.ok(workflowVerifier.includes(contract.workflowContract.deployServicesCommand));
});

test("hosted workflows consume the stronger shared contract and reject the old shallow checks", () => {
  assert.match(deployWorkflow, /- name: Verify public docs contract[\s\S]*python3 scripts\/lib\/m034_public_surface_contract\.py built-docs/);
  assert.doesNotMatch(deployWorkflow, /DIST_ROOT="website\/docs\/\.vitepress\/dist"/);
  assert.doesNotMatch(deployWorkflow, /missing exact public proof markers/);

  assert.match(deployServicesWorkflow, /deploy-hyperpush-landing:[\s\S]*name: Deploy hyperpush landing[\s\S]*working-directory: mesher\/landing/);
  assert.match(deployServicesWorkflow, /- name: Verify public surface contract[\s\S]*python3 scripts\/lib\/m034_public_surface_contract\.py public-http/);
  assert.match(deployServicesWorkflow, /health-check:[\s\S]*- name: Checkout[\s\S]*- name: Verify public surface contract[\s\S]*- name: Verify hyperpush landing/);
  for (const legacy of [
    "Check registry package search proof",
    "Check packages detail page proof",
    "Check installer endpoints",
    "Check docs pages",
    "--retry 5 --retry-delay 10 --retry-connrefused",
    "curl --silent --show-error --fail --location",
  ]) {
    assert.ok(!deployServicesWorkflow.includes(legacy), `deploy-services.yml must not keep legacy inline proof: ${legacy}`);
  }

  assert.match(s05Verifier, /'Post-deploy health checks': \[[\s\S]*'Verify public surface contract'[\s\S]*'Verify hyperpush landing'/);
  assert.match(workflowVerifier, /Verify public surface contract step must call the shared helper/);
  assert.match(workflowVerifier, /health-check job must verify the hyperpush landing deployment/);
});

test("remote evidence resolves expected remote refs and records freshness failures", () => {
  assert.match(s05Verifier, /def resolve_expected_ref\(entry, spec, slug\):/);
  assert.match(s05Verifier, /\['git', 'ls-remote', '--quiet', 'origin', \*ref_candidates\]/);
  assert.match(s05Verifier, /'expectedRef': 'refs\/heads\/main'/);
  assert.match(s05Verifier, /'expectedRef': f'refs\/tags\/\{binary_tag\}'/);
  assert.match(s05Verifier, /'expectedPeeledRef': f'refs\/tags\/\{binary_tag\}\^\{\{\}\}'/);
  assert.match(s05Verifier, /'expectedPeeledRef': f'refs\/tags\/\{extension_tag\}\^\{\{\}\}'/);
  assert.match(s05Verifier, /'expectedHeadSha': None/);
  assert.match(s05Verifier, /'observedHeadSha': None/);
  assert.match(s05Verifier, /'freshnessStatus': 'pending'/);
  assert.match(s05Verifier, /entry\['expectedHeadSha'\] = expected_head_sha/);
  assert.match(s05Verifier, /entry\['observedHeadSha'\] = observed_head_sha/);
  assert.match(s05Verifier, /did not match expected \{resolved_ref!r\} sha \{expected_head_sha!r\}/);
  assert.match(s05Verifier, /fail_entry\(entry, results, errors, reason, freshness_reason=reason\)/);
  assert.match(s05Verifier, /'headShaMatchesExpected': None/);
  assert.match(s05Verifier, /entry\['headShaMatchesExpected'\] = True/);
});

test("remote evidence keeps reusable workflow matching and caller-run extension proof semantics", () => {
  const proofWorkflow = read(".github/workflows/extension-release-proof.yml");
  const publishWorkflow = read(".github/workflows/publish-extension.yml");

  assert.match(proofWorkflow, /\non:\n  workflow_call:\n/, "extension proof workflow should stay workflow_call-only");
  assert.match(
    publishWorkflow,
    /uses: \.\/\.github\/workflows\/extension-release-proof\.yml/,
    "publish workflow should call the reusable extension proof workflow",
  );
  assert.match(
    s05Verifier,
    /'workflowFile': 'extension-release-proof\.yml',[\s\S]*'queryWorkflowFile': 'publish-extension\.yml',[\s\S]*'requiredHeadBranch': extension_tag,[\s\S]*'expectedRef': f'refs\/tags\/\{extension_tag\}'[\s\S]*'requiredJobs': \['Verify extension release proof'\],[\s\S]*'successFromJobsOnly': True,/,
    "S05 remote evidence should derive extension proof truth from the publish workflow caller run",
  );
  assert.match(
    s05Verifier,
    /def job_name_matches\(actual_name, required_name\):[\s\S]*reusable_suffix = f' \/ \{required_name\}'/,
    "S05 verifier should tolerate reusable-workflow job name prefixes in gh run view output",
  );
});

test("workflow triggers keep binary and extension tags separate", () => {
  assert.match(releaseWorkflow, /tags:\s*\['v\*'\]/, "release.yml must keep v* tags");
  assert.match(
    publishExtensionWorkflow,
    /tags:\s*[\s\S]*-\s*"ext-v\*"/,
    "publish-extension.yml must keep ext-v* tags"
  );
});
