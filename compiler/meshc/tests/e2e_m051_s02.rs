mod support;

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use support::m051_reference_backend as backend;

fn read_source_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn assert_source_contains(path: &Path, needle: &str) {
    let source = read_source_file(path);
    assert!(
        source.contains(needle),
        "expected {} to contain `{}` but it was missing",
        path.display(),
        needle
    );
}

fn assert_source_contains_all(path: &Path, needles: &[&str]) {
    let source = read_source_file(path);
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
    let source = read_source_file(path);
    assert!(
        !source.contains(needle),
        "expected {} to omit `{}` but it was still present",
        path.display(),
        needle
    );
}

fn assert_source_omits_all(path: &Path, needles: &[&str]) {
    let source = read_source_file(path);
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
    let source = read_source_file(path);
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

fn bucket_root() -> PathBuf {
    backend::repo_root().join(".tmp").join("m051-s02")
}

#[test]
fn m051_s02_retained_backend_stage_deploy_bundle_stays_under_m051_bucket() {
    let artifacts = backend::artifact_dir("retained-backend-stage-deploy-bundle");
    let bundle = tempfile::tempdir().expect("failed to create retained backend bundle dir");
    let bundle_dir = bundle.path().to_path_buf();
    backend::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "fixture_root": backend::retained_fixture_root(),
            "checks": [
                "retained stage-deploy builds from scripts/fixtures/backend/reference-backend",
                "bundle pointers and manifests land under .tmp/m051-s02 even though the staged bundle itself stays outside the repo root",
                "the fixture source tree stays source-only after staging"
            ]
        }),
    );

    let output = backend::run_reference_backend_stage_deploy_script(&bundle_dir);
    backend::assert_command_success(
        &output,
        "scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh",
    );
    let combined = backend::command_output_text(&output);

    assert!(
        combined.contains(
            "[stage-deploy] building reference-backend from fixture=scripts/fixtures/backend/reference-backend"
        ),
        "expected retained fixture build marker, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[stage-deploy] staged layout"),
        "expected staged-layout marker, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[stage-deploy] bundle ready dir="),
        "expected bundle-ready marker, got:\n{}",
        combined
    );

    assert!(
        artifacts.starts_with(bucket_root()),
        "expected artifacts under {}, got {}",
        bucket_root().display(),
        artifacts.display()
    );

    let pointer_path = backend::publish_bundle_pointer(&artifacts, &bundle_dir);
    let manifest_path = artifacts.join("bundle-manifest.txt");
    backend::write_bundle_manifest(&bundle_dir, &manifest_path);
    assert!(
        pointer_path.starts_with(bucket_root()),
        "expected bundle pointer under {}, got {}",
        bucket_root().display(),
        pointer_path.display()
    );
    assert!(
        manifest_path.starts_with(bucket_root()),
        "expected bundle manifest under {}, got {}",
        bucket_root().display(),
        manifest_path.display()
    );

    backend::assert_staged_bundle_dir_outside_repo_root(&bundle_dir);
    for relative_path in backend::STAGED_BUNDLE_FILES {
        let staged_path = bundle_dir.join(relative_path);
        if staged_path.extension().is_some_and(|extension| extension == "sql") {
            assert!(
                staged_path.is_file(),
                "expected staged SQL artifact at {}",
                staged_path.display()
            );
        } else {
            backend::assert_is_executable(&staged_path);
        }
    }

    assert!(
        !backend::retained_fixture_root().join("reference-backend").exists(),
        "retained fixture source tree must stay source-only after staging"
    );
    assert_ne!(
        bundle_dir.join("reference-backend"),
        backend::legacy_repo_root_binary_path(),
        "staged bundle must not reuse the repo-root compatibility binary path"
    );
}

#[test]
fn m051_s02_support_helpers_fail_closed_on_wrong_fixture_root() {
    let artifacts = backend::artifact_dir("retained-backend-wrong-root");
    let wrong_root = artifacts.join("wrong-root");
    fs::create_dir_all(&wrong_root)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", wrong_root.display()));

    let error = backend::validate_retained_fixture_root(&wrong_root)
        .expect_err("expected retained fixture helper to fail on a wrong root");
    backend::write_artifact(&artifacts.join("wrong-root.error.txt"), &error);
    assert!(
        error.contains("reference-backend fixture root") && error.contains("missing"),
        "expected explicit wrong-root failure, got: {error}"
    );
}

#[test]
fn m051_s02_repo_root_compat_tree_is_deleted_and_legacy_ignore_rule_is_gone() {
    let legacy_root = backend::repo_root().join("reference-backend");
    assert!(
        !legacy_root.exists(),
        "expected repo-root compatibility tree to be deleted, but {} still exists",
        legacy_root.display()
    );

    let gitignore_path = backend::repo_root().join(".gitignore");
    assert_source_omits(&gitignore_path, "reference-backend/reference-backend");
}

#[test]
fn m051_s02_e2e_reference_backend_rebinds_retained_paths_through_support_module() {
    let target_path = backend::repo_root().join("compiler/meshc/tests/e2e_reference_backend.rs");

    assert_source_contains(&target_path, "mod support;");
    assert_source_contains(
        &target_path,
        "use support::m051_reference_backend as retained_backend;",
    );
    assert_source_contains(&target_path, "retained_backend::build_reference_backend()");
    assert_source_contains(&target_path, "retained_backend::runtime_binary_path()");
    assert_source_contains(
        &target_path,
        "retained_backend::run_reference_backend_migration(database_url, command)",
    );
    assert_source_contains(
        &target_path,
        "retained_backend::run_reference_backend_smoke_script(",
    );
    assert_source_contains(
        &target_path,
        "retained_backend::run_reference_backend_stage_deploy_script(bundle_dir)",
    );

    assert_source_omits(&target_path, ".args([\"build\", \"reference-backend\"])");
    assert_source_omits(
        &target_path,
        ".args([\"migrate\", \"reference-backend\", command])",
    );
    assert_source_omits(&target_path, ".arg(\"reference-backend/scripts/smoke.sh\")");
    assert_source_omits(
        &target_path,
        ".arg(\"reference-backend/scripts/stage-deploy.sh\")",
    );
    assert_source_omits(
        &target_path,
        "repo_root()\n        .join(\"reference-backend\")\n        .join(\"reference-backend\")",
    );
}

#[test]
fn m051_s02_retained_backend_readme_is_the_canonical_maintainer_runbook() {
    let readme_path = backend::retained_readme_path();

    assert_source_contains_all(
        &readme_path,
        &[
            "This README is the canonical maintainer runbook",
            "maintainer-only/internal fixture",
            "sole in-repo backend-only proof surface",
            "repo-root `reference-backend/` compatibility tree was deleted",
            "## Startup contract",
            "## Repo-root maintainer loop",
            "## Staged deploy bundle",
            "## Live runtime smoke",
            "## `/health` recovery interpretation",
            "## Authoritative proof rail",
            "## Post-deletion boundary",
            "cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests",
            "DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate scripts/fixtures/backend/reference-backend status",
            "DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate scripts/fixtures/backend/reference-backend up",
            "DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} PORT=18080 JOB_POLL_MS=500 bash scripts/fixtures/backend/reference-backend/scripts/smoke.sh",
            "tmp_dir=\"$(mktemp -d)\" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh \"$tmp_dir\"",
            "bash \"$bundle_dir/apply-deploy-migrations.sh\" \"$bundle_dir/reference-backend.up.sql\"",
            "BASE_URL=http://127.0.0.1:18080 \\",
            "bash \"$bundle_dir/deploy-smoke.sh\"",
            "restart_count",
            "last_exit_reason",
            "recovered_jobs",
            "last_recovery_at",
            "last_recovery_job_id",
            "last_recovery_count",
            "recovery_active",
            "bash scripts/verify-m051-s02.sh",
            "bash scripts/verify-production-proof-surface.sh",
        ],
    );

    assert_source_order(
        &readme_path,
        &[
            "## Startup contract",
            "## Repo-root maintainer loop",
            "## Staged deploy bundle",
            "## Live runtime smoke",
            "## `/health` recovery interpretation",
            "## Authoritative proof rail",
            "## Post-deletion boundary",
        ],
    );

    assert_source_omits_all(
        &readme_path,
        &[
            "meshlang.dev/install",
            "meshc init --template",
            "website/docs/docs/production-backend-proof",
            "reference-backend/README.md",
            "Do not delete or retarget the repo-root compatibility path in this slice",
            "## Compatibility boundary",
        ],
    );
}

#[test]
fn m051_s02_package_local_scripts_stay_fail_closed_and_internal() {
    let stage_path = backend::retained_stage_deploy_script_path();
    let apply_path = backend::retained_fixture_root().join("scripts/apply-deploy-migrations.sh");
    let deploy_smoke_path = backend::retained_fixture_root().join("scripts/deploy-smoke.sh");
    let smoke_path = backend::retained_smoke_script_path();

    assert_source_contains_all(
        &stage_path,
        &[
            "PACKAGE_REL=\"scripts/fixtures/backend/reference-backend\"",
            "required command missing from PATH: $command_name",
            "cargo run -q -p meshc -- build \"$PACKAGE_REL\" --output \"$TARGET_BINARY\"",
            "require_file \"deploy SQL artifact\" \"$SOURCE_SQL\"",
            "fixture source tree contains an in-place binary",
        ],
    );

    assert_source_contains_all(
        &apply_path,
        &[
            "psql is required but was not found on PATH",
            "DATABASE_URL must be set",
            "MIGRATION_VERSION=\"20260323010000\"",
        ],
    );

    assert_source_contains_all(
        &deploy_smoke_path,
        &[
            "required command missing from PATH: $command_name",
            "PORT must be a positive integer",
            "BASE_URL must start with http:// or https://",
            "/health never became ready at $BASE_URL",
            "job $JOB_ID never reached processed state",
        ],
    );

    assert_source_contains_all(
        &smoke_path,
        &[
            "usage: bash $PACKAGE_REL/scripts/smoke.sh",
            ".tmp/m051-s02/fixture-smoke",
            "required command missing from PATH: $command_name",
            "jobs table is missing; run either: cargo run -q -p meshc -- migrate $PACKAGE_REL up OR bash $PACKAGE_REL/scripts/apply-deploy-migrations.sh $PACKAGE_REL/deploy/reference-backend.up.sql",
            "ensure_source_tree_clean",
        ],
    );
    assert_source_omits(&smoke_path, "reference-backend/reference-backend");
}

#[test]
fn m051_s02_retained_backend_verifier_replays_backend_rails_and_retains_bundle_markers() {
    let verifier_path = backend::repo_root().join("scripts/verify-m051-s02.sh");

    assert_source_contains_all(
        &verifier_path,
        &[
            "m051-s02-contract",
            "m051-s02-package-tests",
            "m051-s02-e2e",
            "m051-s02-delete-surface",
            "m051-s02-db-env-preflight",
            "m051-s02-migration-status-apply",
            "m051-s02-fixture-smoke",
            "m051-s02-deploy-artifact-smoke",
            "m051-s02-worker-crash-recovery",
            "m051-s02-worker-restart-visibility",
            "m051-s02-process-restart-recovery",
            "retain-reference-backend-runtime",
            "retain-fixture-smoke",
            "retain-contract-artifacts",
            "m051-s02-bundle-shape",
            "run_contract_checks \"$ARTIFACT_DIR/m051-s02-contract.log\"",
            "cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests",
            "cargo test -p meshc --test e2e_m051_s02 -- --nocapture",
            "test ! -e reference-backend",
            "cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture",
            "env PORT=\"$SMOKE_PORT\" JOB_POLL_MS=200 bash scripts/fixtures/backend/reference-backend/scripts/smoke.sh",
            "cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture",
            "cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture",
            "cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture",
            "cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture",
            "status.txt",
            "current-phase.txt",
            "phase-report.txt",
            "full-contract.log",
            "latest-proof-bundle.txt",
            "retained-reference-backend-runtime",
            "retained-fixture-smoke",
            "retained-contract-artifacts",
            "fixture.README.md",
            "repo-root.gitignore",
            "scripts.verify-production-proof-surface.sh",
            "verify-m051-s02: ok",
        ],
    );

    assert_source_order(
        &verifier_path,
        &[
            "run_contract_checks \"$ARTIFACT_DIR/m051-s02-contract.log\"",
            "run_expect_success m051-s02-package-tests",
            "run_expect_success m051-s02-e2e",
            "run_expect_success m051-s02-delete-surface",
            "begin_phase m051-s02-db-env-preflight",
            "run_expect_success m051-s02-migration-status-apply",
            "run_expect_success m051-s02-fixture-smoke",
            "run_expect_success m051-s02-deploy-artifact-smoke",
            "run_expect_success m051-s02-worker-crash-recovery",
            "run_expect_success m051-s02-worker-restart-visibility",
            "run_expect_success m051-s02-process-restart-recovery",
            "copy_fixed_dir_or_fail retain-reference-backend-runtime",
            "copy_fixed_dir_or_fail retain-fixture-smoke",
            "copy_new_prefixed_artifacts_or_fail \\",
            "assert_retained_bundle_shape \\",
            "echo \"verify-m051-s02: ok\"",
        ],
    );
}
