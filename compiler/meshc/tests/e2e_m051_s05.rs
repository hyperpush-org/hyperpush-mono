mod support;

use std::fs;
use std::path::{Path, PathBuf};

use support::m046_route_free as route_free;

const LEGACY_COMPAT_ROOT_PATH: &str = "reference-backend";
const PROOF_SURFACE_PATH: &str = "scripts/verify-production-proof-surface.sh";
const VERIFY_M051_S01_PATH: &str = "scripts/verify-m051-s01.sh";
const VERIFY_M051_S02_PATH: &str = "scripts/verify-m051-s02.sh";
const VERIFY_M051_S03_PATH: &str = "scripts/verify-m051-s03.sh";
const VERIFY_M051_S04_PATH: &str = "scripts/verify-m051-s04.sh";
const VERIFY_M051_S05_PATH: &str = "scripts/verify-m051-s05.sh";

fn repo_root() -> PathBuf {
    route_free::repo_root()
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m051-s05", test_name)
}

fn read_and_archive(relative: &str, artifacts: &Path) -> String {
    route_free::read_and_archive(
        &repo_root().join(relative),
        &artifacts.join(relative.replace('/', "__")),
    )
}

fn read_source(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn assert_contains(path_label: &str, source: &str, needle: &str) {
    assert!(
        source.contains(needle),
        "expected {path_label} to contain {needle:?}, got:\n{source}"
    );
}

fn assert_omits(path_label: &str, source: &str, needle: &str) {
    assert!(
        !source.contains(needle),
        "expected {path_label} to omit {needle:?}, got:\n{source}"
    );
}

fn assert_contains_all(path_label: &str, source: &str, needles: &[&str]) {
    for needle in needles {
        assert_contains(path_label, source, needle);
    }
}

fn assert_omits_all(path_label: &str, source: &str, needles: &[&str]) {
    for needle in needles {
        assert_omits(path_label, source, needle);
    }
}

fn assert_order(path_label: &str, source: &str, needles: &[&str]) {
    let mut previous_index = None;
    for needle in needles {
        let index = source.find(needle).unwrap_or_else(|| {
            panic!("expected {path_label} to contain ordered marker {needle:?}")
        });
        if let Some(previous_index) = previous_index {
            assert!(
                index > previous_index,
                "expected {path_label} to keep {needle:?} after the prior ordered marker"
            );
        }
        previous_index = Some(index);
    }
}

#[test]
fn m051_s05_post_deletion_contract_keeps_only_top_level_acceptance_surfaces() {
    let artifacts = artifact_dir("post-deletion-contract");
    let proof_surface = read_and_archive(PROOF_SURFACE_PATH, &artifacts);
    let verify_s01 = read_and_archive(VERIFY_M051_S01_PATH, &artifacts);
    let verify_s02 = read_and_archive(VERIFY_M051_S02_PATH, &artifacts);
    let verify_s03 = read_and_archive(VERIFY_M051_S03_PATH, &artifacts);
    let verify_s04 = read_and_archive(VERIFY_M051_S04_PATH, &artifacts);
    let verify_s05 = read_and_archive(VERIFY_M051_S05_PATH, &artifacts);

    assert!(
        !repo_root().join(LEGACY_COMPAT_ROOT_PATH).exists(),
        "expected the repo-root compatibility tree to stay deleted"
    );

    assert_contains_all(
        PROOF_SURFACE_PATH,
        &proof_surface,
        &[
            "#!/usr/bin/env bash",
            "verify-production-proof-surface",
            "Production Backend Proof",
            "mesher/README.md",
            "bash scripts/verify-m051-s01.sh",
            "bash scripts/verify-m051-s02.sh",
        ],
    );
    assert_omits(
        PROOF_SURFACE_PATH,
        &proof_surface,
        "reference-backend/scripts/verify-production-proof-surface.sh",
    );

    for (label, source) in [
        (VERIFY_M051_S01_PATH, &verify_s01),
        (VERIFY_M051_S02_PATH, &verify_s02),
        (VERIFY_M051_S03_PATH, &verify_s03),
        (VERIFY_M051_S04_PATH, &verify_s04),
    ] {
        assert_contains_all(
            label,
            source,
            &[
                "status.txt",
                "current-phase.txt",
                "phase-report.txt",
                "full-contract.log",
                "latest-proof-bundle.txt",
            ],
        );
    }

    assert_contains(VERIFY_M051_S02_PATH, &verify_s02, "test ! -e reference-backend");
    assert_contains(
        VERIFY_M051_S04_PATH,
        &verify_s04,
        "bash scripts/verify-production-proof-surface.sh",
    );

    assert_contains_all(
        VERIFY_M051_S05_PATH,
        &verify_s05,
        &[
            "scripts/verify-production-proof-surface.sh",
            "bash scripts/verify-m051-s01.sh",
            "bash scripts/verify-m051-s02.sh",
            "bash scripts/verify-m051-s03.sh",
            "bash scripts/verify-m051-s04.sh",
            "retained-m051-s01-verify",
            "retained-m051-s01-proof-bundle",
            "retained-m051-s02-verify",
            "retained-m051-s02-proof-bundle",
            "retained-m051-s03-verify",
            "retained-m051-s03-proof-bundle",
            "retained-m051-s04-verify",
            "retained-m051-s04-proof-bundle",
        ],
    );
    assert_omits_all(
        VERIFY_M051_S05_PATH,
        &verify_s05,
        &[
            "reference-backend/scripts/verify-production-proof-surface.sh",
            "source \"$ROOT_DIR/.env\"",
            "echo \"$DATABASE_URL\"",
            "printf \"%s\\n\" \"$DATABASE_URL\"",
        ],
    );
}

#[test]
fn m051_s05_verifier_replays_post_deletion_wrapper_stack_and_retains_bundle_markers() {
    let artifacts = artifact_dir("verifier-contract");
    let verifier = read_and_archive(VERIFY_M051_S05_PATH, &artifacts);

    assert_contains_all(
        VERIFY_M051_S05_PATH,
        &verifier,
        &[
            "ARTIFACT_ROOT=\".tmp/m051-s05\"",
            "status.txt",
            "current-phase.txt",
            "phase-report.txt",
            "full-contract.log",
            "latest-proof-bundle.txt",
            "RETAINED_PROOF_BUNDLE_DIR=\"$ARTIFACT_DIR/retained-proof-bundle\"",
            "RETAINED_M051_S05_ARTIFACTS_MANIFEST_PATH=\"$ARTIFACT_DIR/retained-m051-s05-artifacts.manifest.txt\"",
            "cargo test -p meshc --test e2e_m051_s05 -- --nocapture",
            "DATABASE_URL must be set for scripts/verify-m051-s05.sh",
            "bash scripts/verify-m051-s01.sh",
            "bash scripts/verify-m051-s02.sh",
            "bash scripts/verify-m051-s03.sh",
            "bash scripts/verify-m051-s04.sh",
            "retain-m051-s01-verify",
            "retain-m051-s01-proof-bundle",
            "retain-m051-s02-verify",
            "retain-m051-s02-proof-bundle",
            "retain-m051-s03-verify",
            "retain-m051-s03-proof-bundle",
            "retain-m051-s04-verify",
            "retain-m051-s04-proof-bundle",
            "retain-m051-s05-artifacts",
            "m051-s05-bundle-shape",
            "copy_pointed_bundle_or_fail",
            "verify-m051-s05: ok",
        ],
    );

    assert_order(
        VERIFY_M051_S05_PATH,
        &verifier,
        &[
            "run_expect_success m051-s05-contract",
            "begin_phase m051-s05-db-env-preflight",
            "run_expect_success m051-s01-wrapper",
            "run_expect_success m051-s02-wrapper",
            "run_expect_success m051-s03-wrapper",
            "run_expect_success m051-s04-wrapper",
            "begin_phase retain-m051-s01-verify",
            "begin_phase retain-m051-s01-proof-bundle",
            "begin_phase retain-m051-s02-verify",
            "begin_phase retain-m051-s02-proof-bundle",
            "begin_phase retain-m051-s03-verify",
            "begin_phase retain-m051-s03-proof-bundle",
            "begin_phase retain-m051-s04-verify",
            "begin_phase retain-m051-s04-proof-bundle",
            "begin_phase retain-m051-s05-artifacts",
            "begin_phase m051-s05-bundle-shape",
        ],
    );

    let proof_surface = read_source(&repo_root().join(PROOF_SURFACE_PATH));
    assert_contains(
        PROOF_SURFACE_PATH,
        &proof_surface,
        "bash scripts/verify-production-proof-surface.sh",
    );
}
