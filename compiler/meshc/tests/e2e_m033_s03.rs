use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use mesh_rt::db::pg::{native_pg_close, native_pg_connect, native_pg_execute, native_pg_query};

type DbRow = HashMap<String, String>;
type OutputMap = HashMap<String, String>;

const MESHER_DATABASE_URL: &str = "postgres://mesh:mesh@127.0.0.1:5432/mesher";
const POSTGRES_IMAGE: &str = "postgres:16";
const POSTGRES_CONTAINER_PREFIX: &str = "mesh-m033-s03-pg";

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

        let template = r#"
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
          println("count_has_cnt=#{bool_text(Map.has_key(row, \"cnt\"))}")
          println("count_value=#{Map.get(row, \"cnt\")}")
        end
      end
      case get_issue_project_id(pool, __ISSUE_ID__) do
        Err( e) -> println("project_err=#{e}")
        Ok( rows) -> do
          println("project_rows=#{List.length(rows)}")
          let row = List.get(rows, 0)
          println("project_has_project_id=#{bool_text(Map.has_key(row, \"project_id\"))}")
          println("project_value=#{Map.get(row, \"project_id\")}")
        end
      end
    end
  end
end
"#;
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

        let template = r#"
from Storage.Queries import validate_session, get_all_project_retention, get_project_settings, get_project_storage

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
          println("retention_has_target=#{bool_text(has_retention_row(rows, __PROJECT_ID__, \"14\", 0, List.length(rows)))}")
        end
      end
      case get_project_settings(pool, __PROJECT_ID__) do
        Err( e) -> println("settings_err=#{e}")
        Ok( rows) -> do
          let row = List.get(rows, 0)
          println("settings_has_retention_days=#{bool_text(Map.has_key(row, \"retention_days\"))}")
          println("settings_retention_days=#{Map.get(row, \"retention_days\")}")
          println("settings_has_sample_rate=#{bool_text(Map.has_key(row, \"sample_rate\"))}")
          println("settings_sample_rate=#{Map.get(row, \"sample_rate\")}")
        end
      end
      case get_project_storage(pool, __PROJECT_ID__) do
        Err( e) -> println("storage_err=#{e}")
        Ok( rows) -> do
          let row = List.get(rows, 0)
          println("storage_has_event_count=#{bool_text(Map.has_key(row, \"event_count\"))}")
          println("storage_event_count=#{Map.get(row, \"event_count\")}")
          println("storage_has_estimated_bytes=#{bool_text(Map.has_key(row, \"estimated_bytes\"))}")
          println("storage_estimated_bytes=#{Map.get(row, \"estimated_bytes\")}")
        end
      end
    end
  end
end
"#;
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

        let template = r#"
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
          println("api_key_active_match=#{bool_text(api_key_row_matches(rows, \"active-key\", false, 0, List.length(rows)))}")
          println("api_key_revoked_match=#{bool_text(api_key_row_matches(rows, \"revoked-key\", true, 0, List.length(rows)))}")
        end
      end
      case list_alert_rules(pool, __PROJECT_ID__) do
        Err( e) -> println("alert_rules_err=#{e}")
        Ok( rows) -> do
          println("alert_rule_count=#{List.length(rows)}")
          println("alert_rule_new_issue_match=#{bool_text(alert_rule_row_matches(rows, \"New issue rule\", false, \"60\", 0, List.length(rows)))}")
          println("alert_rule_threshold_match=#{bool_text(alert_rule_row_matches(rows, \"Threshold rule\", true, \"15\", 0, List.length(rows)))}")
        end
      end
    end
  end
end
"#;
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
