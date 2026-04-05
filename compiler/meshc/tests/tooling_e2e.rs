//! End-to-end integration tests for all Phase 10 developer tools.
//!
//! Verifies that the meshc binary's developer-facing subcommands work together:
//! - `meshc build --json` produces valid JSON diagnostics for type errors
//! - `meshc fmt` formats files, `meshc fmt --check` verifies formatting
//! - `meshc init` creates a compilable project
//! - `meshc init --clustered` creates a clustered project using only public clustered-app surfaces
//! - `meshc init --template todo-api` creates the current local SQLite Todo API starter
//! - `meshc init --template todo-api --db ...` validates the typed DB-selection seam and the SQLite-local vs Postgres-clustered split
//! - `meshc repl --help` confirms REPL subcommand availability
//! - `meshc lsp --help` confirms LSP subcommand availability

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use support::m051_reference_backend as retained_backend;

/// Locate the repository root from the meshc package manifest directory.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("meshc crate should live under compiler/")
        .parent()
        .expect("workspace root should be above compiler/")
        .to_path_buf()
}

/// Locate the meshc binary built by cargo.
fn meshc_bin() -> PathBuf {
    // CARGO_BIN_EXE_meshc is set by cargo when running integration tests
    // for the meshc package.
    PathBuf::from(env!("CARGO_BIN_EXE_meshc"))
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

fn write_override_entry_test_project(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let project_dir = root.join("override-entry-project");
    let tests_dir = project_dir.join("tests");
    let test_file = tests_dir.join("override_entry.test.mpl");

    write_file(
        &project_dir.join("mesh.toml"),
        "[package]\nname = \"override-entry-project\"\nversion = \"0.1.0\"\nentrypoint = \"lib/start.mpl\"\n",
    );
    write_file(
        &project_dir.join("lib/start.mpl"),
        "from App import answer\n\nfn main() do\n  println(\"app=#{answer()}\")\nend\n",
    );
    write_file(
        &project_dir.join("app.mpl"),
        "pub fn answer() -> Int do\n  42\nend\n",
    );
    write_file(
        &tests_dir.join("support.mpl"),
        "pub fn label() -> String do\n  \"support\"\nend\n",
    );
    write_file(
        &test_file,
        "from App import answer\nfrom Tests.Support import label\n\ntest(\"override entry roots all targets the same way\") do\n  assert(answer() == 42)\n  assert(label() == \"support\")\nend\n",
    );

    (project_dir, tests_dir, test_file)
}

fn assert_meshc_test_target_reports_passes(target: &Path, expected_summary: &str) {
    let output = Command::new(meshc_bin())
        .args(["test", target.to_str().unwrap()])
        .output()
        .expect("failed to run meshc test target");

    assert!(
        output.status.success(),
        "meshc test {} failed:\nstdout: {}\nstderr: {}",
        target.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(expected_summary),
        "expected '{}' for target {}, got:\n{}",
        expected_summary,
        target.display(),
        String::from_utf8_lossy(&output.stdout)
    );
}

fn assert_meshc_test_target_succeeds(target: &Path) {
    assert_meshc_test_target_reports_passes(target, "1 passed");
}

fn assert_local_sqlite_todo_template(project_dir: &Path, starter_name: &str) {
    let manifest_path = project_dir.join("mesh.toml");
    let main_path = project_dir.join("main.mpl");
    let config_path = project_dir.join("config.mpl");
    let work_path = project_dir.join("work.mpl");
    let readme_path = project_dir.join("README.md");
    let dockerfile_path = project_dir.join("Dockerfile");
    let dockerignore_path = project_dir.join(".dockerignore");
    let router_path = project_dir.join("api/router.mpl");
    let todos_path = project_dir.join("api/todos.mpl");
    let health_path = project_dir.join("api/health.mpl");
    let registry_path = project_dir.join("runtime/registry.mpl");
    let limiter_path = project_dir.join("services/rate_limiter.mpl");
    let storage_path = project_dir.join("storage/todos.mpl");
    let todo_type_path = project_dir.join("types/todo.mpl");
    let config_test_path = project_dir.join("tests/config.test.mpl");
    let storage_test_path = project_dir.join("tests/storage.test.mpl");

    for path in [
        &manifest_path,
        &main_path,
        &config_path,
        &readme_path,
        &dockerfile_path,
        &dockerignore_path,
        &router_path,
        &todos_path,
        &health_path,
        &registry_path,
        &limiter_path,
        &storage_path,
        &todo_type_path,
        &config_test_path,
        &storage_test_path,
    ] {
        assert!(path.exists(), "missing generated file {}", path.display());
    }
    assert!(
        !work_path.exists(),
        "sqlite starter should not emit work.mpl at {}",
        work_path.display()
    );

    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    assert!(manifest.contains("[package]"));
    assert!(manifest.contains(starter_name));
    assert!(!manifest.contains("[cluster]"));

    let main = std::fs::read_to_string(&main_path).unwrap();
    assert!(main.contains("todo_db_path_key()"));
    assert!(main.contains("start_rate_limiter"));
    assert!(main.contains("start_registry"));
    assert!(main.contains("ensure_schema"));
    assert!(main.contains("[todo-api] local config loaded"));
    assert!(main.contains("[todo-api] SQLite schema ready"));
    assert!(main.contains("[todo-api] local runtime ready"));
    assert!(main.contains("[todo-api] HTTP server starting on"));
    assert!(main.contains("resolve_db_path()"));
    assert!(main.contains("todo_rate_limit_window_seconds_key()"));
    assert!(main.contains("todo_rate_limit_max_requests_key()"));
    assert!(!main.contains("Node.start_from_env()"));
    assert!(!main.contains("BootstrapStatus"));
    assert!(!main.contains("runtime bootstrap"));
    assert!(!main.contains("HTTP.clustered("));
    assert!(!main.contains("MESH_CLUSTER_"));
    assert!(!main.contains("Work.sync_todos"));

    let config = std::fs::read_to_string(&config_path).unwrap();
    assert!(config.contains("todo_db_path_key"));
    assert!(config.contains("default_todo_db_path"));
    assert!(config.contains("invalid_positive_int"));
    assert!(config.contains("invalid_db_path"));
    assert!(config.contains("invalid_todo_id_message"));
    assert!(config.contains("title_required_message"));
    assert!(config.contains("todo_not_found_message"));

    let router = std::fs::read_to_string(&router_path).unwrap();
    assert!(router.contains("HTTP.on_get(\"/health\", handle_health)"));
    assert!(router.contains("HTTP.on_get(\"/todos\", handle_list_todos)"));
    assert!(router.contains("HTTP.on_get(\"/todos/:id\", handle_get_todo)"));
    assert!(router.contains("HTTP.on_post(\"/todos\", handle_create_todo)"));
    assert!(router.contains("HTTP.on_put(\"/todos/:id\", handle_toggle_todo)"));
    assert!(router.contains("HTTP.on_delete(\"/todos/:id\", handle_delete_todo)"));
    assert!(!router.contains("HTTP.clustered("));

    let todos = std::fs::read_to_string(&todos_path).unwrap();
    assert!(todos.contains("allow_write("));
    assert!(todos.contains("todo_error_response"));
    assert!(todos.contains("invalid_todo_id_message"));
    assert!(todos.contains("title_required_message"));
    assert!(todos.contains("pub fn handle_list_todos(_request :: Request) -> Response do"));
    assert!(todos.contains("pub fn handle_get_todo(request :: Request) -> Response do"));
    assert!(todos.contains("bad_request_response"));
    assert!(!todos.contains("HTTP.clustered("));
    assert!(!todos.contains("Work.sync_todos"));

    let health = std::fs::read_to_string(&health_path).unwrap();
    assert!(health.contains("mode : \"local\""));
    assert!(health.contains("db_backend : \"sqlite\""));
    assert!(health.contains("storage_mode : \"single-node\""));
    assert!(health.contains("db_path : get_db_path()"));
    assert!(!health.contains("clustered_handler"));

    let registry = std::fs::read_to_string(&registry_path).unwrap();
    assert!(registry.contains("Process.register(\"todo_api_registry\""));

    let limiter = std::fs::read_to_string(&limiter_path).unwrap();
    assert!(limiter.contains("service TodoWriteRateLimiter do"));
    assert!(limiter.contains("spawn(rate_window_ticker"));

    let storage = std::fs::read_to_string(&storage_path).unwrap();
    assert!(storage.contains("Todo.from_row(row)"));
    assert!(storage.contains("String.to_int(trimmed)"));
    assert!(storage.contains("CREATE TABLE IF NOT EXISTS todos"));
    assert!(storage.contains("completed INTEGER NOT NULL DEFAULT 0"));
    assert!(storage.contains("title_required_message()"));
    assert!(storage.contains("invalid_todo_id_message()"));
    assert!(storage.contains("todo_not_found_message()"));
    assert!(!storage.contains("Map.get(row,"));

    let todo_type = std::fs::read_to_string(&todo_type_path).unwrap();
    assert!(todo_type.contains("end deriving(Json, Row)"));

    let config_test = std::fs::read_to_string(&config_test_path).unwrap();
    assert!(config_test.contains("describe(\"SQLite todo-api config\")"));
    assert!(config_test.contains("TODO_DB_PATH"));
    assert!(config_test.contains("Invalid TODO_DB_PATH: expected a non-empty path"));

    let storage_test = std::fs::read_to_string(&storage_test_path).unwrap();
    assert!(storage_test.contains("describe(\"SQLite todo storage\")"));
    assert!(storage_test.contains("sample_todo"));
    assert!(storage_test.contains("create_todo"));
    assert!(storage_test.contains("list_todos"));
    assert!(storage_test.contains("ensure_schema"));

    let readme = std::fs::read_to_string(&readme_path).unwrap();
    assert!(readme
        .contains("This project was generated by `meshc init --template todo-api --db sqlite`."));
    assert!(!readme.contains("This project was generated by `meshc init --template todo-api`."));
    assert!(readme.contains("single-node SQLite Todo API"));
    assert!(readme.contains("meshc test ."));
    assert!(readme.contains("meshc init --template todo-api --db postgres my-shared-todo"));
    assert!(readme.contains("meshc init --clustered my-clustered-app"));
    assert!(readme.contains(
        "there is no `work.mpl`, `HTTP.clustered(...)`, or `meshc cluster` story in this starter"
    ));
    assert!(readme.contains("TODO_DB_PATH"));
    assert!(readme.contains(&format!("docker build -t {} .", starter_name)));
    assert!(!readme.contains("Node.start_from_env()"));
    assert!(!readme.contains("Work.sync_todos"));
    assert!(!readme.contains("meshc cluster status"));
    assert!(!readme.contains("MESH_CLUSTER_"));

    let dockerfile = std::fs::read_to_string(&dockerfile_path).unwrap();
    assert!(dockerfile.contains("FROM ubuntu:24.04"));
    assert!(dockerfile.contains(&format!("COPY output /usr/local/bin/{}", starter_name)));
    assert!(dockerfile.contains(&format!("ENTRYPOINT [\"/usr/local/bin/{}\"]", starter_name)));
    assert!(dockerfile.contains("EXPOSE 8080"));
    assert!(!dockerfile.contains("4370"));
    assert!(!dockerfile.contains("MESH_CLUSTER_PORT"));

    let dockerignore = std::fs::read_to_string(&dockerignore_path).unwrap();
    assert!(dockerignore.contains("*.sqlite3"));
    assert!(dockerignore.contains("target"));
}

// ── Error messages (--json) ──────────────────────────────────────────

#[test]
fn test_build_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    // Write a Mesh file with a type error (assigning string to Int annotation).
    std::fs::write(project.join("main.mpl"), "let x :: Int = \"hello\"\n").unwrap();

    let output = Command::new(meshc_bin())
        .args(["build", "--json", project.to_str().unwrap()])
        .output()
        .expect("failed to run meshc build --json");

    assert!(
        !output.status.success(),
        "Expected build to fail on type error"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // stderr contains concatenated JSON objects. Use a streaming deserializer
    // to extract the first one (the type error diagnostic).
    let mut stream = serde_json::Deserializer::from_str(&stderr).into_iter::<serde_json::Value>();
    let json = stream
        .next()
        .expect("no JSON object in stderr")
        .expect("first JSON object is not valid");

    // Verify required JSON fields.
    assert!(json.get("code").is_some(), "JSON missing 'code' field");
    assert!(
        json.get("severity").is_some(),
        "JSON missing 'severity' field"
    );
    assert!(
        json.get("message").is_some(),
        "JSON missing 'message' field"
    );
    assert!(json.get("spans").is_some(), "JSON missing 'spans' field");

    // Verify the error code starts with E (type error).
    let code = json["code"].as_str().unwrap();
    assert!(
        code.starts_with('E'),
        "Expected error code starting with E, got: {}",
        code
    );

    // Verify spans array is non-empty.
    let spans = json["spans"].as_array().unwrap();
    assert!(!spans.is_empty(), "Expected at least one span");

    // Verify no ANSI escape codes in output.
    assert!(
        !stderr.contains("\x1b["),
        "JSON mode should not contain ANSI escape codes"
    );
}

// ── Formatter ────────────────────────────────────────────────────────

#[test]
fn test_fmt_formats_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.mpl");

    // Write an unformatted Mesh file (no spaces around operator, no indentation).
    std::fs::write(&file, "fn add(a,b) do\na+b\nend").unwrap();

    let output = Command::new(meshc_bin())
        .args(["fmt", file.to_str().unwrap()])
        .output()
        .expect("failed to run meshc fmt");

    assert!(
        output.status.success(),
        "meshc fmt failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = std::fs::read_to_string(&file).unwrap();

    // Verify the file was reformatted (canonical 2-space indent, spaces around ops).
    assert_eq!(contents, "fn add(a, b) do\n  a + b\nend\n");
}

#[test]
fn test_fmt_check_formatted() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("good.mpl");

    // Write an already-formatted file.
    std::fs::write(&file, "fn add(a, b) do\n  a + b\nend\n").unwrap();

    let output = Command::new(meshc_bin())
        .args(["fmt", "--check", file.to_str().unwrap()])
        .output()
        .expect("failed to run meshc fmt --check");

    assert!(
        output.status.success(),
        "Expected exit 0 for already-formatted file, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_fmt_check_unformatted() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.mpl");

    // Write an unformatted file.
    std::fs::write(&file, "fn bad(a,b) do\na+b\nend").unwrap();

    let output = Command::new(meshc_bin())
        .args(["fmt", "--check", file.to_str().unwrap()])
        .output()
        .expect("failed to run meshc fmt --check");

    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit 1 for unformatted file"
    );

    // File should NOT be modified in check mode.
    let contents = std::fs::read_to_string(&file).unwrap();
    assert_eq!(contents, "fn bad(a,b) do\na+b\nend");
}

#[test]
fn test_fmt_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("idem.mpl");

    // Write an unformatted file.
    std::fs::write(&file, "fn foo(x) do\nlet y = x\ny\nend").unwrap();

    // Format once.
    let output1 = Command::new(meshc_bin())
        .args(["fmt", file.to_str().unwrap()])
        .output()
        .expect("failed to run meshc fmt (first pass)");
    assert!(output1.status.success(), "First format pass failed");

    let after_first = std::fs::read_to_string(&file).unwrap();

    // Format again.
    let output2 = Command::new(meshc_bin())
        .args(["fmt", file.to_str().unwrap()])
        .output()
        .expect("failed to run meshc fmt (second pass)");
    assert!(output2.status.success(), "Second format pass failed");

    let after_second = std::fs::read_to_string(&file).unwrap();

    // Both passes should produce identical output.
    assert_eq!(
        after_first, after_second,
        "Formatting is not idempotent!\nFirst pass:\n{}\nSecond pass:\n{}",
        after_first, after_second
    );

    // Additionally verify --check agrees the file is formatted.
    let check = Command::new(meshc_bin())
        .args(["fmt", "--check", file.to_str().unwrap()])
        .output()
        .expect("failed to run meshc fmt --check after formatting");
    assert!(
        check.status.success(),
        "fmt --check disagrees after formatting: {}",
        String::from_utf8_lossy(&check.stderr)
    );
}

// ── Test runner ──────────────────────────────────────────────────────

#[test]
fn test_test_runs_tests_directory_target() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("proj");
    let tests_dir = project.join("tests");

    std::fs::create_dir_all(&tests_dir).unwrap();
    std::fs::write(
        project.join("main.mpl"),
        "fn main() do\n  println(\"app\")\nend\n",
    )
    .unwrap();
    std::fs::write(
        project.join("math.mpl"),
        "pub fn answer() -> Int do\n  42\nend\n",
    )
    .unwrap();
    std::fs::write(
        tests_dir.join("math.test.mpl"),
        "from Math import answer\n\ntest(\"directory target runs project tests\") do\n  assert(answer() == 42)\nend\n",
    )
    .unwrap();

    let output = Command::new(meshc_bin())
        .args(["test", tests_dir.to_str().unwrap()])
        .output()
        .expect("failed to run meshc test on tests directory");

    assert!(
        output.status.success(),
        "meshc test <tests-dir> failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 passed"),
        "expected a passing file-level summary for directory-target execution, got:\n{}",
        stdout
    );
}

#[test]
fn test_test_project_directory_target_succeeds_for_override_entry_without_root_main() {
    let dir = tempfile::tempdir().unwrap();
    let (project_dir, _, _) = write_override_entry_test_project(dir.path());

    assert_meshc_test_target_succeeds(&project_dir);
}

#[test]
fn test_test_tests_directory_target_succeeds_for_override_entry_without_root_main() {
    let dir = tempfile::tempdir().unwrap();
    let (_, tests_dir, _) = write_override_entry_test_project(dir.path());

    assert_meshc_test_target_succeeds(&tests_dir);
}

#[test]
fn test_test_specific_file_target_succeeds_for_override_entry_without_root_main() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, test_file) = write_override_entry_test_project(dir.path());

    assert_meshc_test_target_succeeds(&test_file);
}

#[test]
fn test_test_specific_file_target_fails_closed_when_no_project_root_exists() {
    let dir = tempfile::tempdir().unwrap();
    let orphan = dir.path().join("orphan.test.mpl");
    write_file(&orphan, "test(\"orphan\") do\n  assert(true)\nend\n");

    let output = Command::new(meshc_bin())
        .args(["test", orphan.to_str().unwrap()])
        .output()
        .expect("failed to run meshc test on orphan test file");

    assert!(
        !output.status.success(),
        "meshc test should fail closed for orphan file target:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Could not resolve a Mesh project root"),
        "expected truthful wrong-root error, got:\n{}",
        stderr
    );
    assert!(stderr.contains(orphan.to_str().unwrap()), "{}", stderr);
}

#[test]
fn test_test_reference_backend_project_directory_succeeds() {
    let root = repo_root();
    let target = retained_backend::RETAINED_FIXTURE_ROOT_RELATIVE;

    let output = Command::new(meshc_bin())
        .current_dir(&root)
        .args(["test", target])
        .output()
        .expect("failed to run meshc test on the retained reference-backend fixture");

    assert!(
        output.status.success(),
        "meshc test {} failed:\nstdout: {}\nstderr: {}",
        target,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("2 passed"),
        "expected meshc test {} to execute two passing retained fixture test files, got:\n{}",
        target,
        stdout
    );
}

#[test]
fn test_test_coverage_reports_unsupported_contract() {
    let root = repo_root();
    let target = retained_backend::RETAINED_FIXTURE_ROOT_RELATIVE;

    let output = Command::new(meshc_bin())
        .current_dir(&root)
        .args(["test", "--coverage", target])
        .output()
        .expect("failed to run meshc test --coverage on the retained reference-backend fixture");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected unsupported coverage contract to exit 1, got stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("coverage reporting is not implemented for `meshc test`; run the command without --coverage"),
        "expected honest unsupported coverage message, got:\n{}",
        stderr
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("Coverage reporting coming soon"),
        "coverage command should no longer claim success with a placeholder"
    );
}

// ── Package manager ──────────────────────────────────────────────────

#[test]
fn test_init_creates_project() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(meshc_bin())
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init");

    assert!(
        output.status.success(),
        "meshc init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify mesh.toml exists.
    let toml_path = dir.path().join("test-project").join("mesh.toml");
    assert!(
        toml_path.exists(),
        "mesh.toml not created at {}",
        toml_path.display()
    );

    // Verify main.mpl exists.
    let main_path = dir.path().join("test-project").join("main.mpl");
    assert!(
        main_path.exists(),
        "main.mpl not created at {}",
        main_path.display()
    );

    // Verify mesh.toml contains the project name.
    let toml_contents = std::fs::read_to_string(&toml_path).unwrap();
    assert!(
        toml_contents.contains("test-project"),
        "mesh.toml does not contain project name"
    );
}

#[test]
fn test_init_clustered_creates_project() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(meshc_bin())
        .args(["init", "--clustered", "clustered-project"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init --clustered");

    assert!(
        output.status.success(),
        "meshc init --clustered failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let project_dir = dir.path().join("clustered-project");
    let toml_path = project_dir.join("mesh.toml");
    let main_path = project_dir.join("main.mpl");
    let work_path = project_dir.join("work.mpl");
    let readme_path = project_dir.join("README.md");

    assert!(toml_path.exists(), "clustered mesh.toml should exist");
    assert!(main_path.exists(), "clustered main.mpl should exist");
    assert!(work_path.exists(), "clustered work.mpl should exist");
    assert!(readme_path.exists(), "clustered README.md should exist");

    let toml_contents = std::fs::read_to_string(&toml_path).unwrap();
    assert!(toml_contents.contains("[package]"));
    assert!(toml_contents.contains("clustered-project"));
    assert!(!toml_contents.contains("[cluster]"));
    assert!(!toml_contents.contains("Work.execute_declared_work"));

    let main_contents = std::fs::read_to_string(&main_path).unwrap();
    assert_eq!(main_contents.matches("Node.start_from_env()").count(), 1);
    assert!(main_contents.contains("BootstrapStatus"));
    assert!(main_contents.contains("runtime bootstrap"));
    assert!(main_contents.contains("runtime bootstrap failed"));
    assert!(!main_contents.contains("Continuity.submit_declared_work"));
    assert!(!main_contents.contains("HTTP.serve("));
    assert!(!main_contents.contains("/health"));
    assert!(!main_contents.contains("/work"));
    assert!(!main_contents.contains("Env.get_int"));
    assert!(!main_contents.contains("Node.start("));
    assert!(!main_contents.contains("CLUSTER_PROOF_"));

    let work_contents = std::fs::read_to_string(&work_path).unwrap();
    assert!(work_contents.contains("@cluster pub fn add()"));
    assert!(!work_contents.contains("execute_declared_work"));
    assert!(!work_contents.contains("request_key"));
    assert!(!work_contents.contains("attempt_id"));
    assert!(!work_contents.contains("declared_work_runtime_name"));
    assert!(!work_contents.contains("clustered(work)"));
    assert!(work_contents.contains("1 + 1"));
    assert!(!work_contents.contains("declared_work_target"));
    assert!(!work_contents.contains("Continuity.submit_declared_work"));
    assert!(!work_contents.contains("Continuity.mark_completed"));
    assert!(!work_contents.contains("Timer.sleep"));
    assert!(!work_contents.contains("owner_node"));
    assert!(!work_contents.contains("replica_node"));

    let readme_contents = std::fs::read_to_string(&readme_path).unwrap();
    assert!(
        readme_contents.contains("mesh.toml` is package-only and intentionally omits `[cluster]`")
    );
    assert!(readme_contents.contains("Node.start_from_env()"));
    assert!(readme_contents.contains("@cluster pub fn add()"));
    assert!(readme_contents.contains("Work.add"));
    assert!(readme_contents.contains("1 + 1"));
    assert!(readme_contents.contains("meshc cluster status"));
    assert!(readme_contents.contains("meshc cluster continuity <node-name@host:port> --json"));
    assert!(readme_contents
        .contains("meshc cluster continuity <node-name@host:port> <request-key> --json"));
    assert!(readme_contents.contains("meshc cluster diagnostics"));
    assert!(readme_contents.contains("MESH_CONTINUITY_ROLE"));
    assert!(readme_contents.contains("MESH_CONTINUITY_PROMOTION_EPOCH"));
    assert!(
        readme_contents.contains("automatically starts the source-declared `@cluster` function")
    );
    assert!(!readme_contents.contains("declared_work_runtime_name()"));
    assert!(!readme_contents.contains("clustered(work)"));
    assert!(!readme_contents.contains("Continuity.submit_declared_work"));
    assert!(!readme_contents.contains("HTTP.serve("));
    assert!(!readme_contents.contains("HTTP.clustered("));
    assert!(!readme_contents.contains("/health"));
    assert!(!readme_contents.contains("/work"));
    assert!(!readme_contents.contains("Timer.sleep"));
    assert!(!readme_contents.contains("CLUSTER_PROOF_"));
}

#[test]
fn test_init_clustered_rejects_existing_directory() {
    let dir = tempfile::tempdir().unwrap();

    let first = Command::new(meshc_bin())
        .args(["init", "--clustered", "clustered-project"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run initial meshc init --clustered");
    assert!(
        first.status.success(),
        "initial meshc init --clustered failed: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = Command::new(meshc_bin())
        .args(["init", "--clustered", "clustered-project"])
        .current_dir(dir.path())
        .output()
        .expect("failed to rerun meshc init --clustered");

    assert!(
        !second.status.success(),
        "rerunning meshc init --clustered should fail cleanly"
    );

    let stderr = String::from_utf8_lossy(&second.stderr);
    let stdout = String::from_utf8_lossy(&second.stdout);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        combined.contains("already exists"),
        "expected existing-directory collision message, got:\n{}",
        combined
    );
}

#[test]
fn test_init_todo_template_db_sqlite_default_without_flag_stays_local() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(meshc_bin())
        .args(["init", "--template", "todo-api", "todo-starter"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init --template todo-api");

    assert!(
        output.status.success(),
        "meshc init --template todo-api failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let project_dir = dir.path().join("todo-starter");
    assert_local_sqlite_todo_template(&project_dir, "todo-starter");
    assert_meshc_test_target_reports_passes(&project_dir, "2 passed");
}

#[test]
fn test_init_todo_template_db_sqlite_rejects_existing_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("todo-starter")).unwrap();

    let output = Command::new(meshc_bin())
        .args(["init", "--template", "todo-api", "todo-starter"])
        .current_dir(dir.path())
        .output()
        .expect("failed to rerun meshc init --template todo-api");

    assert!(
        !output.status.success(),
        "rerunning meshc init --template todo-api should fail cleanly"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        combined.contains("already exists"),
        "expected existing-directory collision message, got:\n{}",
        combined
    );
}

#[test]
fn test_init_todo_template_db_sqlite_explicit_flag_matches_local_default_contract() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(meshc_bin())
        .args([
            "init",
            "--template",
            "todo-api",
            "--db",
            "sqlite",
            "todo-starter",
        ])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init --template todo-api --db sqlite");

    assert!(
        output.status.success(),
        "meshc init --template todo-api --db sqlite failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let project_dir = dir.path().join("todo-starter");
    assert_local_sqlite_todo_template(&project_dir, "todo-starter");
}

#[test]
fn test_init_todo_template_db_sqlite_rejects_usage_without_todo_template() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("plain-project");

    let output = Command::new(meshc_bin())
        .args(["init", "--db", "sqlite", "plain-project"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init --db sqlite without template");

    assert!(
        !output.status.success(),
        "meshc init --db sqlite without --template todo-api should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`--db` is only supported"), "{}", stderr);
    assert!(stderr.contains("--template todo-api"), "{}", stderr);
    assert!(
        stderr.contains("sqlite stays the local default"),
        "{}",
        stderr
    );
    assert!(
        !project_dir.exists(),
        "unexpected project created at {}",
        project_dir.display()
    );
}

#[test]
fn test_init_todo_template_db_sqlite_rejects_clustered_todo_template_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("todo-starter");

    let output = Command::new(meshc_bin())
        .args([
            "init",
            "--clustered",
            "--template",
            "todo-api",
            "--db",
            "sqlite",
            "todo-starter",
        ])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init --clustered --template todo-api --db sqlite");

    assert!(
        !output.status.success(),
        "clustered todo-api init should fail instead of silently ignoring flags"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`meshc init --clustered` cannot be combined"),
        "{}",
        stderr
    );
    assert!(
        stderr.contains("--template todo-api --db postgres"),
        "{}",
        stderr
    );
    assert!(
        stderr.contains("meshc init --clustered <name>"),
        "{}",
        stderr
    );
    assert!(
        !project_dir.exists(),
        "unexpected project created at {}",
        project_dir.display()
    );
}

#[test]
fn test_init_todo_template_db_rejects_unknown_values_before_generation() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("todo-starter");

    let output = Command::new(meshc_bin())
        .args([
            "init",
            "--template",
            "todo-api",
            "--db",
            "mysql",
            "todo-starter",
        ])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init --template todo-api --db mysql");

    assert!(
        !output.status.success(),
        "unknown todo-api db values should fail during clap parsing"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value 'mysql'"), "{}", stderr);
    assert!(stderr.contains("--db <DB>"), "{}", stderr);
    assert!(
        !project_dir.exists(),
        "unexpected project created at {}",
        project_dir.display()
    );
}

#[test]
fn test_init_todo_template_postgres_creates_migration_first_project() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("todo-starter");

    let output = Command::new(meshc_bin())
        .args([
            "init",
            "--template",
            "todo-api",
            "--db",
            "postgres",
            "todo-starter",
        ])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init --template todo-api --db postgres");

    assert!(
        output.status.success(),
        "meshc init --template todo-api --db postgres failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let migration_dir = project_dir.join("migrations");
    let migration_entries: Vec<_> = std::fs::read_dir(&migration_dir)
        .expect("failed to read generated migrations dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(
        migration_entries.len(),
        1,
        "expected exactly one generated migration, got {}",
        migration_entries.len()
    );

    let migration_name = migration_entries[0]
        .file_name()
        .to_string_lossy()
        .to_string();
    assert!(
        migration_name.ends_with("_create_todos.mpl"),
        "unexpected migration filename: {}",
        migration_name
    );

    for path in [
        project_dir.join("mesh.toml"),
        project_dir.join("main.mpl"),
        project_dir.join("work.mpl"),
        project_dir.join("config.mpl"),
        project_dir.join("README.md"),
        project_dir.join("Dockerfile"),
        project_dir.join(".dockerignore"),
        project_dir.join(".env.example"),
        project_dir.join("api/router.mpl"),
        project_dir.join("api/todos.mpl"),
        project_dir.join("api/health.mpl"),
        project_dir.join("runtime/registry.mpl"),
        project_dir.join("services/rate_limiter.mpl"),
        project_dir.join("storage/todos.mpl"),
        project_dir.join("tests/config.test.mpl"),
        project_dir.join("types/todo.mpl"),
        migration_entries[0].path(),
    ] {
        assert!(path.exists(), "missing generated file {}", path.display());
    }

    let main = std::fs::read_to_string(project_dir.join("main.mpl")).unwrap();
    assert!(main.contains("Node.start_from_env()"));
    assert!(main.contains("Pool.open(database_url, 1, 4, 5000)"));
    assert!(main.contains("database_url_key()"));
    assert!(main.contains("[todo-api] PostgreSQL pool ready"));
    assert!(main.contains("HTTP.serve(router, port)"));
    assert!(!main.contains("TODO_DB_PATH"));
    assert!(!main.contains("ensure_schema"));

    let config = std::fs::read_to_string(project_dir.join("config.mpl")).unwrap();
    assert!(config.contains("database_url_key"));
    assert!(config.contains("\"DATABASE_URL\""));
    assert!(config.contains("todo_rate_limit_window_seconds_key"));
    assert!(config.contains("todo_rate_limit_max_requests_key"));
    assert!(config.contains("Missing required environment variable"));

    let storage = std::fs::read_to_string(project_dir.join("storage/todos.mpl")).unwrap();
    assert!(storage.contains("Query.from(todos_table())"));
    assert!(storage.contains("Repo.insert_expr"));
    assert!(storage.contains("Repo.update_where_expr"));
    assert!(storage.contains("Repo.delete_where"));
    assert!(storage.contains("Pg.uuid(Expr.value(id))"));
    assert!(!storage.contains("Sqlite.open"));
    assert!(!storage.contains("CREATE TABLE IF NOT EXISTS todos"));

    let migration = std::fs::read_to_string(migration_entries[0].path()).unwrap();
    assert!(migration.contains("# Migration: create_todos"));
    assert!(migration.contains("Pg.create_extension(pool, \"pgcrypto\")"));
    assert!(migration.contains("Migration.create_table(pool,"));
    assert!(migration.contains("Migration.create_index(pool, \"todos\", [\"created_at:DESC\"], \"name:idx_todos_created_at\")"));
    assert!(!migration.contains("CREATE TABLE IF NOT EXISTS todos"));

    let readme = std::fs::read_to_string(project_dir.join("README.md")).unwrap();
    assert!(readme.contains("meshc init --template todo-api --db postgres"));
    assert!(readme.contains("meshc migrate . up"));
    assert!(readme.contains("DATABASE_URL"));
    assert!(readme.contains(".env.example"));
    assert!(readme.contains("packages the binary produced by `meshc build .`"));
    assert!(readme.contains("does not run migrations or create schema at startup"));
}

#[test]
fn test_init_todo_template_postgres_omits_sqlite_contract_markers() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("todo-starter");

    let output = Command::new(meshc_bin())
        .args([
            "init",
            "--template",
            "todo-api",
            "--db",
            "postgres",
            "todo-starter",
        ])
        .current_dir(dir.path())
        .output()
        .expect("failed to run meshc init --template todo-api --db postgres");

    assert!(
        output.status.success(),
        "meshc init --template todo-api --db postgres failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let readme = std::fs::read_to_string(project_dir.join("README.md")).unwrap();
    let dockerfile = std::fs::read_to_string(project_dir.join("Dockerfile")).unwrap();
    let dockerignore = std::fs::read_to_string(project_dir.join(".dockerignore")).unwrap();
    let env_example = std::fs::read_to_string(project_dir.join(".env.example")).unwrap();
    let health = std::fs::read_to_string(project_dir.join("api/health.mpl")).unwrap();

    assert!(!readme.contains("TODO_DB_PATH"));
    assert!(!readme.contains("todo.sqlite3"));
    assert!(!readme.contains("ensure_schema"));
    assert!(!readme.contains("failover"));

    assert!(!dockerfile.contains("TODO_DB_PATH"));
    assert!(!dockerfile.contains("sqlite3"));
    assert!(!dockerfile.contains("VOLUME"));
    assert!(dockerfile.contains("COPY output /usr/local/bin/todo-starter"));

    assert!(!dockerignore.contains("*.sqlite3"));
    assert!(dockerignore.contains(".env"));

    assert!(env_example.contains("DATABASE_URL="));
    assert!(env_example.contains("TODO_RATE_LIMIT_WINDOW_SECONDS=60"));
    assert!(env_example.contains("TODO_RATE_LIMIT_MAX_REQUESTS=5"));
    assert!(!env_example.contains("TODO_DB_PATH"));

    assert!(health.contains("db_backend : \"postgres\""));
    assert!(health.contains("migration_strategy : \"meshc migrate\""));
    assert!(!health.contains("DATABASE_URL"));
}

// ── Update ───────────────────────────────────────────────────────────

#[test]
fn test_update_command_is_listed_in_meshc_help() {
    let output = Command::new(meshc_bin())
        .arg("--help")
        .output()
        .expect("failed to run meshc --help");

    assert!(
        output.status.success(),
        "meshc --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("update"),
        "meshc --help should list the update subcommand, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Refresh installed meshc and meshpkg"),
        "meshc --help should describe the update subcommand honestly, got:\n{}",
        stdout
    );
}

#[test]
fn test_update_subcommand_help_mentions_canonical_installer_path() {
    let output = Command::new(meshc_bin())
        .args(["update", "--help"])
        .output()
        .expect("failed to run meshc update --help");

    assert!(
        output.status.success(),
        "meshc update --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Refresh installed meshc and meshpkg"),
        "meshc update --help should explain the toolchain surface, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("canonical installer path"),
        "meshc update --help should mention the canonical installer path, got:\n{}",
        stdout
    );
}

// ── REPL ─────────────────────────────────────────────────────────────

#[test]
fn test_repl_help() {
    let output = Command::new(meshc_bin())
        .args(["repl", "--help"])
        .output()
        .expect("failed to run meshc repl --help");

    assert!(
        output.status.success(),
        "meshc repl --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Help text should mention REPL or interactive.
    let mentions_repl =
        stdout.to_lowercase().contains("repl") || stdout.to_lowercase().contains("interactive");
    assert!(
        mentions_repl,
        "repl --help should mention REPL or interactive, got:\n{}",
        stdout
    );
}

// ── LSP ──────────────────────────────────────────────────────────────

#[test]
fn test_lsp_subcommand_exists() {
    let output = Command::new(meshc_bin())
        .args(["lsp", "--help"])
        .output()
        .expect("failed to run meshc lsp --help");

    assert!(
        output.status.success(),
        "meshc lsp --help should exit 0, got: {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}
