use super::m046_route_free as route_free;
use mesh_rt::db::pg::{native_pg_close, native_pg_connect, native_pg_execute, native_pg_query};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

pub const PHASE_TIMEOUT: Duration = Duration::from_secs(120);
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(25);
pub const BINARY_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_PROJECT_SLUG: &str = "default";
pub const DEFAULT_API_KEY: &str = "mshr_devdefaultapikey000000000000000000000000000";

const POSTGRES_IMAGE: &str = "postgres:16";
const POSTGRES_CONTAINER_PREFIX: &str = "mesh-m051-s01-pg";

pub type DbRow = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct MesherRuntimeConfig {
    pub http_port: u16,
    pub ws_port: u16,
    pub database_url: String,
    pub rate_limit_window_seconds: u64,
    pub rate_limit_max_events: u64,
    pub cluster_cookie: String,
    pub node_name: String,
    pub discovery_seed: String,
    pub cluster_port: u16,
    pub cluster_role: String,
    pub promotion_epoch: u64,
}

pub struct StartedPostgresContainer {
    name: String,
    pub host_port: u16,
    pub database_url: String,
    artifacts: PathBuf,
}

pub struct CompletedCommand {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub combined: String,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
    pub meta_path: PathBuf,
    pub duration: Duration,
}

pub struct SpawnedMesher {
    child: Child,
    stdout_capture: NamedTempFile,
    stderr_capture: NamedTempFile,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
}

pub struct StoppedMesher {
    pub stdout: String,
    pub stderr: String,
    pub combined: String,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub body: String,
    pub raw: String,
}

impl StartedPostgresContainer {
    fn snapshot_state(&self) {
        let logs_output = Command::new("docker").args(["logs", &self.name]).output();
        if let Ok(output) = logs_output {
            write_artifact(
                &self.artifacts.join("postgres.logs.txt"),
                command_output_text(&output),
            );
        }

        let inspect_output = Command::new("docker")
            .args(["inspect", &self.name])
            .output();
        if let Ok(output) = inspect_output {
            if output.status.success() {
                let raw = String::from_utf8_lossy(&output.stdout);
                write_artifact(&self.artifacts.join("postgres.inspect.json"), raw.as_ref());
            } else {
                write_artifact(
                    &self.artifacts.join("postgres.inspect.txt"),
                    command_output_text(&output),
                );
            }
        }
    }
}

impl Drop for StartedPostgresContainer {
    fn drop(&mut self) {
        self.snapshot_state();
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.name])
            .output();
    }
}

pub fn repo_root() -> PathBuf {
    route_free::repo_root()
}

pub fn meshc_bin() -> PathBuf {
    route_free::meshc_bin()
}

pub fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m051-s01", test_name)
}

pub fn ensure_mesh_rt_staticlib() {
    route_free::ensure_mesh_rt_staticlib();
}

pub fn write_artifact(path: &Path, contents: impl AsRef<str>) {
    route_free::write_artifact(path, contents.as_ref());
}

pub fn write_json_artifact(path: &Path, value: &impl serde::Serialize) {
    route_free::write_json_artifact(path, value);
}

pub fn command_output_text(output: &Output) -> String {
    format!(
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

pub fn default_runtime_config(database_url: &str) -> MesherRuntimeConfig {
    let cluster_port = route_free::dual_stack_cluster_port();
    MesherRuntimeConfig {
        http_port: unused_port(),
        ws_port: unused_port(),
        database_url: database_url.to_string(),
        rate_limit_window_seconds: 60,
        rate_limit_max_events: 1000,
        cluster_cookie: format!("m051-s01-cookie-{}", unique_stamp()),
        node_name: format!("mesher@127.0.0.1:{cluster_port}"),
        discovery_seed: "localhost".to_string(),
        cluster_port,
        cluster_role: "primary".to_string(),
        promotion_epoch: 0,
    }
}

pub fn start_postgres_container(artifacts: &Path, label: &str) -> StartedPostgresContainer {
    cleanup_stale_postgres_containers();

    let host_port = unused_port();
    let stamp = unique_stamp();
    let name = format!(
        "{POSTGRES_CONTAINER_PREFIX}-{}-{stamp}",
        sanitize_label(label)
    );
    let database_url = format!("postgres://mesh:mesh@127.0.0.1:{host_port}/mesher");

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-d",
            "--name",
            &name,
            "-e",
            "POSTGRES_USER=mesh",
            "-e",
            "POSTGRES_PASSWORD=mesh",
            "-e",
            "POSTGRES_DB=mesher",
            "-p",
            &format!("127.0.0.1:{host_port}:5432"),
            POSTGRES_IMAGE,
        ])
        .output()
        .unwrap_or_else(|error| panic!("failed to start temporary postgres container: {error}"));

    write_artifact(
        &artifacts.join("postgres.start.log"),
        command_output_text(&output),
    );
    assert!(
        output.status.success(),
        "failed to start temporary postgres container:\n{}",
        command_output_text(&output)
    );

    write_json_artifact(
        &artifacts.join("postgres.meta.json"),
        &serde_json::json!({
            "container_name": name,
            "image": POSTGRES_IMAGE,
            "host_port": host_port,
            "database_url": "<redacted:DATABASE_URL>",
        }),
    );

    let container = StartedPostgresContainer {
        name,
        host_port,
        database_url,
        artifacts: artifacts.to_path_buf(),
    };
    wait_for_postgres_ready(&container);
    container
}

pub fn run_meshc_migrate_up(database_url: &str, artifacts: &Path) -> CompletedCommand {
    let mut command = Command::new(meshc_bin());
    command
        .current_dir(repo_root())
        .env("DATABASE_URL", database_url)
        .args(["migrate", "mesher", "up"]);
    run_command_capture(
        &mut command,
        artifacts,
        "migrate-up",
        "meshc migrate mesher up",
        PHASE_TIMEOUT,
        &[database_url],
    )
}

pub fn run_meshc_build(artifacts: &Path) -> (CompletedCommand, PathBuf) {
    ensure_mesh_rt_staticlib();

    let binary_dir = artifacts.join("bin");
    fs::create_dir_all(&binary_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", binary_dir.display()));
    let binary_path = binary_dir.join("mesher");

    let mut command = Command::new(meshc_bin());
    command
        .current_dir(repo_root())
        .arg("build")
        .arg("mesher")
        .arg("--output")
        .arg(&binary_path);
    let run = run_command_capture(
        &mut command,
        artifacts,
        "build",
        "meshc build mesher --output <artifact-bin>",
        PHASE_TIMEOUT,
        &[],
    );
    if run.status.success() {
        assert!(
            binary_path.exists(),
            "meshc build reported success but binary is missing at {}",
            binary_path.display()
        );
        write_json_artifact(
            &artifacts.join("build-output.json"),
            &serde_json::json!({
                "binary_path": binary_path,
                "source_package_dir": repo_root().join("mesher"),
            }),
        );
    }
    (run, binary_path)
}

pub fn spawn_mesher(
    binary_path: &Path,
    artifacts: &Path,
    label: &str,
    config: &MesherRuntimeConfig,
) -> SpawnedMesher {
    let stdout_capture = NamedTempFile::new().expect("failed to create temp stdout capture");
    let stderr_capture = NamedTempFile::new().expect("failed to create temp stderr capture");
    let stdout_path = artifacts.join(format!("{label}.stdout.log"));
    let stderr_path = artifacts.join(format!("{label}.stderr.log"));

    let stdout = stdout_capture
        .reopen()
        .expect("failed to reopen temp stdout capture");
    let stderr = stderr_capture
        .reopen()
        .expect("failed to reopen temp stderr capture");

    let mut command = Command::new(binary_path);
    command
        .current_dir(repo_root())
        .env("DATABASE_URL", &config.database_url)
        .env("PORT", config.http_port.to_string())
        .env("MESHER_WS_PORT", config.ws_port.to_string())
        .env(
            "MESHER_RATE_LIMIT_WINDOW_SECONDS",
            config.rate_limit_window_seconds.to_string(),
        )
        .env(
            "MESHER_RATE_LIMIT_MAX_EVENTS",
            config.rate_limit_max_events.to_string(),
        )
        .env("MESH_CLUSTER_COOKIE", &config.cluster_cookie)
        .env("MESH_NODE_NAME", &config.node_name)
        .env("MESH_DISCOVERY_SEED", &config.discovery_seed)
        .env("MESH_CLUSTER_PORT", config.cluster_port.to_string())
        .env("MESH_CONTINUITY_ROLE", &config.cluster_role)
        .env(
            "MESH_CONTINUITY_PROMOTION_EPOCH",
            config.promotion_epoch.to_string(),
        )
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    let child = command.spawn().unwrap_or_else(|error| {
        panic!(
            "failed to start Mesher runtime {}: {error}",
            binary_path.display()
        )
    });

    write_artifact(
        &artifacts.join(format!("{label}.meta.txt")),
        format!(
            "description: mesher runtime\ncwd: {}\ncommand: {}\n",
            repo_root().display(),
            binary_path.display()
        ),
    );

    SpawnedMesher {
        child,
        stdout_capture,
        stderr_capture,
        stdout_path,
        stderr_path,
    }
}

pub fn stop_mesher(mut spawned: SpawnedMesher, secret_values: &[&str]) -> StoppedMesher {
    let _ = spawned.child.kill();
    let _ = spawned.child.wait();

    let stdout = read_capture_file(spawned.stdout_capture.path(), secret_values);
    let stderr = read_capture_file(spawned.stderr_capture.path(), secret_values);
    let combined = format!("{stdout}{stderr}");

    write_artifact(&spawned.stdout_path, &stdout);
    write_artifact(&spawned.stderr_path, &stderr);

    StoppedMesher {
        stdout,
        stderr,
        combined,
        stdout_path: spawned.stdout_path,
        stderr_path: spawned.stderr_path,
    }
}

pub fn wait_for_project_settings(
    config: &MesherRuntimeConfig,
    artifacts: &Path,
    label: &str,
    secret_values: &[&str],
) -> Value {
    let start = Instant::now();
    let mut last_observation = String::new();

    while start.elapsed() < STARTUP_TIMEOUT {
        match send_http_request(
            config.http_port,
            "GET",
            &format!("/api/v1/projects/{DEFAULT_PROJECT_SLUG}/settings"),
            None,
            &[],
        ) {
            Ok(response) if response.status_code == 200 => {
                let json = json_response_snapshot(
                    artifacts,
                    label,
                    &response,
                    200,
                    "Mesher project settings readiness",
                    secret_values,
                );
                if json["retention_days"].is_number() && json["sample_rate"].is_number() {
                    return json;
                }
                last_observation = redact_text(
                    &serde_json::to_string_pretty(&json)
                        .expect("failed to pretty-print readiness JSON"),
                    secret_values,
                );
            }
            Ok(response) => last_observation = redact_text(&response.raw, secret_values),
            Err(error) => last_observation = format!("connect error: {error}"),
        }
        sleep(Duration::from_millis(250));
    }

    let timeout_path = artifacts.join(format!("{label}.timeout.txt"));
    write_artifact(&timeout_path, &last_observation);
    panic!(
        "Mesher never reached /api/v1/projects/{DEFAULT_PROJECT_SLUG}/settings readiness on :{} within {:?}; last observation archived at {}",
        config.http_port,
        STARTUP_TIMEOUT,
        timeout_path.display()
    );
}

pub fn send_http_request(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&str>,
    headers: &[(&str, &str)],
) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;

    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    for (name, value) in headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    if let Some(body) = body {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.as_bytes().len()));
        request.push_str("\r\n");
        request.push_str(body);
    } else {
        request.push_str("\r\n");
    }

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

pub fn archive_raw_response(
    artifacts: &Path,
    name: &str,
    response: &HttpResponse,
    secret_values: &[&str],
) -> PathBuf {
    let path = artifacts.join(format!("{name}.http"));
    write_artifact(&path, redact_text(&response.raw, secret_values));
    path
}

pub fn parse_json_snapshot(
    artifacts: &Path,
    name: &str,
    response: &HttpResponse,
    context: &str,
    secret_values: &[&str],
) -> Value {
    let raw_path = archive_raw_response(artifacts, name, response, secret_values);
    match serde_json::from_str::<Value>(&response.body) {
        Ok(json) => {
            let json_path = artifacts.join(format!("{name}.json"));
            write_json_artifact(&json_path, &json);
            json
        }
        Err(error) => {
            let body_path = artifacts.join(format!("{name}.body.txt"));
            write_artifact(&body_path, redact_text(&response.body, secret_values));
            panic!(
                "expected JSON body for {context}, got {error}. raw response: {} body artifact: {}",
                raw_path.display(),
                body_path.display()
            );
        }
    }
}

pub fn json_response_snapshot(
    artifacts: &Path,
    name: &str,
    response: &HttpResponse,
    expected_status: u16,
    context: &str,
    secret_values: &[&str],
) -> Value {
    if response.status_code != expected_status {
        let raw_path = archive_raw_response(artifacts, name, response, secret_values);
        panic!(
            "expected HTTP {expected_status} for {context}, got raw response in {}:\n{}",
            raw_path.display(),
            redact_text(&response.raw, secret_values)
        );
    }
    parse_json_snapshot(artifacts, name, response, context, secret_values)
}

pub fn query_database_rows(database_url: &str, sql: &str, params: &[&str]) -> Vec<DbRow> {
    let mut conn = native_pg_connect(database_url)
        .unwrap_or_else(|error| panic!("failed to connect to Postgres for query: {error}"));
    let result = native_pg_query(&mut conn, sql, params);
    native_pg_close(conn);
    let rows = result.unwrap_or_else(|error| panic!("query failed: {error}\nsql: {sql}"));
    rows.into_iter()
        .map(|row| row.into_iter().collect())
        .collect()
}

pub fn query_single_row(database_url: &str, sql: &str, params: &[&str]) -> DbRow {
    let rows = query_database_rows(database_url, sql, params);
    assert_eq!(rows.len(), 1, "expected exactly one row for SQL: {sql}");
    rows.into_iter().next().unwrap()
}

pub fn wait_for_query_value(
    database_url: &str,
    sql: &str,
    params: &[&str],
    column: &str,
    expected: &str,
    description: &str,
) -> DbRow {
    let mut last_row = DbRow::new();

    for attempt in 0..40 {
        if attempt > 0 {
            sleep(Duration::from_millis(250));
        }

        let row = query_single_row(database_url, sql, params);
        if row.get(column).map(String::as_str) == Some(expected) {
            return row;
        }
        last_row = row;
    }

    panic!(
        "timed out waiting for {description}; expected {column}={expected}, last_row={last_row:?}"
    );
}

pub fn assert_phase_success(run: &CompletedCommand, description: &str) {
    assert!(
        run.status.success(),
        "{description} failed after {:?}; stdout={} stderr={} meta={}\n{}",
        run.duration,
        run.stdout_path.display(),
        run.stderr_path.display(),
        run.meta_path.display(),
        run.combined
    );
}

pub fn assert_runtime_logs(logs: &StoppedMesher, config: &MesherRuntimeConfig) {
    assert!(
        logs.combined.contains("[Mesher] Config loaded http_port="),
        "expected config-loaded log line, got:\n{}",
        logs.combined
    );
    assert!(
        logs.combined
            .contains("[Mesher] Connecting to PostgreSQL pool..."),
        "expected connect log line, got:\n{}",
        logs.combined
    );
    assert!(
        logs.combined.contains("[Mesher] PostgreSQL pool ready"),
        "expected pool-ready log line, got:\n{}",
        logs.combined
    );
    assert!(
        logs.combined.contains("[Mesher] runtime bootstrap mode="),
        "expected runtime-bootstrap log line, got:\n{}",
        logs.combined
    );
    assert!(
        logs.combined.contains("[Mesher] Foundation ready"),
        "expected foundation-ready log line, got:\n{}",
        logs.combined
    );
    assert!(
        logs.combined.contains(&format!(
            "[Mesher] Runtime ready http_port={} ws_port={} db_backend=postgres rate_limit_window_seconds={} rate_limit_max_events={}",
            config.http_port, config.ws_port, config.rate_limit_window_seconds, config.rate_limit_max_events
        )),
        "expected runtime-ready log line, got:\n{}",
        logs.combined
    );
    assert!(
        logs.combined.contains(&format!(
            "[Mesher] WebSocket server starting on :{}",
            config.ws_port
        )),
        "expected websocket-start log line, got:\n{}",
        logs.combined
    );
    assert!(
        logs.combined.contains(&format!(
            "[Mesher] HTTP server starting on :{}",
            config.http_port
        )),
        "expected HTTP-start log line, got:\n{}",
        logs.combined
    );
    assert!(
        !logs.combined.contains(&config.database_url),
        "runtime logs must not echo DATABASE_URL\nlogs:\n{}",
        logs.combined
    );
}

pub fn assert_artifacts_redacted(artifacts: &Path, secret_values: &[&str]) {
    assert_artifacts_redacted_recursive(artifacts, secret_values);
}

pub fn redact_text(text: &str, secret_values: &[&str]) -> String {
    let mut redacted = text.to_string();
    for secret in secret_values {
        if !secret.is_empty() {
            redacted = redacted.replace(secret, "<redacted:DATABASE_URL>");
        }
    }
    redacted
}

pub fn run_command_capture(
    command: &mut Command,
    artifacts: &Path,
    label: &str,
    description: &str,
    timeout: Duration,
    secret_values: &[&str],
) -> CompletedCommand {
    let stdout_path = artifacts.join(format!("{label}.stdout.log"));
    let stderr_path = artifacts.join(format!("{label}.stderr.log"));
    let meta_path = artifacts.join(format!("{label}.meta.txt"));
    let timeout_path = artifacts.join(format!("{label}.timeout.txt"));

    let stdout_capture = NamedTempFile::new().expect("failed to create temp stdout capture");
    let stderr_capture = NamedTempFile::new().expect("failed to create temp stderr capture");
    let stdout = stdout_capture
        .reopen()
        .expect("failed to reopen temp stdout capture");
    let stderr = stderr_capture
        .reopen()
        .expect("failed to reopen temp stderr capture");

    write_artifact(
        &meta_path,
        format!(
            "description: {description}\ncwd: {}\ncommand: {}\nstatus: running\n",
            command
                .get_current_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<inherit>".to_string()),
            format_command(command)
        ),
    );

    command
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {description}: {error}"));
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let duration = start.elapsed();
                let stdout = read_capture_file(stdout_capture.path(), secret_values);
                let stderr = read_capture_file(stderr_capture.path(), secret_values);
                let combined = format!("{stdout}{stderr}");
                write_artifact(&stdout_path, &stdout);
                write_artifact(&stderr_path, &stderr);
                write_artifact(
                    &meta_path,
                    format!(
                        "description: {description}\ncwd: {}\ncommand: {}\nstatus: {:?}\nduration_ms: {}\n",
                        command
                            .get_current_dir()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| "<inherit>".to_string()),
                        format_command(command),
                        status.code(),
                        duration.as_millis()
                    ),
                );
                return CompletedCommand {
                    status,
                    stdout,
                    stderr,
                    combined,
                    stdout_path,
                    stderr_path,
                    meta_path,
                    duration,
                };
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let stdout = read_capture_file(stdout_capture.path(), secret_values);
                    let stderr = read_capture_file(stderr_capture.path(), secret_values);
                    write_artifact(&stdout_path, &stdout);
                    write_artifact(&stderr_path, &stderr);
                    write_artifact(
                        &timeout_path,
                        format!(
                            "description: {description}\nduration_ms: {}\nstdout: {}\nstderr: {}\n",
                            timeout.as_millis(),
                            stdout_path.display(),
                            stderr_path.display()
                        ),
                    );
                    panic!(
                        "{description} timed out after {:?}; partial logs: stdout={} stderr={} timeout={}",
                        timeout,
                        stdout_path.display(),
                        stderr_path.display(),
                        timeout_path.display()
                    );
                }
                sleep(Duration::from_millis(100));
            }
            Err(error) => panic!("failed to wait on {description}: {error}"),
        }
    }
}

fn cleanup_stale_postgres_containers() {
    let output = Command::new("docker")
        .args([
            "ps",
            "-aq",
            "--filter",
            &format!("name={POSTGRES_CONTAINER_PREFIX}"),
        ])
        .output()
        .expect("failed to list stale docker containers");
    if !output.status.success() {
        panic!(
            "failed to list stale docker containers:\n{}",
            command_output_text(&output)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let ids: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if ids.is_empty() {
        return;
    }

    let mut args = vec!["rm", "-f"];
    args.extend(ids.iter().copied());
    let cleanup = Command::new("docker")
        .args(args)
        .output()
        .expect("failed to remove stale docker containers");
    if !cleanup.status.success() {
        panic!(
            "failed to remove stale docker containers:\n{}",
            command_output_text(&cleanup)
        );
    }
}

fn wait_for_postgres_ready(container: &StartedPostgresContainer) {
    let start = Instant::now();
    let mut last_error = String::new();

    while start.elapsed() < STARTUP_TIMEOUT {
        match native_pg_connect(&container.database_url) {
            Ok(conn) => {
                native_pg_close(conn);
                return;
            }
            Err(error) => {
                last_error = redact_text(&error.to_string(), &[container.database_url.as_str()]);
                sleep(Duration::from_millis(250));
            }
        }
    }

    write_artifact(
        &container.artifacts.join("postgres.timeout.txt"),
        format!(
            "database_url: <redacted:DATABASE_URL>\nhost_port: {}\nlast_error: {}\n",
            container.host_port, last_error
        ),
    );
    container.snapshot_state();
    panic!(
        "temporary Postgres never accepted connections at <redacted:DATABASE_URL>; artifacts under {}",
        container.artifacts.display()
    );
}

fn sanitize_label(label: &str) -> String {
    let sanitized: String = label
        .chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "pg".to_string()
    } else {
        trimmed.to_string()
    }
}

fn unique_stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos()
}

fn unused_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("failed to bind ephemeral port")
        .local_addr()
        .expect("failed to read ephemeral port")
        .port()
}

fn format_command(command: &Command) -> String {
    let program = command.get_program().to_string_lossy().to_string();
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    if args.is_empty() {
        program
    } else {
        format!("{program} {args}")
    }
}

fn read_capture_file(path: &Path, secret_values: &[&str]) -> String {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read capture {}: {error}", path.display()));
    redact_text(&raw, secret_values)
}

fn assert_artifacts_redacted_recursive(path: &Path, secret_values: &[&str]) {
    if path.is_dir() {
        for entry in fs::read_dir(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        {
            let entry = entry
                .unwrap_or_else(|error| panic!("failed to iterate {}: {error}", path.display()));
            assert_artifacts_redacted_recursive(&entry.path(), secret_values);
        }
        return;
    }

    let bytes =
        fs::read(path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let content = String::from_utf8_lossy(&bytes);
    for secret in secret_values {
        if !secret.is_empty() {
            assert!(
                !content.contains(secret),
                "artifact {} leaked a raw DATABASE_URL",
                path.display()
            );
        }
    }
}
