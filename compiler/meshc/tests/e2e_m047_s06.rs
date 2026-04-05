mod support;

use std::path::{Path, PathBuf};
use support::m046_route_free as route_free;

const CLUSTERED_SCAFFOLD_COMMAND: &str = "meshc init --clustered";
const SQLITE_STARTER_COMMAND: &str = "meshc init --template todo-api --db sqlite";
const POSTGRES_STARTER_COMMAND: &str = "meshc init --template todo-api --db postgres";
const TODO_POSTGRES_README: &str = "examples/todo-postgres/README.md";
const TODO_SQLITE_README: &str = "examples/todo-sqlite/README.md";
const REFERENCE_BACKEND_RUNBOOK: &str = "reference-backend/README.md";
const MESHER_RUNBOOK: &str = "mesher/README.md";
const STALE_REPO_BLOB_BASE: &str = "https://github.com/hyperpush-org/hyperpush-mono/blob/main/";
const TODO_POSTGRES_README_URL: &str =
    "https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md";
const TODO_SQLITE_README_URL: &str =
    "https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md";
const REFERENCE_BACKEND_RUNBOOK_URL: &str =
    "https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md";
const MESHER_RUNBOOK_URL: &str =
    "https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md";
const VERIFY_M051_S01: &str = "bash scripts/verify-m051-s01.sh";
const VERIFY_M051_S02: &str = "bash scripts/verify-m051-s02.sh";
const DISTRIBUTED_PROOF_SITE_URL: &str = "https://meshlang.dev/docs/distributed-proof/";
const PRODUCTION_BACKEND_PROOF_DOC_LINK: &str = "/docs/production-backend-proof/";
const PRODUCTION_BACKEND_PROOF_SITE_URL: &str =
    "https://meshlang.dev/docs/production-backend-proof/";
const CLUSTERED_EXAMPLE_DOC_LINK: &str = "/docs/getting-started/clustered-example/";
const CUTOVER_RAIL: &str = "bash scripts/verify-m047-s04.sh";
const TODO_SUBRAIL: &str = "bash scripts/verify-m047-s05.sh";
const CLOSEOUT_RAIL: &str = "bash scripts/verify-m047-s06.sh";
const S07_RAIL_COMMAND: &str = "cargo test -p meshc --test e2e_m047_s07 -- --nocapture";
const FLY_READ_ONLY_RAIL: &str = "bash scripts/verify-m043-s04-fly.sh";
const HISTORICAL_M046_CLOSEOUT_ALIAS: &str = "bash scripts/verify-m046-s06.sh";
const HISTORICAL_M046_EQUAL_SURFACE_ALIAS: &str = "bash scripts/verify-m046-s05.sh";
const HISTORICAL_M046_PACKAGE_ALIAS: &str = "bash scripts/verify-m046-s04.sh";
const HISTORICAL_M045_CLOSEOUT_ALIAS: &str = "bash scripts/verify-m045-s05.sh";
const HISTORICAL_M045_ASSEMBLED_ALIAS: &str = "bash scripts/verify-m045-s04.sh";
const HISTORICAL_FAILOVER_SUBRAIL: &str = "bash scripts/verify-m045-s03.sh";
const STALE_CLUSTERED_NON_GOAL: &str = "`HTTP.clustered(...)` is still not shipped.";
const STALE_GENERIC_TODO_COMMAND: &str = "meshc init --template todo-api <name>";
const STALE_SQLITE_CLUSTERED_GUIDANCE: &str = "adding a SQLite HTTP app";
const STALE_SQLITE_CLUSTERED_ROUTES: &str =
    "local SQLite/HTTP routes plus explicit-count `HTTP.clustered(1, ...)`";
const STALE_INTERNAL_FIXTURE_RUNBOOKS: &[&str] =
    &["tiny-cluster/README.md", "cluster-proof/README.md"];

struct ContractSources {
    readme: String,
    getting_started: String,
    tooling: String,
    clustered_example: String,
    distributed_proof: String,
    distributed: String,
    verifier: String,
    docs_config: String,
}

fn repo_root() -> PathBuf {
    route_free::repo_root()
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m047-s06", test_name)
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

fn assert_onboarding_graph_config(path_label: &str, source: &str) {
    assert_contains_all(
        path_label,
        source,
        &[
            "text: 'Getting Started'",
            "text: 'Reference'",
            "text: 'Proof Surfaces'",
            "link: '/docs/getting-started/'",
            "link: '/docs/getting-started/clustered-example/'",
            "link: '/docs/distributed-proof/'",
            "link: '/docs/production-backend-proof/'",
            "includeInFooter: false",
        ],
    );

    let getting_started_index = source
        .find("text: 'Getting Started'")
        .expect("missing Getting Started group");
    let reference_index = source
        .find("text: 'Reference'")
        .expect("missing Reference group");
    let proof_surfaces_index = source
        .find("text: 'Proof Surfaces'")
        .expect("missing Proof Surfaces group");

    assert!(
        getting_started_index < proof_surfaces_index,
        "expected {path_label} to keep Proof Surfaces after Getting Started"
    );
    assert!(
        reference_index < proof_surfaces_index,
        "expected {path_label} to keep Proof Surfaces after Reference so proof pages stay secondary"
    );
    assert!(
        source.matches("includeInFooter: false").count() >= 2,
        "expected {path_label} to opt both proof pages out of the footer chain"
    );
}

fn load_contract_sources(artifacts: &Path) -> ContractSources {
    let contract_artifacts = artifacts.join("contract");
    ContractSources {
        readme: route_free::read_and_archive(
            &repo_root().join("README.md"),
            &contract_artifacts.join("README.md"),
        ),
        getting_started: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/getting-started/index.md"),
            &contract_artifacts.join("getting-started.index.md"),
        ),
        tooling: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/tooling/index.md"),
            &contract_artifacts.join("tooling.index.md"),
        ),
        clustered_example: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/getting-started/clustered-example/index.md"),
            &contract_artifacts.join("clustered-example.index.md"),
        ),
        distributed_proof: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/distributed-proof/index.md"),
            &contract_artifacts.join("distributed-proof.index.md"),
        ),
        distributed: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/distributed/index.md"),
            &contract_artifacts.join("distributed.index.md"),
        ),
        verifier: route_free::read_and_archive(
            &repo_root().join("scripts/verify-m047-s06.sh"),
            &contract_artifacts.join("verify-m047-s06.sh"),
        ),
        docs_config: route_free::read_and_archive(
            &repo_root().join("website/docs/.vitepress/config.mts"),
            &contract_artifacts.join("docs.vitepress.config.mts"),
        ),
    }
}

#[test]
fn m047_s06_public_docs_split_sqlite_local_from_postgres_clustered_starters() {
    let artifacts = artifact_dir("docs-authority-contract");
    let sources = load_contract_sources(&artifacts);

    assert_onboarding_graph_config("website/docs/.vitepress/config.mts", &sources.docs_config);

    assert_contains_all(
        "README.md",
        &sources.readme,
        &[
            CLUSTERED_SCAFFOLD_COMMAND,
            SQLITE_STARTER_COMMAND,
            POSTGRES_STARTER_COMMAND,
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            MESHER_RUNBOOK,
            PRODUCTION_BACKEND_PROOF_SITE_URL,
        ],
    );
    assert_omits_all(
        "README.md",
        &sources.readme,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            STALE_CLUSTERED_NON_GOAL,
            STALE_GENERIC_TODO_COMMAND,
            STALE_SQLITE_CLUSTERED_GUIDANCE,
            STALE_SQLITE_CLUSTERED_ROUTES,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
            STALE_REPO_BLOB_BASE,
        ],
    );

    assert_contains_all(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        &[
            SQLITE_STARTER_COMMAND,
            POSTGRES_STARTER_COMMAND,
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            TODO_POSTGRES_README_URL,
            TODO_SQLITE_README_URL,
            MESHER_RUNBOOK,
            MESHER_RUNBOOK_URL,
            VERIFY_M051_S01,
            VERIFY_M051_S02,
            "This is the only public-secondary docs page that carries the named clustered verifier rails.",
            CLUSTERED_EXAMPLE_DOC_LINK,
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
        ],
    );
    assert_omits(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        STALE_CLUSTERED_NON_GOAL,
    );
    assert_omits_all(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            REFERENCE_BACKEND_RUNBOOK_URL,
            STALE_GENERIC_TODO_COMMAND,
            STALE_SQLITE_CLUSTERED_GUIDANCE,
            STALE_SQLITE_CLUSTERED_ROUTES,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
            STALE_REPO_BLOB_BASE,
        ],
    );

    assert_contains_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            TODO_POSTGRES_README_URL,
            TODO_SQLITE_README_URL,
            CLUSTERED_EXAMPLE_DOC_LINK,
            "/docs/distributed-proof/",
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            "honest local",
            "shared/deployable",
        ],
    );
    assert_omits("website/docs/docs/distributed/index.md", &sources.distributed, STALE_CLUSTERED_NON_GOAL);
    assert_omits_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            REFERENCE_BACKEND_RUNBOOK_URL,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            STALE_GENERIC_TODO_COMMAND,
            STALE_SQLITE_CLUSTERED_GUIDANCE,
            STALE_SQLITE_CLUSTERED_ROUTES,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
            STALE_REPO_BLOB_BASE,
        ],
    );

    assert_contains_all(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        &[
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            TODO_POSTGRES_README_URL,
            TODO_SQLITE_README_URL,
            CLUSTERED_EXAMPLE_DOC_LINK,
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            "honest local",
            "shared/deployable",
        ],
    );
    assert_omits("website/docs/docs/tooling/index.md", &sources.tooling, STALE_CLUSTERED_NON_GOAL);
    assert_omits_all(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            REFERENCE_BACKEND_RUNBOOK_URL,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            STALE_GENERIC_TODO_COMMAND,
            STALE_SQLITE_CLUSTERED_GUIDANCE,
            STALE_SQLITE_CLUSTERED_ROUTES,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
            STALE_REPO_BLOB_BASE,
        ],
    );

    assert_contains_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[MESHER_RUNBOOK, MESHER_RUNBOOK_URL, VERIFY_M051_S01, VERIFY_M051_S02],
    );

    assert_contains_all(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &sources.clustered_example,
        &[
            CLUSTERED_SCAFFOLD_COMMAND,
            SQLITE_STARTER_COMMAND,
            POSTGRES_STARTER_COMMAND,
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            TODO_POSTGRES_README_URL,
            TODO_SQLITE_README_URL,
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            "Node.start_from_env()",
            "@cluster pub fn add() -> Int do",
            "## After the scaffold, pick the follow-on starter",
            "## Need the retained verifier map?",
            "/docs/distributed-proof/",
        ],
    );
    assert_omits(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &sources.clustered_example,
        STALE_CLUSTERED_NON_GOAL,
    );
    assert_omits_all(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &sources.clustered_example,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            REFERENCE_BACKEND_RUNBOOK_URL,
            STALE_GENERIC_TODO_COMMAND,
            STALE_SQLITE_CLUSTERED_GUIDANCE,
            STALE_SQLITE_CLUSTERED_ROUTES,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
            STALE_REPO_BLOB_BASE,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            "execute_declared_work(...)",
            "Work.execute_declared_work",
        ],
    );

    assert_contains(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        PRODUCTION_BACKEND_PROOF_DOC_LINK,
    );
    assert_contains_all(
        "website/docs/docs/getting-started/index.md",
        &sources.getting_started,
        &[
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            CLUSTERED_EXAMPLE_DOC_LINK,
        ],
    );
    assert_omits(
        "website/docs/docs/getting-started/index.md",
        &sources.getting_started,
        REFERENCE_BACKEND_RUNBOOK,
    );

    let getting_started_clustered_example_index = sources
        .getting_started
        .find("- [Clustered Example](/docs/getting-started/clustered-example/)")
        .expect("missing Getting Started Clustered Example next-step marker");
    let getting_started_production_proof_index = sources
        .getting_started
        .find("- [Production Backend Proof](/docs/production-backend-proof/)")
        .expect("missing Getting Started Production Backend Proof next-step marker");
    assert!(
        getting_started_clustered_example_index < getting_started_production_proof_index,
        "expected website/docs/docs/getting-started/index.md to keep Clustered Example ahead of Production Backend Proof in the onboarding path"
    );

    for (path_label, source) in [
        ("README.md", &sources.readme),
        ("website/docs/docs/tooling/index.md", &sources.tooling),
        (
            "website/docs/docs/getting-started/clustered-example/index.md",
            &sources.clustered_example,
        ),
        (
            "website/docs/docs/distributed-proof/index.md",
            &sources.distributed_proof,
        ),
    ] {
        assert_contains(path_label, source, CLUSTERED_SCAFFOLD_COMMAND);
    }

    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        "prev: false",
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        "next: false",
    );
}

#[test]
fn m047_s06_docs_layer_s04_s05_s06_and_s07_truthfully() {
    let artifacts = artifact_dir("rail-layering-contract");
    let sources = load_contract_sources(&artifacts);

    assert_contains_all(
        "README.md",
        &sources.readme,
        &[
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            MESHER_RUNBOOK,
            PRODUCTION_BACKEND_PROOF_SITE_URL,
        ],
    );
    assert_omits_all(
        "README.md",
        &sources.readme,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
            STALE_REPO_BLOB_BASE,
        ],
    );

    assert_contains_all(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        &[
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            TODO_POSTGRES_README_URL,
            TODO_SQLITE_README_URL,
            MESHER_RUNBOOK_URL,
            VERIFY_M051_S01,
            VERIFY_M051_S02,
            "/docs/production-backend-proof/",
        ],
    );
    assert_omits_all(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            REFERENCE_BACKEND_RUNBOOK_URL,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
        ],
    );
    assert_omits(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        STALE_REPO_BLOB_BASE,
    );

    assert_contains_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[
            "/docs/distributed-proof/",
            CLUSTERED_EXAMPLE_DOC_LINK,
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            TODO_POSTGRES_README_URL,
            TODO_SQLITE_README_URL,
        ],
    );
    assert_omits_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            REFERENCE_BACKEND_RUNBOOK_URL,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
        ],
    );
    assert_omits(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        STALE_REPO_BLOB_BASE,
    );

    assert_contains_all(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        &[
            CLUSTERED_EXAMPLE_DOC_LINK,
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            TODO_POSTGRES_README_URL,
            TODO_SQLITE_README_URL,
        ],
    );
    assert_omits_all(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        &[
            "/docs/distributed-proof/",
            REFERENCE_BACKEND_RUNBOOK,
            REFERENCE_BACKEND_RUNBOOK_URL,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
        ],
    );
    assert_omits(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        STALE_REPO_BLOB_BASE,
    );

    assert_contains_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[MESHER_RUNBOOK_URL, VERIFY_M051_S01, VERIFY_M051_S02],
    );

    assert_contains_all(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &sources.clustered_example,
        &[
            "/docs/distributed-proof/",
            TODO_POSTGRES_README_URL,
            TODO_SQLITE_README_URL,
            "/docs/production-backend-proof/",
        ],
    );
    assert_omits_all(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &sources.clustered_example,
        &[
            REFERENCE_BACKEND_RUNBOOK,
            REFERENCE_BACKEND_RUNBOOK_URL,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            CLOSEOUT_RAIL,
            S07_RAIL_COMMAND,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            STALE_INTERNAL_FIXTURE_RUNBOOKS[0],
            STALE_INTERNAL_FIXTURE_RUNBOOKS[1],
            STALE_REPO_BLOB_BASE,
        ],
    );

    for (path_label, source) in [
        (
            "website/docs/docs/getting-started/clustered-example/index.md",
            &sources.clustered_example,
        ),
        (
            "website/docs/docs/distributed/index.md",
            &sources.distributed,
        ),
    ] {
        assert_contains(path_label, source, "/docs/distributed-proof/");
        assert_omits(path_label, source, REFERENCE_BACKEND_RUNBOOK);
    }
    assert_omits(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        "/docs/distributed-proof/",
    );
    assert_omits(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        REFERENCE_BACKEND_RUNBOOK,
    );
}

#[test]
fn m047_s06_verifier_contract_wraps_s05_and_owns_retained_bundle() {
    let artifacts = artifact_dir("verifier-contract");
    let sources = load_contract_sources(&artifacts);
    let verifier = &sources.verifier;

    assert_contains_all(
        "scripts/verify-m047-s06.sh",
        verifier,
        &[
            "ARTIFACT_ROOT=\".tmp/m047-s06\"",
            "RETAINED_M047_S05_VERIFY_DIR=\"$ARTIFACT_DIR/retained-m047-s05-verify\"",
            "RETAINED_M047_S05_BUNDLE_POINTER_PATH=\"$ARTIFACT_DIR/retained-m047-s05-latest-proof-bundle.txt\"",
            "RETAINED_M047_S06_ARTIFACTS_DIR=\"$ARTIFACT_DIR/retained-m047-s06-artifacts\"",
            "RETAINED_PROOF_BUNDLE_DIR=\"$ARTIFACT_DIR/retained-proof-bundle\"",
            "node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs",
            "m050-s01-onboarding-graph",
            "bash scripts/verify-m047-s05.sh",
            "cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture",
            "npm --prefix website run build",
            "assert_file_contains_regex",
            "assert_file_omits_regex",
            "contract-sidebar-proof-surfaces",
            "contract-sidebar-distributed-proof-link",
            "contract-sidebar-production-proof-link",
            "contract-sidebar-proof-footer-opt-out",
            "contract-distributed-proof-canonical-marker",
            "contract-distributed-proof-s05",
            "contract-distributed-proof-s07",
            "contract-distributed-proof-fly",
            "${safe_name}-proof-ledger",
            "${safe_name}-proof-ledger",
            "https://github\\.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README\\.md",
            "https://github\\.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README\\.md",
            "https://github\\.com/snowdamiz/mesh-lang/blob/main/reference-backend/README\\.md",
            "https://github\\.com/hyperpush-org/hyperpush-mono/blob/main/",
            "examples/todo-postgres/README\\.md",
            "examples/todo-sqlite/README\\.md",
            "reference-backend/README\\.md",
            "meshc init --template todo-api --db sqlite",
            "meshc init --template todo-api --db postgres",
            "meshc init --template todo-api(?! --db (sqlite|postgres))",
            "tiny-cluster/README\\.md|cluster-proof/README\\.md",
            "e2e_m047_s07",
            "contract-clustered-example-proof-page",
            "contract-clustered-example-direct-rails",
            "contract-clustered-example-helper-names",
            "contract-clustered-example-stale-repo",
            "status.txt",
            "current-phase.txt",
            "phase-report.txt",
            "full-contract.log",
            "latest-proof-bundle.txt",
            "retained-m047-s05-verify",
            "retained-m047-s05-latest-proof-bundle.txt",
            "retained-m047-s06-artifacts",
            "retained-proof-bundle",
            "contract-guards",
            "m047-s05-replay",
            "retain-m047-s05-verify",
            "m047-s06-e2e",
            "m047-s06-docs-build",
            "m047-s06-artifacts",
            "m047-s06-bundle-shape",
            "verify-m047-s06: ok",
        ],
    );

    for delegated_phase in [
        "m050-s01-onboarding-graph",
        "m047-s04-replay",
        "retain-m047-s04-verify",
        "m047-s05-pkg",
        "m047-s05-tooling",
        "m047-s05-e2e",
        "m047-s05-docs-build",
        "retain-m047-s05-artifacts",
        "m047-s05-bundle-shape",
    ] {
        assert_contains("scripts/verify-m047-s06.sh", verifier, delegated_phase);
    }

    assert_omits(
        "scripts/verify-m047-s06.sh",
        verifier,
        "ARTIFACT_ROOT=\".tmp/m047-s05\"",
    );
    assert_omits(
        "scripts/verify-m047-s06.sh",
        verifier,
        "bash scripts/verify-m047-s04.sh\n",
    );
    assert_omits(
        "scripts/verify-m047-s06.sh",
        verifier,
        "cargo test -p meshc --test e2e_m047_s05 -- --nocapture",
    );
}
