mod support;

use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;

use support::m051_mesher as mesher;

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_source_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn assert_source_contains(path: &Path, needle: &str) {
    let source = read_source_file(path);
    assert!(
        source.contains(needle),
        "expected {} to contain `{}` but it was missing",
        path.display(),
        needle
    );
}

fn assert_source_omits(path: &Path, needle: &str) {
    let source = read_source_file(path);
    assert!(
        !source.contains(needle),
        "expected {} to omit `{}` but it was still present",
        path.display(),
        needle
    );
}

fn assert_source_contains_all(path: &Path, needles: &[&str]) {
    for needle in needles {
        assert_source_contains(path, needle);
    }
}

fn assert_source_omits_all(path: &Path, needles: &[&str]) {
    for needle in needles {
        assert_source_omits(path, needle);
    }
}

fn assert_source_order(path: &Path, needles: &[&str]) {
    let source = read_source_file(path);
    let mut previous_index = None;
    for needle in needles {
        let index = source.find(needle).unwrap_or_else(|| {
            panic!(
                "expected {} to contain `{}` before checking order",
                path.display(),
                needle
            )
        });
        if let Some(previous_index) = previous_index {
            assert!(
                index > previous_index,
                "expected {} to keep `{}` after the prior ordered marker",
                path.display(),
                needle
            );
        }
        previous_index = Some(index);
    }
}

#[test]
fn m051_s01_mesher_missing_database_url_fails_closed_before_readiness() {
    let artifacts = mesher::artifact_dir("mesher-missing-database-url");

    let (build, binary_path) = mesher::run_meshc_build(&artifacts);
    mesher::assert_phase_success(
        &build,
        "meshc build mesher --output <artifact-bin> should succeed for missing-DATABASE_URL proof",
    );

    let runtime_config =
        mesher::default_runtime_config("postgres://redacted-invalid-placeholder/mesher");
    let mut command = Command::new(&binary_path);
    command
        .current_dir(mesher::repo_root())
        .env("PORT", runtime_config.http_port.to_string())
        .env("MESHER_WS_PORT", runtime_config.ws_port.to_string())
        .env(
            "MESHER_RATE_LIMIT_WINDOW_SECONDS",
            runtime_config.rate_limit_window_seconds.to_string(),
        )
        .env(
            "MESHER_RATE_LIMIT_MAX_EVENTS",
            runtime_config.rate_limit_max_events.to_string(),
        )
        .env("MESH_CLUSTER_COOKIE", &runtime_config.cluster_cookie)
        .env("MESH_NODE_NAME", &runtime_config.node_name)
        .env("MESH_DISCOVERY_SEED", &runtime_config.discovery_seed)
        .env("MESH_CLUSTER_PORT", runtime_config.cluster_port.to_string())
        .env("MESH_CONTINUITY_ROLE", &runtime_config.cluster_role)
        .env(
            "MESH_CONTINUITY_PROMOTION_EPOCH",
            runtime_config.promotion_epoch.to_string(),
        )
        .env_remove("DATABASE_URL");

    let run = mesher::run_command_capture(
        &mut command,
        &artifacts,
        "missing-database-url",
        "Mesher runtime without DATABASE_URL",
        mesher::BINARY_EXIT_TIMEOUT,
        &[],
    );

    assert!(
        run.combined
            .contains("[Mesher] Config error: Missing required environment variable DATABASE_URL"),
        "expected explicit missing-DATABASE_URL error, got:\n{}",
        run.combined
    );
    assert!(
        !run.combined
            .contains("[Mesher] Connecting to PostgreSQL pool..."),
        "Mesher should fail before opening the PostgreSQL pool when DATABASE_URL is missing:\n{}",
        run.combined
    );
    assert!(
        !run.combined.contains("[Mesher] Runtime ready"),
        "Mesher should not claim readiness when DATABASE_URL is missing:\n{}",
        run.combined
    );
    assert!(
        !run.combined.contains("[Mesher] HTTP server starting on :"),
        "Mesher should fail closed before starting HTTP when DATABASE_URL is missing:\n{}",
        run.combined
    );
    mesher::assert_artifacts_redacted(&artifacts, &[]);
}

#[test]
fn m051_s01_mesher_postgres_runtime_truth_proves_seeded_ingest_and_readback() {
    let artifacts = mesher::artifact_dir("mesher-postgres-runtime-truth");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));

    let postgres = mesher::start_postgres_container(&artifacts, "runtime");
    let secret_values = [postgres.database_url.as_str()];

    let migrate = mesher::run_meshc_migrate_up(&postgres.database_url, &artifacts);
    mesher::assert_phase_success(&migrate, "meshc migrate mesher up should succeed");

    let seed_project = mesher::query_single_row(
        &postgres.database_url,
        "SELECT id::text AS id, retention_days::text AS retention_days, sample_rate::text AS sample_rate FROM projects WHERE slug = $1",
        &[mesher::DEFAULT_PROJECT_SLUG],
    );
    let seed_key = mesher::query_single_row(
        &postgres.database_url,
        "SELECT project_id::text AS project_id, label, COALESCE(revoked_at::text, '') AS revoked_at FROM api_keys WHERE key_value = $1",
        &[mesher::DEFAULT_API_KEY],
    );
    mesher::write_json_artifact(&artifacts.join("seed-project.json"), &seed_project);
    mesher::write_json_artifact(&artifacts.join("seed-api-key.json"), &seed_key);
    assert_eq!(
        seed_key.get("project_id").map(String::as_str),
        seed_project.get("id").map(String::as_str),
        "seeded API key should belong to the seeded default project"
    );
    assert_eq!(
        seed_key.get("label").map(String::as_str),
        Some("dev-default")
    );
    assert_eq!(seed_key.get("revoked_at").map(String::as_str), Some(""));

    let (build, binary_path) = mesher::run_meshc_build(&artifacts);
    mesher::assert_phase_success(
        &build,
        "meshc build mesher --output <artifact-bin> should succeed",
    );

    let runtime_config = mesher::default_runtime_config(&postgres.database_url);
    let spawned = mesher::spawn_mesher(&binary_path, &artifacts, "runtime", &runtime_config);

    let run_result = catch_unwind(AssertUnwindSafe(|| {
        let settings = mesher::wait_for_project_settings(
            &runtime_config,
            &artifacts,
            "project-settings-ready",
            &secret_values,
        );
        assert_eq!(settings["retention_days"].as_i64(), Some(90));
        assert_eq!(settings["sample_rate"].as_f64(), Some(1.0));
        assert!(settings.get("database_url").is_none());

        let storage_before = mesher::json_response_snapshot(
            &artifacts,
            "project-storage-before-ingest",
            &mesher::send_http_request(
                runtime_config.http_port,
                "GET",
                "/api/v1/projects/default/storage",
                None,
                &[],
            )
            .unwrap_or_else(|error| {
                panic!(
                    "GET /api/v1/projects/default/storage failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            200,
            "GET /api/v1/projects/default/storage before ingest",
            &secret_values,
        );
        assert_eq!(storage_before["event_count"].as_i64(), Some(0));
        assert!(storage_before["estimated_bytes"].is_number());

        let missing_auth = mesher::json_response_snapshot(
            &artifacts,
            "events-missing-auth",
            &mesher::send_http_request(
                runtime_config.http_port,
                "POST",
                "/api/v1/events",
                Some(r#"{"message":"missing auth","level":"error"}"#),
                &[],
            )
            .unwrap_or_else(|error| {
                panic!(
                    "POST /api/v1/events without auth failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            401,
            "POST /api/v1/events without auth",
            &secret_values,
        );
        assert_eq!(missing_auth["error"].as_str(), Some("unauthorized"));

        let invalid_auth = mesher::json_response_snapshot(
            &artifacts,
            "events-invalid-auth",
            &mesher::send_http_request(
                runtime_config.http_port,
                "POST",
                "/api/v1/events",
                Some(r#"{"message":"invalid auth","level":"error"}"#),
                &[("x-sentry-auth", "mshr_invalid_key")],
            )
            .unwrap_or_else(|error| {
                panic!(
                    "POST /api/v1/events with invalid auth failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            401,
            "POST /api/v1/events with invalid auth",
            &secret_values,
        );
        assert_eq!(invalid_auth["error"].as_str(), Some("unauthorized"));

        let events_after_auth_fail = mesher::query_single_row(
            &postgres.database_url,
            "SELECT count(*)::text AS cnt FROM events",
            &[],
        );
        mesher::write_json_artifact(
            &artifacts.join("events-after-auth-fail.json"),
            &events_after_auth_fail,
        );
        assert_eq!(
            events_after_auth_fail.get("cnt").map(String::as_str),
            Some("0"),
            "unauthorized event submissions should not persist rows"
        );

        let malformed_event = mesher::json_response_snapshot(
            &artifacts,
            "events-malformed-json",
            &mesher::send_http_request(
                runtime_config.http_port,
                "POST",
                "/api/v1/events",
                Some("{\"message\":"),
                &[("x-sentry-auth", mesher::DEFAULT_API_KEY)],
            )
            .unwrap_or_else(|error| {
                panic!(
                    "POST /api/v1/events with malformed JSON failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            400,
            "POST /api/v1/events with malformed JSON",
            &secret_values,
        );
        let malformed_error = malformed_event["error"]
            .as_str()
            .expect("malformed JSON response should expose an error string");
        assert!(
            !malformed_error.is_empty(),
            "malformed JSON response should expose a non-empty error string"
        );

        let events_after_malformed = mesher::query_single_row(
            &postgres.database_url,
            "SELECT count(*)::text AS cnt FROM events",
            &[],
        );
        mesher::write_json_artifact(
            &artifacts.join("events-after-malformed.json"),
            &events_after_malformed,
        );
        assert_eq!(
            events_after_malformed.get("cnt").map(String::as_str),
            Some("0"),
            "malformed event payloads should not persist rows"
        );

        let ingest = mesher::json_response_snapshot(
            &artifacts,
            "events-ingest-accepted",
            &mesher::send_http_request(
                runtime_config.http_port,
                "POST",
                "/api/v1/events",
                Some(r#"{"message":"M051 seeded event","level":"error"}"#),
                &[("x-sentry-auth", mesher::DEFAULT_API_KEY)],
            )
            .unwrap_or_else(|error| {
                panic!(
                    "POST /api/v1/events with seeded API key failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            202,
            "POST /api/v1/events with seeded API key",
            &secret_values,
        );
        assert_eq!(ingest["status"].as_str(), Some("accepted"));

        let issue_row = mesher::wait_for_query_value(
            &postgres.database_url,
            "SELECT id::text AS id, title, level, status, event_count::text AS event_count FROM issues WHERE title = $1",
            &["M051 seeded event"],
            "event_count",
            "1",
            "first Mesher issue row",
        );
        mesher::write_json_artifact(&artifacts.join("issue-row.json"), &issue_row);
        let issue_id = issue_row
            .get("id")
            .cloned()
            .expect("issue row should expose an id");
        assert_eq!(
            issue_row.get("status").map(String::as_str),
            Some("unresolved")
        );
        assert_eq!(issue_row.get("level").map(String::as_str), Some("error"));

        let event_row = mesher::wait_for_query_value(
            &postgres.database_url,
            "SELECT id::text AS id, issue_id::text AS issue_id, level, message FROM events WHERE issue_id = $1::uuid ORDER BY received_at DESC LIMIT 1",
            &[&issue_id],
            "message",
            "M051 seeded event",
            "first Mesher event row",
        );
        mesher::write_json_artifact(&artifacts.join("event-row.json"), &event_row);
        assert_eq!(
            event_row.get("issue_id").map(String::as_str),
            Some(issue_id.as_str())
        );
        assert_eq!(event_row.get("level").map(String::as_str), Some("error"));

        let storage_after = mesher::json_response_snapshot(
            &artifacts,
            "project-storage-after-ingest",
            &mesher::send_http_request(
                runtime_config.http_port,
                "GET",
                "/api/v1/projects/default/storage",
                None,
                &[],
            )
            .unwrap_or_else(|error| {
                panic!(
                    "GET /api/v1/projects/default/storage after ingest failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            200,
            "GET /api/v1/projects/default/storage after ingest",
            &secret_values,
        );
        assert_eq!(storage_after["event_count"].as_i64(), Some(1));
        assert!(storage_after["estimated_bytes"].is_number());

        let issues = mesher::json_response_snapshot(
            &artifacts,
            "project-issues-readback",
            &mesher::send_http_request(
                runtime_config.http_port,
                "GET",
                "/api/v1/projects/default/issues?status=unresolved",
                None,
                &[],
            )
            .unwrap_or_else(|error| {
                panic!(
                    "GET /api/v1/projects/default/issues failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            200,
            "GET /api/v1/projects/default/issues?status=unresolved",
            &secret_values,
        );
        assert_eq!(issues["has_more"].as_bool(), Some(false));
        let issue_items = issues["data"]
            .as_array()
            .expect("issues readback should expose a data array");
        assert_eq!(
            issue_items.len(),
            1,
            "expected exactly one seeded issue in readback"
        );
        let readback_issue = &issue_items[0];
        assert_eq!(readback_issue["id"].as_str(), Some(issue_id.as_str()));
        assert_eq!(readback_issue["title"].as_str(), Some("M051 seeded event"));
        assert_eq!(readback_issue["level"].as_str(), Some("error"));
        assert_eq!(readback_issue["status"].as_str(), Some("unresolved"));
        assert_eq!(readback_issue["event_count"].as_i64(), Some(1));

        let issue_events = mesher::json_response_snapshot(
            &artifacts,
            "issue-events-readback",
            &mesher::send_http_request(
                runtime_config.http_port,
                "GET",
                &format!("/api/v1/issues/{issue_id}/events"),
                None,
                &[],
            )
            .unwrap_or_else(|error| {
                panic!(
                    "GET /api/v1/issues/:issue_id/events failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            200,
            "GET /api/v1/issues/:issue_id/events",
            &secret_values,
        );
        assert_eq!(issue_events["has_more"].as_bool(), Some(false));
        let event_items = issue_events["data"]
            .as_array()
            .expect("issue events readback should expose a data array");
        assert_eq!(
            event_items.len(),
            1,
            "expected exactly one event in issue readback"
        );
        let readback_event = &event_items[0];
        assert_eq!(
            readback_event["id"].as_str(),
            event_row.get("id").map(String::as_str)
        );
        assert_eq!(readback_event["level"].as_str(), Some("error"));
        assert_eq!(
            readback_event["message"].as_str(),
            Some("M051 seeded event")
        );
        assert!(
            readback_event["received_at"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "issue event readback should expose received_at, got: {readback_event}"
        );
    }));

    let logs = mesher::stop_mesher(spawned, &secret_values);

    match run_result {
        Ok(()) => {
            mesher::assert_runtime_logs(&logs, &runtime_config);
            mesher::assert_artifacts_redacted(&artifacts, &secret_values);
        }
        Err(payload) => panic!(
            "Mesher Postgres runtime assertions failed: {}\nartifacts: {}\nstdout: {}\nstderr: {}\nstdout_log: {}\nstderr_log: {}",
            panic_payload_to_string(payload),
            artifacts.display(),
            logs.stdout,
            logs.stderr,
            logs.stdout_path.display(),
            logs.stderr_path.display()
        ),
    }
}

#[test]
fn m051_s01_mesher_readme_is_the_canonical_maintainer_runbook() {
    let repo_root = repo_root();
    let runbook = repo_root.join("mesher/README.md");
    let env_example = repo_root.join("mesher/.env.example");
    let config = repo_root.join("mesher/config.mpl");
    let main = repo_root.join("mesher/main.mpl");
    let auth = repo_root.join("mesher/ingestion/auth.mpl");
    let seed = repo_root.join("mesher/migrations/20260226000000_seed_default_org.mpl");

    assert_source_contains_all(
        &runbook,
        &[
            "This README is the canonical maintainer runbook",
            "## Startup contract",
            "## Seeded development data",
            "## Repo-root maintainer loop",
            "## Live seed-event smoke",
            "## Runtime inspection",
            "## Authoritative proof rail",
            "Node.start_from_env()",
            "mesher/.env.example",
            "cargo run -q -p meshc -- test mesher/tests",
            "DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate mesher status",
            "DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate mesher up",
            "cargo run -q -p meshc -- build mesher",
            "./mesher/mesher",
            "curl -sSf http://127.0.0.1:8080/api/v1/projects/default/settings",
            "http://127.0.0.1:8080/api/v1/events",
            "http://127.0.0.1:8080/api/v1/projects/default/issues?status=unresolved",
            "http://127.0.0.1:8080/api/v1/projects/default/storage",
            "x-sentry-auth: mshr_devdefaultapikey000000000000000000000000000",
            "cargo test -p meshc --test e2e_m051_s01 -- --nocapture",
            "bash scripts/verify-m051-s01.sh",
        ],
    );

    assert_source_order(
        &runbook,
        &[
            "## Startup contract",
            "## Seeded development data",
            "## Repo-root maintainer loop",
            "## Live seed-event smoke",
            "## Runtime inspection",
            "## Authoritative proof rail",
        ],
    );

    for needle in [
        "DATABASE_URL",
        "PORT",
        "MESHER_WS_PORT",
        "MESHER_RATE_LIMIT_WINDOW_SECONDS",
        "MESHER_RATE_LIMIT_MAX_EVENTS",
        "MESH_CLUSTER_COOKIE",
        "MESH_NODE_NAME",
        "MESH_DISCOVERY_SEED",
        "MESH_CLUSTER_PORT",
        "MESH_CONTINUITY_ROLE",
        "MESH_CONTINUITY_PROMOTION_EPOCH",
    ] {
        assert_source_contains(&runbook, needle);
        assert_source_contains(&env_example, needle);
    }

    for needle in [
        "DATABASE_URL",
        "PORT",
        "MESHER_WS_PORT",
        "MESHER_RATE_LIMIT_WINDOW_SECONDS",
        "MESHER_RATE_LIMIT_MAX_EVENTS",
    ] {
        assert_source_contains(&config, needle);
    }

    assert_source_contains_all(
        &main,
        &[
            "\"/api/v1/events\"",
            "\"/api/v1/projects/:project_id/issues\"",
            "\"/api/v1/projects/:project_id/settings\"",
            "\"/api/v1/projects/:project_id/storage\"",
            "cargo run -q -p meshc -- migrate mesher up",
            "Maintainer runbook: mesher/README.md.",
        ],
    );
    assert_source_contains(&auth, "x-sentry-auth");
    assert_source_contains_all(
        &seed,
        &[
            "mshr_devdefaultapikey000000000000000000000000000",
            "cargo run -q -p meshc -- migrate mesher up",
            "/api/v1/projects/default/issues?status=unresolved",
            "README smoke event",
        ],
    );

    assert_source_omits_all(
        &runbook,
        &[
            "reference-backend/README.md",
            "reference-backend/scripts/verify-production-proof-surface.sh",
            "GET /health",
            "POST /jobs",
            "GET /jobs/:id",
            "JOB_POLL_MS",
            "meshc init --template",
            "meshlang.dev/install",
            "website/docs",
        ],
    );
}

#[test]
fn m051_s01_mesher_verifier_replays_runtime_truth_and_fail_closed_contract_checks() {
    let repo_root = repo_root();
    let verifier = repo_root.join("scripts/verify-m051-s01.sh");

    assert_source_contains_all(
        &verifier,
        &[
            "cargo run -q -p meshc -- test mesher/tests",
            "cargo run -q -p meshc -- build mesher",
            "cargo test -p meshc --test e2e_m051_s01 -- --nocapture",
            "m051-s01-package-tests",
            "m051-s01-build",
            "m051-s01-contract",
            "m051-s01-e2e",
            "retain-m051-s01-artifacts",
            "m051-s01-bundle-shape",
            "run_contract_checks",
            "status.txt",
            "current-phase.txt",
            "phase-report.txt",
            "full-contract.log",
            "latest-proof-bundle.txt",
            "retained-m051-s01-artifacts",
            "retained-proof-bundle",
            "mesher-missing-database-url-",
            "mesher-postgres-runtime-truth-",
            "seed-project.json",
            "seed-api-key.json",
            "project-settings-ready.http",
            "events-ingest-accepted.http",
            "project-issues-readback.json",
            "verify-m051-s01: ok",
        ],
    );

    assert_source_order(
        &verifier,
        &[
            "run_expect_success m051-s01-package-tests m051-s01-package-tests no 1800",
            "run_expect_success m051-s01-build m051-s01-build no 1800",
            "if ! run_contract_checks \"$ARTIFACT_DIR/m051-s01-contract.log\"; then",
            "run_expect_success m051-s01-e2e m051-s01-e2e yes 3600",
            "record_phase retain-m051-s01-artifacts started",
            "echo \"verify-m051-s01: ok\"",
        ],
    );

    assert_source_omits_all(
        &verifier,
        &[
            "run_expect_success m051-s01-docs-build",
            "bash scripts/verify-m050-s02.sh",
            "cargo test -p meshc --test tooling_e2e",
            "cargo run -q -p meshc -- build cluster-proof",
        ],
    );
}
