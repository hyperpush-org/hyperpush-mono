use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mesh_rt::db::pg::{native_pg_close, native_pg_connect, native_pg_execute, native_pg_query};
use serde_json::Value;

const REFERENCE_BACKEND_MIGRATION_VERSION: i64 = 20260323010000;
const REFERENCE_BACKEND_MIGRATION_NAME: &str = "create_jobs";

type DbRow = HashMap<String, String>;

#[derive(Clone, Copy, Debug)]
struct ReferenceBackendConfig {
    port: u16,
    job_poll_ms: u64,
}

struct SpawnedReferenceBackend {
    child: Child,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

struct StoppedReferenceBackend {
    stdout: String,
    stderr: String,
    combined: String,
}

struct SpawnedReferenceBackendPair {
    config_a: ReferenceBackendConfig,
    config_b: ReferenceBackendConfig,
    backend_a: SpawnedReferenceBackend,
    backend_b: SpawnedReferenceBackend,
}

struct HttpResponse {
    status_code: u16,
    body: String,
    raw: String,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_meshc() -> PathBuf {
    let mut path = std::env::current_exe()
        .expect("cannot find current exe")
        .parent()
        .expect("cannot find parent dir")
        .to_path_buf();

    if path.file_name().map_or(false, |n| n == "deps") {
        path = path.parent().unwrap().to_path_buf();
    }

    let meshc = path.join("meshc");
    assert!(
        meshc.exists(),
        "meshc binary not found at {}. Run `cargo build -p meshc` first.",
        meshc.display()
    );
    meshc
}

fn build_reference_backend() -> Output {
    let root = repo_root();
    let meshc = find_meshc();
    Command::new(&meshc)
        .current_dir(&root)
        .args(["build", "reference-backend"])
        .output()
        .expect("failed to invoke meshc build for reference-backend")
}

fn assert_reference_backend_build_succeeds() {
    let output = build_reference_backend();
    assert!(
        output.status.success(),
        "meshc build reference-backend failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn reference_backend_binary() -> PathBuf {
    repo_root()
        .join("reference-backend")
        .join("reference-backend")
}

fn pick_unused_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("failed to bind ephemeral port")
        .local_addr()
        .expect("failed to read ephemeral port")
        .port()
}

fn reference_backend_test_config(job_poll_ms: u64) -> ReferenceBackendConfig {
    ReferenceBackendConfig {
        port: pick_unused_port(),
        job_poll_ms,
    }
}

fn run_reference_backend_migration(database_url: &str, command: &str) -> Output {
    let root = repo_root();
    let meshc = find_meshc();
    Command::new(&meshc)
        .current_dir(&root)
        .env("DATABASE_URL", database_url)
        .args(["migrate", "reference-backend", command])
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "failed to invoke meshc migrate reference-backend {}: {}",
                command, e
            )
        })
}

fn assert_reference_backend_migration_succeeds(database_url: &str, command: &str) {
    let output = run_reference_backend_migration(database_url, command);
    assert_command_success(
        &output,
        &format!("meshc migrate reference-backend {command}"),
    );
}

fn command_output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_command_success(output: &Output, description: &str) {
    assert!(
        output.status.success(),
        "{description} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn reference_backend_log_paths() -> (PathBuf, PathBuf) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let base = std::env::temp_dir();
    let stdout_path = base.join(format!("reference-backend-{stamp}-stdout.log"));
    let stderr_path = base.join(format!("reference-backend-{stamp}-stderr.log"));
    (stdout_path, stderr_path)
}

fn spawn_reference_backend(
    database_url: &str,
    config: ReferenceBackendConfig,
) -> SpawnedReferenceBackend {
    let binary = reference_backend_binary();
    let (stdout_path, stderr_path) = reference_backend_log_paths();
    let stdout_file = File::create(&stdout_path)
        .unwrap_or_else(|e| panic!("failed to create {}: {}", stdout_path.display(), e));
    let stderr_file = File::create(&stderr_path)
        .unwrap_or_else(|e| panic!("failed to create {}: {}", stderr_path.display(), e));

    let child = Command::new(&binary)
        .current_dir(repo_root())
        .env("DATABASE_URL", database_url)
        .env("PORT", config.port.to_string())
        .env("JOB_POLL_MS", config.job_poll_ms.to_string())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {}", binary.display(), e));

    SpawnedReferenceBackend {
        child,
        stdout_path,
        stderr_path,
    }
}

fn spawn_staged_reference_backend(
    bundle_dir: &Path,
    database_url: &str,
    config: ReferenceBackendConfig,
) -> SpawnedReferenceBackend {
    let binary = bundle_dir.join("reference-backend");
    let (stdout_path, stderr_path) = reference_backend_log_paths();
    let stdout_file = File::create(&stdout_path)
        .unwrap_or_else(|e| panic!("failed to create {}: {}", stdout_path.display(), e));
    let stderr_file = File::create(&stderr_path)
        .unwrap_or_else(|e| panic!("failed to create {}: {}", stderr_path.display(), e));

    let child = Command::new(&binary)
        .current_dir(bundle_dir)
        .env("DATABASE_URL", database_url)
        .env("PORT", config.port.to_string())
        .env("JOB_POLL_MS", config.job_poll_ms.to_string())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn staged binary {}: {}", binary.display(), e));

    SpawnedReferenceBackend {
        child,
        stdout_path,
        stderr_path,
    }
}

fn spawn_reference_backend_pair(
    database_url: &str,
    job_poll_ms: u64,
) -> SpawnedReferenceBackendPair {
    let config_a = reference_backend_test_config(job_poll_ms);
    let config_b = reference_backend_test_config(job_poll_ms);
    assert_ne!(
        config_a.port, config_b.port,
        "two-instance harness must use unique ports"
    );

    let backend_a = spawn_reference_backend(database_url, config_a);
    let backend_b = spawn_reference_backend(database_url, config_b);

    SpawnedReferenceBackendPair {
        config_a,
        config_b,
        backend_a,
        backend_b,
    }
}

fn send_http_request(
    config: &ReferenceBackendConfig,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(("127.0.0.1", config.port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;

    let request = match body {
        Some(body) => format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.as_bytes().len(),
            body
        ),
        None => format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        ),
    };

    stream.write_all(request.as_bytes())?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;

    let mut parts = raw.splitn(2, "\r\n\r\n");
    let headers = parts.next().unwrap_or("");
    let body = parts.next().unwrap_or("").to_string();
    let status_code = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);

    Ok(HttpResponse {
        status_code,
        body,
        raw,
    })
}

fn assert_json_response(response: HttpResponse, expected_status: u16, description: &str) -> Value {
    assert!(
        response.status_code == expected_status,
        "expected HTTP {expected_status} for {description}, got raw response:\n{}",
        response.raw
    );
    serde_json::from_str(&response.body).unwrap_or_else(|e| {
        panic!(
            "expected JSON body for {description}, got parse error {e}: {}",
            response.body
        )
    })
}

fn get_json(config: &ReferenceBackendConfig, path: &str, expected_status: u16) -> Value {
    let response = send_http_request(config, "GET", path, None)
        .unwrap_or_else(|e| panic!("GET {path} failed on {}: {}", config.port, e));
    assert_json_response(response, expected_status, path)
}

fn post_json(
    config: &ReferenceBackendConfig,
    path: &str,
    body: &str,
    expected_status: u16,
) -> Value {
    let response = send_http_request(config, "POST", path, Some(body))
        .unwrap_or_else(|e| panic!("POST {path} failed on {}: {}", config.port, e));
    assert_json_response(response, expected_status, path)
}

fn wait_for_reference_backend(config: &ReferenceBackendConfig) -> Value {
    let mut last_health = Value::Null;

    for attempt in 0..40 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(250));
        }

        match send_http_request(config, "GET", "/health", None) {
            Ok(response) if response.status_code == 200 => {
                let health = assert_json_response(response, 200, "/health");
                let overall_status = health["status"].as_str().unwrap_or("");
                let liveness = health["worker"]["liveness"].as_str().unwrap_or("");
                let recovery_active = health["worker"]["recovery_active"].as_bool().unwrap_or(true);

                if overall_status == "ok" && liveness == "healthy" && !recovery_active {
                    return health;
                }

                last_health = health;
            }
            Ok(_) | Err(_) => continue,
        }
    }

    panic!(
        "reference-backend never reached healthy startup on :{}; last_health={}",
        config.port, last_health
    );
}

fn collect_stopped_reference_backend(
    mut child: Child,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
) -> StoppedReferenceBackend {
    child
        .wait()
        .expect("failed to collect reference-backend exit status");

    let stdout = fs::read_to_string(&stdout_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", stdout_path.display(), e));
    let stderr = fs::read_to_string(&stderr_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", stderr_path.display(), e));
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    let combined = format!("{stdout}{stderr}");

    StoppedReferenceBackend {
        stdout,
        stderr,
        combined,
    }
}

fn stop_reference_backend(spawned: SpawnedReferenceBackend) -> StoppedReferenceBackend {
    let SpawnedReferenceBackend {
        mut child,
        stdout_path,
        stderr_path,
    } = spawned;

    let _ = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status();
    std::thread::sleep(Duration::from_millis(250));
    if child
        .try_wait()
        .expect("failed to probe reference-backend exit status")
        .is_none()
    {
        let _ = child.kill();
    }

    collect_stopped_reference_backend(child, stdout_path, stderr_path)
}

fn kill_reference_backend(spawned: SpawnedReferenceBackend) -> StoppedReferenceBackend {
    let SpawnedReferenceBackend {
        mut child,
        stdout_path,
        stderr_path,
    } = spawned;

    let _ = child.kill();

    collect_stopped_reference_backend(child, stdout_path, stderr_path)
}

fn stop_reference_backend_pair(
    pair: SpawnedReferenceBackendPair,
) -> (StoppedReferenceBackend, StoppedReferenceBackend) {
    let SpawnedReferenceBackendPair {
        backend_a,
        backend_b,
        ..
    } = pair;
    (
        stop_reference_backend(backend_a),
        stop_reference_backend(backend_b),
    )
}

fn assert_startup_logs(combined: &str, config: &ReferenceBackendConfig, database_url: &str) {
    assert!(
        combined.contains(&format!(
            "[reference-backend] Config loaded port={} job_poll_ms={}",
            config.port, config.job_poll_ms
        )),
        "expected config-loaded log line, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[reference-backend] PostgreSQL pool ready"),
        "expected pool-ready log line, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[reference-backend] Runtime registry ready"),
        "expected registry-ready log line, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[reference-backend] Job worker ready"),
        "expected worker-ready log line, got:\n{}",
        combined
    );
    assert!(
        combined.contains(&format!(
            "[reference-backend] HTTP server starting on :{}",
            config.port
        )),
        "expected HTTP-bind log line, got:\n{}",
        combined
    );
    assert!(
        !combined.contains(database_url),
        "startup logs must not echo DATABASE_URL\nlogs:\n{}",
        combined
    );
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

fn assert_reference_backend_runtime_starts(database_url: &str) {
    assert_reference_backend_build_succeeds();

    let config = reference_backend_test_config(1000);
    let spawned = spawn_reference_backend(database_url, config);
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let health = wait_for_reference_backend(&config);
        assert_eq!(health["status"].as_str(), Some("ok"));
        assert_eq!(health["worker"]["poll_ms"].as_i64(), Some(1000));
    }));
    let logs = stop_reference_backend(spawned);

    match run_result {
        Ok(()) => assert_startup_logs(&logs.combined, &config, database_url),
        Err(payload) => panic!(
            "reference-backend runtime-starts assertions failed: {}\nstdout: {}\nstderr: {}",
            panic_payload_to_string(payload),
            logs.stdout,
            logs.stderr
        ),
    }
}

fn run_reference_backend_smoke_script(
    database_url: &str,
    config: &ReferenceBackendConfig,
) -> Output {
    let root = repo_root();
    Command::new("bash")
        .current_dir(&root)
        .arg("reference-backend/scripts/smoke.sh")
        .env("DATABASE_URL", database_url)
        .env("PORT", config.port.to_string())
        .env("JOB_POLL_MS", config.job_poll_ms.to_string())
        .output()
        .expect("failed to invoke reference-backend/scripts/smoke.sh")
}

fn run_reference_backend_stage_deploy_script(bundle_dir: &Path) -> Output {
    let root = repo_root();
    Command::new("bash")
        .current_dir(&root)
        .arg("reference-backend/scripts/stage-deploy.sh")
        .arg(bundle_dir)
        .output()
        .expect("failed to invoke reference-backend/scripts/stage-deploy.sh")
}

fn run_staged_apply_deploy_migrations_script(bundle_dir: &Path, database_url: &str) -> Output {
    let apply_script = bundle_dir.join("apply-deploy-migrations.sh");
    let sql_path = bundle_dir.join("reference-backend.up.sql");
    Command::new("bash")
        .current_dir(bundle_dir)
        .arg(&apply_script)
        .arg(&sql_path)
        .env("DATABASE_URL", database_url)
        .output()
        .expect("failed to invoke staged apply-deploy-migrations.sh")
}

fn run_staged_deploy_smoke_script(bundle_dir: &Path, config: &ReferenceBackendConfig) -> Output {
    let smoke_script = bundle_dir.join("deploy-smoke.sh");
    let base_url = format!("http://127.0.0.1:{}", config.port);
    Command::new("bash")
        .current_dir(bundle_dir)
        .arg(&smoke_script)
        .env("BASE_URL", &base_url)
        .env("PORT", config.port.to_string())
        .output()
        .expect("failed to invoke staged deploy-smoke.sh")
}

fn assert_staged_bundle_dir_outside_repo_root(bundle_dir: &Path) {
    let bundle_dir = fs::canonicalize(bundle_dir)
        .unwrap_or_else(|e| panic!("failed to canonicalize {}: {}", bundle_dir.display(), e));
    let repo_root = fs::canonicalize(repo_root()).expect("failed to canonicalize repo root");
    assert!(
        !bundle_dir.starts_with(&repo_root),
        "staged bundle dir must live outside the repo root; bundle_dir={} repo_root={}",
        bundle_dir.display(),
        repo_root.display()
    );
}

fn parse_last_json_line(output_text: &str, description: &str) -> Value {
    let json_line = output_text
        .lines()
        .rev()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                Some(trimmed)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("expected trailing JSON line in {description}, got:\n{output_text}"));
    serde_json::from_str(json_line).unwrap_or_else(|e| {
        panic!(
            "expected valid trailing JSON line in {description}, got parse error {e}: {json_line}"
        )
    })
}

fn wait_for_worker_processed_job(
    config: &ReferenceBackendConfig,
    min_processed_jobs: i64,
    max_attempts: usize,
) -> Option<(String, Value)> {
    for attempt in 0..max_attempts {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(100));
        }

        let health = match send_http_request(config, "GET", "/health", None) {
            Ok(response) if response.status_code == 200 => {
                assert_json_response(response, 200, "/health")
            }
            Ok(_) | Err(_) => continue,
        };

        let processed_jobs = health["worker"]["processed_jobs"].as_i64().unwrap_or(-1);
        let failed_jobs = health["worker"]["failed_jobs"].as_i64().unwrap_or(-1);
        let last_job_id = health["worker"]["last_job_id"].as_str().unwrap_or("");

        if processed_jobs >= min_processed_jobs && failed_jobs == 0 && !last_job_id.is_empty() {
            return Some((last_job_id.to_string(), health));
        }
    }

    None
}

fn assert_is_executable(path: &Path) {
    let metadata = fs::metadata(path)
        .unwrap_or_else(|e| panic!("failed to stat {}: {}", path.display(), e));
    assert!(metadata.is_file(), "expected file at {}", path.display());
    assert!(
        metadata.permissions().mode() & 0o111 != 0,
        "expected executable permissions at {}",
        path.display()
    );
}

fn assert_reference_backend_postgres_smoke(database_url: &str) {
    let config = reference_backend_test_config(500);
    assert_reference_backend_migration_succeeds(database_url, "status");
    assert_reference_backend_migration_succeeds(database_url, "up");

    let output = run_reference_backend_smoke_script(database_url, &config);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{stdout}{stderr}");

    assert!(
        output.status.success(),
        "reference-backend smoke script failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    assert!(
        combined.contains("[smoke] building reference-backend"),
        "expected smoke build step, got:\n{}",
        combined
    );
    assert!(
        combined.contains(&format!(
            "[smoke] starting reference-backend on :{}",
            config.port
        )),
        "expected smoke start step, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[smoke] probing running instance via deploy-smoke.sh"),
        "expected smoke handoff step, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[deploy-smoke] health ready body="),
        "expected deploy smoke health step, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[deploy-smoke] created job body="),
        "expected deploy smoke create step, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[deploy-smoke] processed job id="),
        "expected deploy smoke processed step, got:\n{}",
        combined
    );
    assert!(
        combined.contains("\"status\":\"processed\""),
        "expected processed job payload in smoke output, got:\n{}",
        combined
    );
    assert!(
        !combined.contains(database_url),
        "smoke output must not echo DATABASE_URL\nlogs:\n{}",
        combined
    );
}

fn query_database_rows(database_url: &str, sql: &str, params: &[&str]) -> Vec<DbRow> {
    let mut conn = native_pg_connect(database_url)
        .unwrap_or_else(|e| panic!("failed to connect to Postgres for query: {e}"));
    let result = native_pg_query(&mut conn, sql, params);
    native_pg_close(conn);
    let rows = result.unwrap_or_else(|e| panic!("query failed: {e}\nsql: {sql}"));
    rows.into_iter()
        .map(|row| row.into_iter().collect())
        .collect()
}

fn query_single_row(database_url: &str, sql: &str, params: &[&str]) -> DbRow {
    let rows = query_database_rows(database_url, sql, params);
    assert_eq!(rows.len(), 1, "expected exactly one row for SQL: {sql}");
    rows.into_iter().next().unwrap()
}

fn execute_database_sql(database_url: &str, sql: &str, params: &[&str]) -> i64 {
    let mut conn = native_pg_connect(database_url)
        .unwrap_or_else(|e| panic!("failed to connect to Postgres for execute: {e}"));
    let result = native_pg_execute(&mut conn, sql, params);
    native_pg_close(conn);
    result.unwrap_or_else(|e| panic!("execute failed: {e}\nsql: {sql}"))
}

fn reset_reference_backend_database(database_url: &str) {
    let _ = execute_database_sql(database_url, "DROP TABLE IF EXISTS jobs", &[]);
    let _ = execute_database_sql(database_url, "DROP TABLE IF EXISTS _mesh_migrations", &[]);
}

fn wait_for_processed_job_and_health(
    config: &ReferenceBackendConfig,
    job_id: &str,
) -> (Value, Value) {
    let mut last_job = Value::Null;
    let mut last_health = Value::Null;
    let mut last_job_issue = String::new();
    let mut last_health_issue = String::new();

    for attempt in 0..120 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(100));
        }

        let job = match send_http_request(config, "GET", &format!("/jobs/{job_id}"), None) {
            Ok(response) if response.status_code == 200 => {
                last_job_issue.clear();
                assert_json_response(response, 200, &format!("/jobs/{job_id}"))
            }
            Ok(response) => {
                last_job_issue = format!(
                    "unexpected HTTP {} for /jobs/{} on :{}",
                    response.status_code, job_id, config.port
                );
                continue;
            }
            Err(e) => {
                last_job_issue = format!("GET /jobs/{} failed on {}: {}", job_id, config.port, e);
                continue;
            }
        };

        let health = match send_http_request(config, "GET", "/health", None) {
            Ok(response) if response.status_code == 200 => {
                last_health_issue.clear();
                assert_json_response(response, 200, "/health")
            }
            Ok(response) => {
                last_health_issue =
                    format!("unexpected HTTP {} for /health on :{}", response.status_code, config.port);
                continue;
            }
            Err(e) => {
                last_health_issue = format!("GET /health failed on {}: {}", config.port, e);
                continue;
            }
        };

        let job_status = job["status"].as_str().unwrap_or("");
        let processed_at = job["processed_at"].as_str().unwrap_or("");
        let health_status = health["status"].as_str().unwrap_or("");
        let health_liveness = health["worker"]["liveness"].as_str().unwrap_or("");
        let health_recovery_active = health["worker"]["recovery_active"].as_bool().unwrap_or(true);
        let health_last_job_id = health["worker"]["last_job_id"].as_str().unwrap_or("");
        let health_processed_jobs = health["worker"]["processed_jobs"].as_i64().unwrap_or(-1);
        let health_failed_jobs = health["worker"]["failed_jobs"].as_i64().unwrap_or(-1);

        if job_status == "processed"
            && !processed_at.is_empty()
            && health_status == "ok"
            && health_liveness == "healthy"
            && !health_recovery_active
            && health_last_job_id == job_id
            && health_processed_jobs >= 1
            && health_failed_jobs == 0
        {
            return (job, health);
        }

        last_job = job;
        last_health = health;
    }

    panic!(
        "job {job_id} never reached processed health-aligned state; last_job={last_job}; last_health={last_health}; last_job_issue={last_job_issue}; last_health_issue={last_health_issue}"
    );
}

fn wait_for_inflight_processing_window(
    config: &ReferenceBackendConfig,
    database_url: &str,
    job_id: &str,
) -> (Value, Value, DbRow) {
    let mut last_health = Value::Null;
    let mut last_job = Value::Null;
    let mut last_db_row: Option<DbRow> = None;
    let mut last_issue = String::new();

    for attempt in 0..120 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }

        let health = match send_http_request(config, "GET", "/health", None) {
            Ok(response) if response.status_code == 200 => {
                last_issue.clear();
                assert_json_response(response, 200, "/health")
            }
            Ok(response) => {
                last_issue =
                    format!("unexpected HTTP {} for /health on :{}", response.status_code, config.port);
                continue;
            }
            Err(e) => {
                last_issue = format!("GET /health failed on {}: {}", config.port, e);
                continue;
            }
        };

        let job = match send_http_request(config, "GET", &format!("/jobs/{job_id}"), None) {
            Ok(response) if response.status_code == 200 => {
                last_issue.clear();
                assert_json_response(response, 200, &format!("/jobs/{job_id}"))
            }
            Ok(response) => {
                last_issue = format!(
                    "unexpected HTTP {} for /jobs/{} on :{}",
                    response.status_code, job_id, config.port
                );
                continue;
            }
            Err(e) => {
                last_issue = format!("GET /jobs/{} failed on {}: {}", job_id, config.port, e);
                continue;
            }
        };

        let db_row = wait_for_job_row(database_url, job_id, "processing", "1");

        let overall_status = health["status"].as_str().unwrap_or("");
        let worker_status = health["worker"]["status"].as_str().unwrap_or("");
        let liveness = health["worker"]["liveness"].as_str().unwrap_or("");
        let recovery_active = health["worker"]["recovery_active"].as_bool().unwrap_or(true);
        let restart_count = health["worker"]["restart_count"].as_i64().unwrap_or(-1);
        let recovered_jobs = health["worker"]["recovered_jobs"].as_i64().unwrap_or(-1);
        let last_job_id = health["worker"]["last_job_id"].as_str().unwrap_or("");
        let job_status = job["status"].as_str().unwrap_or("");
        let job_attempts = job["attempts"].as_i64().unwrap_or(-1);
        let job_last_error = &job["last_error"];

        if overall_status == "ok"
            && worker_status == "processing"
            && liveness == "healthy"
            && !recovery_active
            && restart_count == 0
            && recovered_jobs == 0
            && last_job_id == job_id
            && job_status == "processing"
            && job_attempts == 1
            && job_last_error.is_null()
            && db_row.get("last_error").map(String::as_str) == Some("")
        {
            return (health, job, db_row);
        }

        last_health = health;
        last_job = job;
        last_db_row = Some(db_row);
    }

    panic!(
        "job never exposed deterministic in-flight processing window; last_health={last_health}; last_job={last_job}; last_db_row={last_db_row:?}; last_issue={last_issue}"
    );
}

fn wait_for_worker_recovery_window(
    config: &ReferenceBackendConfig,
    database_url: &str,
    job_id: &str,
    expected_exit_reason: &str,
    min_restart_count: i64,
    min_recovered_jobs: i64,
) -> (Value, Value, DbRow) {
    let mut last_health = Value::Null;
    let mut last_job = Value::Null;
    let mut last_db_row: Option<DbRow> = None;
    let mut last_issue = String::new();

    for attempt in 0..120 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }

        let health = match send_http_request(config, "GET", "/health", None) {
            Ok(response) if response.status_code == 200 => {
                last_issue.clear();
                assert_json_response(response, 200, "/health")
            }
            Ok(response) => {
                last_issue =
                    format!("unexpected HTTP {} for /health on :{}", response.status_code, config.port);
                continue;
            }
            Err(e) => {
                last_issue = format!("GET /health failed on {}: {}", config.port, e);
                continue;
            }
        };

        let job = match send_http_request(config, "GET", &format!("/jobs/{job_id}"), None) {
            Ok(response) if response.status_code == 200 => {
                last_issue.clear();
                assert_json_response(response, 200, &format!("/jobs/{job_id}"))
            }
            Ok(response) => {
                last_issue = format!(
                    "unexpected HTTP {} for /jobs/{} on :{}",
                    response.status_code, job_id, config.port
                );
                continue;
            }
            Err(e) => {
                last_issue = format!("GET /jobs/{} failed on {}: {}", job_id, config.port, e);
                continue;
            }
        };

        let db_row = query_single_row(
            database_url,
            "SELECT id::text, status, attempts::text, COALESCE(last_error, '') AS last_error FROM jobs WHERE id = $1::uuid",
            &[job_id],
        );

        let overall_status = health["status"].as_str().unwrap_or("");
        let liveness = health["worker"]["liveness"].as_str().unwrap_or("");
        let restart_count = health["worker"]["restart_count"].as_i64().unwrap_or(-1);
        let recovered_jobs = health["worker"]["recovered_jobs"].as_i64().unwrap_or(-1);
        let recovery_active = health["worker"]["recovery_active"].as_bool().unwrap_or(false);
        let last_exit_reason = health["worker"]["last_exit_reason"].as_str().unwrap_or("");
        let job_status = job["status"].as_str().unwrap_or("");
        let job_attempts = job["attempts"].as_i64().unwrap_or(-1);
        let job_last_error = job["last_error"].as_str().unwrap_or("");
        let db_status = db_row.get("status").map(String::as_str).unwrap_or("");
        let db_attempts = db_row.get("attempts").map(String::as_str).unwrap_or("");
        let db_last_error = db_row.get("last_error").map(String::as_str).unwrap_or("");

        if overall_status == "degraded"
            && liveness == "recovering"
            && recovery_active
            && restart_count >= min_restart_count
            && recovered_jobs >= min_recovered_jobs
            && last_exit_reason == expected_exit_reason
            && job_status == "pending"
            && job_attempts == 1
            && job_last_error == "requeued after worker restart"
            && db_status == "pending"
            && db_attempts == "1"
            && db_last_error == "requeued after worker restart"
        {
            return (health, job, db_row);
        }

        last_health = health;
        last_job = job;
        last_db_row = Some(db_row);
    }

    panic!(
        "worker never exposed degraded recovery window with pending job visibility; last_health={last_health}; last_job={last_job}; last_db_row={last_db_row:?}; last_issue={last_issue}"
    );
}

fn wait_for_boot_recovery_window(
    config: &ReferenceBackendConfig,
    database_url: &str,
    job_id: &str,
) -> (Value, Value, DbRow) {
    let mut last_health = Value::Null;
    let mut last_job = Value::Null;
    let mut last_db_row: Option<DbRow> = None;
    let mut last_issue = String::new();

    for attempt in 0..120 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }

        let health = match send_http_request(config, "GET", "/health", None) {
            Ok(response) if response.status_code == 200 => {
                last_issue.clear();
                assert_json_response(response, 200, "/health")
            }
            Ok(response) => {
                last_issue =
                    format!("unexpected HTTP {} for /health on :{}", response.status_code, config.port);
                continue;
            }
            Err(e) => {
                last_issue = format!("GET /health failed on {}: {}", config.port, e);
                continue;
            }
        };

        let job = match send_http_request(config, "GET", &format!("/jobs/{job_id}"), None) {
            Ok(response) if response.status_code == 200 => {
                last_issue.clear();
                assert_json_response(response, 200, &format!("/jobs/{job_id}"))
            }
            Ok(response) => {
                last_issue = format!(
                    "unexpected HTTP {} for /jobs/{} on :{}",
                    response.status_code, job_id, config.port
                );
                continue;
            }
            Err(e) => {
                last_issue = format!("GET /jobs/{} failed on {}: {}", job_id, config.port, e);
                continue;
            }
        };

        let db_row = query_single_row(
            database_url,
            "SELECT id::text, status, attempts::text, COALESCE(last_error, '') AS last_error FROM jobs WHERE id = $1::uuid",
            &[job_id],
        );

        let overall_status = health["status"].as_str().unwrap_or("");
        let liveness = health["worker"]["liveness"].as_str().unwrap_or("");
        let restart_count = health["worker"]["restart_count"].as_i64().unwrap_or(-1);
        let recovered_jobs = health["worker"]["recovered_jobs"].as_i64().unwrap_or(-1);
        let recovery_active = health["worker"]["recovery_active"].as_bool().unwrap_or(false);
        let last_exit_reason = &health["worker"]["last_exit_reason"];
        let job_status = job["status"].as_str().unwrap_or("");
        let job_attempts = job["attempts"].as_i64().unwrap_or(-1);
        let job_last_error = job["last_error"].as_str().unwrap_or("");
        let db_status = db_row.get("status").map(String::as_str).unwrap_or("");
        let db_attempts = db_row.get("attempts").map(String::as_str).unwrap_or("");
        let db_last_error = db_row.get("last_error").map(String::as_str).unwrap_or("");

        if overall_status == "degraded"
            && liveness == "recovering"
            && recovery_active
            && restart_count == 0
            && recovered_jobs >= 1
            && last_exit_reason.is_null()
            && job_status == "pending"
            && job_attempts == 1
            && job_last_error == "requeued abandoned processing job during boot recovery"
            && db_status == "pending"
            && db_attempts == "1"
            && db_last_error == "requeued abandoned processing job during boot recovery"
        {
            return (health, job, db_row);
        }

        last_health = health;
        last_job = job;
        last_db_row = Some(db_row);
    }

    panic!(
        "backend never exposed boot-recovery window with pending job visibility; last_health={last_health}; last_job={last_job}; last_db_row={last_db_row:?}; last_issue={last_issue}"
    );
}

fn wait_for_job_row(
    database_url: &str,
    job_id: &str,
    expected_status: &str,
    expected_attempts: &str,
) -> DbRow {
    let mut last_row: Option<DbRow> = None;

    for attempt in 0..120 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }

        let row = query_single_row(
            database_url,
            "SELECT id::text, status, attempts::text, COALESCE(last_error, '') AS last_error, payload::text AS payload, COALESCE(processed_at::text, '') AS processed_at FROM jobs WHERE id = $1::uuid",
            &[job_id],
        );

        if row.get("status").map(String::as_str) == Some(expected_status)
            && row.get("attempts").map(String::as_str) == Some(expected_attempts)
        {
            return row;
        }

        last_row = Some(row);
    }

    panic!(
        "job row never reached expected state status={expected_status} attempts={expected_attempts}; last_row={last_row:?}"
    );
}

fn wait_for_jobs_processed_in_database(database_url: &str, expected_jobs: usize) -> Vec<DbRow> {
    let mut last_rows = Vec::new();

    for attempt in 0..200 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }

        let rows = query_database_rows(
            database_url,
            "SELECT id::text, status, attempts::text, COALESCE(last_error, '') AS last_error, payload::text AS payload, COALESCE(processed_at::text, '') AS processed_at FROM jobs ORDER BY created_at ASC, id ASC",
            &[],
        );

        let all_processed = rows.len() == expected_jobs
            && rows.iter().all(|row| {
                row.get("status").map(String::as_str) == Some("processed")
                    && row.get("attempts").map(String::as_str) == Some("1")
                    && row.get("last_error").map(String::as_str) == Some("")
                    && row
                        .get("processed_at")
                        .map(|value| !value.is_empty())
                        .unwrap_or(false)
            });

        if all_processed {
            return rows;
        }

        last_rows = rows;
    }

    panic!(
        "jobs never reached all-processed database state; expected_jobs={expected_jobs}; last_rows={last_rows:?}"
    );
}

fn wait_for_multi_instance_health(
    config_a: &ReferenceBackendConfig,
    config_b: &ReferenceBackendConfig,
) -> (Value, Value) {
    let mut last_health_a = Value::Null;
    let mut last_health_b = Value::Null;
    let mut last_issue_a = String::new();
    let mut last_issue_b = String::new();

    for attempt in 0..200 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }

        let health_a = match send_http_request(config_a, "GET", "/health", None) {
            Ok(response) if response.status_code == 200 => {
                last_issue_a.clear();
                assert_json_response(response, 200, "/health")
            }
            Ok(response) => {
                last_issue_a = format!("unexpected HTTP {} on :{}", response.status_code, config_a.port);
                continue;
            }
            Err(e) => {
                last_issue_a = format!("GET /health failed on {}: {}", config_a.port, e);
                continue;
            }
        };
        let health_b = match send_http_request(config_b, "GET", "/health", None) {
            Ok(response) if response.status_code == 200 => {
                last_issue_b.clear();
                assert_json_response(response, 200, "/health")
            }
            Ok(response) => {
                last_issue_b = format!("unexpected HTTP {} on :{}", response.status_code, config_b.port);
                continue;
            }
            Err(e) => {
                last_issue_b = format!("GET /health failed on {}: {}", config_b.port, e);
                continue;
            }
        };
        let failed_a = health_a["worker"]["failed_jobs"].as_i64().unwrap_or(-1);
        let failed_b = health_b["worker"]["failed_jobs"].as_i64().unwrap_or(-1);
        let status_a = health_a["worker"]["status"].as_str().unwrap_or("");
        let status_b = health_b["worker"]["status"].as_str().unwrap_or("");

        if failed_a == 0
            && failed_b == 0
            && health_a["worker"]["last_error"].is_null()
            && health_b["worker"]["last_error"].is_null()
            && matches!(status_a, "processed" | "idle")
            && matches!(status_b, "processed" | "idle")
        {
            return (health_a, health_b);
        }

        last_health_a = health_a;
        last_health_b = health_b;
    }

    panic!(
        "two-instance health never settled cleanly; last_health_a={last_health_a}; last_health_b={last_health_b}; last_issue_a={last_issue_a}; last_issue_b={last_issue_b}"
    );
}

fn assert_reference_backend_multi_instance_exact_once(database_url: &str) {
    reset_reference_backend_database(database_url);
    assert_reference_backend_build_succeeds();
    assert_reference_backend_migration_succeeds(database_url, "up");

    let pair = spawn_reference_backend_pair(database_url, 100);
    let config_a = pair.config_a;
    let config_b = pair.config_b;
    let test_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let startup_health_a = wait_for_reference_backend(&config_a);
        let startup_health_b = wait_for_reference_backend(&config_b);
        assert_eq!(startup_health_a["status"].as_str(), Some("ok"));
        assert_eq!(startup_health_b["status"].as_str(), Some("ok"));
        assert_eq!(startup_health_a["worker"]["failed_jobs"].as_i64(), Some(0));
        assert_eq!(startup_health_b["worker"]["failed_jobs"].as_i64(), Some(0));

        let job_count = 20usize;
        let mut job_ids = Vec::with_capacity(job_count);
        let mut expected_payloads: HashMap<String, Value> = HashMap::with_capacity(job_count);
        for seq in 0..job_count {
            let (creator_label, creator_config) = if seq % 2 == 0 {
                ("a", &config_a)
            } else {
                ("b", &config_b)
            };
            let payload_body = format!(
                r#"{{"kind":"exact-once-probe","seq":{},"source":"rust-harness","created_via":"{}"}}"#,
                seq, creator_label
            );
            let payload_json: Value =
                serde_json::from_str(&payload_body).expect("payload must be valid JSON");
            let created_job = post_json(creator_config, "/jobs", &payload_body, 201);
            let created_status = created_job["status"]
                .as_str()
                .expect("created job status must be a string");
            assert!(
                matches!(created_status, "pending" | "processing" | "processed"),
                "unexpected created job status: {created_status}; body={created_job}"
            );
            assert_eq!(created_job["last_error"], Value::Null);
            assert_eq!(created_job["payload"], payload_json);

            let job_id = created_job["id"]
                .as_str()
                .expect("created job id must be a string")
                .to_string();
            assert!(
                expected_payloads
                    .insert(job_id.clone(), payload_json.clone())
                    .is_none(),
                "duplicate created job id returned from POST /jobs: {job_id}"
            );
            job_ids.push(job_id);
        }

        let db_rows = wait_for_jobs_processed_in_database(database_url, job_count);
        let mut seen_db_ids = HashSet::with_capacity(job_count);
        let mut rows_by_id: HashMap<String, DbRow> = HashMap::with_capacity(job_count);
        for row in db_rows {
            let job_id = row
                .get("id")
                .expect("jobs row must expose id")
                .to_string();
            assert!(
                seen_db_ids.insert(job_id.clone()),
                "duplicate jobs row observed for id={job_id}"
            );
            assert_eq!(row.get("status").map(String::as_str), Some("processed"));
            assert_eq!(row.get("attempts").map(String::as_str), Some("1"));
            assert_eq!(row.get("last_error").map(String::as_str), Some(""));
            assert!(
                row.get("processed_at")
                    .map(|value| !value.is_empty())
                    .unwrap_or(false),
                "jobs row must expose processed_at for id={job_id}: {row:?}"
            );
            let db_payload = serde_json::from_str::<Value>(
                row.get("payload").expect("jobs row must expose payload"),
            )
            .expect("DB payload must be valid JSON");
            assert_eq!(
                expected_payloads.get(&job_id),
                Some(&db_payload),
                "DB payload drifted for id={job_id}"
            );
            rows_by_id.insert(job_id, row);
        }
        assert_eq!(rows_by_id.len(), job_count, "expected one DB row per job id");

        for (seq, job_id) in job_ids.iter().enumerate() {
            let db_row = rows_by_id
                .get(job_id)
                .unwrap_or_else(|| panic!("missing DB row for job id={job_id}"));
            let expected_payload = expected_payloads
                .get(job_id)
                .unwrap_or_else(|| panic!("missing expected payload for job id={job_id}"));
            let db_processed_at = db_row
                .get("processed_at")
                .expect("jobs row must expose processed_at");
            let (instance_label, config) = if seq % 2 == 0 {
                ("b", &config_b)
            } else {
                ("a", &config_a)
            };

            let job_json = get_json(config, &format!("/jobs/{job_id}"), 200);
            assert_eq!(
                job_json["id"].as_str(),
                Some(job_id.as_str()),
                "instance {instance_label} returned wrong job id"
            );
            assert_eq!(
                job_json["status"].as_str(),
                Some("processed"),
                "instance {instance_label} returned non-processed job state for id={job_id}"
            );
            assert_eq!(
                job_json["attempts"].as_i64(),
                Some(1),
                "instance {instance_label} reported wrong attempts for id={job_id}"
            );
            assert_eq!(
                job_json["last_error"],
                Value::Null,
                "instance {instance_label} reported last_error for id={job_id}"
            );
            assert_eq!(
                job_json["payload"],
                expected_payload.clone(),
                "instance {instance_label} returned wrong payload for id={job_id}"
            );
            assert_eq!(
                job_json["processed_at"].as_str(),
                Some(db_processed_at.as_str()),
                "instance {instance_label} disagreed with DB processed_at for id={job_id}"
            );
        }

        let (health_a, health_b) = wait_for_multi_instance_health(&config_a, &config_b);
        let failed_total = health_a["worker"]["failed_jobs"].as_i64().unwrap_or(-1)
            + health_b["worker"]["failed_jobs"].as_i64().unwrap_or(-1);

        assert_eq!(
            failed_total, 0,
            "claim contention must not inflate failed_jobs; health_a={health_a}; health_b={health_b}"
        );
        assert_eq!(health_a["worker"]["last_error"], Value::Null);
        assert_eq!(health_b["worker"]["last_error"], Value::Null);
    }));
    let (logs_a, logs_b) = stop_reference_backend_pair(pair);

    match test_result {
        Ok(()) => {
            assert_startup_logs(&logs_a.combined, &config_a, database_url);
            assert_startup_logs(&logs_b.combined, &config_b, database_url);
            assert!(
                logs_a.combined.contains("Job worker processed id="),
                "instance A never logged a processed job:\n{}",
                logs_a.combined
            );
            assert!(
                logs_b.combined.contains("Job worker processed id="),
                "instance B never logged a processed job:\n{}",
                logs_b.combined
            );
            assert!(
                !logs_a.combined.contains("update_where: no rows matched"),
                "instance A still logged claim-race failures:\n{}",
                logs_a.combined
            );
            assert!(
                !logs_b.combined.contains("update_where: no rows matched"),
                "instance B still logged claim-race failures:\n{}",
                logs_b.combined
            );
        }
        Err(payload) => panic!(
            "reference-backend multi-instance exact-once assertions failed: {}\nstdout_a: {}\nstderr_a: {}\nstdout_b: {}\nstderr_b: {}",
            panic_payload_to_string(payload),
            logs_a.stdout,
            logs_a.stderr,
            logs_b.stdout,
            logs_b.stderr
        ),
    }
}

#[test]
fn e2e_reference_backend_builds() {
    assert_reference_backend_build_succeeds();

    let binary = reference_backend_binary();
    assert!(
        binary.exists(),
        "compiled reference-backend binary not found at {}",
        binary.display()
    );
}

#[test]
fn e2e_reference_backend_stage_deploy_bundle() {
    let bundle = tempfile::tempdir().expect("failed to create temp bundle dir");
    let output = run_reference_backend_stage_deploy_script(bundle.path());
    let combined = command_output_text(&output);

    assert_command_success(&output, "reference-backend/scripts/stage-deploy.sh");
    assert!(
        combined.contains("[stage-deploy] staged layout"),
        "expected staged layout output, got:\n{}",
        combined
    );
    assert!(
        combined.contains("[stage-deploy] bundle ready dir="),
        "expected bundle ready output, got:\n{}",
        combined
    );

    assert_is_executable(&bundle.path().join("reference-backend"));
    assert!(
        bundle.path().join("reference-backend.up.sql").is_file(),
        "expected staged deploy SQL artifact at {}",
        bundle.path().join("reference-backend.up.sql").display()
    );
    assert_is_executable(&bundle.path().join("apply-deploy-migrations.sh"));
    assert_is_executable(&bundle.path().join("deploy-smoke.sh"));
}

#[test]
#[ignore]
fn e2e_reference_backend_deploy_artifact_smoke() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_deploy_artifact_smoke");

    reset_reference_backend_database(&database_url);

    let bundle = tempfile::tempdir().expect("failed to create deploy bundle dir");
    assert_staged_bundle_dir_outside_repo_root(bundle.path());

    let stage_output = run_reference_backend_stage_deploy_script(bundle.path());
    let stage_text = command_output_text(&stage_output);
    assert_command_success(&stage_output, "reference-backend/scripts/stage-deploy.sh");
    assert!(
        stage_text.contains("[stage-deploy] bundle ready dir="),
        "expected staged bundle ready output, got:\n{}",
        stage_text
    );

    let staged_binary = bundle.path().join("reference-backend");
    let staged_sql = bundle.path().join("reference-backend.up.sql");
    assert_is_executable(&staged_binary);
    assert_is_executable(&bundle.path().join("apply-deploy-migrations.sh"));
    assert_is_executable(&bundle.path().join("deploy-smoke.sh"));
    assert!(staged_sql.is_file(), "expected staged deploy SQL artifact");
    assert!(
        !bundle.path().join("meshc").exists(),
        "staged deploy bundle should not include meshc"
    );
    assert!(
        !bundle.path().join("main.mpl").exists(),
        "staged deploy bundle should not require repo source files"
    );

    let apply_output = run_staged_apply_deploy_migrations_script(bundle.path(), &database_url);
    let apply_text = command_output_text(&apply_output);
    assert_command_success(&apply_output, "staged apply-deploy-migrations.sh");
    assert!(
        apply_text.contains("[deploy-apply] sql artifact="),
        "expected deploy-apply artifact output, got:\n{}",
        apply_text
    );
    assert!(
        apply_text.contains(&format!(
            "[deploy-apply] migration recorded version={}",
            REFERENCE_BACKEND_MIGRATION_VERSION
        )),
        "expected recorded migration output, got:\n{}",
        apply_text
    );
    assert!(
        !apply_text.contains(&database_url),
        "staged apply output must not echo DATABASE_URL\n{}",
        apply_text
    );

    let migration_rows = query_database_rows(
        &database_url,
        "SELECT version::text AS version, name, applied_at::text AS applied_at FROM _mesh_migrations ORDER BY version",
        &[],
    );
    assert_eq!(
        migration_rows.len(),
        1,
        "expected exactly one applied migration row after staged deploy apply"
    );
    let migration_row = &migration_rows[0];
    assert_eq!(
        migration_row.get("version").map(String::as_str),
        Some("20260323010000")
    );
    assert_eq!(
        migration_row.get("name").map(String::as_str),
        Some(REFERENCE_BACKEND_MIGRATION_NAME)
    );
    assert!(
        migration_row
            .get("applied_at")
            .map(|value| !value.is_empty())
            .unwrap_or(false),
        "expected applied_at to be recorded for staged deploy migration: {:?}",
        migration_row
    );

    let config = reference_backend_test_config(100);
    let spawned = spawn_staged_reference_backend(bundle.path(), &database_url, config);
    let mut smoke_text = String::new();
    let mut smoke_job_id = String::new();
    let test_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let startup_health = wait_for_reference_backend(&config);
        assert_eq!(startup_health["status"].as_str(), Some("ok"));
        assert_eq!(startup_health["worker"]["poll_ms"].as_i64(), Some(100));
        assert_eq!(startup_health["worker"]["processed_jobs"].as_i64(), Some(0));
        assert_eq!(startup_health["worker"]["failed_jobs"].as_i64(), Some(0));

        let smoke_output = run_staged_deploy_smoke_script(bundle.path(), &config);
        smoke_text = command_output_text(&smoke_output);
        assert_command_success(&smoke_output, "staged deploy-smoke.sh");
        assert!(
            smoke_text.contains("[deploy-smoke] health ready body="),
            "expected staged deploy smoke health output, got:\n{}",
            smoke_text
        );
        assert!(
            smoke_text.contains("[deploy-smoke] created job body="),
            "expected staged deploy smoke create output, got:\n{}",
            smoke_text
        );
        assert!(
            smoke_text.contains("[deploy-smoke] polling job id="),
            "expected staged deploy smoke polling output, got:\n{}",
            smoke_text
        );
        assert!(
            smoke_text.contains("[deploy-smoke] processed job id="),
            "expected staged deploy smoke processed output, got:\n{}",
            smoke_text
        );
        assert!(
            !smoke_text.contains(&database_url),
            "staged deploy smoke output must not echo DATABASE_URL\n{}",
            smoke_text
        );

        let smoke_job = parse_last_json_line(&smoke_text, "staged deploy-smoke.sh");
        smoke_job_id = smoke_job["id"]
            .as_str()
            .expect("staged deploy smoke payload must include job id")
            .to_string();
        assert_eq!(smoke_job["status"].as_str(), Some("processed"));
        assert_eq!(smoke_job["attempts"].as_i64(), Some(1));
        assert_eq!(smoke_job["last_error"], Value::Null);
        assert!(
            smoke_job["processed_at"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "staged deploy smoke JSON must include processed_at: {}",
            smoke_job
        );

        let mut worker_job_id = None;
        for seq in 0..6 {
            let payload_body = format!(
                r#"{{"kind":"deploy-artifact-owned","seq":{},"source":"rust-harness"}}"#,
                seq
            );
            let created_job = post_json(&config, "/jobs", &payload_body, 201);
            assert!(
                matches!(created_job["status"].as_str(), Some("pending" | "processing" | "processed")),
                "unexpected created job status while waiting for staged worker ownership: {}",
                created_job
            );

            if let Some((job_id, _)) = wait_for_worker_processed_job(&config, 1, 20) {
                worker_job_id = Some(job_id);
                break;
            }
        }

        let worker_job_id = worker_job_id.expect(
            "staged worker never recorded a processed job in /health; another shared-DB worker may still be active",
        );
        let (job_json, health_json) = wait_for_processed_job_and_health(&config, &worker_job_id);
        let db_row = query_single_row(
            &database_url,
            "SELECT id::text, status, attempts::text, COALESCE(last_error, '') AS last_error, payload::text, COALESCE(processed_at::text, '') AS processed_at FROM jobs WHERE id = $1::uuid",
            &[&worker_job_id],
        );

        assert_eq!(job_json["id"].as_str(), Some(worker_job_id.as_str()));
        assert_eq!(job_json["status"].as_str(), Some("processed"));
        assert_eq!(job_json["attempts"].as_i64().unwrap_or(0), 1);
        assert_eq!(job_json["last_error"], Value::Null);
        let processed_at = job_json["processed_at"]
            .as_str()
            .expect("processed staged job must expose processed_at");
        assert!(!processed_at.is_empty(), "processed_at must be non-empty");

        assert_eq!(db_row.get("id").map(String::as_str), Some(worker_job_id.as_str()));
        assert_eq!(db_row.get("status").map(String::as_str), Some("processed"));
        assert_eq!(db_row.get("attempts").map(String::as_str), Some("1"));
        assert_eq!(db_row.get("last_error").map(String::as_str), Some(""));
        assert_eq!(
            db_row.get("processed_at").map(String::as_str),
            Some(processed_at)
        );
        let db_payload = serde_json::from_str::<Value>(
            db_row.get("payload").expect("payload column must exist"),
        )
        .expect("DB payload must be valid JSON");
        assert_eq!(db_payload, job_json["payload"]);

        assert_eq!(health_json["status"].as_str(), Some("ok"));
        assert_eq!(
            health_json["worker"]["last_job_id"].as_str(),
            Some(worker_job_id.as_str())
        );
        assert!(
            health_json["worker"]["processed_jobs"].as_i64().unwrap_or(0) >= 1,
            "expected staged worker to record at least one processed job: {}",
            health_json
        );
        assert_eq!(health_json["worker"]["failed_jobs"].as_i64(), Some(0));
        assert_eq!(health_json["worker"]["last_error"], Value::Null);
        let worker_status = health_json["worker"]["status"]
            .as_str()
            .expect("worker status must be a string");
        assert!(
            matches!(worker_status, "processed" | "idle"),
            "expected staged worker status to show a healthy post-processing state, got {worker_status}"
        );
    }));
    let logs = stop_reference_backend(spawned);

    match test_result {
        Ok(()) => {
            assert_startup_logs(&logs.combined, &config, &database_url);
            assert!(
                logs.combined.contains("[reference-backend] Job created id="),
                "staged runtime never logged job creation:\n{}",
                logs.combined
            );
            assert!(
                logs.combined.contains("[reference-backend] Job worker processed id="),
                "staged runtime never logged job processing:\n{}",
                logs.combined
            );
            assert!(
                logs.combined.contains("[reference-backend] Job fetched id="),
                "staged runtime never logged /jobs fetches:\n{}",
                logs.combined
            );
        }
        Err(payload) => panic!(
            "reference-backend deploy-artifact smoke assertions failed: {}\nbundle_dir: {}\nstaged_binary: {}\nstage_output: {}\napply_output: {}\nsmoke_output: {}\nsmoke_job_id: {}\nstdout: {}\nstderr: {}",
            panic_payload_to_string(payload),
            bundle.path().display(),
            staged_binary.display(),
            stage_text,
            apply_text,
            smoke_text,
            smoke_job_id,
            logs.stdout,
            logs.stderr
        ),
    }
}

#[test]
#[ignore]
fn e2e_reference_backend_runtime_starts() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_runtime_starts");
    assert_reference_backend_runtime_starts(&database_url);
}

#[test]
#[ignore]
fn e2e_reference_backend_migration_status_and_apply() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_migration_status_and_apply");

    reset_reference_backend_database(&database_url);
    assert_reference_backend_build_succeeds();

    let status_before = run_reference_backend_migration(&database_url, "status");
    assert_command_success(
        &status_before,
        "meshc migrate reference-backend status (before up)",
    );
    let status_before_text = command_output_text(&status_before);
    assert!(
        status_before_text.contains("Migration Status:"),
        "expected status banner, got:\n{}",
        status_before_text
    );
    assert!(
        status_before_text.contains(&format!(
            "[ ] {}_{}",
            REFERENCE_BACKEND_MIGRATION_VERSION, REFERENCE_BACKEND_MIGRATION_NAME
        )),
        "expected pending migration before apply, got:\n{}",
        status_before_text
    );
    assert!(
        status_before_text.contains("0 applied, 1 pending"),
        "expected 0 applied, 1 pending before apply, got:\n{}",
        status_before_text
    );

    let rows_before = query_database_rows(
        &database_url,
        "SELECT version::text AS version, name FROM _mesh_migrations ORDER BY version",
        &[],
    );
    assert!(
        rows_before.is_empty(),
        "expected no applied migrations before up, got: {:?}",
        rows_before
    );

    let up_output = run_reference_backend_migration(&database_url, "up");
    assert_command_success(&up_output, "meshc migrate reference-backend up");
    let up_text = command_output_text(&up_output);
    assert!(
        up_text.contains(&format!(
            "Applying: {}_{}",
            REFERENCE_BACKEND_MIGRATION_VERSION, REFERENCE_BACKEND_MIGRATION_NAME
        )),
        "expected migration apply log, got:\n{}",
        up_text
    );
    assert!(
        up_text.contains("Applied 1 migration(s)"),
        "expected applied count after up, got:\n{}",
        up_text
    );

    let status_after = run_reference_backend_migration(&database_url, "status");
    assert_command_success(
        &status_after,
        "meshc migrate reference-backend status (after up)",
    );
    let status_after_text = command_output_text(&status_after);
    assert!(
        status_after_text.contains(&format!(
            "[x] {}_{}",
            REFERENCE_BACKEND_MIGRATION_VERSION, REFERENCE_BACKEND_MIGRATION_NAME
        )),
        "expected applied migration after up, got:\n{}",
        status_after_text
    );
    assert!(
        status_after_text.contains("1 applied, 0 pending"),
        "expected 1 applied, 0 pending after apply, got:\n{}",
        status_after_text
    );

    let rows_after = query_database_rows(
        &database_url,
        "SELECT version::text AS version, name FROM _mesh_migrations ORDER BY version",
        &[],
    );
    assert_eq!(rows_after.len(), 1, "expected one applied migration row");
    let applied = &rows_after[0];
    assert_eq!(
        applied.get("version").map(String::as_str),
        Some("20260323010000")
    );
    assert_eq!(
        applied.get("name").map(String::as_str),
        Some(REFERENCE_BACKEND_MIGRATION_NAME)
    );
}

#[test]
#[ignore]
fn e2e_reference_backend_job_flow_updates_health_and_db() {
    let database_url = std::env::var("DATABASE_URL").expect(
        "DATABASE_URL must be set for e2e_reference_backend_job_flow_updates_health_and_db",
    );

    reset_reference_backend_database(&database_url);
    assert_reference_backend_build_succeeds();
    assert_reference_backend_migration_succeeds(&database_url, "up");

    let config = reference_backend_test_config(100);
    let spawned = spawn_reference_backend(&database_url, config);
    let test_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let startup_health = wait_for_reference_backend(&config);
        assert_eq!(startup_health["status"].as_str(), Some("ok"));
        assert_eq!(startup_health["worker"]["poll_ms"].as_i64(), Some(100));
        assert_eq!(startup_health["worker"]["processed_jobs"].as_i64(), Some(0));
        assert_eq!(startup_health["worker"]["failed_jobs"].as_i64(), Some(0));

        let payload_body = r#"{"kind":"demo","attempt":1,"source":"rust-harness"}"#;
        let payload_json: Value =
            serde_json::from_str(payload_body).expect("payload must be valid JSON");
        let created_job = post_json(&config, "/jobs", payload_body, 201);

        assert_eq!(created_job["status"].as_str(), Some("pending"));
        assert_eq!(created_job["attempts"].as_i64(), Some(0));
        assert!(created_job["processed_at"].is_null());
        assert!(created_job["last_error"].is_null());
        assert_eq!(created_job["payload"], payload_json);

        let job_id = created_job["id"]
            .as_str()
            .expect("created job id must be a string")
            .to_string();

        let (job_json, health_json) = wait_for_processed_job_and_health(&config, &job_id);
        let db_row = query_single_row(
            &database_url,
            "SELECT id::text, status, attempts::text, COALESCE(last_error, '') AS last_error, payload::text, COALESCE(processed_at::text, '') AS processed_at FROM jobs WHERE id = $1::uuid",
            &[&job_id],
        );

        assert_eq!(job_json["id"].as_str(), Some(job_id.as_str()));
        assert_eq!(job_json["status"].as_str(), Some("processed"));
        assert_eq!(job_json["attempts"].as_i64(), Some(1));
        assert_eq!(job_json["last_error"], Value::Null);
        assert_eq!(job_json["payload"], payload_json);
        let processed_at = job_json["processed_at"]
            .as_str()
            .expect("processed job must expose processed_at");
        assert!(!processed_at.is_empty(), "processed_at must be non-empty");

        assert_eq!(db_row.get("id").map(String::as_str), Some(job_id.as_str()));
        assert_eq!(db_row.get("status").map(String::as_str), Some("processed"));
        assert_eq!(db_row.get("attempts").map(String::as_str), Some("1"));
        assert_eq!(db_row.get("last_error").map(String::as_str), Some(""));
        assert_eq!(
            db_row.get("processed_at").map(String::as_str),
            Some(processed_at)
        );
        let db_payload = serde_json::from_str::<Value>(
            db_row.get("payload").expect("payload column must exist"),
        )
        .expect("DB payload must be valid JSON");
        assert_eq!(db_payload, job_json["payload"]);

        assert_eq!(health_json["status"].as_str(), Some("ok"));
        assert_eq!(
            health_json["worker"]["last_job_id"].as_str(),
            Some(job_id.as_str())
        );
        assert_eq!(health_json["worker"]["processed_jobs"].as_i64(), Some(1));
        assert_eq!(health_json["worker"]["failed_jobs"].as_i64(), Some(0));
        assert_eq!(health_json["worker"]["last_error"], Value::Null);
        let worker_status = health_json["worker"]["status"]
            .as_str()
            .expect("worker status must be a string");
        assert!(
            matches!(worker_status, "processed" | "idle"),
            "expected worker status to show a healthy post-processing state, got {worker_status}"
        );
    }));
    let logs = stop_reference_backend(spawned);

    match test_result {
        Ok(()) => assert_startup_logs(&logs.combined, &config, &database_url),
        Err(payload) => panic!(
            "reference-backend job-flow assertions failed: {}\nstdout: {}\nstderr: {}",
            panic_payload_to_string(payload),
            logs.stdout,
            logs.stderr
        ),
    }
}

#[test]
#[ignore]
fn e2e_reference_backend_claim_contention_is_not_failure() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_claim_contention_is_not_failure");
    assert_reference_backend_multi_instance_exact_once(&database_url);
}

#[test]
#[ignore]
fn e2e_reference_backend_multi_instance_claims_once() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_multi_instance_claims_once");
    assert_reference_backend_multi_instance_exact_once(&database_url);
}

#[test]
#[ignore]
fn e2e_reference_backend_worker_crash_recovers_job() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_worker_crash_recovers_job");

    reset_reference_backend_database(&database_url);
    assert_reference_backend_build_succeeds();
    assert_reference_backend_migration_succeeds(&database_url, "up");

    let config = reference_backend_test_config(100);
    let spawned = spawn_reference_backend(&database_url, config);
    let test_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let startup_health = wait_for_reference_backend(&config);
        assert_eq!(startup_health["status"].as_str(), Some("ok"));
        assert_eq!(startup_health["worker"]["restart_count"].as_i64(), Some(0));
        assert_eq!(startup_health["worker"]["recovered_jobs"].as_i64(), Some(0));

        let payload_body = r#"{"kind":"crash_after_claim_once","source":"rust-harness"}"#;
        let payload_json: Value =
            serde_json::from_str(payload_body).expect("payload must be valid JSON");
        let created_job = post_json(&config, "/jobs", payload_body, 201);
        let job_id = created_job["id"]
            .as_str()
            .expect("created job id must be a string")
            .to_string();

        let (degraded_health, degraded_job, degraded_db_row) = wait_for_worker_recovery_window(
            &config,
            &database_url,
            &job_id,
            "worker_crash_after_claim",
            1,
            1,
        );
        assert_eq!(degraded_health["status"].as_str(), Some("degraded"));
        assert_eq!(degraded_health["worker"]["liveness"].as_str(), Some("recovering"));
        assert!(
            degraded_health["worker"]["last_tick_at"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "recovery health should expose last_tick_at: {}",
            degraded_health
        );
        assert_eq!(degraded_job["id"].as_str(), Some(job_id.as_str()));
        assert_eq!(degraded_job["status"].as_str(), Some("pending"));
        assert_eq!(degraded_job["attempts"].as_i64(), Some(1));
        assert_eq!(
            degraded_job["last_error"].as_str(),
            Some("requeued after worker restart")
        );
        assert_eq!(degraded_db_row.get("status").map(String::as_str), Some("pending"));
        assert_eq!(degraded_db_row.get("attempts").map(String::as_str), Some("1"));
        assert_eq!(
            degraded_db_row.get("last_error").map(String::as_str),
            Some("requeued after worker restart")
        );

        let (job_json, health_json) = wait_for_processed_job_and_health(&config, &job_id);
        let db_row = query_single_row(
            &database_url,
            "SELECT id::text, status, attempts::text, COALESCE(last_error, '') AS last_error, payload::text, COALESCE(processed_at::text, '') AS processed_at FROM jobs WHERE id = $1::uuid",
            &[&job_id],
        );

        assert_eq!(job_json["id"].as_str(), Some(job_id.as_str()));
        assert_eq!(job_json["status"].as_str(), Some("processed"));
        assert_eq!(job_json["attempts"].as_i64(), Some(2));
        assert_eq!(job_json["last_error"], Value::Null);
        assert_eq!(job_json["payload"], payload_json);
        assert!(
            job_json["processed_at"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "processed job should expose processed_at: {}",
            job_json
        );

        assert_eq!(db_row.get("id").map(String::as_str), Some(job_id.as_str()));
        assert_eq!(db_row.get("status").map(String::as_str), Some("processed"));
        assert_eq!(db_row.get("attempts").map(String::as_str), Some("2"));
        assert_eq!(db_row.get("last_error").map(String::as_str), Some(""));
        let db_payload = serde_json::from_str::<Value>(
            db_row.get("payload").expect("payload column must exist"),
        )
        .expect("DB payload must be valid JSON");
        assert_eq!(db_payload, job_json["payload"]);

        assert_eq!(health_json["status"].as_str(), Some("ok"));
        assert_eq!(health_json["worker"]["restart_count"].as_i64(), Some(1));
        assert_eq!(health_json["worker"]["recovered_jobs"].as_i64(), Some(1));
        assert_eq!(
            health_json["worker"]["last_exit_reason"].as_str(),
            Some("worker_crash_after_claim")
        );
        assert_eq!(health_json["worker"]["recovery_active"].as_bool(), Some(false));
        assert_eq!(health_json["worker"]["last_job_id"].as_str(), Some(job_id.as_str()));
    }));
    let logs = stop_reference_backend(spawned);

    match test_result {
        Ok(()) => {
            assert_startup_logs(&logs.combined, &config, &database_url);
            assert!(
                logs.combined.contains("Job worker crash injected id="),
                "expected injected crash log in runtime output:\n{}",
                logs.combined
            );
            assert!(
                logs.combined.contains("Job worker recovered jobs=1"),
                "expected recovery log in runtime output:\n{}",
                logs.combined
            );
        }
        Err(payload) => panic!(
            "reference-backend worker-crash recovery assertions failed: {}\nstdout: {}\nstderr: {}",
            panic_payload_to_string(payload),
            logs.stdout,
            logs.stderr
        ),
    }
}

#[test]
#[ignore]
fn e2e_reference_backend_worker_restart_is_visible_in_health() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_worker_restart_is_visible_in_health");

    reset_reference_backend_database(&database_url);
    assert_reference_backend_build_succeeds();
    assert_reference_backend_migration_succeeds(&database_url, "up");

    let config = reference_backend_test_config(500);
    let spawned = spawn_reference_backend(&database_url, config);
    let test_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let startup_health = wait_for_reference_backend(&config);
        let startup_boot_id = startup_health["worker"]["boot_id"]
            .as_str()
            .expect("startup health must expose boot_id")
            .to_string();
        let startup_started_at = startup_health["worker"]["started_at"]
            .as_str()
            .expect("startup health must expose started_at")
            .to_string();
        assert!(!startup_boot_id.is_empty(), "startup boot_id must be non-empty");
        assert!(
            !startup_started_at.is_empty(),
            "startup started_at must be non-empty"
        );

        let payload_body = r#"{"kind":"crash_after_claim_once","source":"rust-harness"}"#;
        let created_job = post_json(&config, "/jobs", payload_body, 201);
        let job_id = created_job["id"]
            .as_str()
            .expect("created job id must be a string")
            .to_string();

        let (degraded_health, degraded_job, degraded_db_row) = wait_for_worker_recovery_window(
            &config,
            &database_url,
            &job_id,
            "worker_crash_after_claim",
            1,
            1,
        );

        assert_eq!(degraded_health["status"].as_str(), Some("degraded"));
        assert_eq!(degraded_health["worker"]["liveness"].as_str(), Some("recovering"));
        assert_eq!(degraded_health["worker"]["restart_count"].as_i64(), Some(1));
        assert_eq!(degraded_health["worker"]["recovered_jobs"].as_i64(), Some(1));
        assert_eq!(
            degraded_health["worker"]["last_exit_reason"].as_str(),
            Some("worker_crash_after_claim")
        );
        assert_eq!(degraded_health["worker"]["recovery_active"].as_bool(), Some(true));
        let degraded_boot_id = degraded_health["worker"]["boot_id"]
            .as_str()
            .expect("degraded health must expose boot_id")
            .to_string();
        let degraded_started_at = degraded_health["worker"]["started_at"]
            .as_str()
            .expect("degraded health must expose started_at")
            .to_string();
        assert!(!degraded_boot_id.is_empty(), "degraded boot_id must be non-empty");
        assert!(
            !degraded_started_at.is_empty(),
            "degraded started_at must be non-empty"
        );
        assert_ne!(
            degraded_boot_id, startup_boot_id,
            "boot_id should change after the worker restarts"
        );
        assert_ne!(
            degraded_started_at, startup_started_at,
            "started_at should change after the worker restarts"
        );
        assert!(
            degraded_health["worker"]["last_recovery_at"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "degraded health should expose last_recovery_at: {}",
            degraded_health
        );
        assert_eq!(
            degraded_health["worker"]["last_recovery_count"].as_i64(),
            Some(1)
        );
        assert_eq!(
            degraded_health["worker"]["last_recovery_job_id"].as_str(),
            Some(job_id.as_str())
        );

        assert_eq!(degraded_job["id"].as_str(), Some(job_id.as_str()));
        assert_eq!(degraded_job["status"].as_str(), Some("pending"));
        assert_eq!(degraded_job["attempts"].as_i64(), Some(1));
        assert_eq!(
            degraded_job["last_error"].as_str(),
            Some("requeued after worker restart")
        );

        assert_eq!(degraded_db_row.get("status").map(String::as_str), Some("pending"));
        assert_eq!(degraded_db_row.get("attempts").map(String::as_str), Some("1"));
        assert_eq!(
            degraded_db_row.get("last_error").map(String::as_str),
            Some("requeued after worker restart")
        );

        let (job_json, final_health) = wait_for_processed_job_and_health(&config, &job_id);
        assert_eq!(job_json["status"].as_str(), Some("processed"));
        assert_eq!(job_json["attempts"].as_i64(), Some(2));
        assert_eq!(final_health["status"].as_str(), Some("ok"));
        assert_eq!(final_health["worker"]["liveness"].as_str(), Some("healthy"));
        assert_eq!(final_health["worker"]["restart_count"].as_i64(), Some(1));
        assert_eq!(final_health["worker"]["recovered_jobs"].as_i64(), Some(1));
        assert_eq!(final_health["worker"]["recovery_active"].as_bool(), Some(false));
        assert_eq!(
            final_health["worker"]["boot_id"].as_str(),
            Some(degraded_boot_id.as_str())
        );
        assert_eq!(
            final_health["worker"]["started_at"].as_str(),
            Some(degraded_started_at.as_str())
        );
        assert_eq!(
            final_health["worker"]["last_recovery_job_id"].as_str(),
            Some(job_id.as_str())
        );
        assert_eq!(
            final_health["worker"]["last_recovery_count"].as_i64(),
            Some(1)
        );
        assert!(
            final_health["worker"]["last_recovery_at"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "final health should preserve last_recovery_at: {}",
            final_health
        );
    }));
    let logs = stop_reference_backend(spawned);

    match test_result {
        Ok(()) => {
            assert_startup_logs(&logs.combined, &config, &database_url);
            assert!(
                logs.combined.contains("Job worker boot id="),
                "expected worker boot log in runtime output:\n{}",
                logs.combined
            );
        }
        Err(payload) => panic!(
            "reference-backend health-visibility assertions failed: {}\nstdout: {}\nstderr: {}",
            panic_payload_to_string(payload),
            logs.stdout,
            logs.stderr
        ),
    }
}

#[test]
#[ignore]
fn e2e_reference_backend_process_restart_recovers_inflight_job() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_process_restart_recovers_inflight_job");

    reset_reference_backend_database(&database_url);
    assert_reference_backend_build_succeeds();
    assert_reference_backend_migration_succeeds(&database_url, "up");

    let config = reference_backend_test_config(100);
    let mut first_backend = Some(spawn_reference_backend(&database_url, config));
    let mut first_logs: Option<StoppedReferenceBackend> = None;
    let mut restarted_backend: Option<SpawnedReferenceBackend> = None;
    let test_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let startup_health = wait_for_reference_backend(&config);
        let first_boot_id = startup_health["worker"]["boot_id"]
            .as_str()
            .expect("startup health must expose boot_id")
            .to_string();
        let first_started_at = startup_health["worker"]["started_at"]
            .as_str()
            .expect("startup health must expose started_at")
            .to_string();
        assert!(!first_boot_id.is_empty(), "startup boot_id must be non-empty");
        assert!(
            !first_started_at.is_empty(),
            "startup started_at must be non-empty"
        );
        assert_eq!(startup_health["worker"]["restart_count"].as_i64(), Some(0));
        assert_eq!(startup_health["worker"]["recovered_jobs"].as_i64(), Some(0));

        let payload_body = r#"{"kind":"hold_after_claim_once","source":"rust-harness"}"#;
        let payload_json: Value =
            serde_json::from_str(payload_body).expect("payload must be valid JSON");
        let created_job = post_json(&config, "/jobs", payload_body, 201);
        let job_id = created_job["id"]
            .as_str()
            .expect("created job id must be a string")
            .to_string();

        let (inflight_health, inflight_job, inflight_db_row) =
            wait_for_inflight_processing_window(&config, &database_url, &job_id);
        assert_eq!(inflight_health["status"].as_str(), Some("ok"));
        assert_eq!(inflight_health["worker"]["status"].as_str(), Some("processing"));
        assert_eq!(inflight_health["worker"]["liveness"].as_str(), Some("healthy"));
        assert_eq!(inflight_health["worker"]["restart_count"].as_i64(), Some(0));
        assert_eq!(inflight_health["worker"]["recovered_jobs"].as_i64(), Some(0));
        assert_eq!(inflight_health["worker"]["last_job_id"].as_str(), Some(job_id.as_str()));
        assert_eq!(inflight_job["status"].as_str(), Some("processing"));
        assert_eq!(inflight_job["attempts"].as_i64(), Some(1));
        assert_eq!(inflight_job["last_error"], Value::Null);
        assert_eq!(inflight_db_row.get("status").map(String::as_str), Some("processing"));
        assert_eq!(inflight_db_row.get("attempts").map(String::as_str), Some("1"));
        assert_eq!(inflight_db_row.get("last_error").map(String::as_str), Some(""));

        first_logs = Some(kill_reference_backend(
            first_backend.take().expect("first backend must exist before kill"),
        ));
        let first_runtime_logs = &first_logs.as_ref().unwrap().combined;
        assert!(
            first_runtime_logs.contains(&format!("Job worker claimed id={job_id}")),
            "expected claimed log before process kill:\n{}",
            first_runtime_logs
        );
        assert!(
            first_runtime_logs.contains(&format!("Job worker hold-after-claim id={job_id}")),
            "expected hold-after-claim log before process kill:\n{}",
            first_runtime_logs
        );
        assert!(
            !first_runtime_logs.contains(&format!("Job worker processed id={job_id}")),
            "backend should have been killed before processing completed:\n{}",
            first_runtime_logs
        );

        restarted_backend = Some(spawn_reference_backend(&database_url, config));
        let (degraded_health, degraded_job, degraded_db_row) =
            wait_for_boot_recovery_window(&config, &database_url, &job_id);

        assert_eq!(degraded_health["status"].as_str(), Some("degraded"));
        assert_eq!(degraded_health["worker"]["liveness"].as_str(), Some("recovering"));
        assert_eq!(degraded_health["worker"]["restart_count"].as_i64(), Some(0));
        assert_eq!(degraded_health["worker"]["recovered_jobs"].as_i64(), Some(1));
        assert_eq!(degraded_health["worker"]["recovery_active"].as_bool(), Some(true));
        assert_eq!(degraded_health["worker"]["last_exit_reason"], Value::Null);
        let degraded_boot_id = degraded_health["worker"]["boot_id"]
            .as_str()
            .expect("degraded health must expose boot_id")
            .to_string();
        let degraded_started_at = degraded_health["worker"]["started_at"]
            .as_str()
            .expect("degraded health must expose started_at")
            .to_string();
        assert!(!degraded_boot_id.is_empty(), "degraded boot_id must be non-empty");
        assert!(
            !degraded_started_at.is_empty(),
            "degraded started_at must be non-empty"
        );
        assert_ne!(
            degraded_boot_id, first_boot_id,
            "boot_id should change after whole-process restart"
        );
        assert_ne!(
            degraded_started_at, first_started_at,
            "started_at should change after whole-process restart"
        );
        assert!(
            degraded_health["worker"]["last_recovery_at"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "degraded health should expose last_recovery_at: {}",
            degraded_health
        );
        assert_eq!(
            degraded_health["worker"]["last_recovery_count"].as_i64(),
            Some(1)
        );
        assert_eq!(
            degraded_health["worker"]["last_recovery_job_id"].as_str(),
            Some(job_id.as_str())
        );
        assert_eq!(degraded_job["status"].as_str(), Some("pending"));
        assert_eq!(degraded_job["attempts"].as_i64(), Some(1));
        assert_eq!(
            degraded_job["last_error"].as_str(),
            Some("requeued abandoned processing job during boot recovery")
        );
        assert_eq!(degraded_db_row.get("status").map(String::as_str), Some("pending"));
        assert_eq!(degraded_db_row.get("attempts").map(String::as_str), Some("1"));
        assert_eq!(
            degraded_db_row.get("last_error").map(String::as_str),
            Some("requeued abandoned processing job during boot recovery")
        );

        let (job_json, final_health) = wait_for_processed_job_and_health(&config, &job_id);
        let db_row = wait_for_job_row(&database_url, &job_id, "processed", "2");

        assert_eq!(job_json["id"].as_str(), Some(job_id.as_str()));
        assert_eq!(job_json["status"].as_str(), Some("processed"));
        assert_eq!(job_json["attempts"].as_i64(), Some(2));
        assert_eq!(job_json["last_error"], Value::Null);
        assert_eq!(job_json["payload"], payload_json);
        assert!(
            job_json["processed_at"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "processed job should expose processed_at: {}",
            job_json
        );

        assert_eq!(db_row.get("status").map(String::as_str), Some("processed"));
        assert_eq!(db_row.get("attempts").map(String::as_str), Some("2"));
        assert_eq!(db_row.get("last_error").map(String::as_str), Some(""));
        assert!(
            db_row
                .get("processed_at")
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "processed DB row should expose processed_at: {:?}",
            db_row
        );

        assert_eq!(final_health["status"].as_str(), Some("ok"));
        assert_eq!(final_health["worker"]["liveness"].as_str(), Some("healthy"));
        assert_eq!(final_health["worker"]["restart_count"].as_i64(), Some(0));
        assert_eq!(final_health["worker"]["recovered_jobs"].as_i64(), Some(1));
        assert_eq!(final_health["worker"]["recovery_active"].as_bool(), Some(false));
        assert_eq!(final_health["worker"]["last_exit_reason"], Value::Null);
        assert_eq!(final_health["worker"]["last_job_id"].as_str(), Some(job_id.as_str()));
        assert_eq!(
            final_health["worker"]["boot_id"].as_str(),
            Some(degraded_boot_id.as_str())
        );
        assert_eq!(
            final_health["worker"]["started_at"].as_str(),
            Some(degraded_started_at.as_str())
        );
        assert_eq!(
            final_health["worker"]["last_recovery_job_id"].as_str(),
            Some(job_id.as_str())
        );
        assert_eq!(
            final_health["worker"]["last_recovery_count"].as_i64(),
            Some(1)
        );
        assert!(
            final_health["worker"]["last_recovery_at"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "final health should preserve last_recovery_at: {}",
            final_health
        );
    }));

    let first_logs = match first_logs {
        Some(logs) => logs,
        None => first_backend
            .take()
            .map(stop_reference_backend)
            .unwrap_or_else(|| StoppedReferenceBackend {
                stdout: String::new(),
                stderr: String::new(),
                combined: String::new(),
            }),
    };
    let restarted_logs = restarted_backend.take().map(stop_reference_backend);

    match test_result {
        Ok(()) => {
            assert_startup_logs(&first_logs.combined, &config, &database_url);
            let restarted_logs = restarted_logs.expect("restarted backend logs must exist on success");
            assert_startup_logs(&restarted_logs.combined, &config, &database_url);
            assert!(
                restarted_logs.combined.contains("Job worker recovered jobs=1"),
                "expected recovery log after whole-process restart:\n{}",
                restarted_logs.combined
            );
            assert!(
                restarted_logs.combined.contains("Job worker processed id="),
                "expected processed log after whole-process restart:\n{}",
                restarted_logs.combined
            );
        }
        Err(payload) => panic!(
            "reference-backend process-restart recovery assertions failed: {}\nfirst_stdout: {}\nfirst_stderr: {}\nrestarted_stdout: {}\nrestarted_stderr: {}",
            panic_payload_to_string(payload),
            first_logs.stdout,
            first_logs.stderr,
            restarted_logs
                .as_ref()
                .map(|logs| logs.stdout.as_str())
                .unwrap_or(""),
            restarted_logs
                .as_ref()
                .map(|logs| logs.stderr.as_str())
                .unwrap_or("")
        ),
    }
}

#[test]
#[ignore]
fn e2e_reference_backend_postgres_smoke() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for e2e_reference_backend_postgres_smoke");
    assert_reference_backend_postgres_smoke(&database_url);
}
