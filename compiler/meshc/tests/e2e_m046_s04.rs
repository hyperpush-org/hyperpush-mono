mod support;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use support::m046_route_free as route_free;

const SHARED_COOKIE: &str = "mesh-m046-s04-cli-cookie";
const DISCOVERY_SEED: &str = "localhost";
const STARTUP_RUNTIME_NAME: &str = route_free::STARTUP_RUNTIME_NAME;
const STARTUP_SOURCE_DECLARATION: &str = route_free::STARTUP_SOURCE_DECLARATION;
const STARTUP_RUNTIME_NAME_GUIDANCE: &str = route_free::STARTUP_RUNTIME_NAME_GUIDANCE;
const STARTUP_AUTOSTART_GUIDANCE: &str = route_free::STARTUP_AUTOSTART_GUIDANCE;

struct ClusterProofSources {
    manifest: String,
    main: String,
    work: String,
    readme: String,
    work_test: String,
    dockerfile: String,
    fly_toml: String,
    support_mod: String,
    support_helper: String,
    s03_rail: String,
    verify_script: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct FileSnapshot {
    length: usize,
    sha256: String,
}

fn repo_root() -> PathBuf {
    route_free::repo_root()
}

fn cluster_proof_dir() -> PathBuf {
    route_free::cluster_proof_fixture_root()
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m046-s04", test_name)
}

fn load_cluster_proof_sources(artifacts: &Path) -> ClusterProofSources {
    let package_dir = cluster_proof_dir();
    let package_artifacts = artifacts.join("package");
    let contract_artifacts = artifacts.join("contract");
    ClusterProofSources {
        manifest: route_free::read_and_archive(
            &package_dir.join("mesh.toml"),
            &package_artifacts.join("mesh.toml"),
        ),
        main: route_free::read_and_archive(
            &package_dir.join("main.mpl"),
            &package_artifacts.join("main.mpl"),
        ),
        work: route_free::read_and_archive(
            &package_dir.join("work.mpl"),
            &package_artifacts.join("work.mpl"),
        ),
        readme: route_free::read_and_archive(
            &package_dir.join("README.md"),
            &package_artifacts.join("README.md"),
        ),
        work_test: route_free::read_and_archive(
            &package_dir.join("tests").join("work.test.mpl"),
            &package_artifacts.join("tests").join("work.test.mpl"),
        ),
        dockerfile: route_free::read_and_archive(
            &package_dir.join("Dockerfile"),
            &package_artifacts.join("Dockerfile"),
        ),
        fly_toml: route_free::read_and_archive(
            &package_dir.join("fly.toml"),
            &package_artifacts.join("fly.toml"),
        ),
        support_mod: route_free::read_and_archive(
            &repo_root().join("compiler/meshc/tests/support/mod.rs"),
            &contract_artifacts.join("support/mod.rs"),
        ),
        support_helper: route_free::read_and_archive(
            &repo_root().join("compiler/meshc/tests/support/m046_route_free.rs"),
            &contract_artifacts.join("support/m046_route_free.rs"),
        ),
        s03_rail: route_free::read_and_archive(
            &repo_root().join("compiler/meshc/tests/e2e_m046_s03.rs"),
            &contract_artifacts.join("e2e_m046_s03.rs"),
        ),
        verify_script: route_free::read_and_archive(
            &repo_root().join("scripts/verify-m046-s04.sh"),
            &contract_artifacts.join("verify-m046-s04.sh"),
        ),
    }
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

fn assert_cluster_proof_source_contract(sources: &ClusterProofSources) {
    assert_contains("cluster-proof/mesh.toml", &sources.manifest, "[package]");
    assert_omits("cluster-proof/mesh.toml", &sources.manifest, "[cluster]");
    assert_omits("cluster-proof/mesh.toml", &sources.manifest, "declarations");

    assert_contains(
        "cluster-proof/main.mpl",
        &sources.main,
        "Node.start_from_env()",
    );
    assert_contains(
        "cluster-proof/main.mpl",
        &sources.main,
        "[cluster-proof] runtime bootstrap",
    );
    assert_eq!(
        sources.main.matches("Node.start_from_env()").count(),
        1,
        "cluster-proof/main.mpl must keep exactly one Node.start_from_env() call"
    );
    for needle in [
        "HTTP.serve",
        "/work",
        "/status",
        "/membership",
        "/health",
        "Continuity.",
        "from Cluster",
        "from Config",
        "from WorkContinuity",
    ] {
        assert_omits("cluster-proof/main.mpl", &sources.main, needle);
    }

    assert_contains(
        "cluster-proof/work.mpl",
        &sources.work,
        STARTUP_SOURCE_DECLARATION,
    );
    assert_contains("cluster-proof/work.mpl", &sources.work, "1 + 1");
    for needle in [
        "declared_work_runtime_name",
        "clustered(work)",
        "Env.get_int",
        "Timer.sleep",
        "CLUSTER_PROOF_WORK_DELAY_MS",
        "HTTP.serve",
        "/work",
        "/status",
        "/membership",
        "/health",
        "Continuity.",
        "from Cluster",
    ] {
        assert_omits("cluster-proof/work.mpl", &sources.work, needle);
    }

    assert_contains(
        "cluster-proof/tests/work.test.mpl",
        &sources.work_test,
        "manifest and source stay source-first and route-free",
    );
    assert_contains(
        "cluster-proof/tests/work.test.mpl",
        &sources.work_test,
        "assert_not_contains(main_source, \"/work\")",
    );
    assert_contains(
        "cluster-proof/tests/work.test.mpl",
        &sources.work_test,
        "assert_not_contains(work_source, \"Timer.sleep\")",
    );
    assert_contains(
        "cluster-proof/tests/work.test.mpl",
        &sources.work_test,
        "assert_not_contains(readme, \"mesh-cluster-proof.fly.dev\")",
    );

    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        "meshc cluster status",
    );
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        "meshc cluster continuity",
    );
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        "meshc cluster diagnostics",
    );
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        "cargo run -q -p meshc -- build scripts/fixtures/clustered/cluster-proof",
    );
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        "cargo run -q -p meshc -- test scripts/fixtures/clustered/cluster-proof/tests",
    );
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        "docker build -f scripts/fixtures/clustered/cluster-proof/Dockerfile -t mesh-cluster-proof .",
    );
    assert_contains("cluster-proof/README.md", &sources.readme, "route-free");
    assert_contains("cluster-proof/README.md", &sources.readme, "`@cluster`");
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        STARTUP_RUNTIME_NAME_GUIDANCE,
    );
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        STARTUP_AUTOSTART_GUIDANCE,
    );
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        "scripts/fixtures/clustered/cluster-proof/Dockerfile",
    );
    assert_contains(
        "cluster-proof/README.md",
        &sources.readme,
        "scripts/fixtures/clustered/cluster-proof/fly.toml",
    );
    for needle in [
        "/work",
        "/membership",
        "mesh-cluster-proof.fly.dev",
        "CLUSTER_PROOF_WORK_DELAY_MS",
        "declared_work_runtime_name()",
        "clustered(work)",
        "docker-entrypoint.sh",
        "http_service",
        "cargo run -q -p meshc -- build cluster-proof",
        "cargo run -q -p meshc -- test cluster-proof/tests",
        "docker build -f cluster-proof/Dockerfile -t mesh-cluster-proof .",
    ] {
        assert_omits("cluster-proof/README.md", &sources.readme, needle);
    }

    assert_contains(
        "cluster-proof/Dockerfile",
        &sources.dockerfile,
        "COPY --from=builder /tmp/cluster-proof /usr/local/bin/cluster-proof",
    );
    assert_contains(
        "cluster-proof/Dockerfile",
        &sources.dockerfile,
        "./target/debug/meshc build scripts/fixtures/clustered/cluster-proof --output /tmp/cluster-proof --no-color",
    );
    assert_contains(
        "cluster-proof/Dockerfile",
        &sources.dockerfile,
        "ENTRYPOINT [\"/usr/local/bin/cluster-proof\"]",
    );
    assert_contains(
        "cluster-proof/Dockerfile",
        &sources.dockerfile,
        "EXPOSE 4370",
    );
    for needle in [
        "docker-entrypoint.sh",
        "EXPOSE 8080",
        "meshc build cluster-proof --output /tmp/cluster-proof --no-color",
    ] {
        assert_omits("cluster-proof/Dockerfile", &sources.dockerfile, needle);
    }

    assert_contains(
        "cluster-proof/fly.toml",
        &sources.fly_toml,
        "dockerfile = 'scripts/fixtures/clustered/cluster-proof/Dockerfile'",
    );
    assert_contains(
        "cluster-proof/fly.toml",
        &sources.fly_toml,
        "MESH_CLUSTER_PORT = '4370'",
    );
    assert_contains(
        "cluster-proof/fly.toml",
        &sources.fly_toml,
        "MESH_DISCOVERY_SEED = 'mesh-cluster-proof.internal'",
    );
    for needle in [
        "http_service",
        "\n  PORT =",
        "\nPORT =",
        "dockerfile = 'cluster-proof/Dockerfile'",
    ] {
        assert_omits("cluster-proof/fly.toml", &sources.fly_toml, needle);
    }

    assert_contains(
        "compiler/meshc/tests/support/mod.rs",
        &sources.support_mod,
        "pub mod m046_route_free;",
    );
    assert_contains(
        "compiler/meshc/tests/support/m046_route_free.rs",
        &sources.support_helper,
        "build_package_binary_to_output(",
    );
    assert_contains(
        "compiler/meshc/tests/support/m046_route_free.rs",
        &sources.support_helper,
        "cluster_proof_fixture_root()",
    );
    assert_contains(
        "compiler/meshc/tests/support/m046_route_free.rs",
        &sources.support_helper,
        "CLUSTER_PROOF_FIXTURE_DOCKERFILE_RELATIVE",
    );
    assert_contains(
        "compiler/meshc/tests/support/m046_route_free.rs",
        &sources.support_helper,
        "read_required_build_metadata(",
    );
    assert_contains(
        "compiler/meshc/tests/support/m046_route_free.rs",
        &sources.support_helper,
        "wait_for_runtime_name_discovered_with_label(",
    );
    assert_contains(
        "compiler/meshc/tests/support/m046_route_free.rs",
        &sources.support_helper,
        "wait_for_continuity_record_matching(",
    );

    assert_contains(
        "compiler/meshc/tests/e2e_m046_s03.rs",
        &sources.s03_rail,
        "mod support;",
    );
    assert_contains(
        "compiler/meshc/tests/e2e_m046_s03.rs",
        &sources.s03_rail,
        "use support::m046_route_free as route_free;",
    );
    assert_contains(
        "compiler/meshc/tests/e2e_m046_s03.rs",
        &sources.s03_rail,
        "route_free::wait_for_cluster_status_membership",
    );

    assert_contains(
        "scripts/verify-m046-s04.sh",
        &sources.verify_script,
        "latest-proof-bundle.txt",
    );
    assert_contains(
        "scripts/verify-m046-s04.sh",
        &sources.verify_script,
        "retained-m047-s04-verify",
    );
    assert_contains(
        "scripts/verify-m046-s04.sh",
        &sources.verify_script,
        "m047-s04-replay",
    );
    assert_contains(
        "scripts/verify-m046-s04.sh",
        &sources.verify_script,
        "retain-m047-s04-verify",
    );
    assert_contains(
        "scripts/verify-m046-s04.sh",
        &sources.verify_script,
        "bash scripts/verify-m047-s04.sh",
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn snapshot_file(path: &Path) -> Option<FileSnapshot> {
    if !path.exists() {
        return None;
    }

    let bytes = fs::read(path)
        .unwrap_or_else(|error| panic!("failed to read snapshot file {}: {error}", path.display()));
    Some(FileSnapshot {
        length: bytes.len(),
        sha256: sha256_hex(&bytes),
    })
}

fn assert_snapshot_unchanged(
    label: &str,
    path: &Path,
    before: &Option<FileSnapshot>,
    after: &Option<FileSnapshot>,
) {
    assert_eq!(
        before, after,
        "expected {label} at {} to stay unchanged during temp-path build; before={before:?} after={after:?}",
        path.display(),
    );
}

fn build_cluster_proof_binary_to_temp(artifacts: &Path) -> route_free::BuildOutputMetadata {
    let tracked_binary_path = cluster_proof_dir().join("cluster-proof");
    let tracked_llvm_path = cluster_proof_dir().join("cluster-proof.ll");
    let tracked_binary_before = snapshot_file(&tracked_binary_path);
    let tracked_llvm_before = snapshot_file(&tracked_llvm_path);

    let binary_dir = artifacts.join("bin");
    fs::create_dir_all(&binary_dir).expect("failed to create temp binary dir");
    let output_path = binary_dir.join("cluster-proof");
    let build_metadata =
        route_free::build_package_binary_to_output(&cluster_proof_dir(), &output_path, artifacts);
    let metadata = route_free::read_required_build_metadata(artifacts)
        .unwrap_or_else(|error| panic!("temp build metadata should be readable: {error}"));

    assert_eq!(metadata, build_metadata);
    assert_eq!(metadata.source_package_dir, cluster_proof_dir());
    assert_eq!(metadata.binary_path, output_path);
    assert!(
        !metadata.binary_path.starts_with(cluster_proof_dir()),
        "cluster-proof temp build output must stay outside package dir: {}",
        metadata.binary_path.display(),
    );

    let tracked_binary_after = snapshot_file(&tracked_binary_path);
    let tracked_llvm_after = snapshot_file(&tracked_llvm_path);
    assert_snapshot_unchanged(
        "tracked cluster-proof binary",
        &tracked_binary_path,
        &tracked_binary_before,
        &tracked_binary_after,
    );
    assert_snapshot_unchanged(
        "tracked cluster-proof llvm",
        &tracked_llvm_path,
        &tracked_llvm_before,
        &tracked_llvm_after,
    );

    route_free::write_json_artifact(
        &artifacts.join("tracked-binary-snapshots.json"),
        &json!({
            "tracked_binary_path": tracked_binary_path.display().to_string(),
            "tracked_binary_before": tracked_binary_before,
            "tracked_binary_after": tracked_binary_after,
            "tracked_llvm_path": tracked_llvm_path.display().to_string(),
            "tracked_llvm_before": tracked_llvm_before,
            "tracked_llvm_after": tracked_llvm_after,
        }),
    );

    metadata
}

#[test]
fn m046_s04_cluster_proof_helpers_reject_malformed_cluster_json() {
    let error = route_free::parse_json_stdout(b"{not-json}", "continuity response").unwrap_err();
    assert!(error.contains("continuity response returned invalid JSON"));
}

#[test]
fn m046_s04_cluster_proof_helpers_require_precreated_temp_output_parent() {
    let artifacts = artifact_dir("cluster-proof-helper-preflight");
    let missing_parent_output = artifacts
        .join("missing-parent")
        .join("bin")
        .join("cluster-proof");

    let result = std::panic::catch_unwind(|| {
        route_free::build_package_binary_to_output(
            &cluster_proof_dir(),
            &missing_parent_output,
            &artifacts,
        )
    });
    let payload = result.expect_err("missing temp output parent should fail before build");
    let message = route_free::panic_payload_to_string(payload);
    assert!(
        message.contains("pre-created temp output parent"),
        "{message}"
    );
    assert!(artifacts.join("build-preflight-error.txt").exists());
}

#[test]
fn m046_s04_cluster_proof_helpers_reject_malformed_build_metadata() {
    let artifacts = artifact_dir("cluster-proof-helper-build-meta");
    route_free::write_artifact(&artifacts.join("build-meta.json"), "{not-json}");

    let error = route_free::read_required_build_metadata(&artifacts)
        .expect_err("malformed build metadata should fail closed");
    assert!(error.contains("malformed JSON"), "{error}");
}

#[test]
fn m046_s04_cluster_proof_helpers_reject_missing_dockerfile() {
    let artifacts = artifact_dir("cluster-proof-helper-missing-dockerfile");
    let temp = tempfile::tempdir().expect("create broken cluster-proof fixture tempdir");
    let broken_fixture_root = temp.path().join("cluster-proof");
    route_free::archive_directory_tree(&cluster_proof_dir(), &broken_fixture_root);
    fs::remove_file(broken_fixture_root.join("Dockerfile"))
        .expect("remove cluster-proof Dockerfile from broken fixture copy");

    let error = route_free::validate_cluster_proof_fixture_root(&broken_fixture_root)
        .expect_err("missing fixture Dockerfile should fail closed");
    route_free::write_artifact(&artifacts.join("fixture-validation.error.txt"), &error);
    assert!(
        error.contains("missing required files: Dockerfile"),
        "{error}"
    );
}

#[test]
fn m046_s04_cluster_proof_package_contract_remains_source_first_and_route_free() {
    let artifacts = artifact_dir("cluster-proof-package-contract");
    let sources = load_cluster_proof_sources(&artifacts);
    assert_cluster_proof_source_contract(&sources);
}

#[test]
fn m046_s04_cluster_proof_package_builds_to_temp_output_and_runs_repo_smoke_rail() {
    let artifacts = artifact_dir("cluster-proof-package-build-and-test");
    let sources = load_cluster_proof_sources(&artifacts);
    assert_cluster_proof_source_contract(&sources);

    let metadata = build_cluster_proof_binary_to_temp(&artifacts);
    assert!(
        metadata.binary_path.exists(),
        "expected temp cluster-proof binary at {}",
        metadata.binary_path.display()
    );
    route_free::run_package_tests(
        &cluster_proof_dir().join("tests"),
        &artifacts,
        "package-tests",
    );
}

#[test]
fn m046_s04_cluster_proof_startup_dedupes_and_surfaces_runtime_truth_on_two_nodes() {
    let artifacts = artifact_dir("cluster-proof-startup-two-node");
    let sources = load_cluster_proof_sources(&artifacts);
    assert_cluster_proof_source_contract(&sources);
    let build_metadata = build_cluster_proof_binary_to_temp(&artifacts);
    let cluster_port = route_free::dual_stack_cluster_port();
    let primary_node = format!(
        "cluster-proof-primary@{}:{cluster_port}",
        route_free::LOOPBACK_V4
    );
    let standby_node = format!(
        "cluster-proof-standby@[{}]:{}",
        route_free::LOOPBACK_V6,
        cluster_port
    );
    let expected_nodes = vec![primary_node.clone(), standby_node.clone()];

    route_free::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "package_dir": cluster_proof_dir().display().to_string(),
            "binary_path": build_metadata.binary_path.display().to_string(),
            "cluster_port": cluster_port,
            "startup_runtime_name": STARTUP_RUNTIME_NAME,
            "request_key": Value::Null,
            "primary_node": primary_node,
            "standby_node": standby_node,
        }),
    );

    let primary_proc = route_free::spawn_route_free_runtime(
        &build_metadata.binary_path,
        &cluster_proof_dir(),
        &artifacts,
        "primary",
        &primary_node,
        cluster_port,
        "primary",
        0,
        SHARED_COOKIE,
        DISCOVERY_SEED,
    );
    let standby_proc = route_free::spawn_route_free_runtime(
        &build_metadata.binary_path,
        &cluster_proof_dir(),
        &artifacts,
        "standby",
        &standby_node,
        cluster_port,
        "standby",
        0,
        SHARED_COOKIE,
        DISCOVERY_SEED,
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        route_free::wait_for_cluster_status_membership(
            &artifacts,
            "cluster-status-primary",
            &primary_node,
            std::slice::from_ref(&standby_node),
            &expected_nodes,
            "primary",
            0,
            &["local_only", "healthy"],
            SHARED_COOKIE,
        );
        route_free::wait_for_cluster_status_membership(
            &artifacts,
            "cluster-status-standby",
            &standby_node,
            std::slice::from_ref(&primary_node),
            &expected_nodes,
            "standby",
            0,
            &["local_only", "healthy"],
            SHARED_COOKIE,
        );

        let primary_list = route_free::wait_for_runtime_name_discovered_with_label(
            &artifacts,
            "cluster-continuity-list-primary",
            &primary_node,
            STARTUP_RUNTIME_NAME,
            SHARED_COOKIE,
        );
        let standby_list = route_free::wait_for_runtime_name_discovered_with_label(
            &artifacts,
            "cluster-continuity-list-standby",
            &standby_node,
            STARTUP_RUNTIME_NAME,
            SHARED_COOKIE,
        );
        assert_eq!(route_free::required_u64(&primary_list, "total_records"), 1);
        assert_eq!(route_free::required_u64(&standby_list, "total_records"), 1);
        assert!(!route_free::required_bool(&primary_list, "truncated"));
        assert!(!route_free::required_bool(&standby_list, "truncated"));
        assert_eq!(
            route_free::count_records_for_runtime_name(&primary_list, STARTUP_RUNTIME_NAME),
            1
        );
        assert_eq!(
            route_free::count_records_for_runtime_name(&standby_list, STARTUP_RUNTIME_NAME),
            1
        );

        let request_key = route_free::required_str(
            route_free::record_for_runtime_name(&primary_list, STARTUP_RUNTIME_NAME),
            "request_key",
        );
        assert_eq!(
            request_key,
            route_free::required_str(
                route_free::record_for_runtime_name(&standby_list, STARTUP_RUNTIME_NAME),
                "request_key",
            )
        );

        route_free::write_json_artifact(
            &artifacts.join("scenario-meta.json"),
            &json!({
                "package_dir": cluster_proof_dir().display().to_string(),
                "binary_path": build_metadata.binary_path.display().to_string(),
                "cluster_port": cluster_port,
                "startup_runtime_name": STARTUP_RUNTIME_NAME,
                "request_key": request_key,
                "primary_node": primary_node,
                "standby_node": standby_node,
            }),
        );

        let human_list = route_free::run_meshc_cluster(
            &artifacts,
            "cluster-continuity-list-primary-human",
            &["cluster", "continuity", &primary_node],
            SHARED_COOKIE,
        );
        assert!(
            human_list.status.success(),
            "human continuity list should succeed:\n{}",
            route_free::command_output_text(&human_list)
        );
        let human_list_stdout = String::from_utf8_lossy(&human_list.stdout);
        assert!(
            human_list_stdout.contains(&format!(
                "declared_handler_runtime_name={STARTUP_RUNTIME_NAME}"
            )),
            "human continuity list should surface the startup runtime name:\n{human_list_stdout}"
        );
        assert!(
            human_list_stdout.contains(&format!("request_key={request_key}")),
            "human continuity list should surface the startup request key:\n{human_list_stdout}"
        );

        let primary_continuity = route_free::wait_for_continuity_record_completed(
            &artifacts,
            "cluster-continuity-primary-completed",
            &primary_node,
            &request_key,
            STARTUP_RUNTIME_NAME,
            SHARED_COOKIE,
        );
        let standby_continuity = route_free::wait_for_continuity_record_completed(
            &artifacts,
            "cluster-continuity-standby-completed",
            &standby_node,
            &request_key,
            STARTUP_RUNTIME_NAME,
            SHARED_COOKIE,
        );

        let human_single = route_free::run_meshc_cluster(
            &artifacts,
            "cluster-continuity-single-primary-human",
            &["cluster", "continuity", &primary_node, &request_key],
            SHARED_COOKIE,
        );
        assert!(
            human_single.status.success(),
            "human continuity single-record output should succeed:\n{}",
            route_free::command_output_text(&human_single)
        );
        let human_single_stdout = String::from_utf8_lossy(&human_single.stdout);
        assert!(
            human_single_stdout.contains(&format!(
                "declared_handler_runtime_name: {STARTUP_RUNTIME_NAME}"
            )),
            "human continuity single-record output should surface the runtime name:\n{human_single_stdout}"
        );
        assert!(
            human_single_stdout.contains("phase: completed"),
            "human continuity single-record output should surface completion:\n{human_single_stdout}"
        );
        assert!(
            human_single_stdout.contains("result: succeeded"),
            "human continuity single-record output should surface success:\n{human_single_stdout}"
        );

        let primary_record = &primary_continuity["record"];
        let standby_record = &standby_continuity["record"];
        let owner_node = route_free::required_str(primary_record, "owner_node");
        let replica_node = route_free::required_str(primary_record, "replica_node");
        assert_eq!(
            owner_node,
            route_free::required_str(standby_record, "owner_node")
        );
        assert_eq!(
            replica_node,
            route_free::required_str(standby_record, "replica_node")
        );
        assert_eq!(
            route_free::required_str(primary_record, "request_key"),
            request_key
        );
        assert_eq!(
            route_free::required_str(standby_record, "request_key"),
            request_key
        );
        assert_eq!(
            route_free::required_str(primary_record, "declared_handler_runtime_name"),
            STARTUP_RUNTIME_NAME,
        );
        assert_eq!(
            route_free::required_str(standby_record, "declared_handler_runtime_name"),
            STARTUP_RUNTIME_NAME,
        );
        assert_eq!(
            route_free::required_str(primary_record, "phase"),
            "completed"
        );
        assert_eq!(
            route_free::required_str(standby_record, "phase"),
            "completed"
        );
        assert_eq!(
            route_free::required_str(primary_record, "result"),
            "succeeded"
        );
        assert_eq!(
            route_free::required_str(standby_record, "result"),
            "succeeded"
        );
        assert!(expected_nodes.contains(&owner_node));
        assert!(expected_nodes.contains(&replica_node));
        assert_ne!(owner_node, replica_node);
        assert_eq!(
            route_free::required_str(primary_record, "execution_node"),
            owner_node
        );
        assert_eq!(
            route_free::required_str(standby_record, "execution_node"),
            owner_node
        );
        assert_eq!(
            route_free::required_str(primary_record, "replica_status"),
            "mirrored"
        );
        assert_eq!(
            route_free::required_str(standby_record, "replica_status"),
            "mirrored"
        );
        assert_eq!(route_free::required_str(primary_record, "error"), "");
        assert_eq!(route_free::required_str(standby_record, "error"), "");

        let (primary_diagnostics, standby_diagnostics) = route_free::wait_for_startup_diagnostics(
            &artifacts,
            &primary_node,
            &standby_node,
            &request_key,
            SHARED_COOKIE,
        );
        let primary_entries =
            route_free::diagnostic_entries_for_request(&primary_diagnostics, &request_key);
        let standby_entries =
            route_free::diagnostic_entries_for_request(&standby_diagnostics, &request_key);
        let combined_transitions: Vec<_> = primary_entries
            .iter()
            .chain(standby_entries.iter())
            .filter_map(|entry| entry["transition"].as_str())
            .collect();
        assert!(combined_transitions.contains(&"startup_trigger"));
        assert!(combined_transitions.contains(&"startup_dispatch_window"));
        assert!(combined_transitions.contains(&"startup_completed"));
        assert!(!combined_transitions.contains(&"startup_rejected"));
        assert!(!combined_transitions.contains(&"startup_convergence_timeout"));
    }));

    let primary_logs = route_free::stop_process(primary_proc);
    let standby_logs = route_free::stop_process(standby_proc);
    route_free::write_artifact(
        &artifacts.join("primary.combined.log"),
        &primary_logs.combined,
    );
    route_free::write_artifact(
        &artifacts.join("standby.combined.log"),
        &standby_logs.combined,
    );
    if let Err(payload) = result {
        panic!(
            "{}\nartifacts: {}\nprimary stdout:\n{}\nprimary stderr:\n{}\nstandby stdout:\n{}\nstandby stderr:\n{}",
            route_free::panic_payload_to_string(payload),
            artifacts.display(),
            primary_logs.stdout,
            primary_logs.stderr,
            standby_logs.stdout,
            standby_logs.stderr,
        );
    }

    for required in [
        "scenario-meta.json",
        "build.log",
        "build-meta.json",
        "tracked-binary-snapshots.json",
        "cluster-status-primary.json",
        "cluster-status-standby.json",
        "cluster-continuity-list-primary.json",
        "cluster-continuity-list-standby.json",
        "cluster-continuity-primary-completed.json",
        "cluster-continuity-standby-completed.json",
        "cluster-diagnostics-primary.json",
        "cluster-diagnostics-standby.json",
        "cluster-continuity-list-primary-human.log",
        "cluster-continuity-single-primary-human.log",
        "primary.stdout.log",
        "primary.stderr.log",
        "standby.stdout.log",
        "standby.stderr.log",
    ] {
        assert!(
            artifacts.join(required).exists(),
            "missing retained route-free proof artifact {} in {}",
            required,
            artifacts.display(),
        );
    }

    route_free::assert_log_absent(&primary_logs, SHARED_COOKIE);
    route_free::assert_log_absent(&standby_logs, SHARED_COOKIE);
    route_free::assert_log_contains(
        &primary_logs,
        &format!("[cluster-proof] runtime bootstrap mode=cluster node={primary_node}"),
    );
    route_free::assert_log_contains(
        &standby_logs,
        &format!("[cluster-proof] runtime bootstrap mode=cluster node={standby_node}"),
    );
    route_free::assert_log_contains(
        &primary_logs,
        &format!(
            "[mesh-rt startup] transition=startup_dispatch_window runtime_name={STARTUP_RUNTIME_NAME}"
        ),
    );
}
