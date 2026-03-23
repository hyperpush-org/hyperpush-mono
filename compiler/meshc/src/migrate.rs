//! Migration runner and scaffold generation for `meshc migrate`.
//!
//! Provides the following commands:
//! - `meshc migrate up` - Apply all pending migrations
//! - `meshc migrate down` - Rollback the last applied migration
//! - `meshc migrate status` - Show applied vs pending migrations
//! - `meshc migrate generate <name>` - Create a new migration scaffold
//!
//! The runner discovers `.mpl` migration files in `migrations/`, manages the
//! `_mesh_migrations` tracking table in PostgreSQL via direct Rust PG wire
//! protocol, compiles each migration as a synthetic Mesh project, and executes it.

use std::path::Path;

use mesh_rt::db::pg::{native_pg_close, native_pg_connect, native_pg_execute, native_pg_query};
use mesh_typeck::diagnostics::DiagnosticOptions;

// ── Migration Info ──────────────────────────────────────────────────────

/// Metadata about a discovered migration file.
struct MigrationInfo {
    /// Numeric version extracted from filename prefix (e.g., 20260216120000).
    version: i64,
    /// Human-readable name extracted from filename (e.g., "create_users").
    name: String,
    /// Full filename (e.g., "20260216120000_create_users.mpl").
    filename: String,
}

// ── Discovery ───────────────────────────────────────────────────────────

/// Discover migration files in the `migrations/` directory.
///
/// Parses filenames matching `{YYYYMMDDHHMMSS}_{name}.mpl` pattern,
/// extracts version and name, and returns sorted by version ascending.
fn discover_migrations(migrations_dir: &Path) -> Result<Vec<MigrationInfo>, String> {
    if !migrations_dir.exists() {
        return Ok(vec![]);
    }

    let mut migrations = Vec::new();
    for entry in std::fs::read_dir(migrations_dir)
        .map_err(|e| format!("Failed to read migrations directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("mpl") {
            continue;
        }
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        // Parse: YYYYMMDDHHMMSS_name.mpl
        if let Some(underscore_pos) = filename.find('_') {
            let version_str = &filename[..underscore_pos];
            if let Ok(version) = version_str.parse::<i64>() {
                let name = filename[underscore_pos + 1..]
                    .trim_end_matches(".mpl")
                    .to_string();
                migrations.push(MigrationInfo {
                    version,
                    name,
                    filename,
                });
            }
        }
    }

    migrations.sort_by_key(|m| m.version);
    Ok(migrations)
}

// ── Tracking Table ──────────────────────────────────────────────────────

/// SQL to create the migration tracking table.
const CREATE_TRACKING_TABLE: &str = "CREATE TABLE IF NOT EXISTS _mesh_migrations (\
    version BIGINT PRIMARY KEY, \
    name TEXT NOT NULL, \
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now())";

/// Query applied migration versions from the tracking table.
fn query_applied_versions(
    conn: &mut mesh_rt::db::pg::NativePgConn,
) -> Result<Vec<i64>, String> {
    let rows = native_pg_query(
        conn,
        "SELECT version FROM _mesh_migrations ORDER BY version",
        &[],
    )?;
    let mut versions = Vec::new();
    for row in &rows {
        for (col, val) in row {
            if col == "version" {
                if let Ok(v) = val.parse::<i64>() {
                    versions.push(v);
                }
            }
        }
    }
    Ok(versions)
}

// ── Synthetic Mesh Program Generation ───────────────────────────────────

/// Generate a synthetic main.mpl that calls Migration.up(pool) or Migration.down(pool).
///
/// The migration file is copied to the temp directory as `migration.mpl`, so
/// the synthetic main imports it as `Migration`.
fn generate_migration_main(direction: &str) -> String {
    format!(
        r#"from Migration import {dir}

fn handle_ok(pool :: PoolHandle) do
  println("MIGRATION_OK")
end

fn handle_err(e :: String) do
  println("MIGRATION_ERROR:" <> e)
end

fn handle_conn_err(e :: String) do
  println("CONNECTION_ERROR:" <> e)
end

fn run_migration(pool :: PoolHandle) do
  let result = {dir}(pool)
  case result do
    Ok(_) -> handle_ok(pool)
    Err(e) -> handle_err(e)
  end
end

fn run_with_url(url :: String) do
  let pool_result = Pool.open(url, 1, 2, 5000)
  case pool_result do
    Ok(pool) -> run_migration(pool)
    Err(e) -> handle_conn_err(e)
  end
end

fn main() do
  let url = Env.get("DATABASE_URL", "")
  if url != "" do
    run_with_url(url)
  else
    println("DATABASE_URL not set")
  end
end
"#,
        dir = direction
    )
}

/// Compile and run a single migration in a temporary directory.
///
/// 1. Creates a tempdir
/// 2. Copies the migration file as `migration.mpl`
/// 3. Generates a synthetic `main.mpl` that calls up() or down()
/// 4. Compiles using `crate::build()`
/// 5. Executes the resulting binary with DATABASE_URL set
/// 6. Checks exit code and stdout for errors
fn compile_and_run_migration(
    project_dir: &Path,
    url: &str,
    migration: &MigrationInfo,
    direction: &str,
) -> Result<(), String> {
    let tmp =
        tempfile::tempdir().map_err(|e| format!("Failed to create temp directory: {}", e))?;
    let tmp_path = tmp.path();

    // Copy migration file as migration.mpl
    let src = project_dir.join("migrations").join(&migration.filename);
    std::fs::copy(&src, tmp_path.join("migration.mpl")).map_err(|e| {
        format!(
            "Failed to copy migration file '{}': {}",
            migration.filename, e
        )
    })?;

    // Generate synthetic main.mpl
    let main_code = generate_migration_main(direction);
    std::fs::write(tmp_path.join("main.mpl"), &main_code)
        .map_err(|e| format!("Failed to write synthetic main.mpl: {}", e))?;

    // Compile the synthetic project
    let output_path = tmp_path.join("_migrate");
    crate::build(
        tmp_path,
        0,
        false,
        Some(&output_path),
        None,
        &DiagnosticOptions::default(),
    )
    .map_err(|e| {
        format!(
            "Failed to compile migration {}_{}: {}",
            migration.version, migration.name, e
        )
    })?;

    // Execute the compiled binary
    let output = std::process::Command::new(&output_path)
        .env("DATABASE_URL", url)
        .output()
        .map_err(|e| {
            format!(
                "Failed to execute migration {}_{}: {}",
                migration.version, migration.name, e
            )
        })?;

    // Check for errors in stdout (the Mesh program prints errors to stdout via IO.puts)
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = stdout.lines().find(|l| l.starts_with("MIGRATION_ERROR:")) {
        let error_msg = line.trim_start_matches("MIGRATION_ERROR:");
        return Err(format!(
            "Migration {}_{} failed: {}",
            migration.version, migration.name, error_msg
        ));
    }
    if let Some(line) = stdout.lines().find(|l| l.starts_with("CONNECTION_ERROR:")) {
        let error_msg = line.trim_start_matches("CONNECTION_ERROR:");
        return Err(format!(
            "Migration {}_{} connection failed: {}",
            migration.version, migration.name, error_msg
        ));
    }

    // Check process exit code
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Migration {}_{} exited with non-zero status: {}",
            migration.version,
            migration.name,
            stderr.trim()
        ));
    }

    Ok(())
}

// ── Public API ──────────────────────────────────────────────────────────

/// Apply all pending migrations.
///
/// 1. Reads DATABASE_URL from environment
/// 2. Connects to PG and ensures tracking table exists
/// 3. Discovers migration files and queries applied versions
/// 4. For each pending migration: compiles, runs, records in tracking table
pub fn run_migrations_up(project_dir: &Path) -> Result<(), String> {
    let url = std::env::var("DATABASE_URL").map_err(|_| {
        "meshc migrate: DATABASE_URL environment variable is required".to_string()
    })?;

    let migrations_dir = project_dir.join("migrations");
    if !migrations_dir.exists() {
        eprintln!(
            "No migrations directory found. \
             Run 'meshc migrate generate <name>' to create your first migration."
        );
        return Ok(());
    }

    let migrations = discover_migrations(&migrations_dir)?;
    if migrations.is_empty() {
        eprintln!("No migration files found in migrations/");
        return Ok(());
    }

    // Connect to PG for tracking table operations
    let mut conn = native_pg_connect(&url)
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    // Ensure tracking table exists
    native_pg_execute(&mut conn, CREATE_TRACKING_TABLE, &[])
        .map_err(|e| format!("Failed to create tracking table: {}", e))?;

    // Query applied versions
    let applied = query_applied_versions(&mut conn)?;

    // Determine pending migrations
    let pending: Vec<&MigrationInfo> = migrations
        .iter()
        .filter(|m| !applied.contains(&m.version))
        .collect();

    if pending.is_empty() {
        eprintln!("No pending migrations");
        native_pg_close(conn);
        return Ok(());
    }

    eprintln!("Running {} pending migration(s):", pending.len());

    let mut applied_count = 0;
    for migration in &pending {
        eprintln!("  Applying: {}_{}", migration.version, migration.name);

        // Compile and run the migration
        compile_and_run_migration(project_dir, &url, migration, "up")?;

        // Record in tracking table
        let version_str = migration.version.to_string();
        native_pg_execute(
            &mut conn,
            "INSERT INTO _mesh_migrations (version, name) VALUES ($1, $2)",
            &[&version_str, &migration.name],
        )
        .map_err(|e| {
            format!(
                "Failed to record migration {}_{}: {}",
                migration.version, migration.name, e
            )
        })?;

        eprintln!("  Applied:  {}_{}", migration.version, migration.name);
        applied_count += 1;
    }

    native_pg_close(conn);
    eprintln!("Applied {} migration(s)", applied_count);
    Ok(())
}

/// Rollback the last applied migration.
///
/// 1. Connects to PG, queries the last applied version
/// 2. Finds the corresponding migration file
/// 3. Compiles and runs with direction "down"
/// 4. Removes the tracking row
pub fn run_migrations_down(project_dir: &Path) -> Result<(), String> {
    let url = std::env::var("DATABASE_URL").map_err(|_| {
        "meshc migrate: DATABASE_URL environment variable is required".to_string()
    })?;

    let migrations_dir = project_dir.join("migrations");

    // Connect to PG
    let mut conn = native_pg_connect(&url)
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    // Ensure tracking table exists
    native_pg_execute(&mut conn, CREATE_TRACKING_TABLE, &[])
        .map_err(|e| format!("Failed to create tracking table: {}", e))?;

    // Query applied versions
    let applied = query_applied_versions(&mut conn)?;

    if applied.is_empty() {
        eprintln!("No migrations to roll back");
        native_pg_close(conn);
        return Ok(());
    }

    // Find the last applied version
    let last_version = *applied.last().unwrap();

    // Find the corresponding migration file
    let migrations = discover_migrations(&migrations_dir)?;
    let migration = migrations.iter().find(|m| m.version == last_version).ok_or_else(|| {
        format!(
            "Migration file for version {} not found in migrations/",
            last_version
        )
    })?;

    eprintln!("Rolling back: {}_{}", migration.version, migration.name);

    // Compile and run with direction "down"
    compile_and_run_migration(project_dir, &url, migration, "down")?;

    // Remove tracking row
    let version_str = last_version.to_string();
    native_pg_execute(
        &mut conn,
        "DELETE FROM _mesh_migrations WHERE version = $1",
        &[&version_str],
    )
    .map_err(|e| {
        format!(
            "Failed to remove tracking row for version {}: {}",
            last_version, e
        )
    })?;

    native_pg_close(conn);
    eprintln!("Rolled back: {}_{}", migration.version, migration.name);
    Ok(())
}

/// Show migration status (applied vs pending).
///
/// Connects to PG, discovers migration files, and prints a status table
/// showing which migrations have been applied and which are pending.
pub fn show_migration_status(project_dir: &Path) -> Result<(), String> {
    let url = std::env::var("DATABASE_URL").map_err(|_| {
        "meshc migrate: DATABASE_URL environment variable is required".to_string()
    })?;

    let migrations_dir = project_dir.join("migrations");
    if !migrations_dir.exists() {
        eprintln!(
            "No migrations directory found. \
             Run 'meshc migrate generate <name>' to create your first migration."
        );
        return Ok(());
    }

    let migrations = discover_migrations(&migrations_dir)?;
    if migrations.is_empty() {
        eprintln!("No migration files found in migrations/");
        return Ok(());
    }

    // Connect to PG
    let mut conn = native_pg_connect(&url)
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    // Ensure tracking table exists
    native_pg_execute(&mut conn, CREATE_TRACKING_TABLE, &[])
        .map_err(|e| format!("Failed to create tracking table: {}", e))?;

    // Query applied versions
    let applied = query_applied_versions(&mut conn)?;
    native_pg_close(conn);

    // Print status
    eprintln!("Migration Status:");
    let mut applied_count = 0;
    let mut pending_count = 0;
    for migration in &migrations {
        if applied.contains(&migration.version) {
            eprintln!("  [x] {}_{}", migration.version, migration.name);
            applied_count += 1;
        } else {
            eprintln!("  [ ] {}_{}", migration.version, migration.name);
            pending_count += 1;
        }
    }
    eprintln!("{} applied, {} pending", applied_count, pending_count);
    Ok(())
}

// ── Scaffold Generation ─────────────────────────────────────────────────

/// Generate a new migration scaffold file in the `migrations/` directory.
///
/// Creates `migrations/{timestamp}_{name}.mpl` with up/down function stubs.
/// The `migrations/` directory is created if it doesn't exist.
///
/// # Errors
///
/// Returns an error if:
/// - `name` is empty
/// - `name` contains characters other than lowercase ASCII letters, digits, or underscores
/// - Directory creation or file writing fails
pub fn generate_migration(project_dir: &Path, name: &str) -> Result<(), String> {
    // Validate name: must be non-empty, only lowercase letters, digits, underscores
    if name.is_empty() {
        return Err("Migration name cannot be empty".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(
            "Migration name must contain only lowercase letters, digits, and underscores"
                .to_string(),
        );
    }

    let migrations_dir = project_dir.join("migrations");
    std::fs::create_dir_all(&migrations_dir)
        .map_err(|e| format!("Failed to create migrations directory: {}", e))?;

    let timestamp = format_timestamp_now();
    let filename = format!("{}_{}.mpl", timestamp, name);
    let filepath = migrations_dir.join(&filename);

    let content = format!(
        r#"# Migration: {name}
# Generated: {timestamp}

pub fn up(pool :: PoolHandle) -> Int!String do
  # Add your migration code here
  # Examples:
  #   Migration.create_table(pool, "users", [
  #     "id:UUID:PRIMARY KEY DEFAULT gen_random_uuid()",
  #     "name:TEXT:NOT NULL",
  #     "email:TEXT:NOT NULL UNIQUE",
  #     "inserted_at:TIMESTAMPTZ:NOT NULL DEFAULT now()",
  #     "updated_at:TIMESTAMPTZ:NOT NULL DEFAULT now()"
  #   ])?
  #
  #   Migration.create_index(pool, "users", ["email"], "unique:true")?
  #
  #   Migration.add_column(pool, "users", "age:BIGINT")?
  #
  #   Migration.execute(pool, "CREATE EXTENSION IF NOT EXISTS pgcrypto")?
  Ok(0)
end

pub fn down(pool :: PoolHandle) -> Int!String do
  # Add your rollback code here
  # Examples:
  #   Migration.drop_table(pool, "users")?
  #   Migration.drop_column(pool, "users", "age")?
  #   Migration.drop_index(pool, "users", ["email"])?
  Ok(0)
end
"#,
        name = name,
        timestamp = timestamp
    );

    std::fs::write(&filepath, content)
        .map_err(|e| format!("Failed to write migration file: {}", e))?;

    eprintln!("Created migration: migrations/{}", filename);
    Ok(())
}

// ── Timestamp Utilities ─────────────────────────────────────────────────

/// Generate a YYYYMMDDHHMMSS timestamp string from the current UTC time.
fn format_timestamp_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format_timestamp(secs)
}

/// Convert a Unix timestamp (seconds since epoch) to a YYYYMMDDHHMMSS string.
fn format_timestamp(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let (year, month, day) = civil_from_days(days);

    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since 1970-01-01 to (year, month, day).
/// Uses the Howard Hinnant algorithm from chrono-free date calculations.
fn civil_from_days(days: i64) -> (i64, u64, u64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u64, d as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp_epoch() {
        assert_eq!(format_timestamp(0), "19700101000000");
    }

    #[test]
    fn test_format_timestamp_known_date() {
        assert_eq!(format_timestamp(1771243200), "20260216120000");
    }

    #[test]
    fn test_format_timestamp_y2k() {
        assert_eq!(format_timestamp(946684800), "20000101000000");
    }

    #[test]
    fn test_format_timestamp_end_of_day() {
        assert_eq!(format_timestamp(1771286399), "20260216235959");
    }

    #[test]
    fn test_civil_from_days_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn test_civil_from_days_known() {
        assert_eq!(civil_from_days(20500), (2026, 2, 16));
    }

    #[test]
    fn test_civil_from_days_leap_year() {
        assert_eq!(civil_from_days(19782), (2024, 2, 29));
    }

    #[test]
    fn test_generate_migration_validates_empty_name() {
        let tmp = tempfile::tempdir().unwrap();
        let result = generate_migration(tmp.path(), "");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_generate_migration_validates_uppercase() {
        let tmp = tempfile::tempdir().unwrap();
        let result = generate_migration(tmp.path(), "Create_Users");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("lowercase"));
    }

    #[test]
    fn test_generate_migration_validates_spaces() {
        let tmp = tempfile::tempdir().unwrap();
        let result = generate_migration(tmp.path(), "create users");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("lowercase"));
    }

    #[test]
    fn test_generate_migration_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        generate_migration(tmp.path(), "create_users").unwrap();

        let migrations_dir = tmp.path().join("migrations");
        assert!(migrations_dir.exists());

        let entries: Vec<_> = std::fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);

        let filename = entries[0].file_name().to_string_lossy().to_string();
        assert!(filename.ends_with("_create_users.mpl"));
        let prefix = &filename[..14];
        assert!(prefix.chars().all(|c| c.is_ascii_digit()));
        assert_eq!(&filename[14..15], "_");
    }

    #[test]
    fn test_generate_migration_file_content() {
        let tmp = tempfile::tempdir().unwrap();
        generate_migration(tmp.path(), "create_users").unwrap();

        let migrations_dir = tmp.path().join("migrations");
        let entry = std::fs::read_dir(&migrations_dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let content = std::fs::read_to_string(entry.path()).unwrap();

        assert!(content.contains("pub fn up(pool :: PoolHandle) -> Int!String do"));
        assert!(content.contains("pub fn down(pool :: PoolHandle) -> Int!String do"));
        assert!(content.contains("# Migration: create_users"));
        assert!(content.contains("Migration.create_table"));
    }

    #[test]
    fn test_generate_migration_creates_migrations_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let migrations_dir = tmp.path().join("migrations");
        assert!(!migrations_dir.exists());
        generate_migration(tmp.path(), "init").unwrap();
        assert!(migrations_dir.exists());
    }

    #[test]
    fn test_generate_migration_allows_digits_and_underscores() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(generate_migration(tmp.path(), "add_column_v2").is_ok());
    }

    #[test]
    fn test_discover_migrations_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let migrations_dir = tmp.path().join("migrations");
        std::fs::create_dir_all(&migrations_dir).unwrap();
        let migrations = discover_migrations(&migrations_dir).unwrap();
        assert!(migrations.is_empty());
    }

    #[test]
    fn test_discover_migrations_no_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let migrations_dir = tmp.path().join("migrations");
        let migrations = discover_migrations(&migrations_dir).unwrap();
        assert!(migrations.is_empty());
    }

    #[test]
    fn test_discover_migrations_sorts_by_version() {
        let tmp = tempfile::tempdir().unwrap();
        let migrations_dir = tmp.path().join("migrations");
        std::fs::create_dir_all(&migrations_dir).unwrap();

        let stub = "pub fn up(pool :: PoolHandle) -> Int!String do Ok(0) end\n\
                     pub fn down(pool :: PoolHandle) -> Int!String do Ok(0) end\n";
        std::fs::write(migrations_dir.join("20260216120200_add_index.mpl"), stub).unwrap();
        std::fs::write(migrations_dir.join("20260216120000_create_users.mpl"), stub).unwrap();
        std::fs::write(migrations_dir.join("20260216120100_create_posts.mpl"), stub).unwrap();

        let migrations = discover_migrations(&migrations_dir).unwrap();
        assert_eq!(migrations.len(), 3);
        assert_eq!(migrations[0].version, 20260216120000);
        assert_eq!(migrations[0].name, "create_users");
        assert_eq!(migrations[1].version, 20260216120100);
        assert_eq!(migrations[1].name, "create_posts");
        assert_eq!(migrations[2].version, 20260216120200);
        assert_eq!(migrations[2].name, "add_index");
    }

    #[test]
    fn test_discover_migrations_skips_non_mpl() {
        let tmp = tempfile::tempdir().unwrap();
        let migrations_dir = tmp.path().join("migrations");
        std::fs::create_dir_all(&migrations_dir).unwrap();

        let stub = "pub fn up(pool :: PoolHandle) -> Int!String do Ok(0) end\n\
                     pub fn down(pool :: PoolHandle) -> Int!String do Ok(0) end\n";
        std::fs::write(migrations_dir.join("20260216120000_create_users.mpl"), stub).unwrap();
        std::fs::write(migrations_dir.join("README.md"), "readme").unwrap();
        std::fs::write(migrations_dir.join(".gitkeep"), "").unwrap();

        let migrations = discover_migrations(&migrations_dir).unwrap();
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].name, "create_users");
    }

    #[test]
    fn test_generate_migration_main_up() {
        let main = generate_migration_main("up");
        assert!(main.contains("from Migration import up"));
        assert!(main.contains("up(pool)"));
        assert!(main.contains("Pool.open(url, 1, 2, 5000)"));
        assert!(main.contains("Pool.close(pool)"));
    }

    #[test]
    fn test_generate_migration_main_down() {
        let main = generate_migration_main("down");
        assert!(main.contains("from Migration import down"));
        assert!(main.contains("down(pool)"));
    }
}
