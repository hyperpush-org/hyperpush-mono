mod support;

use serde_json::json;
use std::path::{Path, PathBuf};
use support::m046_route_free as route_free;

const TODO_POSTGRES_README: &str = "examples/todo-postgres/README.md";
const TODO_SQLITE_README: &str = "examples/todo-sqlite/README.md";
const MESHER_RUNBOOK: &str = "mesher/README.md";
const VERIFY_M051_S01: &str = "bash scripts/verify-m051-s01.sh";
const VERIFY_M051_S02: &str = "bash scripts/verify-m051-s02.sh";
const DISTRIBUTED_PROOF_SITE_URL: &str = "https://meshlang.dev/docs/distributed-proof/";
const CLUSTERED_EXAMPLE_DOC_LINK: &str = "/docs/getting-started/clustered-example/";
const PRODUCTION_BACKEND_PROOF_DOC_LINK: &str = "/docs/production-backend-proof/";
const PRODUCTION_BACKEND_PROOF_SITE_URL: &str =
    "https://meshlang.dev/docs/production-backend-proof/";
const CUTOVER_RAIL: &str = "bash scripts/verify-m047-s04.sh";
const TODO_SUBRAIL: &str = "bash scripts/verify-m047-s05.sh";
const S07_RAIL_COMMAND: &str = "cargo test -p meshc --test e2e_m047_s07 -- --nocapture";
const CLOSEOUT_RAIL: &str = "bash scripts/verify-m047-s06.sh";
const FLY_READ_ONLY_RAIL: &str = "bash scripts/verify-m043-s04-fly.sh";
const HISTORICAL_M046_CLOSEOUT_ALIAS: &str = "bash scripts/verify-m046-s06.sh";
const HISTORICAL_M046_EQUAL_SURFACE_ALIAS: &str = "bash scripts/verify-m046-s05.sh";
const HISTORICAL_M046_PACKAGE_ALIAS: &str = "bash scripts/verify-m046-s04.sh";
const HISTORICAL_M045_CLOSEOUT_ALIAS: &str = "bash scripts/verify-m045-s05.sh";
const HISTORICAL_M045_ASSEMBLED_ALIAS: &str = "bash scripts/verify-m045-s04.sh";
const HISTORICAL_FAILOVER_SUBRAIL: &str = "bash scripts/verify-m045-s03.sh";
const STALE_M046_AUTHORITY: &str =
    "`bash scripts/verify-m046-s06.sh` — the authoritative assembled closeout rail";

struct ContractSources {
    verify_script: String,
    verify_m045_s04: String,
    verify_m045_s05: String,
    verify_m046_s04: String,
    verify_m046_s05: String,
    verify_m046_s06: String,
    docs_config: String,
    readme: String,
    distributed_proof: String,
    distributed: String,
    tooling: String,
    clustered_example: String,
    todo_postgres_readme: String,
    todo_sqlite_readme: String,
    tiny_cluster_fixture_readme: String,
    cluster_proof_fixture_readme: String,
}

fn repo_root() -> PathBuf {
    route_free::repo_root()
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m047-s04", test_name)
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

fn assert_clustered_surface_omits_routeful_drift(path_label: &str, source: &str) {
    assert_omits_all(
        path_label,
        source,
        &[
            "Continuity.submit_declared_work",
            "/work/:request_key",
            "Timer.sleep(5000)",
        ],
    );
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
        verify_script: route_free::read_and_archive(
            &repo_root().join("scripts/verify-m047-s04.sh"),
            &contract_artifacts.join("verify-m047-s04.sh"),
        ),
        verify_m045_s04: route_free::read_and_archive(
            &repo_root().join("scripts/verify-m045-s04.sh"),
            &contract_artifacts.join("verify-m045-s04.sh"),
        ),
        verify_m045_s05: route_free::read_and_archive(
            &repo_root().join("scripts/verify-m045-s05.sh"),
            &contract_artifacts.join("verify-m045-s05.sh"),
        ),
        verify_m046_s04: route_free::read_and_archive(
            &repo_root().join("scripts/verify-m046-s04.sh"),
            &contract_artifacts.join("verify-m046-s04.sh"),
        ),
        verify_m046_s05: route_free::read_and_archive(
            &repo_root().join("scripts/verify-m046-s05.sh"),
            &contract_artifacts.join("verify-m046-s05.sh"),
        ),
        verify_m046_s06: route_free::read_and_archive(
            &repo_root().join("scripts/verify-m046-s06.sh"),
            &contract_artifacts.join("verify-m046-s06.sh"),
        ),
        docs_config: route_free::read_and_archive(
            &repo_root().join("website/docs/.vitepress/config.mts"),
            &contract_artifacts.join("docs.vitepress.config.mts"),
        ),
        readme: route_free::read_and_archive(
            &repo_root().join("README.md"),
            &contract_artifacts.join("README.md"),
        ),
        distributed_proof: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/distributed-proof/index.md"),
            &contract_artifacts.join("distributed-proof.index.md"),
        ),
        distributed: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/distributed/index.md"),
            &contract_artifacts.join("distributed.index.md"),
        ),
        tooling: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/tooling/index.md"),
            &contract_artifacts.join("tooling.index.md"),
        ),
        clustered_example: route_free::read_and_archive(
            &repo_root().join("website/docs/docs/getting-started/clustered-example/index.md"),
            &contract_artifacts.join("clustered-example.index.md"),
        ),
        todo_postgres_readme: route_free::read_and_archive(
            &repo_root().join("examples/todo-postgres/README.md"),
            &contract_artifacts.join("todo-postgres.README.md"),
        ),
        todo_sqlite_readme: route_free::read_and_archive(
            &repo_root().join("examples/todo-sqlite/README.md"),
            &contract_artifacts.join("todo-sqlite.README.md"),
        ),
        tiny_cluster_fixture_readme: route_free::read_and_archive(
            &route_free::tiny_cluster_fixture_root().join("README.md"),
            &contract_artifacts.join("tiny-cluster.fixture.README.md"),
        ),
        cluster_proof_fixture_readme: route_free::read_and_archive(
            &route_free::cluster_proof_fixture_root().join("README.md"),
            &contract_artifacts.join("cluster-proof.fixture.README.md"),
        ),
    }
}

#[test]
fn m047_s04_authoritative_cutover_rail_replays_source_first_contract_and_snapshots_the_graph() {
    let artifacts = artifact_dir("cutover-verifier-contract");
    let sources = load_contract_sources(&artifacts);

    route_free::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "authoritative_verifier": "scripts/verify-m047-s04.sh",
            "graph_contract": "scripts/tests/verify-m050-s01-onboarding-graph.test.mjs",
            "historical_wrapper_aliases": [
                "scripts/verify-m046-s06.sh",
                "scripts/verify-m046-s05.sh",
                "scripts/verify-m046-s04.sh",
                "scripts/verify-m045-s05.sh",
                "scripts/verify-m045-s04.sh"
            ],
            "public_surfaces": [
                "README.md",
                "website/docs/.vitepress/config.mts",
                "website/docs/docs/distributed-proof/index.md",
                "website/docs/docs/distributed/index.md",
                "website/docs/docs/tooling/index.md",
                "website/docs/docs/getting-started/clustered-example/index.md",
                "examples/todo-postgres/README.md",
                "examples/todo-sqlite/README.md"
            ],
            "retained_fixture_surfaces": [
                "scripts/fixtures/clustered/tiny-cluster/README.md",
                "scripts/fixtures/clustered/cluster-proof/README.md"
            ]
        }),
    );

    assert_contains_all(
        "scripts/verify-m047-s04.sh",
        &sources.verify_script,
        &[
            "node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs",
            "m050-s01-onboarding-graph",
            "cargo test -p mesh-parser m047_s04 -- --nocapture",
            "cargo test -p mesh-pkg m047_s04 -- --nocapture",
            "cargo test -p meshc --test e2e_m047_s01 -- --nocapture",
            "cargo test -p mesh-pkg scaffold_clustered_project_writes_public_cluster_contract -- --nocapture",
            "cargo test -p meshc --test tooling_e2e test_init_clustered_creates_project -- --nocapture",
            "examples/todo-postgres/README.md",
            "examples/todo-sqlite/README.md",
            "contract-sidebar-proof-surfaces",
            "contract-sidebar-distributed-proof-link",
            "contract-sidebar-production-proof-link",
            "contract-sidebar-proof-footer-opt-out",
            "contract-distributed-proof-canonical-marker",
            "contract-distributed-proof-s05",
            "contract-distributed-proof-s07",
            "contract-distributed-proof-fly",
            "${safe_name}-clustered-example-page",
            "${safe_name}-production-proof-page",
            "${safe_name}-proof-ledger",
            "${safe_name}-clustered-example-page",
            "${safe_name}-production-proof-page",
            "${safe_name}-proof-ledger",
            "contract-todo-postgres-init",
            "contract-todo-postgres-clustered-source",
            "contract-todo-postgres-clustered-route",
            "contract-todo-postgres-health",
            "contract-todo-postgres-cluster-status",
            "contract-todo-postgres-env",
            "contract-todo-sqlite-init",
            "contract-todo-sqlite-tests",
            "contract-todo-sqlite-health",
            "contract-todo-sqlite-db-path",
            "contract-todo-sqlite-postgres-branch",
            "contract-todo-sqlite-clustered-branch",
            "cargo run -q -p meshc -- test scripts/fixtures/clustered/tiny-cluster/tests",
            "cargo run -q -p meshc -- build scripts/fixtures/clustered/tiny-cluster",
            "cargo run -q -p meshc -- test scripts/fixtures/clustered/cluster-proof/tests",
            "cargo run -q -p meshc -- build scripts/fixtures/clustered/cluster-proof",
            "npm --prefix website run build",
            "cargo test -p meshc --test e2e_m047_s04 -- --nocapture",
            "status.txt",
            "current-phase.txt",
            "phase-report.txt",
            "full-contract.log",
            "latest-proof-bundle.txt",
            "retained-m047-s04-artifacts",
            "m047-s04-parser",
            "m047-s04-pkg",
            "m047-s04-compiler",
            "m047-s04-scaffold-unit",
            "m047-s04-scaffold-smoke",
            "m047-s04-tiny-cluster-tests",
            "m047-s04-tiny-cluster-build",
            "m047-s04-cluster-proof-tests",
            "m047-s04-cluster-proof-build",
            "m047-s04-docs-build",
            "m047-s04-e2e",
            "m047-s04-artifacts",
            "m047-s04-bundle-shape",
        ],
    );
    assert_omits_all(
        "scripts/verify-m047-s04.sh",
        &sources.verify_script,
        &[
            "cargo run -q -p meshc -- test tiny-cluster/tests",
            "cargo run -q -p meshc -- build tiny-cluster",
            "cargo run -q -p meshc -- test cluster-proof/tests",
            "cargo run -q -p meshc -- build cluster-proof",
        ],
    );

    for (path_label, source) in [
        ("scripts/verify-m045-s04.sh", &sources.verify_m045_s04),
        ("scripts/verify-m045-s05.sh", &sources.verify_m045_s05),
        ("scripts/verify-m046-s04.sh", &sources.verify_m046_s04),
        ("scripts/verify-m046-s05.sh", &sources.verify_m046_s05),
        ("scripts/verify-m046-s06.sh", &sources.verify_m046_s06),
    ] {
        assert_contains_all(
            path_label,
            source,
            &[
                "bash scripts/verify-m047-s04.sh",
                "retained-m047-s04-verify",
                "latest-proof-bundle.txt",
                "phase-report.txt",
                "status.txt",
                "current-phase.txt",
                "full-contract.log",
                "m047-s04-replay",
                "retain-m047-s04-verify",
            ],
        );
    }
}

#[test]
fn m047_s04_public_surfaces_keep_the_m050_onboarding_graph_and_proof_rails_discoverable() {
    let artifacts = artifact_dir("cutover-docs-contract");
    let sources = load_contract_sources(&artifacts);

    assert_onboarding_graph_config("website/docs/.vitepress/config.mts", &sources.docs_config);

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
            "reference-backend/README.md",
            CUTOVER_RAIL,
            CLOSEOUT_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            STALE_M046_AUTHORITY,
            "tiny-cluster/README.md",
            "cluster-proof/README.md",
        ],
    );

    assert_contains_all(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        &[
            "This is the only public-secondary docs page that carries the named clustered verifier rails.",
            CLUSTERED_EXAMPLE_DOC_LINK,
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            MESHER_RUNBOOK,
            VERIFY_M051_S01,
            VERIFY_M051_S02,
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            S07_RAIL_COMMAND,
            CLOSEOUT_RAIL,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
        ],
    );
    assert_omits_all(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
        &[
            "reference-backend/README.md",
            STALE_M046_AUTHORITY,
            "`bash scripts/verify-m046-s05.sh` — the lower-level equal-surface subrail",
            "`bash scripts/verify-m045-s05.sh` — the historical wrapper name retained for replay and transition into the S06 closeout rail",
            "tiny-cluster/README.md",
            "cluster-proof/README.md",
        ],
    );
    assert_clustered_surface_omits_routeful_drift(
        "website/docs/docs/distributed-proof/index.md",
        &sources.distributed_proof,
    );
}

#[test]
fn m047_s04_clustered_runbooks_keep_example_readmes_and_secondary_proof_surfaces_discoverable() {
    let artifacts = artifact_dir("cutover-runbook-contract");
    let sources = load_contract_sources(&artifacts);

    assert_contains_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[
            CLUSTERED_EXAMPLE_DOC_LINK,
            "/docs/distributed-proof/",
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            "honest local",
            "shared/deployable",
        ],
    );
    assert_omits_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[
            "reference-backend/README.md",
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            S07_RAIL_COMMAND,
            CLOSEOUT_RAIL,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            "The authoritative assembled closeout rail is `bash scripts/verify-m046-s06.sh`",
            "the authoritative repo-wide closeout rail is `bash scripts/verify-m046-s06.sh`",
            "For the repo-wide closeout story, `bash scripts/verify-m046-s06.sh` is the authoritative assembled closeout rail",
            "tiny-cluster/README.md",
            "cluster-proof/README.md",
        ],
    );
    assert_clustered_surface_omits_routeful_drift(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
    );

    assert_contains_all(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        &[
            CLUSTERED_EXAMPLE_DOC_LINK,
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            "honest local",
            "shared/deployable",
        ],
    );
    assert_omits_all(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        &[
            "reference-backend/README.md",
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            S07_RAIL_COMMAND,
            CLOSEOUT_RAIL,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            "The authoritative assembled closeout rail is `bash scripts/verify-m046-s06.sh`",
            "the authoritative repo-wide closeout rail is `bash scripts/verify-m046-s06.sh`",
            "For the repo-wide closeout story, `bash scripts/verify-m046-s06.sh` is the authoritative assembled closeout rail",
            "tiny-cluster/README.md",
            "cluster-proof/README.md",
        ],
    );
    assert_clustered_surface_omits_routeful_drift(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
    );

    assert_contains_all(
        "website/docs/docs/distributed/index.md",
        &sources.distributed,
        &[MESHER_RUNBOOK, VERIFY_M051_S01, VERIFY_M051_S02],
    );

    assert_contains_all(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &sources.clustered_example,
        &[
            TODO_POSTGRES_README,
            TODO_SQLITE_README,
            PRODUCTION_BACKEND_PROOF_DOC_LINK,
            "/docs/distributed-proof/",
        ],
    );
    assert_omits_all(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &sources.clustered_example,
        &[
            "reference-backend/README.md",
            CUTOVER_RAIL,
            TODO_SUBRAIL,
            S07_RAIL_COMMAND,
            CLOSEOUT_RAIL,
            FLY_READ_ONLY_RAIL,
            HISTORICAL_M046_CLOSEOUT_ALIAS,
            HISTORICAL_M046_EQUAL_SURFACE_ALIAS,
            HISTORICAL_M046_PACKAGE_ALIAS,
            HISTORICAL_M045_CLOSEOUT_ALIAS,
            HISTORICAL_M045_ASSEMBLED_ALIAS,
            HISTORICAL_FAILOVER_SUBRAIL,
            "tiny-cluster/README.md",
            "cluster-proof/README.md",
        ],
    );
    assert_clustered_surface_omits_routeful_drift(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &sources.clustered_example,
    );

    assert_contains(
        "website/docs/docs/tooling/index.md",
        &sources.tooling,
        PRODUCTION_BACKEND_PROOF_DOC_LINK,
    );
}

#[test]
fn m047_s04_example_readmes_define_the_public_postgres_vs_sqlite_split() {
    let artifacts = artifact_dir("cutover-example-readmes");
    let sources = load_contract_sources(&artifacts);

    assert_contains_all(
        "examples/todo-postgres/README.md",
        &sources.todo_postgres_readme,
        &[
            "meshc init --template todo-api --db postgres",
            "@cluster pub fn sync_todos()",
            "HTTP.clustered(1, ...)",
            "GET /health",
            "meshc cluster status",
            "DATABASE_URL",
        ],
    );
    assert_omits_all(
        "examples/todo-postgres/README.md",
        &sources.todo_postgres_readme,
        &[
            "tiny-cluster/README.md",
            "cluster-proof/README.md",
            "clustered(work)",
        ],
    );

    assert_contains_all(
        "examples/todo-sqlite/README.md",
        &sources.todo_sqlite_readme,
        &[
            "meshc init --template todo-api --db sqlite",
            "meshc test .",
            "GET /health",
            "TODO_DB_PATH",
            "meshc init --template todo-api --db postgres",
            "meshc init --clustered",
        ],
    );
    assert_omits_all(
        "examples/todo-sqlite/README.md",
        &sources.todo_sqlite_readme,
        &[
            "tiny-cluster/README.md",
            "cluster-proof/README.md",
            "@cluster pub fn sync_todos()",
            "meshc cluster continuity",
            "HTTP.clustered(1, ...)",
        ],
    );
}

#[test]
fn m047_s04_internal_fixture_readmes_stay_lower_level_contracts_after_the_move() {
    let artifacts = artifact_dir("cutover-fixture-readmes");
    let sources = load_contract_sources(&artifacts);

    assert_contains_all(
        "scripts/fixtures/clustered/tiny-cluster/README.md",
        &sources.tiny_cluster_fixture_readme,
        &[
            "scripts/fixtures/clustered/tiny-cluster/",
            "cargo run -q -p meshc -- build scripts/fixtures/clustered/tiny-cluster",
            "cargo run -q -p meshc -- test scripts/fixtures/clustered/tiny-cluster/tests",
            CUTOVER_RAIL,
        ],
    );
    assert_omits_all(
        "scripts/fixtures/clustered/tiny-cluster/README.md",
        &sources.tiny_cluster_fixture_readme,
        &[
            "cargo run -q -p meshc -- build tiny-cluster",
            "cargo run -q -p meshc -- test tiny-cluster/tests",
        ],
    );

    assert_contains_all(
        "scripts/fixtures/clustered/cluster-proof/README.md",
        &sources.cluster_proof_fixture_readme,
        &[
            "scripts/fixtures/clustered/cluster-proof/",
            "cargo run -q -p meshc -- build scripts/fixtures/clustered/cluster-proof",
            "cargo run -q -p meshc -- test scripts/fixtures/clustered/cluster-proof/tests",
            "docker build -f scripts/fixtures/clustered/cluster-proof/Dockerfile -t mesh-cluster-proof .",
            CUTOVER_RAIL,
        ],
    );
    assert_omits_all(
        "scripts/fixtures/clustered/cluster-proof/README.md",
        &sources.cluster_proof_fixture_readme,
        &[
            "cargo run -q -p meshc -- build cluster-proof",
            "cargo run -q -p meshc -- test cluster-proof/tests",
            "docker build -f cluster-proof/Dockerfile -t mesh-cluster-proof .",
        ],
    );
}
