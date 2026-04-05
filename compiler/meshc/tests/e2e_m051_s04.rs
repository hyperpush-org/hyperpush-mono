mod support;

use std::fs;
use std::path::{Path, PathBuf};

use support::m046_route_free as route_free;

const README_PATH: &str = "README.md";
const GETTING_STARTED_PATH: &str = "website/docs/docs/getting-started/index.md";
const CLUSTERED_EXAMPLE_PATH: &str = "website/docs/docs/getting-started/clustered-example/index.md";
const TOOLING_PATH: &str = "website/docs/docs/tooling/index.md";
const DISTRIBUTED_PATH: &str = "website/docs/docs/distributed/index.md";
const DISTRIBUTED_PROOF_PATH: &str = "website/docs/docs/distributed-proof/index.md";
const PRODUCTION_BACKEND_PROOF_PATH: &str = "website/docs/docs/production-backend-proof/index.md";
const SCAFFOLD_PATH: &str = "compiler/mesh-pkg/src/scaffold.rs";
const CLUSTERING_SKILL_PATH: &str = "tools/skill/mesh/skills/clustering/SKILL.md";
const ONBOARDING_CONTRACT_PATH: &str = "scripts/tests/verify-m049-s04-onboarding-contract.test.mjs";
const SKILL_CONTRACT_PATH: &str = "scripts/tests/verify-m048-s04-skill-contract.test.mjs";
const FIRST_CONTACT_CONTRACT_PATH: &str = "scripts/tests/verify-m050-s02-first-contact-contract.test.mjs";
const SECONDARY_SURFACES_CONTRACT_PATH: &str = "scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs";
const VERIFY_M050_S01_PATH: &str = "scripts/verify-m050-s01.sh";
const VERIFY_M050_S02_PATH: &str = "scripts/verify-m050-s02.sh";
const VERIFY_M050_S03_PATH: &str = "scripts/verify-m050-s03.sh";
const VERIFY_M051_S04_PATH: &str = "scripts/verify-m051-s04.sh";

fn repo_root() -> PathBuf {
    route_free::repo_root()
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m051-s04", test_name)
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
fn m051_s04_public_docs_scaffold_and_skill_stay_examples_first() {
    let artifacts = artifact_dir("public-surface-contract");
    let readme = read_and_archive(README_PATH, &artifacts);
    let getting_started = read_and_archive(GETTING_STARTED_PATH, &artifacts);
    let clustered_example = read_and_archive(CLUSTERED_EXAMPLE_PATH, &artifacts);
    let tooling = read_and_archive(TOOLING_PATH, &artifacts);
    let distributed = read_and_archive(DISTRIBUTED_PATH, &artifacts);
    let distributed_proof = read_and_archive(DISTRIBUTED_PROOF_PATH, &artifacts);
    let production_backend_proof = read_and_archive(PRODUCTION_BACKEND_PROOF_PATH, &artifacts);
    let scaffold = read_and_archive(SCAFFOLD_PATH, &artifacts);
    let clustering_skill = read_and_archive(CLUSTERING_SKILL_PATH, &artifacts);
    let onboarding_contract = read_and_archive(ONBOARDING_CONTRACT_PATH, &artifacts);
    let skill_contract = read_and_archive(SKILL_CONTRACT_PATH, &artifacts);
    let first_contact_contract = read_and_archive(FIRST_CONTACT_CONTRACT_PATH, &artifacts);
    let secondary_surfaces_contract =
        read_and_archive(SECONDARY_SURFACES_CONTRACT_PATH, &artifacts);

    assert_contains_all(
        README_PATH,
        &readme,
        &[
            "https://meshlang.dev/docs/getting-started/clustered-example/",
            "https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md",
            "https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md",
            "https://meshlang.dev/docs/production-backend-proof/",
            "mesher/README.md",
            "Mesh itself rather than just using it",
        ],
    );
    assert_omits_all(
        README_PATH,
        &readme,
        &[
            "https://meshlang.dev/docs/distributed-proof/",
            "reference-backend/README.md",
        ],
    );

    assert_contains_all(
        GETTING_STARTED_PATH,
        &getting_started,
        &[
            "## Choose your next starter",
            "meshc init --clustered hello_cluster",
            "meshc init --template todo-api --db sqlite todo_api",
            "meshc init --template todo-api --db postgres shared_todo",
            "- [Clustered Example](/docs/getting-started/clustered-example/)",
            "- [Production Backend Proof](/docs/production-backend-proof/)",
        ],
    );
    assert_omits(GETTING_STARTED_PATH, &getting_started, "reference-backend/README.md");
    assert_order(
        GETTING_STARTED_PATH,
        &getting_started,
        &[
            "- [Clustered Example](/docs/getting-started/clustered-example/)",
            "- [SQLite Todo starter](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md)",
            "- [PostgreSQL Todo starter](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md)",
            "- [Production Backend Proof](/docs/production-backend-proof/)",
        ],
    );

    assert_contains_all(
        CLUSTERED_EXAMPLE_PATH,
        &clustered_example,
        &[
            "/docs/production-backend-proof/",
            "/docs/distributed-proof/",
            "meshc init --template todo-api --db sqlite my_local_todo",
            "meshc init --template todo-api --db postgres my_shared_todo",
        ],
    );
    assert_contains_all(
        TOOLING_PATH,
        &tooling,
        &[
            "/docs/getting-started/clustered-example/",
            "/docs/production-backend-proof/",
            "meshc init --template todo-api --db sqlite my_local_todo",
            "meshc init --template todo-api --db postgres my_shared_todo",
            "small backend-shaped Mesh project over real stdio JSON-RPC",
            "same-file go-to-definition inside backend-shaped project code",
        ],
    );
    assert_omits_all(
        TOOLING_PATH,
        &tooling,
        &[
            "/docs/distributed-proof/",
            "meshc test reference-backend",
            "meshc fmt --check reference-backend",
            "reference-backend/api/jobs.mpl",
        ],
    );

    assert_contains_all(
        DISTRIBUTED_PATH,
        &distributed,
        &[
            "/docs/distributed-proof/",
            "/docs/production-backend-proof/",
            "mesher/README.md",
            "bash scripts/verify-m051-s01.sh",
            "bash scripts/verify-m051-s02.sh",
        ],
    );
    assert_contains_all(
        DISTRIBUTED_PROOF_PATH,
        &distributed_proof,
        &[
            "This is the only public-secondary docs page that carries the named clustered verifier rails.",
            "/docs/production-backend-proof/",
            "mesher/README.md",
            "bash scripts/verify-m051-s01.sh",
            "bash scripts/verify-m051-s02.sh",
            "keep the deeper backend handoff on Production Backend Proof, Mesher, and the retained backend-only verifier instead of promoting any repo-root runbook as a coequal first-contact clustered starter",
        ],
    );
    assert_omits(
        DISTRIBUTED_PROOF_PATH,
        &distributed_proof,
        "keep `reference-backend` as the deeper backend proof surface rather than a coequal first-contact clustered starter",
    );
    assert_contains_all(
        PRODUCTION_BACKEND_PROOF_PATH,
        &production_backend_proof,
        &[
            "This is the compact public-secondary handoff for Mesh's backend proof story.",
            "mesher/README.md",
            "bash scripts/verify-m051-s01.sh",
            "bash scripts/verify-m051-s02.sh",
            "bash scripts/verify-production-proof-surface.sh",
        ],
    );
    assert_omits(
        PRODUCTION_BACKEND_PROOF_PATH,
        &production_backend_proof,
        "reference-backend/README.md",
    );

    assert_contains_all(
        SCAFFOLD_PATH,
        &scaffold,
        &[
            "Production Backend Proof",
            "mesher/README.md",
            "bash scripts/verify-m051-s01.sh",
            "bash scripts/verify-m051-s02.sh",
        ],
    );

    assert_contains_all(
        CLUSTERING_SKILL_PATH,
        &clustering_skill,
        &[
            "/docs/production-backend-proof/",
            "mesher/README.md",
            "bash scripts/verify-m051-s01.sh",
            "bash scripts/verify-m051-s02.sh",
            "Do not teach `reference-backend/README.md` or fixture/runbook paths as the public next step",
        ],
    );

    for (path_label, source) in [
        (ONBOARDING_CONTRACT_PATH, &onboarding_contract),
        (SKILL_CONTRACT_PATH, &skill_contract),
        (SECONDARY_SURFACES_CONTRACT_PATH, &secondary_surfaces_contract),
    ] {
        assert_contains(path_label, source, "/docs/production-backend-proof/");
        assert_contains(path_label, source, "mesher/README.md");
        assert_contains(path_label, source, "bash scripts/verify-m051-s01.sh");
        assert_contains(path_label, source, "bash scripts/verify-m051-s02.sh");
    }
    assert_contains(
        FIRST_CONTACT_CONTRACT_PATH,
        &first_contact_contract,
        "/docs/production-backend-proof/",
    );
    assert_contains(
        FIRST_CONTACT_CONTRACT_PATH,
        &first_contact_contract,
        "mesher/README.md",
    );
}

#[test]
fn m051_s04_verifier_replays_wrapper_stack_and_retains_bundle_markers() {
    let artifacts = artifact_dir("verifier-contract");
    let verifier = read_and_archive(VERIFY_M051_S04_PATH, &artifacts);
    let verify_m050_s01 = read_and_archive(VERIFY_M050_S01_PATH, &artifacts);
    let verify_m050_s02 = read_and_archive(VERIFY_M050_S02_PATH, &artifacts);
    let verify_m050_s03 = read_and_archive(VERIFY_M050_S03_PATH, &artifacts);

    assert_contains_all(
        VERIFY_M051_S04_PATH,
        &verifier,
        &[
            "ARTIFACT_ROOT=\".tmp/m051-s04\"",
            "status.txt",
            "current-phase.txt",
            "phase-report.txt",
            "full-contract.log",
            "latest-proof-bundle.txt",
            "BUILT_HTML_DIR=\"$ARTIFACT_DIR/built-html\"",
            "RETAINED_PROOF_BUNDLE_DIR=\"$ARTIFACT_DIR/retained-proof-bundle\"",
            "node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs",
            "node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs",
            "bash scripts/verify-m050-s01.sh",
            "bash scripts/verify-m050-s02.sh",
            "bash scripts/verify-m050-s03.sh",
            "cargo test -p meshc --test e2e_m051_s04 -- --nocapture",
            "npm --prefix website run build",
            "retain-m050-s01-verify",
            "retain-m050-s02-verify",
            "retain-m050-s03-verify",
            "retain-m051-s04-artifacts",
            "retain-built-html",
            "built-html",
            "m051-s04-bundle-shape",
            "verify-m051-s04: ok",
        ],
    );
    assert_order(
        VERIFY_M051_S04_PATH,
        &verifier,
        &[
            "run_expect_success onboarding-contract",
            "run_expect_success skill-contract",
            "run_expect_success m050-s01-wrapper",
            "run_expect_success m050-s02-wrapper",
            "run_expect_success m050-s03-wrapper",
            "run_expect_success m051-s04-contract",
            "run_expect_success docs-build",
            "begin_phase retain-m050-s01-verify",
            "begin_phase retain-m050-s02-verify",
            "begin_phase retain-m050-s03-verify",
            "begin_phase retain-m051-s04-artifacts",
            "begin_phase retain-built-html",
            "begin_phase built-html",
            "begin_phase m051-s04-bundle-shape",
        ],
    );

    for (label, source) in [
        (VERIFY_M050_S01_PATH, &verify_m050_s01),
        (VERIFY_M050_S02_PATH, &verify_m050_s02),
        (VERIFY_M050_S03_PATH, &verify_m050_s03),
    ] {
        assert_contains(label, source, "latest-proof-bundle.txt");
        assert_contains(label, source, "built-html");
        assert_contains(label, source, "phase-report.txt");
    }

    let production_backend_proof = read_source(&repo_root().join(PRODUCTION_BACKEND_PROOF_PATH));
    assert_contains(
        PRODUCTION_BACKEND_PROOF_PATH,
        &production_backend_proof,
        "bash scripts/verify-production-proof-surface.sh",
    );
}
