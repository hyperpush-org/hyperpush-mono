mod support;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use support::m046_route_free as route_free;

const LOOPBACK_V4: &str = "127.0.0.1";
const LOOPBACK_V6: &str = "::1";
const SHARED_COOKIE: &str = "mesh-m046-s03-cli-cookie";
const DISCOVERY_SEED: &str = "localhost";
const STARTUP_RUNTIME_NAME: &str = route_free::STARTUP_RUNTIME_NAME;
const STARTUP_SOURCE_DECLARATION: &str = route_free::STARTUP_SOURCE_DECLARATION;
const STARTUP_RUNTIME_NAME_GUIDANCE: &str = route_free::STARTUP_RUNTIME_NAME_GUIDANCE;
const STARTUP_AUTOSTART_GUIDANCE: &str = route_free::STARTUP_AUTOSTART_GUIDANCE;
const STARTUP_PENDING_WINDOW_SOURCE: &str = "language-owned runtime startup dispatch window";

struct TinyClusterSources {
    manifest: String,
    main: String,
    work: String,
    readme: String,
    work_test: String,
    slice_plan: String,
    verify_script: String,
}

type SpawnedProcess = route_free::SpawnedProcess;
type StoppedProcess = route_free::StoppedProcess;

fn repo_root() -> PathBuf {
    route_free::repo_root()
}

fn tiny_cluster_dir() -> PathBuf {
    route_free::tiny_cluster_fixture_root()
}

fn dual_stack_cluster_port() -> u16 {
    route_free::dual_stack_cluster_port()
}

fn stable_hash_u64(value: &str) -> u64 {
    let digest = Sha256::digest(value.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes)
}

fn startup_request_owns_primary(primary_node: &str, standby_node: &str) -> bool {
    let mut membership = vec![primary_node.to_string(), standby_node.to_string()];
    membership.sort_by_key(|value| (stable_hash_u64(value), value.clone()));
    let owner_index = (stable_hash_u64(&format!("request::{}", startup_request_key())) as usize)
        % membership.len();
    membership[owner_index] == primary_node
}

fn dual_stack_cluster_port_for_primary_owned_startup() -> u16 {
    for _ in 0..64 {
        let cluster_port = dual_stack_cluster_port();
        let primary_node = format!("tiny-cluster-primary@{LOOPBACK_V4}:{cluster_port}");
        let standby_node = format!("tiny-cluster-standby@[{}]:{}", LOOPBACK_V6, cluster_port);
        if startup_request_owns_primary(&primary_node, &standby_node) {
            return cluster_port;
        }
    }

    panic!("failed to find a dual-stack cluster port whose startup record is primary-owned");
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m046-s03", test_name)
}

fn write_artifact(path: &Path, contents: impl AsRef<str>) {
    route_free::write_artifact(path, contents);
}

fn write_json_artifact(path: &Path, value: &impl serde::Serialize) {
    route_free::write_json_artifact(path, value);
}

fn read_and_archive(source_path: &Path, artifact_path: &Path) -> String {
    route_free::read_and_archive(source_path, artifact_path)
}

fn load_tiny_cluster_sources(artifacts: &Path) -> TinyClusterSources {
    let package_dir = tiny_cluster_dir();
    let package_artifacts = artifacts.join("package");
    let contract_artifacts = artifacts.join("contract");
    TinyClusterSources {
        manifest: read_and_archive(
            &package_dir.join("mesh.toml"),
            &package_artifacts.join("mesh.toml"),
        ),
        main: read_and_archive(
            &package_dir.join("main.mpl"),
            &package_artifacts.join("main.mpl"),
        ),
        work: read_and_archive(
            &package_dir.join("work.mpl"),
            &package_artifacts.join("work.mpl"),
        ),
        readme: read_and_archive(
            &package_dir.join("README.md"),
            &package_artifacts.join("README.md"),
        ),
        work_test: read_and_archive(
            &package_dir.join("tests").join("work.test.mpl"),
            &package_artifacts.join("tests").join("work.test.mpl"),
        ),
        slice_plan: read_and_archive(
            &repo_root().join(".gsd/milestones/M046/slices/S03/S03-PLAN.md"),
            &contract_artifacts.join("S03-PLAN.md"),
        ),
        verify_script: read_and_archive(
            &repo_root().join("scripts/verify-m046-s03.sh"),
            &contract_artifacts.join("verify-m046-s03.sh"),
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

fn assert_tiny_cluster_source_contract(sources: &TinyClusterSources) {
    assert_contains("tiny-cluster/mesh.toml", &sources.manifest, "[package]");
    assert_omits("tiny-cluster/mesh.toml", &sources.manifest, "[cluster]");
    assert_omits("tiny-cluster/mesh.toml", &sources.manifest, "declarations");

    assert_contains(
        "tiny-cluster/main.mpl",
        &sources.main,
        "Node.start_from_env()",
    );
    assert_eq!(
        sources.main.matches("Node.start_from_env()").count(),
        1,
        "tiny-cluster/main.mpl must keep exactly one Node.start_from_env() call"
    );
    for needle in ["HTTP.serve", "/work", "/status", "/health", "Continuity."] {
        assert_omits("tiny-cluster/main.mpl", &sources.main, needle);
    }

    assert_contains(
        "tiny-cluster/work.mpl",
        &sources.work,
        STARTUP_SOURCE_DECLARATION,
    );
    assert_contains("tiny-cluster/work.mpl", &sources.work, "1 + 1");
    assert_omits(
        "tiny-cluster/work.mpl",
        &sources.work,
        "declared_work_runtime_name",
    );
    assert_omits("tiny-cluster/work.mpl", &sources.work, "clustered(work)");
    for needle in [
        "Env.get_int",
        "Timer.sleep",
        "TINY_CLUSTER_WORK_DELAY_MS",
        "MESH_STARTUP_WORK_DELAY_MS",
        "HTTP.serve",
        "/work",
        "/status",
        "/health",
        "Continuity.",
    ] {
        assert_omits("tiny-cluster/work.mpl", &sources.work, needle);
    }

    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        "meshc cluster status",
    );
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        "meshc cluster continuity",
    );
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        "meshc cluster diagnostics",
    );
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        "scripts/fixtures/clustered/tiny-cluster",
    );
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        "cargo run -q -p meshc -- build scripts/fixtures/clustered/tiny-cluster",
    );
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        "cargo run -q -p meshc -- test scripts/fixtures/clustered/tiny-cluster/tests",
    );
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        "Node.start_from_env()",
    );
    assert_contains("tiny-cluster/README.md", &sources.readme, "runtime-owned");
    assert_contains("tiny-cluster/README.md", &sources.readme, "`@cluster`");
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        STARTUP_RUNTIME_NAME_GUIDANCE,
    );
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        STARTUP_AUTOSTART_GUIDANCE,
    );
    assert_contains(
        "tiny-cluster/README.md",
        &sources.readme,
        "does not own HTTP routes, submit handlers, or work-delay seams",
    );
    for needle in [
        "TINY_CLUSTER_WORK_DELAY_MS",
        "declared_work_runtime_name()",
        "clustered(work)",
        "/work",
        "/health",
    ] {
        assert_omits("tiny-cluster/README.md", &sources.readme, needle);
    }

    assert_contains(
        "tiny-cluster/tests/work.test.mpl",
        &sources.work_test,
        "manifest and source stay source-first and route-free",
    );
    assert_contains(
        "tiny-cluster/tests/work.test.mpl",
        &sources.work_test,
        "assert_not_contains(manifest, \"[cluster]\")",
    );
    assert_contains(
        "tiny-cluster/tests/work.test.mpl",
        &sources.work_test,
        "assert_not_contains(work_source, \"Env.get_int\")",
    );
    assert_contains(
        "tiny-cluster/tests/work.test.mpl",
        &sources.work_test,
        "scripts/fixtures/clustered/tiny-cluster/mesh.toml",
    );
    assert_contains(
        "tiny-cluster/tests/work.test.mpl",
        &sources.work_test,
        "scripts/fixtures/clustered/tiny-cluster/README.md",
    );

    assert_contains(
        ".gsd/milestones/M046/slices/S03/S03-PLAN.md",
        &sources.slice_plan,
        "**T04: Replaced the last tiny-cluster failover timing knob with a language-owned startup dispatch window and fail-closed contract guards.**",
    );
    assert_contains(
        ".gsd/milestones/M046/slices/S03/S03-PLAN.md",
        &sources.slice_plan,
        "user-directed `MESH_STARTUP_WORK_DELAY_MS` guidance",
    );
    assert_contains(
        ".gsd/milestones/M046/slices/S03/S03-PLAN.md",
        &sources.slice_plan,
        "without app/user-owned timing seams",
    );

    assert_contains(
        "scripts/verify-m046-s03.sh",
        &sources.verify_script,
        "assert_file_lacks_regex() {",
    );
    assert_contains(
        "scripts/verify-m046-s03.sh",
        &sources.verify_script,
        "tiny-cluster/work.mpl",
    );
    assert_contains(
        "scripts/verify-m046-s03.sh",
        &sources.verify_script,
        "compiler/meshc/tests/e2e_m046_s03.rs",
    );
    assert_contains(
        "scripts/verify-m046-s03.sh",
        &sources.verify_script,
        ".gsd/milestones/M046/slices/S03/S03-PLAN.md",
    );
    assert_contains(
        "scripts/verify-m046-s03.sh",
        &sources.verify_script,
        "MESH_STARTUP_WORK_DELAY_MS",
    );
}

fn build_tiny_cluster_binary(artifacts: &Path) -> PathBuf {
    let binary_dir = artifacts.join("bin");
    std::fs::create_dir_all(&binary_dir).expect("failed to create binary artifact dir");
    let binary_path = binary_dir.join("tiny-cluster");
    let metadata =
        route_free::build_package_binary_to_output(&tiny_cluster_dir(), &binary_path, artifacts);
    assert_eq!(metadata.binary_path, binary_path);
    binary_path
}

fn run_tiny_cluster_package_tests(artifacts: &Path) {
    route_free::run_package_tests(
        &tiny_cluster_dir().join("tests"),
        artifacts,
        "package-tests",
    );
}

fn spawn_tiny_cluster_runtime(
    binary_path: &Path,
    artifacts: &Path,
    log_label: &str,
    node_name: &str,
    cluster_port: u16,
    cluster_role: &str,
    promotion_epoch: u64,
) -> SpawnedProcess {
    route_free::spawn_route_free_runtime(
        binary_path,
        &tiny_cluster_dir(),
        artifacts,
        log_label,
        node_name,
        cluster_port,
        cluster_role,
        promotion_epoch,
        SHARED_COOKIE,
        DISCOVERY_SEED,
    )
}

fn stop_process(spawned: SpawnedProcess) -> StoppedProcess {
    route_free::stop_process(spawned)
}

fn run_meshc_cluster(artifacts: &Path, name: &str, args: &[&str]) -> std::process::Output {
    route_free::run_meshc_cluster(artifacts, name, args, SHARED_COOKIE)
}

fn command_output_text(output: &std::process::Output) -> String {
    route_free::command_output_text(output)
}

fn parse_json_stdout(stdout: &[u8], context: &str) -> Result<Value, String> {
    route_free::parse_json_stdout(stdout, context)
}

fn parse_json_output(
    artifacts: &Path,
    name: &str,
    output: &std::process::Output,
    context: &str,
) -> Value {
    route_free::parse_json_output(artifacts, name, output, context)
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    route_free::panic_payload_to_string(payload)
}

fn required_str(json: &Value, field: &str) -> String {
    route_free::required_str(json, field)
}

fn required_bool(json: &Value, field: &str) -> bool {
    route_free::required_bool(json, field)
}

fn required_u64(json: &Value, field: &str) -> u64 {
    route_free::required_u64(json, field)
}

fn required_string_list(json: &Value, field: &str) -> Vec<String> {
    route_free::required_string_list(json, field)
}

fn sorted(values: &[String]) -> Vec<String> {
    route_free::sorted(values)
}

fn find_record_for_runtime_name<'a>(list_json: &'a Value, runtime_name: &str) -> Option<&'a Value> {
    route_free::find_record_for_runtime_name(list_json, runtime_name)
}

fn record_for_runtime_name<'a>(list_json: &'a Value, runtime_name: &str) -> &'a Value {
    route_free::record_for_runtime_name(list_json, runtime_name)
}

fn count_records_for_runtime_name(list_json: &Value, runtime_name: &str) -> usize {
    route_free::count_records_for_runtime_name(list_json, runtime_name)
}

fn diagnostic_entries_for_request<'a>(snapshot: &'a Value, request_key: &str) -> Vec<&'a Value> {
    route_free::diagnostic_entries_for_request(snapshot, request_key)
}

fn wait_for_cluster_status_membership(
    artifacts: &Path,
    name: &str,
    node_name: &str,
    expected_peer_nodes: &[String],
    expected_nodes: &[String],
    expected_role: &str,
    expected_epoch: u64,
) -> Value {
    route_free::wait_for_cluster_status_membership(
        artifacts,
        name,
        node_name,
        expected_peer_nodes,
        expected_nodes,
        expected_role,
        expected_epoch,
        &["local_only", "healthy"],
        SHARED_COOKIE,
    )
}

fn wait_for_runtime_name_discovered_with_label(
    artifacts: &Path,
    name: &str,
    node_name: &str,
    runtime_name: &str,
) -> Value {
    route_free::wait_for_runtime_name_discovered_with_label(
        artifacts,
        name,
        node_name,
        runtime_name,
        SHARED_COOKIE,
    )
}

fn wait_for_continuity_record_completed(
    artifacts: &Path,
    name: &str,
    node_name: &str,
    request_key: &str,
    expected_runtime_name: &str,
) -> Value {
    route_free::wait_for_continuity_record_completed(
        artifacts,
        name,
        node_name,
        request_key,
        expected_runtime_name,
        SHARED_COOKIE,
    )
}

fn wait_for_startup_diagnostics(
    artifacts: &Path,
    primary_node: &str,
    standby_node: &str,
    request_key: &str,
) -> (Value, Value) {
    route_free::wait_for_startup_diagnostics(
        artifacts,
        primary_node,
        standby_node,
        request_key,
        SHARED_COOKIE,
    )
}

fn wait_for_cluster_status_matching<F>(
    artifacts: &Path,
    name: &str,
    node_name: &str,
    predicate_description: &str,
    predicate: F,
) -> Value
where
    F: Fn(&Value) -> bool,
{
    route_free::wait_for_cluster_status_matching(
        artifacts,
        name,
        node_name,
        predicate_description,
        SHARED_COOKIE,
        predicate,
    )
}

fn wait_for_continuity_list_matching<F>(
    artifacts: &Path,
    name: &str,
    node_name: &str,
    predicate_description: &str,
    predicate: F,
) -> Value
where
    F: Fn(&Value) -> bool,
{
    route_free::wait_for_continuity_list_matching(
        artifacts,
        name,
        node_name,
        predicate_description,
        SHARED_COOKIE,
        predicate,
    )
}

fn wait_for_continuity_record_matching<F>(
    artifacts: &Path,
    name: &str,
    node_name: &str,
    request_key: &str,
    predicate_description: &str,
    predicate: F,
) -> Value
where
    F: Fn(&Value) -> bool,
{
    route_free::wait_for_continuity_record_matching(
        artifacts,
        name,
        node_name,
        request_key,
        predicate_description,
        SHARED_COOKIE,
        predicate,
    )
}

fn wait_for_diagnostics_matching<F>(
    artifacts: &Path,
    name: &str,
    node_name: &str,
    predicate_description: &str,
    predicate: F,
) -> Value
where
    F: Fn(&Value) -> bool,
{
    route_free::wait_for_diagnostics_matching(
        artifacts,
        name,
        node_name,
        predicate_description,
        SHARED_COOKIE,
        predicate,
    )
}

fn status_matches(
    json: &Value,
    expected_local_node: &str,
    expected_peer_nodes: &[String],
    expected_nodes: &[String],
    expected_role: &str,
    expected_epoch: u64,
    allowed_health: &[&str],
) -> bool {
    let replication_health = required_str(&json["authority"], "replication_health");
    required_str(&json["membership"], "local_node") == expected_local_node
        && sorted(&required_string_list(&json["membership"], "peer_nodes"))
            == sorted(expected_peer_nodes)
        && sorted(&required_string_list(&json["membership"], "nodes")) == sorted(expected_nodes)
        && required_str(&json["authority"], "cluster_role") == expected_role
        && required_u64(&json["authority"], "promotion_epoch") == expected_epoch
        && allowed_health.contains(&replication_health.as_str())
}

fn pending_record_matches(
    record: &Value,
    request_key: Option<&str>,
    attempt_id: Option<&str>,
    expected_owner: &str,
    expected_replica: &str,
    expected_cluster_role: &str,
    expected_epoch: u64,
    allowed_replica_statuses: &[&str],
) -> bool {
    let actual_request_key = required_str(record, "request_key");
    let actual_attempt_id = required_str(record, "attempt_id");
    let actual_replica_status = required_str(record, "replica_status");

    request_key.map_or(!actual_request_key.is_empty(), |expected| {
        actual_request_key == expected
    }) && attempt_id.map_or(!actual_attempt_id.is_empty(), |expected| {
        actual_attempt_id == expected
    }) && required_str(record, "declared_handler_runtime_name") == STARTUP_RUNTIME_NAME
        && required_str(record, "phase") == "submitted"
        && required_str(record, "result") == "pending"
        && required_str(record, "owner_node") == expected_owner
        && required_str(record, "replica_node") == expected_replica
        && required_str(record, "cluster_role") == expected_cluster_role
        && required_u64(record, "promotion_epoch") == expected_epoch
        && required_str(record, "execution_node").is_empty()
        && required_str(record, "error").is_empty()
        && allowed_replica_statuses.contains(&actual_replica_status.as_str())
}

fn completed_record_matches(
    record: &Value,
    request_key: &str,
    attempt_id: &str,
    expected_owner: &str,
    expected_replica: &str,
    expected_execution_node: &str,
    expected_cluster_role: &str,
    expected_epoch: u64,
) -> bool {
    required_str(record, "request_key") == request_key
        && required_str(record, "attempt_id") == attempt_id
        && required_str(record, "declared_handler_runtime_name") == STARTUP_RUNTIME_NAME
        && required_str(record, "phase") == "completed"
        && required_str(record, "result") == "succeeded"
        && required_str(record, "owner_node") == expected_owner
        && required_str(record, "replica_node") == expected_replica
        && required_str(record, "execution_node") == expected_execution_node
        && required_str(record, "cluster_role") == expected_cluster_role
        && required_u64(record, "promotion_epoch") == expected_epoch
        && required_str(record, "error").is_empty()
}

fn metadata_value<'a>(entry: &'a Value, key: &str) -> Option<&'a str> {
    entry["metadata"].as_array().and_then(|metadata| {
        metadata.iter().find_map(|item| {
            (item["key"].as_str() == Some(key))
                .then(|| item["value"].as_str())
                .flatten()
        })
    })
}

fn has_automatic_promotion(snapshot: &Value, disconnected_node: &str) -> bool {
    snapshot["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("missing diagnostics entries array in {snapshot}"))
        .iter()
        .any(|entry| {
            entry["transition"].as_str() == Some("automatic_promotion")
                && entry["cluster_role"].as_str() == Some("primary")
                && entry["promotion_epoch"].as_u64() == Some(1)
                && (entry["reason"].as_str() == Some(&format!("peer_lost:{disconnected_node}"))
                    || metadata_value(entry, "disconnected_node") == Some(disconnected_node))
                && metadata_value(entry, "previous_epoch") == Some("0")
        })
}

fn automatic_recovery_attempt_id(
    snapshot: &Value,
    request_key: &str,
    previous_attempt_id: &str,
) -> Option<String> {
    diagnostic_entries_for_request(snapshot, request_key)
        .iter()
        .find_map(|entry| {
            (entry["transition"].as_str() == Some("automatic_recovery")
                && entry["request_key"].as_str() == Some(request_key)
                && metadata_value(entry, "previous_attempt_id") == Some(previous_attempt_id)
                && metadata_value(entry, "runtime_name") == Some(STARTUP_RUNTIME_NAME))
            .then(|| entry["attempt_id"].as_str())
            .flatten()
            .map(str::to_string)
        })
}

fn has_recovery_rollover(
    snapshot: &Value,
    request_key: &str,
    previous_attempt_id: &str,
    next_attempt_id: &str,
    owner_node: &str,
) -> bool {
    diagnostic_entries_for_request(snapshot, request_key)
        .iter()
        .any(|entry| {
            entry["transition"].as_str() == Some("recovery_rollover")
                && entry["request_key"].as_str() == Some(request_key)
                && entry["attempt_id"].as_str() == Some(next_attempt_id)
                && entry["owner_node"].as_str() == Some(owner_node)
                && entry["cluster_role"].as_str() == Some("primary")
                && entry["promotion_epoch"].as_u64() == Some(1)
                && metadata_value(entry, "previous_attempt_id") == Some(previous_attempt_id)
        })
}

fn has_fenced_rejoin(snapshot: &Value, request_key: &str, attempt_id: &str) -> bool {
    diagnostic_entries_for_request(snapshot, request_key)
        .iter()
        .any(|entry| {
            entry["transition"].as_str() == Some("fenced_rejoin")
                && entry["request_key"].as_str() == Some(request_key)
                && entry["attempt_id"].as_str() == Some(attempt_id)
                && entry["cluster_role"].as_str() == Some("standby")
                && entry["promotion_epoch"].as_u64() == Some(1)
                && metadata_value(entry, "previous_role") == Some("primary")
                && metadata_value(entry, "previous_epoch") == Some("0")
        })
}

fn startup_request_key() -> String {
    format!("startup::{STARTUP_RUNTIME_NAME}")
}

fn assert_log_contains(logs: &StoppedProcess, needle: &str) {
    assert!(
        logs.combined.contains(needle),
        "expected log `{needle}`\nstdout:\n{}\nstderr:\n{}",
        logs.stdout,
        logs.stderr,
    );
}

fn assert_log_absent(logs: &StoppedProcess, needle: &str) {
    assert!(
        !logs.combined.contains(needle),
        "did not expect log `{needle}`\nstdout:\n{}\nstderr:\n{}",
        logs.stdout,
        logs.stderr,
    );
}

#[test]
fn m046_s03_tiny_cluster_failover_helpers_accept_preparing_and_mirrored_pending_truth() {
    let request_key = startup_request_key();
    let preparing = json!({
        "request_key": request_key,
        "attempt_id": "attempt-0",
        "declared_handler_runtime_name": STARTUP_RUNTIME_NAME,
        "phase": "submitted",
        "result": "pending",
        "owner_node": "tiny-cluster-primary@127.0.0.1:4370",
        "replica_node": "tiny-cluster-standby@[::1]:4370",
        "cluster_role": "standby",
        "promotion_epoch": 0,
        "execution_node": "",
        "replica_status": "preparing",
        "error": ""
    });
    assert!(pending_record_matches(
        &preparing,
        None,
        None,
        "tiny-cluster-primary@127.0.0.1:4370",
        "tiny-cluster-standby@[::1]:4370",
        "standby",
        0,
        &["preparing", "mirrored"],
    ));

    let mirrored = json!({
        "request_key": startup_request_key(),
        "attempt_id": "attempt-0",
        "declared_handler_runtime_name": STARTUP_RUNTIME_NAME,
        "phase": "submitted",
        "result": "pending",
        "owner_node": "tiny-cluster-primary@127.0.0.1:4370",
        "replica_node": "tiny-cluster-standby@[::1]:4370",
        "cluster_role": "standby",
        "promotion_epoch": 0,
        "execution_node": "",
        "replica_status": "mirrored",
        "error": ""
    });
    assert!(pending_record_matches(
        &mirrored,
        Some(request_key.as_str()),
        Some("attempt-0"),
        "tiny-cluster-primary@127.0.0.1:4370",
        "tiny-cluster-standby@[::1]:4370",
        "standby",
        0,
        &["mirrored"],
    ));

    let completed = json!({
        "request_key": startup_request_key(),
        "attempt_id": "attempt-0",
        "declared_handler_runtime_name": STARTUP_RUNTIME_NAME,
        "phase": "completed",
        "result": "succeeded",
        "owner_node": "tiny-cluster-primary@127.0.0.1:4370",
        "replica_node": "tiny-cluster-standby@[::1]:4370",
        "cluster_role": "standby",
        "promotion_epoch": 0,
        "execution_node": "tiny-cluster-primary@127.0.0.1:4370",
        "replica_status": "mirrored",
        "error": ""
    });
    assert!(!pending_record_matches(
        &completed,
        Some(request_key.as_str()),
        Some("attempt-0"),
        "tiny-cluster-primary@127.0.0.1:4370",
        "tiny-cluster-standby@[::1]:4370",
        "standby",
        0,
        &["mirrored"],
    ));
}

#[test]
fn m046_s03_tiny_cluster_failover_helpers_reject_malformed_cluster_json() {
    let error = parse_json_stdout(b"{not-json}", "continuity response").unwrap_err();
    assert!(error.contains("continuity response returned invalid JSON"));
}

#[test]
fn m046_s03_tiny_cluster_failover_proves_promotion_recovery_completion_and_fenced_rejoin_from_cli_surfaces(
) {
    let artifacts = artifact_dir("tiny-cluster-failover-runtime-truth");
    let sources = load_tiny_cluster_sources(&artifacts);
    assert_tiny_cluster_source_contract(&sources);
    let binary_path = build_tiny_cluster_binary(&artifacts);
    let cluster_port = dual_stack_cluster_port_for_primary_owned_startup();
    let primary_node = format!("tiny-cluster-primary@{LOOPBACK_V4}:{cluster_port}");
    let standby_node = format!("tiny-cluster-standby@[{}]:{}", LOOPBACK_V6, cluster_port);
    let full_membership = vec![primary_node.clone(), standby_node.clone()];
    let no_peers: Vec<String> = Vec::new();
    let standby_only_membership = vec![standby_node.clone()];

    write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "package_dir": tiny_cluster_dir().display().to_string(),
            "binary_path": binary_path.display().to_string(),
            "cluster_port": cluster_port,
            "startup_runtime_name": STARTUP_RUNTIME_NAME,
            "startup_pending_window_source": STARTUP_PENDING_WINDOW_SOURCE,
            "request_key": Value::Null,
            "initial_attempt_id": Value::Null,
            "failover_attempt_id": Value::Null,
            "primary_node": primary_node,
            "standby_node": standby_node,
        }),
    );

    let mut primary_run1 = Some(spawn_tiny_cluster_runtime(
        &binary_path,
        &artifacts,
        "primary-run1",
        &primary_node,
        cluster_port,
        "primary",
        0,
    ));
    let mut standby_run1 = Some(spawn_tiny_cluster_runtime(
        &binary_path,
        &artifacts,
        "standby-run1",
        &standby_node,
        cluster_port,
        "standby",
        0,
    ));
    let mut primary_run1_logs: Option<StoppedProcess> = None;
    let mut primary_run2: Option<SpawnedProcess> = None;
    let mut request_key: Option<String> = None;
    let mut initial_attempt_id: Option<String> = None;
    let mut failover_attempt_id: Option<String> = None;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wait_for_cluster_status_membership(
            &artifacts,
            "pre-kill-status-primary",
            &primary_node,
            std::slice::from_ref(&standby_node),
            &full_membership,
            "primary",
            0,
        );
        wait_for_cluster_status_membership(
            &artifacts,
            "pre-kill-status-standby",
            &standby_node,
            std::slice::from_ref(&primary_node),
            &full_membership,
            "standby",
            0,
        );

        let primary_list = wait_for_continuity_list_matching(
            &artifacts,
            "pre-kill-continuity-list-primary",
            &primary_node,
            "pre-kill primary-owned pending startup record",
            |json| {
                let Some(record) = find_record_for_runtime_name(json, STARTUP_RUNTIME_NAME) else {
                    return false;
                };
                pending_record_matches(
                    record,
                    None,
                    None,
                    &primary_node,
                    &standby_node,
                    "primary",
                    0,
                    &["preparing", "mirrored"],
                )
            },
        );
        let standby_list = wait_for_continuity_list_matching(
            &artifacts,
            "pre-kill-continuity-list-standby",
            &standby_node,
            "pre-kill mirrored standby pending startup record",
            |json| {
                let Some(record) = find_record_for_runtime_name(json, STARTUP_RUNTIME_NAME) else {
                    return false;
                };
                pending_record_matches(
                    record,
                    None,
                    None,
                    &primary_node,
                    &standby_node,
                    "standby",
                    0,
                    &["mirrored"],
                )
            },
        );
        assert_eq!(required_u64(&primary_list, "total_records"), 1);
        assert_eq!(required_u64(&standby_list, "total_records"), 1);
        assert!(!required_bool(&primary_list, "truncated"));
        assert!(!required_bool(&standby_list, "truncated"));
        assert_eq!(
            count_records_for_runtime_name(&primary_list, STARTUP_RUNTIME_NAME),
            1
        );
        assert_eq!(
            count_records_for_runtime_name(&standby_list, STARTUP_RUNTIME_NAME),
            1
        );

        let primary_list_record = record_for_runtime_name(&primary_list, STARTUP_RUNTIME_NAME);
        let standby_list_record = record_for_runtime_name(&standby_list, STARTUP_RUNTIME_NAME);
        let selected_request_key = required_str(primary_list_record, "request_key");
        let selected_attempt_id = required_str(primary_list_record, "attempt_id");
        assert_eq!(
            selected_request_key,
            required_str(standby_list_record, "request_key")
        );
        assert_eq!(
            selected_attempt_id,
            required_str(standby_list_record, "attempt_id")
        );
        assert_eq!(
            required_str(primary_list_record, "owner_node"),
            primary_node
        );
        assert_eq!(
            required_str(primary_list_record, "replica_node"),
            standby_node
        );
        assert_eq!(
            required_str(standby_list_record, "owner_node"),
            primary_node
        );
        assert_eq!(
            required_str(standby_list_record, "replica_node"),
            standby_node
        );
        assert_eq!(
            required_str(standby_list_record, "replica_status"),
            "mirrored"
        );
        request_key = Some(selected_request_key.clone());
        initial_attempt_id = Some(selected_attempt_id.clone());

        wait_for_continuity_record_matching(
            &artifacts,
            "pre-kill-continuity-primary",
            &primary_node,
            &selected_request_key,
            "pre-kill primary continuity pending truth",
            |json| {
                pending_record_matches(
                    &json["record"],
                    Some(&selected_request_key),
                    Some(&selected_attempt_id),
                    &primary_node,
                    &standby_node,
                    "primary",
                    0,
                    &["preparing", "mirrored"],
                )
            },
        );
        let standby_prekill = wait_for_continuity_record_matching(
            &artifacts,
            "pre-kill-continuity-standby",
            &standby_node,
            &selected_request_key,
            "pre-kill standby mirrored pending truth",
            |json| {
                pending_record_matches(
                    &json["record"],
                    Some(&selected_request_key),
                    Some(&selected_attempt_id),
                    &primary_node,
                    &standby_node,
                    "standby",
                    0,
                    &["mirrored"],
                )
            },
        );
        assert_eq!(
            required_str(&standby_prekill["record"], "replica_status"),
            "mirrored"
        );

        primary_run1_logs = primary_run1.take().map(stop_process);
        write_artifact(
            &artifacts.join("primary-run1.combined.log"),
            primary_run1_logs
                .as_ref()
                .expect("primary run1 logs missing after forced owner stop")
                .combined
                .as_str(),
        );
        write_json_artifact(
            &artifacts.join("scenario-meta.json"),
            &json!({
                "package_dir": tiny_cluster_dir().display().to_string(),
                "binary_path": binary_path.display().to_string(),
                "cluster_port": cluster_port,
                "startup_runtime_name": STARTUP_RUNTIME_NAME,
                "startup_pending_window_source": STARTUP_PENDING_WINDOW_SOURCE,
                "request_key": selected_request_key,
                "initial_attempt_id": selected_attempt_id,
                "failover_attempt_id": Value::Null,
                "primary_node": primary_node,
                "standby_node": standby_node,
            }),
        );

        wait_for_cluster_status_matching(
            &artifacts,
            "post-kill-status-standby",
            &standby_node,
            "post-kill standby promotion truth",
            |json| {
                status_matches(
                    json,
                    &standby_node,
                    &no_peers,
                    &standby_only_membership,
                    "primary",
                    1,
                    &["local_only"],
                )
            },
        );

        let post_kill_diagnostics = wait_for_diagnostics_matching(
            &artifacts,
            "post-kill-diagnostics-standby",
            &standby_node,
            "post-kill standby promotion/recovery diagnostics truth",
            |snapshot| {
                if !has_automatic_promotion(snapshot, &primary_node) {
                    return false;
                }
                let Some(next_attempt_id) = automatic_recovery_attempt_id(
                    snapshot,
                    request_key.as_deref().unwrap(),
                    initial_attempt_id.as_deref().unwrap(),
                ) else {
                    return false;
                };
                has_recovery_rollover(
                    snapshot,
                    request_key.as_deref().unwrap(),
                    initial_attempt_id.as_deref().unwrap(),
                    &next_attempt_id,
                    &standby_node,
                )
            },
        );
        assert!(!required_bool(&post_kill_diagnostics, "truncated"));
        let recovered_attempt_id = automatic_recovery_attempt_id(
            &post_kill_diagnostics,
            request_key.as_deref().unwrap(),
            initial_attempt_id.as_deref().unwrap(),
        )
        .expect("post-kill diagnostics should expose the automatic recovery attempt id");
        failover_attempt_id = Some(recovered_attempt_id.clone());

        let post_kill_continuity = wait_for_continuity_record_matching(
            &artifacts,
            "post-kill-continuity-standby-completed",
            &standby_node,
            request_key.as_deref().unwrap(),
            "post-kill standby completion truth",
            |json| {
                completed_record_matches(
                    &json["record"],
                    request_key.as_deref().unwrap(),
                    failover_attempt_id.as_deref().unwrap(),
                    &standby_node,
                    "",
                    &standby_node,
                    "primary",
                    1,
                )
            },
        );
        assert_eq!(
            required_str(&post_kill_continuity["record"], "owner_node"),
            standby_node
        );
        assert_eq!(
            required_str(&post_kill_continuity["record"], "execution_node"),
            standby_node
        );

        primary_run2 = Some(spawn_tiny_cluster_runtime(
            &binary_path,
            &artifacts,
            "primary-run2",
            &primary_node,
            cluster_port,
            "primary",
            0,
        ));

        wait_for_cluster_status_matching(
            &artifacts,
            "post-rejoin-status-primary",
            &primary_node,
            "post-rejoin stale-primary fenced status truth",
            |json| {
                status_matches(
                    json,
                    &primary_node,
                    std::slice::from_ref(&standby_node),
                    &full_membership,
                    "standby",
                    1,
                    &["healthy"],
                )
            },
        );
        wait_for_cluster_status_matching(
            &artifacts,
            "post-rejoin-status-standby",
            &standby_node,
            "post-rejoin promoted-standby status truth",
            |json| {
                status_matches(
                    json,
                    &standby_node,
                    std::slice::from_ref(&primary_node),
                    &full_membership,
                    "primary",
                    1,
                    &["local_only", "healthy"],
                )
            },
        );

        let post_rejoin_diagnostics = wait_for_diagnostics_matching(
            &artifacts,
            "post-rejoin-diagnostics-primary",
            &primary_node,
            "post-rejoin fenced rejoin diagnostics truth",
            |snapshot| {
                has_fenced_rejoin(
                    snapshot,
                    request_key.as_deref().unwrap(),
                    failover_attempt_id.as_deref().unwrap(),
                )
            },
        );
        assert!(!required_bool(&post_rejoin_diagnostics, "truncated"));

        wait_for_continuity_record_matching(
            &artifacts,
            "post-rejoin-continuity-primary",
            &primary_node,
            request_key.as_deref().unwrap(),
            "post-rejoin stale-primary continuity truth",
            |json| {
                completed_record_matches(
                    &json["record"],
                    request_key.as_deref().unwrap(),
                    failover_attempt_id.as_deref().unwrap(),
                    &standby_node,
                    "",
                    &standby_node,
                    "standby",
                    1,
                )
            },
        );
        wait_for_continuity_record_matching(
            &artifacts,
            "post-rejoin-continuity-standby",
            &standby_node,
            request_key.as_deref().unwrap(),
            "post-rejoin promoted-standby continuity truth",
            |json| {
                completed_record_matches(
                    &json["record"],
                    request_key.as_deref().unwrap(),
                    failover_attempt_id.as_deref().unwrap(),
                    &standby_node,
                    "",
                    &standby_node,
                    "primary",
                    1,
                )
            },
        );

        write_json_artifact(
            &artifacts.join("scenario-meta.json"),
            &json!({
                "package_dir": tiny_cluster_dir().display().to_string(),
                "binary_path": binary_path.display().to_string(),
                "cluster_port": cluster_port,
                "startup_runtime_name": STARTUP_RUNTIME_NAME,
                "startup_pending_window_source": STARTUP_PENDING_WINDOW_SOURCE,
                "request_key": request_key.as_deref().unwrap(),
                "initial_attempt_id": initial_attempt_id.as_deref().unwrap(),
                "failover_attempt_id": failover_attempt_id.as_deref().unwrap(),
                "primary_node": primary_node,
                "standby_node": standby_node,
            }),
        );
    }));

    let standby_logs = stop_process(
        standby_run1
            .take()
            .expect("standby process missing during cleanup"),
    );
    write_artifact(
        &artifacts.join("standby-run1.combined.log"),
        &standby_logs.combined,
    );

    let primary_run2_logs = primary_run2.take().map(stop_process);
    if let Some(logs) = primary_run2_logs.as_ref() {
        write_artifact(&artifacts.join("primary-run2.combined.log"), &logs.combined);
    }

    let primary_run1_logs = match primary_run1_logs {
        Some(logs) => logs,
        None => stop_process(
            primary_run1
                .take()
                .expect("primary run1 missing during cleanup"),
        ),
    };
    write_artifact(
        &artifacts.join("primary-run1.combined.log"),
        &primary_run1_logs.combined,
    );

    if let Err(payload) = result {
        let run2_stdout = primary_run2_logs
            .as_ref()
            .map(|logs| logs.stdout.as_str())
            .unwrap_or("");
        let run2_stderr = primary_run2_logs
            .as_ref()
            .map(|logs| logs.stderr.as_str())
            .unwrap_or("");
        panic!(
            "tiny-cluster failover assertions failed: {}\nartifacts: {}\nprimary run1 stdout:\n{}\nprimary run1 stderr:\n{}\nprimary run2 stdout:\n{}\nprimary run2 stderr:\n{}\nstandby stdout:\n{}\nstandby stderr:\n{}",
            panic_payload_to_string(payload),
            artifacts.display(),
            primary_run1_logs.stdout,
            primary_run1_logs.stderr,
            run2_stdout,
            run2_stderr,
            standby_logs.stdout,
            standby_logs.stderr,
        );
    }

    let request_key = request_key
        .as_deref()
        .expect("startup request key missing after successful run");
    let initial_attempt_id = initial_attempt_id
        .as_deref()
        .expect("initial startup attempt id missing after successful run");
    let failover_attempt_id = failover_attempt_id
        .as_deref()
        .expect("failover attempt id missing after successful run");
    let primary_run2_logs =
        primary_run2_logs.expect("primary run2 logs missing after successful run");

    for required in [
        "scenario-meta.json",
        "pre-kill-status-primary.json",
        "pre-kill-status-standby.json",
        "pre-kill-continuity-list-primary.json",
        "pre-kill-continuity-list-standby.json",
        "pre-kill-continuity-primary.json",
        "pre-kill-continuity-standby.json",
        "post-kill-status-standby.json",
        "post-kill-diagnostics-standby.json",
        "post-kill-continuity-standby-completed.json",
        "post-rejoin-status-primary.json",
        "post-rejoin-status-standby.json",
        "post-rejoin-diagnostics-primary.json",
        "post-rejoin-continuity-primary.json",
        "post-rejoin-continuity-standby.json",
        "primary-run1.stdout.log",
        "primary-run1.stderr.log",
        "standby-run1.stdout.log",
        "standby-run1.stderr.log",
        "primary-run2.stdout.log",
        "primary-run2.stderr.log",
    ] {
        assert!(
            artifacts.join(required).exists(),
            "missing retained failover artifact {} in {}",
            required,
            artifacts.display(),
        );
    }

    for logs in [&primary_run1_logs, &primary_run2_logs, &standby_logs] {
        assert_log_absent(logs, SHARED_COOKIE);
    }

    assert_log_contains(
        &primary_run1_logs,
        &format!("[tiny-cluster] runtime bootstrap mode=cluster node={primary_node}"),
    );
    assert_log_contains(
        &standby_logs,
        &format!("[tiny-cluster] runtime bootstrap mode=cluster node={standby_node}"),
    );
    assert_log_contains(
        &primary_run2_logs,
        &format!("[tiny-cluster] runtime bootstrap mode=cluster node={primary_node}"),
    );
    assert_log_contains(
        &primary_run1_logs,
        &format!(
            "[mesh-rt startup] transition=startup_dispatch_window runtime_name={STARTUP_RUNTIME_NAME} request_key={request_key}"
        ),
    );
    assert_log_absent(&primary_run1_logs, "transition=startup_dispatch_delay");
    assert_log_contains(
        &standby_logs,
        &format!(
            "[mesh-rt continuity] transition=automatic_promotion disconnected_node={} previous_epoch=0 next_epoch=1",
            primary_node
        ),
    );
    assert_log_contains(
        &standby_logs,
        &format!(
            "[mesh-rt continuity] transition=automatic_recovery request_key={request_key} previous_attempt_id={initial_attempt_id} next_attempt_id={failover_attempt_id} runtime_name={STARTUP_RUNTIME_NAME}"
        ),
    );
    assert_log_contains(
        &standby_logs,
        &format!(
            "[mesh-rt continuity] transition=recovery_rollover request_key={request_key} previous_attempt_id={initial_attempt_id} next_attempt_id={failover_attempt_id}"
        ),
    );
    assert_log_contains(
        &standby_logs,
        &format!(
            "[mesh-rt continuity] transition=completed request_key={request_key} attempt_id={failover_attempt_id} execution={standby_node}"
        ),
    );
    assert_log_absent(
        &primary_run1_logs,
        &format!(
            "[mesh-rt continuity] transition=completed request_key={request_key} attempt_id={initial_attempt_id}"
        ),
    );
    assert_log_contains(
        &primary_run2_logs,
        "[mesh-rt continuity] transition=fenced_rejoin",
    );
    assert_log_absent(
        &primary_run2_logs,
        &format!(
            "[mesh-rt continuity] transition=completed request_key={request_key} attempt_id={failover_attempt_id} execution={primary_node}"
        ),
    );
}

#[test]
fn m046_s03_tiny_cluster_package_contract_remains_source_first_and_route_free() {
    let artifacts = artifact_dir("tiny-cluster-package-contract");
    let sources = load_tiny_cluster_sources(&artifacts);
    assert_tiny_cluster_source_contract(&sources);
}

#[test]
fn m046_s03_tiny_cluster_fixture_helper_fails_closed_on_missing_required_file() {
    let artifacts = artifact_dir("tiny-cluster-fixture-helper-missing-file");
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let broken_root = temp_dir.path().join("broken-tiny-cluster");
    route_free::archive_directory_tree(&tiny_cluster_dir(), &broken_root);

    let removed_file = broken_root.join("work.mpl");
    fs::remove_file(&removed_file).unwrap_or_else(|error| {
        panic!(
            "failed to remove broken tiny-cluster fixture file {}: {error}",
            removed_file.display()
        )
    });

    let error = route_free::validate_tiny_cluster_fixture_root(&broken_root).expect_err(
        "missing fixture files should fail before the retained tiny-cluster rail starts",
    );
    write_artifact(&artifacts.join("fixture-error.txt"), &error);
    assert_contains(
        "broken tiny-cluster fixture error",
        &error,
        &broken_root.display().to_string(),
    );
    assert_contains("broken tiny-cluster fixture error", &error, "work.mpl");
    assert_contains(
        "broken tiny-cluster fixture error",
        &error,
        route_free::TINY_CLUSTER_FIXTURE_ROOT_RELATIVE,
    );
}

#[test]
fn m046_s03_tiny_cluster_package_builds_and_runs_repo_smoke_rail() {
    let artifacts = artifact_dir("tiny-cluster-package-build-and-test");
    let sources = load_tiny_cluster_sources(&artifacts);
    assert_tiny_cluster_source_contract(&sources);
    let _binary = build_tiny_cluster_binary(&artifacts);
    run_tiny_cluster_package_tests(&artifacts);
}

#[test]
fn m046_s03_tiny_cluster_startup_dedupes_and_surfaces_runtime_truth_on_two_nodes() {
    let artifacts = artifact_dir("tiny-cluster-startup-two-node");
    let sources = load_tiny_cluster_sources(&artifacts);
    assert_tiny_cluster_source_contract(&sources);
    let binary_path = build_tiny_cluster_binary(&artifacts);
    let cluster_port = dual_stack_cluster_port();
    let primary_node = format!("tiny-cluster-primary@{LOOPBACK_V4}:{cluster_port}");
    let standby_node = format!("tiny-cluster-standby@[{}]:{}", LOOPBACK_V6, cluster_port);
    let expected_nodes = vec![primary_node.clone(), standby_node.clone()];

    write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "package_dir": tiny_cluster_dir().display().to_string(),
            "binary_path": binary_path.display().to_string(),
            "cluster_port": cluster_port,
            "startup_runtime_name": STARTUP_RUNTIME_NAME,
            "startup_pending_window_source": STARTUP_PENDING_WINDOW_SOURCE,
            "primary_node": primary_node,
            "standby_node": standby_node,
        }),
    );

    let primary_proc = spawn_tiny_cluster_runtime(
        &binary_path,
        &artifacts,
        "primary",
        &primary_node,
        cluster_port,
        "primary",
        0,
    );
    let standby_proc = spawn_tiny_cluster_runtime(
        &binary_path,
        &artifacts,
        "standby",
        &standby_node,
        cluster_port,
        "standby",
        0,
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wait_for_cluster_status_membership(
            &artifacts,
            "cluster-status-primary",
            &primary_node,
            std::slice::from_ref(&standby_node),
            &expected_nodes,
            "primary",
            0,
        );
        wait_for_cluster_status_membership(
            &artifacts,
            "cluster-status-standby",
            &standby_node,
            std::slice::from_ref(&primary_node),
            &expected_nodes,
            "standby",
            0,
        );

        let primary_list = wait_for_runtime_name_discovered_with_label(
            &artifacts,
            "cluster-continuity-list-primary",
            &primary_node,
            STARTUP_RUNTIME_NAME,
        );
        let standby_list = wait_for_runtime_name_discovered_with_label(
            &artifacts,
            "cluster-continuity-list-standby",
            &standby_node,
            STARTUP_RUNTIME_NAME,
        );
        assert_eq!(required_u64(&primary_list, "total_records"), 1);
        assert_eq!(required_u64(&standby_list, "total_records"), 1);
        assert!(!required_bool(&primary_list, "truncated"));
        assert!(!required_bool(&standby_list, "truncated"));
        assert_eq!(
            count_records_for_runtime_name(&primary_list, STARTUP_RUNTIME_NAME),
            1
        );
        assert_eq!(
            count_records_for_runtime_name(&standby_list, STARTUP_RUNTIME_NAME),
            1
        );

        let request_key = required_str(
            record_for_runtime_name(&primary_list, STARTUP_RUNTIME_NAME),
            "request_key",
        );
        assert_eq!(
            request_key,
            required_str(
                record_for_runtime_name(&standby_list, STARTUP_RUNTIME_NAME),
                "request_key",
            )
        );

        let human_list = run_meshc_cluster(
            &artifacts,
            "cluster-continuity-list-primary-human",
            &["cluster", "continuity", &primary_node],
        );
        assert!(
            human_list.status.success(),
            "human continuity list should succeed:\n{}",
            command_output_text(&human_list)
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

        let primary_continuity = wait_for_continuity_record_completed(
            &artifacts,
            "cluster-continuity-primary-completed",
            &primary_node,
            &request_key,
            STARTUP_RUNTIME_NAME,
        );
        let standby_continuity = wait_for_continuity_record_completed(
            &artifacts,
            "cluster-continuity-standby-completed",
            &standby_node,
            &request_key,
            STARTUP_RUNTIME_NAME,
        );

        let human_single = run_meshc_cluster(
            &artifacts,
            "cluster-continuity-single-primary-human",
            &["cluster", "continuity", &primary_node, &request_key],
        );
        assert!(
            human_single.status.success(),
            "human continuity single-record output should succeed:\n{}",
            command_output_text(&human_single)
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
        let owner_node = required_str(primary_record, "owner_node");
        let replica_node = required_str(primary_record, "replica_node");
        assert_eq!(owner_node, required_str(standby_record, "owner_node"));
        assert_eq!(replica_node, required_str(standby_record, "replica_node"));
        assert_eq!(required_str(primary_record, "request_key"), request_key);
        assert_eq!(required_str(standby_record, "request_key"), request_key);
        assert_eq!(
            required_str(primary_record, "declared_handler_runtime_name"),
            STARTUP_RUNTIME_NAME
        );
        assert_eq!(
            required_str(standby_record, "declared_handler_runtime_name"),
            STARTUP_RUNTIME_NAME
        );
        assert_eq!(required_str(primary_record, "phase"), "completed");
        assert_eq!(required_str(standby_record, "phase"), "completed");
        assert_eq!(required_str(primary_record, "result"), "succeeded");
        assert_eq!(required_str(standby_record, "result"), "succeeded");
        assert!(expected_nodes.contains(&owner_node));
        assert!(expected_nodes.contains(&replica_node));
        assert_ne!(owner_node, replica_node);
        assert_eq!(required_str(primary_record, "execution_node"), owner_node);
        assert_eq!(required_str(standby_record, "execution_node"), owner_node);
        assert_eq!(required_str(primary_record, "replica_status"), "mirrored");
        assert_eq!(required_str(standby_record, "replica_status"), "mirrored");
        assert_eq!(required_str(primary_record, "error"), "");
        assert_eq!(required_str(standby_record, "error"), "");

        let (primary_diagnostics, standby_diagnostics) =
            wait_for_startup_diagnostics(&artifacts, &primary_node, &standby_node, &request_key);
        let primary_entries = diagnostic_entries_for_request(&primary_diagnostics, &request_key);
        let standby_entries = diagnostic_entries_for_request(&standby_diagnostics, &request_key);
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

    let primary_logs = stop_process(primary_proc);
    let standby_logs = stop_process(standby_proc);
    write_artifact(
        &artifacts.join("primary.combined.log"),
        &primary_logs.combined,
    );
    write_artifact(
        &artifacts.join("standby.combined.log"),
        &standby_logs.combined,
    );
    if let Err(payload) = result {
        panic!(
            "{}\nartifacts: {}\nprimary stdout:\n{}\nprimary stderr:\n{}\nstandby stdout:\n{}\nstandby stderr:\n{}",
            panic_payload_to_string(payload),
            artifacts.display(),
            primary_logs.stdout,
            primary_logs.stderr,
            standby_logs.stdout,
            standby_logs.stderr
        );
    }

    assert!(
        primary_logs
            .combined
            .contains("[tiny-cluster] runtime bootstrap mode=cluster"),
        "primary runtime bootstrap log missing:\n{}",
        primary_logs.combined
    );
    assert!(
        primary_logs
            .combined
            .contains(&format!("node={primary_node}")),
        "primary runtime bootstrap log missing node name {primary_node}:\n{}",
        primary_logs.combined
    );
    assert!(
        standby_logs
            .combined
            .contains("[tiny-cluster] runtime bootstrap mode=cluster"),
        "standby runtime bootstrap log missing:\n{}",
        standby_logs.combined
    );
    assert!(
        standby_logs
            .combined
            .contains(&format!("node={standby_node}")),
        "standby runtime bootstrap log missing node name {standby_node}:\n{}",
        standby_logs.combined
    );
    assert!(
        primary_logs
            .combined
            .contains(&format!(
                "[mesh-rt startup] transition=startup_dispatch_window runtime_name={STARTUP_RUNTIME_NAME}"
            )),
        "primary startup dispatch window log missing:\n{}",
        primary_logs.combined
    );
    assert!(
        !primary_logs
            .combined
            .contains("transition=startup_dispatch_delay"),
        "legacy startup dispatch delay log should be absent:\n{}",
        primary_logs.combined
    );
}
