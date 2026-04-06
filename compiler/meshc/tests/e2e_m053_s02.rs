mod support;

use serde_json::{json, Value};
use std::any::Any;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

use support::m046_route_free as route_free;
use support::m049_todo_postgres_scaffold as postgres_scaffold;
use support::m053_todo_postgres_deploy as deploy;

fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn required_database_url(test_name: &str) -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| panic!("DATABASE_URL must be set for {test_name}"))
}

fn read_source_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
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

fn startup_transitions(snapshot: &Value, request_key: &str) -> Vec<String> {
    route_free::diagnostic_entries_for_request(snapshot, request_key)
        .iter()
        .filter_map(|entry| entry["transition"].as_str())
        .map(|transition| transition.to_string())
        .collect()
}

fn assert_todo_log_contains(logs: &postgres_scaffold::StoppedTodoApp, needle: &str) {
    assert!(
        logs.combined.contains(needle),
        "expected log `{needle}`\nstdout:\n{}\nstderr:\n{}",
        logs.stdout,
        logs.stderr,
    );
}

fn assert_todo_log_absent(logs: &postgres_scaffold::StoppedTodoApp, needle: &str) {
    assert!(
        !logs.combined.contains(needle),
        "did not expect log `{needle}`\nstdout:\n{}\nstderr:\n{}",
        logs.stdout,
        logs.stderr,
    );
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

fn status_matches(
    json: &Value,
    expected_local_node: &str,
    expected_peer_nodes: &[String],
    expected_nodes: &[String],
    expected_role: &str,
    expected_epoch: u64,
    allowed_health: &[&str],
) -> bool {
    let replication_health = route_free::required_str(&json["authority"], "replication_health");
    route_free::required_str(&json["membership"], "local_node") == expected_local_node
        && route_free::sorted(&route_free::required_string_list(&json["membership"], "peer_nodes"))
            == route_free::sorted(expected_peer_nodes)
        && route_free::sorted(&route_free::required_string_list(&json["membership"], "nodes"))
            == route_free::sorted(expected_nodes)
        && route_free::required_str(&json["authority"], "cluster_role") == expected_role
        && route_free::required_u64(&json["authority"], "promotion_epoch") == expected_epoch
        && allowed_health.contains(&replication_health.as_str())
}

fn pending_record_matches(
    record: &Value,
    request_key: Option<&str>,
    attempt_id: Option<&str>,
    runtime_name: &str,
    expected_owner: &str,
    expected_replica: &str,
    expected_cluster_role: &str,
    expected_epoch: u64,
    allowed_replica_statuses: &[&str],
) -> bool {
    let actual_request_key = route_free::required_str(record, "request_key");
    let actual_attempt_id = route_free::required_str(record, "attempt_id");
    let actual_replica_status = route_free::required_str(record, "replica_status");

    request_key.map_or(!actual_request_key.is_empty(), |expected| {
        actual_request_key == expected
    }) && attempt_id.map_or(!actual_attempt_id.is_empty(), |expected| {
        actual_attempt_id == expected
    }) && route_free::required_str(record, "declared_handler_runtime_name") == runtime_name
        && route_free::required_str(record, "phase") == "submitted"
        && route_free::required_str(record, "result") == "pending"
        && route_free::required_str(record, "owner_node") == expected_owner
        && route_free::required_str(record, "replica_node") == expected_replica
        && route_free::required_str(record, "cluster_role") == expected_cluster_role
        && route_free::required_u64(record, "promotion_epoch") == expected_epoch
        && route_free::required_str(record, "execution_node").is_empty()
        && route_free::required_str(record, "error").is_empty()
        && allowed_replica_statuses.contains(&actual_replica_status.as_str())
}

fn completed_record_matches(
    record: &Value,
    request_key: &str,
    attempt_id: &str,
    runtime_name: &str,
    expected_owner: &str,
    expected_replica: &str,
    expected_execution_node: &str,
    expected_cluster_role: &str,
    expected_epoch: u64,
) -> bool {
    route_free::required_str(record, "request_key") == request_key
        && route_free::required_str(record, "attempt_id") == attempt_id
        && route_free::required_str(record, "declared_handler_runtime_name") == runtime_name
        && route_free::required_str(record, "phase") == "completed"
        && route_free::required_str(record, "result") == "succeeded"
        && route_free::required_str(record, "owner_node") == expected_owner
        && route_free::required_str(record, "replica_node") == expected_replica
        && route_free::required_str(record, "execution_node") == expected_execution_node
        && route_free::required_str(record, "cluster_role") == expected_cluster_role
        && route_free::required_u64(record, "promotion_epoch") == expected_epoch
        && route_free::required_str(record, "error").is_empty()
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
    route_free::diagnostic_entries_for_request(snapshot, request_key)
        .iter()
        .find_map(|entry| {
            (entry["transition"].as_str() == Some("automatic_recovery")
                && entry["request_key"].as_str() == Some(request_key)
                && metadata_value(entry, "previous_attempt_id") == Some(previous_attempt_id)
                && metadata_value(entry, "runtime_name") == Some(deploy::STARTUP_RUNTIME_NAME))
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
    route_free::diagnostic_entries_for_request(snapshot, request_key)
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
    route_free::diagnostic_entries_for_request(snapshot, request_key)
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

#[test]
fn m053_s02_staged_postgres_helper_boots_two_nodes_and_retains_dual_node_operator_artifacts() {
    let test_name =
        "m053_s02_staged_postgres_helper_boots_two_nodes_and_retains_dual_node_operator_artifacts";
    let base_database_url = required_database_url(test_name);
    let artifacts = deploy::artifact_dir_s02("staged-postgres-helper-dual-node-truth");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));

    let project_dir = deploy::init_postgres_todo_project(&workspace_dir, deploy::PACKAGE_NAME, &artifacts);
    let database = deploy::create_isolated_database(&base_database_url, &artifacts, "helper");
    let bundle_dir = deploy::create_retained_bundle_dir("todo-postgres-staged-helper");
    let stage = deploy::run_stage_deploy_script(&project_dir, &bundle_dir, &artifacts, "stage-deploy");
    deploy::assert_phase_success(&stage, "generated Postgres stage-deploy.sh should succeed");

    let bundle = deploy::inspect_staged_bundle(&bundle_dir, &artifacts);
    let loaded_bundle = deploy::load_staged_bundle_from_pointer(&bundle.pointer_path, &artifacts);
    assert_eq!(loaded_bundle.bundle_dir, bundle.bundle_dir);

    let apply = deploy::run_staged_apply_deploy_migrations_script(
        &bundle,
        &artifacts,
        "deploy-apply",
        Some(&database.database_url),
    );
    deploy::assert_phase_success(&apply, "staged apply-deploy-migrations.sh should succeed");

    let runtime = deploy::default_cluster_runtime_pair_for_primary_owned_startup(
        deploy::PACKAGE_NAME,
        &database.database_url,
        Some(2_000),
    );
    let secret_values = [
        base_database_url.as_str(),
        database.database_url.as_str(),
        runtime.primary.cluster_cookie.as_str(),
    ];
    let spawned = deploy::spawn_staged_todo_cluster(&bundle, &artifacts, "runtime", &runtime);

    let run_result = catch_unwind(AssertUnwindSafe(|| {
        let (primary_health, standby_health) =
            deploy::wait_for_cluster_health(&runtime, &artifacts, "health", &secret_values);
        for (label, json) in [("primary", &primary_health), ("standby", &standby_health)] {
            assert_eq!(json["status"].as_str(), Some("ok"), "{label} health should be ok");
            assert_eq!(json["db_backend"].as_str(), Some("postgres"));
            assert_eq!(json["clustered_handler"].as_str(), Some(deploy::STARTUP_RUNTIME_NAME));
            assert!(json.get("database_url").is_none(), "{label} /health must not leak DATABASE_URL");
        }

        let (primary_status, standby_status) =
            deploy::wait_for_dual_node_cluster_status(&artifacts, "cluster-status", &runtime);
        assert_eq!(route_free::required_str(&primary_status["authority"], "cluster_role"), "primary");
        assert_eq!(route_free::required_str(&standby_status["authority"], "cluster_role"), "standby");
        assert_eq!(route_free::required_str(&primary_status["authority"], "replication_health"), "healthy");
        assert_eq!(route_free::required_str(&standby_status["authority"], "replication_health"), "healthy");

        let (primary_before_route, standby_before_route) =
            deploy::continuity_list_snapshot_pair(&artifacts, "continuity-before-route", &runtime);
        for (label, json) in [("primary", &primary_before_route), ("standby", &standby_before_route)] {
            assert_eq!(route_free::required_u64(json, "total_records"), 1, "{label} continuity should only expose the startup record before route traffic");
            assert!(!route_free::required_bool(json, "truncated"));
            assert_eq!(
                route_free::count_records_for_runtime_name(json, deploy::STARTUP_RUNTIME_NAME),
                1,
                "{label} continuity must expose the startup runtime"
            );
            assert_eq!(
                route_free::count_records_for_runtime_name(json, deploy::LIST_ROUTE_RUNTIME_NAME),
                0,
                "{label} continuity must not expose route traffic before the first GET /todos"
            );
        }

        let startup =
            deploy::wait_for_primary_owned_startup_selection(&artifacts, "startup-selection", &runtime);
        assert_eq!(startup.request_key, deploy::startup_request_key());
        assert_eq!(startup.owner_node, runtime.primary.node_name);
        assert_eq!(startup.replica_node, runtime.standby.node_name);

        let (primary_diagnostics, standby_diagnostics) =
            deploy::wait_for_startup_diagnostics_pair(&artifacts, &runtime, &startup.request_key);
        let mut transitions = startup_transitions(&primary_diagnostics, &startup.request_key);
        transitions.extend(startup_transitions(&standby_diagnostics, &startup.request_key));
        assert!(transitions.iter().any(|transition| transition == "startup_trigger"));
        assert!(transitions.iter().any(|transition| transition == "startup_dispatch_window"));
        assert!(transitions.iter().any(|transition| transition == "startup_completed"));
        assert!(!transitions.iter().any(|transition| transition == "startup_rejected"));
        assert!(!transitions.iter().any(|transition| transition == "startup_convergence_timeout"));

        let todos = deploy::json_request_snapshot_for_node(
            &artifacts,
            "todos-list-primary",
            &runtime.primary,
            "GET",
            "/todos",
            None,
            200,
            "GET /todos on primary staged node",
            &secret_values,
        );
        assert!(
            todos.as_array().is_some_and(|items| items.is_empty()),
            "expected an empty todo list before seeding, got: {todos}"
        );

        let (_after_route, route_request_key) = deploy::wait_for_new_route_request_key(
            &artifacts,
            "route-request-key-primary",
            &runtime.primary.node_name,
            &primary_before_route,
            &runtime.primary.cluster_cookie,
        );
        let primary_route_record = deploy::wait_for_continuity_record_completed(
            &artifacts,
            "route-record-primary",
            &runtime.primary.node_name,
            &route_request_key,
            deploy::LIST_ROUTE_RUNTIME_NAME,
            &runtime.primary.cluster_cookie,
        );
        let standby_route_record = deploy::wait_for_continuity_record_completed(
            &artifacts,
            "route-record-standby",
            &runtime.standby.node_name,
            &route_request_key,
            deploy::LIST_ROUTE_RUNTIME_NAME,
            &runtime.standby.cluster_cookie,
        );
        assert_eq!(
            route_free::required_str(&primary_route_record["record"], "request_key"),
            route_request_key
        );
        assert_eq!(
            route_free::required_str(&standby_route_record["record"], "request_key"),
            route_request_key
        );
        assert_eq!(
            route_free::required_u64(&primary_route_record["record"], "replication_count"),
            1
        );
        assert_eq!(
            route_free::required_u64(&standby_route_record["record"], "replication_count"),
            1
        );
    }));

    let stopped = deploy::stop_staged_todo_cluster(spawned, &secret_values);
    deploy::write_artifact(
        &artifacts.join("runtime-primary.combined.log"),
        &stopped.primary.combined,
    );
    deploy::write_artifact(
        &artifacts.join("runtime-standby.combined.log"),
        &stopped.standby.combined,
    );

    match run_result {
        Ok(()) => {
            deploy::assert_runtime_logs(&stopped.primary, &runtime.primary);
            deploy::assert_runtime_logs(&stopped.standby, &runtime.standby);
            deploy::assert_artifacts_redacted(&artifacts, &secret_values);
        }
        Err(payload) => panic!(
            "two-node staged Postgres helper assertions failed: {}\nartifacts: {}\nprimary_stdout: {}\nprimary_stderr: {}\nstandby_stdout: {}\nstandby_stderr: {}",
            panic_payload_to_string(payload),
            artifacts.display(),
            stopped.primary.stdout,
            stopped.primary.stderr,
            stopped.standby.stdout,
            stopped.standby.stderr,
        ),
    }

    for required in [
        "stage-deploy.stdout.log",
        "stage-deploy.stderr.log",
        "stage-deploy.meta.txt",
        "staged-bundle.path.txt",
        "staged-bundle.manifest.json",
        "database.json",
        "deploy-apply.stdout.log",
        "deploy-apply.stderr.log",
        "deploy-apply.meta.txt",
        "runtime.runtime-config.json",
        "health-primary-health.http",
        "health-primary-health.json",
        "health-standby-health.http",
        "health-standby-health.json",
        "cluster-status-primary-status.log",
        "cluster-status-primary-status.json",
        "cluster-status-standby-status.log",
        "cluster-status-standby-status.json",
        "continuity-before-route-primary-continuity.log",
        "continuity-before-route-primary-continuity.json",
        "continuity-before-route-standby-continuity.log",
        "continuity-before-route-standby-continuity.json",
        "startup-selection-primary-startup-list.log",
        "startup-selection-primary-startup-list.json",
        "startup-selection-standby-startup-list.log",
        "startup-selection-standby-startup-list.json",
        "cluster-diagnostics-primary.log",
        "cluster-diagnostics-primary.json",
        "cluster-diagnostics-standby.log",
        "cluster-diagnostics-standby.json",
        "todos-list-primary.http",
        "todos-list-primary.json",
        "route-request-key-primary.log",
        "route-request-key-primary.json",
        "route-record-primary.log",
        "route-record-primary.json",
        "route-record-standby.log",
        "route-record-standby.json",
        "runtime-primary.stdout.log",
        "runtime-primary.stderr.log",
        "runtime-standby.stdout.log",
        "runtime-standby.stderr.log",
        "runtime-primary.combined.log",
        "runtime-standby.combined.log",
    ] {
        assert!(
            artifacts.join(required).exists(),
            "missing retained helper artifact {} in {}",
            required,
            artifacts.display()
        );
    }
}

#[test]
fn m053_s02_staged_postgres_helper_rejects_malformed_bundle_pointer_and_cluster_env() {
    let artifacts = deploy::artifact_dir_s02("staged-postgres-helper-fail-closed");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));

    let missing_pointer = artifacts.join("missing-bundle.path.txt");
    deploy::write_artifact(&missing_pointer, "/definitely/missing/staged-bundle\n");
    let pointer_panic = catch_unwind(AssertUnwindSafe(|| {
        deploy::load_staged_bundle_from_pointer(&missing_pointer, &artifacts)
    }))
    .expect_err("malformed staged bundle pointer should fail closed");
    let pointer_message = panic_payload_to_string(pointer_panic);
    assert!(pointer_message.contains("does not reference a directory"));

    let valid_runtime = deploy::default_cluster_runtime_pair_for_primary_owned_startup(
        deploy::PACKAGE_NAME,
        "postgres://postgres:postgres@127.0.0.1:5432/postgres",
        Some(1_000),
    );

    let mut malformed_node = valid_runtime.primary.clone();
    malformed_node.node_name = "todo-postgres-primary".to_string();
    let malformed_node_panic = catch_unwind(AssertUnwindSafe(|| {
        deploy::assert_valid_runtime_config(&malformed_node)
    }))
    .expect_err("malformed node names should fail closed");
    assert!(panic_payload_to_string(malformed_node_panic).contains("MESH_NODE_NAME"));

    let mut missing_seed = valid_runtime.primary.clone();
    missing_seed.discovery_seed.clear();
    let missing_seed_panic = catch_unwind(AssertUnwindSafe(|| {
        deploy::assert_valid_runtime_config(&missing_seed)
    }))
    .expect_err("missing discovery seed should fail closed");
    assert!(panic_payload_to_string(missing_seed_panic).contains("MESH_DISCOVERY_SEED"));

    let mut invalid_delay = valid_runtime.primary.clone();
    invalid_delay.startup_work_delay_ms = Some(0);
    let invalid_delay_panic = catch_unwind(AssertUnwindSafe(|| {
        deploy::assert_valid_runtime_config(&invalid_delay)
    }))
    .expect_err("zero startup delay should fail closed");
    assert!(panic_payload_to_string(invalid_delay_panic).contains("MESH_STARTUP_WORK_DELAY_MS"));

    let not_ready_output = route_free::run_meshc_cluster(
        &artifacts,
        "cluster-status-not-ready",
        &["cluster", "status", &valid_runtime.primary.node_name, "--json"],
        &valid_runtime.primary.cluster_cookie,
    );
    assert!(
        !not_ready_output.status.success(),
        "meshc cluster status should fail closed before staged startup"
    );

    let malformed_json = route_free::parse_json_stdout(b"{not-json", "malformed operator JSON");
    assert!(
        malformed_json.is_err(),
        "malformed operator JSON should fail closed instead of normalizing"
    );
}

#[test]
fn m053_s02_staged_postgres_failover_proves_clustered_http_and_runtime_recovery() {
    let test_name = "m053_s02_staged_postgres_failover_proves_clustered_http_and_runtime_recovery";
    let base_database_url = required_database_url(test_name);
    let artifacts = deploy::artifact_dir_s02("staged-postgres-failover-runtime-truth");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));

    let project_dir = deploy::init_postgres_todo_project(&workspace_dir, deploy::PACKAGE_NAME, &artifacts);
    let database = deploy::create_isolated_database(&base_database_url, &artifacts, "failover");
    let bundle_dir = deploy::create_retained_bundle_dir("todo-postgres-staged-failover");
    let stage = deploy::run_stage_deploy_script(&project_dir, &bundle_dir, &artifacts, "stage-deploy");
    deploy::assert_phase_success(&stage, "generated Postgres stage-deploy.sh should succeed");

    let bundle = deploy::inspect_staged_bundle(&bundle_dir, &artifacts);
    let loaded_bundle = deploy::load_staged_bundle_from_pointer(&bundle.pointer_path, &artifacts);
    assert_eq!(loaded_bundle.bundle_dir, bundle.bundle_dir);

    let apply = deploy::run_staged_apply_deploy_migrations_script(
        &bundle,
        &artifacts,
        "deploy-apply",
        Some(&database.database_url),
    );
    deploy::assert_phase_success(&apply, "staged apply-deploy-migrations.sh should succeed");

    let runtime = deploy::default_cluster_runtime_pair_for_primary_owned_startup(
        deploy::PACKAGE_NAME,
        &database.database_url,
        Some(20_000),
    );
    let secret_values = [
        base_database_url.as_str(),
        database.database_url.as_str(),
        runtime.primary.cluster_cookie.as_str(),
    ];
    let full_membership = vec![runtime.primary.node_name.clone(), runtime.standby.node_name.clone()];
    let standby_only_membership = vec![runtime.standby.node_name.clone()];
    let no_peers: Vec<String> = Vec::new();

    deploy::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "package_dir": project_dir.display().to_string(),
            "bundle_dir": bundle.bundle_dir.display().to_string(),
            "cluster_port": runtime.primary.cluster_port,
            "startup_runtime_name": deploy::STARTUP_RUNTIME_NAME,
            "list_route_runtime_name": deploy::LIST_ROUTE_RUNTIME_NAME,
            "request_key": Value::Null,
            "initial_attempt_id": Value::Null,
            "failover_attempt_id": Value::Null,
            "list_route_request_key": Value::Null,
            "todo_id": Value::Null,
            "primary_node": runtime.primary.node_name.clone(),
            "standby_node": runtime.standby.node_name.clone(),
        }),
    );
    deploy::write_json_artifact(
        &artifacts.join("runtime.runtime-config.json"),
        &json!({
            "primary": {
                "http_port": runtime.primary.http_port,
                "cluster_cookie": "<redacted:cluster-cookie>",
                "node_name": runtime.primary.node_name.clone(),
                "discovery_seed": runtime.primary.discovery_seed.clone(),
                "cluster_port": runtime.primary.cluster_port,
                "cluster_role": runtime.primary.cluster_role.clone(),
                "promotion_epoch": runtime.primary.promotion_epoch,
                "startup_work_delay_ms": runtime.primary.startup_work_delay_ms,
                "database_url": "<redacted:DATABASE_URL>",
            },
            "standby": {
                "http_port": runtime.standby.http_port,
                "cluster_cookie": "<redacted:cluster-cookie>",
                "node_name": runtime.standby.node_name.clone(),
                "discovery_seed": runtime.standby.discovery_seed.clone(),
                "cluster_port": runtime.standby.cluster_port,
                "cluster_role": runtime.standby.cluster_role.clone(),
                "promotion_epoch": runtime.standby.promotion_epoch,
                "startup_work_delay_ms": runtime.standby.startup_work_delay_ms,
                "database_url": "<redacted:DATABASE_URL>",
            }
        }),
    );

    let mut primary_run1 = Some(deploy::spawn_staged_todo_app(
        &bundle,
        &artifacts,
        "primary-run1",
        &runtime.primary,
    ));
    let mut standby_run1 = Some(deploy::spawn_staged_todo_app(
        &bundle,
        &artifacts,
        "standby-run1",
        &runtime.standby,
    ));
    let mut primary_run1_logs: Option<postgres_scaffold::StoppedTodoApp> = None;
    let mut primary_run2: Option<postgres_scaffold::SpawnedTodoApp> = None;
    let mut request_key: Option<String> = None;
    let mut initial_attempt_id: Option<String> = None;
    let mut failover_attempt_id: Option<String> = None;
    let mut list_route_request_key: Option<String> = None;
    let mut todo_id: Option<String> = None;

    let run_result = catch_unwind(AssertUnwindSafe(|| {
        let (primary_health, standby_health) =
            deploy::wait_for_cluster_health(&runtime, &artifacts, "health", &secret_values);
        for (label, json) in [("primary", &primary_health), ("standby", &standby_health)] {
            assert_eq!(json["status"].as_str(), Some("ok"), "{label} health should be ok");
            assert_eq!(json["db_backend"].as_str(), Some("postgres"));
            assert_eq!(json["clustered_handler"].as_str(), Some(deploy::STARTUP_RUNTIME_NAME));
            assert!(json.get("database_url").is_none(), "{label} /health must not leak DATABASE_URL");
        }

        route_free::wait_for_cluster_status_membership(
            &artifacts,
            "pre-kill-status-primary",
            &runtime.primary.node_name,
            std::slice::from_ref(&runtime.standby.node_name),
            &full_membership,
            "primary",
            0,
            &["healthy"],
            &runtime.primary.cluster_cookie,
        );
        route_free::wait_for_cluster_status_membership(
            &artifacts,
            "pre-kill-status-standby",
            &runtime.standby.node_name,
            std::slice::from_ref(&runtime.primary.node_name),
            &full_membership,
            "standby",
            0,
            &["healthy"],
            &runtime.standby.cluster_cookie,
        );

        let startup =
            deploy::wait_for_primary_owned_startup_selection(&artifacts, "startup-selection", &runtime);
        request_key = Some(startup.request_key.clone());
        initial_attempt_id = Some(startup.attempt_id.clone());

        route_free::wait_for_continuity_record_matching(
            &artifacts,
            "pre-kill-continuity-primary",
            &runtime.primary.node_name,
            &startup.request_key,
            "pre-kill primary-owned startup pending truth",
            &runtime.primary.cluster_cookie,
            |json| {
                pending_record_matches(
                    &json["record"],
                    Some(&startup.request_key),
                    Some(&startup.attempt_id),
                    deploy::STARTUP_RUNTIME_NAME,
                    &runtime.primary.node_name,
                    &runtime.standby.node_name,
                    "primary",
                    0,
                    &["preparing", "mirrored"],
                )
            },
        );
        route_free::wait_for_continuity_record_matching(
            &artifacts,
            "pre-kill-continuity-standby",
            &runtime.standby.node_name,
            &startup.request_key,
            "pre-kill mirrored standby startup pending truth",
            &runtime.standby.cluster_cookie,
            |json| {
                pending_record_matches(
                    &json["record"],
                    Some(&startup.request_key),
                    Some(&startup.attempt_id),
                    deploy::STARTUP_RUNTIME_NAME,
                    &runtime.primary.node_name,
                    &runtime.standby.node_name,
                    "standby",
                    0,
                    &["mirrored"],
                )
            },
        );

        let create_todo = deploy::json_request_snapshot_for_node(
            &artifacts,
            "create-todo-primary",
            &runtime.primary,
            "POST",
            "/todos",
            Some(r#"{"title":"failover smoke todo"}"#),
            201,
            "POST /todos on primary staged node",
            &secret_values,
        );
        let created_todo_id = route_free::required_str(&create_todo, "id");
        todo_id = Some(created_todo_id.clone());
        assert_eq!(route_free::required_str(&create_todo, "title"), "failover smoke todo");
        assert_eq!(create_todo["completed"].as_bool(), Some(false));

        let (before_route_primary, before_route_standby) =
            deploy::continuity_list_snapshot_pair(&artifacts, "continuity-before-list-route", &runtime);
        for (label, json) in [("primary", &before_route_primary), ("standby", &before_route_standby)] {
            assert_eq!(route_free::required_u64(json, "total_records"), 1, "{label} continuity should only expose the startup record before the first clustered GET /todos");
            assert_eq!(
                route_free::count_records_for_runtime_name(json, deploy::LIST_ROUTE_RUNTIME_NAME),
                0,
                "{label} continuity must not expose route traffic before GET /todos"
            );
        }

        let todos_before_failover = deploy::json_request_snapshot_for_node(
            &artifacts,
            "todos-before-failover-primary",
            &runtime.primary,
            "GET",
            "/todos",
            None,
            200,
            "GET /todos on primary staged node before failover",
            &secret_values,
        );
        let todos_before_failover_items = todos_before_failover
            .as_array()
            .unwrap_or_else(|| panic!("expected GET /todos to return an array, got: {todos_before_failover}"));
        let todo_before_failover = todos_before_failover_items
            .iter()
            .find(|item| item["id"].as_str() == Some(created_todo_id.as_str()))
            .unwrap_or_else(|| panic!("GET /todos before failover did not include created todo {created_todo_id}: {todos_before_failover}"));
        assert_eq!(todo_before_failover["title"].as_str(), Some("failover smoke todo"));
        assert_eq!(todo_before_failover["completed"].as_bool(), Some(false));

        let (_route_list_primary, new_list_route_request_key) = deploy::wait_for_new_route_request_key(
            &artifacts,
            "route-request-key-primary",
            &runtime.primary.node_name,
            &before_route_primary,
            &runtime.primary.cluster_cookie,
        );
        list_route_request_key = Some(new_list_route_request_key.clone());
        let route_record_primary = deploy::wait_for_continuity_record_completed(
            &artifacts,
            "route-record-primary",
            &runtime.primary.node_name,
            &new_list_route_request_key,
            deploy::LIST_ROUTE_RUNTIME_NAME,
            &runtime.primary.cluster_cookie,
        );
        let route_record_standby = deploy::wait_for_continuity_record_completed(
            &artifacts,
            "route-record-standby",
            &runtime.standby.node_name,
            &new_list_route_request_key,
            deploy::LIST_ROUTE_RUNTIME_NAME,
            &runtime.standby.cluster_cookie,
        );
        assert_eq!(
            route_free::required_str(&route_record_primary["record"], "request_key"),
            new_list_route_request_key
        );
        assert_eq!(
            route_free::required_str(&route_record_standby["record"], "request_key"),
            new_list_route_request_key
        );
        assert_eq!(
            route_free::required_u64(&route_record_primary["record"], "replication_count"),
            1
        );
        assert_eq!(
            route_free::required_u64(&route_record_standby["record"], "replication_count"),
            1
        );

        deploy::write_json_artifact(
            &artifacts.join("scenario-meta.json"),
            &json!({
                "package_dir": project_dir.display().to_string(),
                "bundle_dir": bundle.bundle_dir.display().to_string(),
                "cluster_port": runtime.primary.cluster_port,
                "startup_runtime_name": deploy::STARTUP_RUNTIME_NAME,
                "list_route_runtime_name": deploy::LIST_ROUTE_RUNTIME_NAME,
                "request_key": startup.request_key,
                "initial_attempt_id": startup.attempt_id,
                "failover_attempt_id": Value::Null,
                "list_route_request_key": new_list_route_request_key,
                "todo_id": created_todo_id,
                "primary_node": runtime.primary.node_name.clone(),
                "standby_node": runtime.standby.node_name.clone(),
            }),
        );

        primary_run1_logs = primary_run1.take().map(|spawned| deploy::stop_todo_app(spawned, &secret_values));
        deploy::write_artifact(
            &artifacts.join("primary-run1.combined.log"),
            primary_run1_logs
                .as_ref()
                .expect("primary run1 logs missing after forced owner stop")
                .combined
                .as_str(),
        );

        route_free::wait_for_cluster_status_matching(
            &artifacts,
            "post-kill-status-standby",
            &runtime.standby.node_name,
            "post-kill standby promotion truth",
            &runtime.standby.cluster_cookie,
            |json| {
                status_matches(
                    json,
                    &runtime.standby.node_name,
                    &no_peers,
                    &standby_only_membership,
                    "primary",
                    1,
                    &["unavailable", "local_only", "healthy"],
                )
            },
        );

        let post_kill_diagnostics = route_free::wait_for_diagnostics_matching(
            &artifacts,
            "post-kill-diagnostics-standby",
            &runtime.standby.node_name,
            "post-kill standby promotion/recovery diagnostics truth",
            &runtime.standby.cluster_cookie,
            |snapshot| {
                if !has_automatic_promotion(snapshot, &runtime.primary.node_name) {
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
                    &runtime.standby.node_name,
                )
            },
        );
        assert!(!route_free::required_bool(&post_kill_diagnostics, "truncated"));
        let recovered_attempt_id = automatic_recovery_attempt_id(
            &post_kill_diagnostics,
            request_key.as_deref().unwrap(),
            initial_attempt_id.as_deref().unwrap(),
        )
        .expect("post-kill diagnostics should expose the automatic recovery attempt id");
        failover_attempt_id = Some(recovered_attempt_id.clone());

        let post_kill_continuity = route_free::wait_for_continuity_record_matching(
            &artifacts,
            "post-kill-continuity-standby-completed",
            &runtime.standby.node_name,
            request_key.as_deref().unwrap(),
            "post-kill standby completion truth",
            &runtime.standby.cluster_cookie,
            |json| {
                completed_record_matches(
                    &json["record"],
                    request_key.as_deref().unwrap(),
                    failover_attempt_id.as_deref().unwrap(),
                    deploy::STARTUP_RUNTIME_NAME,
                    &runtime.standby.node_name,
                    "",
                    &runtime.standby.node_name,
                    "primary",
                    1,
                )
            },
        );
        assert_eq!(
            route_free::required_str(&post_kill_continuity["record"], "owner_node"),
            runtime.standby.node_name
        );
        assert_eq!(
            route_free::required_str(&post_kill_continuity["record"], "execution_node"),
            runtime.standby.node_name
        );

        let todo_after_failover = deploy::json_request_snapshot_for_node(
            &artifacts,
            "get-todo-after-failover-standby",
            &runtime.standby,
            "GET",
            &format!("/todos/{}", created_todo_id),
            None,
            200,
            "GET /todos/:id on promoted standby after failover",
            &secret_values,
        );
        assert_eq!(route_free::required_str(&todo_after_failover, "id"), created_todo_id);
        assert_eq!(route_free::required_str(&todo_after_failover, "title"), "failover smoke todo");
        assert_eq!(todo_after_failover["completed"].as_bool(), Some(false));

        let toggled_after_failover = deploy::json_request_snapshot_for_node(
            &artifacts,
            "toggle-todo-after-failover-standby",
            &runtime.standby,
            "PUT",
            &format!("/todos/{}", created_todo_id),
            None,
            200,
            "PUT /todos/:id on promoted standby after failover",
            &secret_values,
        );
        assert_eq!(toggled_after_failover["completed"].as_bool(), Some(true));

        let get_toggled_after_failover = deploy::json_request_snapshot_for_node(
            &artifacts,
            "get-toggled-todo-after-failover-standby",
            &runtime.standby,
            "GET",
            &format!("/todos/{}", created_todo_id),
            None,
            200,
            "GET /todos/:id on promoted standby after toggle",
            &secret_values,
        );
        assert_eq!(get_toggled_after_failover["completed"].as_bool(), Some(true));

        let deleted_after_failover = deploy::json_request_snapshot_for_node(
            &artifacts,
            "delete-todo-after-failover-standby",
            &runtime.standby,
            "DELETE",
            &format!("/todos/{}", created_todo_id),
            None,
            200,
            "DELETE /todos/:id on promoted standby after failover",
            &secret_values,
        );
        assert_eq!(route_free::required_str(&deleted_after_failover, "status"), "deleted");
        assert_eq!(route_free::required_str(&deleted_after_failover, "id"), created_todo_id);

        let missing_after_delete = deploy::json_request_snapshot_for_node(
            &artifacts,
            "missing-todo-after-delete-standby",
            &runtime.standby,
            "GET",
            &format!("/todos/{}", created_todo_id),
            None,
            404,
            "GET /todos/:id after delete on promoted standby",
            &secret_values,
        );
        assert_eq!(route_free::required_str(&missing_after_delete, "error"), "todo not found");

        primary_run2 = Some(deploy::spawn_staged_todo_app(
            &bundle,
            &artifacts,
            "primary-run2",
            &runtime.primary,
        ));

        route_free::wait_for_cluster_status_matching(
            &artifacts,
            "post-rejoin-status-primary",
            &runtime.primary.node_name,
            "post-rejoin stale-primary fenced status truth",
            &runtime.primary.cluster_cookie,
            |json| {
                status_matches(
                    json,
                    &runtime.primary.node_name,
                    std::slice::from_ref(&runtime.standby.node_name),
                    &full_membership,
                    "standby",
                    1,
                    &["healthy", "local_only"],
                )
            },
        );
        route_free::wait_for_cluster_status_matching(
            &artifacts,
            "post-rejoin-status-standby",
            &runtime.standby.node_name,
            "post-rejoin promoted-standby status truth",
            &runtime.standby.cluster_cookie,
            |json| {
                status_matches(
                    json,
                    &runtime.standby.node_name,
                    std::slice::from_ref(&runtime.primary.node_name),
                    &full_membership,
                    "primary",
                    1,
                    &["unavailable", "local_only", "healthy"],
                )
            },
        );

        let post_rejoin_diagnostics = route_free::wait_for_diagnostics_matching(
            &artifacts,
            "post-rejoin-diagnostics-primary",
            &runtime.primary.node_name,
            "post-rejoin fenced rejoin diagnostics truth",
            &runtime.primary.cluster_cookie,
            |snapshot| {
                has_fenced_rejoin(
                    snapshot,
                    request_key.as_deref().unwrap(),
                    failover_attempt_id.as_deref().unwrap(),
                )
            },
        );
        assert!(!route_free::required_bool(&post_rejoin_diagnostics, "truncated"));

        route_free::wait_for_continuity_record_matching(
            &artifacts,
            "post-rejoin-continuity-primary",
            &runtime.primary.node_name,
            request_key.as_deref().unwrap(),
            "post-rejoin stale-primary continuity truth",
            &runtime.primary.cluster_cookie,
            |json| {
                completed_record_matches(
                    &json["record"],
                    request_key.as_deref().unwrap(),
                    failover_attempt_id.as_deref().unwrap(),
                    deploy::STARTUP_RUNTIME_NAME,
                    &runtime.standby.node_name,
                    "",
                    &runtime.standby.node_name,
                    "standby",
                    1,
                )
            },
        );
        route_free::wait_for_continuity_record_matching(
            &artifacts,
            "post-rejoin-continuity-standby",
            &runtime.standby.node_name,
            request_key.as_deref().unwrap(),
            "post-rejoin promoted-standby continuity truth",
            &runtime.standby.cluster_cookie,
            |json| {
                completed_record_matches(
                    &json["record"],
                    request_key.as_deref().unwrap(),
                    failover_attempt_id.as_deref().unwrap(),
                    deploy::STARTUP_RUNTIME_NAME,
                    &runtime.standby.node_name,
                    "",
                    &runtime.standby.node_name,
                    "primary",
                    1,
                )
            },
        );

        deploy::write_json_artifact(
            &artifacts.join("scenario-meta.json"),
            &json!({
                "package_dir": project_dir.display().to_string(),
                "bundle_dir": bundle.bundle_dir.display().to_string(),
                "cluster_port": runtime.primary.cluster_port,
                "startup_runtime_name": deploy::STARTUP_RUNTIME_NAME,
                "list_route_runtime_name": deploy::LIST_ROUTE_RUNTIME_NAME,
                "request_key": request_key.as_deref().unwrap(),
                "initial_attempt_id": initial_attempt_id.as_deref().unwrap(),
                "failover_attempt_id": failover_attempt_id.as_deref().unwrap(),
                "list_route_request_key": list_route_request_key.as_deref().unwrap(),
                "todo_id": todo_id.as_deref().unwrap(),
                "primary_node": runtime.primary.node_name.clone(),
                "standby_node": runtime.standby.node_name.clone(),
            }),
        );
    }));

    let standby_logs = deploy::stop_todo_app(
        standby_run1
            .take()
            .expect("standby process missing during cleanup"),
        &secret_values,
    );
    deploy::write_artifact(
        &artifacts.join("standby-run1.combined.log"),
        &standby_logs.combined,
    );

    let primary_run2_logs = primary_run2
        .take()
        .map(|spawned| deploy::stop_todo_app(spawned, &secret_values));
    if let Some(logs) = primary_run2_logs.as_ref() {
        deploy::write_artifact(&artifacts.join("primary-run2.combined.log"), &logs.combined);
    }

    let primary_run1_logs = match primary_run1_logs {
        Some(logs) => logs,
        None => deploy::stop_todo_app(
            primary_run1
                .take()
                .expect("primary run1 missing during cleanup"),
            &secret_values,
        ),
    };
    deploy::write_artifact(
        &artifacts.join("primary-run1.combined.log"),
        &primary_run1_logs.combined,
    );

    if let Err(payload) = run_result {
        let run2_stdout = primary_run2_logs
            .as_ref()
            .map(|logs| logs.stdout.as_str())
            .unwrap_or("");
        let run2_stderr = primary_run2_logs
            .as_ref()
            .map(|logs| logs.stderr.as_str())
            .unwrap_or("");
        panic!(
            "staged Postgres failover assertions failed: {}\nartifacts: {}\nprimary run1 stdout:\n{}\nprimary run1 stderr:\n{}\nprimary run2 stdout:\n{}\nprimary run2 stderr:\n{}\nstandby stdout:\n{}\nstandby stderr:\n{}",
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

    let request_key = request_key.as_deref().expect("startup request key missing after successful run");
    let initial_attempt_id = initial_attempt_id
        .as_deref()
        .expect("initial startup attempt id missing after successful run");
    let failover_attempt_id = failover_attempt_id
        .as_deref()
        .expect("failover attempt id missing after successful run");
    let list_route_request_key = list_route_request_key
        .as_deref()
        .expect("list route request key missing after successful run");
    let todo_id = todo_id.as_deref().expect("todo id missing after successful run");
    let primary_run2_logs = primary_run2_logs.expect("primary run2 logs missing after successful run");

    for required in [
        "scenario-meta.json",
        "runtime.runtime-config.json",
        "stage-deploy.stdout.log",
        "stage-deploy.stderr.log",
        "stage-deploy.meta.txt",
        "staged-bundle.path.txt",
        "staged-bundle.manifest.json",
        "database.json",
        "deploy-apply.stdout.log",
        "deploy-apply.stderr.log",
        "deploy-apply.meta.txt",
        "health-primary-health.http",
        "health-primary-health.json",
        "health-standby-health.http",
        "health-standby-health.json",
        "pre-kill-status-primary.log",
        "pre-kill-status-primary.json",
        "pre-kill-status-standby.log",
        "pre-kill-status-standby.json",
        "startup-selection-primary-startup-list.log",
        "startup-selection-primary-startup-list.json",
        "startup-selection-standby-startup-list.log",
        "startup-selection-standby-startup-list.json",
        "pre-kill-continuity-primary.log",
        "pre-kill-continuity-primary.json",
        "pre-kill-continuity-standby.log",
        "pre-kill-continuity-standby.json",
        "create-todo-primary.http",
        "create-todo-primary.json",
        "continuity-before-list-route-primary-continuity.log",
        "continuity-before-list-route-primary-continuity.json",
        "continuity-before-list-route-standby-continuity.log",
        "continuity-before-list-route-standby-continuity.json",
        "todos-before-failover-primary.http",
        "todos-before-failover-primary.json",
        "route-request-key-primary.log",
        "route-request-key-primary.json",
        "route-record-primary.log",
        "route-record-primary.json",
        "route-record-standby.log",
        "route-record-standby.json",
        "post-kill-status-standby.log",
        "post-kill-status-standby.json",
        "post-kill-diagnostics-standby.log",
        "post-kill-diagnostics-standby.json",
        "post-kill-continuity-standby-completed.log",
        "post-kill-continuity-standby-completed.json",
        "get-todo-after-failover-standby.http",
        "get-todo-after-failover-standby.json",
        "toggle-todo-after-failover-standby.http",
        "toggle-todo-after-failover-standby.json",
        "get-toggled-todo-after-failover-standby.http",
        "get-toggled-todo-after-failover-standby.json",
        "delete-todo-after-failover-standby.http",
        "delete-todo-after-failover-standby.json",
        "missing-todo-after-delete-standby.http",
        "missing-todo-after-delete-standby.json",
        "post-rejoin-status-primary.log",
        "post-rejoin-status-primary.json",
        "post-rejoin-status-standby.log",
        "post-rejoin-status-standby.json",
        "post-rejoin-diagnostics-primary.log",
        "post-rejoin-diagnostics-primary.json",
        "post-rejoin-continuity-primary.log",
        "post-rejoin-continuity-primary.json",
        "post-rejoin-continuity-standby.log",
        "post-rejoin-continuity-standby.json",
        "primary-run1.stdout.log",
        "primary-run1.stderr.log",
        "primary-run1.combined.log",
        "standby-run1.stdout.log",
        "standby-run1.stderr.log",
        "standby-run1.combined.log",
        "primary-run2.stdout.log",
        "primary-run2.stderr.log",
        "primary-run2.combined.log",
    ] {
        assert!(
            artifacts.join(required).exists(),
            "missing retained failover artifact {} in {}",
            required,
            artifacts.display(),
        );
    }

    deploy::assert_runtime_logs(&primary_run1_logs, &runtime.primary);
    deploy::assert_runtime_logs(&standby_logs, &runtime.standby);
    deploy::assert_runtime_logs(&primary_run2_logs, &runtime.primary);
    deploy::assert_artifacts_redacted(&artifacts, &secret_values);

    assert_todo_log_contains(
        &standby_logs,
        &format!(
            "[mesh-rt continuity] transition=automatic_promotion disconnected_node={} previous_epoch=0 next_epoch=1",
            runtime.primary.node_name
        ),
    );
    assert_todo_log_contains(
        &standby_logs,
        &format!(
            "[mesh-rt continuity] transition=automatic_recovery request_key={request_key} previous_attempt_id={initial_attempt_id} next_attempt_id={failover_attempt_id} runtime_name={}",
            deploy::STARTUP_RUNTIME_NAME
        ),
    );
    assert_todo_log_contains(
        &standby_logs,
        &format!(
            "[mesh-rt continuity] transition=recovery_rollover request_key={request_key} previous_attempt_id={initial_attempt_id} next_attempt_id={failover_attempt_id}"
        ),
    );
    assert_todo_log_contains(
        &standby_logs,
        &format!(
            "[mesh-rt continuity] transition=completed request_key={request_key} attempt_id={failover_attempt_id} execution={}",
            runtime.standby.node_name
        ),
    );
    assert_todo_log_absent(
        &primary_run1_logs,
        &format!(
            "[mesh-rt continuity] transition=completed request_key={request_key} attempt_id={initial_attempt_id}"
        ),
    );
    assert_todo_log_contains(&primary_run2_logs, "[mesh-rt continuity] transition=fenced_rejoin");
    assert_todo_log_absent(
        &primary_run2_logs,
        &format!(
            "[mesh-rt continuity] transition=completed request_key={request_key} attempt_id={failover_attempt_id} execution={}",
            runtime.primary.node_name
        ),
    );
    assert!(!list_route_request_key.is_empty(), "list route request key should be recorded");
    assert!(!todo_id.is_empty(), "todo id should be recorded");
}

#[test]
fn m053_s02_retained_verifier_keeps_nested_s01_logs_and_non_timeout_failure_reasoning() {
    let verifier_path = deploy::repo_root().join("scripts/verify-m053-s02.sh");

    assert_source_contains_all(
        &verifier_path,
        &[
            "failure_reason_for_exit",
            "retain_nested_verifier_logs",
            "run_nested_m053_s01_contract",
            "upstream-m053-s01-verify",
            "m053-s01-example-e2e.log",
            "m053-s01-staged-deploy-e2e.log",
            "manifest.txt",
            "command exited with status %s before %ss deadline",
            "command timed out after %ss",
        ],
    );

    assert_source_order(
        &verifier_path,
        &[
            "failure_reason_for_exit",
            "retain_nested_verifier_logs",
            "run_nested_m053_s01_contract",
            "run_nested_m053_s01_contract m053-s01-contract m053-s01-contract 3600 \\",
            "run_expect_success_with_database_url m053-s02-failover-e2e",
        ],
    );

    assert_source_omits_all(
        &verifier_path,
        &[
            "run_expect_success_with_database_url m053-s01-contract m053-s01-contract no 3600 scripts/verify-m053-s01.sh",
            "expected success within ${timeout_secs}s",
        ],
    );
}

#[test]
fn m053_s02_staged_postgres_helper_keeps_readme_bounded_and_work_source_clean() {
    let readme_path = deploy::repo_root().join("examples/todo-postgres/README.md");
    let work_path = deploy::repo_root().join("examples/todo-postgres/work.mpl");

    assert_source_contains_all(
        &readme_path,
        &[
            "meshc cluster status",
            "meshc cluster continuity",
            "meshc cluster diagnostics",
            "The staged bundle is the public deploy contract.",
            "GET /todos",
            "GET /todos/:id",
        ],
    );
    assert_source_omits_all(
        &readme_path,
        &[
            "owner_lost",
            "automatic_promotion",
            "automatic_recovery",
            "fenced_rejoin",
            "stale-primary",
            "verify-m053-s02",
            "MESH_STARTUP_WORK_DELAY_MS",
        ],
    );

    assert_source_contains_all(
        &work_path,
        &[
            "@cluster pub fn sync_todos()",
            "1 + 1",
        ],
    );
    assert_source_omits_all(
        &work_path,
        &[
            "Timer.sleep",
            "Env.get_int",
            "MESH_STARTUP_WORK_DELAY_MS",
            "owner_node",
            "replica_node",
            "clustered(work)",
        ],
    );
}
