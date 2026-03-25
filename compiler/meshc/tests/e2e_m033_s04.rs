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
const POSTGRES_CONTAINER_PREFIX: &str = "mesh-m033-s04-pg";

#[derive(Clone, Copy, Debug)]
struct MesherConfig {
    http_port: u16,
    ws_port: u16,
}

struct PostgresContainer {
    name: String,
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

fn row_text<'a>(row: &'a DbRow, key: &str) -> &'a str {
    row.get(key)
        .map(String::as_str)
        .unwrap_or_else(|| panic!("missing {key} in row {row:?}"))
}

fn row_int(row: &DbRow, key: &str) -> i64 {
    row_text(row, key)
        .parse::<i64>()
        .unwrap_or_else(|e| panic!("failed to parse {key} as i64: {e}; row={row:?}"))
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
        std::thread::sleep(Duration::from_millis(250));
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
    ensure_mesh_rt_staticlib();
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

fn partition_window(database_url: &str, offset_days: i64) -> DbRow {
    let offset = offset_days.to_string();
    query_single_row(
        database_url,
        "SELECT to_char((current_date + ($1 || ' days')::interval)::date, 'YYYYMMDD') AS suffix, ((current_date + ($1 || ' days')::interval)::date)::text AS start_day, (((current_date + ($1 || ' days')::interval)::date) + 1)::text AS end_day",
        &[&offset],
    )
}

fn partition_name_for_offset(database_url: &str, offset_days: i64) -> String {
    let window = partition_window(database_url, offset_days);
    format!("events_{}", row_text(&window, "suffix"))
}

fn create_partition_for_offset(database_url: &str, offset_days: i64) -> String {
    let window = partition_window(database_url, offset_days);
    let partition_name = format!("events_{}", row_text(&window, "suffix"));
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS {partition_name} PARTITION OF events FOR VALUES FROM ('{}') TO ('{}')",
        row_text(&window, "start_day"),
        row_text(&window, "end_day")
    );
    execute_database_sql(database_url, &sql, &[]);
    partition_name
}

fn relation_exists(database_url: &str, relation_name: &str) -> bool {
    let row = query_single_row(
        database_url,
        "SELECT COALESCE(to_regclass($1)::text, '') AS regclass",
        &[relation_name],
    );
    !row_text(&row, "regclass").is_empty()
}

fn count_partitions_in_window(database_url: &str, start_offset_days: i64, end_offset_days: i64) -> i64 {
    let start = start_offset_days.to_string();
    let end = end_offset_days.to_string();
    let row = query_single_row(
        database_url,
        "SELECT count(*)::text AS count FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid JOIN pg_class p ON p.oid = i.inhparent WHERE p.relname = 'events' AND left(c.relname, 7) = 'events_' AND char_length(c.relname) = 15 AND substring(c.relname from 8 for 8) ~ '^[0-9]{8}$' AND to_date(substring(c.relname from 8 for 8), 'YYYYMMDD') BETWEEN ((current_date + ($1 || ' days')::interval)::date) AND ((current_date + ($2 || ' days')::interval)::date)",
        &[&start, &end],
    );
    row_int(&row, "count")
}

fn ensure_mesh_rt_staticlib() {
    static BUILD_ONCE: OnceLock<()> = OnceLock::new();
    BUILD_ONCE.get_or_init(|| {
        let output = Command::new("cargo")
            .current_dir(repo_root())
            .args(["build", "-q", "-p", "mesh-rt"])
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
        ensure_mesh_rt_staticlib();
        let output = build_mesher();
        assert_command_success(&output, "meshc build mesher");
        assert!(mesher_binary().exists(), "mesher binary was not built");
    });
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

fn collect_stopped_mesher(mut child: Child, stdout_path: PathBuf, stderr_path: PathBuf) -> StoppedMesher {
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

#[test]
fn e2e_m033_s04_migrations_render_pg_catalog_state() {
    with_mesher_postgres("catalogs", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");

        let extension = query_single_row(
            MESHER_DATABASE_URL,
            "SELECT extname::text AS extname FROM pg_extension WHERE extname = 'pgcrypto'",
            &[],
        );
        assert_eq!(
            row_text(&extension, "extname"),
            "pgcrypto",
            "pgcrypto extension was not installed by the S04 migration helpers: {extension:?}"
        );

        let partitioned = query_single_row(
            MESHER_DATABASE_URL,
            "SELECT c.relname::text AS table_name, a.attname::text AS column_name FROM pg_partitioned_table pt JOIN pg_class c ON c.oid = pt.partrelid JOIN pg_attribute a ON a.attrelid = pt.partrelid AND a.attnum = pt.partattrs[0] WHERE c.relname = 'events'",
            &[],
        );
        assert_eq!(
            row_text(&partitioned, "table_name"),
            "events",
            "events should remain the partitioned parent table: {partitioned:?}"
        );
        assert_eq!(
            row_text(&partitioned, "column_name"),
            "received_at",
            "events should stay partitioned on received_at: {partitioned:?}"
        );

        let regclass = query_single_row(
            MESHER_DATABASE_URL,
            "SELECT COALESCE(to_regclass('public.events')::text, '') AS regclass",
            &[],
        );
        assert!(
            row_text(&regclass, "regclass").contains("events"),
            "to_regclass(public.events) drifted after the helper-driven migration: {regclass:?}"
        );

        let index = query_single_row(
            MESHER_DATABASE_URL,
            "SELECT lower(indexdef) AS indexdef FROM pg_indexes WHERE schemaname = 'public' AND tablename = 'events' AND indexname = 'idx_events_tags'",
            &[],
        );
        let indexdef = row_text(&index, "indexdef");
        assert!(
            indexdef.contains("using gin"),
            "idx_events_tags should remain a GIN index: {index:?}"
        );
        assert!(
            indexdef.contains("jsonb_path_ops"),
            "idx_events_tags should keep the jsonb_path_ops opclass: {index:?}"
        );

        let access_method = query_single_row(
            MESHER_DATABASE_URL,
            "SELECT am.amname::text AS access_method, opc.opcname::text AS opclass FROM pg_index i JOIN pg_class idx ON idx.oid = i.indexrelid JOIN pg_class tbl ON tbl.oid = i.indrelid JOIN pg_am am ON am.oid = idx.relam JOIN pg_opclass opc ON opc.oid = i.indclass[0] WHERE tbl.relname = 'events' AND idx.relname = 'idx_events_tags'",
            &[],
        );
        assert_eq!(
            row_text(&access_method, "access_method"),
            "gin",
            "idx_events_tags access method drifted: {access_method:?}"
        );
        assert_eq!(
            row_text(&access_method, "opclass"),
            "jsonb_path_ops",
            "idx_events_tags opclass drifted: {access_method:?}"
        );
    });
}

#[test]
fn e2e_m033_s04_storage_schema_helpers_manage_runtime_partitions() {
    with_mesher_postgres("schema-helpers", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");

        let expired_partition_name = create_partition_for_offset(MESHER_DATABASE_URL, -120);
        assert!(
            relation_exists(MESHER_DATABASE_URL, &expired_partition_name),
            "seed expired partition was not created: {expired_partition_name}"
        );

        let template = r##"
from Storage.Schema import create_partitions_ahead, get_expired_partitions, drop_partition

fn append_partition_name(acc :: String, name :: String) -> String do
  if String.length(acc) > 0 do
    acc <> "|" <> name
  else
    name
  end
end

fn join_partitions(partitions, i :: Int, total :: Int, acc :: String) -> String do
  if i < total do
    let name = List.get(partitions, i)
    join_partitions(partitions, i + 1, total, append_partition_name(acc, name))
  else
    acc
  end
end

fn main() do
  let pool_result = Pool.open("postgres://mesh:mesh@127.0.0.1:5432/mesher", 1, 1, 5000)
  case pool_result do
    Err( e) -> println("pool_err=#{e}")
    Ok( pool) -> do
      case create_partitions_ahead(pool, 3) do
        Err( e) -> println("create_err=#{e}")
        Ok( _) -> println("create_ok=true")
      end
      case get_expired_partitions(pool, 90) do
        Err( e) -> println("list_err=#{e}")
        Ok( partitions) -> do
          let total = List.length(partitions)
          println("expired_count=#{String.from(total)}")
          println("expired_names=#{join_partitions(partitions, 0, total, "")}")
          if total > 0 do
            let first = List.get(partitions, 0)
            case drop_partition(pool, first) do
              Err( e) -> println("drop_err=#{e}")
              Ok( _) -> println("dropped=#{first}")
            end
          else
            println("dropped=")
          end
        end
      end
    end
  end
end
"##;

        let output = compile_and_run_mesher_storage_probe(template);
        let values = parse_output_map(&output);
        assert_eq!(
            values.get("create_ok").map(String::as_str),
            Some("true"),
            "Storage.Schema.create_partitions_ahead failed in the live probe:\n{output}"
        );
        assert_eq!(
            values.get("expired_count").map(String::as_str),
            Some("1"),
            "Storage.Schema.get_expired_partitions should return exactly the seeded expired partition:\n{output}"
        );
        assert_eq!(
            values.get("expired_names").map(String::as_str),
            Some(expired_partition_name.as_str()),
            "Storage.Schema.get_expired_partitions drifted from the seeded partition:\n{output}"
        );
        assert_eq!(
            values.get("dropped").map(String::as_str),
            Some(expired_partition_name.as_str()),
            "Storage.Schema.drop_partition did not drop the listed partition:\n{output}"
        );

        for offset in 0..3 {
            let partition_name = partition_name_for_offset(MESHER_DATABASE_URL, offset);
            assert!(
                relation_exists(MESHER_DATABASE_URL, &partition_name),
                "create_partitions_ahead failed to materialize {partition_name}"
            );
        }
        assert_eq!(
            count_partitions_in_window(MESHER_DATABASE_URL, 0, 2),
            3,
            "create_partitions_ahead should create three partitions on the database clock"
        );
        assert!(
            !relation_exists(MESHER_DATABASE_URL, &expired_partition_name),
            "drop_partition left the expired partition behind: {expired_partition_name}"
        );
    });
}

#[test]
fn e2e_m033_s04_mesher_startup_bootstraps_partitions_and_logs() {
    with_mesher_postgres("startup", || {
        let migrate_output = run_mesher_migrations(MESHER_DATABASE_URL);
        assert_command_success(&migrate_output, "meshc migrate mesher up");
        assert_eq!(
            count_partitions_in_window(MESHER_DATABASE_URL, 0, 6),
            0,
            "migrations should not pre-create runtime partitions before Mesher startup"
        );

        let config = mesher_test_config();
        let spawned = spawn_mesher(config);
        let wait_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| wait_for_mesher(&config)));
        let logs = stop_mesher(spawned);
        let settings_json = match wait_result {
            Ok(json) => json,
            Err(payload) => panic!(
                "Mesher never reached ready state: {}\nstdout:\n{}\nstderr:\n{}",
                panic_payload_to_string(payload),
                logs.stdout,
                logs.stderr
            ),
        };

        assert!(
            settings_json["retention_days"].is_number(),
            "Mesher settings endpoint returned an unexpected retention_days payload: {settings_json}"
        );
        assert!(
            settings_json["sample_rate"].is_number(),
            "Mesher settings endpoint returned an unexpected sample_rate payload: {settings_json}"
        );
        assert!(
            logs.combined.contains("[Mesher] Connecting to PostgreSQL..."),
            "Mesher startup logs never showed the Postgres connection banner:\n{}",
            logs.combined
        );
        assert!(
            logs.combined.contains("[Mesher] Partition bootstrap succeeded (7 days ahead)"),
            "Mesher startup logs never showed partition bootstrap success:\n{}",
            logs.combined
        );
        assert!(
            logs.combined
                .contains(&format!("[Mesher] HTTP server starting on :{}", config.http_port)),
            "Mesher startup logs never showed the HTTP listener on :{}:\n{}",
            config.http_port,
            logs.combined
        );
        assert!(
            !logs.combined.contains("postgres://"),
            "Mesher startup logs should not print connection URLs or secrets:\n{}",
            logs.combined
        );
        assert!(
            !logs.combined.contains("DATABASE_URL"),
            "Mesher startup logs should not print DATABASE_URL-related secrets:\n{}",
            logs.combined
        );
        assert_eq!(
            count_partitions_in_window(MESHER_DATABASE_URL, 0, 6),
            7,
            "Mesher startup should bootstrap seven daily partitions ahead"
        );
        let furthest_partition = partition_name_for_offset(MESHER_DATABASE_URL, 6);
        assert!(
            relation_exists(MESHER_DATABASE_URL, &furthest_partition),
            "Mesher startup never created the furthest bootstrap partition: {furthest_partition}"
        );
    });
}
