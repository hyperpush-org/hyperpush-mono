mod support;

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use support::m046_route_free as route_free;
use support::m051_reference_backend as backend;

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m051-s03", test_name)
}

fn write_artifact(path: &Path, contents: impl AsRef<str>) {
    route_free::write_artifact(path, contents.as_ref());
}

fn record_output(artifacts: &Path, stem: &str, output: &Output) {
    write_artifact(
        &artifacts.join(format!("{stem}.stdout.log")),
        String::from_utf8_lossy(&output.stdout),
    );
    write_artifact(
        &artifacts.join(format!("{stem}.stderr.log")),
        String::from_utf8_lossy(&output.stderr),
    );
    write_artifact(
        &artifacts.join(format!("{stem}.combined.log")),
        backend::command_output_text(output),
    );
}

fn assert_source_contains(path: &Path, needle: &str) {
    let source = backend::read_source_file(path);
    assert!(
        source.contains(needle),
        "expected {} to contain `{}` but it was missing",
        path.display(),
        needle
    );
}

fn assert_source_contains_all(path: &Path, needles: &[&str]) {
    let source = backend::read_source_file(path);
    for needle in needles {
        assert!(
            source.contains(needle),
            "expected {} to contain `{}` but it was missing",
            path.display(),
            needle
        );
    }
}

fn assert_source_omits(path: &Path, needle: &str) {
    let source = backend::read_source_file(path);
    assert!(
        !source.contains(needle),
        "expected {} to omit `{}` but it was still present",
        path.display(),
        needle
    );
}

fn assert_source_omits_all(path: &Path, needles: &[&str]) {
    let source = backend::read_source_file(path);
    for needle in needles {
        assert!(
            !source.contains(needle),
            "expected {} to omit `{}` but it was still present",
            path.display(),
            needle
        );
    }
}

fn assert_source_order(path: &Path, needles: &[&str]) {
    let source = backend::read_source_file(path);
    let mut current_index = 0usize;
    for needle in needles {
        let local_index = source[current_index..].find(needle).unwrap_or_else(|| {
            panic!(
                "expected {} to contain `{}` after byte {}",
                path.display(),
                needle,
                current_index
            )
        });
        current_index += local_index + needle.len();
    }
}

#[test]
fn m051_s03_support_helpers_resolve_retained_fixture_paths() {
    let artifacts = artifact_dir("support-helpers");
    let fixture_root = backend::retained_fixture_root();
    let formatter_dir = backend::retained_formatter_dir();
    let health_path = backend::retained_health_path();
    let jobs_path = backend::retained_jobs_path();
    let job_type_path = backend::retained_job_type_path();
    let tests_dir = backend::retained_tests_dir();
    let config_test_path = backend::retained_config_test_path();
    let fixture_test_path = backend::retained_fixture_test_path();
    let legacy_root = backend::repo_root().join(backend::LEGACY_COMPAT_ROOT_RELATIVE);

    write_artifact(
        &artifacts.join("resolved-paths.txt"),
        format!(
            "fixture_root: {}\nformatter_dir: {}\nhealth_path: {}\njobs_path: {}\njob_type_path: {}\ntests_dir: {}\nconfig_test_path: {}\nfixture_test_path: {}\nlegacy_root: {}\n",
            fixture_root.display(),
            formatter_dir.display(),
            health_path.display(),
            jobs_path.display(),
            job_type_path.display(),
            tests_dir.display(),
            config_test_path.display(),
            fixture_test_path.display(),
            legacy_root.display(),
        ),
    );

    assert!(
        fixture_root.ends_with(Path::new(backend::RETAINED_FIXTURE_ROOT_RELATIVE)),
        "expected retained fixture root to end with {}, got {}",
        backend::RETAINED_FIXTURE_ROOT_RELATIVE,
        fixture_root.display()
    );
    assert_ne!(
        fixture_root, legacy_root,
        "retained fixture root must not resolve to the repo-root compatibility copy"
    );

    for path in [
        &fixture_root,
        &formatter_dir,
        &health_path,
        &jobs_path,
        &job_type_path,
        &tests_dir,
        &config_test_path,
        &fixture_test_path,
    ] {
        assert!(path.exists(), "expected {} to exist", path.display());
    }

    assert!(
        formatter_dir.starts_with(&fixture_root),
        "formatter dir should stay within the retained fixture root"
    );
    assert!(
        health_path.starts_with(&formatter_dir) && jobs_path.starts_with(&formatter_dir),
        "bounded formatter paths should stay under the retained formatter dir"
    );
    assert!(
        job_type_path.starts_with(&fixture_root) && tests_dir.starts_with(&fixture_root),
        "all retained helper paths should stay under the retained fixture root"
    );
}

#[test]
fn m051_s03_source_rails_target_retained_fixture_and_omit_legacy_root() {
    let artifacts = artifact_dir("source-rails");
    let e2e_lsp_path = backend::repo_root().join("compiler/meshc/tests/e2e_lsp.rs");
    let tooling_path = backend::repo_root().join("compiler/meshc/tests/tooling_e2e.rs");
    let lsp_analysis_path = backend::repo_root().join("compiler/mesh-lsp/src/analysis.rs");

    write_artifact(
        &artifacts.join("source-targets.txt"),
        format!(
            "e2e_lsp: {}\ntooling_e2e: {}\nmesh_lsp_analysis: {}\n",
            e2e_lsp_path.display(),
            tooling_path.display(),
            lsp_analysis_path.display(),
        ),
    );

    assert_source_contains(
        &e2e_lsp_path,
        "use support::m051_reference_backend as retained_backend;",
    );
    assert_source_contains(&e2e_lsp_path, "retained_backend::retained_fixture_root()");
    assert_source_contains(&e2e_lsp_path, "retained_backend::retained_health_path()");
    assert_source_contains(&e2e_lsp_path, "retained_backend::retained_jobs_path()");
    assert_source_omits(&e2e_lsp_path, "root.join(\"reference-backend\")");

    assert_source_contains(&tooling_path, "mod support;");
    assert_source_contains(
        &tooling_path,
        "use support::m051_reference_backend as retained_backend;",
    );
    assert_source_contains(
        &tooling_path,
        "let target = retained_backend::RETAINED_FIXTURE_ROOT_RELATIVE;",
    );
    assert_source_contains(&tooling_path, "stdout.contains(\"2 passed\")");
    assert_source_omits(&tooling_path, ".args([\"test\", \"reference-backend\"])");
    assert_source_omits(
        &tooling_path,
        ".args([\"test\", \"--coverage\", \"reference-backend\"])",
    );

    assert_source_contains(
        &lsp_analysis_path,
        "repo_root.join(\"scripts/fixtures/backend/reference-backend/api/jobs.mpl\")",
    );
    assert_source_omits(
        &lsp_analysis_path,
        "repo_root.join(\"reference-backend/api/jobs.mpl\")",
    );
}

#[test]
fn m051_s03_meshc_test_retained_fixture_reports_two_passing_files() {
    let artifacts = artifact_dir("meshc-test-retained-fixture");
    backend::ensure_mesh_rt_staticlib();

    let output = Command::new(backend::meshc_bin())
        .current_dir(backend::repo_root())
        .args(["test", backend::RETAINED_FIXTURE_ROOT_RELATIVE])
        .output()
        .expect("failed to run meshc test on the retained reference-backend fixture");
    record_output(&artifacts, "meshc-test", &output);

    assert!(
        output.status.success(),
        "meshc test {} failed:\n{}",
        backend::RETAINED_FIXTURE_ROOT_RELATIVE,
        backend::command_output_text(&output)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("2 passed"),
        "expected meshc test {} to report two passing test files, got:\n{}",
        backend::RETAINED_FIXTURE_ROOT_RELATIVE,
        stdout
    );
}

#[test]
fn m051_s03_formatter_contract_stays_bounded_to_retained_api_subtree() {
    let artifacts = artifact_dir("formatter-contract");
    let e2e_fmt_path = backend::repo_root().join("compiler/meshc/tests/e2e_fmt.rs");
    let mesh_fmt_path = backend::repo_root().join("compiler/mesh-fmt/src/lib.rs");

    assert_source_contains(
        &e2e_fmt_path,
        "use support::m051_reference_backend as retained_backend;",
    );
    assert_source_contains(&e2e_fmt_path, "retained_backend::retained_formatter_dir()");
    assert_source_contains(&e2e_fmt_path, "retained_backend::retained_health_path()");
    assert_source_omits(&e2e_fmt_path, "repo_root.join(\"reference-backend\")");

    assert_source_contains(
        &mesh_fmt_path,
        "/../../scripts/fixtures/backend/reference-backend/api/health.mpl",
    );
    assert_source_contains(
        &mesh_fmt_path,
        "/../../scripts/fixtures/backend/reference-backend/types/job.mpl",
    );
    assert_source_omits(&mesh_fmt_path, "/../../reference-backend/api/health.mpl");
    assert_source_omits(&mesh_fmt_path, "/../../reference-backend/types/job.mpl");
    assert_source_omits(&mesh_fmt_path, "tests/fixture.test.mpl");

    let bounded_output = Command::new(backend::meshc_bin())
        .current_dir(backend::repo_root())
        .args(["fmt", "--check", backend::RETAINED_FORMATTER_DIR_RELATIVE])
        .output()
        .expect("failed to run meshc fmt --check on the bounded retained formatter subtree");
    record_output(&artifacts, "fmt-bounded-api", &bounded_output);

    assert!(
        bounded_output.status.success(),
        "meshc fmt --check {} failed:\n{}",
        backend::RETAINED_FORMATTER_DIR_RELATIVE,
        backend::command_output_text(&bounded_output)
    );
    assert!(
        bounded_output.stdout.is_empty() && bounded_output.stderr.is_empty(),
        "meshc fmt --check {} should stay silent on success, got:\n{}",
        backend::RETAINED_FORMATTER_DIR_RELATIVE,
        backend::command_output_text(&bounded_output)
    );

    let red_output = Command::new(backend::meshc_bin())
        .current_dir(backend::repo_root())
        .args(["fmt", "--check", backend::RETAINED_FIXTURE_FIXTURE_TEST_RELATIVE])
        .output()
        .expect("failed to run meshc fmt --check on the known-red retained fixture test file");
    record_output(&artifacts, "fmt-known-red-fixture-test", &red_output);

    assert_eq!(
        red_output.status.code(),
        Some(1),
        "expected meshc fmt --check {} to fail closed, got:\n{}",
        backend::RETAINED_FIXTURE_FIXTURE_TEST_RELATIVE,
        backend::command_output_text(&red_output)
    );

    let combined = backend::command_output_text(&red_output);
    assert!(
        combined.contains("would reformat:")
            && combined.contains(backend::RETAINED_FIXTURE_FIXTURE_TEST_RELATIVE),
        "expected bounded formatter contract to keep {} out of the green acceptance target, got:\n{}",
        backend::RETAINED_FIXTURE_FIXTURE_TEST_RELATIVE,
        combined
    );
}

#[test]
fn m051_s03_editor_smokes_and_shared_corpus_target_retained_fixture() {
    let artifacts = artifact_dir("editor-and-corpus-targets");
    let vscode_smoke_path =
        backend::repo_root().join("tools/editors/vscode-mesh/src/test/suite/extension.test.ts");
    let neovim_smoke_path = backend::repo_root().join("tools/editors/neovim-mesh/tests/smoke.lua");
    let syntax_corpus_path = backend::repo_root().join("scripts/fixtures/m036-s01-syntax-corpus.json");
    let expected_corpus_fixture_path = format!("{}/main.mpl", backend::RETAINED_FIXTURE_ROOT_RELATIVE);

    assert_source_contains(
        &vscode_smoke_path,
        "const retainedReferenceBackendRoot = path.join(",
    );
    assert_source_contains(
        &vscode_smoke_path,
        "    log(`Using retained backend fixture root ${retainedReferenceBackendRoot}`);",
    );
    assert_source_contains(
        &vscode_smoke_path,
        "const retainedHealthPath = path.join(",
    );
    assert_source_contains(
        &vscode_smoke_path,
        "const retainedJobsPath = path.join(",
    );
    assert_source_contains(
        &vscode_smoke_path,
        "openDocument(retainedHealthPath, \"health\")",
    );
    assert_source_contains(
        &vscode_smoke_path,
        "openDocument(retainedJobsPath, \"jobs\")",
    );
    assert_source_omits(
        &vscode_smoke_path,
        "path.join(repoRoot, \"reference-backend\", \"api\", \"health.mpl\")",
    );
    assert_source_omits(
        &vscode_smoke_path,
        "path.join(repoRoot, \"reference-backend\", \"api\", \"jobs.mpl\")",
    );

    assert_source_contains(
        &neovim_smoke_path,
        "local retained_backend_root = vim.fs.joinpath(repo_root, 'scripts', 'fixtures', 'backend', 'reference-backend')",
    );
    assert_source_contains(
        &neovim_smoke_path,
        "local retained_health_path = vim.fs.joinpath(retained_backend_root, 'api', 'health.mpl')",
    );
    assert_source_contains(
        &neovim_smoke_path,
        "local retained_jobs_path = vim.fs.joinpath(retained_backend_root, 'api', 'jobs.mpl')",
    );
    assert_source_contains(&neovim_smoke_path, "local health_path = retained_health_path");
    assert_source_contains(&neovim_smoke_path, "local jobs_path = retained_jobs_path");
    assert_source_contains(
        &neovim_smoke_path,
        "local expected_root = canonical(retained_backend_root)",
    );
    assert_source_contains(
        &neovim_smoke_path,
        "assert_missing_override_fails(retained_health_path)",
    );
    assert_source_omits(
        &neovim_smoke_path,
        "vim.fs.joinpath(repo_root, 'reference-backend', 'api', 'health.mpl')",
    );
    assert_source_omits(
        &neovim_smoke_path,
        "vim.fs.joinpath(repo_root, 'reference-backend', 'api', 'jobs.mpl')",
    );
    assert_source_omits(
        &neovim_smoke_path,
        "canonical(vim.fs.joinpath(repo_root, 'reference-backend'))",
    );

    let corpus_source =
        fs::read_to_string(&syntax_corpus_path).expect("failed to read shared syntax corpus JSON");
    let corpus: Value =
        serde_json::from_str(&corpus_source).expect("shared syntax corpus should stay valid JSON");
    let contract_version = corpus
        .get("contractVersion")
        .and_then(Value::as_str)
        .expect("shared syntax corpus contractVersion must be present");
    let cases = corpus
        .get("cases")
        .and_then(Value::as_array)
        .expect("shared syntax corpus cases array must be present");
    let retained_backend_case = cases
        .iter()
        .find(|case| case.get("id").and_then(Value::as_str) == Some("reference-backend-http-port-log"))
        .expect("shared syntax corpus must keep the backend-shaped interpolation case");
    let retained_backend_case_path = retained_backend_case
        .get("path")
        .and_then(Value::as_str)
        .expect("backend-shaped interpolation case must keep its path string");

    write_artifact(
        &artifacts.join("editor-and-corpus-targets.txt"),
        format!(
            "vscode_smoke: {}\nneovim_smoke: {}\nsyntax_corpus: {}\ncontract_version: {}\ncase_count: {}\nretained_backend_case_path: {}\n",
            vscode_smoke_path.display(),
            neovim_smoke_path.display(),
            syntax_corpus_path.display(),
            contract_version,
            cases.len(),
            retained_backend_case_path,
        ),
    );

    assert_eq!(
        contract_version, "m036-s01-syntax-corpus-v1",
        "shared syntax corpus contract version drifted"
    );
    assert_eq!(
        cases.len(), 15,
        "shared syntax corpus case count drifted; keep the bounded 15-case contract"
    );
    assert_eq!(
        retained_backend_case_path,
        expected_corpus_fixture_path,
        "shared syntax corpus should point the backend-shaped case at the retained fixture"
    );
    assert!(
        !cases.iter().any(|case| {
            case.get("path").and_then(Value::as_str) == Some("reference-backend/main.mpl")
        }),
        "shared syntax corpus should not drift back to the repo-root compatibility copy"
    );
}

#[test]
fn m051_s03_editor_readmes_keep_backend_proof_generic_and_public() {
    let artifacts = artifact_dir("editor-readmes");
    let vscode_readme_path = backend::repo_root().join("tools/editors/vscode-mesh/README.md");
    let neovim_readme_path = backend::repo_root().join("tools/editors/neovim-mesh/README.md");

    write_artifact(
        &artifacts.join("readme-paths.txt"),
        format!(
            "vscode_readme: {}\nneovim_readme: {}\n",
            vscode_readme_path.display(),
            neovim_readme_path.display(),
        ),
    );

    assert_source_contains_all(
        &vscode_readme_path,
        &[
            "VS Code is a **first-class** editor host in the public Mesh tooling contract.",
            "real stdio JSON-RPC against a small backend-shaped Mesh project",
            "same-file go-to-definition inside backend-shaped project code",
            "manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`",
            "bash scripts/verify-m036-s03.sh",
        ],
    );
    assert_source_omits_all(
        &vscode_readme_path,
        &[
            "reference-backend/",
            "reference-backend/api/jobs.mpl",
            "scripts/fixtures/backend/reference-backend",
            "reference-backend/README.md",
        ],
    );

    assert_source_contains_all(
        &neovim_readme_path,
        &[
            "Together with VS Code, Neovim is a **first-class** editor host in the public Mesh tooling contract:",
            "backend-shaped manifest-rooted fixture",
            "manifest-first override-entry fixture",
            "bash scripts/verify-m036-s02.sh",
            "bash scripts/verify-m036-s03.sh",
        ],
    );
    assert_source_omits_all(
        &neovim_readme_path,
        &[
            "reference-backend/",
            "scripts/fixtures/backend/reference-backend",
        ],
    );
}

#[test]
fn m051_s03_contract_test_fails_closed_on_stale_editor_wording() {
    let contract_test_path = backend::repo_root().join("scripts/tests/verify-m036-s03-contract.test.mjs");

    assert_source_contains_all(
        &contract_test_path,
        &[
            "real stdio JSON-RPC against a small backend-shaped Mesh project",
            "same-file go-to-definition inside backend-shaped project code",
            "backend-shaped manifest-rooted fixture",
            "reference-backend/api/jobs.mpl",
            "scripts/fixtures/backend/reference-backend",
            "contract validation fails closed when the VS Code README reintroduces repo-root backend proof wording",
            "contract validation fails closed when editor READMEs leak the retained fixture path",
        ],
    );
}

#[test]
fn m051_s03_verifier_replays_migrated_rails_and_retains_bundle_markers() {
    let verifier_path = backend::repo_root().join("scripts/verify-m051-s03.sh");

    assert_source_contains_all(
        &verifier_path,
        &[
            "m051-s03-contract",
            "m051-s03-rust-rails",
            "m051-s03-vscode-smoke",
            "m051-s03-neovim-syntax",
            "m051-s03-neovim-lsp",
            "m051-s03-historical-wrapper",
            "retain-m036-s02-syntax",
            "retain-m036-s02-lsp",
            "retain-m036-s02-all",
            "retain-m036-s03-vscode-smoke",
            "retain-m036-s03-verify",
            "retain-m051-s03-artifacts",
            "m051-s03-bundle-shape",
            "node --test scripts/tests/verify-m036-s03-contract.test.mjs",
            "cargo test -p meshc --test e2e_m051_s03 -- --nocapture",
            "npm --prefix tools/editors/vscode-mesh run test:smoke",
            "env \"NEOVIM_BIN=$NEOVIM_BIN_RESOLVED\" bash scripts/verify-m036-s02.sh syntax",
            "env \"NEOVIM_BIN=$NEOVIM_BIN_RESOLVED\" bash scripts/verify-m036-s02.sh lsp",
            "bash scripts/verify-m036-s03.sh",
            "status.txt",
            "current-phase.txt",
            "phase-report.txt",
            "full-contract.log",
            "latest-proof-bundle.txt",
            "verify-m051-s03: ok",
        ],
    );

    assert_source_order(
        &verifier_path,
        &[
            "run_expect_success m051-s03-contract",
            "run_expect_success m051-s03-rust-rails",
            "run_expect_success m051-s03-vscode-smoke",
            "run_expect_success m051-s03-neovim-syntax",
            "run_expect_success m051-s03-neovim-lsp",
            "run_expect_success m051-s03-historical-wrapper",
            "copy_fixed_dir_or_fail retain-m036-s02-syntax",
            "copy_fixed_dir_or_fail retain-m036-s02-lsp",
            "copy_fixed_dir_or_fail retain-m036-s02-all",
            "copy_fixed_dir_or_fail retain-m036-s03-vscode-smoke",
            "copy_fixed_dir_or_fail retain-m036-s03-verify",
            "copy_new_prefixed_artifacts_or_fail \\",
            "assert_retained_bundle_shape \\",
            "echo \"verify-m051-s03: ok\"",
        ],
    );
}
