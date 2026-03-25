use std::any::Any;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mesh_rt::db::pg::{native_pg_close, native_pg_connect, native_pg_execute, native_pg_query};
use serde_json::Value;

type DbRow = HashMap<String, String>;
type OutputMap = HashMap<String, String>;

const MESHER_DATABASE_URL: &str = "postgres://mesh:mesh@127.0.0.1:5432/mesher";
const POSTGRES_IMAGE: &str = "postgres:16";
const POSTGRES_CONTAINER_PREFIX: &str = "mesh-m033-s03-pg";

#[derive(Clone, Copy, Debug)]
struct MesherConfig {
    http_port: u16,
    ws_port: u16,
}

struct SpawnedMesher {
    child: Child,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

struct StoppedMesher {
    stdout: String,
    stderr: String,
    combined: String,
}

struct HttpResponse {
    status_code: u16,
    body: String,
    raw: String,
}

struct PostgresContainer {
    name: String,
}

impl Drop for PostgresContainer {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
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

    if path.file_name().is_some_and(|n| n == "deps") {
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

fn command_output_text(output: &Output) -> String {
    format!(
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_command_success(output: &Output, description: &str) {
    assert!(
        output.status.success(),
        "{description} failed:\n{}",
        command_output_text(output)
    );
}

fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        (*msg).to_string()
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn pick_unused_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("failed to bind ephemeral port")
        .local_addr()
        .expect("failed to read ephemeral port")
        .port()
}

fn mesher_test_config() -> MesherConfig {
    MesherConfig {
        http_port: pick_unused_port(),
        ws_port: pick_unused_port(),
    }
}

fn mesher_binary() -> PathBuf {
    repo_root().join("mesher").join("mesher")
}

fn mesher_log_paths() -> (PathBuf, PathBuf) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let base = std::env::temp_dir();
    let stdout_path = base.join(format!("mesher-{stamp}-stdout.log"));
    let stderr_path = base.join(format!("mesher-{stamp}-stderr.log"));
    (stdout_path, stderr_path)
}

fn build_mesher() -> Output {
    Command::new(find_meshc())
        .current_dir(repo_root())
        .args(["build", "mesher"])
        .output()
        .expect("failed to invoke meshc build mesher")
}

fn ensure_mesher_binary() {
    static BUILD_ONCE: OnceLock<()> = OnceLock::new();
    BUILD_ONCE.get_or_init(|| {
        let output = build_mesher();
        assert_command_success(&output, "meshc build mesher");
        assert!(mesher_binary().exists(), "mesher binary was not built");
    });
}

fn spawn_mesher(config: MesherConfig) -> SpawnedMesher {
    ensure_mesher_binary();

    let binary = mesher_binary();
    let (stdout_path, stderr_path) = mesher_log_paths();
    let stdout_file = File::create(&stdout_path)
        .unwrap_or_else(|e| panic!("failed to create {}: {}", stdout_path.display(), e));
    let stderr_file = File::create(&stderr_path)
        .unwrap_or_else(|e| panic!("failed to create {}: {}", stderr_path.display(), e));

    let child = Command::new(&binary)
        .current_dir(repo_root())
        .env("MESHER_HTTP_PORT", config.http_port.to_string())
        .env("MESHER_WS_PORT", config.ws_port.to_string())
        .env("MESHER_RATE_LIMIT_WINDOW_SECONDS", "60")
        .env("MESHER_RATE_LIMIT_MAX_EVENTS", "100")
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {}", binary.display(), e));

    SpawnedMesher {
        child,
        stdout_path,
        stderr_path,
    }
}

fn collect_stopped_mesher(
    mut child: Child,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
) -> StoppedMesher {
    child.wait().expect("failed to collect mesher exit status");

    let stdout = fs::read_to_string(&stdout_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", stdout_path.display(), e));
    let stderr = fs::read_to_string(&stderr_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", stderr_path.display(), e));
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    let combined = format!("{stdout}{stderr}");

    StoppedMesher {
        stdout,
        stderr,
        combined,
    }
}

fn stop_mesher(spawned: SpawnedMesher) -> StoppedMesher {
    let SpawnedMesher {
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
        .expect("failed to probe mesher exit status")
        .is_none()
    {
        let _ = child.kill();
    }

    collect_stopped_mesher(child, stdout_path, stderr_path)
}

fn assert_mesher_logs(logs: &StoppedMesher, config: &MesherConfig) {
    assert!(
        logs.combined.contains("[Mesher] Connecting to PostgreSQL..."),
        "mesher logs never showed the Postgres connection banner:\n{}",
        logs.combined
    );
    assert!(
        logs.combined
            .contains(&format!("[Mesher] HTTP server starting on :{}", config.http_port)),
        "mesher logs never showed the HTTP listener on :{}:\n{}",
        config.http_port,
        logs.combined
    );
}

fn send_http_request(
    config: &MesherConfig,
    method: &str,
    path: &str,
    body: Option<&str>,
    headers: &[(&str, &str)],
) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(("127.0.0.1", config.http_port))?;
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
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
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

fn get_json(config: &MesherConfig, path: &str, expected_status: u16) -> Value {
    let response = send_http_request(config, "GET", path, None, &[])
        .unwrap_or_else(|e| panic!("GET {path} failed on {}: {}", config.http_port, e));
    assert_json_response(response, expected_status, path)
}

fn post_json_with_headers(
    config: &MesherConfig,
    path: &str,
    body: &str,
    headers: &[(&str, &str)],
    expected_status: u16,
) -> Value {
    let response = send_http_request(config, "POST", path, Some(body), headers)
        .unwrap_or_else(|e| panic!("POST {path} failed on {}: {}", config.http_port, e));
    assert_json_response(response, expected_status, path)
}

fn wait_for_mesher(config: &MesherConfig) -> Value {
    let mut last_response = Value::Null;

    for attempt in 0..60 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(250));
        }

        match send_http_request(
            config,
            "GET",
            "/api/v1/projects/default/settings",
            None,
            &[],
        ) {
            Ok(response) if response.status_code == 200 => {
                let json = assert_json_response(response, 200, "/api/v1/projects/default/settings");
                if json["retention_days"].is_number() && json["sample_rate"].is_number() {
                    return json;
                }
                last_response = json;
            }
            Ok(response) => last_response = Value::String(response.raw),
            Err(_) => continue,
        }
    }

    panic!(
        "mesher never reached ready settings response on :{}; last_response={}",
        config.http_port, last_response
    );
}

fn wait_for_query_value(
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
            std::thread::sleep(Duration::from_millis(250));
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

fn json_value_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).expect("failed to serialize nested JSON value"),
    }
}

fn json_array_signature(items: &[Value], fields: &[&str]) -> String {
    items.iter()
        .map(|item| {
            fields
                .iter()
                .map(|field| json_value_string(&item[*field]))
                .collect::<Vec<_>>()
                .join("~")
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn url_encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn db_json(row: &DbRow, column: &str) -> Value {
    serde_json::from_str(
        row.get(column)
            .unwrap_or_else(|| panic!("missing column {column} in row {row:?}")),
    )
    .unwrap_or_else(|e| panic!("failed to parse JSON column {column}: {e}; row={row:?}"))
}

fn assert_nullable_timestamp(value: &Value, expected: &str, field: &str) {
    if expected.is_empty() {
        assert!(value.is_null(), "expected null for {field}, got {value}");
    } else {
        assert_eq!(value.as_str(), Some(expected), "timestamp drifted for {field}");
    }
}

fn cleanup_stale_mesher_postgres_containers() {
    let output = Command::new("docker")
        .args([
            "ps",
            "-aq",
            "--filter",
            &format!("name={POSTGRES_CONTAINER_PREFIX}"),
        ])
        .output()
        .expect("failed to list stale docker containers");
    assert!(
        output.status.success(),
        "failed to list stale docker containers:\n{}",
        command_output_text(&output)
    );

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
    assert!(
        cleanup.status.success(),
        "failed to remove stale docker containers:\n{}",
        command_output_text(&cleanup)
    );
}

fn wait_for_postgres_ready() {
    for _ in 0..80 {
        if native_pg_connect(MESHER_DATABASE_URL).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    panic!("temporary Postgres never accepted connections");
}

fn start_postgres_container(label: &str) -> PostgresContainer {
    cleanup_stale_mesher_postgres_containers();

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();
    let name = format!("{POSTGRES_CONTAINER_PREFIX}-{label}-{stamp}");
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
            "5432:5432",
            POSTGRES_IMAGE,
        ])
        .output()
        .expect("failed to start temporary postgres container");
    assert!(
        output.status.success(),
        "failed to start temporary postgres container:\n{}",
        command_output_text(&output)
    );

    wait_for_postgres_ready();
    PostgresContainer { name }
}

fn with_mesher_postgres<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _container = start_postgres_container(label);
    f()
}

fn run_mesher_migrations(database_url: &str) -> Output {
    Command::new(find_meshc())
        .current_dir(repo_root())
        .env("DATABASE_URL", database_url)
        .args(["migrate", "mesher", "up"])
        .output()
        .expect("failed to invoke meshc migrate mesher up")
}

fn query_database_rows(database_url: &str, sql: &str, params: &[&str]) -> Vec<DbRow> {
    let mut conn = native_pg_connect(database_url)
        .unwrap_or_else(|e| panic!("failed to connect to Postgres for query: {e}"));
    let result = native_pg_query(&mut conn, sql, params);
    native_pg_close(conn);
    let rows = result.unwrap_or_else(|e| panic!("query failed: {e}\nsql: {sql}"));
    rows.into_iter().map(|row| row.into_iter().collect()).collect()
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

fn ensure_today_event_partition(database_url: &str) {
    let day = query_single_row(
        database_url,
        "SELECT to_char(current_date, 'YYYYMMDD') AS suffix, current_date::text AS start_day, (current_date + 1)::text AS end_day",
        &[],
    );
    let suffix = day.get("suffix").expect("missing partition suffix");
    let start_day = day.get("start_day").expect("missing partition start_day");
    let end_day = day.get("end_day").expect("missing partition end_day");
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS events_{suffix} PARTITION OF events FOR VALUES FROM ('{start_day}') TO ('{end_day}')"
    );
    execute_database_sql(database_url, &sql, &[]);
}

fn default_project_id(database_url: &str) -> String {
    query_single_row(
        database_url,
        "SELECT id::text AS id FROM projects WHERE slug = 'default'",
        &[],
    )
    .remove("id")
    .expect("default project id missing")
}

fn project_org_id(database_url: &str, project_id: &str) -> String {
    query_single_row(
        database_url,
        "SELECT org_id::text AS org_id FROM projects WHERE id = $1::uuid",
        &[project_id],
    )
    .remove("org_id")
    .expect("project org_id missing")
}

fn insert_org_and_project(database_url: &str, slug: &str) -> String {
    let org_slug = format!("org-{slug}");
    let org_name = format!("Org {slug}");
    let project_name = format!("Project {slug}");

    let org_id = query_single_row(
        database_url,
        "INSERT INTO organizations (name, slug) VALUES ($1, $2) RETURNING id::text AS id",
        &[&org_name, &org_slug],
    )
    .remove("id")
    .expect("org id missing");

    query_single_row(
        database_url,
        "INSERT INTO projects (org_id, name, platform, slug) VALUES ($1::uuid, $2, 'mesh', $3) RETURNING id::text AS id",
        &[&org_id, &project_name, slug],
    )
    .remove("id")
    .expect("project id missing")
}

fn insert_issue(database_url: &str, project_id: &str, fingerprint: &str, title: &str, level: &str) -> String {
    query_single_row(
        database_url,
        "INSERT INTO issues (project_id, fingerprint, title, level) VALUES ($1::uuid, $2, $3, $4) RETURNING id::text AS id",
        &[project_id, fingerprint, title, level],
    )
    .remove("id")
    .expect("issue id missing")
}

fn insert_seed_event(
    database_url: &str,
    project_id: &str,
    issue_id: &str,
    level: &str,
    message: &str,
    fingerprint: &str,
    tags_json: &str,
) -> String {
    query_single_row(
        database_url,
        "INSERT INTO events (project_id, issue_id, level, message, fingerprint, tags, extra) VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6::jsonb, '{}'::jsonb) RETURNING id::text AS id",
        &[project_id, issue_id, level, message, fingerprint, tags_json],
    )
    .remove("id")
    .expect("event id missing")
}

fn insert_user(database_url: &str, email: &str, password: &str, display_name: &str) -> String {
    query_single_row(
        database_url,
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, crypt($2, gen_salt('bf', 12)), $3) RETURNING id::text AS id",
        &[email, password, display_name],
    )
    .remove("id")
    .expect("user id missing")
}

fn insert_session_with_offset(database_url: &str, token: &str, user_id: &str, offset_hours: i64) {
    let offset = offset_hours.to_string();
    execute_database_sql(
        database_url,
        "INSERT INTO sessions (token, user_id, expires_at) VALUES ($1, $2::uuid, now() + ($3 || ' hours')::interval)",
        &[token, user_id, &offset],
    );
}

fn insert_api_key_row(
    database_url: &str,
    project_id: &str,
    key_value: &str,
    label: &str,
    created_offset_minutes: i64,
    revoked_offset_minutes: Option<i64>,
) -> String {
    let created = created_offset_minutes.to_string();
    match revoked_offset_minutes {
        Some(revoked) => {
            let revoked = revoked.to_string();
            query_single_row(
                database_url,
                "INSERT INTO api_keys (project_id, key_value, label, created_at, revoked_at) VALUES ($1::uuid, $2, $3, now() + ($4 || ' minutes')::interval, now() + ($5 || ' minutes')::interval) RETURNING id::text AS id",
                &[project_id, key_value, label, &created, &revoked],
            )
            .remove("id")
            .expect("api key id missing")
        }
        None => query_single_row(
            database_url,
            "INSERT INTO api_keys (project_id, key_value, label, created_at) VALUES ($1::uuid, $2, $3, now() + ($4 || ' minutes')::interval) RETURNING id::text AS id",
            &[project_id, key_value, label, &created],
        )
        .remove("id")
        .expect("api key id missing"),
    }
}

fn insert_alert_rule_row(
    database_url: &str,
    project_id: &str,
    name: &str,
    condition_json: &str,
    action_json: &str,
    enabled: bool,
    cooldown_minutes: i64,
    last_fired_offset_minutes: Option<i64>,
    created_offset_minutes: i64,
) -> String {
    let enabled = if enabled { "true" } else { "false" };
    let cooldown = cooldown_minutes.to_string();
    let created = created_offset_minutes.to_string();
    match last_fired_offset_minutes {
        Some(last_fired) => {
            let last_fired = last_fired.to_string();
            query_single_row(
                database_url,
                "INSERT INTO alert_rules (project_id, name, condition_json, action_json, enabled, cooldown_minutes, last_fired_at, created_at) VALUES ($1::uuid, $2, $3::jsonb, $4::jsonb, $5::boolean, $6::int, now() + ($7 || ' minutes')::interval, now() + ($8 || ' minutes')::interval) RETURNING id::text AS id",
                &[
                    project_id,
                    name,
                    condition_json,
                    action_json,
                    enabled,
                    &cooldown,
                    &last_fired,
                    &created,
                ],
            )
            .remove("id")
            .expect("alert rule id missing")
        }
        None => query_single_row(
            database_url,
            "INSERT INTO alert_rules (project_id, name, condition_json, action_json, enabled, cooldown_minutes, created_at) VALUES ($1::uuid, $2, $3::jsonb, $4::jsonb, $5::boolean, $6::int, now() + ($7 || ' minutes')::interval) RETURNING id::text AS id",
            &[project_id, name, condition_json, action_json, enabled, &cooldown, &created],
        )
        .remove("id")
        .expect("alert rule id missing"),
    }
}

fn insert_org_membership_row(
    database_url: &str,
    user_id: &str,
    org_id: &str,
    role: &str,
    joined_offset_minutes: i64,
) -> String {
    let joined = joined_offset_minutes.to_string();
    query_single_row(
        database_url,
        "INSERT INTO org_memberships (user_id, org_id, role, joined_at) VALUES ($1::uuid, $2::uuid, $3, now() + ($4 || ' minutes')::interval) RETURNING id::text AS id",
        &[user_id, org_id, role, &joined],
    )
    .remove("id")
    .expect("org membership id missing")
}

fn update_issue_read_fields(
    database_url: &str,
    issue_id: &str,
    status: &str,
    event_count: i64,
    first_seen_offset_minutes: i64,
    last_seen_offset_minutes: i64,
    assigned_to: Option<&str>,
) {
    let event_count = event_count.to_string();
    let first_seen = first_seen_offset_minutes.to_string();
    let last_seen = last_seen_offset_minutes.to_string();
    match assigned_to {
        Some(assigned_to) => {
            execute_database_sql(
                database_url,
                "UPDATE issues SET status = $2, event_count = $3::int, first_seen = now() + ($4 || ' minutes')::interval, last_seen = now() + ($5 || ' minutes')::interval, assigned_to = $6::uuid WHERE id = $1::uuid",
                &[issue_id, status, &event_count, &first_seen, &last_seen, assigned_to],
            );
        }
        None => {
            execute_database_sql(
                database_url,
                "UPDATE issues SET status = $2, event_count = $3::int, first_seen = now() + ($4 || ' minutes')::interval, last_seen = now() + ($5 || ' minutes')::interval, assigned_to = NULL WHERE id = $1::uuid",
                &[issue_id, status, &event_count, &first_seen, &last_seen],
            );
        }
    }
}

fn insert_event_row(
    database_url: &str,
    project_id: &str,
    issue_id: &str,
    level: &str,
    message: &str,
    fingerprint: &str,
    exception_json: Option<&str>,
    stacktrace_json: Option<&str>,
    breadcrumbs_json: Option<&str>,
    tags_json: &str,
    extra_json: &str,
    user_context_json: Option<&str>,
    sdk_name: Option<&str>,
    sdk_version: Option<&str>,
    received_offset_minutes: i64,
) -> String {
    let received = received_offset_minutes.to_string();
    query_single_row(
        database_url,
        "INSERT INTO events (project_id, issue_id, level, message, fingerprint, exception, stacktrace, breadcrumbs, tags, extra, user_context, sdk_name, sdk_version, received_at) VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6::jsonb, $7::jsonb, $8::jsonb, $9::jsonb, $10::jsonb, $11::jsonb, $12, $13, now() + ($14 || ' minutes')::interval) RETURNING id::text AS id",
        &[
            project_id,
            issue_id,
            level,
            message,
            fingerprint,
            exception_json.unwrap_or("null"),
            stacktrace_json.unwrap_or("null"),
            breadcrumbs_json.unwrap_or("null"),
            tags_json,
            extra_json,
            user_context_json.unwrap_or("null"),
            sdk_name.unwrap_or(""),
            sdk_version.unwrap_or(""),
            &received,
        ],
    )
    .remove("id")
    .expect("event id missing")
}

fn insert_alert_row(
    database_url: &str,
    rule_id: &str,
    project_id: &str,
    status: &str,
    message: &str,
    condition_snapshot_json: &str,
    triggered_offset_minutes: i64,
    acknowledged_offset_minutes: Option<i64>,
    resolved_offset_minutes: Option<i64>,
) -> String {
    let triggered = triggered_offset_minutes.to_string();
    match (acknowledged_offset_minutes, resolved_offset_minutes) {
        (Some(acknowledged), Some(resolved)) => {
            let acknowledged = acknowledged.to_string();
            let resolved = resolved.to_string();
            query_single_row(
                database_url,
                "INSERT INTO alerts (rule_id, project_id, status, message, condition_snapshot, triggered_at, acknowledged_at, resolved_at) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, now() + ($6 || ' minutes')::interval, now() + ($7 || ' minutes')::interval, now() + ($8 || ' minutes')::interval) RETURNING id::text AS id",
                &[
                    rule_id,
                    project_id,
                    status,
                    message,
                    condition_snapshot_json,
                    &triggered,
                    &acknowledged,
                    &resolved,
                ],
            )
            .remove("id")
            .expect("alert id missing")
        }
        (Some(acknowledged), None) => {
            let acknowledged = acknowledged.to_string();
            query_single_row(
                database_url,
                "INSERT INTO alerts (rule_id, project_id, status, message, condition_snapshot, triggered_at, acknowledged_at) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, now() + ($6 || ' minutes')::interval, now() + ($7 || ' minutes')::interval) RETURNING id::text AS id",
                &[
                    rule_id,
                    project_id,
                    status,
                    message,
                    condition_snapshot_json,
                    &triggered,
                    &acknowledged,
                ],
            )
            .remove("id")
            .expect("alert id missing")
        }
        (None, Some(resolved)) => {
            let resolved = resolved.to_string();
            query_single_row(
                database_url,
                "INSERT INTO alerts (rule_id, project_id, status, message, condition_snapshot, triggered_at, resolved_at) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, now() + ($6 || ' minutes')::interval, now() + ($7 || ' minutes')::interval) RETURNING id::text AS id",
                &[
                    rule_id,
                    project_id,
                    status,
                    message,
                    condition_snapshot_json,
                    &triggered,
                    &resolved,
                ],
            )
            .remove("id")
            .expect("alert id missing")
        }
        (None, None) => query_single_row(
            database_url,
            "INSERT INTO alerts (rule_id, project_id, status, message, condition_snapshot, triggered_at) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, now() + ($6 || ' minutes')::interval) RETURNING id::text AS id",
            &[
                rule_id,
                project_id,
                status,
                message,
                condition_snapshot_json,
                &triggered,
            ],
        )
        .remove("id")
        .expect("alert id missing"),
    }
}

fn ensure_mesh_rt_staticlib() {
    static BUILD_ONCE: OnceLock<()> = OnceLock::new();
    BUILD_ONCE.get_or_init(|| {
        let output = Command::new("cargo")
            .current_dir(repo_root())
            .args(["build", "-p", "mesh-rt"])
            .output()
            .expect("failed to invoke cargo build -p mesh-rt");
        assert_command_success(&output, "cargo build -p mesh-rt");
    });
}

fn copy_mpl_tree(src: &Path, dst: &Path) {
    if !src.exists() {
        panic!("source tree missing: {}", src.display());
    }
    fs::create_dir_all(dst).unwrap_or_else(|e| {
        panic!(
            "failed to create destination tree {}: {}",
            dst.display(),
            e
        )
    });

    for entry in fs::read_dir(src).unwrap_or_else(|e| panic!("failed to read {}: {}", src.display(), e)) {
        let entry = entry.unwrap_or_else(|e| panic!("failed to read dir entry in {}: {}", src.display(), e));
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_mpl_tree(&path, &target);
        } else if path.extension().is_some_and(|ext| ext == "mpl") {
            fs::copy(&path, &target).unwrap_or_else(|e| {
                panic!(
                    "failed to copy {} -> {}: {}",
                    path.display(),
                    target.display(),
                    e
                )
            });
        }
    }
}

fn render_mesh_template(template: &str, replacements: &[(&str, String)]) -> String {
    let mut rendered = template.to_string();
    for (needle, value) in replacements {
        rendered = rendered.replace(needle, value);
    }
    rendered
}

fn mesh_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("failed to encode mesh string literal")
}

fn compile_and_run_mesher_storage_probe(main_source: &str) -> String {
    ensure_mesh_rt_staticlib();

    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    fs::create_dir_all(&project_dir).expect("failed to create project dir");

    copy_mpl_tree(
        &repo_root().join("mesher").join("storage"),
        &project_dir.join("storage"),
    );
    copy_mpl_tree(
        &repo_root().join("mesher").join("types"),
        &project_dir.join("types"),
    );
    fs::write(project_dir.join("main.mpl"), main_source).expect("failed to write main.mpl");

    let meshc = find_meshc();
    let build_output = Command::new(&meshc)
        .current_dir(repo_root())
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc build");
    assert!(
        build_output.status.success(),
        "meshc build failed for Mesher storage probe:\n{}",
        command_output_text(&build_output)
    );

    let binary = project_dir.join("project");
    let run_output = Command::new(&binary)
        .current_dir(&project_dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {}", binary.display(), e));
    assert!(
        run_output.status.success(),
        "Mesher storage probe failed with exit code {:?}:\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    String::from_utf8_lossy(&run_output.stdout).to_string()
}

fn parse_output_map(output: &str) -> OutputMap {
    output
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn rows_signature(rows: &[DbRow], columns: &[&str]) -> String {
    rows.iter()
        .map(|row| {
            columns
                .iter()
                .map(|column| row.get(*column).cloned().unwrap_or_default())
                .collect::<Vec<_>>()
                .join("~")
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn e2e_m033_s03_basic_reads_issue_helpers() {
    with_mesher_postgres("basic-issues", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");

        let default_project_id = default_project_id(MESHER_DATABASE_URL);
        let unresolved_issue = insert_issue(
            MESHER_DATABASE_URL,
            &default_project_id,
            "fp-s03-basic-unresolved",
            "Unresolved issue",
            "error",
        );
        let resolved_issue = insert_issue(
            MESHER_DATABASE_URL,
            &default_project_id,
            "fp-s03-basic-resolved",
            "Resolved issue",
            "warning",
        );
        execute_database_sql(
            MESHER_DATABASE_URL,
            "UPDATE issues SET status = 'resolved' WHERE id = $1::uuid",
            &[&resolved_issue],
        );

        let other_project_id = insert_org_and_project(MESHER_DATABASE_URL, "m033-s03-basic-issues-alt");
        insert_issue(
            MESHER_DATABASE_URL,
            &other_project_id,
            "fp-s03-basic-other",
            "Other project issue",
            "error",
        );

        let template = r##"
from Storage.Queries import count_unresolved_issues, get_issue_project_id

fn bool_text(value :: Bool) -> String do
  if value do
    "true"
  else
    "false"
  end
end

fn main() do
  let pool_result = Pool.open("postgres://mesh:mesh@127.0.0.1:5432/mesher", 1, 1, 5000)
  case pool_result do
    Err( e) -> println("pool_err=#{e}")
    Ok( pool) -> do
      case count_unresolved_issues(pool, __PROJECT_ID__) do
        Err( e) -> println("count_err=#{e}")
        Ok( rows) -> do
          println("count_rows=#{List.length(rows)}")
          let row = List.get(rows, 0)
          println("count_has_cnt=#{bool_text(Map.has_key(row, "cnt"))}")
          println("count_value=#{Map.get(row, "cnt")}")
        end
      end
      case get_issue_project_id(pool, __ISSUE_ID__) do
        Err( e) -> println("project_err=#{e}")
        Ok( rows) -> do
          println("project_rows=#{List.length(rows)}")
          let row = List.get(rows, 0)
          println("project_has_project_id=#{bool_text(Map.has_key(row, "project_id"))}")
          println("project_value=#{Map.get(row, "project_id")}")
        end
      end
    end
  end
end
"##;
        let source = render_mesh_template(
            template,
            &[
                ("__PROJECT_ID__", mesh_string_literal(&default_project_id)),
                ("__ISSUE_ID__", mesh_string_literal(&unresolved_issue)),
            ],
        );

        let output = compile_and_run_mesher_storage_probe(&source);
        let values = parse_output_map(&output);
        assert_eq!(
            values.get("count_rows").map(String::as_str),
            Some("1"),
            "e2e_m033_s03_basic_reads_issue_helpers should return one aggregate row:\n{output}"
        );
        assert_eq!(
            values.get("count_has_cnt").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_issue_helpers cnt key drifted:\n{output}"
        );
        assert_eq!(
            values.get("count_value").map(String::as_str),
            Some("1"),
            "e2e_m033_s03_basic_reads_issue_helpers unresolved count drifted:\n{output}"
        );
        assert_eq!(
            values.get("project_rows").map(String::as_str),
            Some("1"),
            "e2e_m033_s03_basic_reads_issue_helpers project lookup should return one row:\n{output}"
        );
        assert_eq!(
            values.get("project_has_project_id").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_issue_helpers project_id key drifted:\n{output}"
        );
        assert_eq!(
            values.get("project_value").map(String::as_str),
            Some(default_project_id.as_str()),
            "e2e_m033_s03_basic_reads_issue_helpers project_id value drifted:\n{output}"
        );

        let issue_rows = query_database_rows(
            MESHER_DATABASE_URL,
            "SELECT project_id::text AS project_id, status FROM issues ORDER BY project_id::text, fingerprint",
            &[],
        );
        let unresolved_default = issue_rows
            .iter()
            .filter(|row| {
                row.get("project_id").map(String::as_str) == Some(default_project_id.as_str())
                    && row.get("status").map(String::as_str) == Some("unresolved")
            })
            .count();
        let resolved_default = issue_rows
            .iter()
            .filter(|row| {
                row.get("project_id").map(String::as_str) == Some(default_project_id.as_str())
                    && row.get("status").map(String::as_str) == Some("resolved")
            })
            .count();
        assert_eq!(unresolved_default, 1, "default project unresolved issue count drifted");
        assert_eq!(resolved_default, 1, "default project resolved issue count drifted");
    });
}

#[test]
fn e2e_m033_s03_basic_reads_session_and_project_helpers() {
    with_mesher_postgres("basic-session-project", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");
        ensure_today_event_partition(MESHER_DATABASE_URL);

        let project_id = insert_org_and_project(MESHER_DATABASE_URL, "m033-s03-basic-project");
        execute_database_sql(
            MESHER_DATABASE_URL,
            "UPDATE projects SET retention_days = 14, sample_rate = 0.25 WHERE id = $1::uuid",
            &[&project_id],
        );

        let issue_id = insert_issue(
            MESHER_DATABASE_URL,
            &project_id,
            "fp-s03-storage",
            "Storage issue",
            "error",
        );
        insert_seed_event(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_id,
            "error",
            "storage event one",
            "fp-s03-storage",
            r#"{"env":"prod"}"#,
        );
        insert_seed_event(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_id,
            "warning",
            "storage event two",
            "fp-s03-storage",
            r#"{"env":"prod"}"#,
        );

        let user_id = insert_user(
            MESHER_DATABASE_URL,
            "m033-s03-session@example.com",
            "mesh-password-42",
            "M033 Session",
        );
        let valid_token = "m033_s03_valid_session_token";
        let expired_token = "m033_s03_expired_session_token";
        insert_session_with_offset(MESHER_DATABASE_URL, valid_token, &user_id, 24);
        insert_session_with_offset(MESHER_DATABASE_URL, expired_token, &user_id, -24);

        let template = r##"
from Storage.Queries import validate_session, get_all_project_retention, get_project_settings, get_project_storage
from Types.User import Session

fn bool_text(value :: Bool) -> String do
  if value do
    "true"
  else
    "false"
  end
end

fn has_retention_row(rows, target_id :: String, target_days :: String, i :: Int, total :: Int) -> Bool do
  if i < total do
    let row = List.get(rows, i)
    if Map.get(row, "id") == target_id do
      Map.get(row, "retention_days") == target_days
    else
      has_retention_row(rows, target_id, target_days, i + 1, total)
    end
  else
    false
  end
end

fn main() do
  let pool_result = Pool.open("postgres://mesh:mesh@127.0.0.1:5432/mesher", 1, 1, 5000)
  case pool_result do
    Err( e) -> println("pool_err=#{e}")
    Ok( pool) -> do
      case validate_session(pool, __VALID_TOKEN__) do
        Err( e) -> println("session_err=#{e}")
        Ok( session) -> do
          println("session_valid=true")
          println("session_token_matches=#{bool_text(session.token == __VALID_TOKEN__)}")
          println("session_user_id=#{session.user_id}")
          println("session_created_present=#{bool_text(String.length(session.created_at) > 0)}")
          println("session_expires_present=#{bool_text(String.length(session.expires_at) > 0)}")
        end
      end
      case validate_session(pool, __EXPIRED_TOKEN__) do
        Ok( _) -> println("expired_status=unexpected_ok")
        Err( e) -> println("expired_status=#{e}")
      end
      case get_all_project_retention(pool) do
        Err( e) -> println("retention_err=#{e}")
        Ok( rows) -> do
          println("retention_count=#{List.length(rows)}")
          println("retention_has_target=#{bool_text(has_retention_row(rows, __PROJECT_ID__, "14", 0, List.length(rows)))}")
        end
      end
      case get_project_settings(pool, __PROJECT_ID__) do
        Err( e) -> println("settings_err=#{e}")
        Ok( rows) -> do
          let row = List.get(rows, 0)
          println("settings_has_retention_days=#{bool_text(Map.has_key(row, "retention_days"))}")
          println("settings_retention_days=#{Map.get(row, "retention_days")}")
          println("settings_has_sample_rate=#{bool_text(Map.has_key(row, "sample_rate"))}")
          println("settings_sample_rate=#{Map.get(row, "sample_rate")}")
        end
      end
      case get_project_storage(pool, __PROJECT_ID__) do
        Err( e) -> println("storage_err=#{e}")
        Ok( rows) -> do
          let row = List.get(rows, 0)
          println("storage_has_event_count=#{bool_text(Map.has_key(row, "event_count"))}")
          println("storage_event_count=#{Map.get(row, "event_count")}")
          println("storage_has_estimated_bytes=#{bool_text(Map.has_key(row, "estimated_bytes"))}")
          println("storage_estimated_bytes=#{Map.get(row, "estimated_bytes")}")
        end
      end
    end
  end
end
"##;
        let source = render_mesh_template(
            template,
            &[
                ("__VALID_TOKEN__", mesh_string_literal(valid_token)),
                ("__EXPIRED_TOKEN__", mesh_string_literal(expired_token)),
                ("__PROJECT_ID__", mesh_string_literal(&project_id)),
            ],
        );

        let output = compile_and_run_mesher_storage_probe(&source);
        let values = parse_output_map(&output);
        assert_eq!(
            values.get("session_valid").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers valid session lookup failed:\n{output}"
        );
        assert_eq!(
            values.get("session_token_matches").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers token field drifted:\n{output}"
        );
        assert_eq!(
            values.get("session_user_id").map(String::as_str),
            Some(user_id.as_str()),
            "e2e_m033_s03_basic_reads_session_and_project_helpers user_id drifted:\n{output}"
        );
        assert_eq!(
            values.get("session_created_present").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers created_at missing:\n{output}"
        );
        assert_eq!(
            values.get("session_expires_present").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers expires_at missing:\n{output}"
        );
        assert_eq!(
            values.get("expired_status").map(String::as_str),
            Some("not found"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers expired session should be rejected:\n{output}"
        );
        assert_eq!(
            values.get("retention_has_target").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers retention row drifted:\n{output}"
        );
        assert_eq!(
            values.get("settings_has_retention_days").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers retention_days key drifted:\n{output}"
        );
        assert_eq!(
            values.get("settings_retention_days").map(String::as_str),
            Some("14"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers retention_days value drifted:\n{output}"
        );
        assert_eq!(
            values.get("settings_has_sample_rate").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers sample_rate key drifted:\n{output}"
        );
        assert_eq!(
            values.get("settings_sample_rate").map(String::as_str),
            Some("0.25"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers sample_rate value drifted:\n{output}"
        );
        assert_eq!(
            values.get("storage_has_event_count").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers event_count key drifted:\n{output}"
        );
        assert_eq!(
            values.get("storage_event_count").map(String::as_str),
            Some("2"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers event_count value drifted:\n{output}"
        );
        assert_eq!(
            values.get("storage_has_estimated_bytes").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers estimated_bytes key drifted:\n{output}"
        );
        assert_eq!(
            values.get("storage_estimated_bytes").map(String::as_str),
            Some("2048"),
            "e2e_m033_s03_basic_reads_session_and_project_helpers estimated_bytes value drifted:\n{output}"
        );

        let project_row = query_single_row(
            MESHER_DATABASE_URL,
            "SELECT retention_days::text AS retention_days, sample_rate::text AS sample_rate FROM projects WHERE id = $1::uuid",
            &[&project_id],
        );
        assert_eq!(
            project_row.get("retention_days").map(String::as_str),
            Some("14"),
            "project retention_days seed drifted"
        );
        assert_eq!(
            project_row.get("sample_rate").map(String::as_str),
            Some("0.25"),
            "project sample_rate seed drifted"
        );

        let storage_row = query_single_row(
            MESHER_DATABASE_URL,
            "SELECT count(*)::text AS event_count, (count(*) * 1024)::text AS estimated_bytes FROM events WHERE project_id = $1::uuid",
            &[&project_id],
        );
        assert_eq!(
            storage_row.get("event_count").map(String::as_str),
            Some("2"),
            "project storage event_count seed drifted"
        );
        assert_eq!(
            storage_row.get("estimated_bytes").map(String::as_str),
            Some("2048"),
            "project storage estimated_bytes seed drifted"
        );

        let session_row = query_single_row(
            MESHER_DATABASE_URL,
            "SELECT user_id::text AS user_id, (expires_at > now())::text AS is_active FROM sessions WHERE token = $1",
            &[valid_token],
        );
        assert_eq!(
            session_row.get("user_id").map(String::as_str),
            Some(user_id.as_str()),
            "session user_id seed drifted"
        );
        assert_eq!(
            session_row.get("is_active").map(String::as_str),
            Some("true"),
            "valid session should remain active in the database"
        );
    });
}

#[test]
fn e2e_m033_s03_basic_reads_api_key_and_alert_rule_lists() {
    with_mesher_postgres("basic-api-alerts", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");

        let project_id = default_project_id(MESHER_DATABASE_URL);
        insert_api_key_row(
            MESHER_DATABASE_URL,
            &project_id,
            "mshr_s03_revoked_key_0000000000000000000000000",
            "revoked-key",
            -20,
            Some(-10),
        );
        insert_api_key_row(
            MESHER_DATABASE_URL,
            &project_id,
            "mshr_s03_active_key_00000000000000000000000000",
            "active-key",
            -5,
            None,
        );

        insert_alert_rule_row(
            MESHER_DATABASE_URL,
            &project_id,
            "Threshold rule",
            r#"{"condition_type":"threshold","threshold":"5","window_minutes":"10"}"#,
            r#"{"type":"email"}"#,
            true,
            15,
            Some(-15),
            -20,
        );
        insert_alert_rule_row(
            MESHER_DATABASE_URL,
            &project_id,
            "New issue rule",
            r#"{"condition_type":"new_issue"}"#,
            r#"{"type":"websocket"}"#,
            true,
            60,
            None,
            -5,
        );

        let template = r##"
from Storage.Queries import list_api_keys, list_alert_rules

fn bool_text(value :: Bool) -> String do
  if value do
    "true"
  else
    "false"
  end
end

fn api_key_row_matches(rows, label :: String, expect_revoked :: Bool, i :: Int, total :: Int) -> Bool do
  if i < total do
    let row = List.get(rows, i)
    if Map.get(row, "label") == label do
      if Map.has_key(row, "id") do
        if Map.has_key(row, "project_id") do
          if Map.has_key(row, "key_value") do
            if Map.has_key(row, "created_at") do
              if Map.has_key(row, "revoked_at") do
                if expect_revoked do
                  String.length(Map.get(row, "revoked_at")) > 0
                else
                  String.length(Map.get(row, "revoked_at")) == 0
                end
              else
                false
              end
            else
              false
            end
          else
            false
          end
        else
          false
        end
      else
        false
      end
    else
      api_key_row_matches(rows, label, expect_revoked, i + 1, total)
    end
  else
    false
  end
end

fn alert_rule_row_matches(rows, name :: String, expect_last_fired :: Bool, cooldown :: String, i :: Int, total :: Int) -> Bool do
  if i < total do
    let row = List.get(rows, i)
    if Map.get(row, "name") == name do
      if Map.has_key(row, "id") do
        if Map.has_key(row, "project_id") do
          if Map.has_key(row, "condition_json") do
            if Map.has_key(row, "action_json") do
              if Map.has_key(row, "enabled") do
                if Map.has_key(row, "cooldown_minutes") do
                  if Map.has_key(row, "last_fired_at") do
                    if Map.has_key(row, "created_at") do
                      if Map.get(row, "cooldown_minutes") == cooldown do
                        if expect_last_fired do
                          String.length(Map.get(row, "last_fired_at")) > 0
                        else
                          String.length(Map.get(row, "last_fired_at")) == 0
                        end
                      else
                        false
                      end
                    else
                      false
                    end
                  else
                    false
                  end
                else
                  false
                end
              else
                false
              end
            else
              false
            end
          else
            false
          end
        else
          false
        end
      else
        false
      end
    else
      alert_rule_row_matches(rows, name, expect_last_fired, cooldown, i + 1, total)
    end
  else
    false
  end
end

fn main() do
  let pool_result = Pool.open("postgres://mesh:mesh@127.0.0.1:5432/mesher", 1, 1, 5000)
  case pool_result do
    Err( e) -> println("pool_err=#{e}")
    Ok( pool) -> do
      case list_api_keys(pool, __PROJECT_ID__) do
        Err( e) -> println("api_keys_err=#{e}")
        Ok( rows) -> do
          println("api_key_count=#{List.length(rows)}")
          println("api_key_active_match=#{bool_text(api_key_row_matches(rows, "active-key", false, 0, List.length(rows)))}")
          println("api_key_revoked_match=#{bool_text(api_key_row_matches(rows, "revoked-key", true, 0, List.length(rows)))}")
        end
      end
      case list_alert_rules(pool, __PROJECT_ID__) do
        Err( e) -> println("alert_rules_err=#{e}")
        Ok( rows) -> do
          println("alert_rule_count=#{List.length(rows)}")
          println("alert_rule_new_issue_match=#{bool_text(alert_rule_row_matches(rows, "New issue rule", false, "60", 0, List.length(rows)))}")
          println("alert_rule_threshold_match=#{bool_text(alert_rule_row_matches(rows, "Threshold rule", true, "15", 0, List.length(rows)))}")
        end
      end
    end
  end
end
"##;
        let source = render_mesh_template(
            template,
            &[("__PROJECT_ID__", mesh_string_literal(&project_id))],
        );

        let output = compile_and_run_mesher_storage_probe(&source);
        let values = parse_output_map(&output);
        assert_eq!(
            values.get("api_key_count").map(String::as_str),
            Some("3"),
            "e2e_m033_s03_basic_reads_api_key_and_alert_rule_lists api key count drifted:\n{output}"
        );
        assert_eq!(
            values.get("api_key_active_match").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_api_key_and_alert_rule_lists active api key row drifted:\n{output}"
        );
        assert_eq!(
            values.get("api_key_revoked_match").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_api_key_and_alert_rule_lists revoked api key row drifted:\n{output}"
        );
        assert_eq!(
            values.get("alert_rule_count").map(String::as_str),
            Some("2"),
            "e2e_m033_s03_basic_reads_api_key_and_alert_rule_lists alert rule count drifted:\n{output}"
        );
        assert_eq!(
            values.get("alert_rule_new_issue_match").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_api_key_and_alert_rule_lists new issue alert row drifted:\n{output}"
        );
        assert_eq!(
            values.get("alert_rule_threshold_match").map(String::as_str),
            Some("true"),
            "e2e_m033_s03_basic_reads_api_key_and_alert_rule_lists threshold alert row drifted:\n{output}"
        );

        let api_key_rows = query_database_rows(
            MESHER_DATABASE_URL,
            "SELECT label, (revoked_at IS NULL)::text AS is_active FROM api_keys WHERE project_id = $1::uuid ORDER BY created_at DESC",
            &[&project_id],
        );
        assert!(
            api_key_rows.iter().any(|row| {
                row.get("label").map(String::as_str) == Some("active-key")
                    && row.get("is_active").map(String::as_str) == Some("true")
            }),
            "expected an active api key row"
        );
        assert!(
            api_key_rows.iter().any(|row| {
                row.get("label").map(String::as_str) == Some("revoked-key")
                    && row.get("is_active").map(String::as_str) == Some("false")
            }),
            "expected a revoked api key row"
        );

        let alert_rule_rows = query_database_rows(
            MESHER_DATABASE_URL,
            "SELECT name, cooldown_minutes::text AS cooldown_minutes, (last_fired_at IS NULL)::text AS last_fired_missing FROM alert_rules WHERE project_id = $1::uuid ORDER BY created_at DESC",
            &[&project_id],
        );
        assert!(
            alert_rule_rows.iter().any(|row| {
                row.get("name").map(String::as_str) == Some("New issue rule")
                    && row.get("cooldown_minutes").map(String::as_str) == Some("60")
                    && row.get("last_fired_missing").map(String::as_str) == Some("true")
            }),
            "expected the unfired new issue alert rule row"
        );
        assert!(
            alert_rule_rows.iter().any(|row| {
                row.get("name").map(String::as_str) == Some("Threshold rule")
                    && row.get("cooldown_minutes").map(String::as_str) == Some("15")
                    && row.get("last_fired_missing").map(String::as_str) == Some("false")
            }),
            "expected the fired threshold alert rule row"
        );
    });
}


#[test]
fn e2e_m033_s03_composed_reads_joined_issue_and_team_rows() {
    with_mesher_postgres("composed-joined-team", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");

        let auth_project_id = insert_org_and_project(MESHER_DATABASE_URL, "m033-s03-composed-auth");
        let active_key = "mshr_s03_composed_active_key_0000000000000000000001";
        let revoked_key = "mshr_s03_composed_revoked_key_000000000000000000001";
        insert_api_key_row(
            MESHER_DATABASE_URL,
            &auth_project_id,
            active_key,
            "active-composed",
            -15,
            None,
        );
        insert_api_key_row(
            MESHER_DATABASE_URL,
            &auth_project_id,
            revoked_key,
            "revoked-composed",
            -60,
            Some(-30),
        );

        let project_id = insert_org_and_project(MESHER_DATABASE_URL, "m033-s03-composed-joined");
        let org_id = project_org_id(MESHER_DATABASE_URL, &project_id);
        let member_user = insert_user(
            MESHER_DATABASE_URL,
            "m033-s03-member@example.com",
            "mesh-password-42",
            "Member Example",
        );
        let owner_user = insert_user(
            MESHER_DATABASE_URL,
            "m033-s03-owner@example.com",
            "mesh-password-42",
            "Owner Example",
        );
        insert_org_membership_row(MESHER_DATABASE_URL, &owner_user, &org_id, "owner", -90);
        insert_org_membership_row(MESHER_DATABASE_URL, &member_user, &org_id, "member", -10);

        let recent_issue = insert_issue(
            MESHER_DATABASE_URL,
            &project_id,
            "fp-s03-composed-recent",
            "Recent unresolved issue",
            "error",
        );
        update_issue_read_fields(
            MESHER_DATABASE_URL,
            &recent_issue,
            "unresolved",
            7,
            -180,
            -5,
            Some(&member_user),
        );

        let older_issue = insert_issue(
            MESHER_DATABASE_URL,
            &project_id,
            "fp-s03-composed-older",
            "Older unresolved issue",
            "warning",
        );
        update_issue_read_fields(
            MESHER_DATABASE_URL,
            &older_issue,
            "unresolved",
            2,
            -240,
            -45,
            None,
        );

        let resolved_issue = insert_issue(
            MESHER_DATABASE_URL,
            &project_id,
            "fp-s03-composed-resolved",
            "Resolved issue",
            "error",
        );
        update_issue_read_fields(
            MESHER_DATABASE_URL,
            &resolved_issue,
            "resolved",
            11,
            -300,
            -1,
            None,
        );

        let config = mesher_test_config();
        let spawned = spawn_mesher(config);
        let result = std::panic::catch_unwind(|| {
            wait_for_mesher(&config);

            let active_response = post_json_with_headers(
                &config,
                "/api/v1/events",
                r#"{"message":"S03 auth proof accepted","level":"error"}"#,
                &[("x-sentry-auth", active_key)],
                202,
            );
            assert_eq!(active_response["status"].as_str(), Some("accepted"));
            wait_for_query_value(
                MESHER_DATABASE_URL,
                "SELECT count(*)::text AS cnt FROM events WHERE project_id = $1::uuid",
                &[&auth_project_id],
                "cnt",
                "1",
                "authorized ingest event",
            );

            let revoked_response = post_json_with_headers(
                &config,
                "/api/v1/events",
                r#"{"message":"S03 auth proof rejected","level":"error"}"#,
                &[("x-sentry-auth", revoked_key)],
                401,
            );
            assert_eq!(revoked_response["error"].as_str(), Some("unauthorized"));
            let auth_event_count = query_single_row(
                MESHER_DATABASE_URL,
                "SELECT count(*)::text AS cnt FROM events WHERE project_id = $1::uuid",
                &[&auth_project_id],
            );
            assert_eq!(auth_event_count.get("cnt").map(String::as_str), Some("1"));

            let issues_path = format!("/api/v1/projects/{project_id}/issues?status=unresolved");
            let issues_json = get_json(&config, &issues_path, 200);
            assert_eq!(issues_json["has_more"].as_bool(), Some(false));
            let issue_items = issues_json["data"]
                .as_array()
                .expect("issues response should expose a data array");
            let issue_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT id::text AS id, title, level, status, event_count::text AS event_count, first_seen::text AS first_seen, last_seen::text AS last_seen, COALESCE(assigned_to::text, '') AS assigned_to FROM issues WHERE project_id = $1::uuid AND status = 'unresolved' ORDER BY last_seen DESC, id DESC LIMIT 25",
                &[&project_id],
            );
            let expected_issue_signature = rows_signature(
                &issue_rows,
                &[
                    "id",
                    "title",
                    "level",
                    "status",
                    "event_count",
                    "first_seen",
                    "last_seen",
                    "assigned_to",
                ],
            );
            assert_eq!(
                json_array_signature(issue_items, &["id", "title", "level", "status", "event_count", "first_seen", "last_seen", "assigned_to"]),
                expected_issue_signature,
                "e2e_m033_s03_composed_reads_joined_issue_and_team_rows unresolved issue contract drifted"
            );

            let members_json = get_json(&config, &format!("/api/v1/orgs/{org_id}/members"), 200);
            let member_items = members_json
                .as_array()
                .expect("members response should be a JSON array");
            let member_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT org_memberships.id::text AS id, org_memberships.user_id::text AS user_id, users.email, users.display_name, org_memberships.role, org_memberships.joined_at::text AS joined_at FROM org_memberships JOIN users ON users.id = org_memberships.user_id WHERE org_memberships.org_id = $1::uuid ORDER BY org_memberships.joined_at ASC",
                &[&org_id],
            );
            let expected_member_signature = rows_signature(
                &member_rows,
                &["id", "user_id", "email", "display_name", "role", "joined_at"],
            );
            assert_eq!(
                json_array_signature(
                    member_items,
                    &["id", "user_id", "email", "display_name", "role", "joined_at"],
                ),
                expected_member_signature,
                "e2e_m033_s03_composed_reads_joined_issue_and_team_rows team member contract drifted"
            );
        });
        let logs = stop_mesher(spawned);
        match result {
            Ok(()) => assert_mesher_logs(&logs, &config),
            Err(payload) => panic!(
                "M033/S03 joined/team assertions failed: {}\nstdout:\n{}\nstderr:\n{}",
                panic_payload_to_string(payload),
                logs.stdout,
                logs.stderr,
            ),
        }
    });
}

#[test]
fn e2e_m033_s03_composed_reads_dashboard_aggregates() {
    with_mesher_postgres("composed-dashboard", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");
        ensure_today_event_partition(MESHER_DATABASE_URL);

        let project_id = insert_org_and_project(MESHER_DATABASE_URL, "m033-s03-composed-dashboard");

        let issue_alpha = insert_issue(
            MESHER_DATABASE_URL,
            &project_id,
            "fp-s03-dashboard-alpha",
            "Alpha issue",
            "error",
        );
        update_issue_read_fields(
            MESHER_DATABASE_URL,
            &issue_alpha,
            "unresolved",
            9,
            -240,
            -5,
            None,
        );

        let issue_beta = insert_issue(
            MESHER_DATABASE_URL,
            &project_id,
            "fp-s03-dashboard-beta",
            "Beta issue",
            "warning",
        );
        update_issue_read_fields(
            MESHER_DATABASE_URL,
            &issue_beta,
            "unresolved",
            4,
            -200,
            -15,
            None,
        );

        let issue_gamma = insert_issue(
            MESHER_DATABASE_URL,
            &project_id,
            "fp-s03-dashboard-gamma",
            "Gamma resolved issue",
            "error",
        );
        update_issue_read_fields(
            MESHER_DATABASE_URL,
            &issue_gamma,
            "resolved",
            50,
            -300,
            -1,
            None,
        );

        insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_alpha,
            "error",
            "alpha error one",
            "fp-s03-dashboard-alpha",
            None,
            None,
            None,
            r#"{"env":"prod"}"#,
            r#"{}"#,
            None,
            None,
            None,
            -180,
        );
        insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_alpha,
            "error",
            "alpha error two",
            "fp-s03-dashboard-alpha",
            None,
            None,
            None,
            r#"{"env":"prod"}"#,
            r#"{}"#,
            None,
            None,
            None,
            -120,
        );
        insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_alpha,
            "error",
            "alpha error three",
            "fp-s03-dashboard-alpha",
            None,
            None,
            None,
            r#"{"env":"prod"}"#,
            r#"{}"#,
            None,
            None,
            None,
            -30,
        );
        insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_beta,
            "warning",
            "beta warning one",
            "fp-s03-dashboard-beta",
            None,
            None,
            None,
            r#"{"env":"staging"}"#,
            r#"{}"#,
            None,
            None,
            None,
            -90,
        );
        insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_beta,
            "warning",
            "beta warning two",
            "fp-s03-dashboard-beta",
            None,
            None,
            None,
            r#"{"env":"staging"}"#,
            r#"{}"#,
            None,
            None,
            None,
            -60,
        );
        insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_beta,
            "info",
            "beta info event",
            "fp-s03-dashboard-beta",
            None,
            None,
            None,
            r#"{"env":"prod"}"#,
            r#"{}"#,
            None,
            None,
            None,
            -10,
        );

        let config = mesher_test_config();
        let spawned = spawn_mesher(config);
        let result = std::panic::catch_unwind(|| {
            wait_for_mesher(&config);

            let volume_json = get_json(
                &config,
                &format!("/api/v1/projects/{project_id}/dashboard/volume?bucket=hour"),
                200,
            );
            let volume_items = volume_json
                .as_array()
                .expect("volume response should be a JSON array");
            let volume_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT date_trunc('hour', received_at)::text AS bucket, count(*)::text AS count FROM events WHERE project_id = $1::uuid AND received_at > now() - interval '24 hours' GROUP BY 1 ORDER BY 1 ASC",
                &[&project_id],
            );
            assert_eq!(
                json_array_signature(volume_items, &["bucket", "count"]),
                rows_signature(&volume_rows, &["bucket", "count"]),
                "e2e_m033_s03_composed_reads_dashboard_aggregates volume contract drifted"
            );

            let levels_json = get_json(
                &config,
                &format!("/api/v1/projects/{project_id}/dashboard/levels"),
                200,
            );
            let level_items = levels_json
                .as_array()
                .expect("levels response should be a JSON array");
            let level_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT level, count(*)::text AS count FROM events WHERE project_id = $1::uuid AND received_at > now() - interval '24 hours' GROUP BY 1 ORDER BY count(*) DESC, level ASC",
                &[&project_id],
            );
            assert_eq!(
                json_array_signature(level_items, &["level", "count"]),
                rows_signature(&level_rows, &["level", "count"]),
                "e2e_m033_s03_composed_reads_dashboard_aggregates level contract drifted"
            );

            let top_issues_json = get_json(
                &config,
                &format!("/api/v1/projects/{project_id}/dashboard/top-issues?limit=2"),
                200,
            );
            let top_issue_items = top_issues_json
                .as_array()
                .expect("top-issues response should be a JSON array");
            let top_issue_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT id::text AS id, title, level, status, event_count::text AS event_count, last_seen::text AS last_seen FROM issues WHERE project_id = $1::uuid AND status = 'unresolved' ORDER BY event_count DESC, id ASC LIMIT 2",
                &[&project_id],
            );
            assert_eq!(
                json_array_signature(
                    top_issue_items,
                    &["id", "title", "level", "status", "event_count", "last_seen"],
                ),
                rows_signature(
                    &top_issue_rows,
                    &["id", "title", "level", "status", "event_count", "last_seen"],
                ),
                "e2e_m033_s03_composed_reads_dashboard_aggregates top-issues contract drifted"
            );

            let tags_json = get_json(
                &config,
                &format!("/api/v1/projects/{project_id}/dashboard/tags?key=env"),
                200,
            );
            let tag_items = tags_json
                .as_array()
                .expect("tag breakdown response should be a JSON array");
            let tag_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT jsonb_extract_path_text(tags, 'env') AS value, count(*)::text AS count FROM events WHERE project_id = $1::uuid AND received_at > now() - interval '24 hours' AND jsonb_exists(tags, 'env') GROUP BY 1 ORDER BY count(*) DESC, value ASC LIMIT 20",
                &[&project_id],
            );
            assert_eq!(
                json_array_signature(tag_items, &["value", "count"]),
                rows_signature(&tag_rows, &["value", "count"]),
                "e2e_m033_s03_composed_reads_dashboard_aggregates tag contract drifted"
            );
        });
        let logs = stop_mesher(spawned);
        match result {
            Ok(()) => assert_mesher_logs(&logs, &config),
            Err(payload) => panic!(
                "M033/S03 dashboard assertions failed: {}\nstdout:\n{}\nstderr:\n{}",
                panic_payload_to_string(payload),
                logs.stdout,
                logs.stderr,
            ),
        }
    });
}

#[test]
fn e2e_m033_s03_composed_reads_detail_and_issue_event_lists() {
    with_mesher_postgres("composed-detail-lists", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");
        ensure_today_event_partition(MESHER_DATABASE_URL);

        let project_id = insert_org_and_project(MESHER_DATABASE_URL, "m033-s03-composed-detail");
        let issue_id = insert_issue(
            MESHER_DATABASE_URL,
            &project_id,
            "fp-s03-composed-detail",
            "Detail issue",
            "error",
        );
        update_issue_read_fields(
            MESHER_DATABASE_URL,
            &issue_id,
            "unresolved",
            3,
            -240,
            -5,
            None,
        );

        let oldest_event = insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_id,
            "warning",
            "oldest event",
            "fp-s03-composed-detail",
            None,
            None,
            None,
            r#"{"env":"prod"}"#,
            r#"{}"#,
            None,
            None,
            None,
            -120,
        );
        let detail_event = insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_id,
            "error",
            "detail event",
            "fp-s03-composed-detail",
            None,
            None,
            None,
            r#"{}"#,
            r#"{}"#,
            None,
            None,
            None,
            -60,
        );
        let newest_event = insert_event_row(
            MESHER_DATABASE_URL,
            &project_id,
            &issue_id,
            "info",
            "newest event",
            "fp-s03-composed-detail",
            None,
            None,
            None,
            r#"{"env":"prod"}"#,
            r#"{}"#,
            None,
            Some("mesher-js"),
            Some("1.2.3"),
            -5,
        );

        let config = mesher_test_config();
        let spawned = spawn_mesher(config);
        let result = std::panic::catch_unwind(|| {
            wait_for_mesher(&config);

            let page1_path = format!("/api/v1/issues/{issue_id}/events?limit=2");
            let page1_json = get_json(&config, &page1_path, 200);
            assert_eq!(page1_json["has_more"].as_bool(), Some(true));
            let page1_items = page1_json["data"]
                .as_array()
                .expect("first issue-event page should expose a data array");
            let page1_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT id::text AS id, level, message, received_at::text AS received_at FROM events WHERE issue_id = $1::uuid ORDER BY received_at DESC, id DESC LIMIT 2",
                &[&issue_id],
            );
            assert_eq!(
                json_array_signature(page1_items, &["id", "level", "message", "received_at"]),
                rows_signature(&page1_rows, &["id", "level", "message", "received_at"]),
                "e2e_m033_s03_composed_reads_detail_and_issue_event_lists first page drifted"
            );

            let cursor_received_at = page1_rows[1]
                .get("received_at")
                .cloned()
                .expect("missing page1 cursor received_at");
            let cursor_id = page1_rows[1]
                .get("id")
                .cloned()
                .expect("missing page1 cursor id");
            assert_eq!(page1_json["next_cursor"].as_str(), Some(cursor_received_at.as_str()));
            assert_eq!(page1_json["next_cursor_id"].as_str(), Some(cursor_id.as_str()));

            let page2_path = format!(
                "/api/v1/issues/{issue_id}/events?limit=2&cursor={}&cursor_id={}",
                url_encode_component(&cursor_received_at),
                url_encode_component(&cursor_id),
            );
            let page2_json = get_json(&config, &page2_path, 200);
            assert_eq!(page2_json["has_more"].as_bool(), Some(false));
            let page2_items = page2_json["data"]
                .as_array()
                .expect("second issue-event page should expose a data array");
            let page2_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT id::text AS id, level, message, received_at::text AS received_at FROM events WHERE issue_id = $1::uuid AND (received_at, id) < ($2::timestamptz, $3::uuid) ORDER BY received_at DESC, id DESC LIMIT 2",
                &[&issue_id, &cursor_received_at, &cursor_id],
            );
            assert_eq!(
                json_array_signature(page2_items, &["id", "level", "message", "received_at"]),
                rows_signature(&page2_rows, &["id", "level", "message", "received_at"]),
                "e2e_m033_s03_composed_reads_detail_and_issue_event_lists second page drifted"
            );

            let detail_json = get_json(&config, &format!("/api/v1/events/{detail_event}"), 200);
            let event = &detail_json["event"];
            let detail_row = query_single_row(
                MESHER_DATABASE_URL,
                "SELECT id::text AS id, project_id::text AS project_id, issue_id::text AS issue_id, level, message, fingerprint, COALESCE(exception::text, 'null') AS exception, COALESCE(stacktrace::text, '[]') AS stacktrace, COALESCE(breadcrumbs::text, '[]') AS breadcrumbs, COALESCE(tags::text, '{}') AS tags, COALESCE(extra::text, '{}') AS extra, COALESCE(user_context::text, 'null') AS user_context, COALESCE(sdk_name, '') AS sdk_name, COALESCE(sdk_version, '') AS sdk_version, received_at::text AS received_at FROM events WHERE id = $1::uuid",
                &[&detail_event],
            );
            assert_eq!(event["id"].as_str(), detail_row.get("id").map(String::as_str));
            assert_eq!(
                event["project_id"].as_str(),
                detail_row.get("project_id").map(String::as_str)
            );
            assert_eq!(event["issue_id"].as_str(), detail_row.get("issue_id").map(String::as_str));
            assert_eq!(event["level"].as_str(), detail_row.get("level").map(String::as_str));
            assert_eq!(event["message"].as_str(), detail_row.get("message").map(String::as_str));
            assert_eq!(
                event["fingerprint"].as_str(),
                detail_row.get("fingerprint").map(String::as_str)
            );
            assert_eq!(event["exception"], db_json(&detail_row, "exception"));
            assert_eq!(event["stacktrace"], db_json(&detail_row, "stacktrace"));
            assert_eq!(event["breadcrumbs"], db_json(&detail_row, "breadcrumbs"));
            assert_eq!(event["tags"], db_json(&detail_row, "tags"));
            assert_eq!(event["extra"], db_json(&detail_row, "extra"));
            assert_eq!(event["user_context"], db_json(&detail_row, "user_context"));
            assert_eq!(event["sdk_name"].as_str(), detail_row.get("sdk_name").map(String::as_str));
            assert_eq!(
                event["sdk_version"].as_str(),
                detail_row.get("sdk_version").map(String::as_str)
            );
            assert_eq!(
                event["received_at"].as_str(),
                detail_row.get("received_at").map(String::as_str)
            );

            let navigation = &detail_json["navigation"];
            assert_eq!(navigation["next_id"].as_str(), Some(newest_event.as_str()));
            assert_eq!(navigation["prev_id"].as_str(), Some(oldest_event.as_str()));
        });
        let logs = stop_mesher(spawned);
        match result {
            Ok(()) => assert_mesher_logs(&logs, &config),
            Err(payload) => panic!(
                "M033/S03 detail/list assertions failed: {}\nstdout:\n{}\nstderr:\n{}",
                panic_payload_to_string(payload),
                logs.stdout,
                logs.stderr,
            ),
        }
    });
}

#[test]
fn e2e_m033_s03_composed_reads_alert_lists_and_predicates() {
    with_mesher_postgres("composed-alerts", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");
        ensure_today_event_partition(MESHER_DATABASE_URL);

        let project_id = insert_org_and_project(MESHER_DATABASE_URL, "m033-s03-composed-alerts");
        let active_key = "mshr_s03_alerts_active_key_000000000000000000000001";
        insert_api_key_row(
            MESHER_DATABASE_URL,
            &project_id,
            active_key,
            "alerts-active",
            -5,
            None,
        );

        let fresh_rule = insert_alert_rule_row(
            MESHER_DATABASE_URL,
            &project_id,
            "Fresh new-issue rule",
            r#"{"condition_type":"new_issue"}"#,
            r#"{"type":"websocket"}"#,
            true,
            60,
            None,
            -30,
        );
        let hot_rule = insert_alert_rule_row(
            MESHER_DATABASE_URL,
            &project_id,
            "Hot new-issue rule",
            r#"{"condition_type":"new_issue"}"#,
            r#"{"type":"email"}"#,
            true,
            60,
            Some(-5),
            -10,
        );
        let acknowledged_rule = insert_alert_rule_row(
            MESHER_DATABASE_URL,
            &project_id,
            "Historical acknowledged rule",
            r#"{"condition_type":"threshold","threshold":"5","window_minutes":"10"}"#,
            r#"{"type":"email"}"#,
            true,
            60,
            Some(-120),
            -60,
        );
        let resolved_rule = insert_alert_rule_row(
            MESHER_DATABASE_URL,
            &project_id,
            "Historical resolved rule",
            r#"{"condition_type":"threshold","threshold":"10","window_minutes":"15"}"#,
            r#"{"type":"websocket"}"#,
            true,
            60,
            Some(-180),
            -90,
        );

        insert_alert_row(
            MESHER_DATABASE_URL,
            &acknowledged_rule,
            &project_id,
            "acknowledged",
            "Threshold alert cooled down",
            r#"{"condition_type":"threshold"}"#,
            -20,
            Some(-15),
            None,
        );
        insert_alert_row(
            MESHER_DATABASE_URL,
            &resolved_rule,
            &project_id,
            "resolved",
            "Threshold alert resolved",
            r#"{"condition_type":"threshold"}"#,
            -40,
            Some(-35),
            Some(-30),
        );

        let config = mesher_test_config();
        let spawned = spawn_mesher(config);
        let result = std::panic::catch_unwind(|| {
            wait_for_mesher(&config);

            let first_event = post_json_with_headers(
                &config,
                "/api/v1/events",
                r#"{"message":"S03 new issue alert","level":"error"}"#,
                &[("x-sentry-auth", active_key)],
                202,
            );
            assert_eq!(first_event["status"].as_str(), Some("accepted"));
            wait_for_query_value(
                MESHER_DATABASE_URL,
                "SELECT count(*)::text AS cnt FROM events WHERE project_id = $1::uuid",
                &[&project_id],
                "cnt",
                "1",
                "first alert-triggering event",
            );
            wait_for_query_value(
                MESHER_DATABASE_URL,
                "SELECT count(*)::text AS cnt FROM alerts WHERE project_id = $1::uuid",
                &[&project_id],
                "cnt",
                "3",
                "fresh-rule alert insert",
            );

            let second_event = post_json_with_headers(
                &config,
                "/api/v1/events",
                r#"{"message":"S03 new issue alert","level":"error"}"#,
                &[("x-sentry-auth", active_key)],
                202,
            );
            assert_eq!(second_event["status"].as_str(), Some("accepted"));
            wait_for_query_value(
                MESHER_DATABASE_URL,
                "SELECT count(*)::text AS cnt FROM events WHERE project_id = $1::uuid",
                &[&project_id],
                "cnt",
                "2",
                "second event on existing issue",
            );

            let created_issue = query_single_row(
                MESHER_DATABASE_URL,
                "SELECT event_count::text AS event_count, (first_seen = last_seen)::text AS is_new FROM issues WHERE project_id = $1::uuid AND title = $2",
                &[&project_id, "S03 new issue alert"],
            );
            assert_eq!(
                created_issue.get("event_count").map(String::as_str),
                Some("2"),
                "issue event_count drifted after repeated ingest"
            );
            assert_eq!(
                created_issue.get("is_new").map(String::as_str),
                Some("false"),
                "issue should no longer satisfy the new-issue predicate after the second event"
            );

            let all_alerts_json = get_json(&config, &format!("/api/v1/projects/{project_id}/alerts"), 200);
            let all_alert_items = all_alerts_json
                .as_array()
                .expect("alerts response should be a JSON array");
            let alert_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT alerts.id::text AS id, alerts.rule_id::text AS rule_id, alerts.project_id::text AS project_id, alerts.status, alerts.message, alerts.condition_snapshot::text AS condition_snapshot, alerts.triggered_at::text AS triggered_at, COALESCE(alerts.acknowledged_at::text, '') AS acknowledged_at, COALESCE(alerts.resolved_at::text, '') AS resolved_at, alert_rules.name AS rule_name FROM alerts JOIN alert_rules ON alert_rules.id = alerts.rule_id WHERE alerts.project_id = $1::uuid ORDER BY alerts.triggered_at DESC LIMIT 50",
                &[&project_id],
            );
            assert_eq!(all_alert_items.len(), alert_rows.len());
            for (item, row) in all_alert_items.iter().zip(alert_rows.iter()) {
                assert_eq!(item["id"].as_str(), row.get("id").map(String::as_str));
                assert_eq!(item["rule_id"].as_str(), row.get("rule_id").map(String::as_str));
                assert_eq!(
                    item["project_id"].as_str(),
                    row.get("project_id").map(String::as_str)
                );
                assert_eq!(item["status"].as_str(), row.get("status").map(String::as_str));
                assert_eq!(item["message"].as_str(), row.get("message").map(String::as_str));
                assert_eq!(
                    item["rule_name"].as_str(),
                    row.get("rule_name").map(String::as_str)
                );
                assert_eq!(item["condition_snapshot"], db_json(row, "condition_snapshot"));
                assert_eq!(
                    item["triggered_at"].as_str(),
                    row.get("triggered_at").map(String::as_str)
                );
                assert_nullable_timestamp(
                    &item["acknowledged_at"],
                    row.get("acknowledged_at").map(String::as_str).unwrap_or(""),
                    "acknowledged_at",
                );
                assert_nullable_timestamp(
                    &item["resolved_at"],
                    row.get("resolved_at").map(String::as_str).unwrap_or(""),
                    "resolved_at",
                );
            }

            let active_alerts_json = get_json(
                &config,
                &format!("/api/v1/projects/{project_id}/alerts?status=active"),
                200,
            );
            let active_alert_items = active_alerts_json
                .as_array()
                .expect("active-alerts response should be a JSON array");
            let active_rows = query_database_rows(
                MESHER_DATABASE_URL,
                "SELECT alerts.id::text AS id, alerts.rule_id::text AS rule_id, alerts.project_id::text AS project_id, alerts.status, alerts.message, alerts.condition_snapshot::text AS condition_snapshot, alerts.triggered_at::text AS triggered_at, COALESCE(alerts.acknowledged_at::text, '') AS acknowledged_at, COALESCE(alerts.resolved_at::text, '') AS resolved_at, alert_rules.name AS rule_name FROM alerts JOIN alert_rules ON alert_rules.id = alerts.rule_id WHERE alerts.project_id = $1::uuid AND alerts.status = 'active' ORDER BY alerts.triggered_at DESC LIMIT 50",
                &[&project_id],
            );
            assert_eq!(active_alert_items.len(), active_rows.len());
            assert_eq!(active_alert_items.len(), 1, "only the fresh rule should fire");
            assert_eq!(
                active_alert_items[0]["rule_name"].as_str(),
                Some("Fresh new-issue rule")
            );
            assert_eq!(active_rows[0].get("rule_id").map(String::as_str), Some(fresh_rule.as_str()));

            let fresh_alert_count = query_single_row(
                MESHER_DATABASE_URL,
                "SELECT count(*)::text AS cnt FROM alerts WHERE rule_id = $1::uuid",
                &[&fresh_rule],
            );
            assert_eq!(fresh_alert_count.get("cnt").map(String::as_str), Some("1"));
            let hot_alert_count = query_single_row(
                MESHER_DATABASE_URL,
                "SELECT count(*)::text AS cnt FROM alerts WHERE rule_id = $1::uuid",
                &[&hot_rule],
            );
            assert_eq!(hot_alert_count.get("cnt").map(String::as_str), Some("0"));
        });
        let logs = stop_mesher(spawned);
        match result {
            Ok(()) => assert_mesher_logs(&logs, &config),
            Err(payload) => panic!(
                "M033/S03 alert assertions failed: {}\nstdout:\n{}\nstderr:\n{}",
                panic_payload_to_string(payload),
                logs.stdout,
                logs.stderr,
            ),
        }
    });
}
