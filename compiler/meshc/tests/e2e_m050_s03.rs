mod support;

use serde_json::json;
use std::path::{Path, PathBuf};
use support::m046_route_free as route_free;

const VERIFIER_PATH: &str = "scripts/verify-m050-s03.sh";
const SECONDARY_SURFACES_CONTRACT_COMMAND: &str =
    "node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs";
const M047_S04_DOCS_COMMAND: &str =
    "cargo test -p meshc --test e2e_m047_s04 m047_s04_ -- --nocapture";
const M047_S05_DOCS_COMMAND: &str = "cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture";
const M047_S06_DOCS_COMMAND: &str =
    "cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture";
const PRODUCTION_PROOF_SURFACE_COMMAND: &str =
    "bash scripts/verify-production-proof-surface.sh";
const DOCS_BUILD_COMMAND: &str = "npm --prefix website run build";

fn repo_root() -> PathBuf {
    route_free::repo_root()
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m050-s03", test_name)
}

fn require_includes(errors: &mut Vec<String>, path_label: &str, source: &str, needles: &[&str]) {
    for needle in needles {
        if !source.contains(needle) {
            errors.push(format!("{path_label} missing {needle:?}"));
        }
    }
}

fn require_omits(errors: &mut Vec<String>, path_label: &str, source: &str, needles: &[&str]) {
    for needle in needles {
        if source.contains(needle) {
            errors.push(format!("{path_label} still contains stale text {needle:?}"));
        }
    }
}

fn require_order(errors: &mut Vec<String>, path_label: &str, source: &str, needles: &[&str]) {
    let mut previous_index = None;
    for needle in needles {
        let Some(index) = source.find(needle) else {
            errors.push(format!("{path_label} missing ordered marker {needle:?}"));
            return;
        };
        if let Some(previous_index) = previous_index {
            if index <= previous_index {
                errors.push(format!("{path_label} drifted order around {needle:?}"));
                return;
            }
        }
        previous_index = Some(index);
    }
}

fn load_verifier_source(artifacts: &Path) -> String {
    let contract_artifacts = artifacts.join("contract");
    route_free::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "verifier": VERIFIER_PATH,
            "commands": [
                SECONDARY_SURFACES_CONTRACT_COMMAND,
                M047_S04_DOCS_COMMAND,
                M047_S05_DOCS_COMMAND,
                M047_S06_DOCS_COMMAND,
                PRODUCTION_PROOF_SURFACE_COMMAND,
                DOCS_BUILD_COMMAND,
            ],
            "expected_phase_markers": [
                "init",
                "secondary-surfaces-contract",
                "m047-s04-docs-contract",
                "m047-s05-docs-contract",
                "m047-s06-docs-contract",
                "production-proof-surface",
                "docs-build",
                "retain-built-html",
                "built-html",
                "m050-s03-bundle-shape",
            ],
            "expected_bundle_paths": [
                ".tmp/m050-s03/verify/status.txt",
                ".tmp/m050-s03/verify/current-phase.txt",
                ".tmp/m050-s03/verify/phase-report.txt",
                ".tmp/m050-s03/verify/full-contract.log",
                ".tmp/m050-s03/verify/latest-proof-bundle.txt",
                ".tmp/m050-s03/verify/secondary-surfaces-contract.log",
                ".tmp/m050-s03/verify/m047-s04-docs-contract.log",
                ".tmp/m050-s03/verify/m047-s05-docs-contract.log",
                ".tmp/m050-s03/verify/m047-s06-docs-contract.log",
                ".tmp/m050-s03/verify/production-proof-surface.log",
                ".tmp/m050-s03/verify/docs-build.log",
                ".tmp/m050-s03/verify/built-html/distributed.index.html",
                ".tmp/m050-s03/verify/built-html/distributed-proof.index.html",
                ".tmp/m050-s03/verify/built-html/production-backend-proof.index.html",
                ".tmp/m050-s03/verify/built-html/summary.json",
            ],
        }),
    );

    let verifier_source = route_free::read_and_archive(
        &repo_root().join(VERIFIER_PATH),
        &contract_artifacts.join("verify-m050-s03.sh"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs"),
        &contract_artifacts.join("verify-m050-s03-secondary-surfaces.test.mjs"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("compiler/meshc/tests/e2e_m047_s04.rs"),
        &contract_artifacts.join("e2e_m047_s04.rs"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("compiler/meshc/tests/e2e_m047_s05.rs"),
        &contract_artifacts.join("e2e_m047_s05.rs"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("compiler/meshc/tests/e2e_m047_s06.rs"),
        &contract_artifacts.join("e2e_m047_s06.rs"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("scripts/verify-production-proof-surface.sh"),
        &contract_artifacts.join("verify-production-proof-surface.sh"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("website/docs/docs/distributed/index.md"),
        &contract_artifacts.join("distributed.index.md"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("website/docs/docs/distributed-proof/index.md"),
        &contract_artifacts.join("distributed-proof.index.md"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("website/docs/docs/production-backend-proof/index.md"),
        &contract_artifacts.join("production-backend-proof.index.md"),
    );
    verifier_source
}

fn validate_verifier_contract(source: &str) -> Vec<String> {
    let mut errors = Vec::new();

    require_includes(
        &mut errors,
        VERIFIER_PATH,
        source,
        &[
            "ARTIFACT_ROOT=\".tmp/m050-s03\"",
            "PHASE_REPORT_PATH=\"$ARTIFACT_DIR/phase-report.txt\"",
            "STATUS_PATH=\"$ARTIFACT_DIR/status.txt\"",
            "CURRENT_PHASE_PATH=\"$ARTIFACT_DIR/current-phase.txt\"",
            "LATEST_PROOF_BUNDLE_PATH=\"$ARTIFACT_DIR/latest-proof-bundle.txt\"",
            "BUILT_HTML_DIR=\"$ARTIFACT_DIR/built-html\"",
            "BUILT_HTML_SUMMARY_PATH=\"$BUILT_HTML_DIR/summary.json\"",
            "printf '%s\\n' \"$ARTIFACT_DIR\" >\"$LATEST_PROOF_BUNDLE_PATH\"",
            SECONDARY_SURFACES_CONTRACT_COMMAND,
            M047_S04_DOCS_COMMAND,
            M047_S05_DOCS_COMMAND,
            M047_S06_DOCS_COMMAND,
            PRODUCTION_PROOF_SURFACE_COMMAND,
            DOCS_BUILD_COMMAND,
            "secondary-surfaces-contract",
            "m047-s04-docs-contract",
            "m047-s05-docs-contract",
            "m047-s06-docs-contract",
            "production-proof-surface",
            "docs-build",
            "retain-built-html",
            "built-html",
            "m050-s03-bundle-shape",
            "website/docs/.vitepress/dist/docs/distributed/index.html",
            "website/docs/.vitepress/dist/docs/distributed-proof/index.html",
            "website/docs/.vitepress/dist/docs/production-backend-proof/index.html",
            "$BUILT_HTML_DIR/distributed.index.html",
            "$BUILT_HTML_DIR/distributed-proof.index.html",
            "$BUILT_HTML_DIR/production-backend-proof.index.html",
            "Clustered proof surfaces:",
            "This is the only public-secondary docs page that carries the named clustered verifier rails.",
            "This is the compact public-secondary handoff for Mesh's backend proof story.",
            "Public surfaces and verifier rails",
            "Operator workflow across the public clustered surfaces",
            "Retained backend-only recovery signals",
            "When to use this page vs the generic guides",
            "Failure inspection map",
            "/docs/getting-started/clustered-example/",
            "/docs/distributed-proof/",
            "/docs/production-backend-proof/",
            "/docs/web/",
            "/docs/databases/",
            "/docs/testing/",
            "/docs/concurrency/",
            "/docs/tooling/",
            "https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md",
            "https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md",
            "https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md",
            "bash scripts/verify-m047-s04.sh",
            "bash scripts/verify-m047-s05.sh",
            "bash scripts/verify-m047-s06.sh",
            "cargo test -p meshc --test e2e_m047_s07 -- --nocapture",
            "bash scripts/verify-m043-s04-fly.sh --help",
            "meshc init --clustered",
            "meshc init --template todo-api --db sqlite",
            "meshc init --template todo-api --db postgres",
            "When to use this page vs the generic guides",
            "bash scripts/verify-production-proof-surface.sh",
            "restart_count",
            "recovery_active",
        ],
    );

    require_omits(
        &mut errors,
        VERIFIER_PATH,
        source,
        &[
            "scripts/tests/verify-m050-s02-first-contact-contract.test.mjs",
            "website/docs/.vitepress/dist/docs/getting-started/index.html",
            "website/docs/.vitepress/dist/docs/getting-started/clustered-example/index.html",
            "$BUILT_HTML_DIR/getting-started.index.html",
            "$BUILT_HTML_DIR/clustered-example.index.html",
            "m050-s02-bundle-shape",
            "bash scripts/verify-m050-s02.sh",
        ],
    );

    require_order(
        &mut errors,
        VERIFIER_PATH,
        source,
        &[
            "run_expect_success secondary-surfaces-contract secondary-surfaces-contract no 300",
            "run_expect_success m047-s04-docs-contract m047-s04-docs-contract yes 1800",
            "run_expect_success m047-s05-docs-contract m047-s05-docs-contract yes 1800",
            "run_expect_success m047-s06-docs-contract m047-s06-docs-contract yes 1800",
            "run_expect_success production-proof-surface production-proof-surface no 300",
            "run_expect_success docs-build docs-build no 1800",
        ],
    );

    require_order(
        &mut errors,
        VERIFIER_PATH,
        source,
        &[
            "run_expect_success secondary-surfaces-contract",
            "run_expect_success m047-s04-docs-contract",
            "run_expect_success m047-s05-docs-contract",
            "run_expect_success m047-s06-docs-contract",
            "run_expect_success production-proof-surface",
            "run_expect_success docs-build",
            "begin_phase retain-built-html",
            "begin_phase built-html",
            "begin_phase m050-s03-bundle-shape",
        ],
    );

    errors
}

#[test]
fn m050_s03_verifier_replays_secondary_surface_source_contract_and_docs_truth() {
    let artifacts = artifact_dir("verifier-contract");
    let verifier_source = load_verifier_source(&artifacts);
    let errors = validate_verifier_contract(&verifier_source);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn m050_s03_contract_fails_closed_when_phase_order_drifts() {
    let artifacts = artifact_dir("phase-order-drift");
    let verifier_source = load_verifier_source(&artifacts);

    let mutated = verifier_source
        .replacen(
            SECONDARY_SURFACES_CONTRACT_COMMAND,
            "node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs # moved-later",
            1,
        )
        .replacen(
            M047_S05_DOCS_COMMAND,
            SECONDARY_SURFACES_CONTRACT_COMMAND,
            1,
        )
        .replacen(
            "node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs # moved-later",
            M047_S05_DOCS_COMMAND,
            1,
        )
        .replacen(
            "run_expect_success production-proof-surface",
            "run_expect_success m047-s06-docs-contract",
            1,
        )
        .replacen(
            "run_expect_success m047-s06-docs-contract",
            "run_expect_success production-proof-surface",
            1,
        );

    let errors = validate_verifier_contract(&mutated);
    assert!(
        errors.iter().any(|error| error.contains("drifted order")),
        "{}",
        errors.join("\n")
    );
}

#[test]
fn m050_s03_contract_fails_closed_when_built_html_bundle_markers_disappear() {
    let artifacts = artifact_dir("bundle-shape-drift");
    let verifier_source = load_verifier_source(&artifacts);

    let mutated = verifier_source
        .replace(
            "$BUILT_HTML_DIR/production-backend-proof.index.html",
            "$BUILT_HTML_DIR/production-proof.index.html",
        )
        .replace(
            "begin_phase m050-s03-bundle-shape",
            "begin_phase m050-s03-bundle",
        )
        .replace(
            "printf '%s\\n' \"$ARTIFACT_DIR\" >\"$LATEST_PROOF_BUNDLE_PATH\"",
            "printf '%s\\n' \"$BUILT_HTML_DIR\" >\"$LATEST_PROOF_BUNDLE_PATH\"",
        );

    let errors = validate_verifier_contract(&mutated);
    assert!(
        errors.iter().any(|error| {
            error.contains("$BUILT_HTML_DIR/production-backend-proof.index.html")
                || error.contains("begin_phase m050-s03-bundle-shape")
                || (error.contains("LATEST_PROOF_BUNDLE_PATH") && error.contains("ARTIFACT_DIR"))
        }),
        "{}",
        errors.join("\n")
    );
}
