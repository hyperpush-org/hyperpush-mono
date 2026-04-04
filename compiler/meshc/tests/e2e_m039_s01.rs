mod support;

use std::fs;
use std::path::{Path, PathBuf};
use support::m046_route_free as route_free;

const DISCOVERY_SEED: &str = "localhost";
const SHARED_COOKIE: &str = "mesh-m039-s01-cookie";

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m039-s01", test_name)
}

fn build_cluster_proof_binary(artifacts: &Path) -> (PathBuf, route_free::BuildOutputMetadata) {
    let fixture_root = route_free::cluster_proof_fixture_root();
    let binary_dir = artifacts.join("bin");
    fs::create_dir_all(&binary_dir).expect("failed to create cluster-proof binary dir");
    let output_path = binary_dir.join("cluster-proof");
    let metadata =
        route_free::build_package_binary_to_output(&fixture_root, &output_path, artifacts);
    (fixture_root, metadata)
}

#[test]
fn e2e_m039_s01_converges_without_manual_peers() {
    let artifacts = artifact_dir("e2e-m039-s01-converges");
    let (fixture_root, build_metadata) = build_cluster_proof_binary(&artifacts);

    let cluster_port = route_free::dual_stack_cluster_port();
    let primary_node = format!("node-a@{}:{cluster_port}", route_free::LOOPBACK_V4);
    let standby_node = format!("node-b@[{}]:{}", route_free::LOOPBACK_V6, cluster_port);
    let expected_nodes = vec![primary_node.clone(), standby_node.clone()];

    route_free::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &serde_json::json!({
            "fixture_root": fixture_root.display().to_string(),
            "binary_path": build_metadata.binary_path.display().to_string(),
            "cluster_port": cluster_port,
            "primary_node": primary_node,
            "standby_node": standby_node,
            "transport": "meshc cluster status",
        }),
    );

    let primary_proc = route_free::spawn_route_free_runtime(
        &build_metadata.binary_path,
        &fixture_root,
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
        &fixture_root,
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
    }));

    let standby_logs = route_free::stop_process(standby_proc);
    let primary_logs = route_free::stop_process(primary_proc);
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
}

#[test]
fn e2e_m039_s01_membership_updates_after_node_loss() {
    let artifacts = artifact_dir("e2e-m039-s01-node-loss");
    let (fixture_root, build_metadata) = build_cluster_proof_binary(&artifacts);

    let cluster_port = route_free::dual_stack_cluster_port();
    let primary_node = format!("node-a@{}:{cluster_port}", route_free::LOOPBACK_V4);
    let standby_node = format!("node-b@[{}]:{}", route_free::LOOPBACK_V6, cluster_port);
    let expected_nodes = vec![primary_node.clone(), standby_node.clone()];

    route_free::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &serde_json::json!({
            "fixture_root": fixture_root.display().to_string(),
            "binary_path": build_metadata.binary_path.display().to_string(),
            "cluster_port": cluster_port,
            "primary_node": primary_node,
            "standby_node": standby_node,
            "transport": "meshc cluster status",
            "loss_target": "standby",
        }),
    );

    let primary_proc = route_free::spawn_route_free_runtime(
        &build_metadata.binary_path,
        &fixture_root,
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
        &fixture_root,
        &artifacts,
        "standby",
        &standby_node,
        cluster_port,
        "standby",
        0,
        SHARED_COOKIE,
        DISCOVERY_SEED,
    );

    let mut primary_proc = Some(primary_proc);
    let mut standby_proc = Some(standby_proc);
    let mut stopped_standby_logs = None;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        route_free::wait_for_cluster_status_membership(
            &artifacts,
            "cluster-status-primary-before-loss",
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
            "cluster-status-standby-before-loss",
            &standby_node,
            std::slice::from_ref(&primary_node),
            &expected_nodes,
            "standby",
            0,
            &["local_only", "healthy"],
            SHARED_COOKIE,
        );

        stopped_standby_logs = Some(route_free::stop_process(
            standby_proc
                .take()
                .expect("standby process missing before loss injection"),
        ));

        route_free::wait_for_cluster_status_membership(
            &artifacts,
            "cluster-status-primary-after-loss",
            &primary_node,
            &[],
            std::slice::from_ref(&primary_node),
            "primary",
            0,
            // After standby loss, membership should converge to a single primary node,
            // but authority health now reflects the retained startup continuity state.
            &["unavailable", "degraded"],
            SHARED_COOKIE,
        );
    }));

    let primary_logs = route_free::stop_process(
        primary_proc
            .take()
            .expect("primary process missing during cleanup"),
    );
    let standby_logs = match stopped_standby_logs {
        Some(logs) => logs,
        None => route_free::stop_process(
            standby_proc
                .take()
                .expect("standby process missing during cleanup"),
        ),
    };
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
}
