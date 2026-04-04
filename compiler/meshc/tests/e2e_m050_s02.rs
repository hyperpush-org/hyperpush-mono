mod support;

use serde_json::json;
use std::path::{Path, PathBuf};
use support::m046_route_free as route_free;

const VERIFIER_PATH: &str = "scripts/verify-m050-s02.sh";
const FIRST_CONTACT_CONTRACT_COMMAND: &str =
    "node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs";
const M047_S05_DOCS_COMMAND: &str = "cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture";
const M047_S06_DOCS_COMMAND: &str =
    "cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture";
const M048_TOOLING_CONTRACT_COMMAND: &str =
    "node --test scripts/tests/verify-m048-s05-contract.test.mjs";
const M036_TOOLING_CONTRACT_COMMAND: &str =
    "node --test scripts/tests/verify-m036-s03-contract.test.mjs";
const DOCS_BUILD_COMMAND: &str = "npm --prefix website run build";

fn repo_root() -> PathBuf {
    route_free::repo_root()
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m050-s02", test_name)
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
                FIRST_CONTACT_CONTRACT_COMMAND,
                M047_S05_DOCS_COMMAND,
                M047_S06_DOCS_COMMAND,
                M048_TOOLING_CONTRACT_COMMAND,
                M036_TOOLING_CONTRACT_COMMAND,
                DOCS_BUILD_COMMAND,
            ],
            "expected_phase_markers": [
                "init",
                "first-contact-contract",
                "m047-s05-docs-contract",
                "m047-s06-docs-contract",
                "m048-s05-tooling-contract",
                "m036-s03-tooling-contract",
                "docs-build",
                "retain-built-html",
                "built-html",
                "m050-s02-bundle-shape",
            ],
            "expected_bundle_paths": [
                ".tmp/m050-s02/verify/status.txt",
                ".tmp/m050-s02/verify/current-phase.txt",
                ".tmp/m050-s02/verify/phase-report.txt",
                ".tmp/m050-s02/verify/full-contract.log",
                ".tmp/m050-s02/verify/latest-proof-bundle.txt",
                ".tmp/m050-s02/verify/first-contact-contract.log",
                ".tmp/m050-s02/verify/m047-s05-docs-contract.log",
                ".tmp/m050-s02/verify/m047-s06-docs-contract.log",
                ".tmp/m050-s02/verify/m048-s05-tooling-contract.log",
                ".tmp/m050-s02/verify/m036-s03-tooling-contract.log",
                ".tmp/m050-s02/verify/docs-build.log",
                ".tmp/m050-s02/verify/built-html/getting-started.index.html",
                ".tmp/m050-s02/verify/built-html/clustered-example.index.html",
                ".tmp/m050-s02/verify/built-html/tooling.index.html",
                ".tmp/m050-s02/verify/built-html/summary.json",
            ],
        }),
    );

    let verifier_source = route_free::read_and_archive(
        &repo_root().join(VERIFIER_PATH),
        &contract_artifacts.join("verify-m050-s02.sh"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("scripts/tests/verify-m050-s02-first-contact-contract.test.mjs"),
        &contract_artifacts.join("verify-m050-s02-first-contact-contract.test.mjs"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("scripts/tests/verify-m048-s05-contract.test.mjs"),
        &contract_artifacts.join("verify-m048-s05-contract.test.mjs"),
    );
    let _ = route_free::read_and_archive(
        &repo_root().join("scripts/tests/verify-m036-s03-contract.test.mjs"),
        &contract_artifacts.join("verify-m036-s03-contract.test.mjs"),
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
        &repo_root().join("website/docs/docs/tooling/index.md"),
        &contract_artifacts.join("tooling.index.md"),
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
            "ARTIFACT_ROOT=\".tmp/m050-s02\"",
            "PHASE_REPORT_PATH=\"$ARTIFACT_DIR/phase-report.txt\"",
            "STATUS_PATH=\"$ARTIFACT_DIR/status.txt\"",
            "CURRENT_PHASE_PATH=\"$ARTIFACT_DIR/current-phase.txt\"",
            "LATEST_PROOF_BUNDLE_PATH=\"$ARTIFACT_DIR/latest-proof-bundle.txt\"",
            "BUILT_HTML_DIR=\"$ARTIFACT_DIR/built-html\"",
            "BUILT_HTML_SUMMARY_PATH=\"$BUILT_HTML_DIR/summary.json\"",
            "printf '%s\\n' \"$ARTIFACT_DIR\" >\"$LATEST_PROOF_BUNDLE_PATH\"",
            FIRST_CONTACT_CONTRACT_COMMAND,
            M047_S05_DOCS_COMMAND,
            M047_S06_DOCS_COMMAND,
            M048_TOOLING_CONTRACT_COMMAND,
            M036_TOOLING_CONTRACT_COMMAND,
            DOCS_BUILD_COMMAND,
            "first-contact-contract",
            "m047-s05-docs-contract",
            "m047-s06-docs-contract",
            "m048-s05-tooling-contract",
            "m036-s03-tooling-contract",
            "docs-build",
            "retain-built-html",
            "built-html",
            "m050-s02-bundle-shape",
            "website/docs/.vitepress/dist/docs/getting-started/index.html",
            "website/docs/.vitepress/dist/docs/getting-started/clustered-example/index.html",
            "website/docs/.vitepress/dist/docs/tooling/index.html",
            "$BUILT_HTML_DIR/getting-started.index.html",
            "$BUILT_HTML_DIR/clustered-example.index.html",
            "$BUILT_HTML_DIR/tooling.index.html",
            "CURRENT_REPO_BLOB_BASE = 'https://github.com/snowdamiz/mesh-lang/blob/main/'",
            "STALE_REPO_BLOB_BASE = 'https://github.com/hyperpush-org/hyperpush-mono/blob/main/'",
            "Choose your next starter",
            "After the scaffold, pick the follow-on starter",
            "Assembled first-contact docs verifier",
            "bash scripts/verify-m050-s02.sh",
            "Release Assembly Runbook",
            "Assembled scaffold/example verifier",
            "bash scripts/verify-m049-s05.sh",
        ],
    );

    require_omits(
        &mut errors,
        VERIFIER_PATH,
        source,
        &[
            "bash scripts/verify-m050-s01.sh",
            "cargo test -p meshc --test e2e_m049_s01 -- --nocapture",
            "cargo test -p meshc --test e2e_m049_s02 -- --nocapture",
            "cargo test -p meshc --test e2e_m049_s03 -- --nocapture",
            "bash scripts/verify-m039-s01.sh",
            "bash scripts/verify-m045-s02.sh",
            ".tmp/m049-s01/local-postgres/connection.env",
            "reference-backend/scripts/verify-production-proof-surface.sh",
            "website/docs/.vitepress/dist/docs/distributed-proof/index.html",
            "website/docs/.vitepress/dist/docs/production-backend-proof/index.html",
        ],
    );

    require_order(
        &mut errors,
        VERIFIER_PATH,
        source,
        &[
            FIRST_CONTACT_CONTRACT_COMMAND,
            M047_S05_DOCS_COMMAND,
            M047_S06_DOCS_COMMAND,
            M048_TOOLING_CONTRACT_COMMAND,
            M036_TOOLING_CONTRACT_COMMAND,
            DOCS_BUILD_COMMAND,
        ],
    );

    require_order(
        &mut errors,
        VERIFIER_PATH,
        source,
        &[
            "run_expect_success first-contact-contract",
            "run_expect_success m047-s05-docs-contract",
            "run_expect_success m047-s06-docs-contract",
            "run_expect_success m048-s05-tooling-contract",
            "run_expect_success m036-s03-tooling-contract",
            "run_expect_success docs-build",
            "begin_phase retain-built-html",
            "begin_phase built-html",
            "begin_phase m050-s02-bundle-shape",
        ],
    );

    errors
}

#[test]
fn m050_s02_verifier_replays_the_first_contact_docs_and_tooling_stack() {
    let artifacts = artifact_dir("verifier-contract");
    let verifier_source = load_verifier_source(&artifacts);
    let errors = validate_verifier_contract(&verifier_source);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn m050_s02_contract_fails_closed_when_phase_order_drifts() {
    let artifacts = artifact_dir("phase-order-drift");
    let verifier_source = load_verifier_source(&artifacts);

    let mutated = verifier_source
        .replacen(
            FIRST_CONTACT_CONTRACT_COMMAND,
            "node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs # moved-later",
            1,
        )
        .replacen(M036_TOOLING_CONTRACT_COMMAND, FIRST_CONTACT_CONTRACT_COMMAND, 1)
        .replacen(
            "node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs # moved-later",
            M036_TOOLING_CONTRACT_COMMAND,
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
fn m050_s02_contract_fails_closed_when_built_html_bundle_markers_disappear() {
    let artifacts = artifact_dir("bundle-shape-drift");
    let verifier_source = load_verifier_source(&artifacts);

    let mutated = verifier_source
        .replace(
            "$BUILT_HTML_DIR/tooling.index.html",
            "$BUILT_HTML_DIR/tooling.html",
        )
        .replace(
            "begin_phase m050-s02-bundle-shape",
            "begin_phase m050-s02-bundle",
        )
        .replace(
            "printf '%s\\n' \"$ARTIFACT_DIR\" >\"$LATEST_PROOF_BUNDLE_PATH\"",
            "printf '%s\\n' \"$BUILT_HTML_DIR\" >\"$LATEST_PROOF_BUNDLE_PATH\"",
        );

    let errors = validate_verifier_contract(&mutated);
    assert!(
        errors.iter().any(|error| {
            error.contains("$BUILT_HTML_DIR/tooling.index.html")
                || error.contains("begin_phase m050-s02-bundle-shape")
                || (error.contains("LATEST_PROOF_BUNDLE_PATH") && error.contains("ARTIFACT_DIR"))
        }),
        "{}",
        errors.join("\n")
    );
}
