mod support;

use serde_json::{json, Value};
use std::any::Any;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use support::m046_route_free as route_free;
use support::m047_todo_scaffold as todo;

const SHARED_COOKIE: &str = "mesh-m047-s05-todo-cookie";
const TODO_ROUTE_RUNTIME_NAME: &str = todo::TODO_LIST_ROUTE_RUNTIME_HANDLER;

fn repo_root() -> PathBuf {
    todo::repo_root()
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

fn read_source(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn single_node_cluster_config(
    node_basename: &str,
    cluster_port: u16,
) -> todo::TodoClusterRuntimeConfig {
    todo::TodoClusterRuntimeConfig {
        cookie: SHARED_COOKIE.to_string(),
        node_name: format!("{node_basename}@{}:{cluster_port}", route_free::LOOPBACK_V4),
        discovery_seed: route_free::LOOPBACK_V4.to_string(),
        cluster_port,
        cluster_role: "primary".to_string(),
        promotion_epoch: 0,
    }
}

fn assert_single_node_clustered_list_route_record(
    record: &Value,
    request_key: &str,
    node_name: &str,
) {
    assert_eq!(
        route_free::required_str(record, "request_key"),
        request_key,
        "route continuity record should preserve the request key"
    );
    assert_eq!(
        route_free::required_str(record, "declared_handler_runtime_name"),
        TODO_ROUTE_RUNTIME_NAME,
        "route continuity record should preserve the wrapped handler runtime name"
    );
    assert_eq!(
        route_free::required_u64(record, "replication_count"),
        1,
        "single-node clustered route should retain explicit replication_count=1"
    );
    assert_eq!(route_free::required_str(record, "phase"), "completed");
    assert_eq!(route_free::required_str(record, "result"), "succeeded");
    assert_eq!(route_free::required_str(record, "cluster_role"), "primary");
    assert_eq!(route_free::required_u64(record, "promotion_epoch"), 0);
    assert_eq!(
        route_free::required_str(record, "replication_health"),
        "local_only"
    );
    assert_eq!(route_free::required_str(record, "ingress_node"), node_name);
    assert_eq!(route_free::required_str(record, "owner_node"), node_name);
    assert_eq!(
        route_free::required_str(record, "execution_node"),
        node_name
    );
    assert_eq!(route_free::required_str(record, "replica_node"), "");
    assert_eq!(
        route_free::required_str(record, "replica_status"),
        "unassigned"
    );
    assert_eq!(route_free::required_str(record, "error"), "");
    assert!(!route_free::required_str(record, "attempt_id").is_empty());
    assert!(!route_free::required_str(record, "payload_hash").is_empty());
    assert!(!route_free::required_bool(record, "routed_remotely"));
    assert!(route_free::required_bool(record, "fell_back_locally"));
}

fn assert_clustered_list_route_truth(
    artifacts: &Path,
    label_prefix: &str,
    config: &todo::TodoAppConfig,
    cluster: &todo::TodoClusterRuntimeConfig,
    expected_todo_id: &str,
    operator_container_name: Option<&str>,
) -> String {
    let health = todo::wait_for_health(config, artifacts, &format!("{label_prefix}-health"));
    assert_eq!(health["status"].as_str(), Some("ok"));
    assert_eq!(
        health["clustered_handler"].as_str(),
        Some(todo::TODO_RUNTIME_HANDLER)
    );

    todo::wait_for_single_node_cluster_status(
        artifacts,
        &format!("{label_prefix}-status"),
        cluster,
        operator_container_name,
    );
    let before_list = todo::continuity_list_snapshot(
        artifacts,
        &format!("{label_prefix}-continuity-before"),
        cluster,
        operator_container_name,
    );
    assert_eq!(
        route_free::count_records_for_runtime_name(&before_list, TODO_ROUTE_RUNTIME_NAME),
        0,
        "clustered route proof should begin before the first wrapped GET /todos request"
    );

    let listed = todo::get_json_snapshot(
        config,
        "/todos",
        200,
        artifacts,
        &format!("{label_prefix}-todos"),
    );
    let listed_items = listed
        .as_array()
        .expect("clustered list todos response should be an array");
    assert_eq!(
        listed_items.len(),
        1,
        "expected one persisted todo in the clustered GET /todos proof"
    );
    assert_eq!(listed_items[0]["id"].as_str(), Some(expected_todo_id));

    let (_, request_key) = todo::wait_for_new_request_key_for_runtime_name(
        artifacts,
        &format!("{label_prefix}-continuity-after"),
        cluster,
        &before_list,
        TODO_ROUTE_RUNTIME_NAME,
        1,
        operator_container_name,
    );
    let record = todo::wait_for_continuity_record_completed(
        artifacts,
        &format!("{label_prefix}-continuity-record"),
        cluster,
        &request_key,
        TODO_ROUTE_RUNTIME_NAME,
        operator_container_name,
    );
    assert_single_node_clustered_list_route_record(
        &record["record"],
        &request_key,
        &cluster.node_name,
    );
    request_key
}

#[test]
fn m047_s05_todo_scaffold_runtime_truth_persists_natively_and_in_container() {
    let artifacts = todo::artifact_dir("todo-scaffold-runtime-truth");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));
    let project_dir = todo::init_todo_project(&workspace_dir, "todo-starter", &artifacts);
    let init_log = read_source(&artifacts.join("init.log"));
    assert_contains("fixture init log", &init_log, "source=fixture-copy");
    assert_contains(
        "fixture init log",
        &init_log,
        "fixture_root_relative=scripts/fixtures/m047-s05-clustered-todo",
    );
    assert_contains("fixture init log", &init_log, "project_name=todo-starter");
    assert_contains("fixture init log", &init_log, "- mesh.toml");
    assert_omits(
        "fixture init log",
        &init_log,
        "meshc init --template todo-api",
    );

    let manifest = todo::read_and_archive(
        &project_dir.join("mesh.toml"),
        &artifacts.join("package").join("mesh.toml"),
    );
    let main = todo::read_and_archive(
        &project_dir.join("main.mpl"),
        &artifacts.join("package").join("main.mpl"),
    );
    let work = todo::read_and_archive(
        &project_dir.join("work.mpl"),
        &artifacts.join("package").join("work.mpl"),
    );
    let readme = todo::read_and_archive(
        &project_dir.join("README.md"),
        &artifacts.join("package").join("README.md"),
    );
    let dockerfile = todo::read_and_archive(
        &project_dir.join("Dockerfile"),
        &artifacts.join("package").join("Dockerfile"),
    );
    let router = todo::read_and_archive(
        &project_dir.join("api").join("router.mpl"),
        &artifacts.join("package").join("api.router.mpl"),
    );
    let todos_api = todo::read_and_archive(
        &project_dir.join("api").join("todos.mpl"),
        &artifacts.join("package").join("api.todos.mpl"),
    );

    assert_contains("generated mesh.toml", &manifest, "[package]");
    assert_omits("generated mesh.toml", &manifest, "[cluster]");
    assert_contains("generated main.mpl", &main, "Node.start_from_env()");
    assert_contains("generated main.mpl", &main, "start_rate_limiter");
    assert_contains("generated main.mpl", &main, "HTTP.serve(router, port)");
    assert_contains("generated work.mpl", &work, todo::TODO_STARTUP_HANDLER);
    assert_contains("generated README.md", &readme, todo::TODO_RUNTIME_HANDLER);
    assert_contains(
        "generated README.md",
        &readme,
        "docker build -t todo-starter .",
    );
    assert_contains(
        "generated README.md",
        &readme,
        "packages the binary produced by `meshc build .`",
    );
    assert_contains(
        "generated README.md",
        &readme,
        "the Dockerfile copies the already-compiled `./output` binary",
    );
    assert_contains(
        "generated README.md",
        &readme,
        "If you're driving Docker from macOS or Windows, emit `./output` from a Linux builder host, CI job, or container first",
    );
    assert_contains(
        "generated Dockerfile",
        &dockerfile,
        "COPY output /usr/local/bin/todo-starter",
    );
    assert_contains(
        "generated api/router.mpl",
        &router,
        "HTTP.on_get(\"/health\", handle_health)",
    );
    assert_contains(
        "generated api/router.mpl",
        &router,
        "HTTP.on_get(\"/todos\", HTTP.clustered(1, handle_list_todos))",
    );
    assert_contains(
        "generated api/router.mpl",
        &router,
        "HTTP.on_get(\"/todos/:id\", HTTP.clustered(1, handle_get_todo))",
    );
    assert_contains(
        "generated api/router.mpl",
        &router,
        "HTTP.on_post(\"/todos\", handle_create_todo)",
    );
    assert_contains(
        "generated api/router.mpl",
        &router,
        "HTTP.on_put(\"/todos/:id\", handle_toggle_todo)",
    );
    assert_contains(
        "generated api/router.mpl",
        &router,
        "HTTP.on_delete(\"/todos/:id\", handle_delete_todo)",
    );
    assert_omits(
        "generated api/router.mpl",
        &router,
        "HTTP.on_get(\"/health\", HTTP.clustered(",
    );
    assert_omits(
        "generated api/router.mpl",
        &router,
        "HTTP.on_post(\"/todos\", HTTP.clustered(",
    );
    assert_omits(
        "generated api/router.mpl",
        &router,
        "HTTP.on_put(\"/todos/:id\", HTTP.clustered(",
    );
    assert_omits(
        "generated api/router.mpl",
        &router,
        "HTTP.on_delete(\"/todos/:id\", HTTP.clustered(",
    );
    assert_omits(
        "generated api/router.mpl",
        &router,
        "HTTP.clustered(handle_list_todos)",
    );
    assert_omits(
        "generated api/router.mpl",
        &router,
        "HTTP.clustered(handle_get_todo)",
    );
    assert_contains("generated api/todos.mpl", &todos_api, "allow_write(");
    assert_contains(
        "generated api/todos.mpl",
        &todos_api,
        "pub fn handle_list_todos(_request :: Request) -> Response do",
    );
    assert_contains(
        "generated api/todos.mpl",
        &todos_api,
        "pub fn handle_get_todo(request :: Request) -> Response do",
    );
    for (label, source) in [
        ("generated main.mpl", &main),
        ("generated work.mpl", &work),
        ("generated README.md", &readme),
        ("generated api/router.mpl", &router),
        ("generated api/todos.mpl", &todos_api),
    ] {
        assert_omits(label, source, "execute_declared_work");
    }
    assert_contains(
        "generated README.md",
        &readme,
        "`GET /todos` and `GET /todos/:id` use `HTTP.clustered(1, ...)`",
    );
    assert_contains(
        "generated README.md",
        &readme,
        "`GET /health` — local runtime + rate-limit configuration snapshot",
    );
    assert_contains(
        "generated README.md",
        &readme,
        "list todos through `HTTP.clustered(1, ...)`",
    );
    assert_contains(
        "generated README.md",
        &readme,
        "fetch one todo through `HTTP.clustered(1, ...)`",
    );
    assert_contains(
        "generated README.md",
        &readme,
        "Mutating routes (`POST`, `PUT`, `DELETE`) stay local",
    );
    assert_omits(
        "generated README.md",
        &readme,
        "does **not** pretend `HTTP.clustered(...)` exists yet",
    );

    let config = todo::TodoAppConfig {
        http_port: todo::unused_port(),
        db_path: PathBuf::from("todo.sqlite3"),
        rate_limit_window_seconds: 1,
        rate_limit_max_requests: 2,
    };
    let binary_path = todo::build_todo_binary(&project_dir, &artifacts);

    let first_run =
        todo::spawn_todo_app(&binary_path, &project_dir, &artifacts, "first-run", &config);
    let first_health = todo::wait_for_health(&config, &artifacts, "health-first");
    assert_eq!(first_health["status"].as_str(), Some("ok"));
    assert_eq!(
        first_health["clustered_handler"].as_str(),
        Some(todo::TODO_RUNTIME_HANDLER)
    );
    assert_eq!(first_health["rate_limit_window_seconds"].as_u64(), Some(1));
    assert_eq!(first_health["rate_limit_max_requests"].as_u64(), Some(2));
    assert_eq!(
        first_health["db_path"].as_str(),
        Some(config.db_path.to_str().unwrap())
    );

    let created = todo::post_json(&config, "/todos", r#"{"title":"Buy milk"}"#, 201);
    let first_id = created["id"]
        .as_str()
        .expect("create todo response should include id")
        .to_string();
    assert_eq!(created["title"].as_str(), Some("Buy milk"));
    assert_eq!(created["completed"].as_bool(), Some(false));

    let listed = todo::get_json(&config, "/todos", 200);
    let listed_items = listed
        .as_array()
        .expect("list todos response should be an array");
    assert_eq!(
        listed_items.len(),
        1,
        "expected one todo after first create"
    );
    assert_eq!(listed_items[0]["id"].as_str(), Some(first_id.as_str()));

    let fetched = todo::get_json(&config, &format!("/todos/{first_id}"), 200);
    assert_eq!(fetched["title"].as_str(), Some("Buy milk"));
    assert_eq!(fetched["completed"].as_bool(), Some(false));

    let toggled = todo::put_json(&config, &format!("/todos/{first_id}"), "{}", 200);
    assert_eq!(toggled["id"].as_str(), Some(first_id.as_str()));
    assert_eq!(toggled["completed"].as_bool(), Some(true));

    let rate_limited = todo::post_json(&config, "/todos", r#"{"title":"Second"}"#, 429);
    assert_eq!(rate_limited["error"].as_str(), Some("rate limited"));

    std::thread::sleep(Duration::from_millis(1250));

    let created_after_reset = todo::post_json(&config, "/todos", r#"{"title":"Walk dog"}"#, 201);
    let second_id = created_after_reset["id"]
        .as_str()
        .expect("second create todo response should include id")
        .to_string();
    assert_eq!(created_after_reset["title"].as_str(), Some("Walk dog"));

    let stopped_first = todo::stop_todo_app(first_run);
    todo::write_artifact(
        &artifacts.join("first-run.combined.log"),
        &stopped_first.combined,
    );
    assert!(
        stopped_first
            .combined
            .contains("[todo-api] Runtime ready port="),
        "expected runtime-ready log line, got:\n{}",
        stopped_first.combined
    );
    assert!(
        stopped_first
            .combined
            .contains("write_limit_window_seconds=1 write_limit_max=2"),
        "expected rate-limit configuration in logs, got:\n{}",
        stopped_first.combined
    );
    let db_file_path = project_dir.join(&config.db_path);
    assert!(
        db_file_path.exists(),
        "expected SQLite database to persist at {}",
        db_file_path.display()
    );
    todo::archive_file(
        &db_file_path,
        &artifacts.join("db").join("after-first-run.sqlite3"),
    );

    let second_run = todo::spawn_todo_app(
        &binary_path,
        &project_dir,
        &artifacts,
        "second-run",
        &config,
    );
    let second_health = todo::wait_for_health(&config, &artifacts, "health-second");
    assert_eq!(second_health["status"].as_str(), Some("ok"));

    let restarted_list = todo::get_json(&config, "/todos", 200);
    let restarted_items = restarted_list
        .as_array()
        .expect("restart list todos response should be an array");
    assert_eq!(
        restarted_items.len(),
        2,
        "expected both todos to survive restart persistence"
    );
    let persisted_first = restarted_items
        .iter()
        .find(|item| item["id"].as_str() == Some(first_id.as_str()))
        .expect("missing first todo after restart");
    let persisted_second = restarted_items
        .iter()
        .find(|item| item["id"].as_str() == Some(second_id.as_str()))
        .expect("missing second todo after restart");
    assert_eq!(persisted_first["completed"].as_bool(), Some(true));
    assert_eq!(persisted_second["title"].as_str(), Some("Walk dog"));

    let deleted = todo::delete_json(&config, &format!("/todos/{second_id}"), 200);
    assert_eq!(deleted["status"].as_str(), Some("deleted"));
    assert_eq!(deleted["id"].as_str(), Some(second_id.as_str()));

    let final_list = todo::get_json(&config, "/todos", 200);
    let final_items = final_list
        .as_array()
        .expect("final list todos response should be an array");
    assert_eq!(
        final_items.len(),
        1,
        "expected delete route to remove one todo"
    );
    assert_eq!(final_items[0]["id"].as_str(), Some(first_id.as_str()));

    let stopped_second = todo::stop_todo_app(second_run);
    todo::write_artifact(
        &artifacts.join("second-run.combined.log"),
        &stopped_second.combined,
    );
    todo::archive_file(
        &db_file_path,
        &artifacts.join("db").join("after-second-run.sqlite3"),
    );

    let image_tag = format!(
        "mesh-m047-s05-todo-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time before epoch")
            .as_nanos()
    );
    todo::docker_build(&project_dir, &artifacts, &image_tag);

    let container_artifacts = artifacts.join("container");
    fs::create_dir_all(&container_artifacts).unwrap_or_else(|error| {
        panic!(
            "failed to create {}: {error}",
            container_artifacts.display()
        )
    });
    let happy_container_config = todo::TodoDockerContainerConfig {
        container_name: format!("m047-s05-todo-happy-{}", config.http_port),
        host_data_dir: container_artifacts.join("todo-data"),
        container_data_dir: PathBuf::from("/var/lib/todo"),
        db_path: PathBuf::from("/var/lib/todo/todo.sqlite3"),
        rate_limit_window_seconds: 1,
        rate_limit_max_requests: 2,
        publish_http: true,
        cluster: None,
    };
    let broken_db_config = todo::TodoDockerContainerConfig {
        container_name: format!("m047-s05-todo-bad-db-{}", config.http_port),
        host_data_dir: container_artifacts.join("broken-db-data"),
        container_data_dir: PathBuf::from("/var/lib/todo"),
        db_path: PathBuf::from("/dev/null/todo.sqlite3"),
        rate_limit_window_seconds: 1,
        rate_limit_max_requests: 2,
        publish_http: true,
        cluster: None,
    };
    let missing_port_config = todo::TodoDockerContainerConfig {
        container_name: format!("m047-s05-todo-no-port-{}", config.http_port),
        host_data_dir: container_artifacts.join("missing-port-data"),
        container_data_dir: PathBuf::from("/var/lib/todo"),
        db_path: PathBuf::from("/var/lib/todo/todo.sqlite3"),
        rate_limit_window_seconds: 1,
        rate_limit_max_requests: 2,
        publish_http: false,
        cluster: None,
    };

    let mut happy_container = None;
    let mut broken_db_container = None;
    let mut missing_port_container = None;
    let mut broken_db_failed = false;
    let mut missing_port_failed = false;

    let container_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        happy_container = Some(todo::docker_spawn_todo_container(
            &happy_container_config,
            &container_artifacts,
            "container-happy",
            &image_tag,
        ));
        let happy_port = todo::wait_for_published_http_port(
            &happy_container_config.container_name,
            &container_artifacts,
            "container-happy-port",
            Duration::from_secs(10),
        );
        let container_runtime = todo::TodoAppConfig {
            http_port: happy_port,
            db_path: happy_container_config.db_path.clone(),
            rate_limit_window_seconds: 1,
            rate_limit_max_requests: 2,
        };
        let container_health =
            todo::wait_for_health(&container_runtime, &container_artifacts, "container-health");
        assert_eq!(container_health["status"].as_str(), Some("ok"));
        assert_eq!(
            container_health["clustered_handler"].as_str(),
            Some(todo::TODO_RUNTIME_HANDLER)
        );
        assert_eq!(
            container_health["db_path"].as_str(),
            Some(happy_container_config.db_path.to_str().unwrap())
        );
        assert_eq!(
            container_health["rate_limit_window_seconds"].as_u64(),
            Some(1)
        );
        assert_eq!(
            container_health["rate_limit_max_requests"].as_u64(),
            Some(2)
        );

        let container_created = todo::post_json_snapshot(
            &container_runtime,
            "/todos",
            r#"{"title":"Container milk"}"#,
            201,
            &container_artifacts,
            "container-create",
        );
        let container_id = container_created["id"]
            .as_str()
            .expect("container create response should include id")
            .to_string();
        assert_eq!(container_created["title"].as_str(), Some("Container milk"));
        assert_eq!(container_created["completed"].as_bool(), Some(false));

        let container_fetch = todo::get_json_snapshot(
            &container_runtime,
            &format!("/todos/{container_id}"),
            200,
            &container_artifacts,
            "container-fetch",
        );
        assert_eq!(container_fetch["id"].as_str(), Some(container_id.as_str()));
        assert_eq!(container_fetch["title"].as_str(), Some("Container milk"));

        let container_list = todo::get_json_snapshot(
            &container_runtime,
            "/todos",
            200,
            &container_artifacts,
            "container-list",
        );
        let container_items = container_list
            .as_array()
            .expect("container list response should be an array");
        assert_eq!(
            container_items.len(),
            1,
            "expected one todo inside container"
        );
        assert_eq!(
            container_items[0]["id"].as_str(),
            Some(container_id.as_str())
        );

        broken_db_container = Some(todo::docker_spawn_todo_container(
            &broken_db_config,
            &container_artifacts,
            "container-broken-db",
            &image_tag,
        ));
        let broken_db_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let broken_port = todo::wait_for_published_http_port(
                &broken_db_config.container_name,
                &container_artifacts,
                "container-broken-db-port",
                Duration::from_secs(5),
            );
            let broken_runtime = todo::TodoAppConfig {
                http_port: broken_port,
                db_path: broken_db_config.db_path.clone(),
                rate_limit_window_seconds: 1,
                rate_limit_max_requests: 2,
            };
            todo::wait_for_health_with_timeout(
                &broken_runtime,
                &container_artifacts,
                "container-broken-db-health",
                Duration::from_secs(3),
            );
        }));
        assert!(
            broken_db_result.is_err(),
            "broken TODO_DB_PATH should fail before container proof can claim success"
        );
        broken_db_failed = true;
        todo::docker_container_inspect(
            &broken_db_config.container_name,
            &container_artifacts.join("container-broken-db.inspect.json"),
        );

        missing_port_container = Some(todo::docker_spawn_todo_container(
            &missing_port_config,
            &container_artifacts,
            "container-missing-port",
            &image_tag,
        ));
        let missing_port_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            todo::wait_for_published_http_port(
                &missing_port_config.container_name,
                &container_artifacts,
                "container-missing-port",
                Duration::from_secs(2),
            );
        }));
        assert!(
            missing_port_result.is_err(),
            "container proof should fail closed when no host port is published"
        );
        missing_port_failed = true;
        todo::docker_container_inspect(
            &missing_port_config.container_name,
            &container_artifacts.join("container-missing-port.inspect.json"),
        );
    }));

    let happy_logs = happy_container.take().map(|container| {
        todo::docker_stop_todo_container(container, &container_artifacts, "container-happy")
    });
    let broken_db_logs = broken_db_container.take().map(|container| {
        todo::docker_stop_todo_container(container, &container_artifacts, "container-broken-db")
    });
    let missing_port_logs = missing_port_container.take().map(|container| {
        todo::docker_stop_todo_container(container, &container_artifacts, "container-missing-port")
    });

    if let Some(logs) = &happy_logs {
        todo::write_artifact(
            &container_artifacts.join("container-happy.combined.log"),
            &logs.combined,
        );
    }
    if let Some(logs) = &broken_db_logs {
        todo::write_artifact(
            &container_artifacts.join("container-broken-db.combined.log"),
            &logs.combined,
        );
    }
    if let Some(logs) = &missing_port_logs {
        todo::write_artifact(
            &container_artifacts.join("container-missing-port.combined.log"),
            &logs.combined,
        );
    }

    todo::docker_remove_container(
        &happy_container_config.container_name,
        &container_artifacts,
        "container-happy",
    );
    todo::docker_remove_container(
        &broken_db_config.container_name,
        &container_artifacts,
        "container-broken-db",
    );
    todo::docker_remove_container(
        &missing_port_config.container_name,
        &container_artifacts,
        "container-missing-port",
    );
    todo::docker_remove(&image_tag);

    if let Err(payload) = container_result {
        panic!(
            "container runtime truth assertions failed: {}\nartifacts: {}\nhappy container logs:\n{}\n\nbroken-db container logs:\n{}\n\nmissing-port container logs:\n{}",
            panic_payload_to_string(payload),
            container_artifacts.display(),
            happy_logs
                .as_ref()
                .map(|logs| logs.combined.as_str())
                .unwrap_or("<missing>"),
            broken_db_logs
                .as_ref()
                .map(|logs| logs.combined.as_str())
                .unwrap_or("<missing>"),
            missing_port_logs
                .as_ref()
                .map(|logs| logs.combined.as_str())
                .unwrap_or("<missing>"),
        );
    }

    assert!(
        broken_db_failed,
        "broken TODO_DB_PATH negative path should run"
    );
    assert!(
        missing_port_failed,
        "missing published port negative path should run"
    );

    let happy_logs = happy_logs.expect("missing happy container logs after cleanup");
    assert!(
        happy_logs
            .combined
            .contains("[todo-api] Runtime ready port=8080 db_path=/var/lib/todo/todo.sqlite3"),
        "expected runtime-ready container log, got:\n{}",
        happy_logs.combined
    );
    assert!(
        happy_logs
            .combined
            .contains("write_limit_window_seconds=1 write_limit_max=2"),
        "expected rate-limit configuration in container logs, got:\n{}",
        happy_logs.combined
    );

    let broken_db_logs = broken_db_logs.expect("missing broken-db container logs after cleanup");
    assert!(
        broken_db_logs.combined.contains("Database init failed"),
        "expected broken-db logs to retain startup failure, got:\n{}",
        broken_db_logs.combined
    );

    let missing_port_logs =
        missing_port_logs.expect("missing missing-port container logs after cleanup");
    assert!(
        missing_port_logs
            .combined
            .contains("[todo-api] Runtime ready port=8080"),
        "expected unpublished container to boot cleanly, got:\n{}",
        missing_port_logs.combined
    );

    let container_db_path = happy_container_config.host_data_dir.join("todo.sqlite3");
    assert!(
        container_db_path.exists(),
        "expected mounted container SQLite database to persist at {}",
        container_db_path.display()
    );
    todo::archive_file(
        &container_db_path,
        &container_artifacts.join("container-db.sqlite3"),
    );

    for required_path in [
        container_artifacts.join("container-happy-port.inspect.json"),
        container_artifacts.join("container-health.http"),
        container_artifacts.join("container-health.json"),
        container_artifacts.join("container-create.http"),
        container_artifacts.join("container-create.json"),
        container_artifacts.join("container-fetch.http"),
        container_artifacts.join("container-fetch.json"),
        container_artifacts.join("container-list.http"),
        container_artifacts.join("container-list.json"),
        container_artifacts.join("container-broken-db.inspect.json"),
        container_artifacts.join("container-missing-port.inspect.json"),
    ] {
        assert!(
            required_path.is_file(),
            "missing retained artifact {}",
            required_path.display()
        );
    }
    assert!(
        container_artifacts
            .join("container-missing-port.timeout.txt")
            .is_file(),
        "missing unpublished-port timeout artifact"
    );
    assert!(
        container_artifacts
            .join("container-broken-db-port.timeout.txt")
            .is_file()
            || container_artifacts
                .join("container-broken-db-health.timeout.txt")
                .is_file(),
        "broken TODO_DB_PATH should retain either port or health timeout evidence"
    );
}

#[test]
fn m047_s05_cluster_runtime_helpers_reject_missing_mesh_env_values() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        todo::assert_valid_cluster_runtime_config(&todo::TodoClusterRuntimeConfig {
            cookie: "".to_string(),
            node_name: "todo@127.0.0.1:4370".to_string(),
            discovery_seed: route_free::LOOPBACK_V4.to_string(),
            cluster_port: 4370,
            cluster_role: "primary".to_string(),
            promotion_epoch: 0,
        })
    }));
    assert!(
        result.is_err(),
        "clustered todo helpers should reject missing MESH_* env before spawn/query"
    );
}

#[test]
fn m047_s05_continuity_diff_helpers_ignore_record_order_and_empty_lists() {
    let before = json!({
        "records": [
            {
                "request_key": "http-route::Api.Todos.handle_list_todos::1",
                "declared_handler_runtime_name": TODO_ROUTE_RUNTIME_NAME,
                "replication_count": 1
            },
            {
                "request_key": "startup::Work.sync_todos",
                "declared_handler_runtime_name": todo::TODO_RUNTIME_HANDLER,
                "replication_count": 1
            }
        ]
    });
    let reordered_after = json!({
        "records": [
            {
                "request_key": "startup::Work.sync_todos",
                "declared_handler_runtime_name": todo::TODO_RUNTIME_HANDLER,
                "replication_count": 1
            },
            {
                "request_key": "http-route::Api.Todos.handle_list_todos::2",
                "declared_handler_runtime_name": TODO_ROUTE_RUNTIME_NAME,
                "replication_count": 1
            },
            {
                "request_key": "http-route::Api.Todos.handle_list_todos::1",
                "declared_handler_runtime_name": TODO_ROUTE_RUNTIME_NAME,
                "replication_count": 1
            }
        ]
    });
    let stale_after = json!({
        "records": [
            {
                "request_key": "startup::Work.sync_todos",
                "declared_handler_runtime_name": todo::TODO_RUNTIME_HANDLER,
                "replication_count": 1
            },
            {
                "request_key": "http-route::Api.Todos.handle_list_todos::1",
                "declared_handler_runtime_name": TODO_ROUTE_RUNTIME_NAME,
                "replication_count": 1
            }
        ]
    });
    let empty_after = json!({ "records": [] });

    assert_eq!(
        route_free::new_request_keys_for_runtime_name_and_replication_count(
            &before,
            &reordered_after,
            TODO_ROUTE_RUNTIME_NAME,
            1,
        ),
        vec!["http-route::Api.Todos.handle_list_todos::2".to_string()]
    );
    assert!(
        route_free::new_request_keys_for_runtime_name_and_replication_count(
            &before,
            &stale_after,
            TODO_ROUTE_RUNTIME_NAME,
            1,
        )
        .is_empty(),
        "stale continuity snapshots should not invent a new request key"
    );
    assert!(
        route_free::new_request_keys_for_runtime_name_and_replication_count(
            &before,
            &empty_after,
            TODO_ROUTE_RUNTIME_NAME,
            1,
        )
        .is_empty(),
        "empty continuity snapshots should stay empty instead of matching by list position"
    );
}

#[test]
fn m047_s05_continuity_diff_helpers_fail_closed_on_missing_request_key() {
    let before = json!({ "records": [] });
    let malformed_after = json!({
        "records": [
            {
                "declared_handler_runtime_name": TODO_ROUTE_RUNTIME_NAME,
                "replication_count": 1
            }
        ]
    });

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        route_free::new_request_keys_for_runtime_name_and_replication_count(
            &before,
            &malformed_after,
            TODO_ROUTE_RUNTIME_NAME,
            1,
        )
    }));
    assert!(
        result.is_err(),
        "missing request keys in matching continuity records should fail closed"
    );
}

#[test]
fn m047_s05_todo_scaffold_clustered_list_route_truth_is_real_natively_and_in_container() {
    let artifacts = todo::artifact_dir("todo-scaffold-clustered-route-truth");
    let workspace_dir = artifacts.join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));
    let project_dir = todo::init_todo_project(&workspace_dir, "todo-starter", &artifacts);
    let binary_path = todo::build_todo_binary(&project_dir, &artifacts);

    let seed_config = todo::TodoAppConfig {
        http_port: todo::unused_port(),
        db_path: PathBuf::from("todo.sqlite3"),
        rate_limit_window_seconds: 1,
        rate_limit_max_requests: 2,
    };
    let mut seeded_todo_id: Option<String> = None;
    let seed_run = todo::spawn_todo_app(
        &binary_path,
        &project_dir,
        &artifacts,
        "seed-run",
        &seed_config,
    );
    let seed_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let seed_health = todo::wait_for_health(&seed_config, &artifacts, "seed-health");
        assert_eq!(seed_health["status"].as_str(), Some("ok"));
        let created = todo::post_json_snapshot(
            &seed_config,
            "/todos",
            r#"{"title":"Seeded clustered todo"}"#,
            201,
            &artifacts,
            "seed-create",
        );
        seeded_todo_id = Some(
            created["id"]
                .as_str()
                .expect("seeded todo response should include id")
                .to_string(),
        );
        assert_eq!(created["title"].as_str(), Some("Seeded clustered todo"));
    }));
    let seed_logs = todo::stop_todo_app(seed_run);
    todo::write_artifact(
        &artifacts.join("seed-run.combined.log"),
        &seed_logs.combined,
    );
    if let Err(payload) = seed_result {
        panic!(
            "seeded todo setup failed: {}\nartifacts: {}\nstdout:\n{}\nstderr:\n{}",
            panic_payload_to_string(payload),
            artifacts.display(),
            seed_logs.stdout,
            seed_logs.stderr,
        );
    }
    let seeded_todo_id = seeded_todo_id.expect("missing seeded todo id after setup");

    let native_cluster = single_node_cluster_config(
        "m047-s05-native-todo",
        route_free::dual_stack_cluster_port(),
    );
    let native_cluster_config = todo::TodoAppConfig {
        http_port: todo::unused_port(),
        db_path: seed_config.db_path.clone(),
        rate_limit_window_seconds: 1,
        rate_limit_max_requests: 2,
    };
    let mut native_request_key: Option<String> = None;
    let native_run = todo::spawn_todo_app_clustered(
        &binary_path,
        &project_dir,
        &artifacts,
        "native-cluster-run",
        &native_cluster_config,
        &native_cluster,
    );
    let native_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        native_request_key = Some(assert_clustered_list_route_truth(
            &artifacts,
            "native-cluster",
            &native_cluster_config,
            &native_cluster,
            &seeded_todo_id,
            None,
        ));
    }));
    let native_logs = todo::stop_todo_app(native_run);
    todo::write_artifact(
        &artifacts.join("native-cluster-run.combined.log"),
        &native_logs.combined,
    );
    if let Err(payload) = native_result {
        panic!(
            "native clustered todo route proof failed: {}\nartifacts: {}\nstdout:\n{}\nstderr:\n{}",
            panic_payload_to_string(payload),
            artifacts.display(),
            native_logs.stdout,
            native_logs.stderr,
        );
    }
    let native_request_key = native_request_key.expect("missing native clustered request key");
    assert!(
        native_logs.combined.contains(&format!(
            "[todo-api] runtime bootstrap mode=cluster node={}",
            native_cluster.node_name
        )),
        "expected clustered native bootstrap log, got:\n{}",
        native_logs.combined
    );
    assert!(
        native_logs.combined.contains(&format!(
            "[todo-api] Runtime ready port={} db_path={}",
            native_cluster_config.http_port,
            native_cluster_config.db_path.display()
        )),
        "expected clustered native runtime-ready log, got:\n{}",
        native_logs.combined
    );

    let image_tag = format!(
        "mesh-m047-s05-clustered-route-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time before epoch")
            .as_nanos()
    );
    todo::docker_build(&project_dir, &artifacts, &image_tag);

    let container_artifacts = artifacts.join("container");
    fs::create_dir_all(&container_artifacts).unwrap_or_else(|error| {
        panic!(
            "failed to create {}: {error}",
            container_artifacts.display()
        )
    });
    let docker_cluster = single_node_cluster_config(
        "m047-s05-docker-todo",
        route_free::dual_stack_cluster_port(),
    );
    let clustered_container_config = todo::TodoDockerContainerConfig {
        container_name: format!("m047-s05-clustered-todo-{}", seed_config.http_port),
        host_data_dir: container_artifacts.join("clustered-data"),
        container_data_dir: PathBuf::from("/var/lib/todo"),
        db_path: PathBuf::from("/var/lib/todo/todo.sqlite3"),
        rate_limit_window_seconds: 1,
        rate_limit_max_requests: 2,
        publish_http: true,
        cluster: Some(todo::TodoDockerClusterConfig {
            runtime: docker_cluster.clone(),
            publish_cluster_port: true,
        }),
    };
    let missing_cluster_port_runtime = single_node_cluster_config(
        "m047-s05-docker-no-cluster-port",
        route_free::dual_stack_cluster_port(),
    );
    let missing_cluster_port_config = todo::TodoDockerContainerConfig {
        container_name: format!("m047-s05-missing-cluster-port-{}", seed_config.http_port),
        host_data_dir: container_artifacts.join("missing-cluster-port-data"),
        container_data_dir: PathBuf::from("/var/lib/todo"),
        db_path: PathBuf::from("/var/lib/todo/todo.sqlite3"),
        rate_limit_window_seconds: 1,
        rate_limit_max_requests: 2,
        publish_http: true,
        cluster: Some(todo::TodoDockerClusterConfig {
            runtime: missing_cluster_port_runtime.clone(),
            publish_cluster_port: false,
        }),
    };

    let mut clustered_container = None;
    let mut missing_cluster_port_container = None;
    let mut docker_request_key: Option<String> = None;
    let mut missing_cluster_port_failed = false;

    let container_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        clustered_container = Some(todo::docker_spawn_todo_container(
            &clustered_container_config,
            &container_artifacts,
            "clustered-container",
            &image_tag,
        ));
        let clustered_http_port = todo::wait_for_published_http_port(
            &clustered_container_config.container_name,
            &container_artifacts,
            "clustered-container-http-port",
            Duration::from_secs(10),
        );
        let published_cluster_port = todo::wait_for_published_cluster_port(
            &clustered_container_config.container_name,
            docker_cluster.cluster_port,
            &container_artifacts,
            "clustered-container-cluster-port",
            Duration::from_secs(10),
        );
        assert_eq!(published_cluster_port, docker_cluster.cluster_port);
        let clustered_runtime = todo::TodoAppConfig {
            http_port: clustered_http_port,
            db_path: clustered_container_config.db_path.clone(),
            rate_limit_window_seconds: 1,
            rate_limit_max_requests: 2,
        };
        let container_created = todo::post_json_snapshot(
            &clustered_runtime,
            "/todos",
            r#"{"title":"Clustered container todo"}"#,
            201,
            &container_artifacts,
            "clustered-container-create",
        );
        let container_todo_id = container_created["id"]
            .as_str()
            .expect("clustered container create response should include id")
            .to_string();
        assert_eq!(
            container_created["title"].as_str(),
            Some("Clustered container todo")
        );

        docker_request_key = Some(assert_clustered_list_route_truth(
            &container_artifacts,
            "clustered-container",
            &clustered_runtime,
            &docker_cluster,
            &container_todo_id,
            Some(&clustered_container_config.container_name),
        ));

        missing_cluster_port_container = Some(todo::docker_spawn_todo_container(
            &missing_cluster_port_config,
            &container_artifacts,
            "missing-cluster-port",
            &image_tag,
        ));
        let missing_cluster_http_port = todo::wait_for_published_http_port(
            &missing_cluster_port_config.container_name,
            &container_artifacts,
            "missing-cluster-port-http-port",
            Duration::from_secs(10),
        );
        let missing_cluster_runtime = todo::TodoAppConfig {
            http_port: missing_cluster_http_port,
            db_path: missing_cluster_port_config.db_path.clone(),
            rate_limit_window_seconds: 1,
            rate_limit_max_requests: 2,
        };
        let missing_cluster_health = todo::wait_for_health(
            &missing_cluster_runtime,
            &container_artifacts,
            "missing-cluster-port-health",
        );
        assert_eq!(missing_cluster_health["status"].as_str(), Some("ok"));
        let missing_cluster_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            todo::wait_for_published_cluster_port(
                &missing_cluster_port_config.container_name,
                missing_cluster_port_runtime.cluster_port,
                &container_artifacts,
                "missing-cluster-port",
                Duration::from_secs(2),
            );
        }));
        assert!(
            missing_cluster_result.is_err(),
            "cluster-mode docker proof should fail closed when the cluster port is unpublished"
        );
        missing_cluster_port_failed = true;
        todo::docker_container_inspect(
            &missing_cluster_port_config.container_name,
            &container_artifacts.join("missing-cluster-port.inspect.json"),
        );
    }));

    let clustered_container_logs = clustered_container.take().map(|container| {
        todo::docker_stop_todo_container(container, &container_artifacts, "clustered-container")
    });
    let missing_cluster_port_logs = missing_cluster_port_container.take().map(|container| {
        todo::docker_stop_todo_container(container, &container_artifacts, "missing-cluster-port")
    });

    if let Some(logs) = &clustered_container_logs {
        todo::write_artifact(
            &container_artifacts.join("clustered-container.combined.log"),
            &logs.combined,
        );
    }
    if let Some(logs) = &missing_cluster_port_logs {
        todo::write_artifact(
            &container_artifacts.join("missing-cluster-port.combined.log"),
            &logs.combined,
        );
    }

    todo::docker_remove_container(
        &clustered_container_config.container_name,
        &container_artifacts,
        "clustered-container",
    );
    todo::docker_remove_container(
        &missing_cluster_port_config.container_name,
        &container_artifacts,
        "missing-cluster-port",
    );
    todo::docker_remove(&image_tag);

    if let Err(payload) = container_result {
        panic!(
            "docker clustered todo route proof failed: {}\nartifacts: {}\nclustered container logs:\n{}\n\nmissing-cluster-port logs:\n{}",
            panic_payload_to_string(payload),
            container_artifacts.display(),
            clustered_container_logs
                .as_ref()
                .map(|logs| logs.combined.as_str())
                .unwrap_or("<missing>"),
            missing_cluster_port_logs
                .as_ref()
                .map(|logs| logs.combined.as_str())
                .unwrap_or("<missing>"),
        );
    }

    let docker_request_key = docker_request_key.expect("missing docker clustered request key");
    assert!(
        missing_cluster_port_failed,
        "missing published cluster port negative path should run"
    );

    let clustered_container_logs =
        clustered_container_logs.expect("missing clustered container logs after cleanup");
    assert!(
        clustered_container_logs.combined.contains(&format!(
            "[todo-api] runtime bootstrap mode=cluster node={}",
            docker_cluster.node_name
        )),
        "expected clustered docker bootstrap log, got:\n{}",
        clustered_container_logs.combined
    );
    assert!(
        clustered_container_logs
            .combined
            .contains("[todo-api] Runtime ready port=8080 db_path=/var/lib/todo/todo.sqlite3"),
        "expected clustered docker runtime-ready log, got:\n{}",
        clustered_container_logs.combined
    );

    let missing_cluster_port_logs =
        missing_cluster_port_logs.expect("missing missing-cluster-port logs after cleanup");
    assert!(
        missing_cluster_port_logs.combined.contains(&format!(
            "[todo-api] runtime bootstrap mode=cluster node={}",
            missing_cluster_port_runtime.node_name
        )),
        "expected missing-cluster-port container to boot in clustered mode before publication fails, got:\n{}",
        missing_cluster_port_logs.combined
    );

    todo::write_json_artifact(
        &artifacts.join("clustered-route-proof.meta.json"),
        &json!({
            "native_node": native_cluster.node_name,
            "native_request_key": native_request_key,
            "docker_node": docker_cluster.node_name,
            "docker_request_key": docker_request_key,
            "docker_cluster_port": docker_cluster.cluster_port,
            "missing_cluster_port_node": missing_cluster_port_runtime.node_name,
            "missing_cluster_port": missing_cluster_port_runtime.cluster_port,
        }),
    );

    for required_path in [
        artifacts.join("seed-health.http"),
        artifacts.join("seed-health.json"),
        artifacts.join("seed-create.http"),
        artifacts.join("seed-create.json"),
        artifacts.join("seed-run.combined.log"),
        artifacts.join("native-cluster-health.http"),
        artifacts.join("native-cluster-health.json"),
        artifacts.join("native-cluster-status.json"),
        artifacts.join("native-cluster-continuity-before.json"),
        artifacts.join("native-cluster-todos.http"),
        artifacts.join("native-cluster-todos.json"),
        artifacts.join("native-cluster-continuity-after.json"),
        artifacts.join("native-cluster-continuity-record.json"),
        artifacts.join("native-cluster-run.combined.log"),
        artifacts.join("clustered-route-proof.meta.json"),
        container_artifacts.join("clustered-container-http-port.inspect.json"),
        container_artifacts.join("clustered-container-http-port.ports.txt"),
        container_artifacts.join("clustered-container-cluster-port.inspect.json"),
        container_artifacts.join("clustered-container-cluster-port.ports.txt"),
        container_artifacts.join("clustered-container-health.http"),
        container_artifacts.join("clustered-container-health.json"),
        container_artifacts.join("clustered-container-create.http"),
        container_artifacts.join("clustered-container-create.json"),
        container_artifacts.join("clustered-container-status.json"),
        container_artifacts.join("clustered-container-continuity-before.json"),
        container_artifacts.join("clustered-container-todos.http"),
        container_artifacts.join("clustered-container-todos.json"),
        container_artifacts.join("clustered-container-continuity-after.json"),
        container_artifacts.join("clustered-container-continuity-record.json"),
        container_artifacts.join("clustered-container.combined.log"),
        container_artifacts.join("missing-cluster-port-http-port.inspect.json"),
        container_artifacts.join("missing-cluster-port-http-port.ports.txt"),
        container_artifacts.join("missing-cluster-port-health.http"),
        container_artifacts.join("missing-cluster-port-health.json"),
        container_artifacts.join("missing-cluster-port.inspect.json"),
        container_artifacts.join("missing-cluster-port.ports.txt"),
        container_artifacts.join("missing-cluster-port.combined.log"),
    ] {
        assert!(
            required_path.is_file(),
            "missing retained clustered-route artifact {}",
            required_path.display()
        );
    }
    assert!(
        container_artifacts
            .join("missing-cluster-port.timeout.txt")
            .is_file(),
        "missing unpublished-cluster-port timeout artifact"
    );
}

#[test]
fn m047_s05_http_snapshot_helpers_fail_closed_on_bad_json_and_status() {
    let artifacts = todo::artifact_dir("todo-scaffold-http-fail-closed");

    let malformed = todo::HttpResponse {
        status_code: 200,
        raw: "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\nnot-json".to_string(),
        body: "not-json".to_string(),
    };
    let malformed_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        todo::json_response_snapshot(
            &artifacts,
            "malformed-health",
            &malformed,
            200,
            "malformed /health snapshot",
        )
    }));
    assert!(
        malformed_result.is_err(),
        "malformed JSON should fail closed"
    );
    assert!(artifacts.join("malformed-health.http").is_file());
    assert!(artifacts.join("malformed-health.body.txt").is_file());

    let wrong_status = todo::HttpResponse {
        status_code: 500,
        raw: "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\r\n{}"
            .to_string(),
        body: "{}".to_string(),
    };
    let wrong_status_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        todo::json_response_snapshot(
            &artifacts,
            "wrong-status",
            &wrong_status,
            200,
            "wrong-status CRUD snapshot",
        )
    }));
    assert!(
        wrong_status_result.is_err(),
        "unexpected status should fail closed"
    );
    assert!(artifacts.join("wrong-status.http").is_file());
}

#[test]
fn m047_s05_todo_fixture_contract_is_committed_and_helper_is_fixture_backed() {
    let artifacts = todo::artifact_dir("fixture-contract");
    let fixture_root = todo::todo_fixture_root();
    let helper_source = todo::read_and_archive(
        &repo_root().join("compiler/meshc/tests/support/m047_todo_scaffold.rs"),
        &artifacts.join("m047_todo_scaffold.rs"),
    );
    let manifest = todo::read_and_archive(
        &fixture_root.join("mesh.toml"),
        &artifacts.join("mesh.toml"),
    );
    let main = todo::read_and_archive(&fixture_root.join("main.mpl"), &artifacts.join("main.mpl"));
    let work = todo::read_and_archive(&fixture_root.join("work.mpl"), &artifacts.join("work.mpl"));
    let router = todo::read_and_archive(
        &fixture_root.join("api").join("router.mpl"),
        &artifacts.join("api.router.mpl"),
    );
    let readme = todo::read_and_archive(
        &fixture_root.join("README.md"),
        &artifacts.join("README.md"),
    );
    let dockerfile = todo::read_and_archive(
        &fixture_root.join("Dockerfile"),
        &artifacts.join("Dockerfile"),
    );

    for required_relative_path in todo::TODO_FIXTURE_REQUIRED_FILES {
        assert!(
            fixture_root.join(required_relative_path).is_file(),
            "missing committed fixture file {}",
            fixture_root.join(required_relative_path).display()
        );
    }

    assert_contains("fixture mesh.toml", &manifest, "name = \"todo-starter\"");
    assert_contains("fixture main.mpl", &main, "Node.start_from_env()");
    assert_contains("fixture work.mpl", &work, todo::TODO_STARTUP_HANDLER);
    assert_contains(
        "fixture api/router.mpl",
        &router,
        "HTTP.on_get(\"/todos\", HTTP.clustered(1, handle_list_todos))",
    );
    assert_contains(
        "fixture README.md",
        &readme,
        "This project was generated by `meshc init --template todo-api`.",
    );
    assert_contains(
        "fixture Dockerfile",
        &dockerfile,
        "COPY output /usr/local/bin/todo-starter",
    );

    assert_contains(
        "m047_todo_scaffold.rs",
        &helper_source,
        "scripts/fixtures/m047-s05-clustered-todo",
    );
    assert_contains(
        "m047_todo_scaffold.rs",
        &helper_source,
        "source=fixture-copy",
    );
    assert_contains(
        "m047_todo_scaffold.rs",
        &helper_source,
        "init_todo_project_from_fixture_root",
    );
    assert_omits(
        "m047_todo_scaffold.rs",
        &helper_source,
        "meshc init --template todo-api",
    );
}

#[test]
fn m047_s05_todo_fixture_copy_rejects_unexpected_project_name() {
    let artifacts = todo::artifact_dir("fixture-copy-rejects-name");
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let workspace_dir = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        todo::init_todo_project_from_fixture_root(
            &todo::todo_fixture_root(),
            &workspace_dir,
            "unexpected-name",
            &artifacts,
        )
    }));
    assert!(
        result.is_err(),
        "fixture-backed helper should reject hidden renames instead of acting like a second public init mode"
    );

    let error_log = read_source(&artifacts.join("init.error.txt"));
    assert_contains(
        "fixture init error log",
        &error_log,
        "historical M047 todo fixture only supports project name",
    );
    assert_contains(
        "fixture init error log",
        &error_log,
        todo::TODO_FIXTURE_PACKAGE_NAME,
    );
    assert_contains("fixture init error log", &error_log, "unexpected-name");
    assert!(
        !workspace_dir.join("unexpected-name").exists(),
        "unexpected project name should not leave a generated project behind"
    );
}

#[test]
fn m047_s05_todo_fixture_copy_fails_closed_on_missing_required_file() {
    let artifacts = todo::artifact_dir("fixture-copy-missing-file");
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let fixture_root = temp_dir.path().join("broken-fixture");
    let workspace_dir = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", workspace_dir.display()));
    todo::archive_directory_tree(&todo::todo_fixture_root(), &fixture_root);
    fs::remove_file(fixture_root.join("work.mpl"))
        .unwrap_or_else(|error| panic!("failed to remove fixture work.mpl: {error}"));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        todo::init_todo_project_from_fixture_root(
            &fixture_root,
            &workspace_dir,
            todo::TODO_FIXTURE_PACKAGE_NAME,
            &artifacts,
        )
    }));
    assert!(
        result.is_err(),
        "missing fixture files should fail before any runtime proof starts"
    );

    let error_log = read_source(&artifacts.join("init.error.txt"));
    assert_contains(
        "fixture init error log",
        &error_log,
        "missing required files",
    );
    assert_contains("fixture init error log", &error_log, "work.mpl");
    assert!(
        !workspace_dir.join(todo::TODO_FIXTURE_PACKAGE_NAME).exists(),
        "missing required files should not leave a partial copied project behind"
    );
}

#[test]
fn m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template() {
    let artifacts = todo::artifact_dir("public-surface-contract");
    let readme =
        todo::read_and_archive(&repo_root().join("README.md"), &artifacts.join("README.md"));
    let distributed_proof = todo::read_and_archive(
        &repo_root().join("website/docs/docs/distributed-proof/index.md"),
        &artifacts.join("distributed-proof.index.md"),
    );
    let distributed = todo::read_and_archive(
        &repo_root().join("website/docs/docs/distributed/index.md"),
        &artifacts.join("distributed.index.md"),
    );
    let tooling = todo::read_and_archive(
        &repo_root().join("website/docs/docs/tooling/index.md"),
        &artifacts.join("tooling.index.md"),
    );
    let clustered_example = todo::read_and_archive(
        &repo_root().join("website/docs/docs/getting-started/clustered-example/index.md"),
        &artifacts.join("clustered-example.index.md"),
    );
    let tiny_cluster_work = todo::read_and_archive(
        &repo_root().join("scripts/fixtures/clustered/tiny-cluster/work.mpl"),
        &artifacts.join("tiny-cluster.work.mpl"),
    );
    let cluster_proof_work = todo::read_and_archive(
        &repo_root().join("scripts/fixtures/clustered/cluster-proof/work.mpl"),
        &artifacts.join("cluster-proof.work.mpl"),
    );
    let tooling_e2e = todo::read_and_archive(
        &repo_root().join("compiler/meshc/tests/tooling_e2e.rs"),
        &artifacts.join("tooling_e2e.rs"),
    );

    let current_repo_blob_base = "https://github.com/snowdamiz/mesh-lang/blob/main/";
    let stale_repo_blob_base = "https://github.com/hyperpush-org/hyperpush-mono/blob/main/";
    let todo_postgres_readme_url =
        "https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md";
    let todo_sqlite_readme_url =
        "https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md";
    let mesher_readme_url =
        "https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md";
    let verify_m051_s01 = "bash scripts/verify-m051-s01.sh";
    let verify_m051_s02 = "bash scripts/verify-m051-s02.sh";

    assert_contains("README.md", &readme, "meshc init --clustered hello_cluster");
    assert_contains(
        "README.md",
        &readme,
        "meshc init --template todo-api --db sqlite todo_api",
    );
    assert_contains(
        "README.md",
        &readme,
        "meshc init --template todo-api --db postgres shared_todo",
    );
    assert_contains("README.md", &readme, "@cluster pub fn add()");
    assert_contains("README.md", &readme, "actor-backed write rate limiting");
    assert_contains("README.md", &readme, "Work.add");

    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "This is the only public-secondary docs page that carries the named clustered verifier rails.",
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "ordinary `@cluster pub fn add()` / `@cluster pub fn sync_todos()`-style declaration",
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "/docs/production-backend-proof/",
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "bash scripts/verify-m047-s04.sh",
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "bash scripts/verify-m047-s05.sh",
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "cargo test -p meshc --test e2e_m047_s07 -- --nocapture",
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "bash scripts/verify-m047-s06.sh",
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "bash scripts/verify-m043-s04-fly.sh",
    );
    for needle in [
        "bash scripts/verify-m046-s06.sh",
        "bash scripts/verify-m046-s05.sh",
        "bash scripts/verify-m046-s04.sh",
        "bash scripts/verify-m045-s05.sh",
        "bash scripts/verify-m045-s04.sh",
        "bash scripts/verify-m045-s03.sh",
    ] {
        assert_contains(
            "website/docs/docs/distributed-proof/index.md",
            &distributed_proof,
            needle,
        );
    }
    assert_contains(
        "website/docs/docs/tooling/index.md",
        &tooling,
        "meshc init --template todo-api --db sqlite my_local_todo",
    );
    assert_contains(
        "website/docs/docs/tooling/index.md",
        &tooling,
        "meshc init --template todo-api --db postgres my_shared_todo",
    );
    assert_contains(
        "website/docs/docs/tooling/index.md",
        &tooling,
        "@cluster pub fn sync_todos()",
    );
    assert_contains(
        "website/docs/docs/tooling/index.md",
        &tooling,
        "The SQLite Todo starter is the honest local starter",
    );
    assert_contains(
        "website/docs/docs/tooling/index.md",
        &tooling,
        "/docs/getting-started/clustered-example/",
    );
    assert_contains(
        "website/docs/docs/tooling/index.md",
        &tooling,
        "/docs/production-backend-proof/",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "meshc init --clustered hello_cluster",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "## After the scaffold, pick the follow-on starter",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "meshc init --template todo-api --db sqlite my_local_todo",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "meshc init --template todo-api --db postgres my_shared_todo",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "@cluster pub fn add() -> Int do",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "Node.start_from_env()",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "meshc cluster status",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "meshc cluster continuity",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "meshc cluster diagnostics",
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        todo_sqlite_readme_url,
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        todo_postgres_readme_url,
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        mesher_readme_url,
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        verify_m051_s01,
    );
    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        verify_m051_s02,
    );
    assert_contains(
        "website/docs/docs/getting-started/clustered-example/index.md",
        &clustered_example,
        "/docs/distributed-proof/",
    );
    assert!(
        tiny_cluster_work.contains("@cluster pub fn add()")
            || tiny_cluster_work.contains("@cluster(3) pub fn add()"),
        "expected tiny-cluster/work.mpl to keep the source-first add() declaration, got:\n{tiny_cluster_work}"
    );
    assert_contains(
        "cluster-proof/work.mpl",
        &cluster_proof_work,
        "@cluster pub fn add()",
    );
    assert_contains(
        "compiler/meshc/tests/tooling_e2e.rs",
        &tooling_e2e,
        "@cluster pub fn add()",
    );
    assert_contains(
        "compiler/meshc/tests/tooling_e2e.rs",
        &tooling_e2e,
        "meshc init --template todo-api",
    );

    for (label, source) in [
        (
            "website/docs/docs/distributed-proof/index.md",
            &distributed_proof,
        ),
        ("website/docs/docs/tooling/index.md", &tooling),
        (
            "website/docs/docs/getting-started/clustered-example/index.md",
            &clustered_example,
        ),
    ] {
        assert_contains(label, source, todo_postgres_readme_url);
        assert_contains(label, source, todo_sqlite_readme_url);
        assert_contains(label, source, "/docs/production-backend-proof/");
        assert_contains(label, source, current_repo_blob_base);
        assert_omits(label, source, "reference-backend/README.md");
        assert_omits(label, source, stale_repo_blob_base);
    }

    assert_contains(
        "website/docs/docs/distributed/index.md",
        &distributed,
        mesher_readme_url,
    );
    assert_contains(
        "website/docs/docs/distributed/index.md",
        &distributed,
        verify_m051_s01,
    );
    assert_contains(
        "website/docs/docs/distributed/index.md",
        &distributed,
        verify_m051_s02,
    );

    for (label, source) in [
        (
            "website/docs/docs/distributed-proof/index.md",
            &distributed_proof,
        ),
        ("website/docs/docs/tooling/index.md", &tooling),
    ] {
        assert_contains(label, source, "execute_declared_work(...)");
        assert_contains(label, source, "Work.execute_declared_work");
        assert_contains(label, source, "HTTP.clustered(1, ...)");
        assert_contains(label, source, "GET /todos");
        assert_contains(label, source, "GET /todos/:id");
        assert_contains(label, source, "GET /health");
        assert_contains(label, source, "mutating routes stay local");
        assert_omits(label, source, "HTTP.clustered(...) is still not shipped");
    }

    for (label, source) in [("README.md", &readme)] {
        assert_contains(label, source, "HTTP.clustered(1, ...)");
        assert_contains(label, source, "GET /todos");
        assert_contains(label, source, "GET /todos/:id");
        assert_contains(label, source, "GET /health");
        assert_contains(label, source, "mutating routes stay local");
        assert_omits(label, source, "HTTP.clustered(...) is still not shipped");
    }

    assert_contains(
        "website/docs/docs/distributed-proof/index.md",
        &distributed_proof,
        "e2e_m047_s07",
    );

    for needle in [
        "bash scripts/verify-m047-s04.sh",
        "bash scripts/verify-m047-s05.sh",
        "bash scripts/verify-m047-s06.sh",
        "e2e_m047_s07",
        "bash scripts/verify-m043-s04-fly.sh",
        "bash scripts/verify-m046-s06.sh",
        "bash scripts/verify-m046-s05.sh",
        "bash scripts/verify-m046-s04.sh",
        "bash scripts/verify-m045-s05.sh",
        "bash scripts/verify-m045-s04.sh",
        "bash scripts/verify-m045-s03.sh",
    ] {
        assert_omits("website/docs/docs/tooling/index.md", &tooling, needle);
    }

    for needle in [
        "execute_declared_work(...)",
        "Work.execute_declared_work",
        "scripts/verify-m047-s04.sh",
        "scripts/verify-m047-s05.sh",
        "scripts/verify-m047-s06.sh",
        "e2e_m047_s07",
    ] {
        assert_omits(
            "website/docs/docs/getting-started/clustered-example/index.md",
            &clustered_example,
            needle,
        );
    }

    for (label, source) in [
        ("tiny-cluster/work.mpl", &tiny_cluster_work),
        ("cluster-proof/work.mpl", &cluster_proof_work),
    ] {
        assert_omits(label, source, "execute_declared_work");
        assert_omits(label, source, "Work.execute_declared_work");
    }
}

#[test]
fn m047_s05_assembled_verifier_replays_cutover_and_todo_rails() {
    let artifacts = todo::artifact_dir("assembled-verifier-contract");
    let verifier = todo::read_and_archive(
        &repo_root().join("scripts/verify-m047-s05.sh"),
        &artifacts.join("verify-m047-s05.sh"),
    );

    for needle in [
        "bash scripts/verify-m047-s04.sh",
        "cargo test -p meshc --test e2e_m047_s05 m047_s05_todo_fixture_contract_is_committed_and_helper_is_fixture_backed -- --nocapture",
        "cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture",
        "cargo test -p meshc --test e2e_m047_s05 -- --nocapture",
        "npm --prefix website run build",
        "status.txt",
        "current-phase.txt",
        "phase-report.txt",
        "full-contract.log",
        "latest-proof-bundle.txt",
        "retained-m047-s04-verify",
        "retained-m047-s05-artifacts",
        "m047-s04-replay",
        "m047-s05-pkg",
        "m047-s05-tooling",
        "m047-s05-e2e",
        "m047-s05-docs-build",
        "m047-s05-fixture-provenance",
        "m047-s05-bundle-shape",
        "verify-m047-s05: ok",
        "source=fixture-copy",
        "fixture_root_relative=scripts/fixtures/m047-s05-clustered-todo",
    ] {
        assert_contains("scripts/verify-m047-s05.sh", &verifier, needle);
    }
}
