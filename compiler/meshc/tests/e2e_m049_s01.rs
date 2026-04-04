mod support;

use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::Command;

use support::m049_todo_postgres_scaffold as todo;

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
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

#[test]
fn m049_s01_postgres_todo_api_runtime_truth_proves_migrate_test_build_boot_and_crud() {
    let test_name =
        "m049_s01_postgres_todo_api_runtime_truth_proves_migrate_test_build_boot_and_crud";
    let base_database_url = required_database_url(test_name);
    let artifacts = todo::artifact_dir("todo-api-postgres-runtime-truth");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));

    let project_dir = todo::init_postgres_todo_project(&workspace_dir, "todo-starter", &artifacts);
    let database = todo::create_isolated_database(&base_database_url, &artifacts, "runtime");
    let secret_values = [base_database_url.as_str(), database.database_url.as_str()];

    let migrate = todo::run_meshc_migrate_up(&project_dir, &database.database_url, &artifacts);
    todo::assert_phase_success(&migrate, "meshc migrate <project> up should succeed");
    assert!(
        migrate.combined.contains("Applying:")
            && migrate.combined.contains("Applied 1 migration(s)"),
        "expected apply markers in migrate output, got:\n{}",
        migrate.combined
    );

    let meshc_test = todo::run_meshc_tests(&project_dir, &artifacts);
    todo::assert_phase_success(&meshc_test, "meshc test <project> should succeed");
    assert!(
        meshc_test.stdout.contains("2 passed"),
        "expected generated package tests to report 2 passed, got:\n{}",
        meshc_test.combined
    );

    let (build, binary_path) = todo::run_meshc_build(&project_dir, &artifacts);
    todo::assert_phase_success(&build, "meshc build <project> should succeed");

    let runtime_config = todo::default_runtime_config("todo-starter", &database.database_url);
    let spawned = todo::spawn_todo_app(
        &binary_path,
        &project_dir,
        &artifacts,
        "runtime",
        &runtime_config,
    );

    let run_result = catch_unwind(AssertUnwindSafe(|| {
        let health = todo::wait_for_health(&runtime_config, &artifacts, "health", &secret_values);
        assert_eq!(health["status"].as_str(), Some("ok"));
        assert_eq!(health["db_backend"].as_str(), Some("postgres"));
        assert_eq!(health["migration_strategy"].as_str(), Some("meshc migrate"));
        assert_eq!(
            health["clustered_handler"].as_str(),
            Some("Work.sync_todos")
        );
        assert_eq!(
            health["rate_limit_window_seconds"].as_i64(),
            Some(runtime_config.rate_limit_window_seconds as i64)
        );
        assert_eq!(
            health["rate_limit_max_requests"].as_i64(),
            Some(runtime_config.rate_limit_max_requests as i64)
        );
        assert!(health.get("database_url").is_none());
        assert!(health.get("db_path").is_none());

        let empty_list = todo::json_response_snapshot(
            &artifacts,
            "todos-empty",
            &todo::send_http_request(runtime_config.http_port, "GET", "/todos", None)
                .unwrap_or_else(|error| {
                    panic!("GET /todos failed on {}: {error}", runtime_config.http_port)
                }),
            200,
            "GET /todos",
            &secret_values,
        );
        assert!(
            empty_list.as_array().is_some_and(|items| items.is_empty()),
            "expected an empty todo list before the first create, got: {empty_list}"
        );

        let missing_get = todo::json_response_snapshot(
            &artifacts,
            "todos-missing-get",
            &todo::send_http_request(
                runtime_config.http_port,
                "GET",
                &format!("/todos/{}", todo::MISSING_TODO_ID),
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "GET missing todo failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            404,
            "GET /todos/:id missing",
            &secret_values,
        );
        assert_eq!(missing_get["error"].as_str(), Some("todo not found"));

        let empty_title = todo::json_response_snapshot(
            &artifacts,
            "todos-empty-title",
            &todo::send_http_request(
                runtime_config.http_port,
                "POST",
                "/todos",
                Some(r#"{"title":"   "}"#),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "POST empty-title todo failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            400,
            "POST /todos empty title",
            &secret_values,
        );
        assert_eq!(empty_title["error"].as_str(), Some("title is required"));

        let invalid_json = todo::json_response_snapshot(
            &artifacts,
            "todos-invalid-json",
            &todo::send_http_request(runtime_config.http_port, "POST", "/todos", Some("not-json"))
                .unwrap_or_else(|error| {
                    panic!(
                        "POST invalid-json todo failed on {}: {error}",
                        runtime_config.http_port
                    )
                }),
            400,
            "POST /todos invalid JSON",
            &secret_values,
        );
        assert_eq!(invalid_json["error"].as_str(), Some("title is required"));

        let malformed_get = todo::json_response_snapshot(
            &artifacts,
            "todos-malformed-get",
            &todo::send_http_request(
                runtime_config.http_port,
                "GET",
                &format!("/todos/{}", todo::MALFORMED_TODO_ID),
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "GET malformed todo id failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            500,
            "GET /todos/:id malformed id",
            &secret_values,
        );
        let malformed_get_error = malformed_get["error"]
            .as_str()
            .expect("malformed GET response should expose an error string");
        assert!(
            malformed_get_error.contains("invalid input syntax for type uuid"),
            "expected invalid uuid error, got: {malformed_get_error}"
        );

        let created = todo::json_response_snapshot(
            &artifacts,
            "todos-created",
            &todo::send_http_request(
                runtime_config.http_port,
                "POST",
                "/todos",
                Some(r#"{"title":"buy milk"}"#),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "POST create todo failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            201,
            "POST /todos create",
            &secret_values,
        );
        let todo_id = created["id"]
            .as_str()
            .expect("created todo id should be a string")
            .to_string();
        assert_eq!(created["title"].as_str(), Some("buy milk"));
        assert_eq!(created["completed"].as_bool(), Some(false));
        assert!(
            created["created_at"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "created todo should expose created_at, got: {created}"
        );

        let fetched = todo::json_response_snapshot(
            &artifacts,
            "todos-fetched",
            &todo::send_http_request(
                runtime_config.http_port,
                "GET",
                &format!("/todos/{todo_id}"),
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "GET created todo failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            200,
            "GET /todos/:id created",
            &secret_values,
        );
        assert_eq!(
            fetched, created,
            "fetched todo should match the created todo"
        );

        let missing_toggle = todo::json_response_snapshot(
            &artifacts,
            "todos-missing-toggle",
            &todo::send_http_request(
                runtime_config.http_port,
                "PUT",
                &format!("/todos/{}", todo::MISSING_TODO_ID),
                Some("{}"),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "PUT missing todo failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            404,
            "PUT /todos/:id missing",
            &secret_values,
        );
        assert_eq!(missing_toggle["error"].as_str(), Some("todo not found"));

        let malformed_toggle = todo::json_response_snapshot(
            &artifacts,
            "todos-malformed-toggle",
            &todo::send_http_request(
                runtime_config.http_port,
                "PUT",
                &format!("/todos/{}", todo::MALFORMED_TODO_ID),
                Some("{}"),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "PUT malformed todo id failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            500,
            "PUT /todos/:id malformed id",
            &secret_values,
        );
        let malformed_toggle_error = malformed_toggle["error"]
            .as_str()
            .expect("malformed PUT response should expose an error string");
        assert!(
            malformed_toggle_error.contains("invalid input syntax for type uuid"),
            "expected invalid uuid error, got: {malformed_toggle_error}"
        );

        let toggled = todo::json_response_snapshot(
            &artifacts,
            "todos-toggled",
            &todo::send_http_request(
                runtime_config.http_port,
                "PUT",
                &format!("/todos/{todo_id}"),
                Some("{}"),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "PUT toggle todo failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            200,
            "PUT /todos/:id toggle",
            &secret_values,
        );
        assert_eq!(toggled["id"].as_str(), Some(todo_id.as_str()));
        assert_eq!(toggled["title"].as_str(), Some("buy milk"));
        assert_eq!(toggled["completed"].as_bool(), Some(true));

        let missing_delete = todo::json_response_snapshot(
            &artifacts,
            "todos-missing-delete",
            &todo::send_http_request(
                runtime_config.http_port,
                "DELETE",
                &format!("/todos/{}", todo::MISSING_TODO_ID),
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "DELETE missing todo failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            404,
            "DELETE /todos/:id missing",
            &secret_values,
        );
        assert_eq!(missing_delete["error"].as_str(), Some("todo not found"));

        let malformed_delete = todo::json_response_snapshot(
            &artifacts,
            "todos-malformed-delete",
            &todo::send_http_request(
                runtime_config.http_port,
                "DELETE",
                &format!("/todos/{}", todo::MALFORMED_TODO_ID),
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "DELETE malformed todo id failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            500,
            "DELETE /todos/:id malformed id",
            &secret_values,
        );
        let malformed_delete_error = malformed_delete["error"]
            .as_str()
            .expect("malformed DELETE response should expose an error string");
        assert!(
            malformed_delete_error.contains("invalid input syntax for type uuid"),
            "expected invalid uuid error, got: {malformed_delete_error}"
        );

        let deleted = todo::json_response_snapshot(
            &artifacts,
            "todos-deleted",
            &todo::send_http_request(
                runtime_config.http_port,
                "DELETE",
                &format!("/todos/{todo_id}"),
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "DELETE todo failed on {}: {error}",
                    runtime_config.http_port
                )
            }),
            200,
            "DELETE /todos/:id",
            &secret_values,
        );
        assert_eq!(deleted["status"].as_str(), Some("deleted"));
        assert_eq!(deleted["id"].as_str(), Some(todo_id.as_str()));

        let empty_after_delete = todo::json_response_snapshot(
            &artifacts,
            "todos-empty-after-delete",
            &todo::send_http_request(runtime_config.http_port, "GET", "/todos", None)
                .unwrap_or_else(|error| {
                    panic!(
                        "GET /todos after delete failed on {}: {error}",
                        runtime_config.http_port
                    )
                }),
            200,
            "GET /todos after delete",
            &secret_values,
        );
        assert!(
            empty_after_delete
                .as_array()
                .is_some_and(|items| items.is_empty()),
            "expected an empty todo list after delete, got: {empty_after_delete}"
        );
    }));

    let logs = todo::stop_todo_app(spawned, &secret_values);

    match run_result {
        Ok(()) => {
            todo::assert_runtime_logs(&logs, &runtime_config);
            todo::assert_artifacts_redacted(&artifacts, &secret_values);
        }
        Err(payload) => panic!(
            "generated Postgres todo runtime assertions failed: {}\nartifacts: {}\nstdout: {}\nstderr: {}\nstdout_log: {}\nstderr_log: {}",
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
fn m049_s01_postgres_todo_api_missing_database_url_fails_closed() {
    let artifacts = todo::artifact_dir("todo-api-postgres-missing-database-url");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));

    let project_dir = todo::init_postgres_todo_project(&workspace_dir, "todo-starter", &artifacts);
    let (build, binary_path) = todo::run_meshc_build(&project_dir, &artifacts);
    todo::assert_phase_success(
        &build,
        "meshc build <project> should succeed for missing-DATABASE_URL proof",
    );

    let runtime_config =
        todo::default_runtime_config("todo-starter", "postgres://redacted-invalid-placeholder/db");
    let mut command = Command::new(&binary_path);
    command.current_dir(&project_dir);
    command
        .env("PORT", runtime_config.http_port.to_string())
        .env(
            "TODO_RATE_LIMIT_WINDOW_SECONDS",
            runtime_config.rate_limit_window_seconds.to_string(),
        )
        .env(
            "TODO_RATE_LIMIT_MAX_REQUESTS",
            runtime_config.rate_limit_max_requests.to_string(),
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

    let run = todo::run_command_capture(
        &mut command,
        &artifacts,
        "missing-database-url",
        "generated todo runtime without DATABASE_URL",
        todo::BINARY_EXIT_TIMEOUT,
        &[],
    );

    assert!(
        run.combined.contains(
            "[todo-api] Config error: Missing required environment variable DATABASE_URL"
        ),
        "expected explicit missing-DATABASE_URL error, got:\n{}",
        run.combined
    );
    assert!(
        !run.combined
            .contains("[todo-api] HTTP server starting on :"),
        "runtime should fail closed before starting HTTP when DATABASE_URL is missing:\n{}",
        run.combined
    );
    assert!(
        !run.combined.contains("[todo-api] Runtime ready"),
        "runtime should not claim readiness when DATABASE_URL is missing:\n{}",
        run.combined
    );
    todo::assert_artifacts_redacted(&artifacts, &[]);
}

#[test]
fn m049_s01_postgres_todo_api_unmigrated_database_returns_explicit_json_error() {
    let test_name = "m049_s01_postgres_todo_api_unmigrated_database_returns_explicit_json_error";
    let base_database_url = required_database_url(test_name);
    let artifacts = todo::artifact_dir("todo-api-postgres-unmigrated-database");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));

    let project_dir = todo::init_postgres_todo_project(&workspace_dir, "todo-starter", &artifacts);
    let database = todo::create_isolated_database(&base_database_url, &artifacts, "unmigrated");
    let secret_values = [base_database_url.as_str(), database.database_url.as_str()];

    let (build, binary_path) = todo::run_meshc_build(&project_dir, &artifacts);
    todo::assert_phase_success(
        &build,
        "meshc build <project> should succeed for unmigrated-db proof",
    );

    let runtime_config = todo::default_runtime_config("todo-starter", &database.database_url);
    let spawned = todo::spawn_todo_app(
        &binary_path,
        &project_dir,
        &artifacts,
        "runtime",
        &runtime_config,
    );

    let run_result = catch_unwind(AssertUnwindSafe(|| {
        let health = todo::wait_for_health(&runtime_config, &artifacts, "health", &secret_values);
        assert_eq!(health["status"].as_str(), Some("ok"));

        let unmigrated_list = todo::json_response_snapshot(
            &artifacts,
            "todos-unmigrated",
            &todo::send_http_request(runtime_config.http_port, "GET", "/todos", None)
                .unwrap_or_else(|error| {
                    panic!(
                        "GET /todos on unmigrated database failed on {}: {error}",
                        runtime_config.http_port
                    )
                }),
            500,
            "GET /todos on unmigrated database",
            &secret_values,
        );
        let error = unmigrated_list["error"]
            .as_str()
            .expect("unmigrated response should expose an error string");
        assert!(
            error.contains("relation \"todos\" does not exist"),
            "expected missing-table error, got: {error}"
        );
    }));

    let logs = todo::stop_todo_app(spawned, &secret_values);

    match run_result {
        Ok(()) => {
            todo::assert_runtime_logs(&logs, &runtime_config);
            todo::assert_artifacts_redacted(&artifacts, &secret_values);
        }
        Err(payload) => panic!(
            "generated Postgres todo unmigrated-database assertions failed: {}\nartifacts: {}\nstdout: {}\nstderr: {}\nstdout_log: {}\nstderr_log: {}",
            panic_payload_to_string(payload),
            artifacts.display(),
            logs.stdout,
            logs.stderr,
            logs.stdout_path.display(),
            logs.stderr_path.display()
        ),
    }
}
